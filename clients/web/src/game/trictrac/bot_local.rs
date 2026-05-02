use trictrac_store::{Board, CheckerMove, Color, GameState, MoveRules, Stage, TurnStage};

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

/// Score a candidate move sequence from the bot's (Black) perspective.
/// `m1` and `m2` are in mirrored (White) space, as returned by MoveRules for Color::Black.
fn score_seq(board: &Board, m1: &CheckerMove, m2: &CheckerMove) -> f32 {
    let mut b = board.mirror();
    let _ = b.move_checker(&Color::White, *m1);
    let _ = b.move_checker(&Color::White, *m2);
    evaluate(&b)
}

/// Evaluate a board position from White's perspective (call after mirroring for Black).
fn evaluate(board: &Board) -> f32 {
    let mut score = 0.0f32;

    let white_fields = board.get_color_fields(Color::White);
    let black_fields = board.get_color_fields(Color::Black);

    // Quarter fill progress — quarters 1-6, 7-12, 19-24.
    // Quarter 13-18 is skipped: field 13 is the opponent's rest corner so White can never fill it.
    for &q in &[1usize, 7, 19] {
        if board.is_quarter_filled(Color::White, q) {
            score += 8.0;
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

    // Exit zone progress: reward checkers already in fields 19-24.
    for (field, count) in &white_fields {
        if *field >= 19 {
            score += count.abs() as f32 * 0.3;
        }
    }

    score
}
