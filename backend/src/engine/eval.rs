//! Feature extraction, weight application, and gradient computation.
//! Evaluates positions using a weighted linear combination of hand-crafted features
//! with game-phase interpolation (middlegame/endgame).
//! Computes gradients for TD-Leaf learning via sigmoid transformation.

use cozy_chess::{
    get_bishop_moves, get_knight_moves, get_rook_moves, Board as CozyBoard, Color, Piece, Square,
};

use super::board::Board;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Sigmoid scaling factor: tanh(eval / SIGMOID_SCALE).
/// At 100 cp (1 pawn) the sigmoid is ~0.245.
const SIGMOID_SCALE: f64 = 400.0;

/// Material values used for game-phase calculation.
/// Knight=1, Bishop=1, Rook=2, Queen=4.
const PHASE_KNIGHT: f64 = 1.0;
const PHASE_BISHOP: f64 = 1.0;
const PHASE_ROOK: f64 = 2.0;
const PHASE_QUEEN: f64 = 4.0;
/// Total material phase in the starting position (2N + 2B + 2R + 1Q per side).
const PHASE_TOTAL: f64 = 2.0 * PHASE_KNIGHT + 2.0 * PHASE_BISHOP + 2.0 * PHASE_ROOK + PHASE_QUEEN;
/// Two sides.
const PHASE_TOTAL_BOTH: f64 = 2.0 * PHASE_TOTAL;

// ---------------------------------------------------------------------------
// Weight layout in flat vector
// ---------------------------------------------------------------------------
// piece_values_mg:  5   (indices 0..5)
// piece_values_eg:  5   (indices 5..10)
// psqt_mg:          6*64 = 384  (indices 10..394)
// psqt_eg:          6*64 = 384  (indices 394..778)
// mobility_mg:      6   (indices 778..784)
// mobility_eg:      6   (indices 784..790)
// bishop_pair_mg:   1   (index 790)
// bishop_pair_eg:   1   (index 791)
// rook_open_file_mg:     1   (index 792)
// rook_open_file_eg:     1   (index 793)
// rook_semi_open_file_mg: 1  (index 794)
// rook_semi_open_file_eg: 1  (index 795)

/// Total number of weights in the evaluation function.
pub const NUM_WEIGHTS: usize = 796;

const OFF_PV_MG: usize = 0;
const OFF_PV_EG: usize = 5;
const OFF_PSQT_MG: usize = 10;
const OFF_PSQT_EG: usize = OFF_PSQT_MG + 384;
const OFF_MOB_MG: usize = OFF_PSQT_EG + 384;
const OFF_MOB_EG: usize = OFF_MOB_MG + 6;
const OFF_BP_MG: usize = OFF_MOB_EG + 6;
const OFF_BP_EG: usize = OFF_BP_MG + 1;
const OFF_ROF_MG: usize = OFF_BP_EG + 1;
const OFF_ROF_EG: usize = OFF_ROF_MG + 1;
const OFF_RSOF_MG: usize = OFF_ROF_EG + 1;
const OFF_RSOF_EG: usize = OFF_RSOF_MG + 1;

// Sanity check at compile time.
const _: () = assert!(OFF_RSOF_EG + 1 == NUM_WEIGHTS);

// ---------------------------------------------------------------------------
// Feature categories
// ---------------------------------------------------------------------------

/// Feature categories for per-category learning rates and clipping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeatureCategory {
    PieceValue,
    PieceSquareTable,
    Mobility,
    BishopPair,
    RookPlacement,
}

// ---------------------------------------------------------------------------
// Piece-square tables (from White's perspective, a1=index 0)
// ---------------------------------------------------------------------------

/// The PSQ tables are stored with rank 1 first (index 0 = a1).
/// Given the task specification tables that list rank 8 first in the visual,
/// we reverse them so index 0 corresponds to a1.

