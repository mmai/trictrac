//! DQN: self-play data generation, replay buffer, and training step.
//!
//! # Algorithm
//!
//! Deep Q-Network with:
//! - **ε-greedy** exploration (linearly decayed).
//! - **Dense per-turn rewards**: `my_score_delta − opponent_score_delta` where
//!   `score = holes × 12 + points`.
//! - **Experience replay** with a fixed-capacity circular buffer.
//! - **Target network**: hard-copied from the online Q-net every
//!   `target_update_freq` gradient steps for training stability.
//!
//! # Modules
//!
//! | Module | Contents |
//! |--------|----------|
//! | [`episode`] | [`DqnSample`], [`generate_dqn_episode`] |
//! | [`trainer`] | [`dqn_train_step`], [`compute_target_q`], [`hard_update`] |

pub mod episode;
pub mod trainer;

pub use episode::generate_dqn_episode;
pub use trainer::{compute_target_q, dqn_train_step, hard_update};

use std::collections::VecDeque;
use rand::Rng;

// ── DqnSample ─────────────────────────────────────────────────────────────────

/// One transition `(s, a, r, s', done)` collected during self-play.
#[derive(Clone, Debug)]
pub struct DqnSample {
    /// Observation from the acting player's perspective (`obs_size` floats).
    pub obs: Vec<f32>,
    /// Action index taken.
    pub action: usize,
    /// Per-turn reward: `my_score_delta − opponent_score_delta`.
    pub reward: f32,
    /// Next observation from the same player's perspective.
    /// All-zeros when `done = true` (ignored by the TD target).
    pub next_obs: Vec<f32>,
    /// Legal actions at `next_obs`.  Empty when `done = true`.
    pub next_legal: Vec<usize>,
    /// `true` when `next_obs` is a terminal state.
    pub done: bool,
}

// ── DqnReplayBuffer ───────────────────────────────────────────────────────────

/// Fixed-capacity circular replay buffer for [`DqnSample`]s.
///
/// When full, the oldest sample is evicted on push.
/// Batches are drawn without replacement via a partial Fisher-Yates shuffle.
pub struct DqnReplayBuffer {
    data: VecDeque<DqnSample>,
    capacity: usize,
}

impl DqnReplayBuffer {
    pub fn new(capacity: usize) -> Self {
        Self { data: VecDeque::with_capacity(capacity.min(1024)), capacity }
    }

    pub fn push(&mut self, sample: DqnSample) {
        if self.data.len() == self.capacity {
            self.data.pop_front();
        }
        self.data.push_back(sample);
    }

    pub fn extend(&mut self, samples: impl IntoIterator<Item = DqnSample>) {
        for s in samples { self.push(s); }
    }

    pub fn len(&self) -> usize { self.data.len() }
    pub fn is_empty(&self) -> bool { self.data.is_empty() }

    /// Sample up to `n` distinct samples without replacement.
    pub fn sample_batch(&self, n: usize, rng: &mut impl Rng) -> Vec<&DqnSample> {
        let len = self.data.len();
        let n = n.min(len);
        let mut indices: Vec<usize> = (0..len).collect();
        for i in 0..n {
            let j = rng.random_range(i..len);
            indices.swap(i, j);
        }
        indices[..n].iter().map(|&i| &self.data[i]).collect()
    }
}

// ── DqnConfig ─────────────────────────────────────────────────────────────────

