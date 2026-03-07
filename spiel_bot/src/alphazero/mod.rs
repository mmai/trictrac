//! AlphaZero: self-play data generation, replay buffer, and training step.
//!
//! # Modules
//!
//! | Module | Contents |
//! |--------|----------|
//! | [`replay`] | [`TrainSample`], [`ReplayBuffer`] |
//! | [`selfplay`] | [`BurnEvaluator`], [`generate_episode`] |
//! | [`trainer`] | [`train_step`] |
//!
//! # Typical outer loop
//!
//! ```rust,ignore
//! use burn::backend::{Autodiff, NdArray};
//! use burn::optim::AdamConfig;
//! use spiel_bot::{
//!     alphazero::{AlphaZeroConfig, BurnEvaluator, ReplayBuffer, generate_episode, train_step},
//!     env::TrictracEnv,
//!     mcts::MctsConfig,
//!     network::{MlpConfig, MlpNet},
//! };
//!
//! type Infer = NdArray<f32>;
//! type Train = Autodiff<NdArray<f32>>;
//!
//! let device = Default::default();
//! let env = TrictracEnv;
//! let config = AlphaZeroConfig::default();
//!
//! // Build training model and optimizer.
//! let mut train_model = MlpNet::<Train>::new(&MlpConfig::default(), &device);
//! let mut optimizer = AdamConfig::new().init();
//! let mut replay = ReplayBuffer::new(config.replay_capacity);
//! let mut rng = rand::rngs::SmallRng::seed_from_u64(0);
//!
//! for _iter in 0..config.n_iterations {
//!     // Convert to inference backend for self-play.
//!     let infer_model = MlpNet::<Infer>::new(&MlpConfig::default(), &device)
//!         .load_record(train_model.clone().into_record());
//!     let eval = BurnEvaluator::new(infer_model, device.clone());
//!
//!     // Self-play: generate episodes.
//!     for _ in 0..config.n_games_per_iter {
//!         let samples = generate_episode(&env, &eval, &config.mcts,
//!             &|step| if step < 30 { 1.0 } else { 0.0 }, &mut rng);
//!         replay.extend(samples);
//!     }
//!
//!     // Training: gradient steps.
//!     if replay.len() >= config.batch_size {
//!         for _ in 0..config.n_train_steps_per_iter {
//!             let batch: Vec<_> = replay.sample_batch(config.batch_size, &mut rng)
//!                 .into_iter().cloned().collect();
//!             let (m, _loss) = train_step(train_model, &mut optimizer, &batch, &device,
//!                 config.learning_rate);
//!             train_model = m;
//!         }
//!     }
//! }
//! ```

pub mod replay;
pub mod selfplay;
pub mod trainer;

pub use replay::{ReplayBuffer, TrainSample};
pub use selfplay::{BurnEvaluator, generate_episode};
pub use trainer::{cosine_lr, train_step};

use crate::mcts::MctsConfig;

// ── Configuration ─────────────────────────────────────────────────────────

/// Top-level AlphaZero hyperparameters.
///
/// The MCTS parameters live in [`MctsConfig`]; this struct holds the
/// outer training-loop parameters.
#[derive(Debug, Clone)]
pub struct AlphaZeroConfig {
    /// MCTS parameters for self-play.
    pub mcts: MctsConfig,
    /// Number of self-play games per training iteration.
    pub n_games_per_iter: usize,
    /// Number of gradient steps per training iteration.
    pub n_train_steps_per_iter: usize,
    /// Mini-batch size for each gradient step.
    pub batch_size: usize,
    /// Maximum number of samples in the replay buffer.
    pub replay_capacity: usize,
    /// Initial (peak) Adam learning rate.
    pub learning_rate: f64,
    /// Minimum learning rate for cosine annealing (floor of the schedule).
    ///
    /// Pass `learning_rate == lr_min` to disable scheduling (constant LR).
    /// Compute the current LR with [`cosine_lr`]:
    ///
    /// ```rust,ignore
    /// let lr = cosine_lr(config.learning_rate, config.lr_min, step, total_steps);
    /// ```
    pub lr_min: f64,
    /// Number of outer iterations (self-play + train) to run.
    pub n_iterations: usize,
    /// Move index after which the action temperature drops to 0 (greedy play).
    pub temperature_drop_move: usize,
}

impl Default for AlphaZeroConfig {
    fn default() -> Self {
        Self {
            mcts: MctsConfig {
                n_simulations: 100,
                c_puct: 1.5,
                dirichlet_alpha: 0.1,
                dirichlet_eps: 0.25,
                temperature: 1.0,
            },
            n_games_per_iter: 10,
            n_train_steps_per_iter: 20,
            batch_size: 64,
            replay_capacity: 50_000,
            learning_rate: 1e-3,
            lr_min: 1e-4,    // cosine annealing floor
            n_iterations: 100,
            temperature_drop_move: 30,
        }
    }
}