#[rustfmt::skip]
const PAWN_MG: [f64; 64] = [
    // rank 1 (pawns never here)
     0.0,  0.0,  0.0,  0.0,  0.0,  0.0,  0.0,  0.0,
    // rank 2
     5.0, 10.0, 10.0,-20.0,-20.0, 10.0, 10.0,  5.0,
    // rank 3
     5.0, -5.0,-10.0,  0.0,  0.0,-10.0, -5.0,  5.0,
    // rank 4
     0.0,  0.0,  0.0, 20.0, 20.0,  0.0,  0.0,  0.0,
    // rank 5
     5.0,  5.0, 10.0, 25.0, 25.0, 10.0,  5.0,  5.0,
    // rank 6
    10.0, 10.0, 20.0, 30.0, 30.0, 20.0, 10.0, 10.0,
    // rank 7
    50.0, 50.0, 50.0, 50.0, 50.0, 50.0, 50.0, 50.0,
    // rank 8 (pawns never here)
     0.0,  0.0,  0.0,  0.0,  0.0,  0.0,  0.0,  0.0,
];

#[rustfmt::skip]
const KNIGHT_MG: [f64; 64] = [
    -50.0,-40.0,-30.0,-30.0,-30.0,-30.0,-40.0,-50.0,
    -40.0,-20.0,  0.0,  5.0,  5.0,  0.0,-20.0,-40.0,
    -30.0,  5.0, 10.0, 15.0, 15.0, 10.0,  5.0,-30.0,
    -30.0,  0.0, 15.0, 20.0, 20.0, 15.0,  0.0,-30.0,
    -30.0,  5.0, 15.0, 20.0, 20.0, 15.0,  5.0,-30.0,
    -30.0,  0.0, 10.0, 15.0, 15.0, 10.0,  0.0,-30.0,
    -40.0,-20.0,  0.0,  0.0,  0.0,  0.0,-20.0,-40.0,
    -50.0,-40.0,-30.0,-30.0,-30.0,-30.0,-40.0,-50.0,
];

#[rustfmt::skip]
const BISHOP_MG: [f64; 64] = [
    -20.0,-10.0,-10.0,-10.0,-10.0,-10.0,-10.0,-20.0,
    -10.0,  5.0,  0.0,  0.0,  0.0,  0.0,  5.0,-10.0,
    -10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0,-10.0,
    -10.0,  0.0, 10.0, 10.0, 10.0, 10.0,  0.0,-10.0,
    -10.0,  5.0,  5.0, 10.0, 10.0,  5.0,  5.0,-10.0,
    -10.0,  0.0,  5.0, 10.0, 10.0,  5.0,  0.0,-10.0,
    -10.0,  0.0,  0.0,  0.0,  0.0,  0.0,  0.0,-10.0,
    -20.0,-10.0,-10.0,-10.0,-10.0,-10.0,-10.0,-20.0,
];

#[rustfmt::skip]
const ROOK_MG: [f64; 64] = [
     0.0,  0.0,  0.0,  5.0,  5.0,  0.0,  0.0,  0.0,
    -5.0,  0.0,  0.0,  0.0,  0.0,  0.0,  0.0, -5.0,
    -5.0,  0.0,  0.0,  0.0,  0.0,  0.0,  0.0, -5.0,
    -5.0,  0.0,  0.0,  0.0,  0.0,  0.0,  0.0, -5.0,
    -5.0,  0.0,  0.0,  0.0,  0.0,  0.0,  0.0, -5.0,
    -5.0,  0.0,  0.0,  0.0,  0.0,  0.0,  0.0, -5.0,
     5.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0,  5.0,
     0.0,  0.0,  0.0,  0.0,  0.0,  0.0,  0.0,  0.0,
];

#[rustfmt::skip]
const QUEEN_MG: [f64; 64] = [
    -20.0,-10.0,-10.0, -5.0, -5.0,-10.0,-10.0,-20.0,
    -10.0,  0.0,  5.0,  0.0,  0.0,  0.0,  0.0,-10.0,
    -10.0,  5.0,  5.0,  5.0,  5.0,  5.0,  0.0,-10.0,
      0.0,  0.0,  5.0,  5.0,  5.0,  5.0,  0.0, -5.0,
     -5.0,  0.0,  5.0,  5.0,  5.0,  5.0,  0.0, -5.0,
    -10.0,  0.0,  5.0,  5.0,  5.0,  5.0,  0.0,-10.0,
    -10.0,  0.0,  0.0,  0.0,  0.0,  0.0,  0.0,-10.0,
    -20.0,-10.0,-10.0, -5.0, -5.0,-10.0,-10.0,-20.0,
];

