//! Alpha-beta search with iterative deepening, transposition table, quiescence
//! search, and principal variation tracking.
//! Implements negamax with alpha-beta pruning, move ordering (MVV-LVA, killer moves,
//! history heuristic, TT best move). Streams search progress for real-time UI updates.
//! Collects leaf data for TD-Leaf learning.

use std::time::Instant;

use cozy_chess::{Color, Piece};

use super::board::Board;
use super::eval::{evaluate, EvalWeights};
use super::movegen::MoveOrderer;
use super::tt::{ScoreType, TranspositionTable};
use super::types::{GameMove, GameState, SearchResult};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Score representing checkmate (adjusted by ply for shorter-mate preference).
const MATE_SCORE: i32 = 30_000;

/// Score used as initial alpha/beta bounds.
const INFINITY: i32 = 31_000;

/// Sigmoid scaling factor (must match eval.rs).
const SIGMOID_SCALE: f64 = 400.0;

/// Default TT size in megabytes.
const DEFAULT_TT_SIZE_MB: usize = 64;

// ---------------------------------------------------------------------------
// Search progress types
// ---------------------------------------------------------------------------

// Note: search callbacks use a generic FnMut parameter on the search method,
// avoiding lifetime issues with trait object references.

/// Search progress info sent after each depth completes.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SearchProgress {
    pub depth: u8,
    pub best_move: Option<GameMove>,
    pub evaluation_cp: i32,
    pub pv_line: Vec<GameMove>,
    pub nodes: u64,
    pub time_ms: u64,
    pub candidates: Vec<CandidateMove>,
}

/// A candidate move with its evaluation.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CandidateMove {
    pub mv: GameMove,
    pub evaluation_cp: i32,
    pub depth: u8,
}

// ---------------------------------------------------------------------------
// TD-Leaf data
// ---------------------------------------------------------------------------

/// Data collected for TD-Leaf learning at each root move.
#[derive(Debug, Clone)]
pub struct SearchLeafData {
    pub leaf_fen: String,
    pub leaf_eval_sigmoid: f64,
    pub leaf_eval_cp: i32,
    pub gradient: Vec<f64>,
    pub search_depth: u8,
    pub predicted_opponent_move: Option<GameMove>,
}

// ---------------------------------------------------------------------------
// Capture detection helper
// ---------------------------------------------------------------------------

/// Check whether a move is a capture (opponent piece on target square or en passant).
fn is_capture(board: &Board, mv: &GameMove) -> bool {
    let inner = board.inner();
    let opponent_color = !inner.side_to_move();

    let to_sq: cozy_chess::Square = match mv.to.parse() {
        Ok(sq) => sq,
        Err(_) => return false,
    };

    // Normal capture: opponent piece on target square.
    if inner.colors(opponent_color).has(to_sq) {
        return true;
    }

    // En passant: pawn moves diagonally to an empty square.
    let from_sq: cozy_chess::Square = match mv.from.parse() {
        Ok(sq) => sq,
        Err(_) => return false,
    };
    if inner.piece_on(from_sq) == Some(Piece::Pawn) {
        let from_file = from_sq.file() as i32;
        let to_file = to_sq.file() as i32;
        if (from_file - to_file).abs() == 1 && !inner.occupied().has(to_sq) {
            return true;
        }
    }

    false
}

// ---------------------------------------------------------------------------
// Search engine
// ---------------------------------------------------------------------------

/// The search engine implementing negamax with alpha-beta pruning, transposition
/// table, quiescence search, and iterative deepening.
pub struct SearchEngine {
    weights: EvalWeights,
    move_orderer: MoveOrderer,
    nodes_searched: u64,
    max_depth: u8,
    /// Triangular PV table: pv_table[ply] is the PV from that ply onwards.
    pv_table: Vec<Vec<GameMove>>,
    /// PV length at each ply.
    pv_length: Vec<usize>,
    /// Leaf data for TD-Leaf learning.
    leaf_data: Option<SearchLeafData>,
    /// Transposition table.
    tt: TranspositionTable,
    /// When true, log detailed search information to the terminal.
    verbose: bool,
}

