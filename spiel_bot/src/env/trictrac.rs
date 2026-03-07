//! [`GameEnv`] implementation for Trictrac.
//!
//! # Game flow (schools_enabled = false)
//!
//! With scoring schools disabled (the standard training configuration),
//! `MarkPoints` and `MarkAdvPoints` stages are never reached — the engine
//! applies them automatically inside `RollResult` and `Move`.  The only
//! four stages that actually occur are:
//!
//! | `TurnStage` | [`Player`] kind | Handled by |
//! |-------------|-----------------|------------|
//! | `RollDice`  | `Chance`        | [`apply_chance`] |
//! | `RollWaiting` | `Chance`      | [`apply_chance`] |
//! | `HoldOrGoChoice` | `P1`/`P2` | [`apply`] |
//! | `Move`      | `P1`/`P2`       | [`apply`] |
//!
//! # Perspective
//!
//! The Trictrac engine always reasons from White's perspective.  Player 1 is
//! White; Player 2 is Black.  When Player 2 is active, the board is mirrored
//! before computing legal actions / the observation tensor, and the resulting
//! event is mirrored back before being applied to the real state.  This
//! mirrors the pattern used in `cxxengine.rs` and `random_game.rs`.

use trictrac_store::{
    training_common::{get_valid_action_indices, TrictracAction, ACTION_SPACE_SIZE},
    Dice, GameEvent, GameState, Stage, TurnStage,
};

use super::{GameEnv, Player};

/// Stateless factory that produces Trictrac [`GameState`] environments.
///
/// Schools (`schools_enabled`) are always disabled — scoring is automatic.
#[derive(Clone, Debug, Default)]
pub struct TrictracEnv;

impl GameEnv for TrictracEnv {
    type State = GameState;

    // ── State creation ────────────────────────────────────────────────────

    fn new_game(&self) -> GameState {
        GameState::new_with_players("P1", "P2")
    }

    // ── Node queries ──────────────────────────────────────────────────────

    fn current_player(&self, s: &GameState) -> Player {
        if s.stage == Stage::Ended {
            return Player::Terminal;
        }
        match s.turn_stage {
            TurnStage::RollDice | TurnStage::RollWaiting => Player::Chance,
            _ => {
                if s.active_player_id == 1 {
                    Player::P1
                } else {
                    Player::P2
                }
            }
        }
    }

    /// Returns the legal action indices for the active player.
    ///
    /// The board is automatically mirrored for Player 2 so that the engine
    /// always reasons from White's perspective.  The returned indices are
    /// identical in meaning for both players (checker ordinals are
    /// perspective-relative).
    ///
    /// # Panics
    ///
    /// Panics in debug builds if called at a `Chance` or `Terminal` node.
    fn legal_actions(&self, s: &GameState) -> Vec<usize> {
        debug_assert!(
            self.current_player(s).is_decision(),
            "legal_actions called at a non-decision node (turn_stage={:?})",
            s.turn_stage
        );
        let indices = if s.active_player_id == 2 {
            get_valid_action_indices(&s.mirror())
        } else {
            get_valid_action_indices(s)
        };
        indices.unwrap_or_default()
    }

    // ── State mutation ────────────────────────────────────────────────────

    /// Apply a player action index to the game state.
    ///
    /// For Player 2, the action is decoded against the mirrored board and
    /// the resulting event is un-mirrored before being applied.
    ///
    /// # Panics
    ///
    /// Panics in debug builds if `action` cannot be decoded or does not
    /// produce a valid event for the current state.
    fn apply(&self, s: &mut GameState, action: usize) {
        let needs_mirror = s.active_player_id == 2;

        let event = if needs_mirror {
            let view = s.mirror();
            TrictracAction::from_action_index(action)
                .and_then(|a| a.to_event(&view))
                .map(|e| e.get_mirror(false))
        } else {
            TrictracAction::from_action_index(action).and_then(|a| a.to_event(s))
        };

        match event {
            Some(e) => {
                s.consume(&e).expect("apply: consume failed for valid action");
            }
            None => {
                panic!("apply: action index {action} produced no event in state {s}");
            }
        }
    }

