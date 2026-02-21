//! WebSocket upgrade handler and message dispatch.
//! Handles WebSocket connection upgrades via axum, routes incoming messages
//! to the appropriate game session handlers, and manages connection lifecycle.
//! Includes self-play mode with per-game streaming and cancellation support.

use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::State,
    response::IntoResponse,
    routing::get,
    Router,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tower_http::services::ServeDir;

use crate::engine::eval::EvalWeights;
use crate::engine::types::GameResult;
use crate::learning::self_play::{result_to_string, SelfPlayRunner};
use crate::server::protocol::{ClientMessage, ServerMessage};
use crate::server::session::{GameSession, SessionState};

/// Maximum search depth for self-play games. Keeps games fast (~1s each)
/// so the live board updates promptly. Higher depths make each game take
/// minutes, which blocks all progress messages.
const SELF_PLAY_MAX_DEPTH: u8 = 2;

/// Shared server configuration passed to each WebSocket connection.
#[derive(Clone)]
pub struct ServerConfig {
    pub weights: EvalWeights,
    pub max_depth: u8,
    pub search_log: bool,
}

/// Create the axum router with WebSocket endpoint and static file serving.
pub fn create_router(weights: EvalWeights, max_depth: u8, search_log: bool, frontend_dir: &str) -> Router {
    let config = Arc::new(ServerConfig { weights, max_depth, search_log });

    Router::new()
        .route("/ws", get(ws_handler))
        .with_state(config)
        .fallback_service(ServeDir::new(frontend_dir))
}

/// WebSocket upgrade handler: upgrades HTTP to WebSocket and spawns the connection handler.
async fn ws_handler(
    ws: WebSocketUpgrade,
    State(config): State<Arc<ServerConfig>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, config))
}

/// Handle a single WebSocket connection.
/// Creates a GameSession and loops: receive -> deserialize -> handle -> serialize -> send.
/// When self-play is requested, enters a dedicated loop with per-game progress streaming.
async fn handle_socket(mut socket: WebSocket, config: Arc<ServerConfig>) {
    tracing::info!("WebSocket client connected");

    let mut session = GameSession::new(config.weights.clone(), config.max_depth, config.search_log);

    loop {
        let msg = match socket.recv().await {
            Some(Ok(msg)) => msg,
            Some(Err(e)) => {
                tracing::warn!("WebSocket receive error: {}", e);
                break;
            }
            None => {
                tracing::info!("WebSocket client disconnected");
                break;
            }
        };

        let text = match msg {
            Message::Text(t) => t,
            Message::Close(_) => {
                tracing::info!("WebSocket client sent close frame");
                break;
            }
            // Ignore binary, ping, pong.
            _ => continue,
        };

        let client_msg: ClientMessage = match serde_json::from_str(&text) {
            Ok(m) => m,
            Err(e) => {
                let error_msg = ServerMessage::Error {
                    code: "invalid_message".to_string(),
                    message: format!("Failed to parse message: {}", e),
                };
                let json = serde_json::to_string(&error_msg).unwrap_or_default();
                if socket.send(Message::Text(json.into())).await.is_err() {
                    break;
                }
                continue;
            }
        };

        // Handle self-play at the WebSocket level (not delegated to session).
        if let ClientMessage::StartSelfPlay { num_games } = client_msg {
            if *session.session_state() != SessionState::ColorSelection {
                let err = ServerMessage::Error {
                    code: "invalid_state".to_string(),
                    message: "Self-play can only be started from the color selection screen"
                        .to_string(),
                };
                if send_message(&mut socket, &err).await.is_err() {
                    break;
                }
                continue;
            }

            session.set_state(SessionState::SelfPlaying);
            let disconnected = run_self_play(
                &mut socket,
                &mut session,
                &config,
                num_games,
            )
            .await;

            if disconnected {
                break;
            }
            continue;
        }

        let responses = session.handle_message(client_msg);

        for response in responses {
            if send_message(&mut socket, &response).await.is_err() {
                tracing::warn!("Failed to send message, client likely disconnected");
                return;
            }
        }
    }
}