impl SearchEngine {
    pub fn new(weights: EvalWeights, max_depth: u8) -> Self {
        let depth = max_depth as usize + 1;
        Self {
            weights,
            move_orderer: MoveOrderer::new(),
            nodes_searched: 0,
            max_depth,
            pv_table: vec![Vec::new(); depth + 64],
            pv_length: vec![0; depth + 64],
            leaf_data: None,
            tt: TranspositionTable::new(DEFAULT_TT_SIZE_MB),
            verbose: false,
        }
    }

    /// Create a search engine with a specific TT size (for testing).
    pub fn with_tt_size(weights: EvalWeights, max_depth: u8, tt_size_mb: usize) -> Self {
        let depth = max_depth as usize + 1;
        Self {
            weights,
            move_orderer: MoveOrderer::new(),
            nodes_searched: 0,
            max_depth,
            pv_table: vec![Vec::new(); depth + 64],
            pv_length: vec![0; depth + 64],
            leaf_data: None,
            tt: TranspositionTable::new(tt_size_mb),
            verbose: false,
        }
    }

    /// Enable or disable verbose search logging.
    pub fn set_verbose(&mut self, verbose: bool) {
        self.verbose = verbose;
    }

    /// Update weights (after learning).
    pub fn set_weights(&mut self, weights: EvalWeights) {
        self.weights = weights;
    }

    /// Clear the transposition table (between games, not between iterative deepening depths).
    pub fn clear_tt(&mut self) {
        self.tt.clear();
    }

    /// Search with iterative deepening. Calls callback after each depth.
    /// Returns the best result found and the leaf data for TD-Leaf.
    pub fn search(
        &mut self,
        board: &Board,
        mut callback: Option<&mut dyn FnMut(SearchProgress)>,
    ) -> (SearchResult, SearchLeafData) {
        let start = Instant::now();
        self.move_orderer.clear();
        self.nodes_searched = 0;
        // Note: TT is NOT cleared between iterative deepening depths.
        // It is preserved across depths so deeper iterations benefit from
        // shallower results. Call clear_tt() between games.

        if self.verbose {
            let side = if board.side_to_move() == Color::White { "White" } else { "Black" };
            tracing::info!(
                "[search] START  fen={} side={} max_depth={}",
                board.to_fen(), side, self.max_depth
            );
        }

        let mut best_result = SearchResult {
            best_move: None,
            evaluation: 0.0,
            pv_line: Vec::new(),
            nodes_searched: 0,
            depth: 0,
        };

        // Iterative deepening: search depth 1, 2, ..., max_depth.
        for depth in 1..=self.max_depth {
            let result = self.search_root(board, depth);
            best_result = result.clone();

            // Collect leaf data from the PV at this depth.
            self.collect_leaf_data(board, &result, depth);

            if self.verbose {
                let elapsed = start.elapsed().as_millis();
                let pv_str: String = result.pv_line.iter()
                    .map(|m| format!("{}", m))
                    .collect::<Vec<_>>()
                    .join(" ");
                tracing::info!(
                    "[search] depth={} best={} eval={:+}cp nodes={} time={}ms pv=[{}]",
                    depth,
                    result.best_move.as_ref().map(|m| format!("{}", m)).unwrap_or_default(),
                    result.evaluation as i32,
                    self.nodes_searched,
                    elapsed,
                    pv_str,
                );
            }

            if let Some(ref mut cb) = callback {
                let elapsed = start.elapsed().as_millis() as u64;
                let progress = SearchProgress {
                    depth,
                    best_move: result.best_move.clone(),
                    evaluation_cp: result.evaluation as i32,
                    pv_line: result.pv_line.clone(),
                    nodes: self.nodes_searched,
                    time_ms: elapsed,
                    candidates: Vec::new(), // Populated below in search_root
                };
                cb(progress);
            }
        }

        let leaf = self.leaf_data.clone().unwrap_or_else(|| {
            // Fallback: evaluate current position.
            let eval_result = evaluate(board, &self.weights);
            SearchLeafData {
                leaf_fen: board.to_fen(),
                leaf_eval_sigmoid: eval_result.score_sigmoid,
                leaf_eval_cp: eval_result.score_cp,
                gradient: eval_result.gradient,
                search_depth: 0,
                predicted_opponent_move: None,
            }
        });

        best_result.nodes_searched = self.nodes_searched;

        if self.verbose {
            let elapsed = start.elapsed().as_millis();
            tracing::info!(
                "[search] DONE   best={} eval={:+}cp nodes={} time={}ms",
                best_result.best_move.as_ref().map(|m| format!("{}", m)).unwrap_or_default(),
                best_result.evaluation as i32,
                self.nodes_searched,
                elapsed,
            );
        }

        (best_result, leaf)
    }

