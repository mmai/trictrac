//! Run one or many games of trictrac between two random players.
//! In single-game mode, prints play-by-play like OpenSpiel's `example.cc`.
//! In multi-game mode, runs silently and reports throughput at the end.
//!
//! Usage:
//!   cargo run --bin random_game -- [--seed <u64>] [--games <usize>] [--max-steps <usize>] [--verbose]

use std::borrow::Cow;
use std::env;
use std::time::Instant;

use trictrac_store::{
    training_common::sample_valid_action,
    Dice, DiceRoller, GameEvent, GameState, Stage, TurnStage,
};

// ── CLI args ──────────────────────────────────────────────────────────────────

struct Args {
    seed: Option<u64>,
    games: usize,
    max_steps: usize,
    verbose: bool,
}

fn parse_args() -> Args {
    let args: Vec<String> = env::args().collect();
    let mut seed = None;
    let mut games = 1;
    let mut max_steps = 10_000;
    let mut verbose = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--seed" => {
                i += 1;
                seed = args.get(i).and_then(|s| s.parse().ok());
            }
            "--games" => {
                i += 1;
                if let Some(v) = args.get(i).and_then(|s| s.parse().ok()) {
                    games = v;
                }
            }
            "--max-steps" => {
                i += 1;
                if let Some(v) = args.get(i).and_then(|s| s.parse().ok()) {
                    max_steps = v;
                }
            }
            "--verbose" => verbose = true,
            _ => {}
        }
        i += 1;
    }

    Args {
        seed,
        games,
        max_steps,
        verbose,
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn player_label(id: u64) -> &'static str {
    if id == 1 { "White" } else { "Black" }
}

/// Apply a `Roll` + `RollResult` in one logical step, returning the dice.
/// This collapses the two-step dice phase into a single "chance node" action,
/// matching how the OpenSpiel layer exposes it.
fn apply_dice_roll(state: &mut GameState, roller: &mut DiceRoller) -> Result<Dice, String> {
    // RollDice → RollWaiting
    state
        .consume(&GameEvent::Roll { player_id: state.active_player_id })
        .map_err(|e| format!("Roll event failed: {e}"))?;

    // RollWaiting → Move / HoldOrGoChoice (or Stage::Ended if 13th hole)
    let dice = roller.roll();
    state
        .consume(&GameEvent::RollResult { player_id: state.active_player_id, dice })
        .map_err(|e| format!("RollResult event failed: {e}"))?;

    Ok(dice)
}

/// Sample a random action and apply it to `state`, handling the Black-mirror
/// transform exactly as `cxxengine.rs::apply_action` does:
///
///   1. For Black, build a mirrored view of the state so that `sample_valid_action`
///      and `to_event` always reason from White's perspective.
///   2. Mirror the resulting event back to the original coordinate frame before
///      calling `state.consume`.
///
/// Returns the chosen action (in the view's coordinate frame) for display.
fn apply_player_action(state: &mut GameState) -> Result<(), String> {
    let needs_mirror = state.active_player_id == 2;

    // Build a White-perspective view: borrowed for White, owned mirror for Black.
    let view: Cow<GameState> = if needs_mirror {
        Cow::Owned(state.mirror())
    } else {
        Cow::Borrowed(state)
    };

    let action = sample_valid_action(&view)
        .ok_or_else(|| format!("no valid action in stage {:?}", state.turn_stage))?;

    let event = action
        .to_event(&view)
        .ok_or_else(|| format!("could not convert {action:?} to event"))?;

    // Translate the event from the view's frame back to the game's frame.
    let event = if needs_mirror { event.get_mirror(false) } else { event };

    state
        .consume(&event)
        .map_err(|e| format!("consume({action:?}): {e}"))?;

    Ok(())
}

// ── Single game ────────────────────────────────────────────────────────────────

/// Run one full game, optionally printing play-by-play.
/// Returns `(steps, truncated)`.
fn run_game(roller: &mut DiceRoller, max_steps: usize, quiet: bool, verbose: bool) -> (usize, bool) {
    let mut state = GameState::new_with_players("White", "Black");
    let mut step = 0usize;

    if !quiet {
        println!("{state}");
    }

    while state.stage != Stage::Ended {
        step += 1;
        if step > max_steps {
            return (step - 1, true);
        }

        match state.turn_stage {
            TurnStage::RollDice => {
                let player = state.active_player_id;
                match apply_dice_roll(&mut state, roller) {
                    Ok(dice) => {
                        if !quiet {
                            println!(
                                "[step {step:4}] {} rolls: {} & {}",
                                player_label(player),
                                dice.values.0,
                                dice.values.1
                            );
                        }
                    }
                    Err(e) => {
                        eprintln!("Error during dice roll: {e}");
                        eprintln!("State:\n{state}");
                        return (step, true);
                    }
                }
            }
            stage => {
                let player = state.active_player_id;
                match apply_player_action(&mut state) {
                    Ok(()) => {
                        if !quiet {
                            println!(
                                "[step {step:4}] {} ({stage:?})",
                                player_label(player)
                            );
                            if verbose {
                                println!("{state}");
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Error: {e}");
                        eprintln!("State:\n{state}");
                        return (step, true);
                    }
                }
            }
        }
    }

    if !quiet {
        println!("\n=== Game over after {step} steps ===\n");
        println!("{state}");

        let white = state.players.get(&1);
        let black = state.players.get(&2);

        match (white, black) {
            (Some(w), Some(b)) => {
                println!("White — holes: {:2}, points: {:2}", w.holes, w.points);
                println!("Black — holes: {:2}, points: {:2}", b.holes, b.points);
                println!();

                let white_score = w.holes as i32 * 12 + w.points as i32;
                let black_score = b.holes as i32 * 12 + b.points as i32;

                if white_score > black_score {
                    println!("Winner: White (+{})", white_score - black_score);
                } else if black_score > white_score {
                    println!("Winner: Black (+{})", black_score - white_score);
                } else {
                    println!("Draw");
                }
            }
            _ => eprintln!("Could not read final player scores."),
        }
    }

    (step, false)
}

// ── Main ──────────────────────────────────────────────────────────────────────

fn main() {
    let args = parse_args();
    let mut roller = DiceRoller::new(args.seed);

    if args.games == 1 {
        println!("=== Trictrac — random game ===");
        if let Some(s) = args.seed {
            println!("seed: {s}");
        }
        println!();
        run_game(&mut roller, args.max_steps, false, args.verbose);
    } else {
        println!("=== Trictrac — {} games ===", args.games);
        if let Some(s) = args.seed {
            println!("seed: {s}");
        }
        println!();

        let mut total_steps = 0u64;
        let mut truncated = 0usize;

        let t0 = Instant::now();
        for _ in 0..args.games {
            let (steps, trunc) = run_game(&mut roller, args.max_steps, !args.verbose, args.verbose);
            total_steps += steps as u64;
            if trunc {
                truncated += 1;
            }
        }
        let elapsed = t0.elapsed();

        let secs = elapsed.as_secs_f64();
        println!("Games      : {}", args.games);
        println!("Truncated  : {truncated}");
        println!("Total steps: {total_steps}");
        println!("Avg steps  : {:.1}", total_steps as f64 / args.games as f64);
        println!("Elapsed    : {:.3} s", secs);
        println!("Throughput : {:.1} games/s", args.games as f64 / secs);
        println!("            {:.0} steps/s", total_steps as f64 / secs);
    }
}
