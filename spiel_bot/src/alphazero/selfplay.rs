//! Self-play episode generation and Burn-backed evaluator.

use std::marker::PhantomData;

use burn::tensor::{backend::Backend, Tensor, TensorData};
use rand::Rng;

use crate::env::GameEnv;
use crate::mcts::{self, Evaluator, MctsConfig, MctsNode};
use crate::network::PolicyValueNet;

use super::replay::TrainSample;

// ── BurnEvaluator ──────────────────────────────────────────────────────────

/// Wraps a [`PolicyValueNet`] as an [`Evaluator`] for MCTS.
///
/// Use the **inference backend** (`NdArray<f32>`, no `Autodiff` wrapper) so
/// that self-play generates no gradient tape overhead.
pub struct BurnEvaluator<B: Backend, N: PolicyValueNet<B>> {
    model: N,
    device: B::Device,
    _b: PhantomData<B>,
}

impl<B: Backend, N: PolicyValueNet<B>> BurnEvaluator<B, N> {
    pub fn new(model: N, device: B::Device) -> Self {
        Self { model, device, _b: PhantomData }
    }

    pub fn into_model(self) -> N {
        self.model
    }
}

// Safety: NdArray<f32> modules are Send; we never share across threads without
// external synchronisation.
unsafe impl<B: Backend, N: PolicyValueNet<B>> Send for BurnEvaluator<B, N> {}
unsafe impl<B: Backend, N: PolicyValueNet<B>> Sync for BurnEvaluator<B, N> {}

impl<B: Backend, N: PolicyValueNet<B>> Evaluator for BurnEvaluator<B, N> {
    fn evaluate(&self, obs: &[f32]) -> (Vec<f32>, f32) {
        let obs_size = obs.len();
        let data = TensorData::new(obs.to_vec(), [1, obs_size]);
        let obs_tensor = Tensor::<B, 2>::from_data(data, &self.device);

        let (policy_tensor, value_tensor) = self.model.forward(obs_tensor);

        let policy: Vec<f32> = policy_tensor.into_data().to_vec().unwrap();
        let value: Vec<f32> = value_tensor.into_data().to_vec().unwrap();

        (policy, value[0])
    }
}

// ── Episode generation ─────────────────────────────────────────────────────

/// One pending observation waiting for its game-outcome value label.
struct PendingSample {
    obs: Vec<f32>,
    policy: Vec<f32>,
    player: usize,
}

