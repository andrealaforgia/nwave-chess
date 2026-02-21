//! Board representation wrapping cozy-chess.
//! Provides FEN import/export, legal move generation, and game state detection.
//! Manages Zobrist hashing for transposition table lookups and position history
//! for threefold repetition detection.

use cozy_chess::{self, Color, File, GameStatus, Piece, Rank, Square};

use super::types::{GameMove, GameState};

/// Chess board wrapper around `cozy_chess::Board` with move and position history
/// for repetition detection and fifty-move rule tracking.
#[derive(Debug, Clone)]
pub struct Board {
    inner: cozy_chess::Board,
    move_history: Vec<cozy_chess::Move>,
    position_history: Vec<u64>,
    halfmove_clock: u32,
    fullmove_number: u32,
}

impl Board {
    /// Create a new board from the standard starting position.
    pub fn new() -> Self {
        let inner = cozy_chess::Board::default();
        let hash = inner.hash();
        Self {
            inner,
            move_history: Vec::new(),
            position_history: vec![hash],
            halfmove_clock: 0,
            fullmove_number: 1,
        }
    }

    /// Create from a FEN string.
    pub fn from_fen(fen: &str) -> Result<Self, String> {
        let inner: cozy_chess::Board = fen.parse().map_err(|e| format!("Invalid FEN: {}", e))?;
        let hash = inner.hash();
        Ok(Self {
            halfmove_clock: inner.halfmove_clock() as u32,
            fullmove_number: inner.fullmove_number() as u32,
            inner,
            move_history: Vec::new(),
            position_history: vec![hash],
        })
    }

    /// Get the FEN string for the current position.
    pub fn to_fen(&self) -> String {
        format!("{}", self.inner)
    }

    /// Get all legal moves as GameMove.
    pub fn legal_moves(&self) -> Vec<GameMove> {
        let mut moves = Vec::new();
        self.inner.generate_moves(|piece_moves| {
            for mv in piece_moves {
                moves.push(Self::cozy_move_to_game_move(mv));
            }
            false
        });
        moves
    }

    /// Make a move (validates legality, returns error if illegal).
    pub fn make_move(&mut self, mv: &GameMove) -> Result<(), String> {
        let cozy_mv = self.game_move_to_cozy_move(mv)?;

        self.inner
            .try_play(cozy_mv)
            .map_err(|_| format!("Illegal move: {}", mv))?;

        self.move_history.push(cozy_mv);
        self.halfmove_clock = self.inner.halfmove_clock() as u32;
        self.fullmove_number = self.inner.fullmove_number() as u32;
        self.position_history.push(self.inner.hash());

        Ok(())
    }

    /// Get the current game state, considering check, checkmate, stalemate,
    /// repetition, fifty-move rule, and insufficient material.
    pub fn game_state(&self) -> GameState {
        // Check insufficient material first (cozy_chess status() doesn't detect it)
        if self.is_insufficient_material() {
            return GameState::DrawByInsufficientMaterial;
        }

        // Check threefold repetition
        if self.is_threefold_repetition() {
            return GameState::DrawByRepetition;
        }

        match self.inner.status() {
            GameStatus::Won => GameState::Checkmate,
            GameStatus::Drawn => {
                // Distinguish stalemate from fifty-move rule
                if self.inner.halfmove_clock() >= 100 {
                    GameState::DrawByFiftyMove
                } else {
                    GameState::Stalemate
                }
            }
            GameStatus::Ongoing => {
                if self.is_check() {
                    GameState::Check
                } else {
                    GameState::InProgress
                }
            }
        }
    }

    /// Is the current side in check?
    pub fn is_check(&self) -> bool {
        !self.inner.checkers().is_empty()
    }

    /// Get the side to move.
    pub fn side_to_move(&self) -> Color {
        self.inner.side_to_move()
    }

    /// Access the inner cozy_chess::Board for evaluation.
    pub fn inner(&self) -> &cozy_chess::Board {
        &self.inner
    }