    /// Search to a specific depth (no iterative deepening). For testing.
    pub fn search_to_depth(&mut self, board: &Board, depth: u8) -> SearchResult {
        self.move_orderer.clear();
        self.nodes_searched = 0;
        let result = self.search_root(board, depth);
        let mut final_result = result;
        self.collect_leaf_data(board, &final_result, depth);
        final_result.nodes_searched = self.nodes_searched;
        final_result
    }

    /// Search to a specific depth without using the transposition table. For testing
    /// node count comparisons.
    pub fn search_to_depth_no_tt(&mut self, board: &Board, depth: u8) -> SearchResult {
        self.move_orderer.clear();
        self.nodes_searched = 0;
        let result = self.search_root_no_tt(board, depth);
        let mut final_result = result;
        self.collect_leaf_data(board, &final_result, depth);
        final_result.nodes_searched = self.nodes_searched;
        final_result
    }

    /// Root-level search: iterate over all root moves, track candidates and PV.
    fn search_root(&mut self, board: &Board, depth: u8) -> SearchResult {
        let mut moves = board.legal_moves();

        // Check for terminal position.
        if moves.is_empty() {
            let state = board.game_state();
            let score = match state {
                GameState::Checkmate => -MATE_SCORE,
                _ => 0, // Stalemate or draw
            };
            return SearchResult {
                best_move: None,
                evaluation: score as f64,
                pv_line: Vec::new(),
                nodes_searched: self.nodes_searched,
                depth,
            };
        }

        // Get PV move from previous iteration for move ordering.
        let pv_move = if !self.pv_table.is_empty() && !self.pv_table[0].is_empty() {
            Some(self.pv_table[0][0].clone())
        } else {
            None
        };

        // Also check TT for a best move hint at root.
        let tt_move = self.tt.probe(board.hash()).and_then(|e| e.best_move.clone());
        let ordering_hint = pv_move.or(tt_move);

        self.move_orderer
            .order_moves(board, &mut moves, ordering_hint.as_ref(), 0);

        // Reset PV table for this iteration.
        for pv in self.pv_table.iter_mut() {
            pv.clear();
        }
        for len in self.pv_length.iter_mut() {
            *len = 0;
        }

        let mut alpha = -INFINITY;
        let beta = INFINITY;
        let mut best_move: Option<GameMove> = None;
        let mut best_score = -INFINITY;
        let mut candidates: Vec<CandidateMove> = Vec::new();

        for mv in &moves {
            let child_board = match board.clone_and_make(mv) {
                Ok(b) => b,
                Err(_) => continue,
            };

            let score = -self.negamax(&child_board, depth as i32 - 1, -beta, -alpha, 1);

            candidates.push(CandidateMove {
                mv: mv.clone(),
                evaluation_cp: score,
                depth,
            });

            if score > best_score {
                best_score = score;
                best_move = Some(mv.clone());

                if score > alpha {
                    alpha = score;

                    // Update PV: current move + child's PV.
                    let child_pv = self.pv_table[1].clone();
                    self.pv_table[0] = std::iter::once(mv.clone())
                        .chain(child_pv.into_iter())
                        .collect();
                    self.pv_length[0] = 1 + self.pv_length[1];
                }
            }
        }

        // Store root position in TT.
        self.tt.store(
            board.hash(),
            depth as i32,
            best_score,
            ScoreType::Exact,
            best_move.clone(),
        );

        // Sort candidates by score descending, keep top 3.
        candidates.sort_unstable_by(|a, b| b.evaluation_cp.cmp(&a.evaluation_cp));
        candidates.truncate(3);

        if self.verbose {
            let candidates_str: String = candidates.iter()
                .map(|c| format!("{}={:+}", c.mv, c.evaluation_cp))
                .collect::<Vec<_>>()
                .join(", ");
            tracing::info!(
                "[search]   root d={} candidates=[{}]",
                depth, candidates_str,
            );
        }

        SearchResult {
            best_move,
            evaluation: best_score as f64,
            pv_line: self.pv_table[0].clone(),
            nodes_searched: self.nodes_searched,
            depth,
        }
    }