/// Top-level DQN hyperparameters for the training loop.
#[derive(Debug, Clone)]
pub struct DqnConfig {
    /// Initial exploration rate (1.0 = fully random).
    pub epsilon_start: f32,
    /// Final exploration rate after decay.
    pub epsilon_end: f32,
    /// Number of gradient steps over which ε decays linearly from start to end.
    ///
    /// Should be calibrated to the total number of gradient steps
    /// (`n_iterations × n_train_steps_per_iter`).  A value larger than that
    /// means exploration never reaches `epsilon_end` during the run.
    pub epsilon_decay_steps: usize,
    /// Discount factor γ for the TD target.  Typical: 0.99.
    pub gamma: f32,
    /// Hard-copy Q → target every this many gradient steps.
    ///
    /// Should be much smaller than the total number of gradient steps
    /// (`n_iterations × n_train_steps_per_iter`).
    pub target_update_freq: usize,
    /// Adam learning rate.
    pub learning_rate: f64,
    /// Mini-batch size for each gradient step.
    pub batch_size: usize,
    /// Maximum number of samples in the replay buffer.
    pub replay_capacity: usize,
    /// Number of outer iterations (self-play + train).
    pub n_iterations: usize,
    /// Self-play games per iteration.
    pub n_games_per_iter: usize,
    /// Gradient steps per iteration.
    pub n_train_steps_per_iter: usize,
    /// Reward normalisation divisor.
    ///
    /// Per-turn rewards (score delta) are divided by this constant before being
    /// stored.  Without normalisation, rewards can reach ±24 (jan with
    /// bredouille = 12 pts × 2), driving Q-values into the hundreds and
    /// causing MSE loss to grow unboundedly.
    ///
    /// A value of `12.0` maps one hole (12 points) to `±1.0`, keeping
    /// Q-value magnitudes in a stable range.  Set to `1.0` to disable.
    pub reward_scale: f32,
}

impl Default for DqnConfig {
    fn default() -> Self {
        // Total gradient steps with these defaults = 500 × 20 = 10_000,
        // so epsilon decays fully and the target is updated 100 times.
        Self {
            epsilon_start: 1.0,
            epsilon_end: 0.05,
            epsilon_decay_steps: 10_000,
            gamma: 0.99,
            target_update_freq: 100,
            learning_rate: 1e-3,
            batch_size: 64,
            replay_capacity: 50_000,
            n_iterations: 500,
            n_games_per_iter: 10,
            n_train_steps_per_iter: 20,
            reward_scale: 12.0,
        }
    }
}

/// Linear ε schedule: decays from `start` to `end` over `decay_steps` steps.
pub fn linear_epsilon(start: f32, end: f32, step: usize, decay_steps: usize) -> f32 {
    if decay_steps == 0 || step >= decay_steps {
        return end;
    }
    start + (end - start) * (step as f32 / decay_steps as f32)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{SeedableRng, rngs::SmallRng};

    fn dummy(reward: f32) -> DqnSample {
        DqnSample {
            obs: vec![0.0],
            action: 0,
            reward,
            next_obs: vec![0.0],
            next_legal: vec![0],
            done: false,
        }
    }

    #[test]
    fn push_and_len() {
        let mut buf = DqnReplayBuffer::new(10);
        assert!(buf.is_empty());
        buf.push(dummy(1.0));
        buf.push(dummy(2.0));
        assert_eq!(buf.len(), 2);
    }

    #[test]
    fn evicts_oldest_at_capacity() {
        let mut buf = DqnReplayBuffer::new(3);
        buf.push(dummy(1.0));
        buf.push(dummy(2.0));
        buf.push(dummy(3.0));
        buf.push(dummy(4.0));
        assert_eq!(buf.len(), 3);
        assert_eq!(buf.data[0].reward, 2.0);
    }

    #[test]
    fn sample_batch_size() {
        let mut buf = DqnReplayBuffer::new(20);
        for i in 0..10 { buf.push(dummy(i as f32)); }
        let mut rng = SmallRng::seed_from_u64(0);
        assert_eq!(buf.sample_batch(5, &mut rng).len(), 5);
    }

    #[test]
    fn linear_epsilon_start() {
        assert!((linear_epsilon(1.0, 0.05, 0, 100) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn linear_epsilon_end() {
        assert!((linear_epsilon(1.0, 0.05, 100, 100) - 0.05).abs() < 1e-6);
    }

    #[test]
    fn linear_epsilon_monotone() {
        let mut prev = f32::INFINITY;
        for step in 0..=100 {
            let e = linear_epsilon(1.0, 0.05, step, 100);
            assert!(e <= prev + 1e-6);
            prev = e;
        }
    }
}
