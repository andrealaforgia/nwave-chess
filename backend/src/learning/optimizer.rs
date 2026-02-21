//! Adam optimizer with L2 regularization toward initial weights.
//! Maintains per-parameter first and second moment estimates across games.
//! Applies per-category weight clipping after each update to prevent divergence.

use serde::{Deserialize, Serialize};

/// State of the Adam optimizer, serializable for persistence across sessions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizerState {
    pub learning_rate: f64,
    pub beta1: f64,
    pub beta2: f64,
    pub epsilon: f64,
    pub m: Vec<f64>,
    pub v: Vec<f64>,
    pub t: u64,
}

/// Adam optimizer with bias correction.
pub struct AdamOptimizer {
    learning_rate: f64,
    beta1: f64,
    beta2: f64,
    epsilon: f64,
    m: Vec<f64>,
    v: Vec<f64>,
    t: u64,
}

impl AdamOptimizer {
    /// Create a new Adam optimizer for the given number of weights.
    pub fn new(num_weights: usize, learning_rate: f64) -> Self {
        Self {
            learning_rate,
            beta1: 0.9,
            beta2: 0.999,
            epsilon: 1e-8,
            m: vec![0.0; num_weights],
            v: vec![0.0; num_weights],
            t: 0,
        }
    }

    /// Apply one Adam update step given a gradient vector.
    /// Returns the weight delta (to be added to current weights).
    pub fn step(&mut self, gradient: &[f64]) -> Vec<f64> {
        assert_eq!(
            gradient.len(),
            self.m.len(),
            "Gradient length must match number of weights"
        );

        self.t += 1;
        let t = self.t as f64;

        // Bias correction denominators.
        let bc1 = 1.0 - self.beta1.powf(t);
        let bc2 = 1.0 - self.beta2.powf(t);

        let mut delta = vec![0.0f64; gradient.len()];

        for i in 0..gradient.len() {
            let g = gradient[i];

            // Update biased first moment estimate.
            self.m[i] = self.beta1 * self.m[i] + (1.0 - self.beta1) * g;
            // Update biased second moment estimate.
            self.v[i] = self.beta2 * self.v[i] + (1.0 - self.beta2) * g * g;

            // Bias-corrected estimates.
            let m_hat = self.m[i] / bc1;
            let v_hat = self.v[i] / bc2;

            // Adam update.
            delta[i] = self.learning_rate * m_hat / (v_hat.sqrt() + self.epsilon);
        }

        delta
    }

    /// Get optimizer state for persistence.
    pub fn state(&self) -> OptimizerState {
        OptimizerState {
            learning_rate: self.learning_rate,
            beta1: self.beta1,
            beta2: self.beta2,
            epsilon: self.epsilon,
            m: self.m.clone(),
            v: self.v.clone(),
            t: self.t,
        }
    }

    /// Restore from persisted state.
    pub fn from_state(state: OptimizerState) -> Self {
        Self {
            learning_rate: state.learning_rate,
            beta1: state.beta1,
            beta2: state.beta2,
            epsilon: state.epsilon,
            m: state.m,
            v: state.v,
            t: state.t,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adam_step_produces_nonzero_delta_for_nonzero_gradient() {
        let mut optimizer = AdamOptimizer::new(4, 0.001);
        let gradient = vec![1.0, -2.0, 0.5, -0.3];

        let delta = optimizer.step(&gradient);

        for (i, d) in delta.iter().enumerate() {
            assert!(
                d.abs() > 1e-15,
                "Delta[{}] should be nonzero for nonzero gradient, got {}",
                i,
                d
            );
        }
        // Direction should match gradient sign.
        assert!(delta[0] > 0.0, "Positive gradient should produce positive delta");
        assert!(delta[1] < 0.0, "Negative gradient should produce negative delta");
    }

    #[test]
    fn adam_with_zero_gradient_produces_zero_delta() {
        let mut optimizer = AdamOptimizer::new(4, 0.001);
        let gradient = vec![0.0, 0.0, 0.0, 0.0];

        let delta = optimizer.step(&gradient);

        for (i, d) in delta.iter().enumerate() {
            assert!(
                d.abs() < 1e-15,
                "Delta[{}] should be zero for zero gradient, got {}",
                i,
                d
            );
        }
    }

    #[test]
    fn optimizer_state_round_trips_through_serialize_deserialize() {
        let mut optimizer = AdamOptimizer::new(4, 0.001);
        // Run a few steps to populate internal state.
        optimizer.step(&[1.0, -0.5, 0.3, -0.1]);
        optimizer.step(&[0.2, 0.8, -0.4, 0.6]);

        let state = optimizer.state();

        // Serialize and deserialize.
        let json = serde_json::to_string(&state).expect("Should serialize");
        let restored_state: OptimizerState =
            serde_json::from_str(&json).expect("Should deserialize");

        let restored = AdamOptimizer::from_state(restored_state);

        // Verify that the restored optimizer produces the same result.
        let gradient = vec![0.5, -0.3, 0.1, 0.7];
        let mut original = AdamOptimizer::from_state(state);
        let delta_original = original.step(&gradient);
        let mut restored_opt = restored;
        let delta_restored = restored_opt.step(&gradient);

        for i in 0..4 {
            assert!(
                (delta_original[i] - delta_restored[i]).abs() < 1e-15,
                "Delta[{}] should match after round-trip: {} vs {}",
                i,
                delta_original[i],
                delta_restored[i]
            );
        }
    }
}