#[rustfmt::skip]
const KING_MG: [f64; 64] = [
     20.0, 30.0, 10.0,  0.0,  0.0, 10.0, 30.0, 20.0,
     20.0, 20.0,  0.0,  0.0,  0.0,  0.0, 20.0, 20.0,
    -10.0,-20.0,-20.0,-20.0,-20.0,-20.0,-20.0,-10.0,
    -20.0,-30.0,-30.0,-40.0,-40.0,-30.0,-30.0,-20.0,
    -30.0,-40.0,-40.0,-50.0,-50.0,-40.0,-40.0,-30.0,
    -30.0,-40.0,-40.0,-50.0,-50.0,-40.0,-40.0,-30.0,
    -30.0,-40.0,-40.0,-50.0,-50.0,-40.0,-40.0,-30.0,
    -30.0,-40.0,-40.0,-50.0,-50.0,-40.0,-40.0,-30.0,
];

#[rustfmt::skip]
const KING_EG: [f64; 64] = [
    -50.0,-30.0,-30.0,-30.0,-30.0,-30.0,-30.0,-50.0,
    -30.0,-30.0,  0.0,  0.0,  0.0,  0.0,-30.0,-30.0,
    -30.0,-10.0, 20.0, 30.0, 30.0, 20.0,-10.0,-30.0,
    -30.0,-10.0, 30.0, 40.0, 40.0, 30.0,-10.0,-30.0,
    -30.0,-10.0, 30.0, 40.0, 40.0, 30.0,-10.0,-30.0,
    -30.0,-10.0, 20.0, 30.0, 30.0, 20.0,-10.0,-30.0,
    -30.0,-20.0,-10.0,  0.0,  0.0,-10.0,-20.0,-30.0,
    -50.0,-40.0,-30.0,-20.0,-20.0,-30.0,-40.0,-50.0,
];

// ---------------------------------------------------------------------------
// EvalWeights
// ---------------------------------------------------------------------------

/// All evaluation weights in a structured form.
#[derive(Debug, Clone)]
pub struct EvalWeights {
    /// Piece values in centipawns (P, N, B, R, Q) -- middlegame.
    pub piece_values_mg: [f64; 5],
    /// Piece values in centipawns (P, N, B, R, Q) -- endgame.
    pub piece_values_eg: [f64; 5],
    /// Piece-square table bonuses, 6 piece types x 64 squares -- middlegame.
    /// Indexed by [piece_index][square_index] from White's perspective.
    pub psqt_mg: [[f64; 64]; 6],
    /// Piece-square table bonuses -- endgame.
    pub psqt_eg: [[f64; 64]; 6],
    /// Mobility bonus per pseudo-legal move, one per piece type -- middlegame.
    pub mobility_mg: [f64; 6],
    /// Mobility bonus per pseudo-legal move -- endgame.
    pub mobility_eg: [f64; 6],
    /// Bishop pair bonus -- middlegame.
    pub bishop_pair_mg: f64,
    /// Bishop pair bonus -- endgame.
    pub bishop_pair_eg: f64,
    /// Rook on fully open file bonus -- middlegame.
    pub rook_open_file_mg: f64,
    /// Rook on fully open file bonus -- endgame.
    pub rook_open_file_eg: f64,
    /// Rook on semi-open file bonus -- middlegame.
    pub rook_semi_open_file_mg: f64,
    /// Rook on semi-open file bonus -- endgame.
    pub rook_semi_open_file_eg: f64,
}

