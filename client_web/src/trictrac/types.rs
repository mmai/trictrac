use serde::{Deserialize, Serialize};
use trictrac_store::{CheckerMove, GameState, Jan, Stage, TurnStage};

// ── Actions sent by a player to the host backend ─────────────────────────────

#[derive(Clone, Serialize, Deserialize)]
pub enum PlayerAction {
    /// Active player requests a dice roll.
    Roll,
    /// Both checker moves for this turn. Use `EMPTY_MOVE` (from=0, to=0) when a die
    /// has no valid move.
    Move(CheckerMove, CheckerMove),
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
    /// Jans (scoring events) triggered by the last dice roll.
    pub dice_jans: Vec<JanEntry>,
    /// Last two checker moves played; default when no move has occurred yet.
    pub dice_moves: (CheckerMove, CheckerMove),
}

/// One scoring event from a dice roll.
#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct JanEntry {
    pub jan: Jan,
    /// True when the dice are doubles (both same value) — changes the point value.
    /// Special case for HelplessMan: true when *both* dice are unplayable.
    pub is_double: bool,
    /// Number of distinct move pairs that produce this jan.
    pub ways: usize,
    /// Points per way (negative = scored against the active player).
    pub points_per: i8,
    /// Total = points_per × ways.
    pub total: i8,
    /// The move pairs that produce this jan (for move display).
    pub moves: Vec<(CheckerMove, CheckerMove)>,
}

impl ViewState {
    pub fn default_with_names(host_name: &str, guest_name: &str) -> Self {
        ViewState {
            board: [0i8; 24],
            stage: SerStage::PreGame,
            turn_stage: SerTurnStage::RollDice,
            active_mp_player: None,
            scores: [
                PlayerScore { name: host_name.to_string(), points: 0, holes: 0, can_bredouille: false },
                PlayerScore { name: guest_name.to_string(), points: 0, holes: 0, can_bredouille: false },
            ],
            dice: (0, 0),
            dice_jans: Vec::new(),
            dice_moves: (CheckerMove::default(), CheckerMove::default()),
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
                    can_bredouille: p.can_bredouille,
                })
                .unwrap_or_else(|| PlayerScore { name: String::new(), points: 0, holes: 0, can_bredouille: false })
        };

        // is_double for scoring: dice show the same value (both dice identical).
        // Exception: HelplessMan uses a special rule (see below).
        let dice_are_double = gs.dice.values.0 == gs.dice.values.1;

        // Build JanEntry list from the PossibleJans map.
        let empty_move = CheckerMove::new(0, 0).unwrap_or_default();
        let mut dice_jans: Vec<JanEntry> = gs.dice_jans
            .iter()
            .map(|(jan, moves)| {
                // HelplessMan: is_double = true only when *both* dice are unplayable
                // (the moves list contains a single (empty, empty) sentinel).
                let is_double = if *jan == Jan::HelplessMan {
                    moves.first().map(|&(m1, m2)| m1 == empty_move && m2 == empty_move)
                        .unwrap_or(false)
                } else {
                    dice_are_double
                };
                let points_per = jan.get_points(is_double);
                let ways = moves.len();
                let total = points_per.saturating_mul(ways as i8);
                JanEntry {
                    jan: jan.clone(),
                    is_double,
                    ways,
                    points_per,
                    total,
                    moves: moves.clone(),
                }
            })
            .collect();
        // Sort: highest total first, most-negative last.
        dice_jans.sort_by_key(|e| std::cmp::Reverse(e.total));

        ViewState {
            board,
            stage,
            turn_stage,
            active_mp_player,
            scores: [score_for(host_store_id), score_for(guest_store_id)],
            dice: (gs.dice.values.0, gs.dice.values.1),
            dice_jans,
            dice_moves: gs.dice_moves,
        }
    }
}

// ── Scored event (notification) ──────────────────────────────────────────

/// Points scored in a single scoring event, used for the notification panel.
#[derive(Clone, PartialEq)]
pub struct ScoredEvent {
    /// Raw points earned (sum of jan values; before hole wrapping).
    pub points_earned: u8,
    /// Number of holes gained (0 = no hole).
    pub holes_gained: u8,
    /// Total holes after this event.
    pub holes_total: u8,
    /// Was bredouille active when the hole was made (doubles hole count)?
    pub bredouille: bool,
    /// Contributing jans from this player's perspective (totals always positive).
    pub jans: Vec<JanEntry>,
}

// ── Score snapshot ────────────────────────────────────────────────────────────

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerScore {
    pub name: String,
    pub points: u8,
    pub holes: u8,
    pub can_bredouille: bool,
}

// ── Serialisable mirrors of store enums ──────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SerStage {
    PreGame,
    InGame,
    Ended,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SerTurnStage {
    RollDice,
    RollWaiting,
    MarkPoints,
    HoldOrGoChoice,
    Move,
    MarkAdvPoints,
}
