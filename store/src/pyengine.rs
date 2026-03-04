//! # Expose trictrac game state and rules in a python module
use pyo3::prelude::*;

use crate::dice::Dice;
use crate::game::{GameEvent, GameState, Stage, TurnStage};
use crate::player::PlayerId;
use crate::training_common::{get_valid_action_indices, TrictracAction};

#[pyclass]
struct TricTrac {
    game_state: GameState,
}

#[pymethods]
impl TricTrac {
    #[new]
    fn new() -> Self {
        let mut game_state = GameState::new(false); // schools_enabled = false

        // Initialiser 2 joueurs
        game_state.init_player("player1");
        game_state.init_player("player2");

        // Commencer la partie avec le joueur 1
        let _ = game_state.consume(&GameEvent::BeginGame { goes_first: 1 });

        TricTrac { game_state }
    }

    fn needs_roll(&self) -> bool {
        self.game_state.turn_stage == TurnStage::RollWaiting
    }

    fn is_game_ended(&self) -> bool {
        self.game_state.stage == Stage::Ended
    }

    // 0 or 1
    fn current_player_idx(&self) -> u64 {
        self.game_state.active_player_id - 1
    }

    fn get_legal_actions(&self, player_idx: u64) -> Vec<usize> {
        if player_idx == self.current_player_idx() {
            if player_idx == 0 {
                get_valid_action_indices(&self.game_state).unwrap()
            } else {
                let mirror = self.game_state.mirror();
                get_valid_action_indices(&mirror).unwrap()
            }
        } else {
            vec![]
        }
    }

    fn action_to_string(&self, player_idx: u64, action_idx: usize) -> String {
        TrictracAction::from_action_index(action_idx)
            .map(|a| format!("{}:{}", player_idx, a))
            .unwrap_or("unknown action".into())
    }

    fn apply_dice_roll(&mut self, dices: (u8, u8)) -> PyResult<()> {
        let player_id = self.game_state.active_player_id;

        if self.game_state.turn_stage != TurnStage::RollWaiting {
            return Err(pyo3::exceptions::PyRuntimeError::new_err(
                "Not in RollWaiting stage",
            ));
        }

        let dice = Dice { values: dices };
        let _ = self
            .game_state
            .consume(&GameEvent::RollResult { player_id, dice });
        Ok(())
    }

    fn apply_action(&mut self, action_idx: usize) -> PyResult<()> {
        if let Some(event) = TrictracAction::from_action_index(action_idx).and_then(|a| {
            let needs_mirror = self.game_state.active_player_id == 2;
            let game_state = if needs_mirror {
                &self.game_state.mirror()
            } else {
                &self.game_state
            };
            a.to_event(game_state)
                .map(|e| if needs_mirror { e.get_mirror(false) } else { e })
        }) {
            if self.game_state.validate(&event) {
                let _ = self.game_state.consume(&event);
                return Ok(());
            } else {
                return Err(pyo3::exceptions::PyRuntimeError::new_err(
                    "Action is invalid",
                ));
            }
        }
        Err(pyo3::exceptions::PyRuntimeError::new_err(
            "Could not apply action",
        ))
    }

    /// Get a player total score (holes & points)
    fn get_score(&self, player_id: PlayerId) -> i32 {
        if let Some(player) = self.game_state.players.get(&player_id) {
            player.holes as i32 * 12 + player.points as i32
        } else {
            -1
        }
    }

    fn get_players_scores(&self) -> [i32; 2] {
        [self.get_score(1), self.get_score(2)]
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

    /// Afficher l'état du jeu (pour le débogage)
    fn __str__(&self) -> String {
        format!("{}", self.game_state)
    }
}

/// A Python module implemented in Rust. The name of this function must match
/// the `lib.name` setting in the `Cargo.toml`, else Python will not be able to
/// import the module.
#[pymodule]
fn trictrac_store(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<TricTrac>()?;

    Ok(())
}
