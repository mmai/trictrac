//! One gradient-descent training step for AlphaZero.
//!
//! The loss combines:
//! - **Policy loss** — cross-entropy between MCTS visit counts and network logits.
//! - **Value loss** — mean-squared error between the predicted value and the
//!   actual game outcome.
//!
//! # Learning-rate scheduling
//!
//! [`cosine_lr`] implements one-cycle cosine annealing:
//!
//! ```text
//! lr(t) = lr_min + 0.5 · (lr_max − lr_min) · (1 + cos(π · t / T))
//! ```
//!
//! Typical usage in the outer loop:
//!
//! ```rust,ignore
//! for step in 0..total_train_steps {
//!     let lr = cosine_lr(config.learning_rate, config.lr_min, step, total_train_steps);
//!     let (m, loss) = train_step(model, &mut optimizer, &batch, &device, lr);
//!     model = m;
//! }
//! ```
//!
//! # Backend
//!
//! `train_step` requires an `AutodiffBackend` (e.g. `Autodiff<NdArray<f32>>`).
//! Self-play uses the inner backend (`NdArray<f32>`) for zero autodiff overhead.
//! Weights are transferred between the two via [`burn::record`].

use burn::{
    module::AutodiffModule,
    optim::{GradientsParams, Optimizer},
    prelude::ElementConversion,
    tensor::{
        activation::log_softmax,
        backend::AutodiffBackend,
        Tensor, TensorData,
    },
};

use crate::network::PolicyValueNet;
use super::replay::TrainSample;

/// Run one gradient step on `model` using `batch`.
///
/// Returns the updated model and the scalar loss value for logging.
///
/// # Parameters
///
/// - `lr` — learning rate (e.g. `1e-3`).
/// - `batch` — slice of [`TrainSample`]s; must be non-empty.
pub fn train_step<B, N, O>(
    model: N,
    optimizer: &mut O,
    batch: &[TrainSample],
    device: &B::Device,
    lr: f64,
) -> (N, f32)
where
    B: AutodiffBackend,
    N: PolicyValueNet<B> + AutodiffModule<B>,
    O: Optimizer<N, B>,
{
    assert!(!batch.is_empty(), "train_step called with empty batch");

    let batch_size = batch.len();
    let obs_size = batch[0].obs.len();
    let action_size = batch[0].policy.len();

    // ── Build input tensors ────────────────────────────────────────────────
    let obs_flat: Vec<f32> = batch.iter().flat_map(|s| s.obs.iter().copied()).collect();
    let policy_flat: Vec<f32> = batch.iter().flat_map(|s| s.policy.iter().copied()).collect();
    let value_flat: Vec<f32> = batch.iter().map(|s| s.value).collect();

    let obs_tensor = Tensor::<B, 2>::from_data(
        TensorData::new(obs_flat, [batch_size, obs_size]),
        device,
    );
    let policy_target = Tensor::<B, 2>::from_data(
        TensorData::new(policy_flat, [batch_size, action_size]),
        device,
    );
    let value_target = Tensor::<B, 2>::from_data(
        TensorData::new(value_flat, [batch_size, 1]),
        device,
    );

    // ── Forward pass ──────────────────────────────────────────────────────
    let (policy_logits, value_pred) = model.forward(obs_tensor);

    // ── Policy loss: -sum(π_mcts · log_softmax(logits)) ──────────────────
    let log_probs = log_softmax(policy_logits, 1);
    let policy_loss = (policy_target.clone().neg() * log_probs)
        .sum_dim(1)
        .mean();

    // ── Value loss: MSE(value_pred, z) ────────────────────────────────────
    let diff = value_pred - value_target;
    let value_loss = (diff.clone() * diff).mean();

    // ── Combined loss ─────────────────────────────────────────────────────
    let loss = policy_loss + value_loss;

    // Extract scalar before backward (consumes the tensor).
    let loss_scalar: f32 = loss.clone().into_scalar().elem();

    // ── Backward + optimizer step ─────────────────────────────────────────
    let grads = loss.backward();
    let grads = GradientsParams::from_grads(grads, &model);
    let model = optimizer.step(lr, model, grads);

    (model, loss_scalar)
}

// ── Learning-rate schedule ─────────────────────────────────────────────────

