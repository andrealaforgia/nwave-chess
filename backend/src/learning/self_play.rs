//! Self-play game runner for training data generation.
//! Plays complete games with the engine on both sides, collecting TD-Leaf
//! learning data after each game. Supports cancellation via an atomic flag.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use cozy_chess::Color;
use rand::seq::SliceRandom;

use crate::engine::board::Board;
use crate::engine::eval::EvalWeights;
use crate::engine::search::SearchEngine;
use crate::engine::types::{GameResult, GameState};
use crate::learning::collector::GameCollector;
use crate::learning::optimizer::AdamOptimizer;
use crate::learning::td_leaf;
use crate::learning::weights::WeightManager;

/// Info about a single move during self-play (for live board display).
#[derive(Debug, Clone)]
pub struct SelfPlayMoveInfo {
    pub from: String,
    pub to: String,
    pub promotion: Option<String>,
    pub fen: String,
    pub move_number: u32,
}

/// Result of a single self-play game.
#[derive(Debug, Clone)]
pub struct SelfPlayGameResult {
    pub result: GameResult,
    pub reason: String,
    pub move_count: u32,
    pub avg_td_error: f64,
    pub weight_version: u32,
    pub moves: Vec<SelfPlayMoveInfo>,
}

/// Runs self-play games where the engine plays both sides.
pub struct SelfPlayRunner {
    search_engine: SearchEngine,
    weight_manager: WeightManager,
    optimizer: AdamOptimizer,
    cancel_flag: Arc<AtomicBool>,
}

/// Maximum number of moves per game to prevent infinite games.
const MAX_MOVES_PER_GAME: u32 = 300;

/// Number of half-moves at the start of each game where a random legal move
/// is played instead of searching. This creates diverse openings so self-play
/// games don't all follow the same line.
const RANDOM_OPENING_PLIES: u32 = 8;

/// Learning rate multiplier for self-play (reduced to avoid overfitting).
const SELF_PLAY_LR_MULTIPLIER: f64 = 0.3;

impl SelfPlayRunner {
    pub fn new(
        weights: EvalWeights,
        max_depth: u8,
        cancel_flag: Arc<AtomicBool>,
        search_log: bool,
    ) -> Self {
        let num_weights = EvalWeights::num_weights();
        let mut search_engine = SearchEngine::new(weights.clone(), max_depth);
        search_engine.set_verbose(search_log);
        Self {
            search_engine,
            weight_manager: WeightManager::new(weights),
            optimizer: AdamOptimizer::new(num_weights, 0.001),
            cancel_flag,
        }
    }

    /// Check if cancellation has been requested.
    pub fn is_cancelled(&self) -> bool {
        self.cancel_flag.load(Ordering::Relaxed)
    }

    /// Play a single self-play game and apply learning.
    /// Returns None if cancelled mid-game.
    pub fn run_game(&mut self) -> Option<SelfPlayGameResult> {
        let mut board = Board::new();
        let mut collector = GameCollector::new();
        self.search_engine.clear_tt();
        let mut move_count: u32 = 0;
        let mut moves = Vec::new();

        loop {
            if self.is_cancelled() {
                return None;
            }

            let game_state = board.game_state();
            if !game_state.is_ongoing() {
                let (result, reason) = terminal_result(&game_state, &board);
                collector.set_result(result);
                return self.apply_learning_and_build_result(
                    &collector, result, reason, move_count, moves,
                );
            }

            if move_count >= MAX_MOVES_PER_GAME {
                collector.set_result(GameResult::Draw);
                return self.apply_learning_and_build_result(
                    &collector,
                    GameResult::Draw,
                    "max_moves".to_string(),
                    move_count,
                    moves,
                );
            }

            // During the opening, pick a random legal move for diversity.
            // After the opening phase, use the search engine.
            let best_move = if move_count < RANDOM_OPENING_PLIES {
                let legal_moves = board.legal_moves();
                if legal_moves.is_empty() {
                    collector.set_result(GameResult::Draw);
                    return self.apply_learning_and_build_result(
                        &collector,
                        GameResult::Draw,
                        "no_moves".to_string(),
                        move_count,
                        moves,
                    );
                }
                let mut rng = rand::thread_rng();
                legal_moves.choose(&mut rng).unwrap().clone()
            } else {
                // Run search for the current side.
                let (search_result, leaf_data) = self.search_engine.search(&board, None);

                let chosen = match search_result.best_move {
                    Some(mv) => mv,
                    None => {
                        collector.set_result(GameResult::Draw);
                        return self.apply_learning_and_build_result(
                            &collector,
                            GameResult::Draw,
                            "no_moves".to_string(),
                            move_count,
                            moves,
                        );
                    }
                };

                // Record search data for White's moves (learning from White's perspective).
                if board.side_to_move() == Color::White {
                    if collector.len() > 0 && move_count >= 2 {
                        // Skip opponent tracking in self-play.
                    }
                    collector.record_move(leaf_data);
                }

                chosen
            };

            // Apply the move.
            if let Err(e) = board.make_move(&best_move) {
                tracing::error!("Self-play engine produced illegal move: {}", e);
                collector.set_result(GameResult::Draw);
                return self.apply_learning_and_build_result(
                    &collector,
                    GameResult::Draw,
                    "engine_error".to_string(),
                    move_count,
                    moves,
                );
            }

            move_count += 1;

            // Record move info for live board display.
            moves.push(SelfPlayMoveInfo {
                from: best_move.from.clone(),
                to: best_move.to.clone(),
                promotion: best_move.promotion.clone(),
                fen: board.to_fen(),
                move_number: move_count,
            });
        }
    }