impl EvalWeights {
    /// Create weights initialized with the Simplified Evaluation Function defaults.
    pub fn default_weights() -> Self {
        // For endgame PSQ tables, use same as middlegame for non-king pieces (v1).
        Self {
            piece_values_mg: [100.0, 320.0, 330.0, 500.0, 900.0],
            piece_values_eg: [120.0, 330.0, 340.0, 520.0, 930.0],
            psqt_mg: [PAWN_MG, KNIGHT_MG, BISHOP_MG, ROOK_MG, QUEEN_MG, KING_MG],
            psqt_eg: [PAWN_MG, KNIGHT_MG, BISHOP_MG, ROOK_MG, QUEEN_MG, KING_EG],
            mobility_mg: [0.0, 4.0, 3.0, 2.0, 1.0, 0.0],
            mobility_eg: [0.0, 4.0, 3.0, 2.0, 1.0, 0.0],
            bishop_pair_mg: 30.0,
            bishop_pair_eg: 50.0,
            rook_open_file_mg: 20.0,
            rook_open_file_eg: 15.0,
            rook_semi_open_file_mg: 10.0,
            rook_semi_open_file_eg: 10.0,
        }
    }

    /// Number of weights.
    pub fn num_weights() -> usize {
        NUM_WEIGHTS
    }

    /// Serialize to a flat f64 vector (for Adam optimizer, persistence).
    pub fn to_vec(&self) -> Vec<f64> {
        let mut v = Vec::with_capacity(NUM_WEIGHTS);
        v.extend_from_slice(&self.piece_values_mg);
        v.extend_from_slice(&self.piece_values_eg);
        for piece_idx in 0..6 {
            v.extend_from_slice(&self.psqt_mg[piece_idx]);
        }
        for piece_idx in 0..6 {
            v.extend_from_slice(&self.psqt_eg[piece_idx]);
        }
        v.extend_from_slice(&self.mobility_mg);
        v.extend_from_slice(&self.mobility_eg);
        v.push(self.bishop_pair_mg);
        v.push(self.bishop_pair_eg);
        v.push(self.rook_open_file_mg);
        v.push(self.rook_open_file_eg);
        v.push(self.rook_semi_open_file_mg);
        v.push(self.rook_semi_open_file_eg);
        debug_assert_eq!(v.len(), NUM_WEIGHTS);
        v
    }

    /// Deserialize from a flat f64 vector.
    pub fn from_vec(v: &[f64]) -> Result<Self, String> {
        if v.len() != NUM_WEIGHTS {
            return Err(format!(
                "Expected {} weights, got {}",
                NUM_WEIGHTS,
                v.len()
            ));
        }
        let piece_values_mg: [f64; 5] = v[OFF_PV_MG..OFF_PV_EG].try_into().unwrap();
        let piece_values_eg: [f64; 5] = v[OFF_PV_EG..OFF_PSQT_MG].try_into().unwrap();

        let mut psqt_mg = [[0.0f64; 64]; 6];
        for p in 0..6 {
            let start = OFF_PSQT_MG + p * 64;
            psqt_mg[p].copy_from_slice(&v[start..start + 64]);
        }
        let mut psqt_eg = [[0.0f64; 64]; 6];
        for p in 0..6 {
            let start = OFF_PSQT_EG + p * 64;
            psqt_eg[p].copy_from_slice(&v[start..start + 64]);
        }

        let mobility_mg: [f64; 6] = v[OFF_MOB_MG..OFF_MOB_EG].try_into().unwrap();
        let mobility_eg: [f64; 6] = v[OFF_MOB_EG..OFF_BP_MG].try_into().unwrap();

        Ok(Self {
            piece_values_mg,
            piece_values_eg,
            psqt_mg,
            psqt_eg,
            mobility_mg,
            mobility_eg,
            bishop_pair_mg: v[OFF_BP_MG],
            bishop_pair_eg: v[OFF_BP_EG],
            rook_open_file_mg: v[OFF_ROF_MG],
            rook_open_file_eg: v[OFF_ROF_EG],
            rook_semi_open_file_mg: v[OFF_RSOF_MG],
            rook_semi_open_file_eg: v[OFF_RSOF_EG],
        })
    }
}

// ---------------------------------------------------------------------------
// EvalResult
// ---------------------------------------------------------------------------

