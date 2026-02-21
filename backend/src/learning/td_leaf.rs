//! TD-Leaf(lambda) temporal difference learning implementation.
//! Computes weight updates after each game using temporal differences between
//! consecutive PV leaf evaluations. Applies the KnightCap blunder filter to prevent
//! learning from opponent mistakes. Coordinates with the optimizer for weight updates.

use crate::engine::eval::NUM_WEIGHTS;
use crate::engine::types::GameResult;

use super::collector::GameCollector;

/// Lambda decay factor for TD-Leaf(lambda).
pub const TD_LAMBDA: f64 = 0.7;

/// Result of a TD-Leaf update computation.
#[derive(Debug, Clone)]
pub struct TdUpdateResult {
    /// Accumulated gradient for the weight update (same length as NUM_WEIGHTS).
    pub total_gradient: Vec<f64>,
    /// Average absolute temporal difference error across all moves.
    pub avg_td_error: f64,
    /// Maximum absolute temporal difference error.
    pub max_td_error: f64,
    /// Index of the move with the maximum TD error.
    pub max_td_error_move: usize,
    /// Number of moves in the game.
    pub num_moves: usize,
}

/// Compute TD-Leaf(lambda) weight update gradient from a completed game.
///
/// The algorithm:
/// 1. Compute temporal differences: d_t = J_{t+1} - J_t (terminal uses game result).
/// 2. Apply KnightCap blunder filter: zero out positive d_t when the opponent move
///    was not predicted (do not learn from opponent blunders).
/// 3. Compute lambda-weighted cumulative TD errors:
///    delta_t = SUM(j=t..N-1) lambda^(j-t) * d_j
/// 4. Compute total gradient: total_grad = SUM(t=0..N-1) gradient_t * delta_t
pub fn compute_td_update(collector: &GameCollector) -> TdUpdateResult {
    compute_td_update_with_lambda(collector, TD_LAMBDA)
}

/// Same as `compute_td_update` but with a configurable lambda value.
/// Useful for testing with lambda=0 to verify immediate-next-position behavior.
pub fn compute_td_update_with_lambda(collector: &GameCollector, lambda: f64) -> TdUpdateResult {
    let moves = collector.moves();
    let result = collector.game_result().expect("Game must be complete");
    let n = moves.len();

    assert!(n > 0, "Game must have at least one move");

    let terminal_value = result_to_value(result);

    // Step 1: Compute raw temporal differences d_t = J_{t+1} - J_t.
    let mut td_errors = vec![0.0f64; n];
    for t in 0..n {
        let j_t = moves[t].leaf_eval_sigmoid;
        let j_next = if t + 1 < n {
            moves[t + 1].leaf_eval_sigmoid
        } else {
            terminal_value
        };
        td_errors[t] = j_next - j_t;
    }

    // Step 2: Apply KnightCap blunder filter.
    // If d_t > 0 (position improved) AND the opponent's actual move was not the predicted one,
    // zero out the error (do not learn from opponent blunders).
    for t in 0..n {
        if td_errors[t] > 0.0 {
            let record = &moves[t];
            if let (Some(predicted), Some(actual)) = (
                &record.predicted_opponent_move,
                &record.actual_opponent_move,
            ) {
                if predicted != actual {
                    td_errors[t] = 0.0;
                }
            }
        }
    }

    // Step 3: Compute lambda-weighted cumulative TD errors.
    // delta_t = SUM(j=t..N-1) lambda^(j-t) * d_j
    // Efficient backward pass: delta_{N-1} = d_{N-1}; delta_t = d_t + lambda * delta_{t+1}.
    let mut deltas = vec![0.0f64; n];
    deltas[n - 1] = td_errors[n - 1];
    for t in (0..n - 1).rev() {
        deltas[t] = td_errors[t] + lambda * deltas[t + 1];
    }

    // Step 4: Compute total gradient.
    // total_grad = SUM(t=0..N-1) gradient_t * delta_t
    let mut total_gradient = vec![0.0f64; NUM_WEIGHTS];
    for t in 0..n {
        let delta_t = deltas[t];
        let grad = &moves[t].gradient;
        for i in 0..NUM_WEIGHTS {
            total_gradient[i] += grad[i] * delta_t;
        }
    }

    // Compute statistics.
    let abs_errors: Vec<f64> = td_errors.iter().map(|e| e.abs()).collect();
    let avg_td_error = abs_errors.iter().sum::<f64>() / n as f64;
    let (max_td_error_move, max_td_error) = abs_errors
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
        .map(|(i, &v)| (i, v))
        .unwrap_or((0, 0.0));

    TdUpdateResult {
        total_gradient,
        avg_td_error,
        max_td_error,
        max_td_error_move,
        num_moves: n,
    }
}

