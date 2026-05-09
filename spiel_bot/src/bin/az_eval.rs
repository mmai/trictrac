//! Evaluate a trained AlphaZero checkpoint against a random player.
//!
//! # Usage
//!
//! ```sh
//! # Random weights (sanity check — should be ~50 %)
//! cargo run -p spiel_bot --bin az_eval --release
//!
//! # Trained MLP checkpoint
//! cargo run -p spiel_bot --bin az_eval --release -- \
//!     --checkpoint model.mpk --arch mlp --n-games 200 --n-sim 50
//!
//! # Trained ResNet checkpoint
//! cargo run -p spiel_bot --bin az_eval --release -- \
//!     --checkpoint model.mpk --arch resnet --hidden 512 --n-games 100 --n-sim 100
//! ```
//!
//! # Options
//!
//! | Flag | Default | Description |
//! |------|---------|-------------|
//! | `--checkpoint <path>` | (none) | Load weights from `.mpk` file; random weights if omitted |
//! | `--arch mlp\|resnet` | `mlp` | Network architecture |
//! | `--hidden <N>` | 256 (mlp) / 512 (resnet) | Hidden size |
//! | `--n-games <N>` | `100` | Games per side (total = 2 × N) |
//! | `--n-sim <N>` | `50` | MCTS simulations per move |
//! | `--seed <N>` | `42` | RNG seed |
//! | `--c-puct <F>` | `1.5` | PUCT exploration constant |

use std::path::PathBuf;

use burn::backend::NdArray;
use rand::{SeedableRng, rngs::SmallRng, Rng};

use spiel_bot::{
    alphazero::BurnEvaluator,
    env::{GameEnv, Player, TrictracEnv},
    mcts::{Evaluator, MctsConfig, run_mcts, select_action},
    network::{MlpConfig, MlpNet, ResNet, ResNetConfig},
};

type InferB = NdArray<f32>;

// ── CLI ───────────────────────────────────────────────────────────────────────

struct Args {
    checkpoint: Option<PathBuf>,
    arch: String,
    hidden: Option<usize>,
    n_games: usize,
    n_sim: usize,
    seed: u64,
    c_puct: f32,
}

impl Default for Args {
    fn default() -> Self {
        Self {
            checkpoint: None,
            arch: "mlp".into(),
            hidden: None,
            n_games: 100,
            n_sim: 50,
            seed: 42,
            c_puct: 1.5,
        }
    }
}

fn parse_args() -> Args {
    let raw: Vec<String> = std::env::args().collect();
    let mut args = Args::default();
    let mut i = 1;
    while i < raw.len() {
        match raw[i].as_str() {
            "--checkpoint" => { i += 1; args.checkpoint = Some(PathBuf::from(&raw[i])); }
            "--arch"       => { i += 1; args.arch = raw[i].clone(); }
            "--hidden"     => { i += 1; args.hidden = Some(raw[i].parse().expect("--hidden must be an integer")); }
            "--n-games"    => { i += 1; args.n_games = raw[i].parse().expect("--n-games must be an integer"); }
            "--n-sim"      => { i += 1; args.n_sim = raw[i].parse().expect("--n-sim must be an integer"); }
            "--seed"       => { i += 1; args.seed = raw[i].parse().expect("--seed must be an integer"); }
            "--c-puct"     => { i += 1; args.c_puct = raw[i].parse().expect("--c-puct must be a float"); }
            other => { eprintln!("Unknown argument: {other}"); std::process::exit(1); }
        }
        i += 1;
    }
    args
}

// ── Game loop ─────────────────────────────────────────────────────────────────

/// Play one complete game.
///
/// `mcts_side` — 0 means MctsAgent plays as P1 (White), 1 means P2 (Black).
/// Returns `[r1, r2]` — P1 and P2 outcomes (+1 / -1 / 0).
fn play_game(
    env: &TrictracEnv,
    mcts_side: usize,
    evaluator: &dyn Evaluator,
    mcts_cfg: &MctsConfig,
    rng: &mut SmallRng,
) -> [f32; 2] {
    let mut state = env.new_game();
    loop {
        match env.current_player(&state) {
            Player::Terminal => {
                return env.returns(&state).expect("Terminal state must have returns");
            }
            Player::Chance => env.apply_chance(&mut state, rng),
            player => {
                let side = player.index().unwrap(); // 0 = P1, 1 = P2
                let action = if side == mcts_side {
                    let root = run_mcts(env, &state, evaluator, mcts_cfg, rng);
                    select_action(&root, 0.0, rng) // greedy (temperature = 0)
                } else {
                    let actions = env.legal_actions(&state);
                    actions[rng.random_range(0..actions.len())]
                };
                env.apply(&mut state, action);
            }
        }
    }
}

// ── Statistics ────────────────────────────────────────────────────────────────

#[derive(Default)]
struct Stats {
    wins: u32,
    draws: u32,
    losses: u32,
}

impl Stats {
    fn record(&mut self, mcts_return: f32) {
        if mcts_return > 0.0 { self.wins += 1; }
        else if mcts_return < 0.0 { self.losses += 1; }
        else { self.draws += 1; }
    }