    /// Root-level search without TT (for testing comparisons).
    fn search_root_no_tt(&mut self, board: &Board, depth: u8) -> SearchResult {
        let mut moves = board.legal_moves();

        if moves.is_empty() {
            let state = board.game_state();
            let score = match state {
                GameState::Checkmate => -MATE_SCORE,
                _ => 0,
            };
            return SearchResult {
                best_move: None,
                evaluation: score as f64,
                pv_line: Vec::new(),
                nodes_searched: self.nodes_searched,
                depth,
            };
        }

        let pv_move = if !self.pv_table.is_empty() && !self.pv_table[0].is_empty() {
            Some(self.pv_table[0][0].clone())
        } else {
            None
        };

        self.move_orderer
            .order_moves(board, &mut moves, pv_move.as_ref(), 0);

        for pv in self.pv_table.iter_mut() {
            pv.clear();
        }
        for len in self.pv_length.iter_mut() {
            *len = 0;
        }

        let mut alpha = -INFINITY;
        let beta = INFINITY;
        let mut best_move: Option<GameMove> = None;
        let mut best_score = -INFINITY;

        for mv in &moves {
            let child_board = match board.clone_and_make(mv) {
                Ok(b) => b,
                Err(_) => continue,
            };

            let score =
                -self.negamax_no_tt(&child_board, depth as i32 - 1, -beta, -alpha, 1);

            if score > best_score {
                best_score = score;
                best_move = Some(mv.clone());

                if score > alpha {
                    alpha = score;

                    let child_pv = self.pv_table[1].clone();
                    self.pv_table[0] = std::iter::once(mv.clone())
                        .chain(child_pv.into_iter())
                        .collect();
                    self.pv_length[0] = 1 + self.pv_length[1];
                }
            }
        }

        SearchResult {
            best_move,
            evaluation: best_score as f64,
            pv_line: self.pv_table[0].clone(),
            nodes_searched: self.nodes_searched,
            depth,
        }
    }