    /// Zobrist hash for transposition table.
    pub fn hash(&self) -> u64 {
        self.inner.hash()
    }

    /// Number of moves played so far.
    pub fn move_count(&self) -> usize {
        self.move_history.len()
    }

    /// Clone the board, apply a move on the clone, and return it.
    /// The original board is not modified.
    pub fn clone_and_make(&self, mv: &GameMove) -> Result<Self, String> {
        let mut cloned = self.clone();
        cloned.make_move(mv)?;
        Ok(cloned)
    }

    /// Check for threefold repetition by counting occurrences of the current
    /// position hash in the position history.
    fn is_threefold_repetition(&self) -> bool {
        let current_hash = self.inner.hash();
        let count = self.position_history.iter().filter(|&&h| h == current_hash).count();
        count >= 3
    }

    /// Check for insufficient material to force checkmate.
    /// Detects: K vs K, K+B vs K, K+N vs K, K+B vs K+B (same color bishops).
    fn is_insufficient_material(&self) -> bool {
        let occupied = self.inner.occupied();
        let total_pieces = occupied.len();

        // K vs K
        if total_pieces == 2 {
            return true;
        }

        // K+minor vs K
        if total_pieces == 3 {
            let knights = self.inner.pieces(Piece::Knight);
            let bishops = self.inner.pieces(Piece::Bishop);
            if knights.len() == 1 || bishops.len() == 1 {
                return true;
            }
        }

        // K+B vs K+B with bishops on same color squares
        if total_pieces == 4 {
            let white_bishops = self.inner.colored_pieces(Color::White, Piece::Bishop);
            let black_bishops = self.inner.colored_pieces(Color::Black, Piece::Bishop);
            if white_bishops.len() == 1 && black_bishops.len() == 1 {
                let white_bishop_sq = white_bishops.next_square().unwrap();
                let black_bishop_sq = black_bishops.next_square().unwrap();
                // Same color square if (file + rank) parity is the same
                let white_parity =
                    (white_bishop_sq.file() as u8 + white_bishop_sq.rank() as u8) % 2;
                let black_parity =
                    (black_bishop_sq.file() as u8 + black_bishop_sq.rank() as u8) % 2;
                if white_parity == black_parity {
                    return true;
                }
            }
        }

        false
    }

    /// Convert a cozy_chess::Move to a GameMove.
    pub fn cozy_move_to_game_move(mv: cozy_chess::Move) -> GameMove {
        GameMove {
            from: format!("{}", mv.from),
            to: format!("{}", mv.to),
            promotion: mv.promotion.map(|p| format!("{}", p)),
        }
    }

    /// Convert a GameMove to a cozy_chess::Move, resolving castling encoding.
    /// cozy-chess encodes castling as king captures own rook, so we need to
    /// translate standard notation (e.g., e1g1) to that encoding.
    pub fn game_move_to_cozy_move(&self, mv: &GameMove) -> Result<cozy_chess::Move, String> {
        let from: Square = mv
            .from
            .parse()
            .map_err(|_| format!("Invalid from square: {}", mv.from))?;
        let to: Square = mv
            .to
            .parse()
            .map_err(|_| format!("Invalid to square: {}", mv.to))?;
        let promotion: Option<Piece> = match &mv.promotion {
            Some(p) => {
                let piece: Piece = p
                    .parse()
                    .map_err(|_| format!("Invalid promotion piece: {}", p))?;
                Some(piece)
            }
            None => None,
        };

        // Handle castling: if the king moves to g1/c1/g8/c8, translate to
        // king captures own rook encoding used by cozy-chess.
        let piece_on_from = self.inner.piece_on(from);
        if piece_on_from == Some(Piece::King) && promotion.is_none() {
            let color = self.inner.side_to_move();
            let back_rank = if color == Color::White {
                Rank::First
            } else {
                Rank::Eighth
            };

            if from.rank() == back_rank && to.rank() == back_rank {
                let rights = self.inner.castle_rights(color);

                // Kingside castle: e1->g1 or e8->g8
                if to.file() == File::G {
                    if let Some(rook_file) = rights.short {
                        let rook_sq = Square::new(rook_file, back_rank);
                        return Ok(cozy_chess::Move {
                            from,
                            to: rook_sq,
                            promotion: None,
                        });
                    }
                }

                // Queenside castle: e1->c1 or e8->c8
                if to.file() == File::C {
                    if let Some(rook_file) = rights.long {
                        let rook_sq = Square::new(rook_file, back_rank);
                        return Ok(cozy_chess::Move {
                            from,
                            to: rook_sq,
                            promotion: None,
                        });
                    }
                }
            }
        }

        Ok(cozy_chess::Move {
            from,
            to,
            promotion,
        })
    }