/// Evaluation result with gradient information for TD-Leaf learning.
pub struct EvalResult {
    /// Centipawn score (positive = good for side to move).
    pub score_cp: i32,
    /// tanh-transformed score in [-1, 1].
    pub score_sigmoid: f64,
    /// Game phase 0.0 (middlegame) to 1.0 (endgame).
    pub phase: f64,
    /// d(score_sigmoid)/d(weight_i) for each weight.
    pub gradient: Vec<f64>,
}

// ---------------------------------------------------------------------------
// Game-phase computation
// ---------------------------------------------------------------------------

/// Compute game phase from remaining material.
/// Returns 0.0 (pure middlegame) to 1.0 (pure endgame).
pub fn compute_phase(board: &Board) -> f64 {
    let inner = board.inner();
    let mut phase_material = 0.0;
    phase_material += (inner.pieces(Piece::Knight)).len() as f64 * PHASE_KNIGHT;
    phase_material += (inner.pieces(Piece::Bishop)).len() as f64 * PHASE_BISHOP;
    phase_material += (inner.pieces(Piece::Rook)).len() as f64 * PHASE_ROOK;
    phase_material += (inner.pieces(Piece::Queen)).len() as f64 * PHASE_QUEEN;

    // phase = 1 - (remaining / total), clamped to [0, 1]
    (1.0 - phase_material / PHASE_TOTAL_BOTH).clamp(0.0, 1.0)
}

// ---------------------------------------------------------------------------
// Mobility counting
// ---------------------------------------------------------------------------

/// Count pseudo-legal mobility per piece type for the given color.
/// Returns an array indexed by piece type: [Pawn, Knight, Bishop, Rook, Queen, King].
fn count_mobility(inner: &CozyBoard, color: Color) -> [f64; 6] {
    let mut counts = [0.0f64; 6];
    let occupied = inner.occupied();
    let friendly = inner.colors(color);

    // Knights
    for sq in inner.colored_pieces(color, Piece::Knight) {
        let moves = get_knight_moves(sq) & !friendly;
        counts[1] += moves.len() as f64;
    }

    // Bishops
    for sq in inner.colored_pieces(color, Piece::Bishop) {
        let moves = get_bishop_moves(sq, occupied) & !friendly;
        counts[2] += moves.len() as f64;
    }

    // Rooks
    for sq in inner.colored_pieces(color, Piece::Rook) {
        let moves = get_rook_moves(sq, occupied) & !friendly;
        counts[3] += moves.len() as f64;
    }

    // Queens
    for sq in inner.colored_pieces(color, Piece::Queen) {
        let moves =
            (get_bishop_moves(sq, occupied) | get_rook_moves(sq, occupied)) & !friendly;
        counts[4] += moves.len() as f64;
    }

    counts
}

// ---------------------------------------------------------------------------
// Rook placement helpers
// ---------------------------------------------------------------------------

/// For a rook on a given file, determine if it is on an open or semi-open file.
/// Returns (is_open, is_semi_open).
fn rook_file_status(inner: &CozyBoard, rook_sq: Square, color: Color) -> (bool, bool) {
    let file_bb = rook_sq.file().bitboard();
    let our_pawns = inner.colored_pieces(color, Piece::Pawn) & file_bb;
    let their_pawns = inner.colored_pieces(!color, Piece::Pawn) & file_bb;

    if our_pawns.is_empty() && their_pawns.is_empty() {
        (true, false) // open
    } else if our_pawns.is_empty() && !their_pawns.is_empty() {
        (false, true) // semi-open
    } else {
        (false, false) // closed
    }
}

// ---------------------------------------------------------------------------
// Main evaluation function
// ---------------------------------------------------------------------------

