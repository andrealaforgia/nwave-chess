//! Game session state machine.
//! Manages the lifecycle of a single game session through states:
//! ColorSelection -> InGame -> GameOver -> Learning -> Idle.
//! Holds board state, game history, and coordinates with engine search and learning.

use cozy_chess::Color;

use crate::engine::board::Board;
use crate::engine::eval::EvalWeights;
use crate::engine::search::{SearchEngine, SearchProgress};
use crate::engine::types::{GameMove, GameResult, GameState};
use crate::learning::collector::GameCollector;
use crate::learning::optimizer::AdamOptimizer;
use crate::learning::td_leaf;
use crate::learning::weights::WeightManager;
use crate::server::protocol::{ClientMessage, ServerMessage};

/// Session lifecycle states.
#[derive(Debug, Clone, PartialEq)]
pub enum SessionState {
    ColorSelection,
    InGame,
    GameOver,
    Learning,
    SelfPlaying,
}

/// Manages a single game session between a human player and the engine.
pub struct GameSession {
    state: SessionState,
    board: Board,
    player_color: Option<Color>,
    search_engine: SearchEngine,
    collector: GameCollector,
    weight_manager: WeightManager,
    optimizer: AdamOptimizer,
    /// Number of engine moves made in the current game (for collector indexing).
    engine_move_count: u32,
    game_id_counter: i64,
}

impl GameSession {
    /// Create a new session with the given evaluation weights and search depth.
    pub fn new(weights: EvalWeights, max_depth: u8, search_log: bool) -> Self {
        let num_weights = EvalWeights::num_weights();
        let mut search_engine = SearchEngine::new(weights.clone(), max_depth);
        search_engine.set_verbose(search_log);
        Self {
            state: SessionState::ColorSelection,
            board: Board::new(),
            player_color: None,
            search_engine,
            collector: GameCollector::new(),
            weight_manager: WeightManager::new(weights),
            optimizer: AdamOptimizer::new(num_weights, 0.001),
            engine_move_count: 0,
            game_id_counter: 0,
        }
    }

    /// Process a client message and return server messages to send back.
    /// May return multiple messages (e.g., move_accepted + engine_thinking + engine_move).
    pub fn handle_message(&mut self, msg: ClientMessage) -> Vec<ServerMessage> {
        match msg {
            ClientMessage::SelectColor { color } => self.handle_select_color(&color),
            ClientMessage::MakeMove {
                from,
                to,
                promotion,
            } => self.handle_make_move(&from, &to, promotion.as_deref()),
            ClientMessage::Resign => self.handle_resign(),
            ClientMessage::NewGame => self.handle_new_game(),
            ClientMessage::RequestLearningStatus => {
                // Placeholder: learning status not yet implemented with persistence.
                vec![ServerMessage::Error {
                    code: "not_implemented".to_string(),
                    message: "Learning status query not yet implemented".to_string(),
                }]
            }
            ClientMessage::StartSelfPlay { .. } | ClientMessage::CancelSelfPlay => {
                // Self-play is handled at the WebSocket layer, not by the session.
                vec![ServerMessage::Error {
                    code: "invalid_state".to_string(),
                    message: "Self-play messages are handled at the connection level".to_string(),
                }]
            }
        }
    }

    /// Handle color selection: set up a new game and optionally run engine's first move.
    fn handle_select_color(&mut self, color: &str) -> Vec<ServerMessage> {
        if self.state != SessionState::ColorSelection {
            return vec![ServerMessage::Error {
                code: "invalid_state".to_string(),
                message: "Color selection is only available before a game starts".to_string(),
            }];
        }

        let player_color = match color {
            "white" => Color::White,
            "black" => Color::Black,
            _ => {
                return vec![ServerMessage::Error {
                    code: "invalid_color".to_string(),
                    message: format!("Invalid color '{}'. Choose 'white' or 'black'.", color),
                }];
            }
        };

        self.player_color = Some(player_color);
        self.board = Board::new();
        self.collector = GameCollector::new();
        self.search_engine.clear_tt();
        self.engine_move_count = 0;
        self.state = SessionState::InGame;

        let engine_color = !player_color;
        let mut messages = vec![ServerMessage::GameStarted {
            fen: self.board.to_fen(),
            player_color: color_to_string(player_color),
            engine_color: color_to_string(engine_color),
            weight_version: self.weight_manager.version(),
        }];

        // If the engine plays White, it moves first.
        if engine_color == Color::White {
            messages.extend(self.run_engine_move());
        }

        messages
    }

