//! Weight tuner for the trictrac heuristic bot.
//!
//! Uses self-play (greedy heuristic with candidate weights vs current champion weights)
//! to measure win-rate signal. Since both bots are similarly capable, small weight
//! differences produce a gradient near 50%, unlike vs-random where the heuristic wins
//! ~100% regardless of weights.
//!
//! Algorithm: coordinate-descent hill-climbing. For each weight, probe +step and -step;
//! accept the change that pushes the challenger win-rate above 50%. Halve step when no
//! weight in the current pass improved. Stop when step < min_step.
//!
//! Each win-rate estimate runs `n_games` games with the challenger as White AND as Black
//! (total 2×n_games), eliminating first-move bias.
//!
//! Usage:
//!   cargo run --release --bin weight_tuner -- [--games <N>] [--seed <u64>] [--step <f32>] [--min-step <f32>]
//!
//! Prints the best weights at the end; paste them into bot_local.rs.

use std::borrow::Cow;
use std::time::Instant;

use trictrac_store::{
    training_common::sample_valid_action, Board, CheckerMove, Color, DiceRoller, GameEvent,
    GameState, MoveRules, Stage, TurnStage,
};

// ── Weights ───────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq)]
struct Weights {
    corner_filled: f32,     // bonus if rest corner (field 12 for White) is occupied
    quarter_filled: f32,    // bonus per fully filled quarter
    quarter_progress: f32,  // bonus per non-missing checker in the most-promising unfilled quarter
    singleton_penalty: f32, // penalty per exposed singleton (opponent checker at higher field)
    exit_zone: f32,         // bonus per checker already in fields 19-24
}

const WEIGHT_NAMES: [&str; 5] = [
    "corner_filled",
    "quarter_filled",
    "quarter_progress",
    "singleton_penalty",
    "exit_zone",
];

impl Weights {
    fn initial() -> Self {
        // Current hard-coded values from bot_local.rs
        Self {
            corner_filled: 5.0,
            quarter_filled: 8.0,
            quarter_progress: 0.3,
            singleton_penalty: 0.5,
            exit_zone: 0.3,
        }
    }

    fn get(&self, i: usize) -> f32 {
        match i {
            0 => self.corner_filled,
            1 => self.quarter_filled,
            2 => self.quarter_progress,
            3 => self.singleton_penalty,
            4 => self.exit_zone,
            _ => panic!("weight index out of range"),
        }
    }

    fn with(&self, i: usize, v: f32) -> Self {
        let mut w = self.clone();
        match i {
            0 => w.corner_filled = v,
            1 => w.quarter_filled = v,
            2 => w.quarter_progress = v,
            3 => w.singleton_penalty = v,
            4 => w.exit_zone = v,
            _ => panic!("weight index out of range"),
        }
        w
    }
}

// ── Evaluation ────────────────────────────────────────────────────────────────

/// Evaluate a board from White's perspective.
/// Mirrors evaluate() in bot_local.rs with parameterised weights.
fn evaluate(board: &Board, w: &Weights) -> f32 {
    let mut score = 0.0f32;

    let white_fields = board.get_color_fields(Color::White);
    let black_fields = board.get_color_fields(Color::Black);

    let corner_field = board.get_color_corner(&Color::White);
    let (corner_count, _) = board.get_field_checkers(corner_field).unwrap();
    if corner_count > 0 {
        score += w.corner_filled;
    }

    for &q in &[1usize, 7, 19] {
        if board.is_quarter_filled(Color::White, q) {
            score += w.quarter_filled;
        } else {
            let missing = board.get_quarter_filling_candidate(Color::White);
            score += (6 - missing.len().min(6)) as f32 * w.quarter_progress;
        }
    }

    let max_black_field = black_fields.iter().map(|(f, _)| *f).max().unwrap_or(0);
    for (f, count) in &white_fields {
        if *count == 1 && *f < max_black_field {
            score -= w.singleton_penalty;
        }
    }

    for (field, count) in &white_fields {
        if *field >= 19 {
            score += count.abs() as f32 * w.exit_zone;
        }
    }

    score
}

/// Greedy score for a move sequence.
/// `m1`, `m2` are in the MoveRules output space for `color` (mirrored White space for Black).
fn score_seq(board: &Board, m1: &CheckerMove, m2: &CheckerMove, color: Color, w: &Weights) -> f32 {
    // MoveRules for Black mirrors the board; sequences are in White space after mirror.
    // Replicate: use the mirrored board for Black, original for White.
    let mut b = if color == Color::White { board.clone() } else { board.mirror() };
    let _ = b.move_checker(&Color::White, *m1);
    let _ = b.move_checker(&Color::White, *m2);
    evaluate(&b, w)
}

// ── Bot actions ───────────────────────────────────────────────────────────────