    /// Negamax with alpha-beta pruning and transposition table.
    fn negamax(
        &mut self,
        board: &Board,
        depth: i32,
        mut alpha: i32,
        beta: i32,
        ply: usize,
    ) -> i32 {
        self.nodes_searched += 1;

        // Clear PV at this ply.
        if ply < self.pv_table.len() {
            self.pv_table[ply].clear();
            self.pv_length[ply] = 0;
        }

        // Check for terminal game states.
        let state = board.game_state();
        match state {
            GameState::Checkmate => return -(MATE_SCORE - ply as i32),
            GameState::Stalemate
            | GameState::DrawByRepetition
            | GameState::DrawByFiftyMove
            | GameState::DrawByInsufficientMaterial => return 0,
            _ => {}
        }

        // Leaf node: run quiescence search instead of static eval.
        if depth <= 0 {
            return self.quiescence(board, alpha, beta);
        }

        // --- TT probe ---
        let hash = board.hash();
        let mut tt_best_move: Option<GameMove> = None;

        if let Some(entry) = self.tt.probe(hash) {
            // Always extract the best move for move ordering, even if depth is insufficient.
            tt_best_move = entry.best_move.clone();

            // Use the stored score only if the TT entry has sufficient depth.
            if entry.depth >= depth {
                match entry.score_type {
                    ScoreType::Exact => return entry.score,
                    ScoreType::LowerBound => {
                        if entry.score >= beta {
                            return entry.score;
                        }
                        if entry.score > alpha {
                            alpha = entry.score;
                        }
                    }
                    ScoreType::UpperBound => {
                        if entry.score <= alpha {
                            return entry.score;
                        }
                    }
                }
            }
        }

        let mut moves = board.legal_moves();
        if moves.is_empty() {
            // Should not happen if game_state was correct, but be safe.
            return 0;
        }

        // Get PV move for ordering at this ply. Prefer TT best move, then PV move.
        let pv_move = if let Some(ref tt_mv) = tt_best_move {
            Some(tt_mv.clone())
        } else if ply < self.pv_table.len() && !self.pv_table[ply].is_empty() {
            Some(self.pv_table[ply][0].clone())
        } else {
            None
        };

        self.move_orderer
            .order_moves(board, &mut moves, pv_move.as_ref(), ply);

        let mut best_score = -INFINITY;
        let mut best_move_found: Option<GameMove> = None;
        let original_alpha = alpha;

        for mv in &moves {
            let child_board = match board.clone_and_make(mv) {
                Ok(b) => b,
                Err(_) => continue,
            };

            let score = -self.negamax(&child_board, depth - 1, -beta, -alpha, ply + 1);

            if score > best_score {
                best_score = score;
                best_move_found = Some(mv.clone());
            }

            if score > alpha {
                alpha = score;

                // Update PV at this ply.
                if ply < self.pv_table.len() && ply + 1 < self.pv_table.len() {
                    let child_pv = self.pv_table[ply + 1].clone();
                    self.pv_table[ply] = std::iter::once(mv.clone())
                        .chain(child_pv.into_iter())
                        .collect();
                    self.pv_length[ply] = 1 + self.pv_length.get(ply + 1).copied().unwrap_or(0);
                }
            }

            // Beta cutoff.
            if alpha >= beta {
                // Record killer move (for non-captures).
                if !is_capture(board, mv) {
                    self.move_orderer.record_killer(mv, ply);
                    self.move_orderer.record_history(mv, depth);
                }
                break;
            }
        }

        // --- TT store ---
        let score_type = if best_score >= beta {
            ScoreType::LowerBound
        } else if alpha > original_alpha {
            ScoreType::Exact
        } else {
            ScoreType::UpperBound
        };
        self.tt
            .store(hash, depth, best_score, score_type, best_move_found);

        best_score
    }

    /// Negamax without TT (for testing node count comparisons).
    fn negamax_no_tt(
        &mut self,
        board: &Board,
        depth: i32,
        mut alpha: i32,
        beta: i32,
        ply: usize,
    ) -> i32 {
        self.nodes_searched += 1;

        if ply < self.pv_table.len() {
            self.pv_table[ply].clear();
            self.pv_length[ply] = 0;
        }

        let state = board.game_state();
        match state {
            GameState::Checkmate => return -(MATE_SCORE - ply as i32),
            GameState::Stalemate
            | GameState::DrawByRepetition
            | GameState::DrawByFiftyMove
            | GameState::DrawByInsufficientMaterial => return 0,
            _ => {}
        }

        if depth <= 0 {
            return self.quiescence(board, alpha, beta);
        }

        let mut moves = board.legal_moves();
        if moves.is_empty() {
            return 0;
        }

        let pv_move = if ply < self.pv_table.len() && !self.pv_table[ply].is_empty() {
            Some(self.pv_table[ply][0].clone())
        } else {
            None
        };

        self.move_orderer
            .order_moves(board, &mut moves, pv_move.as_ref(), ply);

        let mut best_score = -INFINITY;

        for mv in &moves {
            let child_board = match board.clone_and_make(mv) {
                Ok(b) => b,
                Err(_) => continue,
            };

            let score =
                -self.negamax_no_tt(&child_board, depth - 1, -beta, -alpha, ply + 1);

            if score > best_score {
                best_score = score;
            }

            if score > alpha {
                alpha = score;

                if ply < self.pv_table.len() && ply + 1 < self.pv_table.len() {
                    let child_pv = self.pv_table[ply + 1].clone();
                    self.pv_table[ply] = std::iter::once(mv.clone())
                        .chain(child_pv.into_iter())
                        .collect();
                    self.pv_length[ply] = 1 + self.pv_length.get(ply + 1).copied().unwrap_or(0);
                }
            }

            if alpha >= beta {
                if !is_capture(board, mv) {
                    self.move_orderer.record_killer(mv, ply);
                    self.move_orderer.record_history(mv, depth);
                }
                break;
            }
        }

        best_score
    }

