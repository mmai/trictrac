//! End-to-end integration tests for the AlphaZero training pipeline.
//!
//! Each test exercises the full chain:
//!   [`GameEnv`] → MCTS → [`generate_episode`] → [`ReplayBuffer`] → [`train_step`]
//!
//! Two environments are used:
//! - **CountdownEnv** — trivial deterministic game, terminates in < 10 moves.
//!   Used when we need many iterations without worrying about runtime.
//! - **TrictracEnv** — the real game.  Used to verify tensor shapes and that
//!   the full pipeline compiles and runs correctly with 217-dim observations
//!   and 514-dim action spaces.
//!
//! All tests use `n_simulations = 2` and `hidden_size = 64` to keep
//! runtime minimal; correctness, not training quality, is what matters here.

use burn::{
    backend::{Autodiff, NdArray},
    module::AutodiffModule,
    optim::AdamConfig,
};
use rand::{SeedableRng, rngs::SmallRng};

use spiel_bot::{
    alphazero::{BurnEvaluator, ReplayBuffer, TrainSample, generate_episode, train_step},
    env::{GameEnv, Player, TrictracEnv},
    mcts::MctsConfig,
    network::{MlpConfig, MlpNet, PolicyValueNet},
};

// ── Backend aliases ────────────────────────────────────────────────────────

type Train = Autodiff<NdArray<f32>>;
type Infer = NdArray<f32>;

// ── Helpers ────────────────────────────────────────────────────────────────

fn train_device() -> <Train as burn::tensor::backend::Backend>::Device {
    Default::default()
}

fn infer_device() -> <Infer as burn::tensor::backend::Backend>::Device {
    Default::default()
}

/// Tiny 64-unit MLP, compatible with an obs/action space of any size.
fn tiny_mlp(obs: usize, actions: usize) -> MlpNet<Train> {
    let cfg = MlpConfig { obs_size: obs, action_size: actions, hidden_size: 64 };
    MlpNet::new(&cfg, &train_device())
}

fn tiny_mcts(n: usize) -> MctsConfig {
    MctsConfig {
        n_simulations: n,
        c_puct: 1.5,
        dirichlet_alpha: 0.0,
        dirichlet_eps: 0.0,
        temperature: 1.0,
    }
}

fn seeded() -> SmallRng {
    SmallRng::seed_from_u64(0)
}

// ── Countdown environment (fast, local, no external deps) ─────────────────
//
// Two players alternate subtracting 1 or 2 from a counter that starts at N.
// The player who brings the counter to 0 wins.

#[derive(Clone, Debug)]
struct CState {
    remaining: u8,
    to_move: usize,
}

#[derive(Clone)]
struct CountdownEnv(u8); // starting value

impl GameEnv for CountdownEnv {
    type State = CState;

    fn new_game(&self) -> CState {
        CState { remaining: self.0, to_move: 0 }
    }

    fn current_player(&self, s: &CState) -> Player {
        if s.remaining == 0 { Player::Terminal }
        else if s.to_move == 0 { Player::P1 }
        else { Player::P2 }
    }

    fn legal_actions(&self, s: &CState) -> Vec<usize> {
        if s.remaining >= 2 { vec![0, 1] } else { vec![0] }
    }

    fn apply(&self, s: &mut CState, action: usize) {
        let sub = (action as u8) + 1;
        if s.remaining <= sub {
            s.remaining = 0;
        } else {
            s.remaining -= sub;
            s.to_move = 1 - s.to_move;
        }
    }

    fn apply_chance<R: rand::Rng>(&self, _s: &mut CState, _rng: &mut R) {}

