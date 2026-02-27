//! C++ bindings for the TricTrac game engine via cxx.rs.
//!
//! Exposes an opaque `TricTracEngine` type to C++. The C++ side
//! (open_spiel/games/trictrac/trictrac.cc) holds it via
//! `rust::Box<trictrac_engine::TricTracEngine>`.
//!
//! The Rust engine always reasons from White's (player 1's) perspective.
//! For Black (player 2), the board is mirrored before computing actions
//! and events are mirrored back before being applied — exactly as in
//! pyengine.rs.

use crate::dice::Dice;
use crate::game::{GameEvent, GameState, Stage, TurnStage};
use crate::training_common::{get_valid_action_indices, TrictracAction};

// ── cxx bridge declaration ────────────────────────────────────────────────────

#[cxx::bridge(namespace = "trictrac_engine")]
pub mod ffi {
    // ── Shared types (transparent to both Rust and C++) ───────────────────────

    /// Two dice values passed from C++ when applying a chance outcome.
    struct DicePair {
        die1: u8,
        die2: u8,
    }

    /// Both players' cumulative scores: holes * 12 + points.
    struct PlayerScores {
        score_p1: i32,
        score_p2: i32,
    }

    // ── Opaque Rust type and its free-function constructor ────────────────────

    extern "Rust" {
        /// Opaque handle to a running TricTrac game.
        /// C++ accesses this only through `rust::Box<TricTracEngine>`.
        type TricTracEngine;

        /// Construct a fresh engine with two players; player 1 (White) goes first.
        fn new_trictrac_engine() -> Box<TricTracEngine>;

        /// Deep-copy the engine — required by OpenSpiel's State::Clone().
        fn clone_engine(self: &TricTracEngine) -> Box<TricTracEngine>;

        // ── Queries ───────────────────────────────────────────────────────────

        /// True when the game is in TurnStage::RollWaiting (OpenSpiel chance node).
        fn needs_roll(self: &TricTracEngine) -> bool;

        /// True when Stage::Ended.
        fn is_game_ended(self: &TricTracEngine) -> bool;

        /// Active player index: 0 = player 1 (White), 1 = player 2 (Black).
        fn current_player_idx(self: &TricTracEngine) -> u64;

        /// Legal action indices for `player_idx` in [0, 513].
        /// Returns an empty vector when it is not that player's turn.
        fn get_legal_actions(self: &TricTracEngine, player_idx: u64) -> Result<Vec<u64>>;

        /// Human-readable description of an action index.
        fn action_to_string(self: &TricTracEngine, player_idx: u64, action_idx: u64) -> String;

        /// Both players' scores.
        fn get_players_scores(self: &TricTracEngine) -> PlayerScores;

        /// 36-element state vector (i8).  Mirrored for player_idx == 1.
        fn get_tensor(self: &TricTracEngine, player_idx: u64) -> Vec<i8>;

        /// Human-readable state description for `player_idx`.
        fn get_observation_string(self: &TricTracEngine, player_idx: u64) -> String;

        /// Full debug representation of the current state.
        fn to_debug_string(self: &TricTracEngine) -> String;

        // ── Mutations ─────────────────────────────────────────────────────────

        /// Apply a dice-roll result.  Returns Err (C++ exception) if not in
        /// the RollWaiting stage.
        fn apply_dice_roll(self: &mut TricTracEngine, dice: DicePair) -> Result<()>;

        /// Apply a player action.  Returns Err (C++ exception) if the action
        /// is not legal in the current state.
        fn apply_action(self: &mut TricTracEngine, action_idx: u64) -> Result<()>;
    }
}

// ── Opaque type ───────────────────────────────────────────────────────────────

pub struct TricTracEngine {
    game_state: GameState,
}

// ── Free-function constructor (declared in the bridge as a plain function) ────

