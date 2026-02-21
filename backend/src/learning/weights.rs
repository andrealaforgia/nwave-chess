//! Weight management, versioning, and clipping.
//! Handles loading, saving, and versioning of evaluation feature weights.
//! Enforces per-category weight bounds and provides rollback to previous versions.

use crate::engine::eval::{EvalWeights, FeatureCategory, NUM_WEIGHTS};

use super::optimizer::AdamOptimizer;

/// Weight configuration for clipping and regularization.
pub struct WeightConfig {
    /// Per-weight (min, max) clip ranges.
    pub clip_ranges: Vec<(f64, f64)>,
    /// Per-weight L2 regularization strength.
    pub regularization_strengths: Vec<f64>,
    /// Initial weights used as the regularization anchor.
    pub initial_weights: Vec<f64>,
}

impl WeightConfig {
    /// Create default config based on feature categories.
    /// Piece values get tight clipping; PSQT tables get wider ranges; bonus features moderate.
    pub fn default_config() -> Self {
        let initial = EvalWeights::default_weights().to_vec();
        let mut clip_ranges = Vec::with_capacity(NUM_WEIGHTS);
        let mut reg_strengths = Vec::with_capacity(NUM_WEIGHTS);

        for i in 0..NUM_WEIGHTS {
            let category = weight_category(i);
            let (clip, reg) = match category {
                FeatureCategory::PieceValue => {
                    // Piece values: allow +/-50% from initial.
                    let init = initial[i];
                    let margin = init.abs() * 0.5;
                    ((init - margin, init + margin), 0.001)
                }
                FeatureCategory::PieceSquareTable => {
                    // PSQT: allow wide range.
                    ((-100.0, 100.0), 0.0001)
                }
                FeatureCategory::Mobility => {
                    // Mobility: moderate range.
                    ((-20.0, 20.0), 0.0005)
                }
                FeatureCategory::BishopPair => {
                    // Bishop pair bonus: keep positive, moderate range.
                    ((0.0, 100.0), 0.0005)
                }
                FeatureCategory::RookPlacement => {
                    // Rook file bonuses: keep positive, moderate range.
                    ((0.0, 50.0), 0.0005)
                }
            };
            clip_ranges.push(clip);
            reg_strengths.push(reg);
        }

        Self {
            clip_ranges,
            regularization_strengths: reg_strengths,
            initial_weights: initial,
        }
    }
}

/// Summary of a weight update.
#[derive(Debug, Clone, serde::Serialize)]
pub struct WeightUpdateSummary {
    pub version: u32,
    pub weight_change_norm: f64,
    pub top_changes: Vec<FeatureChange>,
}

/// Describes how a single feature weight changed.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FeatureChange {
    pub name: String,
    pub old_value: f64,
    pub new_value: f64,
    pub change_pct: f64,
}

/// Manages weights with versioning, regularization, and clipping.
pub struct WeightManager {
    current_weights: EvalWeights,
    weight_vec: Vec<f64>,
    version: u32,
    config: WeightConfig,
}

impl WeightManager {
    pub fn new(initial_weights: EvalWeights) -> Self {
        let weight_vec = initial_weights.to_vec();
        Self {
            current_weights: initial_weights,
            weight_vec,
            version: 0,
            config: WeightConfig::default_config(),
        }
    }

