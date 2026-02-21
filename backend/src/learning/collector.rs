//! Game data collection during play for TD-Leaf learning.
//! Records PV leaf positions, leaf evaluations, gradients, and opponent move predictions
//! for each move in a game. Provides the collected data to the TD-Leaf update pipeline.

use crate::engine::search::SearchLeafData;
use crate::engine::types::{GameMove, GameResult};

/// Per-move record captured during a game.
#[derive(Debug, Clone)]
pub struct MoveRecord {
    pub fen: String,
    pub leaf_eval_sigmoid: f64,
    pub leaf_eval_cp: i32,
    pub gradient: Vec<f64>,
    pub search_depth: u8,
    pub predicted_opponent_move: Option<GameMove>,
    pub actual_opponent_move: Option<GameMove>,
}

/// Collects per-move data during a game for post-game TD-Leaf processing.
pub struct GameCollector {
    moves: Vec<MoveRecord>,
    game_result: Option<GameResult>,
}

impl GameCollector {
    pub fn new() -> Self {
        Self {
            moves: Vec::new(),
            game_result: None,
        }
    }

    /// Record a move from search leaf data.
    pub fn record_move(&mut self, leaf_data: SearchLeafData) {
        self.moves.push(MoveRecord {
            fen: leaf_data.leaf_fen,
            leaf_eval_sigmoid: leaf_data.leaf_eval_sigmoid,
            leaf_eval_cp: leaf_data.leaf_eval_cp,
            gradient: leaf_data.gradient,
            search_depth: leaf_data.search_depth,
            predicted_opponent_move: leaf_data.predicted_opponent_move,
            actual_opponent_move: None,
        });
    }

    /// Set the actual opponent move for a previously recorded move.
    pub fn set_opponent_move(&mut self, move_index: usize, mv: GameMove) {
        if let Some(record) = self.moves.get_mut(move_index) {
            record.actual_opponent_move = Some(mv);
        }
    }

    /// Set the game result once the game is complete.
    pub fn set_result(&mut self, result: GameResult) {
        self.game_result = Some(result);
    }

    /// Access the recorded moves.
    pub fn moves(&self) -> &[MoveRecord] {
        &self.moves
    }

    /// Access the game result.
    pub fn game_result(&self) -> Option<GameResult> {
        self.game_result
    }

    /// Whether a game result has been set.
    pub fn is_complete(&self) -> bool {
        self.game_result.is_some()
    }

    /// Number of recorded moves.
    pub fn len(&self) -> usize {
        self.moves.len()
    }

    /// Whether no moves have been recorded.
    pub fn is_empty(&self) -> bool {
        self.moves.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::eval::NUM_WEIGHTS;

    fn make_leaf_data(sigmoid: f64, cp: i32, predicted: Option<GameMove>) -> SearchLeafData {
        SearchLeafData {
            leaf_fen: "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1".to_string(),
            leaf_eval_sigmoid: sigmoid,
            leaf_eval_cp: cp,
            gradient: vec![0.0; NUM_WEIGHTS],
            search_depth: 4,
            predicted_opponent_move: predicted,
        }
    }

    #[test]
    fn record_moves_and_retrieve_them() {
        let mut collector = GameCollector::new();
        collector.record_move(make_leaf_data(0.1, 40, None));
        collector.record_move(make_leaf_data(0.2, 80, None));
        collector.record_move(make_leaf_data(-0.05, -20, None));

        assert_eq!(collector.len(), 3);
        let moves = collector.moves();
        assert_eq!(moves[0].leaf_eval_cp, 40);
        assert_eq!(moves[1].leaf_eval_cp, 80);
        assert_eq!(moves[2].leaf_eval_cp, -20);
    }

    #[test]
    fn set_opponent_move_updates_the_record() {
        let mut collector = GameCollector::new();
        collector.record_move(make_leaf_data(0.1, 40, Some(GameMove::new("e7", "e5", None))));
        collector.record_move(make_leaf_data(0.2, 80, None));

        assert!(collector.moves()[0].actual_opponent_move.is_none());

        let actual = GameMove::new("d7", "d5", None);
        collector.set_opponent_move(0, actual.clone());

        assert_eq!(
            collector.moves()[0].actual_opponent_move.as_ref().unwrap(),
            &actual
        );
        // Second move should still be None.
        assert!(collector.moves()[1].actual_opponent_move.is_none());
    }

    #[test]
    fn collector_is_not_complete_until_result_is_set() {
        let mut collector = GameCollector::new();
        collector.record_move(make_leaf_data(0.1, 40, None));

        assert!(!collector.is_complete());
        assert!(collector.game_result().is_none());

        collector.set_result(GameResult::White);

        assert!(collector.is_complete());
        assert_eq!(collector.game_result(), Some(GameResult::White));
    }
}