/// Play one full game using MCTS guided by `evaluator`.
///
/// Returns a [`TrainSample`] for every decision step in the game.
///
/// `temperature_fn(step)` controls exploration: return `1.0` for early
/// moves and `0.0` after a fixed number of moves (e.g. move 30).
pub fn generate_episode<E: GameEnv>(
    env: &E,
    evaluator: &dyn Evaluator,
    mcts_config: &MctsConfig,
    temperature_fn: &dyn Fn(usize) -> f32,
    rng: &mut impl Rng,
) -> Vec<TrainSample> {
    let mut state = env.new_game();
    let mut pending: Vec<PendingSample> = Vec::new();
    let mut step = 0usize;

    loop {
        // Advance through chance nodes automatically.
        while env.current_player(&state).is_chance() {
            env.apply_chance(&mut state, rng);
        }

        if env.current_player(&state).is_terminal() {
            break;
        }

        let player_idx = env.current_player(&state).index().unwrap();

        // Run MCTS to get a policy.
        let root: MctsNode = mcts::run_mcts(env, &state, evaluator, mcts_config, rng);
        let policy = mcts::mcts_policy(&root, env.action_space());

        // Record the observation from the acting player's perspective.
        let obs = env.observation(&state, player_idx);
        pending.push(PendingSample { obs, policy: policy.clone(), player: player_idx });

        // Select and apply the action.
        let temperature = temperature_fn(step);
        let action = mcts::select_action(&root, temperature, rng);
        env.apply(&mut state, action);
        step += 1;
    }

    // Assign game outcomes.
    let returns = env.returns(&state).unwrap_or([0.0; 2]);
    pending
        .into_iter()
        .map(|s| TrainSample {
            obs: s.obs,
            policy: s.policy,
            value: returns[s.player],
        })
        .collect()
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use burn::backend::NdArray;
    use rand::{SeedableRng, rngs::SmallRng};

    use crate::env::Player;
    use crate::mcts::{Evaluator, MctsConfig};
    use crate::network::{MlpConfig, MlpNet};

    type B = NdArray<f32>;

    fn device() -> <B as Backend>::Device {
        Default::default()
    }

    fn rng() -> SmallRng {
        SmallRng::seed_from_u64(7)
    }

    // Countdown game (same as in mcts tests).
    #[derive(Clone, Debug)]
    struct CState { remaining: u8, to_move: usize }

    #[derive(Clone)]
    struct CountdownEnv;

    impl GameEnv for CountdownEnv {
        type State = CState;
        fn new_game(&self) -> CState { CState { remaining: 4, to_move: 0 } }
        fn current_player(&self, s: &CState) -> Player {
            if s.remaining == 0 { Player::Terminal }
            else if s.to_move == 0 { Player::P1 } else { Player::P2 }
        }
        fn legal_actions(&self, s: &CState) -> Vec<usize> {
            if s.remaining >= 2 { vec![0, 1] } else { vec![0] }
        }
        fn apply(&self, s: &mut CState, action: usize) {
            let sub = (action as u8) + 1;
            if s.remaining <= sub { s.remaining = 0; }
            else { s.remaining -= sub; s.to_move = 1 - s.to_move; }
        }
        fn apply_chance<R: Rng>(&self, _s: &mut CState, _rng: &mut R) {}
        fn observation(&self, s: &CState, _pov: usize) -> Vec<f32> {
            vec![s.remaining as f32 / 4.0, s.to_move as f32]
        }
        fn obs_size(&self) -> usize { 2 }
        fn action_space(&self) -> usize { 2 }
        fn returns(&self, s: &CState) -> Option<[f32; 2]> {
            if s.remaining != 0 { return None; }
            let mut r = [-1.0f32; 2];
            r[s.to_move] = 1.0;
            Some(r)
        }
    }

    fn tiny_config() -> MctsConfig {
        MctsConfig { n_simulations: 5, c_puct: 1.5,
            dirichlet_alpha: 0.0, dirichlet_eps: 0.0, temperature: 1.0 }
    }

    // ── BurnEvaluator tests ───────────────────────────────────────────────

    #[test]
    fn burn_evaluator_output_shapes() {
        let config = MlpConfig { obs_size: 2, action_size: 2, hidden_size: 8 };
        let model = MlpNet::<B>::new(&config, &device());
        let eval = BurnEvaluator::new(model, device());
        let (policy, value) = eval.evaluate(&[0.5f32, 0.5]);
        assert_eq!(policy.len(), 2, "policy length should equal action_space");
        assert!(value > -1.0 && value < 1.0, "value {value} should be in (-1,1)");
    }

    // ── generate_episode tests ────────────────────────────────────────────

    #[test]
    fn episode_terminates_and_has_samples() {
        let env = CountdownEnv;
        let config = MlpConfig { obs_size: 2, action_size: 2, hidden_size: 8 };
        let model = MlpNet::<B>::new(&config, &device());
        let eval = BurnEvaluator::new(model, device());
        let samples = generate_episode(&env, &eval, &tiny_config(), &|_| 1.0, &mut rng());
        assert!(!samples.is_empty(), "episode must produce at least one sample");
    }

    #[test]
    fn episode_sample_values_are_valid() {
        let env = CountdownEnv;
        let config = MlpConfig { obs_size: 2, action_size: 2, hidden_size: 8 };
        let model = MlpNet::<B>::new(&config, &device());
        let eval = BurnEvaluator::new(model, device());
        let samples = generate_episode(&env, &eval, &tiny_config(), &|_| 1.0, &mut rng());
        for s in &samples {
            assert!(s.value == 1.0 || s.value == -1.0 || s.value == 0.0,
                "unexpected value {}", s.value);
            let sum: f32 = s.policy.iter().sum();
            assert!((sum - 1.0).abs() < 1e-4, "policy sums to {sum}");
            assert_eq!(s.obs.len(), 2);
        }
    }

    #[test]
    fn episode_with_temperature_zero() {
        let env = CountdownEnv;
        let config = MlpConfig { obs_size: 2, action_size: 2, hidden_size: 8 };
        let model = MlpNet::<B>::new(&config, &device());
        let eval = BurnEvaluator::new(model, device());
        // temperature=0 means greedy; episode must still terminate
        let samples = generate_episode(&env, &eval, &tiny_config(), &|_| 0.0, &mut rng());
        assert!(!samples.is_empty());
    }
}
