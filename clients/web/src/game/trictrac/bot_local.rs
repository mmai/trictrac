use rand::prelude::IndexedRandom;
use trictrac_store::{CheckerMove, Color, GameState, MoveRules, Stage, TurnStage};

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
            let mut rng = rand::rng();
            let (m1, m2) = sequences
                .choose(&mut rng)
                .cloned()
                .unwrap_or((CheckerMove::default(), CheckerMove::default()));
            // MoveRules with Color::Black mirrors the board internally, so
            // returned move coordinates are in mirrored (White) space — mirror back.
            Some(PlayerAction::Move(m1.mirror(), m2.mirror()))
        }
        _ => None,
    }
}
