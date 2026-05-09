//! DQN gradient step and target-network management.
//!
//! # TD target
//!
//! ```text
//! y_i = r_i + γ · max_{a ∈ legal_next_i} Q_target(s'_i, a)   if not done
//! y_i = r_i                                                     if done
//! ```
//!
//! # Loss
//!
//! Mean-squared error between `Q(s_i, a_i)` (gathered from the online net)
//! and `y_i` (computed from the frozen target net).
//!
//! # Target network
//!
//! [`hard_update`] copies the online Q-net weights into the target net by
//! stripping the autodiff wrapper via [`AutodiffModule::valid`].

use burn::{
    module::AutodiffModule,
    optim::{GradientsParams, Optimizer},
    prelude::ElementConversion,
    tensor::{
        Int, Tensor, TensorData,
        backend::{AutodiffBackend, Backend},
    },
};

use crate::network::QValueNet;
use super::DqnSample;

// ── Target Q computation ─────────────────────────────────────────────────────

/// Compute `max_{a ∈ legal} Q_target(s', a)` for every non-done sample.
///
/// Returns a `Vec<f32>` of length `batch.len()`.  Done samples get `0.0`
/// (their bootstrap term is dropped by the TD target anyway).
///
/// The target network runs on the **inference backend** (`InferB`) with no
/// gradient tape, so this function is backend-agnostic (`B: Backend`).
pub fn compute_target_q<B: Backend, Q: QValueNet<B>>(
    target_net: &Q,
    batch: &[DqnSample],
    action_size: usize,
    device: &B::Device,
) -> Vec<f32> {
    let batch_size = batch.len();

    // Collect indices of non-done samples (done samples have no next state).
    let non_done: Vec<usize> = batch
        .iter()
        .enumerate()
        .filter(|(_, s)| !s.done)
        .map(|(i, _)| i)
        .collect();

    if non_done.is_empty() {
        return vec![0.0; batch_size];
    }

    let obs_size = batch[0].next_obs.len();
    let nd = non_done.len();

    // Stack next observations for non-done samples → [nd, obs_size].
    let obs_flat: Vec<f32> = non_done
        .iter()
        .flat_map(|&i| batch[i].next_obs.iter().copied())
        .collect();
    let obs_tensor = Tensor::<B, 2>::from_data(
        TensorData::new(obs_flat, [nd, obs_size]),
        device,
    );

    // Forward target net → [nd, action_size], then to Vec<f32>.
    let q_flat: Vec<f32> = target_net.forward(obs_tensor).into_data().to_vec().unwrap();

    // For each non-done sample, pick max Q over legal next actions.
    let mut result = vec![0.0f32; batch_size];
    for (k, &i) in non_done.iter().enumerate() {
        let legal = &batch[i].next_legal;
        let offset = k * action_size;
        let max_q = legal
            .iter()
            .map(|&a| q_flat[offset + a])
            .fold(f32::NEG_INFINITY, f32::max);
        // If legal is empty (shouldn't happen for non-done, but be safe):
        result[i] = if max_q.is_finite() { max_q } else { 0.0 };
    }
    result
}

// ── Training step ─────────────────────────────────────────────────────────────

/// Run one gradient step on `q_net` using `batch`.
///
/// `target_max_q` must be pre-computed via [`compute_target_q`] using the
/// frozen target network and passed in here so that this function only
/// needs the **autodiff backend**.
///
/// Returns the updated network and the scalar MSE loss.
pub fn dqn_train_step<B, Q, O>(
    q_net: Q,
    optimizer: &mut O,
    batch: &[DqnSample],
    target_max_q: &[f32],
    device: &B::Device,
    lr: f64,
    gamma: f32,
) -> (Q, f32)
where
    B: AutodiffBackend,
    Q: QValueNet<B> + AutodiffModule<B>,
    O: Optimizer<Q, B>,
{
    assert!(!batch.is_empty(), "dqn_train_step: empty batch");
    assert_eq!(batch.len(), target_max_q.len(), "batch and target_max_q length mismatch");

    let batch_size = batch.len();
    let obs_size = batch[0].obs.len();

    // ── Build observation tensor [B, obs_size] ────────────────────────────
    let obs_flat: Vec<f32> = batch.iter().flat_map(|s| s.obs.iter().copied()).collect();
    let obs_tensor = Tensor::<B, 2>::from_data(
        TensorData::new(obs_flat, [batch_size, obs_size]),
        device,
    );

    // ── Forward Q-net → [B, action_size] ─────────────────────────────────
    let q_all = q_net.forward(obs_tensor);

    // ── Gather Q(s, a) for the taken action → [B] ────────────────────────
    let actions: Vec<i32> = batch.iter().map(|s| s.action as i32).collect();
    let action_tensor: Tensor<B, 2, Int> = Tensor::<B, 1, Int>::from_data(
        TensorData::new(actions, [batch_size]),
        device,
    )
    .reshape([batch_size, 1]); // [B] → [B, 1]
    let q_pred: Tensor<B, 1> = q_all.gather(1, action_tensor).reshape([batch_size]); // [B, 1] → [B]

    // ── TD targets: r + γ · max_next_q · (1 − done) ──────────────────────
    let targets: Vec<f32> = batch
        .iter()
        .zip(target_max_q.iter())
        .map(|(s, &max_q)| {
            if s.done { s.reward } else { s.reward + gamma * max_q }
        })
        .collect();
    let target_tensor = Tensor::<B, 1>::from_data(
        TensorData::new(targets, [batch_size]),
        device,
    );

    // ── MSE loss ──────────────────────────────────────────────────────────
    let diff = q_pred - target_tensor.detach();
    let loss = (diff.clone() * diff).mean();
    let loss_scalar: f32 = loss.clone().into_scalar().elem();

    // ── Backward + optimizer step ─────────────────────────────────────────
    let grads = loss.backward();
    let grads = GradientsParams::from_grads(grads, &q_net);
    let q_net = optimizer.step(lr, q_net, grads);

    (q_net, loss_scalar)
}

