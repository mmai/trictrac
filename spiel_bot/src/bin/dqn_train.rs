//! DQN self-play training loop.
//!
//! # Usage
//!
//! ```sh
//! # Start fresh with default settings
//! cargo run -p spiel_bot --bin dqn_train --release
//!
//! # Custom hyperparameters
//! cargo run -p spiel_bot --bin dqn_train --release -- \
//!     --hidden 512 --n-iter 200 --n-games 20 --epsilon-decay 5000
//!
//! # Resume from a checkpoint
//! cargo run -p spiel_bot --bin dqn_train --release -- \
//!     --resume checkpoints/dqn_iter_0050.mpk --n-iter 100
//! ```
//!
//! # Options
//!
//! | Flag | Default | Description |
//! |------|---------|-------------|
//! | `--hidden N` | 256 | Hidden layer width |
//! | `--out DIR` | `checkpoints/` | Directory for checkpoint files |
//! | `--n-iter N` | 100 | Training iterations |
//! | `--n-games N` | 10 | Self-play games per iteration |
//! | `--n-train N` | 20 | Gradient steps per iteration |
//! | `--batch N` | 64 | Mini-batch size |
//! | `--replay-cap N` | 50000 | Replay buffer capacity |
//! | `--lr F` | 1e-3 | Adam learning rate |
//! | `--epsilon-start F` | 1.0 | Initial exploration rate |
//! | `--epsilon-end F` | 0.05 | Final exploration rate |
//! | `--epsilon-decay N` | 10000 | Gradient steps for ε to reach its floor |
//! | `--gamma F` | 0.99 | Discount factor |
//! | `--target-update N` | 500 | Hard-update target net every N steps |
//! | `--reward-scale F` | 12.0 | Divide raw rewards by this (12 = one hole → ±1) |
//! | `--save-every N` | 10 | Save checkpoint every N iterations |
//! | `--seed N` | 42 | RNG seed |
//! | `--resume PATH` | (none) | Load weights before training |

use std::path::{Path, PathBuf};
use std::time::Instant;

use burn::{
    backend::{Autodiff, NdArray},
    module::AutodiffModule,
    optim::AdamConfig,
    tensor::backend::Backend,
};
use rand::{SeedableRng, rngs::SmallRng};

use spiel_bot::{
    dqn::{
        DqnConfig, DqnReplayBuffer, compute_target_q, dqn_train_step,
        generate_dqn_episode, hard_update, linear_epsilon,
    },
    env::TrictracEnv,
    network::{QNet, QNetConfig},
};

type TrainB = Autodiff<NdArray<f32>>;
type InferB = NdArray<f32>;

// ── CLI ───────────────────────────────────────────────────────────────────────

struct Args {
    hidden: usize,
    out_dir: PathBuf,
    save_every: usize,
    seed: u64,
    resume: Option<PathBuf>,
    config: DqnConfig,
}

impl Default for Args {
    fn default() -> Self {
        Self {
            hidden: 256,
            out_dir: PathBuf::from("checkpoints"),
            save_every: 10,
            seed: 42,
            resume: None,
            config: DqnConfig::default(),
        }
    }
}

fn parse_args() -> Args {
    let raw: Vec<String> = std::env::args().collect();
    let mut a = Args::default();
    let mut i = 1;
    while i < raw.len() {
        match raw[i].as_str() {
            "--hidden"         => { i += 1; a.hidden = raw[i].parse().expect("--hidden: integer"); }
            "--out"            => { i += 1; a.out_dir = PathBuf::from(&raw[i]); }
            "--n-iter"         => { i += 1; a.config.n_iterations = raw[i].parse().expect("--n-iter: integer"); }
            "--n-games"        => { i += 1; a.config.n_games_per_iter = raw[i].parse().expect("--n-games: integer"); }
            "--n-train"        => { i += 1; a.config.n_train_steps_per_iter = raw[i].parse().expect("--n-train: integer"); }
            "--batch"          => { i += 1; a.config.batch_size = raw[i].parse().expect("--batch: integer"); }
            "--replay-cap"     => { i += 1; a.config.replay_capacity = raw[i].parse().expect("--replay-cap: integer"); }
            "--lr"             => { i += 1; a.config.learning_rate = raw[i].parse().expect("--lr: float"); }
            "--epsilon-start"  => { i += 1; a.config.epsilon_start = raw[i].parse().expect("--epsilon-start: float"); }
            "--epsilon-end"    => { i += 1; a.config.epsilon_end = raw[i].parse().expect("--epsilon-end: float"); }
            "--epsilon-decay"  => { i += 1; a.config.epsilon_decay_steps = raw[i].parse().expect("--epsilon-decay: integer"); }
            "--gamma"          => { i += 1; a.config.gamma = raw[i].parse().expect("--gamma: float"); }
            "--target-update"  => { i += 1; a.config.target_update_freq = raw[i].parse().expect("--target-update: integer"); }
            "--reward-scale"   => { i += 1; a.config.reward_scale = raw[i].parse().expect("--reward-scale: float"); }
            "--save-every"     => { i += 1; a.save_every = raw[i].parse().expect("--save-every: integer"); }
            "--seed"           => { i += 1; a.seed = raw[i].parse().expect("--seed: integer"); }
            "--resume"         => { i += 1; a.resume = Some(PathBuf::from(&raw[i])); }
            other => { eprintln!("Unknown argument: {other}"); std::process::exit(1); }
        }
        i += 1;
    }
    a
}

