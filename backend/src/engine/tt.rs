//! Transposition table for caching search results.
//! Fixed-size hash table keyed by Zobrist hash, storing depth, score, score type
//! (exact/lower/upper bound), and best move. Uses depth-preferred replacement.

use super::types::GameMove;

// ---------------------------------------------------------------------------
// Score type
// ---------------------------------------------------------------------------

/// Transposition table entry score type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScoreType {
    /// Score is exact (PV node).
    Exact,
    /// Score is a lower bound (beta cutoff).
    LowerBound,
    /// Score is an upper bound (alpha not improved).
    UpperBound,
}

// ---------------------------------------------------------------------------
// TT entry
// ---------------------------------------------------------------------------

/// A single transposition table entry.
#[derive(Debug, Clone)]
pub struct TTEntry {
    /// Full Zobrist hash for verification.
    pub hash: u64,
    /// Search depth this entry was computed at.
    pub depth: i32,
    /// Score in centipawns.
    pub score: i32,
    /// Score type (exact, lower bound, upper bound).
    pub score_type: ScoreType,
    /// Best move found (for move ordering).
    pub best_move: Option<GameMove>,
}

// ---------------------------------------------------------------------------
// Transposition table
// ---------------------------------------------------------------------------

/// Fixed-size transposition table with always-replace-if-deeper policy.
pub struct TranspositionTable {
    entries: Vec<Option<TTEntry>>,
    size: usize,
    hits: u64,
    misses: u64,
}

impl TranspositionTable {
    /// Create a TT with the given size in MB.
    /// Each entry is approximately 80 bytes (with Option overhead and GameMove strings).
    /// 1 MB ~ 13,000 entries conservatively.
    pub fn new(size_mb: usize) -> Self {
        // Estimate ~80 bytes per Option<TTEntry> (GameMove has String fields).
        // Use a power-of-2 friendly size for fast modulo.
        let entry_size = 80;
        let num_entries = (size_mb * 1024 * 1024) / entry_size;
        // Round down to power of 2 for fast indexing.
        let size = if num_entries == 0 {
            1024
        } else {
            num_entries.next_power_of_two() >> 1
        };
        let size = size.max(1024);

        Self {
            entries: vec![None; size],
            size,
            hits: 0,
            misses: 0,
        }
    }

    /// Probe the TT. Returns entry if hash matches.
    pub fn probe(&mut self, hash: u64) -> Option<&TTEntry> {
        let index = (hash as usize) % self.size;
        match &self.entries[index] {
            Some(entry) if entry.hash == hash => {
                self.hits += 1;
                // Return reference to the inner TTEntry.
                self.entries[index].as_ref()
            }
            _ => {
                self.misses += 1;
                None
            }
        }
    }

    /// Store an entry. Replacement policy: always replace if new depth >= stored depth.
    pub fn store(
        &mut self,
        hash: u64,
        depth: i32,
        score: i32,
        score_type: ScoreType,
        best_move: Option<GameMove>,
    ) {
        let index = (hash as usize) % self.size;
        let should_replace = match &self.entries[index] {
            None => true,
            Some(existing) => depth >= existing.depth || existing.hash == hash,
        };
        if should_replace {
            self.entries[index] = Some(TTEntry {
                hash,
                depth,
                score,
                score_type,
                best_move,
            });
        }
    }

    /// Clear all entries (between games).
    pub fn clear(&mut self) {
        for entry in self.entries.iter_mut() {
            *entry = None;
        }
        self.hits = 0;
        self.misses = 0;
    }

    /// Hit rate for diagnostics.
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }

    /// Total number of probes (hits + misses).
    pub fn total_probes(&self) -> u64 {
        self.hits + self.misses
    }

    /// Number of hits.
    pub fn hits(&self) -> u64 {
        self.hits
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::types::GameMove;

    #[test]
    fn store_and_probe_returns_correct_entry() {
        let mut tt = TranspositionTable::new(1);
        let hash = 0xDEAD_BEEF_CAFE_BABEu64;
        let best_move = Some(GameMove::new("e2", "e4", None));

        tt.store(hash, 5, 150, ScoreType::Exact, best_move.clone());

        let entry = tt.probe(hash).expect("Should find stored entry");
        assert_eq!(entry.hash, hash);
        assert_eq!(entry.depth, 5);
        assert_eq!(entry.score, 150);
        assert_eq!(entry.score_type, ScoreType::Exact);
        assert_eq!(entry.best_move, best_move);
    }

    #[test]
    fn probe_miss_returns_none() {
        let mut tt = TranspositionTable::new(1);
        let hash = 0x1234_5678_9ABC_DEF0u64;

        // Never stored this hash.
        let result = tt.probe(hash);
        assert!(result.is_none(), "Probe of unstored hash should return None");
    }

    #[test]
    fn higher_depth_replaces_lower_depth() {
        let mut tt = TranspositionTable::new(1);
        let hash = 0xAAAA_BBBB_CCCC_DDDDu64;

        // Store at depth 3.
        tt.store(hash, 3, 100, ScoreType::Exact, None);
        let entry = tt.probe(hash).expect("Should find entry at depth 3");
        assert_eq!(entry.depth, 3);
        assert_eq!(entry.score, 100);

        // Store at depth 5 (higher) -- should replace.
        tt.store(hash, 5, 200, ScoreType::LowerBound, None);
        let entry = tt.probe(hash).expect("Should find replaced entry");
        assert_eq!(entry.depth, 5);
        assert_eq!(entry.score, 200);
        assert_eq!(entry.score_type, ScoreType::LowerBound);
    }

    #[test]
    fn hit_rate_is_calculated_correctly() {
        let mut tt = TranspositionTable::new(1);
        let hash1 = 0x1111_1111_1111_1111u64;
        let hash2 = 0x2222_2222_2222_2222u64;

        tt.store(hash1, 3, 50, ScoreType::Exact, None);

        // One hit.
        tt.probe(hash1);
        // One miss.
        tt.probe(hash2);

        let rate = tt.hit_rate();
        assert!(
            (rate - 0.5).abs() < 0.01,
            "Hit rate should be 0.5 (1 hit, 1 miss), got {}",
            rate
        );
    }
}