    /// Get the halfmove clock value.
    pub fn halfmove_clock(&self) -> u32 {
        self.halfmove_clock
    }

    /// Get the fullmove number.
    pub fn fullmove_number(&self) -> u32 {
        self.fullmove_number
    }

    /// Get a reference to the position history (Zobrist hashes).
    pub fn position_history(&self) -> &[u64] {
        &self.position_history
    }
}

impl Default for Board {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_game_creates_standard_position() {
        let board = Board::new();
        assert_eq!(
            board.to_fen(),
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"
        );
        assert_eq!(board.side_to_move(), Color::White);
        assert_eq!(board.move_count(), 0);
        assert_eq!(board.halfmove_clock(), 0);
        assert_eq!(board.fullmove_number(), 1);
    }

    #[test]
    fn fen_round_trip() {
        let fens = [
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1",
            "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
            "8/8/8/8/8/8/8/4K2k w - - 0 1",
        ];
        for fen in fens {
            let board = Board::from_fen(fen).unwrap();
            assert_eq!(board.to_fen(), fen, "FEN round-trip failed for: {}", fen);
        }
    }

    #[test]
    fn legal_moves_from_starting_position() {
        let board = Board::new();
        let moves = board.legal_moves();
        assert_eq!(moves.len(), 20, "Starting position should have 20 legal moves");
    }

    #[test]
    fn make_legal_move() {
        let mut board = Board::new();
        board.make_move(&GameMove::new("e2", "e4", None)).unwrap();
        assert_eq!(board.side_to_move(), Color::Black);
        assert_eq!(board.move_count(), 1);
        assert_eq!(
            board.to_fen(),
            "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1"
        );
    }

    #[test]
    fn reject_illegal_move() {
        let mut board = Board::new();
        let result = board.make_move(&GameMove::new("e1", "e8", None));
        assert!(result.is_err(), "Should reject illegal move e1e8");
    }

    #[test]
    fn detect_checkmate_scholars_mate() {
        // Scholar's mate: 1.e4 e5 2.Bc4 Nc6 3.Qh5 Nf6?? 4.Qxf7#
        let mut board = Board::new();
        let moves = [
            ("e2", "e4", None),
            ("e7", "e5", None),
            ("f1", "c4", None),
            ("b8", "c6", None),
            ("d1", "h5", None),
            ("g8", "f6", None),
            ("h5", "f7", None),
        ];
        for (from, to, promo) in &moves {
            board
                .make_move(&GameMove::new(from, to, *promo))
                .unwrap();
        }
        assert_eq!(board.game_state(), GameState::Checkmate);
        // The loser is the side to move (Black)
        assert_eq!(board.side_to_move(), Color::Black);
    }

    #[test]
    fn detect_stalemate() {
        // Black king on a8, White queen on b6 and king on c8: Black has no legal moves
        // but is not in check.
        let board = Board::from_fen("k7/8/1Q6/8/8/8/8/K7 b - - 0 1").unwrap();
        assert_eq!(board.game_state(), GameState::Stalemate);
    }