pub fn new_trictrac_engine() -> Box<TricTracEngine> {
    let mut game_state = GameState::new(false); // schools_enabled = false
    game_state.init_player("player1");
    game_state.init_player("player2");
    game_state.consume(&GameEvent::BeginGame { goes_first: 1 });
    Box::new(TricTracEngine { game_state })
}

// ── Method implementations ────────────────────────────────────────────────────

impl TricTracEngine {
    fn clone_engine(&self) -> Box<TricTracEngine> {
        Box::new(TricTracEngine {
            game_state: self.game_state.clone(),
        })
    }

    fn needs_roll(&self) -> bool {
        self.game_state.turn_stage == TurnStage::RollWaiting
    }

    fn is_game_ended(&self) -> bool {
        self.game_state.stage == Stage::Ended
    }

    fn current_player_idx(&self) -> u64 {
        self.game_state.active_player_id - 1
    }

    fn get_legal_actions(&self, player_idx: u64) -> anyhow::Result<Vec<u64>> {
        if player_idx != self.current_player_idx() {
            return Ok(vec![]);
        }
        if player_idx == 0 {
            get_valid_action_indices(&self.game_state)
                .map(|v| v.into_iter().map(|i| i as u64).collect())
        } else {
            let mirror = self.game_state.mirror();
            get_valid_action_indices(&mirror).map(|v| v.into_iter().map(|i| i as u64).collect())
        }
    }

    fn action_to_string(&self, player_idx: u64, action_idx: u64) -> String {
        TrictracAction::from_action_index(action_idx as usize)
            .map(|a| format!("{}:{}", player_idx, a))
            .unwrap_or_else(|| "unknown action".into())
    }

    fn get_players_scores(&self) -> ffi::PlayerScores {
        ffi::PlayerScores {
            score_p1: self.score_for(1),
            score_p2: self.score_for(2),
        }
    }

    fn score_for(&self, player_id: u64) -> i32 {
        self.game_state
            .players
            .get(&player_id)
            .map(|p| p.holes as i32 * 12 + p.points as i32)
            .unwrap_or(-1)
    }

    fn get_tensor(&self, player_idx: u64) -> Vec<i8> {
        if player_idx == 0 {
            self.game_state.to_vec()
        } else {
            self.game_state.mirror().to_vec()
        }
    }

    fn get_observation_string(&self, player_idx: u64) -> String {
        if player_idx == 0 {
            format!("{}", self.game_state)
        } else {
            format!("{}", self.game_state.mirror())
        }
    }

    fn to_debug_string(&self) -> String {
        format!("{}", self.game_state)
    }

    fn apply_dice_roll(&mut self, dice: ffi::DicePair) -> anyhow::Result<()> {
        if self.game_state.turn_stage != TurnStage::RollWaiting {
            anyhow::bail!(
                "apply_dice_roll: not in RollWaiting stage (currently {:?})",
                self.game_state.turn_stage
            );
        }
        let player_id = self.game_state.active_player_id;
        let dice = Dice {
            values: (dice.die1, dice.die2),
        };
        self.game_state
            .consume(&GameEvent::RollResult { player_id, dice });
        Ok(())
    }

    fn apply_action(&mut self, action_idx: u64) -> anyhow::Result<()> {
        let needs_mirror = self.game_state.active_player_id == 2;

        let event = TrictracAction::from_action_index(action_idx as usize).and_then(|a| {
            let state = if needs_mirror {
                &self.game_state.mirror()
            } else {
                &self.game_state
            };
            a.to_event(state)
                .map(|e| if needs_mirror { e.get_mirror(false) } else { e })
        });

        match event {
            Some(evt) if self.game_state.validate(&evt) => {
                self.game_state.consume(&evt);
                Ok(())
            }
            Some(_) => anyhow::bail!(
                "apply_action: action {} is not valid in current state",
                action_idx
            ),
            None => anyhow::bail!(
                "apply_action: could not build event from action index {}",
                action_idx
            ),
        }
    }
}