    /// Apply TD-Leaf learning from a completed game and return the result.
    fn apply_learning_and_build_result(
        &mut self,
        collector: &GameCollector,
        result: GameResult,
        reason: String,
        move_count: u32,
        moves: Vec<SelfPlayMoveInfo>,
    ) -> Option<SelfPlayGameResult> {
        let mut avg_td_error = 0.0;

        if !collector.is_empty() && collector.is_complete() {
            let td_result = td_leaf::compute_td_update(collector);
            avg_td_error = td_result.avg_td_error;

            self.weight_manager.apply_update(
                &mut self.optimizer,
                &td_result.total_gradient,
                SELF_PLAY_LR_MULTIPLIER,
            );

            // Update the search engine with new weights.
            self.search_engine
                .set_weights(self.weight_manager.current_weights().clone());
        }

        Some(SelfPlayGameResult {
            result,
            reason,
            move_count,
            avg_td_error,
            weight_version: self.weight_manager.version(),
            moves,
        })
    }

    /// Get the current weight version.
    pub fn weight_version(&self) -> u32 {
        self.weight_manager.version()
    }

    /// Get the current weights (for updating the main session after self-play).
    pub fn current_weights(&self) -> &EvalWeights {
        self.weight_manager.current_weights()
    }
}

/// Determine the game result and reason string from a terminal GameState.
fn terminal_result(state: &GameState, board: &Board) -> (GameResult, String) {
    match state {
        GameState::Checkmate => {
            let loser = board.side_to_move();
            let result = match loser {
                Color::White => GameResult::Black,
                Color::Black => GameResult::White,
            };
            (result, "checkmate".to_string())
        }
        GameState::Stalemate => (GameResult::Draw, "stalemate".to_string()),
        GameState::DrawByRepetition => (GameResult::Draw, "threefold_repetition".to_string()),
        GameState::DrawByFiftyMove => (GameResult::Draw, "fifty_move".to_string()),
        GameState::DrawByInsufficientMaterial => {
            (GameResult::Draw, "insufficient_material".to_string())
        }
        _ => (GameResult::Draw, "unknown".to_string()),
    }
}

/// Convert a GameResult to a protocol-compatible string.
pub fn result_to_string(result: GameResult) -> String {
    match result {
        GameResult::White => "white".to_string(),
        GameResult::Black => "black".to_string(),
        GameResult::Draw => "draw".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn self_play_runner_completes_a_game() {
        let weights = EvalWeights::default_weights();
        let cancel = Arc::new(AtomicBool::new(false));
        let mut runner = SelfPlayRunner::new(weights, 1, cancel, false);

        let result = runner.run_game();
        assert!(result.is_some(), "Game should complete without cancellation");

        let game = result.unwrap();
        assert!(game.move_count > 0, "Game should have at least one move");
        assert!(
            matches!(game.result, GameResult::White | GameResult::Black | GameResult::Draw),
            "Game should have a valid result"
        );
    }

    #[test]
    fn self_play_runner_respects_cancellation() {
        let weights = EvalWeights::default_weights();
        let cancel = Arc::new(AtomicBool::new(true)); // Pre-cancelled
        let mut runner = SelfPlayRunner::new(weights, 1, cancel, false);

        let result = runner.run_game();
        assert!(result.is_none(), "Game should be cancelled immediately");
    }

    #[test]
    fn self_play_runner_increments_weight_version() {
        let weights = EvalWeights::default_weights();
        let cancel = Arc::new(AtomicBool::new(false));
        let mut runner = SelfPlayRunner::new(weights, 1, cancel, false);

        assert_eq!(runner.weight_version(), 0);

        let result = runner.run_game();
        assert!(result.is_some());

        // Weight version should have incremented after learning.
        assert!(
            runner.weight_version() > 0,
            "Weight version should increment after a game with learning"
        );
    }
}