// ── Training loop ─────────────────────────────────────────────────────────────

fn train_loop(
    mut q_net: QNet<TrainB>,
    cfg: &QNetConfig,
    save_fn: &dyn Fn(&QNet<TrainB>, &Path) -> anyhow::Result<()>,
    args: &Args,
) {
    let train_device: <TrainB as Backend>::Device = Default::default();
    let infer_device: <InferB as Backend>::Device = Default::default();

    let mut optimizer = AdamConfig::new().init();
    let mut replay = DqnReplayBuffer::new(args.config.replay_capacity);
    let mut rng = SmallRng::seed_from_u64(args.seed);
    let env = TrictracEnv;

    let mut target_net: QNet<InferB> = hard_update::<TrainB, _>(&q_net);
    let mut global_step = 0usize;
    let mut epsilon = args.config.epsilon_start;

    println!(
        "\n{:-<60}\n dqn_train | {} iters | {} games/iter | {} train-steps/iter\n{:-<60}",
        "", args.config.n_iterations, args.config.n_games_per_iter,
        args.config.n_train_steps_per_iter, ""
    );

    for iter in 0..args.config.n_iterations {
        let t0 = Instant::now();

        // ── Self-play ────────────────────────────────────────────────────
        let infer_q: QNet<InferB> = q_net.valid();
        let mut new_samples = 0usize;

        for _ in 0..args.config.n_games_per_iter {
            let samples = generate_dqn_episode(
                &env, &infer_q, epsilon, &mut rng, &infer_device, args.config.reward_scale,
            );
            new_samples += samples.len();
            replay.extend(samples);
        }

        // ── Training ─────────────────────────────────────────────────────
        let mut loss_sum = 0.0f32;
        let mut n_steps = 0usize;

        if replay.len() >= args.config.batch_size {
            for _ in 0..args.config.n_train_steps_per_iter {
                let batch: Vec<_> = replay
                    .sample_batch(args.config.batch_size, &mut rng)
                    .into_iter()
                    .cloned()
                    .collect();

                // Target Q-values computed on the inference backend.
                let target_q = compute_target_q(
                    &target_net, &batch, cfg.action_size, &infer_device,
                );

                let (q, loss) = dqn_train_step(
                    q_net, &mut optimizer, &batch, &target_q,
                    &train_device, args.config.learning_rate, args.config.gamma,
                );
                q_net = q;
                loss_sum += loss;
                n_steps += 1;
                global_step += 1;

                // Hard-update target net every target_update_freq steps.
                if global_step % args.config.target_update_freq == 0 {
                    target_net = hard_update::<TrainB, _>(&q_net);
                }

                // Linear epsilon decay.
                epsilon = linear_epsilon(
                    args.config.epsilon_start,
                    args.config.epsilon_end,
                    global_step,
                    args.config.epsilon_decay_steps,
                );
            }
        }

        // ── Logging ──────────────────────────────────────────────────────
        let elapsed = t0.elapsed();
        let avg_loss = if n_steps > 0 { loss_sum / n_steps as f32 } else { f32::NAN };

        println!(
            "iter {:4}/{} | buf {:6} | +{:<4} samples | loss {:7.4} | ε {:.3} | {:.1}s",
            iter + 1,
            args.config.n_iterations,
            replay.len(),
            new_samples,
            avg_loss,
            epsilon,
            elapsed.as_secs_f32(),
        );

        // ── Checkpoint ───────────────────────────────────────────────────
        let is_last = iter + 1 == args.config.n_iterations;
        if (iter + 1) % args.save_every == 0 || is_last {
            let path = args.out_dir.join(format!("dqn_iter_{:04}.mpk", iter + 1));
            match save_fn(&q_net, &path) {
                Ok(()) => println!("  -> saved {}", path.display()),
                Err(e) => eprintln!("  Warning: checkpoint save failed: {e}"),
            }
        }
    }

    println!("\nDQN training complete.");
}

// ── Main ──────────────────────────────────────────────────────────────────────

fn main() {
    let args = parse_args();

    if let Err(e) = std::fs::create_dir_all(&args.out_dir) {
        eprintln!("Cannot create output directory {}: {e}", args.out_dir.display());
        std::process::exit(1);
    }

    let train_device: <TrainB as Backend>::Device = Default::default();
    let cfg = QNetConfig { obs_size: 217, action_size: 514, hidden_size: args.hidden };

    let q_net = match &args.resume {
        Some(path) => {
            println!("Resuming from {}", path.display());
            QNet::<TrainB>::load(&cfg, path, &train_device)
                .unwrap_or_else(|e| { eprintln!("Load failed: {e}"); std::process::exit(1); })
        }
        None => QNet::<TrainB>::new(&cfg, &train_device),
    };

    train_loop(q_net, &cfg, &|m: &QNet<TrainB>, path| m.valid().save(path), &args);
}
