//! AlphaZero pipeline benchmarks.
//!
//! Run with:
//!
//! ```sh
//! cargo bench -p spiel_bot
//! ```
//!
//! Use `-- <filter>` to run a specific group, e.g.:
//!
//! ```sh
//! cargo bench -p spiel_bot -- env/
//! cargo bench -p spiel_bot -- network/
//! cargo bench -p spiel_bot -- mcts/
//! cargo bench -p spiel_bot -- episode/
//! cargo bench -p spiel_bot -- train/
//! ```
//!
//! Target: ≥ 500 games/s for random play on CPU (consistent with
//! `random_game` throughput in `trictrac-store`).

use std::time::Duration;

use burn::{
    backend::NdArray,
    tensor::{Tensor, TensorData, backend::Backend},
};
use criterion::{BatchSize, BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use rand::{Rng, SeedableRng, rngs::SmallRng};

use spiel_bot::{
    alphazero::{BurnEvaluator, TrainSample, generate_episode, train_step},
    env::{GameEnv, Player, TrictracEnv},
    mcts::{Evaluator, MctsConfig, run_mcts},
    network::{MlpConfig, MlpNet, PolicyValueNet},
};

// ── Shared types ───────────────────────────────────────────────────────────

type InferB = NdArray<f32>;
type TrainB = burn::backend::Autodiff<NdArray<f32>>;

fn infer_device() -> <InferB as Backend>::Device { Default::default() }
fn train_device() -> <TrainB as Backend>::Device { Default::default() }

fn seeded() -> SmallRng { SmallRng::seed_from_u64(0) }

/// Uniform evaluator (returns zero logits and zero value).
/// Used to isolate MCTS tree-traversal cost from network cost.
struct ZeroEval(usize);
impl Evaluator for ZeroEval {
    fn evaluate(&self, _obs: &[f32]) -> (Vec<f32>, f32) {
        (vec![0.0f32; self.0], 0.0)
    }
}

// ── 1. Environment primitives ──────────────────────────────────────────────

/// Baseline performance of the raw Trictrac environment without MCTS.
/// Target: ≥ 500 full games / second.
fn bench_env(c: &mut Criterion) {
    let env = TrictracEnv;

    let mut group = c.benchmark_group("env");
    group.measurement_time(Duration::from_secs(10));

    // ── apply_chance ──────────────────────────────────────────────────────
    group.bench_function("apply_chance", |b| {
        b.iter_batched(
            || {
                // A fresh game is always at RollDice (Chance) — ready for apply_chance.
                env.new_game()
            },
            |mut s| {
                env.apply_chance(&mut s, &mut seeded());
                black_box(s)
            },
            BatchSize::SmallInput,
        )
    });

    // ── legal_actions ─────────────────────────────────────────────────────
    group.bench_function("legal_actions", |b| {
        let mut rng = seeded();
        let mut s = env.new_game();
        env.apply_chance(&mut s, &mut rng);
        b.iter(|| black_box(env.legal_actions(&s)))
    });

    // ── observation (to_tensor) ───────────────────────────────────────────
    group.bench_function("observation", |b| {
        let mut rng = seeded();
        let mut s = env.new_game();
        env.apply_chance(&mut s, &mut rng);
        b.iter(|| black_box(env.observation(&s, 0)))
    });

    // ── full random game ──────────────────────────────────────────────────
    group.sample_size(50);
    group.bench_function("random_game", |b| {
        b.iter_batched(
            seeded,
            |mut rng| {
                let mut s = env.new_game();
                loop {
                    match env.current_player(&s) {
                        Player::Terminal => break,
                        Player::Chance => env.apply_chance(&mut s, &mut rng),
                        _ => {
                            let actions = env.legal_actions(&s);
                            let idx = rng.random_range(0..actions.len());
                            env.apply(&mut s, actions[idx]);
                        }
                    }
                }
                black_box(s)
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

// ── 2. Network inference ───────────────────────────────────────────────────

/// Forward-pass latency for MLP variants (hidden = 64 / 256).
fn bench_network(c: &mut Criterion) {
    let mut group = c.benchmark_group("network");
    group.measurement_time(Duration::from_secs(5));

    for &hidden in &[64usize, 256] {
        let cfg = MlpConfig { obs_size: 217, action_size: 514, hidden_size: hidden };
        let model = MlpNet::<InferB>::new(&cfg, &infer_device());
        let obs: Vec<f32> = vec![0.5; 217];

        // Batch size 1 — single-position evaluation as in MCTS.
        group.bench_with_input(
            BenchmarkId::new("mlp_b1", hidden),
            &hidden,
            |b, _| {
                b.iter(|| {
                    let data = TensorData::new(obs.clone(), [1, 217]);
                    let t = Tensor::<InferB, 2>::from_data(data, &infer_device());
                    black_box(model.forward(t))
                })
            },
        );

        // Batch size 32 — training mini-batch.
        let obs32: Vec<f32> = vec![0.5; 217 * 32];
        group.bench_with_input(
            BenchmarkId::new("mlp_b32", hidden),
            &hidden,
            |b, _| {
                b.iter(|| {
                    let data = TensorData::new(obs32.clone(), [32, 217]);
                    let t = Tensor::<InferB, 2>::from_data(data, &infer_device());
                    black_box(model.forward(t))
                })
            },
        );
    }

    group.finish();
}

// ── 3. MCTS ───────────────────────────────────────────────────────────────

/// MCTS cost at different simulation budgets with two evaluator types:
/// - `zero` — isolates tree-traversal overhead (no network).
/// - `mlp64` — real MLP, shows end-to-end cost per move.
fn bench_mcts(c: &mut Criterion) {
    let env = TrictracEnv;

    // Build a decision-node state (after dice roll).
    let state = {
        let mut s = env.new_game();
        let mut rng = seeded();
        while env.current_player(&s).is_chance() {
            env.apply_chance(&mut s, &mut rng);
        }
        s
    };

    let mut group = c.benchmark_group("mcts");
    group.measurement_time(Duration::from_secs(10));

    let zero_eval = ZeroEval(514);
    let mlp_cfg = MlpConfig { obs_size: 217, action_size: 514, hidden_size: 64 };
    let mlp_model = MlpNet::<InferB>::new(&mlp_cfg, &infer_device());
    let mlp_eval = BurnEvaluator::<InferB, _>::new(mlp_model, infer_device());

    for &n_sim in &[1usize, 5, 20] {
        let cfg = MctsConfig {
            n_simulations: n_sim,
            c_puct: 1.5,
            dirichlet_alpha: 0.0,
            dirichlet_eps: 0.0,
            temperature: 1.0,
        };

        // Zero evaluator: tree traversal only.
        group.bench_with_input(
            BenchmarkId::new("zero_eval", n_sim),
            &n_sim,
            |b, _| {
                b.iter_batched(
                    seeded,
                    |mut rng| black_box(run_mcts(&env, &state, &zero_eval, &cfg, &mut rng)),
                    BatchSize::SmallInput,
                )
            },
        );

        // MLP evaluator: full cost per decision.
        group.bench_with_input(
            BenchmarkId::new("mlp64", n_sim),
            &n_sim,
            |b, _| {
                b.iter_batched(
                    seeded,
                    |mut rng| black_box(run_mcts(&env, &state, &mlp_eval, &cfg, &mut rng)),
                    BatchSize::SmallInput,
                )
            },
        );
    }

    group.finish();
}

// ── 4. Episode generation ─────────────────────────────────────────────────

/// Full self-play episode latency (one complete game) at different MCTS
/// simulation budgets.  Target: ≥ 1 game/s at n_sim=20 on CPU.
fn bench_episode(c: &mut Criterion) {
    let env = TrictracEnv;
    let mlp_cfg = MlpConfig { obs_size: 217, action_size: 514, hidden_size: 64 };
    let model = MlpNet::<InferB>::new(&mlp_cfg, &infer_device());
    let eval = BurnEvaluator::<InferB, _>::new(model, infer_device());

    let mut group = c.benchmark_group("episode");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(60));

    for &n_sim in &[1usize, 2] {
        let mcts_cfg = MctsConfig {
            n_simulations: n_sim,
            c_puct: 1.5,
            dirichlet_alpha: 0.0,
            dirichlet_eps: 0.0,
            temperature: 1.0,
        };

        group.bench_with_input(
            BenchmarkId::new("trictrac", n_sim),
            &n_sim,
            |b, _| {
                b.iter_batched(
                    seeded,
                    |mut rng| {
                        black_box(generate_episode(
                            &env,
                            &eval,
                            &mcts_cfg,
                            &|_| 1.0,
                            &mut rng,
                        ))
                    },
                    BatchSize::SmallInput,
                )
            },
        );
    }

    group.finish();
}

// ── 5. Training step ───────────────────────────────────────────────────────

/// Gradient-step latency for different batch sizes.
fn bench_train(c: &mut Criterion) {
    use burn::optim::AdamConfig;

    let mut group = c.benchmark_group("train");
    group.measurement_time(Duration::from_secs(10));

    let mlp_cfg = MlpConfig { obs_size: 217, action_size: 514, hidden_size: 64 };

    let dummy_samples = |n: usize| -> Vec<TrainSample> {
        (0..n)
            .map(|i| TrainSample {
                obs: vec![0.5; 217],
                policy: {
                    let mut p = vec![0.0f32; 514];
                    p[i % 514] = 1.0;
                    p
                },
                value: if i % 2 == 0 { 1.0 } else { -1.0 },
            })
            .collect()
    };

    for &batch_size in &[16usize, 64] {
        let batch = dummy_samples(batch_size);

        group.bench_with_input(
            BenchmarkId::new("mlp64_adam", batch_size),
            &batch_size,
            |b, _| {
                b.iter_batched(
                    || {
                        (
                            MlpNet::<TrainB>::new(&mlp_cfg, &train_device()),
                            AdamConfig::new().init::<TrainB, MlpNet<TrainB>>(),
                        )
                    },
                    |(model, mut opt)| {
                        black_box(train_step(model, &mut opt, &batch, &train_device(), 1e-3))
                    },
                    BatchSize::SmallInput,
                )
            },
        );
    }

    group.finish();
}

// ── Criterion entry point ──────────────────────────────────────────────────

criterion_group!(
    benches,
    bench_env,
    bench_network,
    bench_mcts,
    bench_episode,
    bench_train,
);
criterion_main!(benches);
