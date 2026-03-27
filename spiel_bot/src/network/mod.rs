//! Neural network abstractions for policy-value learning.
//!
//! # Trait
//!
//! [`PolicyValueNet<B>`] is the single trait that all network architectures
//! implement.  It takes an observation tensor and returns raw policy logits
//! plus a tanh-squashed scalar value estimate.
//!
//! # Architectures
//!
//! | Module | Description | Default hidden |
//! |--------|-------------|----------------|
//! | [`MlpNet`] | 2-hidden-layer MLP — fast to train, good baseline | 256 |
//! | [`ResNet`] | 4-residual-block network — stronger long-term | 512 |
//!
//! # Backend convention
//!
//! * **Inference / self-play** — use `NdArray<f32>` (no autodiff overhead).
//! * **Training** — use `Autodiff<NdArray<f32>>` so Burn can differentiate
//!   through the forward pass.
//!
//! Both modes use the exact same struct; only the type-level backend changes:
//!
//! ```rust,ignore
//! use burn::backend::{Autodiff, NdArray};
//! type InferBackend = NdArray<f32>;
//! type TrainBackend = Autodiff<NdArray<f32>>;
//!
//! let infer_net = MlpNet::<InferBackend>::new(&MlpConfig::default(), &Default::default());
//! let train_net = MlpNet::<TrainBackend>::new(&MlpConfig::default(), &Default::default());
//! ```
//!
//! # Output shapes
//!
//! Given a batch of `B` observations of size `obs_size`:
//!
//! | Output | Shape | Range |
//! |--------|-------|-------|
//! | `policy_logits` | `[B, action_size]` | ℝ (unnormalised) |
//! | `value` | `[B, 1]` | (-1, 1) via tanh |
//!
//! Callers are responsible for masking illegal actions in `policy_logits`
//! before passing to softmax.

pub mod mlp;
pub mod qnet;
pub mod resnet;

pub use mlp::{MlpConfig, MlpNet};
pub use qnet::{QNet, QNetConfig};
pub use resnet::{ResNet, ResNetConfig};

use burn::{module::Module, tensor::backend::Backend, tensor::Tensor};

/// A neural network that produces a policy and a value from an observation.
///
/// # Shapes
/// - `obs`: `[batch, obs_size]`
/// - policy output: `[batch, action_size]`  — raw logits (no softmax applied)
/// - value output:  `[batch, 1]`            — tanh-squashed ∈ (-1, 1)
///
/// Note: `Sync` is intentionally absent — Burn's `Module` internally uses
/// `OnceCell` for lazy parameter initialisation, which is not `Sync`.
/// Use an `Arc<Mutex<N>>` wrapper if cross-thread sharing is needed.
pub trait PolicyValueNet<B: Backend>: Module<B> + Send + 'static {
    fn forward(&self, obs: Tensor<B, 2>) -> (Tensor<B, 2>, Tensor<B, 2>);
}

/// A neural network that outputs one Q-value per action.
///
/// # Shapes
/// - `obs`: `[batch, obs_size]`
/// - output: `[batch, action_size]` — raw Q-values (no activation)
///
/// Note: `Sync` is intentionally absent for the same reason as [`PolicyValueNet`].
pub trait QValueNet<B: Backend>: Module<B> + Send + 'static {
    fn forward(&self, obs: Tensor<B, 2>) -> Tensor<B, 2>;
}