/// Run the self-play loop: play N games, send progress after each, check for cancellation.
/// Returns true if the client disconnected (caller should break the main loop).
async fn run_self_play(
    socket: &mut WebSocket,
    session: &mut GameSession,
    config: &ServerConfig,
    num_games: u32,
) -> bool {
    // Cap self-play depth for fast game throughput (depth 6+ takes minutes per game).
    let self_play_depth = config.max_depth.min(SELF_PLAY_MAX_DEPTH);
    tracing::info!("Starting self-play: {} games at depth {}", num_games, self_play_depth);

    let cancel_flag = Arc::new(AtomicBool::new(false));
    let mut runner = SelfPlayRunner::new(
        config.weights.clone(),
        self_play_depth,
        cancel_flag.clone(),
        config.search_log,
    );

    let mut white_wins: u32 = 0;
    let mut black_wins: u32 = 0;
    let mut draws: u32 = 0;

    for game_number in 1..=num_games {
        // Check for cancellation message (non-blocking).
        if check_for_cancel(socket, &cancel_flag).await {
            tracing::info!("Self-play cancelled at game {}/{}", game_number, num_games);
            break;
        }

        if cancel_flag.load(Ordering::Relaxed) {
            break;
        }

        // Run one game (blocking but fast at low depths).
        let game_result = runner.run_game();

        let game = match game_result {
            Some(g) => g,
            None => {
                // Game was cancelled mid-game.
                tracing::info!("Self-play game cancelled mid-game");
                break;
            }
        };

        // Update totals.
        match game.result {
            GameResult::White => white_wins += 1,
            GameResult::Black => black_wins += 1,
            GameResult::Draw => draws += 1,
        }

        // Send per-move messages for live board display.
        for mv in &game.moves {
            let move_msg = ServerMessage::SelfPlayMove {
                game_number,
                total_games: num_games,
                move_number: mv.move_number,
                from: mv.from.clone(),
                to: mv.to.clone(),
                promotion: mv.promotion.clone(),
                fen: mv.fen.clone(),
            };
            if send_message(socket, &move_msg).await.is_err() {
                return true;
            }
        }

        // Send progress.
        let progress = ServerMessage::SelfPlayProgress {
            game_number,
            total_games: num_games,
            result: result_to_string(game.result),
            reason: game.reason,
            moves: game.move_count,
            avg_td_error: game.avg_td_error,
            weight_version: game.weight_version,
        };

        if send_message(socket, &progress).await.is_err() {
            return true; // Client disconnected.
        }
    }

    let completed_games = white_wins + black_wins + draws;

    // Send completion message.
    let complete = ServerMessage::SelfPlayComplete {
        total_games: completed_games,
        white_wins,
        black_wins,
        draws,
        weight_version: runner.weight_version(),
    };

    if send_message(socket, &complete).await.is_err() {
        return true;
    }

    // Update session weights from self-play learning and return to color selection.
    session.update_weights_from_self_play(runner.current_weights().clone());
    session.set_state(SessionState::ColorSelection);

    tracing::info!(
        "Self-play complete: {} games (W:{} B:{} D:{}), weight v{}",
        completed_games, white_wins, black_wins, draws, runner.weight_version()
    );

    false
}

/// Non-blocking check for a cancellation message on the WebSocket.
/// Returns true if CancelSelfPlay was received.
async fn check_for_cancel(socket: &mut WebSocket, cancel_flag: &Arc<AtomicBool>) -> bool {
    // Use a very short timeout to check for pending messages.
    let result = tokio::time::timeout(Duration::from_millis(10), socket.recv()).await;

    match result {
        Ok(Some(Ok(Message::Text(text)))) => {
            if let Ok(ClientMessage::CancelSelfPlay) = serde_json::from_str(&text) {
                cancel_flag.store(true, Ordering::Relaxed);
                return true;
            }
            // Ignore other messages during self-play.
            false
        }
        Ok(Some(Ok(Message::Close(_)))) => {
            cancel_flag.store(true, Ordering::Relaxed);
            true
        }
        Ok(Some(Err(_))) => {
            cancel_flag.store(true, Ordering::Relaxed);
            true
        }
        Ok(None) => {
            // Connection closed.
            cancel_flag.store(true, Ordering::Relaxed);
            true
        }
        Ok(Some(Ok(_))) => {
            // Ignore binary, ping, pong during self-play.
            false
        }
        Err(_) => {
            // Timeout — no message pending, continue.
            false
        }
    }
}

/// Send a server message over the WebSocket. Returns Err if the send fails.
async fn send_message(socket: &mut WebSocket, msg: &ServerMessage) -> Result<(), ()> {
    let json = match serde_json::to_string(msg) {
        Ok(j) => j,
        Err(e) => {
            tracing::error!("Failed to serialize response: {}", e);
            return Ok(()); // Serialization error is not a send failure.
        }
    };
    socket.send(Message::Text(json.into())).await.map_err(|_| ())
}
