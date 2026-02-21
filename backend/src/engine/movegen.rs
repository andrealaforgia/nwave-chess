//! Move ordering and generation utilities.
//! Provides MVV-LVA (Most Valuable Victim - Least Valuable Attacker) capture ordering,
//! killer move tracking, and history heuristic for optimal search pruning.

use cozy_chess::Piece;

use super::board::Board;
use super::types::GameMove;

/// Piece values for MVV-LVA scoring (indexed by piece type ordinal).
/// Pawn=1, Knight=3, Bishop=3, Rook=5, Queen=9, King=100.
const PIECE_VALUES: [i32; 6] = [1, 3, 3, 5, 9, 100];

/// Maximum ply depth for killer move storage.
const MAX_PLY: usize = 128;

/// Order moves for better alpha-beta pruning.
/// Priority: 1) PV move, 2) Captures (MVV-LVA), 3) Killer moves, 4) History heuristic, 5) Quiet moves
pub struct MoveOrderer {
    killer_moves: [[Option<GameMove>; 2]; MAX_PLY],
    history_table: [[i32; 64]; 64],
}

impl MoveOrderer {
    pub fn new() -> Self {
        Self {
            killer_moves: std::array::from_fn(|_| [None, None]),
            history_table: [[0; 64]; 64],
        }
    }

    /// Sort moves for a given position. Returns moves in best-first order.
    pub fn order_moves(
        &self,
        board: &Board,
        moves: &mut Vec<GameMove>,
        pv_move: Option<&GameMove>,
        ply: usize,
    ) {
        let scores: Vec<i32> = moves
            .iter()
            .map(|mv| self.score_move(board, mv, pv_move, ply))
            .collect();

        // Create index array and sort by score descending.
        let mut indices: Vec<usize> = (0..moves.len()).collect();
        indices.sort_unstable_by(|&a, &b| scores[b].cmp(&scores[a]));

        let original = moves.clone();
        for (i, &idx) in indices.iter().enumerate() {
            moves[i] = original[idx].clone();
        }
    }

    /// Record a killer move at the given ply.
    pub fn record_killer(&mut self, mv: &GameMove, ply: usize) {
        if ply >= MAX_PLY {
            return;
        }
        // Don't store duplicates: if it matches slot 0, skip.
        if self.killer_moves[ply][0].as_ref() == Some(mv) {
            return;
        }
        // Shift slot 0 to slot 1, store new killer in slot 0.
        self.killer_moves[ply][1] = self.killer_moves[ply][0].clone();
        self.killer_moves[ply][0] = Some(mv.clone());
    }

    /// Record a history heuristic success.
    pub fn record_history(&mut self, mv: &GameMove, depth: i32) {
        let from_idx = square_index(&mv.from);
        let to_idx = square_index(&mv.to);
        if let (Some(f), Some(t)) = (from_idx, to_idx) {
            self.history_table[f][t] += depth * depth;
        }
    }

    /// Clear for new search.
    pub fn clear(&mut self) {
        self.killer_moves = std::array::from_fn(|_| [None, None]);
        self.history_table = [[0; 64]; 64];
    }

    /// Score a move for ordering. Higher scores = searched first.
    fn score_move(
        &self,
        board: &Board,
        mv: &GameMove,
        pv_move: Option<&GameMove>,
        ply: usize,
    ) -> i32 {
        // PV move gets highest priority.
        if let Some(pv) = pv_move {
            if mv == pv {
                return 100_000;
            }
        }

        // Check if it is a capture by examining the target square for an opponent piece.
        let to_sq: cozy_chess::Square = match mv.to.parse() {
            Ok(sq) => sq,
            Err(_) => return 0,
        };
        let from_sq: cozy_chess::Square = match mv.from.parse() {
            Ok(sq) => sq,
            Err(_) => return 0,
        };

        let inner = board.inner();
        let opponent_color = !inner.side_to_move();

        // Capture detection: is there an opponent piece on the target square?
        let is_capture = inner.colors(opponent_color).has(to_sq);

        if is_capture {
            // MVV-LVA: victim_value * 10 - attacker_value
            let victim_value = piece_value_on_square(inner, to_sq);
            let attacker_value = piece_value_on_square(inner, from_sq);
            return 50_000 + victim_value * 10 - attacker_value;
        }

        // En passant capture: pawn moves diagonally to an empty square.
        if inner.piece_on(from_sq) == Some(Piece::Pawn) {
            let from_file = from_sq.file() as i32;
            let to_file = to_sq.file() as i32;
            if (from_file - to_file).abs() == 1 && !inner.occupied().has(to_sq) {
                // En passant capture (captures a pawn)
                return 50_000 + PIECE_VALUES[0] * 10 - PIECE_VALUES[0];
            }
        }

        // Killer moves.
        if ply < MAX_PLY {
            if self.killer_moves[ply][0].as_ref() == Some(mv) {
                return 40_000;
            }
            if self.killer_moves[ply][1].as_ref() == Some(mv) {
                return 39_000;
            }
        }

        // History heuristic.
        let from_idx = square_index(&mv.from);
        let to_idx = square_index(&mv.to);
        if let (Some(f), Some(t)) = (from_idx, to_idx) {
            return self.history_table[f][t];
        }

        0
    }
}

impl Default for MoveOrderer {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert a square string (e.g., "e4") to an index 0..63.
fn square_index(sq_str: &str) -> Option<usize> {
    let sq: cozy_chess::Square = sq_str.parse().ok()?;
    Some(sq as usize)
}

/// Get the MVV-LVA piece value for the piece on a given square.
fn piece_value_on_square(board: &cozy_chess::Board, sq: cozy_chess::Square) -> i32 {
    match board.piece_on(sq) {
        Some(piece) => PIECE_VALUES[piece as usize],
        None => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn move_ordering_puts_captures_before_quiet_moves() {
        // Position with captures available: White queen can capture Black pawn.
        // White: Ke1, Qd1. Black: Ke8, pd5. White to move.
        let board = Board::from_fen("4k3/8/8/3p4/8/8/8/3QK3 w - - 0 1").unwrap();
        let mut moves = board.legal_moves();
        let orderer = MoveOrderer::new();
        orderer.order_moves(&board, &mut moves, None, 0);

        // The capture Qxd5 should be first among all moves.
        let first = &moves[0];
        assert_eq!(first.to, "d5", "Capture Qxd5 should be ordered first, got {}", first);
    }

    #[test]
    fn pv_move_ordered_first() {
        let board = Board::new();
        let mut moves = board.legal_moves();
        let orderer = MoveOrderer::new();
        let pv_move = GameMove::new("g1", "f3", None);
        orderer.order_moves(&board, &mut moves, Some(&pv_move), 0);

        assert_eq!(moves[0], pv_move, "PV move should be ordered first");
    }

    #[test]
    fn killer_moves_are_recorded_and_used() {
        let board = Board::new();
        let mut orderer = MoveOrderer::new();
        let killer = GameMove::new("g1", "f3", None);
        orderer.record_killer(&killer, 0);

        let mut moves = board.legal_moves();
        orderer.order_moves(&board, &mut moves, None, 0);

        // Killer should be among the top moves (after captures, but there are none
        // in the starting position so it should be first).
        assert_eq!(moves[0], killer, "Killer move should be ordered first when no captures exist");
    }
}