/// Convert a GameResult to a numeric value from White's perspective.
/// White wins = +1.0, Black wins = -1.0, Draw = 0.0.
fn result_to_value(result: GameResult) -> f64 {
    match result {
        GameResult::White => 1.0,
        GameResult::Black => -1.0,
        GameResult::Draw => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::eval::NUM_WEIGHTS;
    use crate::engine::search::SearchLeafData;
    use crate::engine::types::GameMove;
    use crate::learning::collector::GameCollector;

    fn make_leaf(sigmoid: f64, cp: i32, gradient_value: f64) -> SearchLeafData {
        SearchLeafData {
            leaf_fen: "8/8/8/8/8/8/8/8 w - - 0 1".to_string(),
            leaf_eval_sigmoid: sigmoid,
            leaf_eval_cp: cp,
            gradient: vec![gradient_value; NUM_WEIGHTS],
            search_depth: 4,
            predicted_opponent_move: None,
        }
    }

    fn make_leaf_with_prediction(
        sigmoid: f64,
        cp: i32,
        gradient_value: f64,
        predicted: Option<GameMove>,
    ) -> SearchLeafData {
        SearchLeafData {
            leaf_fen: "8/8/8/8/8/8/8/8 w - - 0 1".to_string(),
            leaf_eval_sigmoid: sigmoid,
            leaf_eval_cp: cp,
            gradient: vec![gradient_value; NUM_WEIGHTS],
            search_depth: 4,
            predicted_opponent_move: predicted,
        }
    }

    #[test]
    fn td_update_with_simple_three_move_game_produces_nonzero_gradient() {
        let mut collector = GameCollector::new();
        // Three moves with increasing sigmoid values; White wins.
        collector.record_move(make_leaf(0.1, 40, 1.0));
        collector.record_move(make_leaf(0.2, 80, 1.0));
        collector.record_move(make_leaf(0.3, 120, 1.0));
        collector.set_result(GameResult::White);

        let result = compute_td_update(&collector);

        assert_eq!(result.num_moves, 3);
        // All TD errors are positive (0.1, 0.1, 0.7) and gradient is uniform 1.0,
        // so total gradient should be nonzero.
        let norm: f64 = result
            .total_gradient
            .iter()
            .map(|g| g * g)
            .sum::<f64>()
            .sqrt();
        assert!(
            norm > 1e-10,
            "Total gradient norm should be nonzero, got {}",
            norm
        );
        assert!(result.avg_td_error > 0.0);
    }

    #[test]
    fn knightcap_blunder_filter_zeros_out_positive_td_errors_when_opponent_not_predicted() {
        let mut collector = GameCollector::new();

        // Move 0: predicted e7e5, actual d7d5 (mispredicted). sigmoid 0.1
        let predicted = GameMove::new("e7", "e5", None);
        collector.record_move(make_leaf_with_prediction(0.1, 40, 1.0, Some(predicted)));
        let actual = GameMove::new("d7", "d5", None);
        collector.set_opponent_move(0, actual);

        // Move 1: no prediction needed (last move). sigmoid 0.5
        collector.record_move(make_leaf(0.5, 200, 1.0));

        // White wins, so terminal = 1.0
        collector.set_result(GameResult::White);

        let result = compute_td_update(&collector);

        // d_0 = 0.5 - 0.1 = 0.4 (positive) and mispredicted -> zeroed by blunder filter
        // d_1 = 1.0 - 0.5 = 0.5 (positive) but no prediction info -> kept as-is
        //
        // Now compare with a game where the opponent move WAS predicted.
        let mut collector_no_filter = GameCollector::new();
        let predicted2 = GameMove::new("e7", "e5", None);
        collector_no_filter
            .record_move(make_leaf_with_prediction(0.1, 40, 1.0, Some(predicted2)));
        let actual2 = GameMove::new("e7", "e5", None); // Same as predicted
        collector_no_filter.set_opponent_move(0, actual2);
        collector_no_filter.record_move(make_leaf(0.5, 200, 1.0));
        collector_no_filter.set_result(GameResult::White);

        let result_no_filter = compute_td_update(&collector_no_filter);

        // With the blunder filter, the gradient should be smaller in magnitude
        // because d_0 was zeroed.
        let norm_filtered: f64 = result
            .total_gradient
            .iter()
            .map(|g| g * g)
            .sum::<f64>()
            .sqrt();
        let norm_unfiltered: f64 = result_no_filter
            .total_gradient
            .iter()
            .map(|g| g * g)
            .sum::<f64>()
            .sqrt();

        assert!(
            norm_filtered < norm_unfiltered,
            "Blunder-filtered gradient norm ({}) should be smaller than unfiltered ({})",
            norm_filtered,
            norm_unfiltered
        );
    }

    #[test]
    fn lambda_zero_only_considers_immediate_next_position() {
        let mut collector = GameCollector::new();
        // 3 moves: sigmas 0.0, 0.0, 0.0; White wins (terminal = 1.0)
        // With lambda=0: delta_t = d_t (no propagation from future).
        // d_0 = 0.0 - 0.0 = 0.0
        // d_1 = 0.0 - 0.0 = 0.0
        // d_2 = 1.0 - 0.0 = 1.0
        //
        // With lambda=0: delta_t = d_t only.
        // So only move 2 contributes to the gradient.
        // Use distinct gradient values per move to verify.
        let mut leaf0 = make_leaf(0.0, 0, 0.0);
        leaf0.gradient = vec![1.0; NUM_WEIGHTS]; // Move 0's gradient
        let mut leaf1 = make_leaf(0.0, 0, 0.0);
        leaf1.gradient = vec![2.0; NUM_WEIGHTS]; // Move 1's gradient
        let mut leaf2 = make_leaf(0.0, 0, 0.0);
        leaf2.gradient = vec![3.0; NUM_WEIGHTS]; // Move 2's gradient

        collector.record_move(leaf0);
        collector.record_move(leaf1);
        collector.record_move(leaf2);
        collector.set_result(GameResult::White);

        let result = compute_td_update_with_lambda(&collector, 0.0);

        // With lambda=0: delta_0 = 0.0, delta_1 = 0.0, delta_2 = 1.0
        // total_grad[i] = 1.0 * 0.0 + 2.0 * 0.0 + 3.0 * 1.0 = 3.0
        for i in 0..NUM_WEIGHTS {
            assert!(
                (result.total_gradient[i] - 3.0).abs() < 1e-10,
                "Gradient[{}] should be 3.0 (only move 2 contributes), got {}",
                i,
                result.total_gradient[i]
            );
        }
    }

    #[test]
    fn terminal_correction_uses_game_result() {
        // Single-move game: sigmoid = 0.5, White wins -> terminal = 1.0
        let mut collector_white = GameCollector::new();
        collector_white.record_move(make_leaf(0.5, 200, 1.0));
        collector_white.set_result(GameResult::White);
        let result_white = compute_td_update(&collector_white);

        // Single-move game: sigmoid = 0.5, Black wins -> terminal = -1.0
        let mut collector_black = GameCollector::new();
        collector_black.record_move(make_leaf(0.5, 200, 1.0));
        collector_black.set_result(GameResult::Black);
        let result_black = compute_td_update(&collector_black);

        // White wins: d_0 = 1.0 - 0.5 = 0.5 -> delta_0 = 0.5 -> gradient = 0.5
        // Black wins: d_0 = -1.0 - 0.5 = -1.5 -> delta_0 = -1.5 -> gradient = -1.5
        assert!(
            result_white.total_gradient[0] > 0.0,
            "White win should produce positive gradient"
        );
        assert!(
            result_black.total_gradient[0] < 0.0,
            "Black win should produce negative gradient"
        );

        // Verify exact values: gradient_value is 1.0, so total_gradient = delta * 1.0
        let expected_white = 0.5; // 1.0 - 0.5
        let expected_black = -1.5; // -1.0 - 0.5
        assert!(
            (result_white.total_gradient[0] - expected_white).abs() < 1e-10,
            "White win gradient should be {}, got {}",
            expected_white,
            result_white.total_gradient[0]
        );
        assert!(
            (result_black.total_gradient[0] - expected_black).abs() < 1e-10,
            "Black win gradient should be {}, got {}",
            expected_black,
            result_black.total_gradient[0]
        );
    }
}