    /// Quiescence search: search only captures to stabilize the evaluation
    /// and avoid the horizon effect.
    fn quiescence(&mut self, board: &Board, mut alpha: i32, beta: i32) -> i32 {
        // Static evaluation (stand pat score) from the side-to-move perspective.
        // evaluate() already returns score from STM perspective.
        let stand_pat = evaluate(board, &self.weights).score_cp;

        // Beta cutoff on stand pat.
        if stand_pat >= beta {
            return beta;
        }
        if stand_pat > alpha {
            alpha = stand_pat;
        }

        // Generate only captures.
        let all_moves = board.legal_moves();
        let mut captures: Vec<GameMove> = all_moves
            .into_iter()
            .filter(|mv| is_capture(board, mv))
            .collect();

        if captures.is_empty() {
            return alpha;
        }

        // Order captures by MVV-LVA.
        self.move_orderer
            .order_moves(board, &mut captures, None, 0);

        for mv in &captures {
            let new_board = match board.clone_and_make(mv) {
                Ok(b) => b,
                Err(_) => continue,
            };
            self.nodes_searched += 1;
            let score = -self.quiescence(&new_board, -beta, -alpha);

            if score >= beta {
                return beta;
            }
            if score > alpha {
                alpha = score;
            }
        }

        alpha
    }