    #[test]
    fn detect_threefold_repetition() {
        let mut board = Board::new();
        // Move knights back and forth to repeat the starting-like positions
        let moves = [
            // Round 1: reach position A
            ("g1", "f3", None),
            ("g8", "f6", None),
            // Round 1: return to start-like
            ("f3", "g1", None),
            ("f6", "g8", None),
            // Round 2: reach position A again (2nd time)
            ("g1", "f3", None),
            ("g8", "f6", None),
            // Round 2: return to start-like (3rd occurrence of starting position)
            ("f3", "g1", None),
            ("f6", "g8", None),
        ];
        for (from, to, promo) in &moves {
            let state = board.game_state();
            assert!(
                state.is_ongoing(),
                "Game should still be ongoing before repetition, but got {:?}",
                state
            );
            board
                .make_move(&GameMove::new(from, to, *promo))
                .unwrap();
        }
        assert_eq!(board.game_state(), GameState::DrawByRepetition);
    }

    #[test]
    fn detect_fifty_move_rule() {
        // Set up a position and use FEN with halfmove clock at 99
        // Then make one more non-pawn non-capture move.
        let mut board =
            Board::from_fen("4k3/8/8/8/8/8/8/R3K3 w Q - 99 50").unwrap();
        assert_eq!(board.halfmove_clock(), 99);
        // King move increments halfmove clock to 100
        board.make_move(&GameMove::new("e1", "d1", None)).unwrap();
        assert_eq!(board.halfmove_clock(), 100);
        assert_eq!(board.game_state(), GameState::DrawByFiftyMove);
    }

    #[test]
    fn detect_insufficient_material_k_vs_k() {
        let board = Board::from_fen("4k3/8/8/8/8/8/8/4K3 w - - 0 1").unwrap();
        assert_eq!(board.game_state(), GameState::DrawByInsufficientMaterial);
    }

    #[test]
    fn detect_insufficient_material_kb_vs_k() {
        let board = Board::from_fen("4k3/8/8/8/8/8/8/4KB2 w - - 0 1").unwrap();
        assert_eq!(board.game_state(), GameState::DrawByInsufficientMaterial);
    }

    #[test]
    fn detect_insufficient_material_kn_vs_k() {
        let board = Board::from_fen("4k3/8/8/8/8/8/8/4KN2 w - - 0 1").unwrap();
        assert_eq!(board.game_state(), GameState::DrawByInsufficientMaterial);
    }

    #[test]
    fn detect_insufficient_material_kb_vs_kb_same_color() {
        // K+B vs K+B with both bishops on same-colored squares.
        // White B on c1: file C=2, rank First=0, 2+0=2 even (dark square)
        // Black b on f8: file F=5, rank Eighth=7, 5+7=12 even (dark square)
        // Same parity -> insufficient material
        let board =
            Board::from_fen("4kb2/8/8/8/8/8/8/2B1K3 w - - 0 1").unwrap();
        assert_eq!(board.game_state(), GameState::DrawByInsufficientMaterial);
    }

    #[test]
    fn sufficient_material_kb_vs_kb_different_color() {
        // Bishops on different colored squares: can force checkmate in some positions
        // White B on c1 (2+0=2 even), Black B on e8 (4+7=11 odd) -- different
        let board =
            Board::from_fen("4k3/8/8/8/8/8/8/2B1Kb2 w - - 0 1").unwrap();
        // White B on c1: file C=2, rank First=0, 2+0=2 even
        // Black b on f1: file F=5, rank First=0, 5+0=5 odd
        // Different parity -> sufficient material
        assert_ne!(board.game_state(), GameState::DrawByInsufficientMaterial);
    }

    #[test]
    fn en_passant() {
        // Set up en passant: White pawn on e5, Black pawn on d5, en passant on d6
        let mut board =
            Board::from_fen("rnbqkbnr/ppp1pppp/8/3pP3/8/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 3")
                .unwrap();
        // White captures en passant: e5xd6
        board.make_move(&GameMove::new("e5", "d6", None)).unwrap();
        let fen = board.to_fen();
        // After en passant: White pawn on d6, Black pawn on d5 is gone, e5 is empty
        assert!(
            fen.starts_with("rnbqkbnr/ppp1pppp/3P4/8/"),
            "En passant should place pawn on d6 and remove d5 pawn, FEN: {}",
            fen
        );
    }