/// Pick the greedy best move for the heuristic bot with the given color and weights.
/// Returns a GameEvent::Move with moves in the game's (non-mirrored) coordinate space.
fn heuristic_action(state: &GameState, color: Color, weights: &Weights) -> GameEvent {
    let rules = MoveRules::new(&color, &state.board, state.dice);
    let seqs = rules.get_possible_moves_sequences(true, vec![]);
    let (m1, m2) = seqs
        .iter()
        .max_by(|(a1, a2), (b1, b2)| {
            score_seq(&state.board, a1, a2, color, weights)
                .partial_cmp(&score_seq(&state.board, b1, b2, color, weights))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .copied()
        .unwrap_or_default();
    // MoveRules for Black returns moves in mirrored (White) space — mirror back.
    let (m1, m2) = if color == Color::Black { (m1.mirror(), m2.mirror()) } else { (m1, m2) };
    GameEvent::Move { player_id: state.active_player_id, moves: (m1, m2) }
}

/// Pick a uniformly random move for the random bot (used only in --vs-random mode).
fn random_action(state: &GameState) -> GameEvent {
    let view: Cow<GameState> = Cow::Owned(state.mirror());
    if let Some(action) = sample_valid_action(&view) {
        if let Some(event) = action.to_event(&view) {
            return event.get_mirror(false);
        }
    }
    GameEvent::Move {
        player_id: state.active_player_id,
        moves: (CheckerMove::default(), CheckerMove::default()),
    }
}

// ── Game simulation ───────────────────────────────────────────────────────────

const MAX_STEPS: usize = 8_000;

/// Simulate one self-play game.
/// Player 1 (White) uses `weights_p1`, player 2 (Black) uses `weights_p2`.
/// Returns the winner's player_id, or None on truncation.
fn run_selfplay_game(
    weights_p1: &Weights,
    weights_p2: &Weights,
    roller: &mut DiceRoller,
) -> Option<u64> {
    let mut state = GameState::new_with_players("Bot1", "Bot2");
    let mut steps = 0;

    while state.stage != Stage::Ended {
        steps += 1;
        if steps > MAX_STEPS {
            return None;
        }

        match state.turn_stage {
            TurnStage::RollDice => {
                let _ = state.consume(&GameEvent::Roll { player_id: state.active_player_id });
                let dice = roller.roll();
                let _ = state
                    .consume(&GameEvent::RollResult { player_id: state.active_player_id, dice });
            }
            _ => {
                let event = if state.active_player_id == 1 {
                    heuristic_action(&state, Color::White, weights_p1)
                } else {
                    heuristic_action(&state, Color::Black, weights_p2)
                };
                if state.consume(&event).is_err() {
                    return None;
                }
            }
        }
    }

    state.determine_winner()
}

/// Estimate challenger's win rate against champion via self-play.
/// Runs n_games with challenger as White and n_games with challenger as Black
/// to eliminate first-move bias. Returns fraction of games won by challenger.
fn self_play_win_rate(
    challenger: &Weights,
    champion: &Weights,
    n_games: usize,
    roller: &mut DiceRoller,
) -> f32 {
    let mut challenger_wins = 0usize;
    let total = n_games * 2;

    for _ in 0..n_games {
        // Challenger as White (player 1)
        if run_selfplay_game(challenger, champion, roller) == Some(1) {
            challenger_wins += 1;
        }
        // Challenger as Black (player 2)
        if run_selfplay_game(champion, challenger, roller) == Some(2) {
            challenger_wins += 1;
        }
    }

    challenger_wins as f32 / total as f32
}

/// Win rate of the heuristic bot (player 1 / White) against the random bot.
/// Useful as a sanity check, but not suitable for hill-climbing (win rate ≈ 100%).
fn vs_random_win_rate(weights: &Weights, n_games: usize, roller: &mut DiceRoller) -> f32 {
    let mut wins = 0usize;
    for _ in 0..n_games {
        let mut state = GameState::new_with_players("Heuristic", "Random");
        let mut steps = 0;
        while state.stage != Stage::Ended {
            steps += 1;
            if steps > MAX_STEPS {
                break;
            }
            match state.turn_stage {
                TurnStage::RollDice => {
                    let _ = state.consume(&GameEvent::Roll { player_id: state.active_player_id });
                    let dice = roller.roll();
                    let _ = state.consume(&GameEvent::RollResult {
                        player_id: state.active_player_id,
                        dice,
                    });
                }
                _ => {
                    let event = if state.active_player_id == 1 {
                        heuristic_action(&state, Color::White, weights)
                    } else {
                        random_action(&state)
                    };
                    let _ = state.consume(&event);
                }
            }
        }
        if state.determine_winner() == Some(1) {
            wins += 1;
        }
    }
    wins as f32 / n_games as f32
}

// ── Hill-climbing ─────────────────────────────────────────────────────────────

/// Coordinate-descent hill-climbing via self-play.
///
/// Compares each candidate (champion ± step on one weight) against the current
/// champion. Accepts the candidate if its self-play win rate exceeds `0.5 + margin`
/// (default 0.52 ≈ 2σ at N=150 games, i.e. N=300 total trials).
/// Halves step when a full pass produces no improvement; stops when step < min_step.
fn hill_climb(
    initial: Weights,
    n_games: usize,
    initial_step: f32,
    min_step: f32,
    margin: f32,
    roller: &mut DiceRoller,
) -> Weights {
    let threshold = 0.5 + margin;
    let mut champion = initial;
    let mut step = initial_step;

    println!("Initial weights: {:?}", champion);
    println!("Acceptance threshold: >{:.0}%  (margin={:.3})", threshold * 100.0, margin);
    println!();

    let mut iteration = 0usize;
    while step >= min_step {
        let mut improved = false;
        iteration += 1;

        for i in 0..5 {
            // Probe +step (clamped to non-negative).
            let up = champion.with(i, (champion.get(i) + step).max(0.0));
            let wr_up = self_play_win_rate(&up, &champion, n_games, roller);

            // Probe -step.
            let dn = champion.with(i, (champion.get(i) - step).max(0.0));
            let wr_dn = self_play_win_rate(&dn, &champion, n_games, roller);

            let best_wr = wr_up.max(wr_dn);
            if best_wr >= threshold {
                let (accepted, wr_accepted) =
                    if wr_up >= wr_dn { (up, wr_up) } else { (dn, wr_dn) };
                let dir = if wr_up >= wr_dn { '+' } else { '-' };
                println!(
                    "  iter {:3}  {} {}{:.3}  self-play win {:.1}%  {:?}",
                    iteration,
                    WEIGHT_NAMES[i],
                    dir,
                    step,
                    wr_accepted * 100.0,
                    accepted
                );
                champion = accepted;
                improved = true;
            }
        }

        if !improved {
            step *= 0.5;
            println!(
                "  iter {:3}  no improvement at step {:.3} → halving to {:.3}",
                iteration,
                step * 2.0,
                step
            );
        }
    }

    champion
}

// ── CLI args ──────────────────────────────────────────────────────────────────

struct Args {
    n_games: usize,
    seed: Option<u64>,
    initial_step: f32,
    min_step: f32,
    margin: f32,
    vs_random: bool,
}

fn parse_args() -> Args {
    let args: Vec<String> = std::env::args().collect();
    let mut n_games = 200usize;
    let mut seed: Option<u64> = None;
    let mut initial_step = 2.0f32;
    let mut min_step = 0.1f32;
    // At N=200 games × 2 directions = 400 total trials, σ ≈ sqrt(0.25/400) ≈ 2.5%.
    // margin=0.03 ≈ 1.2σ: catches real improvements while filtering most noise.
    let mut margin = 0.03f32;
    let mut vs_random = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--games" => {
                i += 1;
                if let Some(v) = args.get(i).and_then(|s| s.parse().ok()) {
                    n_games = v;
                }
            }
            "--seed" => {
                i += 1;
                seed = args.get(i).and_then(|s| s.parse().ok());
            }
            "--step" => {
                i += 1;
                if let Some(v) = args.get(i).and_then(|s| s.parse().ok()) {
                    initial_step = v;
                }
            }
            "--min-step" => {
                i += 1;
                if let Some(v) = args.get(i).and_then(|s| s.parse().ok()) {
                    min_step = v;
                }
            }
            "--margin" => {
                i += 1;
                if let Some(v) = args.get(i).and_then(|s| s.parse().ok()) {
                    margin = v;
                }
            }
            "--vs-random" => vs_random = true,
            _ => {}
        }
        i += 1;
    }

    Args { n_games, seed, initial_step, min_step, margin, vs_random }
}