    /// Handle a player move: validate, apply, check game over, run engine response.
    fn handle_make_move(
        &mut self,
        from: &str,
        to: &str,
        promotion: Option<&str>,
    ) -> Vec<ServerMessage> {
        if self.state != SessionState::InGame {
            return vec![ServerMessage::Error {
                code: "invalid_state".to_string(),
                message: "No game in progress".to_string(),
            }];
        }

        // Verify it is the player's turn.
        if let Some(player_color) = self.player_color {
            if self.board.side_to_move() != player_color {
                return vec![ServerMessage::MoveRejected {
                    reason: "Not your turn".to_string(),
                    fen: self.board.to_fen(),
                }];
            }
        }

        let game_move = GameMove::new(from, to, promotion);

        // Try to apply the move.
        if let Err(reason) = self.board.make_move(&game_move) {
            return vec![ServerMessage::MoveRejected {
                reason,
                fen: self.board.to_fen(),
            }];
        }

        // Record the player's move as the actual opponent move for the last engine search.
        if self.engine_move_count > 0 {
            let last_engine_idx = (self.engine_move_count - 1) as usize;
            self.collector
                .set_opponent_move(last_engine_idx, game_move.clone());
        }

        let mut messages = vec![ServerMessage::MoveAccepted {
            from: from.to_string(),
            to: to.to_string(),
            fen: self.board.to_fen(),
        }];

        // Check if the player's move ended the game.
        if let Some(game_over_msg) = self.check_game_over() {
            messages.push(game_over_msg);
            messages.extend(self.run_learning());
            return messages;
        }

        // Engine responds.
        messages.extend(self.run_engine_move());

        messages
    }

    /// Handle resignation: the player resigns and the engine wins.
    fn handle_resign(&mut self) -> Vec<ServerMessage> {
        if self.state != SessionState::InGame {
            return vec![ServerMessage::Error {
                code: "invalid_state".to_string(),
                message: "No game in progress".to_string(),
            }];
        }

        let player_color = self.player_color.unwrap_or(Color::White);
        let result = match player_color {
            Color::White => "black",
            Color::Black => "white",
        };

        // Record the game result for learning.
        let game_result = match player_color {
            Color::White => GameResult::Black,
            Color::Black => GameResult::White,
        };
        self.collector.set_result(game_result);
        self.state = SessionState::GameOver;

        let mut messages = vec![ServerMessage::GameOver {
            result: result.to_string(),
            reason: "resignation".to_string(),
            fen: self.board.to_fen(),
        }];

        messages.extend(self.run_learning());
        messages
    }

    /// Handle new game request: reset state to color selection.
    fn handle_new_game(&mut self) -> Vec<ServerMessage> {
        self.state = SessionState::ColorSelection;
        self.board = Board::new();
        self.player_color = None;
        self.collector = GameCollector::new();
        self.engine_move_count = 0;

        // Return an indication that the session is reset. The client should send select_color.
        // We use an error-free approach: just send nothing, client knows to show color selection.
        // Actually, to confirm the reset, we could send a message. But the protocol does not
        // define one. We will just return empty -- the client transitions on receiving this.
        vec![]
    }