    /// Apply a TD-Leaf update: gradient -> Adam -> regularization -> clipping.
    pub fn apply_update(
        &mut self,
        optimizer: &mut AdamOptimizer,
        gradient: &[f64],
        learning_rate_multiplier: f64,
    ) -> WeightUpdateSummary {
        let old_vec = self.weight_vec.clone();

        // Apply L2 regularization to the gradient: g_i += reg_i * (w_i - w_init_i)
        let mut reg_gradient = gradient.to_vec();
        for i in 0..NUM_WEIGHTS {
            let reg = self.config.regularization_strengths[i];
            if reg > 0.0 {
                reg_gradient[i] -= reg * (self.weight_vec[i] - self.config.initial_weights[i]);
            }
        }

        // Get the Adam delta.
        let delta = optimizer.step(&reg_gradient);

        // Apply the delta with the learning rate multiplier.
        for i in 0..NUM_WEIGHTS {
            self.weight_vec[i] += delta[i] * learning_rate_multiplier;
        }

        // Clip weights to their allowed ranges.
        for i in 0..NUM_WEIGHTS {
            let (lo, hi) = self.config.clip_ranges[i];
            self.weight_vec[i] = self.weight_vec[i].clamp(lo, hi);
        }

        // Update the structured weights.
        self.current_weights =
            EvalWeights::from_vec(&self.weight_vec).expect("Weight vec should be valid");
        self.version += 1;

        // Compute summary.
        let change_norm: f64 = self
            .weight_vec
            .iter()
            .zip(old_vec.iter())
            .map(|(n, o)| (n - o).powi(2))
            .sum::<f64>()
            .sqrt();

        let top_changes = self.top_changed_features(&old_vec, 5);

        WeightUpdateSummary {
            version: self.version,
            weight_change_norm: change_norm,
            top_changes,
        }
    }

    pub fn current_weights(&self) -> &EvalWeights {
        &self.current_weights
    }

    pub fn version(&self) -> u32 {
        self.version
    }

    pub fn weight_vec(&self) -> &[f64] {
        &self.weight_vec
    }

    /// Get the top N features that changed the most (by absolute change).
    pub fn top_changed_features(&self, old_vec: &[f64], n: usize) -> Vec<FeatureChange> {
        let mut changes: Vec<(usize, f64)> = self
            .weight_vec
            .iter()
            .zip(old_vec.iter())
            .enumerate()
            .map(|(i, (new, old))| (i, (new - old).abs()))
            .filter(|(_, abs_change)| *abs_change > 1e-15)
            .collect();

        changes.sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        changes.truncate(n);

        changes
            .into_iter()
            .map(|(i, _)| {
                let old = old_vec[i];
                let new = self.weight_vec[i];
                let change_pct = if old.abs() > 1e-10 {
                    ((new - old) / old.abs()) * 100.0
                } else {
                    0.0
                };
                FeatureChange {
                    name: weight_name(i),
                    old_value: old,
                    new_value: new,
                    change_pct,
                }
            })
            .collect()
    }
}

/// Map a weight index to its feature category.
fn weight_category(index: usize) -> FeatureCategory {
    match index {
        0..10 => FeatureCategory::PieceValue,
        10..778 => FeatureCategory::PieceSquareTable,
        778..790 => FeatureCategory::Mobility,
        790..792 => FeatureCategory::BishopPair,
        792..796 => FeatureCategory::RookPlacement,
        _ => panic!("Weight index {} out of range", index),
    }
}