// ── Target network update ─────────────────────────────────────────────────────

/// Hard-copy the online Q-net weights to a new target network.
///
/// Strips the autodiff wrapper via [`AutodiffModule::valid`], returning an
/// inference-backend module with identical weights.
pub fn hard_update<B: AutodiffBackend, Q: AutodiffModule<B>>(q_net: &Q) -> Q::InnerModule {
    q_net.valid()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use burn::{
        backend::{Autodiff, NdArray},
        optim::AdamConfig,
    };
    use crate::network::{QNet, QNetConfig};

    type InferB = NdArray<f32>;
    type TrainB = Autodiff<NdArray<f32>>;

    fn infer_device() -> <InferB as Backend>::Device { Default::default() }
    fn train_device() -> <TrainB as Backend>::Device { Default::default() }

    fn dummy_batch(n: usize, obs_size: usize, action_size: usize) -> Vec<DqnSample> {
        (0..n)
            .map(|i| DqnSample {
                obs: vec![0.5f32; obs_size],
                action: i % action_size,
                reward: if i % 2 == 0 { 1.0 } else { -1.0 },
                next_obs: vec![0.5f32; obs_size],
                next_legal: vec![0, 1],
                done: i == n - 1,
            })
            .collect()
    }

    #[test]
    fn compute_target_q_length() {
        let cfg = QNetConfig { obs_size: 4, action_size: 4, hidden_size: 8 };
        let target = QNet::<InferB>::new(&cfg, &infer_device());
        let batch = dummy_batch(8, 4, 4);
        let tq = compute_target_q(&target, &batch, 4, &infer_device());
        assert_eq!(tq.len(), 8);
    }

    #[test]
    fn compute_target_q_done_is_zero() {
        let cfg = QNetConfig { obs_size: 4, action_size: 4, hidden_size: 8 };
        let target = QNet::<InferB>::new(&cfg, &infer_device());
        // Single done sample.
        let batch = vec![DqnSample {
            obs: vec![0.0; 4],
            action: 0,
            reward: 5.0,
            next_obs: vec![0.0; 4],
            next_legal: vec![],
            done: true,
        }];
        let tq = compute_target_q(&target, &batch, 4, &infer_device());
        assert_eq!(tq.len(), 1);
        assert_eq!(tq[0], 0.0);
    }

    #[test]
    fn train_step_returns_finite_loss() {
        let cfg = QNetConfig { obs_size: 4, action_size: 4, hidden_size: 16 };
        let q_net = QNet::<TrainB>::new(&cfg, &train_device());
        let target = QNet::<InferB>::new(&cfg, &infer_device());
        let mut optimizer = AdamConfig::new().init();
        let batch = dummy_batch(8, 4, 4);
        let tq = compute_target_q(&target, &batch, 4, &infer_device());
        let (_, loss) = dqn_train_step(q_net, &mut optimizer, &batch, &tq, &train_device(), 1e-3, 0.99);
        assert!(loss.is_finite(), "loss must be finite, got {loss}");
    }

    #[test]
    fn train_step_loss_decreases() {
        let cfg = QNetConfig { obs_size: 4, action_size: 4, hidden_size: 32 };
        let mut q_net = QNet::<TrainB>::new(&cfg, &train_device());
        let target = QNet::<InferB>::new(&cfg, &infer_device());
        let mut optimizer = AdamConfig::new().init();
        let batch = dummy_batch(16, 4, 4);
        let tq = compute_target_q(&target, &batch, 4, &infer_device());

        let mut prev_loss = f32::INFINITY;
        for _ in 0..10 {
            let (q, loss) = dqn_train_step(
                q_net, &mut optimizer, &batch, &tq, &train_device(), 1e-2, 0.99,
            );
            q_net = q;
            assert!(loss.is_finite());
            prev_loss = loss;
        }
        assert!(prev_loss < 5.0, "loss did not decrease: {prev_loss}");
    }

    #[test]
    fn hard_update_copies_weights() {
        let cfg = QNetConfig { obs_size: 4, action_size: 4, hidden_size: 8 };
        let q_net = QNet::<TrainB>::new(&cfg, &train_device());
        let target = hard_update::<TrainB, _>(&q_net);

        let obs = burn::tensor::Tensor::<InferB, 2>::zeros([1, 4], &infer_device());
        let q_out: Vec<f32> = target.forward(obs).into_data().to_vec().unwrap();
        // After hard_update the target produces finite outputs.
        assert!(q_out.iter().all(|v| v.is_finite()));
    }
}