    /// Run the engine search, collect leaf data, make the engine move.
    /// Returns EngineThinking messages and the final EngineMove (or GameOver).
    fn run_engine_move(&mut self) -> Vec<ServerMessage> {
        let mut messages = Vec::new();

        // Collect thinking progress into messages.
        let mut thinking_messages: Vec<ServerMessage> = Vec::new();
        let mut callback = |progress: SearchProgress| {
            let best_move_str = progress
                .best_move
                .as_ref()
                .map(|m| format!("{}", m))
                .unwrap_or_default();
            let pv_strings: Vec<String> = progress.pv_line.iter().map(|m| format!("{}", m)).collect();
            thinking_messages.push(ServerMessage::EngineThinking {
                depth: progress.depth,
                evaluation_cp: progress.evaluation_cp,
                best_move: best_move_str,
                pv_line: pv_strings,
                nodes: progress.nodes,
                time_ms: progress.time_ms,
            });
        };

        let (result, leaf_data) = self.search_engine.search(&self.board, Some(&mut callback));
        messages.extend(thinking_messages);

        // Record search data for learning.
        self.collector.record_move(leaf_data);
        self.engine_move_count += 1;

        // Make the engine's move.
        let best_move = match result.best_move {
            Some(mv) => mv,
            None => {
                // No legal moves -- should be caught by game_state check, but be safe.
                return messages;
            }
        };

        if let Err(e) = self.board.make_move(&best_move) {
            messages.push(ServerMessage::Error {
                code: "engine_error".to_string(),
                message: format!("Engine produced illegal move: {}", e),
            });
            return messages;
        }

        messages.push(ServerMessage::EngineMoveMsg {
            from: best_move.from.clone(),
            to: best_move.to.clone(),
            promotion: best_move.promotion.clone(),
            fen: self.board.to_fen(),
            evaluation_cp: result.evaluation as i32,
            search_depth: result.depth,
            nodes_searched: result.nodes_searched,
        });

        // Check if the engine's move ended the game.
        if let Some(game_over_msg) = self.check_game_over() {
            messages.push(game_over_msg);
            messages.extend(self.run_learning());
        }

        messages
    }

    /// Check for terminal game states and return a GameOver message if applicable.
    fn check_game_over(&mut self) -> Option<ServerMessage> {
        let game_state = self.board.game_state();
        let (result_str, reason, game_result) = match game_state {
            GameState::Checkmate => {
                // The side to move is the loser (they are in checkmate).
                let loser = self.board.side_to_move();
                let (result_str, game_result) = match loser {
                    Color::White => ("black".to_string(), GameResult::Black),
                    Color::Black => ("white".to_string(), GameResult::White),
                };
                (result_str, "checkmate".to_string(), game_result)
            }
            GameState::Stalemate => {
                ("draw".to_string(), "stalemate".to_string(), GameResult::Draw)
            }
            GameState::DrawByRepetition => (
                "draw".to_string(),
                "threefold_repetition".to_string(),
                GameResult::Draw,
            ),
            GameState::DrawByFiftyMove => {
                ("draw".to_string(), "fifty_move".to_string(), GameResult::Draw)
            }
            GameState::DrawByInsufficientMaterial => (
                "draw".to_string(),
                "insufficient_material".to_string(),
                GameResult::Draw,
            ),
            GameState::InProgress | GameState::Check => return None,
        };

        self.collector.set_result(game_result);
        self.state = SessionState::GameOver;

        Some(ServerMessage::GameOver {
            result: result_str,
            reason,
            fen: self.board.to_fen(),
        })
    }

    /// Run post-game TD-Leaf learning and return a LearningUpdate message.
    fn run_learning(&mut self) -> Vec<ServerMessage> {
        self.state = SessionState::Learning;

        // Need at least one recorded move to compute TD update.
        if self.collector.is_empty() || !self.collector.is_complete() {
            self.state = SessionState::GameOver;
            return vec![];
        }

        let td_result = td_leaf::compute_td_update(&self.collector);
        let summary = self.weight_manager.apply_update(
            &mut self.optimizer,
            &td_result.total_gradient,
            1.0,
        );

        // Update the search engine with the new weights.
        self.search_engine
            .set_weights(self.weight_manager.current_weights().clone());

        self.game_id_counter += 1;

        let msg = ServerMessage::LearningUpdate {
            game_id: self.game_id_counter,
            avg_td_error: td_result.avg_td_error,
            max_td_error: td_result.max_td_error,
            weight_change_norm: summary.weight_change_norm,
            weight_version: summary.version,
        };

        self.state = SessionState::GameOver;
        vec![msg]
    }

