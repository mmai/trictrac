//! Residual-block policy-value network.
//!
//! ```text
//! Input [B, obs_size]
//!   → Linear(obs → hidden) → ReLU          (input projection)
//!   → ResBlock × 4                          (residual trunk)
//!   ├─ policy_head: Linear(hidden → action_size)  [raw logits]
//!   └─ value_head:  Linear(hidden → 1) → tanh     [∈ (-1, 1)]
//!
//! ResBlock:
//!   x → Linear → ReLU → Linear → (+x) → ReLU
//! ```
//!
//! Compared to [`MlpNet`](super::MlpNet) this network is deeper and better
//! suited for long training runs where board-pattern recognition matters.

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

/// Configuration for [`ResNet`].
#[derive(Debug, Clone)]
pub struct ResNetConfig {
    /// Number of input features.  217 for Trictrac's `to_tensor()`.
    pub obs_size: usize,
    /// Number of output actions.  514 for Trictrac's `ACTION_SPACE_SIZE`.
    pub action_size: usize,
    /// Width of all hidden layers (input projection + residual blocks).
    pub hidden_size: usize,
}

impl Default for ResNetConfig {
    fn default() -> Self {
        Self {
            obs_size: 217,
            action_size: 514,
            hidden_size: 512,
        }
    }
}

// ── Residual block ────────────────────────────────────────────────────────────

/// A single residual block: `x ↦ ReLU(fc2(ReLU(fc1(x))) + x)`.
///
/// Both linear layers preserve the hidden dimension so the skip connection
/// can be added without projection.
#[derive(Module, Debug)]
struct ResBlock<B: Backend> {
    fc1: Linear<B>,
    fc2: Linear<B>,
}

impl<B: Backend> ResBlock<B> {
    fn new(hidden: usize, device: &B::Device) -> Self {
        Self {
            fc1: LinearConfig::new(hidden, hidden).init(device),
            fc2: LinearConfig::new(hidden, hidden).init(device),
        }
    }

    fn forward(&self, x: Tensor<B, 2>) -> Tensor<B, 2> {
        let residual = x.clone();
        let out = relu(self.fc1.forward(x));
        relu(self.fc2.forward(out) + residual)
    }
}

// ── Network ───────────────────────────────────────────────────────────────────

/// Four-residual-block policy-value network.
///
/// Prefer this over [`MlpNet`](super::MlpNet) for longer training runs and
/// when representing complex positional patterns is important.
#[derive(Module, Debug)]
pub struct ResNet<B: Backend> {
    input: Linear<B>,
    block0: ResBlock<B>,
    block1: ResBlock<B>,
    block2: ResBlock<B>,
    block3: ResBlock<B>,
    policy_head: Linear<B>,
    value_head: Linear<B>,
}

impl<B: Backend> ResNet<B> {
    /// Construct a fresh network with random weights.
    pub fn new(config: &ResNetConfig, device: &B::Device) -> Self {
        let h = config.hidden_size;
        Self {
            input: LinearConfig::new(config.obs_size, h).init(device),
            block0: ResBlock::new(h, device),
            block1: ResBlock::new(h, device),
            block2: ResBlock::new(h, device),
            block3: ResBlock::new(h, device),
            policy_head: LinearConfig::new(h, config.action_size).init(device),
            value_head: LinearConfig::new(h, 1).init(device),
        }
    }

    /// Save weights to `path` (MessagePack format via [`CompactRecorder`]).
    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        CompactRecorder::new()
            .record(self.clone().into_record(), path.to_path_buf())
            .map_err(|e| anyhow::anyhow!("ResNet::save failed: {e:?}"))
    }

    /// Load weights from `path` into a fresh model built from `config`.
    pub fn load(config: &ResNetConfig, path: &Path, device: &B::Device) -> anyhow::Result<Self> {
        let record = CompactRecorder::new()
            .load(path.to_path_buf(), device)
            .map_err(|e| anyhow::anyhow!("ResNet::load failed: {e:?}"))?;
        Ok(Self::new(config, device).load_record(record))
    }
}

impl<B: Backend> PolicyValueNet<B> for ResNet<B> {
    fn forward(&self, obs: Tensor<B, 2>) -> (Tensor<B, 2>, Tensor<B, 2>) {
        let x = relu(self.input.forward(obs));
        let x = self.block0.forward(x);
        let x = self.block1.forward(x);
        let x = self.block2.forward(x);
        let x = self.block3.forward(x);
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

    fn small_config() -> ResNetConfig {
        // Use a small hidden size so tests are fast.
        ResNetConfig {
            obs_size: 217,
            action_size: 514,
            hidden_size: 64,
        }
    }

    fn net() -> ResNet<B> {
        ResNet::new(&small_config(), &device())
    }

    // ── Shape tests ───────────────────────────────────────────────────────

    #[test]
    fn forward_output_shapes() {
        let obs = Tensor::zeros([4, 217], &device());
        let (policy, value) = net().forward(obs);
        assert_eq!(policy.dims(), [4, 514], "policy shape mismatch");
        assert_eq!(value.dims(), [4, 1], "value shape mismatch");
    }

    #[test]
    fn forward_single_sample() {
        let (policy, value) = net().forward(Tensor::zeros([1, 217], &device()));
        assert_eq!(policy.dims(), [1, 514]);
        assert_eq!(value.dims(), [1, 1]);
    }

    // ── Value bounds ──────────────────────────────────────────────────────

    #[test]
    fn value_in_tanh_range() {
        let obs = Tensor::<B, 2>::ones([8, 217], &device());
        let (_, value) = net().forward(obs);
        let data: Vec<f32> = value.into_data().to_vec().unwrap();
        for v in &data {
            assert!(
                *v > -1.0 && *v < 1.0,
                "value {v} is outside open interval (-1, 1)"
            );
        }
    }

    // ── Residual connections ──────────────────────────────────────────────

    #[test]
    fn policy_logits_not_all_equal() {
        let (policy, _) = net().forward(Tensor::zeros([1, 217], &device()));
        let data: Vec<f32> = policy.into_data().to_vec().unwrap();
        let first = data[0];
        let all_same = data.iter().all(|&x| (x - first).abs() < 1e-6);
        assert!(!all_same, "all policy logits are identical");
    }

    // ── Save / Load ───────────────────────────────────────────────────────

    #[test]
    fn save_load_preserves_weights() {
        let config = small_config();
        let model = net();
        let obs = Tensor::<B, 2>::ones([2, 217], &device());

        let (policy_before, value_before) = model.forward(obs.clone());

        let path = std::env::temp_dir().join("spiel_bot_test_resnet.mpk");
        model.save(&path).expect("save failed");

        let loaded = ResNet::<B>::load(&config, &path, &device()).expect("load failed");
        let (policy_after, value_after) = loaded.forward(obs);

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

    // ── Integration: both architectures satisfy PolicyValueNet ────────────

    #[test]
    fn resnet_satisfies_trait() {
        fn requires_net<B: Backend, N: PolicyValueNet<B>>(net: &N, obs: Tensor<B, 2>) {
            let (p, v) = net.forward(obs);
            assert_eq!(p.dims()[1], 514);
            assert_eq!(v.dims()[1], 1);
        }
        requires_net(&net(), Tensor::zeros([2, 217], &device()));
    }
}
