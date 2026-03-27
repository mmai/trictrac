//! Single-headed Q-value network for DQN.
//!
//! ```text
//! Input [B, obs_size]
//!   → Linear(obs → hidden) → ReLU
//!   → Linear(hidden → hidden) → ReLU
//!   → Linear(hidden → action_size)   ← raw Q-values, no activation
//! ```

use burn::{
    module::Module,
    nn::{Linear, LinearConfig},
    record::{CompactRecorder, Recorder},
    tensor::{activation::relu, backend::Backend, Tensor},
};
use std::path::Path;

use super::QValueNet;

// ── Config ────────────────────────────────────────────────────────────────────

/// Configuration for [`QNet`].
#[derive(Debug, Clone)]
pub struct QNetConfig {
    /// Number of input features.  217 for Trictrac's `to_tensor()`.
    pub obs_size: usize,
    /// Number of output actions.  514 for Trictrac's `ACTION_SPACE_SIZE`.
    pub action_size: usize,
    /// Width of both hidden layers.
    pub hidden_size: usize,
}

impl Default for QNetConfig {
    fn default() -> Self {
        Self { obs_size: 217, action_size: 514, hidden_size: 256 }
    }
}

// ── Network ───────────────────────────────────────────────────────────────────

/// Two-hidden-layer MLP that outputs one Q-value per action.
#[derive(Module, Debug)]
pub struct QNet<B: Backend> {
    fc1: Linear<B>,
    fc2: Linear<B>,
    q_head: Linear<B>,
}

impl<B: Backend> QNet<B> {
    /// Construct a fresh network with random weights.
    pub fn new(config: &QNetConfig, device: &B::Device) -> Self {
        Self {
            fc1: LinearConfig::new(config.obs_size, config.hidden_size).init(device),
            fc2: LinearConfig::new(config.hidden_size, config.hidden_size).init(device),
            q_head: LinearConfig::new(config.hidden_size, config.action_size).init(device),
        }
    }

    /// Save weights to `path` (MessagePack format via [`CompactRecorder`]).
    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        CompactRecorder::new()
            .record(self.clone().into_record(), path.to_path_buf())
            .map_err(|e| anyhow::anyhow!("QNet::save failed: {e:?}"))
    }

    /// Load weights from `path` into a fresh model built from `config`.
    pub fn load(config: &QNetConfig, path: &Path, device: &B::Device) -> anyhow::Result<Self> {
        let record = CompactRecorder::new()
            .load(path.to_path_buf(), device)
            .map_err(|e| anyhow::anyhow!("QNet::load failed: {e:?}"))?;
        Ok(Self::new(config, device).load_record(record))
    }
}

impl<B: Backend> QValueNet<B> for QNet<B> {
    fn forward(&self, obs: Tensor<B, 2>) -> Tensor<B, 2> {
        let x = relu(self.fc1.forward(obs));
        let x = relu(self.fc2.forward(x));
        self.q_head.forward(x)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use burn::backend::NdArray;

    type B = NdArray<f32>;

    fn device() -> <B as Backend>::Device { Default::default() }

    fn default_net() -> QNet<B> {
        QNet::new(&QNetConfig::default(), &device())
    }

    #[test]
    fn forward_output_shape() {
        let net = default_net();
        let obs = Tensor::zeros([4, 217], &device());
        let q = net.forward(obs);
        assert_eq!(q.dims(), [4, 514]);
    }

    #[test]
    fn forward_single_sample() {
        let net = default_net();
        let q = net.forward(Tensor::zeros([1, 217], &device()));
        assert_eq!(q.dims(), [1, 514]);
    }

    #[test]
    fn q_values_not_all_equal() {
        let net = default_net();
        let q: Vec<f32> = net.forward(Tensor::zeros([1, 217], &device()))
            .into_data().to_vec().unwrap();
        let first = q[0];
        assert!(!q.iter().all(|&x| (x - first).abs() < 1e-6));
    }

    #[test]
    fn custom_config_shapes() {
        let cfg = QNetConfig { obs_size: 10, action_size: 20, hidden_size: 32 };
        let net = QNet::<B>::new(&cfg, &device());
        let q = net.forward(Tensor::zeros([3, 10], &device()));
        assert_eq!(q.dims(), [3, 20]);
    }

    #[test]
    fn save_load_preserves_weights() {
        let net = default_net();
        let obs = Tensor::<B, 2>::ones([2, 217], &device());
        let q_before: Vec<f32> = net.forward(obs.clone()).into_data().to_vec().unwrap();

        let path = std::env::temp_dir().join("spiel_bot_test_qnet.mpk");
        net.save(&path).expect("save failed");

        let loaded = QNet::<B>::load(&QNetConfig::default(), &path, &device()).expect("load failed");
        let q_after: Vec<f32> = loaded.forward(obs).into_data().to_vec().unwrap();

        for (i, (a, b)) in q_before.iter().zip(q_after.iter()).enumerate() {
            assert!((a - b).abs() < 1e-3, "q[{i}]: {a} vs {b}");
        }
        let _ = std::fs::remove_file(path);
    }
}
