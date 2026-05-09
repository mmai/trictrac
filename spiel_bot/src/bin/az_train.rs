//! AlphaZero self-play training loop.
//!
//! # Usage
//!
//! ```sh
//! # Start fresh (MLP, default settings)
//! cargo run -p spiel_bot --bin az_train --release
//!
//! # ResNet, 200 iterations, save every 20
//! cargo run -p spiel_bot --bin az_train --release -- \
//!     --arch resnet --n-iter 200 --save-every 20 --out checkpoints/
//!
//! # Resume from a checkpoint
//! cargo run -p spiel_bot --bin az_train --release -- \
//!     --resume checkpoints/iter_0050.mpk --arch mlp --n-iter 100
//! ```
//!
//! # Options
//!
//! | Flag | Default | Description |
//! |------|---------|-------------|
//! | `--arch mlp\|resnet` | `mlp` | Network architecture |
//! | `--hidden N` | 256/512 | Hidden layer width |
//! | `--out DIR` | `checkpoints/` | Directory for checkpoint files |
//! | `--n-iter N` | `100` | Training iterations |
//! | `--n-games N` | `10` | Self-play games per iteration |
//! | `--n-train N` | `20` | Gradient steps per iteration |
//! | `--n-sim N` | `100` | MCTS simulations per move |
//! | `--batch N` | `64` | Mini-batch size |
//! | `--replay-cap N` | `50000` | Replay buffer capacity |
//! | `--lr F` | `1e-3` | Peak (initial) learning rate |
//! | `--lr-min F` | `1e-4` | Floor learning rate (cosine annealing) |
//! | `--c-puct F` | `1.5` | PUCT exploration constant |
//! | `--dirichlet-alpha F` | `0.1` | Dirichlet noise alpha |
//! | `--dirichlet-eps F` | `0.25` | Dirichlet noise weight |
//! | `--temp-drop N` | `30` | Move after which temperature drops to 0 |
//! | `--save-every N` | `10` | Save checkpoint every N iterations |
//! | `--seed N` | `42` | RNG seed |
//! | `--resume PATH` | (none) | Load weights from checkpoint before training |

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
    alphazero::{
        BurnEvaluator, ReplayBuffer, TrainSample, cosine_lr, generate_episode, train_step,
    },
    env::TrictracEnv,
    mcts::MctsConfig,
    network::{MlpConfig, MlpNet, PolicyValueNet, ResNet, ResNetConfig},
};

type TrainB = Autodiff<NdArray<f32>>;
type InferB = NdArray<f32>;

// ── CLI ───────────────────────────────────────────────────────────────────────

struct Args {
    arch: String,
    hidden: Option<usize>,
    out_dir: PathBuf,
    n_iter: usize,
    n_games: usize,
    n_train: usize,
    n_sim: usize,
    batch_size: usize,
    replay_cap: usize,
    lr: f64,
    lr_min: f64,
    c_puct: f32,
    dirichlet_alpha: f32,
    dirichlet_eps: f32,
    temp_drop: usize,
    save_every: usize,
    seed: u64,
    resume: Option<PathBuf>,
}

impl Default for Args {
    fn default() -> Self {
        Self {
            arch: "mlp".into(),
            hidden: None,
            out_dir: PathBuf::from("checkpoints"),
            n_iter: 100,
            n_games: 10,
            n_train: 20,
            n_sim: 100,
            batch_size: 64,
            replay_cap: 50_000,
            lr: 1e-3,
            lr_min: 1e-4,
            c_puct: 1.5,
            dirichlet_alpha: 0.1,
            dirichlet_eps: 0.25,
            temp_drop: 30,
            save_every: 10,
            seed: 42,
            resume: None,
        }
    }
}