/// Evaluate a position. The score is from the perspective of the side to move.
/// Also computes the gradient d(score_sigmoid)/d(weight_i).
pub fn evaluate(board: &Board, weights: &EvalWeights) -> EvalResult {
    let inner = board.inner();
    let phase = compute_phase(board);

    // We accumulate raw eval from White's perspective, then flip sign if Black to move.
    let mut eval_mg = 0.0f64;
    let mut eval_eg = 0.0f64;

    // Gradient accumulator: stores d(eval_white)/d(weight_i) split into mg and eg
    // contributions. We combine them at the end with phase interpolation.
    let mut grad = vec![0.0f64; NUM_WEIGHTS];

    // -----------------------------------------------------------------------
    // 1. Piece values and piece-square tables
    // -----------------------------------------------------------------------
    let piece_types = [
        Piece::Pawn,
        Piece::Knight,
        Piece::Bishop,
        Piece::Rook,
        Piece::Queen,
        Piece::King,
    ];

    for (piece_idx, &piece) in piece_types.iter().enumerate() {
        // White pieces
        for sq in inner.colored_pieces(Color::White, piece) {
            let sq_idx = sq as usize;
            // Material (king has no piece value)
            if piece_idx < 5 {
                eval_mg += weights.piece_values_mg[piece_idx];
                eval_eg += weights.piece_values_eg[piece_idx];
                grad[OFF_PV_MG + piece_idx] += 1.0 - phase;
                grad[OFF_PV_EG + piece_idx] += phase;
            }
            // PSQ table (already from White's perspective)
            eval_mg += weights.psqt_mg[piece_idx][sq_idx];
            eval_eg += weights.psqt_eg[piece_idx][sq_idx];
            grad[OFF_PSQT_MG + piece_idx * 64 + sq_idx] += 1.0 - phase;
            grad[OFF_PSQT_EG + piece_idx * 64 + sq_idx] += phase;
        }

        // Black pieces (flip rank for PSQ lookup)
        for sq in inner.colored_pieces(Color::Black, piece) {
            let flipped_sq_idx = sq.flip_rank() as usize;
            if piece_idx < 5 {
                eval_mg -= weights.piece_values_mg[piece_idx];
                eval_eg -= weights.piece_values_eg[piece_idx];
                grad[OFF_PV_MG + piece_idx] -= 1.0 - phase;
                grad[OFF_PV_EG + piece_idx] -= phase;
            }
            eval_mg -= weights.psqt_mg[piece_idx][flipped_sq_idx];
            eval_eg -= weights.psqt_eg[piece_idx][flipped_sq_idx];
            grad[OFF_PSQT_MG + piece_idx * 64 + flipped_sq_idx] -= 1.0 - phase;
            grad[OFF_PSQT_EG + piece_idx * 64 + flipped_sq_idx] -= phase;
        }
    }

    // -----------------------------------------------------------------------
    // 2. Mobility
    // -----------------------------------------------------------------------
    let white_mob = count_mobility(inner, Color::White);
    let black_mob = count_mobility(inner, Color::Black);

    for p in 0..6 {
        let diff = white_mob[p] - black_mob[p];
        eval_mg += diff * weights.mobility_mg[p];
        eval_eg += diff * weights.mobility_eg[p];
        grad[OFF_MOB_MG + p] += diff * (1.0 - phase);
        grad[OFF_MOB_EG + p] += diff * phase;
    }

    // -----------------------------------------------------------------------
    // 3. Bishop pair
    // -----------------------------------------------------------------------
    let white_has_pair = inner.colored_pieces(Color::White, Piece::Bishop).len() >= 2;
    let black_has_pair = inner.colored_pieces(Color::Black, Piece::Bishop).len() >= 2;
    let bp_diff = (white_has_pair as i32 - black_has_pair as i32) as f64;

    eval_mg += bp_diff * weights.bishop_pair_mg;
    eval_eg += bp_diff * weights.bishop_pair_eg;
    grad[OFF_BP_MG] += bp_diff * (1.0 - phase);
    grad[OFF_BP_EG] += bp_diff * phase;

    // -----------------------------------------------------------------------
    // 4. Rook on open / semi-open file
    // -----------------------------------------------------------------------
    let mut rook_open_diff = 0.0f64;
    let mut rook_semi_open_diff = 0.0f64;

    for sq in inner.colored_pieces(Color::White, Piece::Rook) {
        let (open, semi_open) = rook_file_status(inner, sq, Color::White);
        if open {
            rook_open_diff += 1.0;
        }
        if semi_open {
            rook_semi_open_diff += 1.0;
        }
    }
    for sq in inner.colored_pieces(Color::Black, Piece::Rook) {
        let (open, semi_open) = rook_file_status(inner, sq, Color::Black);
        if open {
            rook_open_diff -= 1.0;
        }
        if semi_open {
            rook_semi_open_diff -= 1.0;
        }
    }

    eval_mg += rook_open_diff * weights.rook_open_file_mg;
    eval_eg += rook_open_diff * weights.rook_open_file_eg;
    eval_mg += rook_semi_open_diff * weights.rook_semi_open_file_mg;
    eval_eg += rook_semi_open_diff * weights.rook_semi_open_file_eg;

    grad[OFF_ROF_MG] += rook_open_diff * (1.0 - phase);
    grad[OFF_ROF_EG] += rook_open_diff * phase;
    grad[OFF_RSOF_MG] += rook_semi_open_diff * (1.0 - phase);
    grad[OFF_RSOF_EG] += rook_semi_open_diff * phase;

    // -----------------------------------------------------------------------
    // Combine middlegame / endgame with phase interpolation
    // -----------------------------------------------------------------------
    let eval_white = (1.0 - phase) * eval_mg + phase * eval_eg;

    // Flip for side to move
    let sign = if inner.side_to_move() == Color::White {
        1.0
    } else {
        -1.0
    };
    let eval_stm = sign * eval_white;
    let score_cp = eval_stm as i32;

    // Sigmoid
    let score_sigmoid = (eval_stm / SIGMOID_SCALE).tanh();

    // Chain rule: d(sigmoid)/d(w_i) = (1 - tanh^2) / SIGMOID_SCALE * sign * d(eval_white)/d(w_i)
    // But grad already contains d(eval_white)/d(w_i) with phase baked in, because we accumulated
    // grad[mg_weight] += feature * (1-phase) and grad[eg_weight] += feature * phase.
    // However, the gradients currently represent d((1-phase)*eval_mg + phase*eval_eg)/d(w_i)
    // which IS d(eval_white)/d(w_i). So we just need to apply sigmoid chain rule.
    let sigmoid_deriv = (1.0 - score_sigmoid * score_sigmoid) / SIGMOID_SCALE * sign;
    for g in grad.iter_mut() {
        *g *= sigmoid_deriv;
    }

    EvalResult {
        score_cp,
        score_sigmoid,
        phase,
        gradient: grad,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::types::GameMove;

    fn default_weights() -> EvalWeights {
        EvalWeights::default_weights()
    }

    #[test]
    fn starting_position_eval_approximately_zero() {
        let board = Board::new();
        let weights = default_weights();
        let result = evaluate(&board, &weights);
        // The starting position should be roughly balanced.
        // Allow a generous window: the material is equal, but PSQ + mobility might
        // give White a slight edge.
        assert!(
            result.score_cp.abs() < 100,
            "Starting position eval {} cp should be near zero",
            result.score_cp
        );
    }

    #[test]
    fn queen_advantage_gives_large_positive_eval() {
        // White has full material; Black is missing the queen.
        // This gives White a queen advantage (~900+ cp).
        let board =
            Board::from_fen("rnb1kbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
                .unwrap();
        let weights = default_weights();
        let result = evaluate(&board, &weights);
        assert!(
            result.score_cp > 500,
            "Queen advantage should give eval > 500 cp, got {}",
            result.score_cp
        );
    }

    #[test]
    fn phase_starting_position_near_zero() {
        let board = Board::new();
        let phase = compute_phase(&board);
        assert!(
            phase.abs() < 0.01,
            "Starting position phase should be ~0.0, got {}",
            phase
        );
    }

    #[test]
    fn phase_king_vs_king_is_one() {
        let board = Board::from_fen("4k3/8/8/8/8/8/8/4K3 w - - 0 1").unwrap();
        let phase = compute_phase(&board);
        assert!(
            (phase - 1.0).abs() < 0.01,
            "K vs K phase should be ~1.0, got {}",
            phase
        );
    }

    #[test]
    fn sigmoid_of_zero_is_zero() {
        let val = (0.0f64 / SIGMOID_SCALE).tanh();
        assert!(
            val.abs() < 1e-10,
            "sigmoid(0) should be 0, got {}",
            val
        );
    }

    #[test]
    fn sigmoid_of_one_pawn_approximately_0_245() {
        let val = (100.0f64 / SIGMOID_SCALE).tanh();
        assert!(
            (val - 0.245).abs() < 0.01,
            "sigmoid(100) should be ~0.245, got {}",
            val
        );
    }

    #[test]
    fn gradient_has_correct_length() {
        let board = Board::new();
        let weights = default_weights();
        let result = evaluate(&board, &weights);
        assert_eq!(
            result.gradient.len(),
            NUM_WEIGHTS,
            "Gradient length should equal NUM_WEIGHTS"
        );
    }

    #[test]
    fn gradient_nonzero_for_active_features() {
        // Use an asymmetric position so feature differences are nonzero.
        // White has an extra knight compared to Black.
        let board =
            Board::from_fen("rnbqkb1r/pppppppp/8/8/8/5N2/PPPPPPPP/RNBQKB1R w KQkq - 0 1")
                .unwrap();
        let weights = default_weights();
        let result = evaluate(&board, &weights);

        // Knight value mg gradient should be nonzero (White has 2 knights, Black has 1).
        let knight_mg_grad = result.gradient[OFF_PV_MG + 1];
        assert!(
            knight_mg_grad.abs() > 1e-10,
            "Knight MG value gradient should be nonzero in asymmetric position, got {}",
            knight_mg_grad
        );

        // Also check that PSQT gradient is nonzero for the White knight on f3
        // (no Black knight mirrors to that square).
        let f3_idx = Square::F3 as usize;
        let knight_psqt_grad = result.gradient[OFF_PSQT_MG + 1 * 64 + f3_idx];
        assert!(
            knight_psqt_grad.abs() > 1e-10,
            "PSQT gradient for knight on f3 should be nonzero, got {}",
            knight_psqt_grad
        );
    }

    #[test]
    fn eval_weights_round_trip_through_vec() {
        let weights = default_weights();
        let vec = weights.to_vec();
        assert_eq!(vec.len(), NUM_WEIGHTS);
        let restored = EvalWeights::from_vec(&vec).unwrap();
        let vec2 = restored.to_vec();
        for (i, (a, b)) in vec.iter().zip(vec2.iter()).enumerate() {
            assert!(
                (a - b).abs() < 1e-12,
                "Weight {} differs: {} vs {}",
                i,
                a,
                b
            );
        }
    }

    #[test]
    fn from_vec_rejects_wrong_length() {
        let result = EvalWeights::from_vec(&[0.0; 10]);
        assert!(result.is_err());
    }

    #[test]
    fn evaluation_changes_after_move() {
        let board = Board::new();
        let weights = default_weights();
        let eval_before = evaluate(&board, &weights);

        let mut board_after = board.clone();
        board_after
            .make_move(&GameMove::new("e2", "e4", None))
            .unwrap();
        let eval_after = evaluate(&board_after, &weights);

        assert_ne!(
            eval_before.score_cp, eval_after.score_cp,
            "Eval should change after a move"
        );
    }

    #[test]
    fn knight_on_e4_scores_higher_than_a1() {
        // White knight on e4 (central, good PSQ value) vs White knight on a1 (corner, poor).
        // Use minimal positions with just kings and a single knight.
        let board_e4 =
            Board::from_fen("4k3/8/8/8/4N3/8/8/4K3 w - - 0 1").unwrap();
        let board_a1 =
            Board::from_fen("4k3/8/8/8/8/8/8/N3K3 w - - 0 1").unwrap();
        let weights = default_weights();
        let eval_e4 = evaluate(&board_e4, &weights);
        let eval_a1 = evaluate(&board_a1, &weights);
        assert!(
            eval_e4.score_cp > eval_a1.score_cp,
            "Knight on e4 ({} cp) should score higher than knight on a1 ({} cp)",
            eval_e4.score_cp,
            eval_a1.score_cp
        );
    }
}