// ── Main ──────────────────────────────────────────────────────────────────────

fn main() {
    let args = parse_args();

    println!("=== Trictrac weight tuner ===");
    println!("mode       : {}", if args.vs_random { "vs-random (no hill-climbing)" } else { "self-play hill-climbing" });
    println!("games/eval : {}", args.n_games);
    println!("seed       : {:?}", args.seed);
    if !args.vs_random {
        println!("step range : {:.3} → {:.3}", args.initial_step, args.min_step);
        println!("margin     : >{:.0}%", (0.5 + args.margin) * 100.0);
    }
    println!();

    let mut roller = DiceRoller::new(args.seed);
    let t0 = Instant::now();

    if args.vs_random {
        let wr = vs_random_win_rate(&Weights::initial(), args.n_games, &mut roller);
        println!("vs-random win rate: {:.1}%  ({} games)", wr * 100.0, args.n_games);
        println!("Elapsed: {:.1} s", t0.elapsed().as_secs_f64());
        return;
    }

    let best = hill_climb(
        Weights::initial(),
        args.n_games,
        args.initial_step,
        args.min_step,
        args.margin,
        &mut roller,
    );

    let elapsed = t0.elapsed();
    println!();
    println!("=== Optimised weights (paste into bot_local.rs) ===");
    println!("  corner_filled:     {}", best.corner_filled);
    println!("  quarter_filled:    {}", best.quarter_filled);
    println!("  quarter_progress:  {}", best.quarter_progress);
    println!("  singleton_penalty: {}", best.singleton_penalty);
    println!("  exit_zone:         {}", best.exit_zone);
    println!();
    println!("Elapsed: {:.1} s", elapsed.as_secs_f64());
}