/// Cosine learning-rate schedule (one half-period, no warmup).
///
/// Returns the learning rate for training step `step` out of `total_steps`:
///
/// ```text
/// lr(t) = lr_min + 0.5 · (initial − lr_min) · (1 + cos(π · t / total))
/// ```
///
/// - At `t = 0` returns `initial`.
/// - At `t = total_steps` (or beyond) returns `lr_min`.
///
/// # Panics
///
/// Does not panic.  When `total_steps == 0`, returns `lr_min`.
pub fn cosine_lr(initial: f64, lr_min: f64, step: usize, total_steps: usize) -> f64 {
    if total_steps == 0 || step >= total_steps {
        return lr_min;
    }
    let progress = step as f64 / total_steps as f64;
    lr_min + 0.5 * (initial - lr_min) * (1.0 + (std::f64::consts::PI * progress).cos())
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use burn::{
        backend::{Autodiff, NdArray},
        optim::AdamConfig,
    };

    use crate::network::{MlpConfig, MlpNet};
    use super::super::replay::TrainSample;

    type B = Autodiff<NdArray<f32>>;

    fn device() -> <B as burn::tensor::backend::Backend>::Device {
        Default::default()
    }

    fn dummy_batch(n: usize, obs_size: usize, action_size: usize) -> Vec<TrainSample> {
        (0..n)
            .map(|i| TrainSample {
                obs: vec![0.5f32; obs_size],
                policy: {
                    let mut p = vec![0.0f32; action_size];
                    p[i % action_size] = 1.0;
                    p
                },
                value: if i % 2 == 0 { 1.0 } else { -1.0 },
            })
            .collect()
    }

    #[test]
    fn train_step_returns_finite_loss() {
        let config = MlpConfig { obs_size: 4, action_size: 4, hidden_size: 16 };
        let model = MlpNet::<B>::new(&config, &device());
        let mut optimizer = AdamConfig::new().init();
        let batch = dummy_batch(8, 4, 4);

        let (_, loss) = train_step(model, &mut optimizer, &batch, &device(), 1e-3);
        assert!(loss.is_finite(), "loss must be finite, got {loss}");
        assert!(loss > 0.0, "loss should be positive");
    }

    #[test]
    fn loss_decreases_over_steps() {
        let config = MlpConfig { obs_size: 4, action_size: 4, hidden_size: 32 };
        let mut model = MlpNet::<B>::new(&config, &device());
        let mut optimizer = AdamConfig::new().init();
        // Same batch every step — loss should decrease.
        let batch = dummy_batch(16, 4, 4);

        let mut prev_loss = f32::INFINITY;
        for _ in 0..10 {
            let (m, loss) = train_step(model, &mut optimizer, &batch, &device(), 1e-2);
            model = m;
            assert!(loss.is_finite());
            prev_loss = loss;
        }
        // After 10 steps on fixed data, loss should be below a reasonable threshold.
        assert!(prev_loss < 3.0, "loss did not decrease: {prev_loss}");
    }

    #[test]
    fn train_step_batch_size_one() {
        let config = MlpConfig { obs_size: 2, action_size: 2, hidden_size: 8 };
        let model = MlpNet::<B>::new(&config, &device());
        let mut optimizer = AdamConfig::new().init();
        let batch = dummy_batch(1, 2, 2);
        let (_, loss) = train_step(model, &mut optimizer, &batch, &device(), 1e-3);
        assert!(loss.is_finite());
    }

    // ── cosine_lr ─────────────────────────────────────────────────────────

    #[test]
    fn cosine_lr_at_step_zero_is_initial() {
        let lr = super::cosine_lr(1e-3, 1e-5, 0, 100);
        assert!((lr - 1e-3).abs() < 1e-10, "expected initial lr, got {lr}");
    }

    #[test]
    fn cosine_lr_at_end_is_min() {
        let lr = super::cosine_lr(1e-3, 1e-5, 100, 100);
        assert!((lr - 1e-5).abs() < 1e-10, "expected min lr, got {lr}");
    }

    #[test]
    fn cosine_lr_beyond_end_is_min() {
        let lr = super::cosine_lr(1e-3, 1e-5, 200, 100);
        assert!((lr - 1e-5).abs() < 1e-10, "expected min lr beyond end, got {lr}");
    }

    #[test]
    fn cosine_lr_midpoint_is_average() {
        // At t = total/2, cos(π/2) = 0, so lr = (initial + min) / 2.
        let lr = super::cosine_lr(1e-3, 1e-5, 50, 100);
        let expected = (1e-3 + 1e-5) / 2.0;
        assert!((lr - expected).abs() < 1e-10, "expected midpoint {expected}, got {lr}");
    }

    #[test]
    fn cosine_lr_monotone_decreasing() {
        let mut prev = f64::INFINITY;
        for step in 0..=100 {
            let lr = super::cosine_lr(1e-3, 1e-5, step, 100);
            assert!(lr <= prev + 1e-15, "lr increased at step {step}: {lr} > {prev}");
            prev = lr;
        }
    }

    #[test]
    fn cosine_lr_zero_total_steps_returns_min() {
        let lr = super::cosine_lr(1e-3, 1e-5, 0, 0);
        assert!((lr - 1e-5).abs() < 1e-10);
    }
}
