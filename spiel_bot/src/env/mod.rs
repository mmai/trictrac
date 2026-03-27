//! Game environment abstraction — the minimal "Rust OpenSpiel".
//!
//! A `GameEnv` describes the rules of a two-player, zero-sum game that may
//! contain stochastic (chance) nodes.  Algorithms such as AlphaZero, DQN,
//! and PPO interact with a game exclusively through this trait.
//!
//! # Node taxonomy
//!
//! Every game position belongs to one of four categories, returned by
//! [`GameEnv::current_player`]:
//!
//! | [`Player`] | Meaning |
//! |-----------|---------|
//! | `P1` | Player 1 (index 0) must choose an action |
//! | `P2` | Player 2 (index 1) must choose an action |
//! | `Chance` | A stochastic event must be sampled (dice roll, card draw…) |
//! | `Terminal` | The game is over; [`GameEnv::returns`] is meaningful |
//!
//! # Perspective convention
//!
//! [`GameEnv::observation`] always returns the board from *the requested
//! player's* point of view.  Callers pass `pov = 0` for Player 1 and
//! `pov = 1` for Player 2.  The implementation is responsible for any
//! mirroring required (e.g. Trictrac always reasons from White's side).

pub mod trictrac;
pub use trictrac::TrictracEnv;

/// Who controls the current game node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Player {
    /// Player 1 (index 0) is to move.
    P1,
    /// Player 2 (index 1) is to move.
    P2,
    /// A stochastic event (dice roll, etc.) must be resolved.
    Chance,
    /// The game is over.
    Terminal,
}

impl Player {
    /// Returns the player index (0 or 1) if this is a decision node,
    /// or `None` for `Chance` / `Terminal`.
    pub fn index(self) -> Option<usize> {
        match self {
            Player::P1 => Some(0),
            Player::P2 => Some(1),
            _ => None,
        }
    }

    pub fn is_decision(self) -> bool {
        matches!(self, Player::P1 | Player::P2)
    }

    pub fn is_chance(self) -> bool {
        self == Player::Chance
    }

    pub fn is_terminal(self) -> bool {
        self == Player::Terminal
    }
}

/// Trait that completely describes a two-player zero-sum game.
///
/// Implementors must be cheaply cloneable (the type is used as a stateless
/// factory; the mutable game state lives in `Self::State`).
pub trait GameEnv: Clone + Send + Sync + 'static {
    /// The mutable game state.  Must be `Clone` so MCTS can copy
    /// game trees without touching the environment.
    type State: Clone + Send + Sync;

    // ── State creation ────────────────────────────────────────────────────

    /// Create a fresh game state at the initial position.
    fn new_game(&self) -> Self::State;

    // ── Node queries ──────────────────────────────────────────────────────

    /// Classify the current node.
    fn current_player(&self, s: &Self::State) -> Player;

    /// Legal action indices at a decision node (`current_player` is `P1`/`P2`).
    ///
    /// The returned indices are in `[0, action_space())`.
    /// The result is unspecified (may panic or return empty) when called at a
    /// `Chance` or `Terminal` node.
    fn legal_actions(&self, s: &Self::State) -> Vec<usize>;

    // ── State mutation ────────────────────────────────────────────────────

    /// Apply a player action.  `action` must be a value returned by
    /// [`legal_actions`] for the current state.
    fn apply(&self, s: &mut Self::State, action: usize);

    /// Sample and apply a stochastic outcome.  Must only be called when
    /// `current_player(s) == Player::Chance`.
    fn apply_chance<R: rand::Rng>(&self, s: &mut Self::State, rng: &mut R);

    // ── Observation ───────────────────────────────────────────────────────

    /// Observation tensor from player `pov`'s perspective (0 = P1, 1 = P2).
    /// The returned slice has exactly [`obs_size()`] elements, all in `[0, 1]`.
    fn observation(&self, s: &Self::State, pov: usize) -> Vec<f32>;

    /// Number of floats returned by [`observation`].
    fn obs_size(&self) -> usize;

    /// Total number of distinct action indices (the policy head output size).
    fn action_space(&self) -> usize;

    // ── Terminal values ───────────────────────────────────────────────────

    /// Game outcome for each player, or `None` if the game is not over.
    ///
    /// Values are in `[-1, 1]`: `+1.0` = win, `-1.0` = loss, `0.0` = draw.
    /// Index 0 = Player 1, index 1 = Player 2.
    fn returns(&self, s: &Self::State) -> Option<[f32; 2]>;
}
