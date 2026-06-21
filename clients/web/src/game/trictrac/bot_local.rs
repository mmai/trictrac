use trictrac_store::{Board, CheckerMove, Color, Dice, GameState, MoveRules, Stage, TurnStage};

use super::types::{PlayerAction, PreGameRollState};

const GUEST_PLAYER_ID: u64 = 2;

/// Returns the next action for the bot (mp_player 1 / guest), or None if it is not the bot's turn.
/// `pgr` is the current pre-game ceremony state if the ceremony is in progress.
pub fn bot_decide(game: &GameState, pgr: Option<&PreGameRollState>) -> Option<PlayerAction> {
    // During the ceremony, the bot (guest) rolls when its die is missing.
    if game.stage == Stage::PreGame {
        if let Some(pgr) = pgr {
            if pgr.guest_die.is_none() {
                return Some(PlayerAction::PreGameRoll);
            }
        }
        return None;
    }
    if game.stage == Stage::Ended {
        return None;
    }
    if game.active_player_id != GUEST_PLAYER_ID {
        return None;
    }
    match game.turn_stage {
        TurnStage::RollDice => Some(PlayerAction::Roll),
        // TurnStage::HoldOrGoChoice => Some(PlayerAction::Go),
        TurnStage::Move | TurnStage::HoldOrGoChoice => {
            let rules = MoveRules::new(&Color::Black, &game.board, game.dice);
            let sequences = rules.get_possible_moves_sequences(true, vec![]);
            // MoveRules with Color::Black mirrors the board internally, so
            // returned move coordinates are in mirrored (White) space — mirror back.
            let (m1, m2) = sequences
                .iter()
                .max_by(|(m1a, m2a), (m1b, m2b)| {
                    score_seq(&game.board, m1a, m2a)
                        .partial_cmp(&score_seq(&game.board, m1b, m2b))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .cloned()
                .unwrap_or((CheckerMove::default(), CheckerMove::default()));
            Some(PlayerAction::Move(m1.mirror(), m2.mirror()))
        }
        _ => None,
    }
}

/// Score a candidate bot move sequence using depth-1 expectiminimax.
/// For each of the 21 possible opponent dice pairs, the opponent picks the move that
/// minimises the bot's score; we average those minima weighted by dice probability.
/// `m1` and `m2` are in mirrored (White) space, as returned by MoveRules for Color::Black.
fn score_seq(board: &Board, m1: &CheckerMove, m2: &CheckerMove) -> f32 {
    // Apply bot's moves on the mirrored board, then restore normal coordinates → B1.
    let mut b_mirror = board.mirror();
    let _ = b_mirror.move_checker(&Color::White, *m1);
    let _ = b_mirror.move_checker(&Color::White, *m2);
    let b1 = b_mirror.mirror();

    // Expectiminimax: sum over all 21 distinct dice pairs, weighted by probability (out of 36).
    // Non-doubles have probability 2/36 each; doubles 1/36 each.
    let mut total = 0.0f32;
    for d1 in 1u8..=6 {
        for d2 in d1..=6 {
            let weight = if d1 == d2 { 1.0f32 } else { 2.0f32 };
            let opp_rules = MoveRules::new(&Color::White, &b1, Dice { values: (d1, d2) });
            let opp_seqs = opp_rules.get_possible_moves_sequences(true, vec![]);
            let min_score = if opp_seqs.is_empty() {
                evaluate(&b1.mirror())
            } else {
                opp_seqs
                    .iter()
                    .map(|(om1, om2)| {
                        let mut b2 = b1.clone();
                        let _ = b2.move_checker(&Color::White, *om1);
                        let _ = b2.move_checker(&Color::White, *om2);
                        evaluate(&b2.mirror())
                    })
                    .fold(f32::INFINITY, f32::min)
            };
            total += weight * min_score;
        }
    }
    total // proportional to expected score; dividing by 36 doesn't affect move ordering
}

/// Evaluate a board position from White's perspective (call after mirroring for Black).
fn evaluate(board: &Board) -> f32 {
    let mut score = 0.0f32;

    let white_fields = board.get_color_fields(Color::White);
    let black_fields = board.get_color_fields(Color::Black);

    // Bonus if rest corner filled (tuned: 6.0)
    let corner_field = board.get_color_corner(&Color::White);
    let (corner_count, _color) = board.get_field_checkers(corner_field).unwrap();
    if corner_count > 0 {
        score += 6.0;
    }

    // Quarter fill progress — quarters 1-6, 7-12, 19-24.
    // Quarter 13-18 is skipped: field 13 is the opponent's rest corner so White can never fill it.
    // quarter_filled tuned to 5.5 (was 8.0), quarter_progress kept at 0.3.
    for &q in &[1usize, 7, 19] {
        if board.is_quarter_filled(Color::White, q) {
            score += 5.5;
        } else {
            let missing = board.get_quarter_filling_candidate(Color::White);
            score += (6 - missing.len().min(6)) as f32 * 0.3;
        }
    }

    // Singleton exposure: penalise a White singleton at field f only when there is at least
    // one Black checker at a field g > f (opponent can potentially threaten it).
    let max_black_field = black_fields.iter().map(|(f, _)| *f).max().unwrap_or(0);
    for (f, count) in &white_fields {
        if *count == 1 && *f < max_black_field {
            score -= 0.5;
        }
    }

    // Exit zone progress: tuned to 0.0 — mid-game jan-filling dominates.
    // (term kept here as a reminder; re-enable when bearing-off phase is reached)

    score
}