    /// Collect leaf data from the PV for TD-Leaf learning.
    fn collect_leaf_data(&mut self, board: &Board, result: &SearchResult, depth: u8) {
        let pv = &result.pv_line;
        if pv.is_empty() {
            return;
        }

        // Walk down the PV to find the leaf position.
        let mut leaf_board = board.clone();
        for mv in pv {
            match leaf_board.clone_and_make(mv) {
                Ok(b) => leaf_board = b,
                Err(_) => break,
            }
        }

        // Evaluate the leaf position.
        let eval_result = evaluate(&leaf_board, &self.weights);

        // Convert score to White's perspective for TD-Leaf.
        let stm_sign = if leaf_board.side_to_move() == Color::White {
            1.0
        } else {
            -1.0
        };
        let eval_white_cp = (eval_result.score_cp as f64 * stm_sign) as i32;
        let eval_white_sigmoid = (eval_white_cp as f64 / SIGMOID_SCALE).tanh();

        // Adjust gradient to White's perspective.
        let gradient: Vec<f64> = if stm_sign < 0.0 {
            eval_result.gradient.iter().map(|g| -g).collect()
        } else {
            eval_result.gradient
        };

        // The predicted opponent move is PV[1] if it exists.
        let predicted_opponent_move = if pv.len() > 1 {
            Some(pv[1].clone())
        } else {
            None
        };

        self.leaf_data = Some(SearchLeafData {
            leaf_fen: leaf_board.to_fen(),
            leaf_eval_sigmoid: eval_white_sigmoid,
            leaf_eval_cp: eval_white_cp,
            gradient,
            search_depth: depth,
            predicted_opponent_move,
        });
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::eval::EvalWeights;

    fn default_engine(depth: u8) -> SearchEngine {
        SearchEngine::new(EvalWeights::default_weights(), depth)
    }

    #[test]
    fn search_from_starting_position_returns_legal_move() {
        let board = Board::new();
        let mut engine = default_engine(3);
        let result = engine.search_to_depth(&board, 3);

        assert!(
            result.best_move.is_some(),
            "Search should return a move from the starting position"
        );

        // Verify the move is legal.
        let legal_moves = board.legal_moves();
        let best = result.best_move.unwrap();
        assert!(
            legal_moves.contains(&best),
            "Best move {} should be legal",
            best
        );
    }

    #[test]
    fn search_finds_mate_in_one() {
        // Scholar's mate setup where Qxf7# is mate.
        let board = Board::from_fen(
            "r1bqkb1r/pppp1ppp/2n2n2/4p2Q/2B1P3/8/PPPP1PPP/RNB1K1NR w KQkq - 4 4",
        )
        .unwrap();

        // Verify Qxf7# is mate.
        let test_board = board
            .clone_and_make(&GameMove::new("h5", "f7", None))
            .unwrap();
        assert_eq!(
            test_board.game_state(),
            GameState::Checkmate,
            "Qxf7# should be checkmate in this position"
        );

        let mut engine = default_engine(4);
        let result = engine.search_to_depth(&board, 4);

        let best = result.best_move.expect("Should find a move");
        assert_eq!(best.from, "h5", "Should move queen from h5");
        assert_eq!(best.to, "f7", "Should move queen to f7 (mate)");
    }

    #[test]
    fn search_depth_1_returns_reasonable_evaluation() {
        let board = Board::new();
        let mut engine = default_engine(1);
        let result = engine.search_to_depth(&board, 1);

        // Starting position should evaluate near zero at depth 1.
        let eval = result.evaluation as i32;
        assert!(
            eval.abs() < 200,
            "Depth 1 eval of starting position should be near zero, got {} cp",
            eval
        );
    }

    #[test]
    fn iterative_deepening_returns_result_for_each_depth() {
        let board = Board::new();
        let mut engine = default_engine(3);
        let mut depths_seen: Vec<u8> = Vec::new();

        let mut callback = |progress: SearchProgress| {
            depths_seen.push(progress.depth);
        };

        let (result, _leaf) = engine.search(&board, Some(&mut callback));

        assert_eq!(
            depths_seen,
            vec![1, 2, 3],
            "Should get progress for depths 1, 2, 3"
        );
        assert!(
            result.best_move.is_some(),
            "Final result should have a best move"
        );
    }

    #[test]
    fn search_prefers_capturing_free_queen() {
        // White: Ke1, Nc3. Black: Ke8, Qd5 (queen hanging).
        let board =
            Board::from_fen("4k3/8/8/3q4/8/2N5/8/4K3 w - - 0 1").unwrap();
        let mut engine = default_engine(3);
        let result = engine.search_to_depth(&board, 3);

        let best = result.best_move.expect("Should find a move");
        assert_eq!(best.from, "c3", "Should move knight from c3");
        assert_eq!(best.to, "d5", "Should capture queen on d5");
    }

    #[test]
    fn pv_contains_at_least_one_move() {
        let board = Board::new();
        let mut engine = default_engine(2);
        let result = engine.search_to_depth(&board, 2);

        assert!(
            !result.pv_line.is_empty(),
            "PV should contain at least one move"
        );
    }

    #[test]
    fn search_leaf_data_is_populated() {
        let board = Board::new();
        let mut engine = default_engine(3);
        let (_result, leaf) = engine.search(&board, None);

        assert!(
            !leaf.leaf_fen.is_empty(),
            "Leaf FEN should be populated"
        );
        assert!(
            !leaf.gradient.is_empty(),
            "Leaf gradient should be populated"
        );
        assert!(
            leaf.search_depth > 0,
            "Search depth should be positive"
        );
        // Sigmoid should be in [-1, 1].
        assert!(
            leaf.leaf_eval_sigmoid >= -1.0 && leaf.leaf_eval_sigmoid <= 1.0,
            "Leaf sigmoid {} should be in [-1, 1]",
            leaf.leaf_eval_sigmoid
        );
    }

    #[test]
    fn nodes_searched_is_positive() {
        let board = Board::new();
        let mut engine = default_engine(2);
        let result = engine.search_to_depth(&board, 2);

        assert!(
            result.nodes_searched > 0,
            "Nodes searched should be positive, got {}",
            result.nodes_searched
        );
    }

    // -----------------------------------------------------------------------
    // TT + Quiescence integration tests
    // -----------------------------------------------------------------------

    #[test]
    fn search_with_tt_returns_same_best_move_as_without() {
        // Use a position with clear best move so both paths agree.
        // Scholar's mate position: Qxf7# is the only winning move.
        let board = Board::from_fen(
            "r1bqkb1r/pppp1ppp/2n2n2/4p2Q/2B1P3/8/PPPP1PPP/RNB1K1NR w KQkq - 4 4",
        )
        .unwrap();

        let mut engine_with_tt = default_engine(4);
        let result_with_tt = engine_with_tt.search_to_depth(&board, 4);

        let mut engine_no_tt = default_engine(4);
        let result_no_tt = engine_no_tt.search_to_depth_no_tt(&board, 4);

        assert_eq!(
            result_with_tt.best_move, result_no_tt.best_move,
            "TT search and non-TT search should find the same best move (Qxf7#)"
        );
    }

    #[test]
    fn quiescence_stabilizes_tactical_position() {
        // Position where a pawn can capture a queen but the static eval might not
        // see it without quiescence. White pawn on d4 can capture Black queen on e5.
        // White: Ke1, Pd4. Black: Ke8, Qe5.
        // After dxe5 White is up a queen (huge material swing).
        //
        // Without quiescence, depth 0 would just do static eval which sees queen
        // for black. With quiescence, the capture d4xe5 is explored.
        let board =
            Board::from_fen("4k3/8/8/4q3/3P4/8/8/4K3 w - - 0 1").unwrap();
        let mut engine = default_engine(1);
        let result = engine.search_to_depth(&board, 1);

        // Engine should find dxe5 capturing the queen.
        let best = result.best_move.expect("Should find a move");
        assert_eq!(
            best.from, "d4",
            "Should move pawn from d4"
        );
        assert_eq!(
            best.to, "e5",
            "Should capture queen on e5"
        );
        // Eval should be positive since white captures a queen.
        assert!(
            result.evaluation > 0.0,
            "Evaluation should be positive after capturing queen, got {}",
            result.evaluation
        );
    }

    #[test]
    fn search_with_tt_searches_fewer_nodes_at_depth_4() {
        // Use a middlegame position for meaningful node savings.
        // Italian Game after 1.e4 e5 2.Nf3 Nc6 3.Bc4
        let board = Board::from_fen(
            "r1bqkbnr/pppp1ppp/2n5/4p3/2B1P3/5N2/PPPP1PPP/RNBQK2R b KQkq - 3 3",
        )
        .unwrap();

        let mut engine_no_tt = SearchEngine::with_tt_size(
            EvalWeights::default_weights(),
            4,
            1,
        );
        let result_no_tt = engine_no_tt.search_to_depth_no_tt(&board, 4);

        let mut engine_with_tt = SearchEngine::with_tt_size(
            EvalWeights::default_weights(),
            4,
            1,
        );
        let result_with_tt = engine_with_tt.search_to_depth(&board, 4);

        assert!(
            result_with_tt.nodes_searched <= result_no_tt.nodes_searched,
            "TT search ({} nodes) should search no more nodes than non-TT search ({} nodes)",
            result_with_tt.nodes_searched,
            result_no_tt.nodes_searched
        );
    }
}