fn parse_args() -> Args {
    let raw: Vec<String> = std::env::args().collect();
    let mut a = Args::default();
    let mut i = 1;
    while i < raw.len() {
        match raw[i].as_str() {
            "--arch"            => { i += 1; a.arch = raw[i].clone(); }
            "--hidden"          => { i += 1; a.hidden = Some(raw[i].parse().expect("--hidden: integer")); }
            "--out"             => { i += 1; a.out_dir = PathBuf::from(&raw[i]); }
            "--n-iter"          => { i += 1; a.n_iter = raw[i].parse().expect("--n-iter: integer"); }
            "--n-games"         => { i += 1; a.n_games = raw[i].parse().expect("--n-games: integer"); }
            "--n-train"         => { i += 1; a.n_train = raw[i].parse().expect("--n-train: integer"); }
            "--n-sim"           => { i += 1; a.n_sim = raw[i].parse().expect("--n-sim: integer"); }
            "--batch"           => { i += 1; a.batch_size = raw[i].parse().expect("--batch: integer"); }
            "--replay-cap"      => { i += 1; a.replay_cap = raw[i].parse().expect("--replay-cap: integer"); }
            "--lr"              => { i += 1; a.lr = raw[i].parse().expect("--lr: float"); }
            "--lr-min"          => { i += 1; a.lr_min = raw[i].parse().expect("--lr-min: float"); }
            "--c-puct"          => { i += 1; a.c_puct = raw[i].parse().expect("--c-puct: float"); }
            "--dirichlet-alpha" => { i += 1; a.dirichlet_alpha = raw[i].parse().expect("--dirichlet-alpha: float"); }
            "--dirichlet-eps"   => { i += 1; a.dirichlet_eps = raw[i].parse().expect("--dirichlet-eps: float"); }
            "--temp-drop"       => { i += 1; a.temp_drop = raw[i].parse().expect("--temp-drop: integer"); }
            "--save-every"      => { i += 1; a.save_every = raw[i].parse().expect("--save-every: integer"); }
            "--seed"            => { i += 1; a.seed = raw[i].parse().expect("--seed: integer"); }
            "--resume"          => { i += 1; a.resume = Some(PathBuf::from(&raw[i])); }
            other => { eprintln!("Unknown argument: {other}"); std::process::exit(1); }
        }
        i += 1;
    }
    a
}

// ── Training loop ─────────────────────────────────────────────────────────────