/// Human-readable name for a weight index.
fn weight_name(index: usize) -> String {
    let piece_names = ["pawn", "knight", "bishop", "rook", "queen"];
    let piece_names_6 = ["pawn", "knight", "bishop", "rook", "queen", "king"];

    match index {
        0..5 => format!("{}_value_mg", piece_names[index]),
        5..10 => format!("{}_value_eg", piece_names[index - 5]),
        10..394 => {
            let rel = index - 10;
            let piece = rel / 64;
            let sq = rel % 64;
            format!("psqt_mg_{}_{}", piece_names_6[piece], sq)
        }
        394..778 => {
            let rel = index - 394;
            let piece = rel / 64;
            let sq = rel % 64;
            format!("psqt_eg_{}_{}", piece_names_6[piece], sq)
        }
        778..784 => format!("mobility_mg_{}", piece_names_6[index - 778]),
        784..790 => format!("mobility_eg_{}", piece_names_6[index - 784]),
        790 => "bishop_pair_mg".to_string(),
        791 => "bishop_pair_eg".to_string(),
        792 => "rook_open_file_mg".to_string(),
        793 => "rook_open_file_eg".to_string(),
        794 => "rook_semi_open_file_mg".to_string(),
        795 => "rook_semi_open_file_eg".to_string(),
        _ => format!("unknown_{}", index),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::eval::NUM_WEIGHTS;
    use crate::learning::optimizer::AdamOptimizer;

    #[test]
    fn weight_clipping_enforces_bounds() {
        let weights = EvalWeights::default_weights();
        let mut manager = WeightManager::new(weights);
        let mut optimizer = AdamOptimizer::new(NUM_WEIGHTS, 100.0); // Very large LR to force clipping

        // Create a large gradient that will push weights far beyond clip bounds.
        let gradient = vec![1000.0; NUM_WEIGHTS];
        manager.apply_update(&mut optimizer, &gradient, 1.0);

        // Every weight should be within its clip range.
        for i in 0..NUM_WEIGHTS {
            let (lo, hi) = manager.config.clip_ranges[i];
            let w = manager.weight_vec()[i];
            assert!(
                w >= lo - 1e-10 && w <= hi + 1e-10,
                "Weight {} = {} should be in [{}, {}]",
                i,
                w,
                lo,
                hi
            );
        }
    }

    #[test]
    fn l2_regularization_pulls_weights_toward_initial_values() {
        let weights = EvalWeights::default_weights();
        let initial_vec = weights.to_vec();
        let mut manager = WeightManager::new(weights);

        // Manually push weights away from initial.
        for i in 0..NUM_WEIGHTS {
            manager.weight_vec[i] += 10.0;
        }
        manager.current_weights =
            EvalWeights::from_vec(&manager.weight_vec).expect("Valid weights");

        let mut optimizer = AdamOptimizer::new(NUM_WEIGHTS, 0.1);

        // Zero gradient: only regularization should act.
        let gradient = vec![0.0; NUM_WEIGHTS];
        manager.apply_update(&mut optimizer, &gradient, 1.0);

        // Count how many weights moved back toward initial (decreased, since we pushed +10).
        let moved_toward_initial = (0..NUM_WEIGHTS)
            .filter(|&i| {
                let reg = manager.config.regularization_strengths[i];
                if reg > 0.0 {
                    // Weight was at initial + 10, it should have moved closer to initial.
                    let current = manager.weight_vec()[i];
                    current < initial_vec[i] + 10.0
                } else {
                    // No regularization, skip.
                    true
                }
            })
            .count();

        assert_eq!(
            moved_toward_initial, NUM_WEIGHTS,
            "All regularized weights should move toward initial values"
        );
    }

    #[test]
    fn top_changed_features_identifies_the_largest_changes() {
        let weights = EvalWeights::default_weights();
        let mut manager = WeightManager::new(weights);

        let old_vec = manager.weight_vec().to_vec();

        // Manually change specific weights by known amounts.
        manager.weight_vec[0] += 50.0; // pawn_value_mg
        manager.weight_vec[790] += 30.0; // bishop_pair_mg
        manager.weight_vec[100] += 10.0; // some psqt weight
        manager.current_weights =
            EvalWeights::from_vec(&manager.weight_vec).expect("Valid weights");

        let top = manager.top_changed_features(&old_vec, 3);

        assert_eq!(top.len(), 3, "Should return top 3 changes");
        // The largest change should be index 0 (changed by 50).
        assert_eq!(
            top[0].name, "pawn_value_mg",
            "Largest change should be pawn_value_mg"
        );
        assert!(
            (top[0].new_value - top[0].old_value - 50.0).abs() < 1e-10,
            "Change should be 50.0"
        );
        // Second largest should be bishop_pair_mg (changed by 30).
        assert_eq!(
            top[1].name, "bishop_pair_mg",
            "Second largest change should be bishop_pair_mg"
        );
    }
}