    fn total(&self) -> u32 { self.wins + self.draws + self.losses }

    fn win_rate_decisive(&self) -> f64 {
        let d = self.wins + self.losses;
        if d == 0 { 0.5 } else { self.wins as f64 / d as f64 }
    }

    fn print(&self) {
        let n = self.total();
        let pct = |k: u32| 100.0 * k as f64 / n as f64;
        println!(
            "  Win {}/{n} ({:.1}%)  Draw {}/{n} ({:.1}%)  Loss {}/{n} ({:.1}%)",
            self.wins, pct(self.wins), self.draws, pct(self.draws), self.losses, pct(self.losses),
        );
    }
}

// ── Evaluation ────────────────────────────────────────────────────────────────

fn run_evaluation(
    evaluator: &dyn Evaluator,
    n_games: usize,
    mcts_cfg: &MctsConfig,
    seed: u64,
) -> (Stats, Stats) {
    let env = TrictracEnv;
    let total = n_games * 2;
    let mut as_p1 = Stats::default();
    let mut as_p2 = Stats::default();

    for i in 0..total {
        // Alternate sides: even games → MctsAgent as P1, odd → as P2.
        let mcts_side = i % 2;
        let mut rng = SmallRng::seed_from_u64(seed.wrapping_add(i as u64));
        let result = play_game(&env, mcts_side, evaluator, mcts_cfg, &mut rng);

        let mcts_return = result[mcts_side];
        if mcts_side == 0 { as_p1.record(mcts_return); } else { as_p2.record(mcts_return); }

        let done = i + 1;
        if done % 10 == 0 || done == total {
            eprint!("\r  [{done}/{total}] ", );
        }
    }
    eprintln!();
    (as_p1, as_p2)
}

// ── Main ──────────────────────────────────────────────────────────────────────

fn main() {
    let args = parse_args();
    let device: <InferB as burn::tensor::backend::Backend>::Device = Default::default();

    // ── Load model ────────────────────────────────────────────────────────
    let evaluator: Box<dyn Evaluator> = match args.arch.as_str() {
        "resnet" => {
            let hidden = args.hidden.unwrap_or(512);
            let cfg = ResNetConfig { obs_size: 217, action_size: 514, hidden_size: hidden };
            let model = match &args.checkpoint {
                Some(path) => ResNet::<InferB>::load(&cfg, path, &device)
                    .unwrap_or_else(|e| { eprintln!("Load failed: {e}"); std::process::exit(1); }),
                None => ResNet::new(&cfg, &device),
            };
            Box::new(BurnEvaluator::<InferB, ResNet<InferB>>::new(model, device))
        }
        "mlp" | _ => {
            let hidden = args.hidden.unwrap_or(256);
            let cfg = MlpConfig { obs_size: 217, action_size: 514, hidden_size: hidden };
            let model = match &args.checkpoint {
                Some(path) => MlpNet::<InferB>::load(&cfg, path, &device)
                    .unwrap_or_else(|e| { eprintln!("Load failed: {e}"); std::process::exit(1); }),
                None => MlpNet::new(&cfg, &device),
            };
            Box::new(BurnEvaluator::<InferB, MlpNet<InferB>>::new(model, device))
        }
    };

    let mcts_cfg = MctsConfig {
        n_simulations: args.n_sim,
        c_puct: args.c_puct,
        dirichlet_alpha: 0.0, // no exploration noise during evaluation
        dirichlet_eps: 0.0,
        temperature: 0.0,     // greedy action selection
    };

    // ── Header ────────────────────────────────────────────────────────────
    let ckpt_label = args.checkpoint
        .as_deref()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("random weights");

    println!();
    println!("az_eval — MctsAgent ({}, {ckpt_label}, n_sim={}) vs RandomAgent",
        args.arch, args.n_sim);
    println!("Games per side: {}  |  Total: {}  |  Seed: {}",
        args.n_games, args.n_games * 2, args.seed);
    println!();

    // ── Run ───────────────────────────────────────────────────────────────
    let (as_p1, as_p2) = run_evaluation(evaluator.as_ref(), args.n_games, &mcts_cfg, args.seed);

    // ── Results ───────────────────────────────────────────────────────────
    println!("MctsAgent as P1 (White):");
    as_p1.print();

    println!("MctsAgent as P2 (Black):");
    as_p2.print();

    let combined_wins = as_p1.wins + as_p2.wins;
    let combined_decisive = combined_wins + as_p1.losses + as_p2.losses;
    let combined_wr = if combined_decisive == 0 { 0.5 }
                      else { combined_wins as f64 / combined_decisive as f64 };

    println!();
    println!("Combined win rate (excluding draws): {:.1}%  [{}/{}]",
        combined_wr * 100.0, combined_wins, combined_decisive);
    println!("  P1 decisive: {:.1}%  |  P2 decisive: {:.1}%",
        as_p1.win_rate_decisive() * 100.0,
        as_p2.win_rate_decisive() * 100.0);
}