/// Generic training loop, parameterised over the network type.
///
/// `save_fn` receives the **training-backend** model and the target path;
/// it is called in the match arm where the concrete network type is known.
fn train_loop<N>(
    mut model: N,
    save_fn: &dyn Fn(&N, &Path) -> anyhow::Result<()>,
    args: &Args,
)
where
    N: PolicyValueNet<TrainB> + AutodiffModule<TrainB> + Clone,
    <N as AutodiffModule<TrainB>>::InnerModule: PolicyValueNet<InferB> + Send + 'static,
{
    let train_device: <TrainB as Backend>::Device = Default::default();
    let infer_device: <InferB as Backend>::Device = Default::default();

    // Type is inferred as OptimizerAdaptor<Adam, N, TrainB> at the call site.
    let mut optimizer = AdamConfig::new().init();
    let mut replay = ReplayBuffer::new(args.replay_cap);
    let mut rng = SmallRng::seed_from_u64(args.seed);
    let env = TrictracEnv;

    // Total gradient steps (used for cosine LR denominator).
    let total_train_steps = (args.n_iter * args.n_train).max(1);
    let mut global_step = 0usize;

    println!(
        "\n{:-<60}\n az_train — {} | {} iters | {} games/iter | {} sims/move\n{:-<60}",
        "", args.arch, args.n_iter, args.n_games, args.n_sim, ""
    );

    for iter in 0..args.n_iter {
        let t0 = Instant::now();

        // ── Self-play ────────────────────────────────────────────────────
        // Convert to inference backend (zero autodiff overhead).
        let infer_model: <N as AutodiffModule<TrainB>>::InnerModule = model.valid();
        let evaluator: BurnEvaluator<InferB, <N as AutodiffModule<TrainB>>::InnerModule> =
            BurnEvaluator::new(infer_model, infer_device.clone());

        let mcts_cfg = MctsConfig {
            n_simulations: args.n_sim,
            c_puct: args.c_puct,
            dirichlet_alpha: args.dirichlet_alpha,
            dirichlet_eps: args.dirichlet_eps,
            temperature: 1.0,
        };

        let temp_drop = args.temp_drop;
        let temperature_fn = |step: usize| -> f32 {
            if step < temp_drop { 1.0 } else { 0.0 }
        };

        let mut new_samples = 0usize;
        for _ in 0..args.n_games {
            let samples =
                generate_episode(&env, &evaluator, &mcts_cfg, &temperature_fn, &mut rng);
            new_samples += samples.len();
            replay.extend(samples);
        }

        // ── Training ─────────────────────────────────────────────────────
        let mut loss_sum = 0.0f32;
        let mut n_steps = 0usize;

        if replay.len() >= args.batch_size {
            for _ in 0..args.n_train {
                let lr = cosine_lr(args.lr, args.lr_min, global_step, total_train_steps);
                let batch: Vec<TrainSample> = replay
                    .sample_batch(args.batch_size, &mut rng)
                    .into_iter()
                    .cloned()
                    .collect();
                let (m, loss) =
                    train_step(model, &mut optimizer, &batch, &train_device, lr);
                model = m;
                loss_sum += loss;
                n_steps += 1;
                global_step += 1;
            }
        }

        // ── Logging ──────────────────────────────────────────────────────
        let elapsed = t0.elapsed();
        let avg_loss = if n_steps > 0 { loss_sum / n_steps as f32 } else { f32::NAN };
        let lr_now = cosine_lr(args.lr, args.lr_min, global_step, total_train_steps);

        println!(
            "iter {:4}/{} | buf {:6} | +{:<4} samples | loss {:7.4} | lr {:.2e} | {:.1}s",
            iter + 1,
            args.n_iter,
            replay.len(),
            new_samples,
            avg_loss,
            lr_now,
            elapsed.as_secs_f32(),
        );

        // ── Checkpoint ───────────────────────────────────────────────────
        let is_last = iter + 1 == args.n_iter;
        if (iter + 1) % args.save_every == 0 || is_last {
            let path = args.out_dir.join(format!("iter_{:04}.mpk", iter + 1));
            match save_fn(&model, &path) {
                Ok(()) => println!("  -> saved {}", path.display()),
                Err(e) => eprintln!("  Warning: checkpoint save failed: {e}"),
            }
        }
    }

    println!("\nTraining complete.");
}

// ── Main ──────────────────────────────────────────────────────────────────────

fn main() {
    let args = parse_args();

    // Create output directory if it doesn't exist.
    if let Err(e) = std::fs::create_dir_all(&args.out_dir) {
        eprintln!("Cannot create output directory {}: {e}", args.out_dir.display());
        std::process::exit(1);
    }

    let train_device: <TrainB as Backend>::Device = Default::default();

    match args.arch.as_str() {
        "resnet" => {
            let hidden = args.hidden.unwrap_or(512);
            let cfg = ResNetConfig { obs_size: 217, action_size: 514, hidden_size: hidden };

            let model = match &args.resume {
                Some(path) => {
                    println!("Resuming from {}", path.display());
                    ResNet::<TrainB>::load(&cfg, path, &train_device)
                        .unwrap_or_else(|e| { eprintln!("Load failed: {e}"); std::process::exit(1); })
                }
                None => ResNet::<TrainB>::new(&cfg, &train_device),
            };

            train_loop(
                model,
                &|m: &ResNet<TrainB>, path: &Path| {
                    // Save via inference model to avoid autodiff record overhead.
                    m.valid().save(path)
                },
                &args,
            );
        }

        "mlp" | _ => {
            let hidden = args.hidden.unwrap_or(256);
            let cfg = MlpConfig { obs_size: 217, action_size: 514, hidden_size: hidden };

            let model = match &args.resume {
                Some(path) => {
                    println!("Resuming from {}", path.display());
                    MlpNet::<TrainB>::load(&cfg, path, &train_device)
                        .unwrap_or_else(|e| { eprintln!("Load failed: {e}"); std::process::exit(1); })
                }
                None => MlpNet::<TrainB>::new(&cfg, &train_device),
            };

            train_loop(
                model,
                &|m: &MlpNet<TrainB>, path: &Path| m.valid().save(path),
                &args,
            );
        }
    }
}