    #[test]
    fn castling_kingside() {
        // Position where white can castle kingside
        let mut board =
            Board::from_fen("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQkq - 0 1").unwrap();
        board.make_move(&GameMove::new("e1", "g1", None)).unwrap();
        let fen = board.to_fen();
        // After kingside castling, king on g1 and rook on f1
        assert!(
            fen.starts_with("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R4RK1"),
            "Kingside castling failed, FEN: {}",
            fen
        );
    }

    #[test]
    fn castling_queenside() {
        let mut board =
            Board::from_fen("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQkq - 0 1").unwrap();
        board.make_move(&GameMove::new("e1", "c1", None)).unwrap();
        let fen = board.to_fen();
        // After queenside castling, king on c1 and rook on d1
        assert!(
            fen.starts_with("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/2KR3R"),
            "Queenside castling failed, FEN: {}",
            fen
        );
    }

    #[test]
    fn pawn_promotion() {
        // White pawn on a7, promote to queen
        let mut board = Board::from_fen("8/P3k3/8/8/8/8/8/4K3 w - - 0 1").unwrap();
        board
            .make_move(&GameMove::new("a7", "a8", Some("q")))
            .unwrap();
        let fen = board.to_fen();
        assert!(
            fen.starts_with("Q"),
            "Pawn should have promoted to queen, FEN: {}",
            fen
        );
    }

    #[test]
    fn pawn_promotion_to_knight() {
        let mut board = Board::from_fen("8/P3k3/8/8/8/8/8/4K3 w - - 0 1").unwrap();
        board
            .make_move(&GameMove::new("a7", "a8", Some("n")))
            .unwrap();
        let fen = board.to_fen();
        assert!(
            fen.starts_with("N"),
            "Pawn should have promoted to knight, FEN: {}",
            fen
        );
    }

    #[test]
    fn clone_and_make_does_not_modify_original() {
        let board = Board::new();
        let original_fen = board.to_fen();
        let original_move_count = board.move_count();

        let new_board = board
            .clone_and_make(&GameMove::new("e2", "e4", None))
            .unwrap();

        // Original unchanged
        assert_eq!(board.to_fen(), original_fen);
        assert_eq!(board.move_count(), original_move_count);

        // New board has the move applied
        assert_eq!(new_board.move_count(), 1);
        assert_eq!(new_board.side_to_move(), Color::Black);
    }

    #[test]
    fn hash_changes_after_move() {
        let board = Board::new();
        let hash_before = board.hash();
        let new_board = board
            .clone_and_make(&GameMove::new("e2", "e4", None))
            .unwrap();
        assert_ne!(hash_before, new_board.hash());
    }

    #[test]
    fn check_detection() {
        // Position where black king is in check: Rook on e2 gives check to Ke8 along e-file
        let board = Board::from_fen("4k3/8/8/8/8/8/4R3/4K3 b - - 0 1").unwrap();
        assert!(board.is_check());
        assert_eq!(board.game_state(), GameState::Check);
    }

    #[test]
    fn game_state_in_progress_at_start() {
        let board = Board::new();
        assert_eq!(board.game_state(), GameState::InProgress);
    }

    #[test]
    fn invalid_fen_returns_error() {
        let result = Board::from_fen("not a valid fen");
        assert!(result.is_err());
    }

    #[test]
    fn invalid_square_in_move_returns_error() {
        let mut board = Board::new();
        let result = board.make_move(&GameMove::new("z9", "e4", None));
        assert!(result.is_err());
    }

    #[test]
    fn legal_moves_after_e4() {
        let mut board = Board::new();
        board.make_move(&GameMove::new("e2", "e4", None)).unwrap();
        let moves = board.legal_moves();
        // Black should have 20 legal moves from the standard response
        assert_eq!(moves.len(), 20);
    }
}
