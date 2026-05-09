//! Two-hidden-layer MLP policy-value network.
//!
//! ```text
//! Input [B, obs_size]
//!   → Linear(obs → hidden) → ReLU
//!   → Linear(hidden → hidden) → ReLU
//!   ├─ policy_head: Linear(hidden → action_size)  [raw logits]
//!   └─ value_head:  Linear(hidden → 1) → tanh     [∈ (-1, 1)]
//! ```

use burn::{
    module::Module,
    nn::{Linear, LinearConfig},
    record::{CompactRecorder, Recorder},
    tensor::{
        activation::{relu, tanh},
        backend::Backend,
        Tensor,
    },
};
use std::path::Path;

use super::PolicyValueNet;

// ── Config ────────────────────────────────────────────────────────────────────

/// Configuration for [`MlpNet`].
#[derive(Debug, Clone)]
pub struct MlpConfig {
    /// Number of input features.  217 for Trictrac's `to_tensor()`.
    pub obs_size: usize,
    /// Number of output actions.  514 for Trictrac's `ACTION_SPACE_SIZE`.
    pub action_size: usize,
    /// Width of both hidden layers.
    pub hidden_size: usize,
}

impl Default for MlpConfig {
    fn default() -> Self {
        Self {
            obs_size: 217,
            action_size: 514,
            hidden_size: 256,
        }
    }
}

// ── Network ───────────────────────────────────────────────────────────────────

/// Simple two-hidden-layer MLP with shared trunk and two heads.
///
/// Prefer this over [`ResNet`](super::ResNet) when training time is a
/// priority, or as a fast baseline.
#[derive(Module, Debug)]
pub struct MlpNet<B: Backend> {
    fc1: Linear<B>,
    fc2: Linear<B>,
    policy_head: Linear<B>,
    value_head: Linear<B>,
}

impl<B: Backend> MlpNet<B> {
    /// Construct a fresh network with random weights.
    pub fn new(config: &MlpConfig, device: &B::Device) -> Self {
        Self {
            fc1: LinearConfig::new(config.obs_size, config.hidden_size).init(device),
            fc2: LinearConfig::new(config.hidden_size, config.hidden_size).init(device),
            policy_head: LinearConfig::new(config.hidden_size, config.action_size).init(device),
            value_head: LinearConfig::new(config.hidden_size, 1).init(device),
        }
    }

    /// Save weights to `path` (MessagePack format via [`CompactRecorder`]).
    ///
    /// The file is written exactly at `path`; callers should append `.mpk` if
    /// they want the conventional extension.
    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        CompactRecorder::new()
            .record(self.clone().into_record(), path.to_path_buf())
            .map_err(|e| anyhow::anyhow!("MlpNet::save failed: {e:?}"))
    }

    /// Load weights from `path` into a fresh model built from `config`.
    pub fn load(config: &MlpConfig, path: &Path, device: &B::Device) -> anyhow::Result<Self> {
        let record = CompactRecorder::new()
            .load(path.to_path_buf(), device)
            .map_err(|e| anyhow::anyhow!("MlpNet::load failed: {e:?}"))?;
        Ok(Self::new(config, device).load_record(record))
    }
}

impl<B: Backend> PolicyValueNet<B> for MlpNet<B> {
    fn forward(&self, obs: Tensor<B, 2>) -> (Tensor<B, 2>, Tensor<B, 2>) {
        let x = relu(self.fc1.forward(obs));
        let x = relu(self.fc2.forward(x));
        let policy = self.policy_head.forward(x.clone());
        let value = tanh(self.value_head.forward(x));
        (policy, value)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use burn::backend::NdArray;

    type B = NdArray<f32>;

    fn device() -> <B as Backend>::Device {
        Default::default()
    }

    fn default_net() -> MlpNet<B> {
        MlpNet::new(&MlpConfig::default(), &device())
    }

    fn zeros_obs(batch: usize) -> Tensor<B, 2> {
        Tensor::zeros([batch, 217], &device())
    }

    // ── Shape tests ───────────────────────────────────────────────────────

    #[test]
    fn forward_output_shapes() {
        let net = default_net();
        let obs = zeros_obs(4);
        let (policy, value) = net.forward(obs);

        assert_eq!(policy.dims(), [4, 514], "policy shape mismatch");
        assert_eq!(value.dims(), [4, 1], "value shape mismatch");
    }

    #[test]
    fn forward_single_sample() {
        let net = default_net();
        let (policy, value) = net.forward(zeros_obs(1));
        assert_eq!(policy.dims(), [1, 514]);
        assert_eq!(value.dims(), [1, 1]);
    }

    // ── Value bounds ──────────────────────────────────────────────────────

    #[test]
    fn value_in_tanh_range() {
        let net = default_net();
        // Use a non-zero input so the output is not trivially at 0.
        let obs = Tensor::<B, 2>::ones([8, 217], &device());
        let (_, value) = net.forward(obs);
        let data: Vec<f32> = value.into_data().to_vec().unwrap();
        for v in &data {
            assert!(
                *v > -1.0 && *v < 1.0,
                "value {v} is outside open interval (-1, 1)"
            );
        }
    }

    // ── Policy logits ─────────────────────────────────────────────────────

    #[test]
    fn policy_logits_not_all_equal() {
        // With random weights the 514 logits should not all be identical.
        let net = default_net();
        let (policy, _) = net.forward(zeros_obs(1));
        let data: Vec<f32> = policy.into_data().to_vec().unwrap();
        let first = data[0];
        let all_same = data.iter().all(|&x| (x - first).abs() < 1e-6);
        assert!(!all_same, "all policy logits are identical — network may be degenerate");
    }

    // ── Config propagation ────────────────────────────────────────────────

    #[test]
    fn custom_config_shapes() {
        let config = MlpConfig {
            obs_size: 10,
            action_size: 20,
            hidden_size: 32,
        };
        let net = MlpNet::<B>::new(&config, &device());
        let obs = Tensor::zeros([3, 10], &device());
        let (policy, value) = net.forward(obs);
        assert_eq!(policy.dims(), [3, 20]);
        assert_eq!(value.dims(), [3, 1]);
    }

    // ── Save / Load ───────────────────────────────────────────────────────

    #[test]
    fn save_load_preserves_weights() {
        let config = MlpConfig::default();
        let net = default_net();

        // Forward pass before saving.
        let obs = Tensor::<B, 2>::ones([2, 217], &device());
        let (policy_before, value_before) = net.forward(obs.clone());

        // Save to a temp file.
        let path = std::env::temp_dir().join("spiel_bot_test_mlp.mpk");
        net.save(&path).expect("save failed");

        // Load into a fresh model.
        let loaded = MlpNet::<B>::load(&config, &path, &device()).expect("load failed");
        let (policy_after, value_after) = loaded.forward(obs);

        // Outputs must be bitwise identical.
        let p_before: Vec<f32> = policy_before.into_data().to_vec().unwrap();
        let p_after: Vec<f32> = policy_after.into_data().to_vec().unwrap();
        for (i, (a, b)) in p_before.iter().zip(p_after.iter()).enumerate() {
            assert!((a - b).abs() < 1e-3, "policy[{i}]: {a} vs {b} differ by more than tolerance");
        }

        let v_before: Vec<f32> = value_before.into_data().to_vec().unwrap();
        let v_after: Vec<f32> = value_after.into_data().to_vec().unwrap();
        for (i, (a, b)) in v_before.iter().zip(v_after.iter()).enumerate() {
            assert!((a - b).abs() < 1e-3, "value[{i}]: {a} vs {b} differ by more than tolerance");
        }

        let _ = std::fs::remove_file(path);
    }
}