    /// Sample dice and advance through a chance node.
    ///
    /// Handles both `RollDice` (triggers the roll mechanism, then samples
    /// dice) and `RollWaiting` (only samples dice) in a single call so that
    /// callers never need to distinguish the two.
    ///
    /// # Panics
    ///
    /// Panics in debug builds if called at a non-Chance node.
    fn apply_chance<R: rand::Rng>(&self, s: &mut GameState, rng: &mut R) {
        debug_assert!(
            self.current_player(s).is_chance(),
            "apply_chance called at a non-Chance node (turn_stage={:?})",
            s.turn_stage
        );

        // Step 1: RollDice → RollWaiting (player initiates the roll).
        if s.turn_stage == TurnStage::RollDice {
            s.consume(&GameEvent::Roll {
                player_id: s.active_player_id,
            })
            .expect("apply_chance: Roll event failed");
        }

        // Step 2: RollWaiting → Move / HoldOrGoChoice / Ended.
        // With schools_enabled=false, point marking is automatic inside consume().
        let dice = Dice {
            values: (rng.random_range(1u8..=6), rng.random_range(1u8..=6)),
        };
        s.consume(&GameEvent::RollResult {
            player_id: s.active_player_id,
            dice,
        })
        .expect("apply_chance: RollResult event failed");
    }

    // ── Observation ───────────────────────────────────────────────────────

    fn observation(&self, s: &GameState, pov: usize) -> Vec<f32> {
        if pov == 0 {
            s.to_tensor()
        } else {
            s.mirror().to_tensor()
        }
    }

    fn obs_size(&self) -> usize {
        217
    }

    fn action_space(&self) -> usize {
        ACTION_SPACE_SIZE
    }

    // ── Terminal values ───────────────────────────────────────────────────

