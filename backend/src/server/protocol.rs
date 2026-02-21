//! WebSocket message protocol types.
//! Defines serde-serializable structs for all client-to-server and server-to-client
//! messages including game commands, search progress, and learning updates.

use serde::{Deserialize, Serialize};

// === Client-to-Server Messages ===

/// All messages the client can send to the server.
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    #[serde(rename = "select_color")]
    SelectColor { color: String },

    #[serde(rename = "make_move")]
    MakeMove {
        from: String,
        to: String,
        promotion: Option<String>,
    },

    #[serde(rename = "resign")]
    Resign,

    #[serde(rename = "new_game")]
    NewGame,

    #[serde(rename = "request_learning_status")]
    RequestLearningStatus,

    #[serde(rename = "start_self_play")]
    StartSelfPlay { num_games: u32 },

    #[serde(rename = "cancel_self_play")]
    CancelSelfPlay,
}

// === Server-to-Client Messages ===

/// All messages the server can send to the client.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    #[serde(rename = "game_started")]
    GameStarted {
        fen: String,
        player_color: String,
        engine_color: String,
        weight_version: u32,
    },

    #[serde(rename = "move_accepted")]
    MoveAccepted { from: String, to: String, fen: String },

    #[serde(rename = "move_rejected")]
    MoveRejected { reason: String, fen: String },

    #[serde(rename = "engine_thinking")]
    EngineThinking {
        depth: u8,
        evaluation_cp: i32,
        best_move: String,
        pv_line: Vec<String>,
        nodes: u64,
        time_ms: u64,
    },

    #[serde(rename = "engine_move")]
    EngineMoveMsg {
        from: String,
        to: String,
        promotion: Option<String>,
        fen: String,
        evaluation_cp: i32,
        search_depth: u8,
        nodes_searched: u64,
    },

    #[serde(rename = "game_over")]
    GameOver {
        result: String,
        reason: String,
        fen: String,
    },

    #[serde(rename = "learning_update")]
    LearningUpdate {
        game_id: i64,
        avg_td_error: f64,
        max_td_error: f64,
        weight_change_norm: f64,
        weight_version: u32,
    },

    #[serde(rename = "self_play_move")]
    SelfPlayMove {
        game_number: u32,
        total_games: u32,
        move_number: u32,
        from: String,
        to: String,
        promotion: Option<String>,
        fen: String,
    },

    #[serde(rename = "self_play_progress")]
    SelfPlayProgress {
        game_number: u32,
        total_games: u32,
        result: String,
        reason: String,
        moves: u32,
        avg_td_error: f64,
        weight_version: u32,
    },

    #[serde(rename = "self_play_complete")]
    SelfPlayComplete {
        total_games: u32,
        white_wins: u32,
        black_wins: u32,
        draws: u32,
        weight_version: u32,
    },

    #[serde(rename = "error")]
    Error { code: String, message: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_message_deserialization_from_json() {
        // select_color
        let json = r#"{"type":"select_color","color":"white"}"#;
        let msg: ClientMessage = serde_json::from_str(json).unwrap();
        match msg {
            ClientMessage::SelectColor { color } => assert_eq!(color, "white"),
            _ => panic!("Expected SelectColor"),
        }

        // make_move with promotion
        let json = r#"{"type":"make_move","from":"e7","to":"e8","promotion":"q"}"#;
        let msg: ClientMessage = serde_json::from_str(json).unwrap();
        match msg {
            ClientMessage::MakeMove {
                from,
                to,
                promotion,
            } => {
                assert_eq!(from, "e7");
                assert_eq!(to, "e8");
                assert_eq!(promotion, Some("q".to_string()));
            }
            _ => panic!("Expected MakeMove"),
        }

        // make_move without promotion
        let json = r#"{"type":"make_move","from":"e2","to":"e4","promotion":null}"#;
        let msg: ClientMessage = serde_json::from_str(json).unwrap();
        match msg {
            ClientMessage::MakeMove { promotion, .. } => {
                assert_eq!(promotion, None);
            }
            _ => panic!("Expected MakeMove"),
        }

        // resign
        let json = r#"{"type":"resign"}"#;
        let msg: ClientMessage = serde_json::from_str(json).unwrap();
        assert!(matches!(msg, ClientMessage::Resign));

        // new_game
        let json = r#"{"type":"new_game"}"#;
        let msg: ClientMessage = serde_json::from_str(json).unwrap();
        assert!(matches!(msg, ClientMessage::NewGame));

        // request_learning_status
        let json = r#"{"type":"request_learning_status"}"#;
        let msg: ClientMessage = serde_json::from_str(json).unwrap();
        assert!(matches!(msg, ClientMessage::RequestLearningStatus));

        // start_self_play
        let json = r#"{"type":"start_self_play","num_games":10}"#;
        let msg: ClientMessage = serde_json::from_str(json).unwrap();
        match msg {
            ClientMessage::StartSelfPlay { num_games } => assert_eq!(num_games, 10),
            _ => panic!("Expected StartSelfPlay"),
        }

        // cancel_self_play
        let json = r#"{"type":"cancel_self_play"}"#;
        let msg: ClientMessage = serde_json::from_str(json).unwrap();
        assert!(matches!(msg, ClientMessage::CancelSelfPlay));
    }

    #[test]
    fn server_message_serialization_to_json() {
        // game_started
        let msg = ServerMessage::GameStarted {
            fen: "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1".to_string(),
            player_color: "white".to_string(),
            engine_color: "black".to_string(),
            weight_version: 42,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""type":"game_started""#));
        assert!(json.contains(r#""player_color":"white""#));
        assert!(json.contains(r#""weight_version":42"#));

        // engine_move
        let msg = ServerMessage::EngineMoveMsg {
            from: "e2".to_string(),
            to: "e4".to_string(),
            promotion: None,
            fen: "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1".to_string(),
            evaluation_cp: 35,
            search_depth: 6,
            nodes_searched: 1247832,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""type":"engine_move""#));
        assert!(json.contains(r#""from":"e2""#));
        assert!(json.contains(r#""evaluation_cp":35"#));

        // game_over
        let msg = ServerMessage::GameOver {
            result: "white".to_string(),
            reason: "checkmate".to_string(),
            fen: "some_fen".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""type":"game_over""#));
        assert!(json.contains(r#""result":"white""#));
        assert!(json.contains(r#""reason":"checkmate""#));

        // error
        let msg = ServerMessage::Error {
            code: "invalid_message".to_string(),
            message: "Unknown type".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""type":"error""#));
        assert!(json.contains(r#""code":"invalid_message""#));

        // learning_update
        let msg = ServerMessage::LearningUpdate {
            game_id: 15,
            avg_td_error: 0.087,
            max_td_error: 0.342,
            weight_change_norm: 0.045,
            weight_version: 43,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""type":"learning_update""#));
        assert!(json.contains(r#""game_id":15"#));

        // self_play_move
        let msg = ServerMessage::SelfPlayMove {
            game_number: 2,
            total_games: 10,
            move_number: 5,
            from: "e2".to_string(),
            to: "e4".to_string(),
            promotion: None,
            fen: "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""type":"self_play_move""#));
        assert!(json.contains(r#""game_number":2"#));
        assert!(json.contains(r#""move_number":5"#));
        assert!(json.contains(r#""from":"e2""#));

        // self_play_progress
        let msg = ServerMessage::SelfPlayProgress {
            game_number: 3,
            total_games: 10,
            result: "white".to_string(),
            reason: "checkmate".to_string(),
            moves: 42,
            avg_td_error: 0.05,
            weight_version: 5,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""type":"self_play_progress""#));
        assert!(json.contains(r#""game_number":3"#));
        assert!(json.contains(r#""total_games":10"#));

        // self_play_complete
        let msg = ServerMessage::SelfPlayComplete {
            total_games: 10,
            white_wins: 4,
            black_wins: 3,
            draws: 3,
            weight_version: 12,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""type":"self_play_complete""#));
        assert!(json.contains(r#""white_wins":4"#));
    }
}