    /// Get the current session state.
    pub fn session_state(&self) -> &SessionState {
        &self.state
    }

    /// Set the session state (used by ws.rs for self-play transitions).
    pub fn set_state(&mut self, state: SessionState) {
        self.state = state;
    }

    /// Update the search engine weights after self-play learning.
    pub fn update_weights_from_self_play(&mut self, weights: EvalWeights) {
        self.search_engine.set_weights(weights.clone());
        self.weight_manager = WeightManager::new(weights);
    }
}

/// Convert a cozy_chess Color to a protocol string.
fn color_to_string(color: Color) -> String {
    match color {
        Color::White => "white".to_string(),
        Color::Black => "black".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create a session with depth 1 for fast tests.
    fn test_session() -> GameSession {
        GameSession::new(EvalWeights::default_weights(), 1, false)
    }

    /// Helper: select white and return the response messages.
    fn select_white(session: &mut GameSession) -> Vec<ServerMessage> {
        session.handle_message(ClientMessage::SelectColor {
            color: "white".to_string(),
        })
    }

    /// Helper: select black and return the response messages.
    fn select_black(session: &mut GameSession) -> Vec<ServerMessage> {
        session.handle_message(ClientMessage::SelectColor {
            color: "black".to_string(),
        })
    }

    #[test]
    fn new_session_starts_in_color_selection_state() {
        let session = test_session();
        assert_eq!(*session.session_state(), SessionState::ColorSelection);
    }

    #[test]
    fn select_color_transitions_to_in_game_and_returns_game_started() {
        let mut session = test_session();
        let messages = select_white(&mut session);

        assert_eq!(*session.session_state(), SessionState::InGame);

        // First message should be GameStarted.
        assert!(!messages.is_empty());
        match &messages[0] {
            ServerMessage::GameStarted {
                player_color,
                engine_color,
                weight_version,
                fen,
            } => {
                assert_eq!(player_color, "white");
                assert_eq!(engine_color, "black");
                assert_eq!(*weight_version, 0);
                assert!(fen.contains("rnbqkbnr"));
            }
            other => panic!("Expected GameStarted, got {:?}", other),
        }
    }

    #[test]
    fn select_black_causes_engine_to_move_first() {
        let mut session = test_session();
        let messages = select_black(&mut session);

        // Should have: GameStarted, then EngineThinking(s), then EngineMove.
        assert!(messages.len() >= 2, "Expected at least GameStarted + EngineMove, got {}", messages.len());
        assert!(matches!(&messages[0], ServerMessage::GameStarted { .. }));

        // Last message should be EngineMove.
        let last = messages.last().unwrap();
        assert!(
            matches!(last, ServerMessage::EngineMoveMsg { .. }),
            "Last message should be EngineMove, got {:?}",
            last
        );
    }

    #[test]
    fn valid_move_is_accepted_and_engine_responds() {
        let mut session = test_session();
        select_white(&mut session);

        let messages = session.handle_message(ClientMessage::MakeMove {
            from: "e2".to_string(),
            to: "e4".to_string(),
            promotion: None,
        });

        // Should have MoveAccepted, then EngineThinking(s), then EngineMove (or GameOver).
        assert!(!messages.is_empty());
        match &messages[0] {
            ServerMessage::MoveAccepted { from, to, fen } => {
                assert_eq!(from, "e2");
                assert_eq!(to, "e4");
                assert!(fen.contains("4P3"), "FEN should reflect e4 pawn, got {}", fen);
            }
            other => panic!("Expected MoveAccepted, got {:?}", other),
        }

        // There should be an EngineMove somewhere in the response.
        let has_engine_move = messages
            .iter()
            .any(|m| matches!(m, ServerMessage::EngineMoveMsg { .. }));
        assert!(
            has_engine_move,
            "Engine should respond with a move after player's valid move"
        );
    }

    #[test]
    fn invalid_move_is_rejected_with_reason() {
        let mut session = test_session();
        select_white(&mut session);

        // Attempt an illegal move: king to e8 from starting position.
        let messages = session.handle_message(ClientMessage::MakeMove {
            from: "e1".to_string(),
            to: "e8".to_string(),
            promotion: None,
        });

        assert_eq!(messages.len(), 1);
        match &messages[0] {
            ServerMessage::MoveRejected { reason, fen } => {
                assert!(!reason.is_empty(), "Rejection reason should not be empty");
                assert!(fen.contains("rnbqkbnr"), "FEN should be the current position");
            }
            other => panic!("Expected MoveRejected, got {:?}", other),
        }

        // Session should still be InGame.
        assert_eq!(*session.session_state(), SessionState::InGame);
    }

    #[test]
    fn resign_ends_the_game() {
        let mut session = test_session();
        select_white(&mut session);

        let messages = session.handle_message(ClientMessage::Resign);

        // Should contain GameOver with result "black" (engine wins when white resigns).
        let game_over = messages
            .iter()
            .find(|m| matches!(m, ServerMessage::GameOver { .. }));
        assert!(game_over.is_some(), "Should receive GameOver on resign");

        match game_over.unwrap() {
            ServerMessage::GameOver { result, reason, .. } => {
                assert_eq!(result, "black", "Engine (black) should win when white resigns");
                assert_eq!(reason, "resignation");
            }
            _ => unreachable!(),
        }

        assert_eq!(*session.session_state(), SessionState::GameOver);
    }

    #[test]
    fn new_game_resets_to_color_selection() {
        let mut session = test_session();
        select_white(&mut session);
        assert_eq!(*session.session_state(), SessionState::InGame);

        session.handle_message(ClientMessage::NewGame);
        assert_eq!(*session.session_state(), SessionState::ColorSelection);
    }

    #[test]
    fn checkmate_ends_the_game_properly() {
        // Use a custom session where we set up scholar's mate.
        let mut session = test_session();
        select_white(&mut session);

        // Play scholar's mate: 1.e4 ... 2.Bc4 ... 3.Qh5 ... 4.Qxf7#
        // After each player move the engine responds, so we need to track that.
        // Instead, let us set up the board directly before the checkmate move.
        // We will create a session, select color, then manipulate the board to
        // the pre-checkmate position and make the final move.

        // Reset to a position just before Qxf7# (scholar's mate setup).
        // Position: r1bqkb1r/pppp1ppp/2n2n2/4p2Q/2B1P3/8/PPPP1PPP/RNB1K1NR w KQkq - 4 4
        // White plays Qxf7# to checkmate.
        let mut session = GameSession::new(EvalWeights::default_weights(), 1, false);
        session.state = SessionState::InGame;
        session.player_color = Some(Color::White);
        session.board =
            Board::from_fen("r1bqkb1r/pppp1ppp/2n2n2/4p2Q/2B1P3/8/PPPP1PPP/RNB1K1NR w KQkq - 4 4")
                .unwrap();

        let messages = session.handle_message(ClientMessage::MakeMove {
            from: "h5".to_string(),
            to: "f7".to_string(),
            promotion: None,
        });

        // Should have MoveAccepted, then GameOver (checkmate).
        let move_accepted = messages
            .iter()
            .find(|m| matches!(m, ServerMessage::MoveAccepted { .. }));
        assert!(
            move_accepted.is_some(),
            "Should accept the checkmate move"
        );

        let game_over = messages
            .iter()
            .find(|m| matches!(m, ServerMessage::GameOver { .. }));
        assert!(
            game_over.is_some(),
            "Should detect checkmate after Qxf7#"
        );

        match game_over.unwrap() {
            ServerMessage::GameOver { result, reason, .. } => {
                assert_eq!(result, "white", "White should win by checkmate");
                assert_eq!(reason, "checkmate");
            }
            _ => unreachable!(),
        }

        assert_eq!(*session.session_state(), SessionState::GameOver);
    }
}