    /// Returns `Some([r1, r2])` when the game is over, `None` otherwise.
    ///
    /// The winner (higher cumulative score) receives `+1.0`; the loser
    /// receives `-1.0`; an exact tie gives `0.0` each.  A cumulative score
    /// is `holes × 12 + points`.
    fn returns(&self, s: &GameState) -> Option<[f32; 2]> {
        if s.stage != Stage::Ended {
            return None;
        }
        let score = |id: u64| -> i32 {
            s.players
                .get(&id)
                .map(|p| p.holes as i32 * 12 + p.points as i32)
                .unwrap_or(0)
        };
        let s1 = score(1);
        let s2 = score(2);
        Some(match s1.cmp(&s2) {
            std::cmp::Ordering::Greater => [1.0, -1.0],
            std::cmp::Ordering::Less => [-1.0, 1.0],
            std::cmp::Ordering::Equal => [0.0, 0.0],
        })
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{rngs::SmallRng, Rng, SeedableRng};

    fn env() -> TrictracEnv {
        TrictracEnv
    }

    fn seeded_rng(seed: u64) -> SmallRng {
        SmallRng::seed_from_u64(seed)
    }

    // ── Initial state ─────────────────────────────────────────────────────

    #[test]
    fn new_game_is_chance_node() {
        let e = env();
        let s = e.new_game();
        // A fresh game starts at RollDice — a Chance node.
        assert_eq!(e.current_player(&s), Player::Chance);
        assert!(e.returns(&s).is_none());
    }

    #[test]
    fn new_game_is_not_terminal() {
        let e = env();
        let s = e.new_game();
        assert_ne!(e.current_player(&s), Player::Terminal);
        assert!(e.returns(&s).is_none());
    }

    // ── Chance nodes ──────────────────────────────────────────────────────

    #[test]
    fn apply_chance_reaches_decision_node() {
        let e = env();
        let mut s = e.new_game();
        let mut rng = seeded_rng(1);

        // A single chance step must yield a decision node (or end the game,
        // which only happens after 12 holes — impossible on the first roll).
        e.apply_chance(&mut s, &mut rng);
        let p = e.current_player(&s);
        assert!(
            p.is_decision(),
            "expected decision node after first roll, got {p:?}"
        );
    }

    #[test]
    fn apply_chance_from_rollwaiting() {
        // Check that apply_chance works when called mid-way (at RollWaiting).
        let e = env();
        let mut s = e.new_game();
        assert_eq!(s.turn_stage, TurnStage::RollDice);

        // Manually advance to RollWaiting.
        s.consume(&GameEvent::Roll { player_id: s.active_player_id })
            .unwrap();
        assert_eq!(s.turn_stage, TurnStage::RollWaiting);

        let mut rng = seeded_rng(2);
        e.apply_chance(&mut s, &mut rng);

        let p = e.current_player(&s);
        assert!(p.is_decision() || p.is_terminal());
    }

    // ── Legal actions ─────────────────────────────────────────────────────

    #[test]
    fn legal_actions_nonempty_after_roll() {
        let e = env();
        let mut s = e.new_game();
        let mut rng = seeded_rng(3);

        e.apply_chance(&mut s, &mut rng);
        assert!(e.current_player(&s).is_decision());

        let actions = e.legal_actions(&s);
        assert!(
            !actions.is_empty(),
            "legal_actions must be non-empty at a decision node"
        );
    }

    #[test]
    fn legal_actions_within_action_space() {
        let e = env();
        let mut s = e.new_game();
        let mut rng = seeded_rng(4);

        e.apply_chance(&mut s, &mut rng);
        for &a in e.legal_actions(&s).iter() {
            assert!(
                a < e.action_space(),
                "action {a} out of bounds (action_space={})",
                e.action_space()
            );
        }
    }

    // ── Observations ──────────────────────────────────────────────────────

    #[test]
    fn observation_has_correct_size() {
        let e = env();
        let mut s = e.new_game();
        let mut rng = seeded_rng(5);
        e.apply_chance(&mut s, &mut rng);

        assert_eq!(e.observation(&s, 0).len(), e.obs_size());
        assert_eq!(e.observation(&s, 1).len(), e.obs_size());
    }

    #[test]
    fn observation_values_in_unit_interval() {
        let e = env();
        let mut s = e.new_game();
        let mut rng = seeded_rng(6);
        e.apply_chance(&mut s, &mut rng);

        for (pov, obs) in [(0, e.observation(&s, 0)), (1, e.observation(&s, 1))] {
            for (i, &v) in obs.iter().enumerate() {
                assert!(
                    v >= 0.0 && v <= 1.0,
                    "pov={pov}: obs[{i}] = {v} is outside [0,1]"
                );
            }
        }
    }

    #[test]
    fn p1_and_p2_observations_differ() {
        // The board is mirrored for P2, so the two observations should differ
        // whenever there are checkers in non-symmetric positions (always true
        // in a real game after a few moves).
        let e = env();
        let mut s = e.new_game();
        let mut rng = seeded_rng(7);

        // Advance far enough that the board is non-trivial.
        for _ in 0..6 {
            while e.current_player(&s).is_chance() {
                e.apply_chance(&mut s, &mut rng);
            }
            if e.current_player(&s).is_terminal() {
                break;
            }
            let actions = e.legal_actions(&s);
            e.apply(&mut s, actions[0]);
        }

        if !e.current_player(&s).is_terminal() {
            let obs0 = e.observation(&s, 0);
            let obs1 = e.observation(&s, 1);
            assert_ne!(obs0, obs1, "P1 and P2 observations should differ on a non-symmetric board");
        }
    }

    // ── Applying actions ──────────────────────────────────────────────────

    #[test]
    fn apply_changes_state() {
        let e = env();
        let mut s = e.new_game();
        let mut rng = seeded_rng(8);

        e.apply_chance(&mut s, &mut rng);
        assert!(e.current_player(&s).is_decision());

        let before = s.clone();
        let action = e.legal_actions(&s)[0];
        e.apply(&mut s, action);

        assert_ne!(
            before.turn_stage, s.turn_stage,
            "state must change after apply"
        );
    }

    #[test]
    fn apply_all_legal_actions_do_not_panic() {
        // Verify that every action returned by legal_actions can be applied
        // without panicking (on several independent copies of the same state).
        let e = env();
        let mut s = e.new_game();
        let mut rng = seeded_rng(9);

        e.apply_chance(&mut s, &mut rng);
        assert!(e.current_player(&s).is_decision());

        for action in e.legal_actions(&s) {
            let mut copy = s.clone();
            e.apply(&mut copy, action); // must not panic
        }
    }

    // ── Full game ─────────────────────────────────────────────────────────

    /// Run a complete game with random actions through the `GameEnv` trait
    /// and verify that:
    /// - The game terminates.
    /// - `returns()` is `Some` at the end.
    /// - The outcome is valid: scores sum to 0 (zero-sum) or each player's
    ///   score is ±1 / 0.
    /// - No step panics.
    #[test]
    fn full_random_game_terminates() {
        let e = env();
        let mut s = e.new_game();
        let mut rng = seeded_rng(42);
        let max_steps = 50_000;

        for step in 0..max_steps {
            match e.current_player(&s) {
                Player::Terminal => break,
                Player::Chance => e.apply_chance(&mut s, &mut rng),
                Player::P1 | Player::P2 => {
                    let actions = e.legal_actions(&s);
                    assert!(!actions.is_empty(), "step {step}: empty legal actions at decision node");
                    let idx = rng.random_range(0..actions.len());
                    e.apply(&mut s, actions[idx]);
                }
            }
            assert!(step < max_steps - 1, "game did not terminate within {max_steps} steps");
        }

        let result = e.returns(&s);
        assert!(result.is_some(), "returns() must be Some at Terminal");

        let [r1, r2] = result.unwrap();
        let sum = r1 + r2;
        assert!(
            (sum.abs() < 1e-5) || (sum - 0.0).abs() < 1e-5,
            "game must be zero-sum: r1={r1}, r2={r2}, sum={sum}"
        );
        assert!(
            r1.abs() <= 1.0 && r2.abs() <= 1.0,
            "returns must be in [-1,1]: r1={r1}, r2={r2}"
        );
    }

    /// Run multiple games with different seeds to stress-test for panics.
    #[test]
    fn multiple_games_no_panic() {
        let e = env();
        let max_steps = 20_000;

        for seed in 0..10u64 {
            let mut s = e.new_game();
            let mut rng = seeded_rng(seed);

            for _ in 0..max_steps {
                match e.current_player(&s) {
                    Player::Terminal => break,
                    Player::Chance => e.apply_chance(&mut s, &mut rng),
                    Player::P1 | Player::P2 => {
                        let actions = e.legal_actions(&s);
                        let idx = rng.random_range(0..actions.len());
                        e.apply(&mut s, actions[idx]);
                    }
                }
            }
        }
    }

    // ── Returns ───────────────────────────────────────────────────────────

    #[test]
    fn returns_none_mid_game() {
        let e = env();
        let mut s = e.new_game();
        let mut rng = seeded_rng(11);

        // Advance a few steps but do not finish the game.
        for _ in 0..4 {
            match e.current_player(&s) {
                Player::Terminal => break,
                Player::Chance => e.apply_chance(&mut s, &mut rng),
                Player::P1 | Player::P2 => {
                    let actions = e.legal_actions(&s);
                    e.apply(&mut s, actions[0]);
                }
            }
        }

        if !e.current_player(&s).is_terminal() {
            assert!(
                e.returns(&s).is_none(),
                "returns() must be None before the game ends"
            );
        }
    }

    // ── Player 2 actions ──────────────────────────────────────────────────

    /// Verify that Player 2 (Black) can take actions without panicking,
    /// and that the state advances correctly.
    #[test]
    fn player2_can_act() {
        let e = env();
        let mut s = e.new_game();
        let mut rng = seeded_rng(12);

        // Keep stepping until Player 2 gets a turn.
        let max_steps = 5_000;
        let mut p2_acted = false;

        for _ in 0..max_steps {
            match e.current_player(&s) {
                Player::Terminal => break,
                Player::Chance => e.apply_chance(&mut s, &mut rng),
                Player::P2 => {
                    let actions = e.legal_actions(&s);
                    assert!(!actions.is_empty());
                    e.apply(&mut s, actions[0]);
                    p2_acted = true;
                    break;
                }
                Player::P1 => {
                    let actions = e.legal_actions(&s);
                    e.apply(&mut s, actions[0]);
                }
            }
        }

        assert!(p2_acted, "Player 2 never got a turn in {max_steps} steps");
    }
}
