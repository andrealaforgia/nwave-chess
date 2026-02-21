//! Shared types for the engine module.
//! Defines GameMove, GameResult, GameState, SearchResult, and PvData
//! used across search, evaluation, and move generation.

use serde::{Deserialize, Serialize};

/// A chess move using string square representations for WebSocket protocol compatibility.
/// Internally converts to/from cozy_chess types.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GameMove {
    pub from: String,
    pub to: String,
    pub promotion: Option<String>,
}

impl GameMove {
    pub fn new(from: &str, to: &str, promotion: Option<&str>) -> Self {
        Self {
            from: from.to_string(),
            to: to.to_string(),
            promotion: promotion.map(|p| p.to_string()),
        }
    }
}

impl std::fmt::Display for GameMove {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.from, self.to)?;
        if let Some(ref promo) = self.promotion {
            write!(f, "{}", promo)?;
        }
        Ok(())
    }
}

/// The result of a completed game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameResult {
    White,
    Black,
    Draw,
}

/// The current state of a game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameState {
    InProgress,
    Check,
    Checkmate,
    Stalemate,
    DrawByRepetition,
    DrawByFiftyMove,
    DrawByInsufficientMaterial,
}

impl GameState {
    /// Returns true if the game is still ongoing (InProgress or Check).
    pub fn is_ongoing(&self) -> bool {
        matches!(self, GameState::InProgress | GameState::Check)
    }

    /// Returns the game result if the game is over, None if still ongoing.
    pub fn result(&self) -> Option<GameResult> {
        match self {
            GameState::InProgress | GameState::Check => None,
            GameState::Checkmate => None, // caller must check side_to_move to determine winner
            GameState::Stalemate
            | GameState::DrawByRepetition
            | GameState::DrawByFiftyMove
            | GameState::DrawByInsufficientMaterial => Some(GameResult::Draw),
        }
    }
}

/// Result of a search operation.
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub best_move: Option<GameMove>,
    pub evaluation: f64,
    pub pv_line: Vec<GameMove>,
    pub nodes_searched: u64,
    pub depth: u8,
}

/// Principal variation data for TD-Leaf learning.
#[derive(Debug, Clone)]
pub struct PvData {
    pub leaf_fen: String,
    pub leaf_eval: f64,
    pub gradient: Vec<f64>,
}
