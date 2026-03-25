use serde::{Deserialize, Serialize};
use trictrac_store::{GameState, Stage, TurnStage};

// ── Actions sent by a player to the host backend ─────────────────────────────

#[derive(Clone, Serialize, Deserialize)]
pub enum PlayerAction {
    /// Active player requests a dice roll.
    Roll,
    /// Move one checker from `from` to `to` (field numbers 1–24, 0 = exit).
    Move { from: u8, to: u8 },
    /// Choose to "go" (advance) during HoldOrGoChoice.
    Go,
    /// Acknowledge point marking (hold / advance points).
    Mark,
}

// ── Incremental state update broadcast to all clients ────────────────────────

/// Carries a full state snapshot; `apply_delta` replaces the local state.
/// Simple and correct; can be refined to true diffs later.
#[derive(Clone, Serialize, Deserialize)]
pub struct GameDelta {
    pub state: ViewState,
}

// ── Full game snapshot ────────────────────────────────────────────────────────

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct ViewState {
    /// Board positions: index i = field i+1. Positive = white, negative = black.
    pub board: [i8; 24],
    pub stage: SerStage,
    pub turn_stage: SerTurnStage,
    /// Which multiplayer player_id (0 = host, 1 = guest) is the active player.
    pub active_mp_player: Option<u16>,
    /// Scores indexed by multiplayer player_id (0 = host, 1 = guest).
    pub scores: [PlayerScore; 2],
    /// Last rolled dice values.
    pub dice: (u8, u8),
}

impl ViewState {
    pub fn default_with_names(host_name: &str, guest_name: &str) -> Self {
        ViewState {
            board: [0i8; 24],
            stage: SerStage::PreGame,
            turn_stage: SerTurnStage::RollDice,
            active_mp_player: None,
            scores: [
                PlayerScore { name: host_name.to_string(), points: 0, holes: 0 },
                PlayerScore { name: guest_name.to_string(), points: 0, holes: 0 },
            ],
            dice: (0, 0),
        }
    }

    pub fn apply_delta(&mut self, delta: &GameDelta) {
        *self = delta.state.clone();
    }

    /// Convert a store `GameState` to a `ViewState`.
    /// `host_store_id` and `guest_store_id` are the trictrac `PlayerId`s assigned
    /// to the host (mp player 0) and guest (mp player 1) respectively.
    pub fn from_game_state(
        gs: &GameState,
        host_store_id: u64,
        guest_store_id: u64,
    ) -> Self {
        let board_vec = gs.board.to_vec();
        let board: [i8; 24] = board_vec.try_into().expect("board is always 24 fields");

        let stage = match gs.stage {
            Stage::PreGame => SerStage::PreGame,
            Stage::InGame  => SerStage::InGame,
            Stage::Ended   => SerStage::Ended,
        };
        let turn_stage = match gs.turn_stage {
            TurnStage::RollDice     => SerTurnStage::RollDice,
            TurnStage::RollWaiting  => SerTurnStage::RollWaiting,
            TurnStage::MarkPoints   => SerTurnStage::MarkPoints,
            TurnStage::HoldOrGoChoice => SerTurnStage::HoldOrGoChoice,
            TurnStage::Move         => SerTurnStage::Move,
            TurnStage::MarkAdvPoints => SerTurnStage::MarkAdvPoints,
        };

        let active_mp_player = if gs.active_player_id == host_store_id {
            Some(0)
        } else if gs.active_player_id == guest_store_id {
            Some(1)
        } else {
            None
        };

        let score_for = |store_id: u64| -> PlayerScore {
            gs.players
                .get(&store_id)
                .map(|p| PlayerScore {
                    name: p.name.clone(),
                    points: p.points,
                    holes: p.holes,
                })
                .unwrap_or_else(|| PlayerScore { name: String::new(), points: 0, holes: 0 })
        };

        ViewState {
            board,
            stage,
            turn_stage,
            active_mp_player,
            scores: [score_for(host_store_id), score_for(guest_store_id)],
            dice: (gs.dice.values.0, gs.dice.values.1),
        }
    }
}

// ── Score snapshot ────────────────────────────────────────────────────────────

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerScore {
    pub name: String,
    pub points: u8,
    pub holes: u8,
}

// ── Serialisable mirrors of store enums ──────────────────────────────────────

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum SerStage {
    PreGame,
    InGame,
    Ended,
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum SerTurnStage {
    RollDice,
    RollWaiting,
    MarkPoints,
    HoldOrGoChoice,
    Move,
    MarkAdvPoints,
}