    fn observation(&self, s: &CState, _pov: usize) -> Vec<f32> {
        vec![s.remaining as f32 / self.0 as f32, s.to_move as f32]
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

// ── 1. Full loop on CountdownEnv ──────────────────────────────────────────

/// The canonical AlphaZero loop: self-play → replay → train, iterated.
/// Uses CountdownEnv so each game terminates in < 10 moves.
#[test]
fn countdown_full_loop_no_panic() {
    let env = CountdownEnv(8);
    let mut rng = seeded();
    let mcts = tiny_mcts(3);

    let mut model = tiny_mlp(env.obs_size(), env.action_space());
    let mut optimizer = AdamConfig::new().init();
    let mut replay = ReplayBuffer::new(1_000);

    for _iter in 0..5 {
        // Self-play: 3 games per iteration.
        for _ in 0..3 {
            let infer = model.valid();
            let eval = BurnEvaluator::<Infer, _>::new(infer, infer_device());
            let samples = generate_episode(&env, &eval, &mcts, &|_| 1.0, &mut rng);
            assert!(!samples.is_empty());
            replay.extend(samples);
        }

        // Training: 4 gradient steps per iteration.
        if replay.len() >= 4 {
            for _ in 0..4 {
                let batch: Vec<TrainSample> = replay
                    .sample_batch(4, &mut rng)
                    .into_iter()
                    .cloned()
                    .collect();
                let (m, loss) = train_step(model, &mut optimizer, &batch, &train_device(), 1e-3);
                model = m;
                assert!(loss.is_finite(), "loss must be finite, got {loss}");
            }
        }
    }

    assert!(replay.len() > 0);
}

// ── 2. Replay buffer invariants ───────────────────────────────────────────

/// After several Countdown games, replay capacity is respected and batch
/// shapes are consistent.
#[test]
fn replay_buffer_capacity_and_shapes() {
    let env = CountdownEnv(6);
    let mut rng = seeded();
    let mcts = tiny_mcts(2);
    let model = tiny_mlp(env.obs_size(), env.action_space());

    let capacity = 50;
    let mut replay = ReplayBuffer::new(capacity);

    for _ in 0..20 {
        let infer = model.valid();
        let eval = BurnEvaluator::<Infer, _>::new(infer, infer_device());
        let samples = generate_episode(&env, &eval, &mcts, &|_| 1.0, &mut rng);
        replay.extend(samples);
    }

    assert!(replay.len() <= capacity, "buffer exceeded capacity");
    assert!(replay.len() > 0);

    let batch = replay.sample_batch(8, &mut rng);
    assert_eq!(batch.len(), 8.min(replay.len()));
    for s in &batch {
        assert_eq!(s.obs.len(), env.obs_size());
        assert_eq!(s.policy.len(), env.action_space());
        let policy_sum: f32 = s.policy.iter().sum();
        assert!((policy_sum - 1.0).abs() < 1e-4, "policy sums to {policy_sum}");
        assert!(s.value.abs() <= 1.0, "value {} out of range", s.value);
    }
}

// ── 3. TrictracEnv: sample shapes ─────────────────────────────────────────

/// Verify that one TrictracEnv episode produces samples with the correct
/// tensor dimensions: obs = 217, policy = 514.
#[test]
fn trictrac_sample_shapes() {
    let env = TrictracEnv;
    let mut rng = seeded();
    let mcts = tiny_mcts(2);
    let model = tiny_mlp(env.obs_size(), env.action_space());

    let infer = model.valid();
    let eval = BurnEvaluator::<Infer, _>::new(infer, infer_device());
    let samples = generate_episode(&env, &eval, &mcts, &|_| 1.0, &mut rng);

    assert!(!samples.is_empty(), "Trictrac episode produced no samples");

    for (i, s) in samples.iter().enumerate() {
        assert_eq!(s.obs.len(), 217, "sample {i}: obs.len() = {}", s.obs.len());
        assert_eq!(s.policy.len(), 514, "sample {i}: policy.len() = {}", s.policy.len());
        let policy_sum: f32 = s.policy.iter().sum();
        assert!(
            (policy_sum - 1.0).abs() < 1e-4,
            "sample {i}: policy sums to {policy_sum}"
        );
        assert!(
            s.value == 1.0 || s.value == -1.0 || s.value == 0.0,
            "sample {i}: unexpected value {}",
            s.value
        );
    }
}

// ── 4. TrictracEnv: training step after real self-play ────────────────────

/// Collect one Trictrac episode, then verify that a gradient step runs
/// without panic and produces a finite loss.
#[test]
fn trictrac_train_step_finite_loss() {
    let env = TrictracEnv;
    let mut rng = seeded();
    let mcts = tiny_mcts(2);
    let model = tiny_mlp(env.obs_size(), env.action_space());
    let mut optimizer = AdamConfig::new().init();
    let mut replay = ReplayBuffer::new(10_000);

    // Generate one episode.
    let infer = model.valid();
    let eval = BurnEvaluator::<Infer, _>::new(infer, infer_device());
    let samples = generate_episode(&env, &eval, &mcts, &|_| 1.0, &mut rng);
    assert!(!samples.is_empty());
    let n_samples = samples.len();
    replay.extend(samples);

    // Train on a batch from this episode.
    let batch_size = 8.min(n_samples);
    let batch: Vec<TrainSample> = replay
        .sample_batch(batch_size, &mut rng)
        .into_iter()
        .cloned()
        .collect();

    let (_, loss) = train_step(model, &mut optimizer, &batch, &train_device(), 1e-3);
    assert!(loss.is_finite(), "loss must be finite after Trictrac training, got {loss}");
    assert!(loss > 0.0, "loss should be positive");
}

// ── 5. Backend transfer: train → infer → same outputs ─────────────────────

/// Weights transferred from the training backend to the inference backend
/// (via `AutodiffModule::valid()`) must produce bit-identical forward passes.
#[test]
fn valid_model_matches_train_model_outputs() {
    use burn::tensor::{Tensor, TensorData};

    let cfg = MlpConfig { obs_size: 4, action_size: 4, hidden_size: 32 };
    let train_model = MlpNet::<Train>::new(&cfg, &train_device());
    let infer_model: MlpNet<Infer> = train_model.valid();

    // Build the same input on both backends.
    let obs_data: Vec<f32> = vec![0.1, 0.2, 0.3, 0.4];

    let obs_train = Tensor::<Train, 2>::from_data(
        TensorData::new(obs_data.clone(), [1, 4]),
        &train_device(),
    );
    let obs_infer = Tensor::<Infer, 2>::from_data(
        TensorData::new(obs_data, [1, 4]),
        &infer_device(),
    );

    let (p_train, v_train) = train_model.forward(obs_train);
    let (p_infer, v_infer) = infer_model.forward(obs_infer);

    let p_train: Vec<f32> = p_train.into_data().to_vec().unwrap();
    let p_infer: Vec<f32> = p_infer.into_data().to_vec().unwrap();
    let v_train: Vec<f32> = v_train.into_data().to_vec().unwrap();
    let v_infer: Vec<f32> = v_infer.into_data().to_vec().unwrap();

    for (i, (a, b)) in p_train.iter().zip(p_infer.iter()).enumerate() {
        assert!(
            (a - b).abs() < 1e-5,
            "policy[{i}] differs after valid(): train={a}, infer={b}"
        );
    }
    assert!(
        (v_train[0] - v_infer[0]).abs() < 1e-5,
        "value differs after valid(): train={}, infer={}",
        v_train[0], v_infer[0]
    );
}

// ── 6. Loss converges on a fixed batch ────────────────────────────────────

/// With repeated gradient steps on the same Countdown batch, the loss must
/// decrease monotonically (or at least end lower than it started).
#[test]
fn loss_decreases_on_fixed_batch() {
    let env = CountdownEnv(6);
    let mut rng = seeded();
    let mcts = tiny_mcts(3);
    let model = tiny_mlp(env.obs_size(), env.action_space());
    let mut optimizer = AdamConfig::new().init();

    // Collect a fixed batch from one episode.
    let infer = model.valid();
    let eval = BurnEvaluator::<Infer, _>::new(infer, infer_device());
    let samples: Vec<TrainSample> = generate_episode(&env, &eval, &mcts, &|_| 0.0, &mut rng);
    assert!(!samples.is_empty());

    let batch: Vec<TrainSample> = {
        let mut replay = ReplayBuffer::new(1000);
        replay.extend(samples);
        replay.sample_batch(replay.len(), &mut rng).into_iter().cloned().collect()
    };

    // Overfit on the same fixed batch for 20 steps.
    let mut model = tiny_mlp(env.obs_size(), env.action_space());
    let mut first_loss = f32::NAN;
    let mut last_loss = f32::NAN;

    for step in 0..20 {
        let (m, loss) = train_step(model, &mut optimizer, &batch, &train_device(), 1e-2);
        model = m;
        assert!(loss.is_finite(), "loss is not finite at step {step}");
        if step == 0 { first_loss = loss; }
        last_loss = loss;
    }

    assert!(
        last_loss < first_loss,
        "loss did not decrease after 20 steps: first={first_loss}, last={last_loss}"
    );
}

// ── 7. Trictrac: multi-iteration loop ─────────────────────────────────────

/// Two full self-play + train iterations on TrictracEnv.
/// Verifies the entire pipeline runs without panic end-to-end.
#[test]
fn trictrac_two_iteration_loop() {
    let env = TrictracEnv;
    let mut rng = seeded();
    let mcts = tiny_mcts(2);

    let cfg = MlpConfig { obs_size: 217, action_size: 514, hidden_size: 64 };
    let mut model = MlpNet::<Train>::new(&cfg, &train_device());
    let mut optimizer = AdamConfig::new().init();
    let mut replay = ReplayBuffer::new(20_000);

    for iter in 0..2 {
        // Self-play: 1 game per iteration.
        let infer: MlpNet<Infer> = model.valid();
        let eval = BurnEvaluator::<Infer, _>::new(infer, infer_device());
        let samples = generate_episode(&env, &eval, &mcts, &|step| if step < 30 { 1.0 } else { 0.0 }, &mut rng);
        assert!(!samples.is_empty(), "iter {iter}: episode was empty");
        replay.extend(samples);

        // Training: 3 gradient steps.
        let batch_size = 16.min(replay.len());
        for _ in 0..3 {
            let batch: Vec<TrainSample> = replay
                .sample_batch(batch_size, &mut rng)
                .into_iter()
                .cloned()
                .collect();
            let (m, loss) = train_step(model, &mut optimizer, &batch, &train_device(), 1e-3);
            model = m;
            assert!(loss.is_finite(), "iter {iter}: loss={loss}");
        }
    }
}
