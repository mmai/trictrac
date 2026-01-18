//! # Expose trictrac game state and rules in a python module
use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::board::CheckerMove;
use crate::dice::{Dice, DiceRoller};
use crate::game::{GameEvent, GameState, Stage, TurnStage};
use crate::game_rules_moves::MoveRules;
use crate::game_rules_points::PointsRules;
use crate::player::{Color, PlayerId};

#[pyclass]
struct TricTrac {
    game_state: GameState,
    dice_roll_sequence: Vec<(u8, u8)>,
    current_dice_index: usize,
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
        game_state.consume(&GameEvent::BeginGame { goes_first: 1 });

        TricTrac {
            game_state,
            dice_roll_sequence: Vec::new(),
            current_dice_index: 0,
        }
    }

    /// Obtenir l'état du jeu sous forme de dictionnaire
    fn get_state_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new(py);
        dict.set_item("stage", format!("{:?}", self.game_state.stage))?;
        dict.set_item("turn_stage", format!("{:?}", self.game_state.turn_stage))?;
        dict.set_item("active_player_id", self.game_state.active_player_id)?;
        
        // Board
        let board_list = self.game_state.board.to_vec(); // returns Vec<i8>
        dict.set_item("board", board_list)?;

        // Dice
        dict.set_item("dice", (self.game_state.dice.values.0, self.game_state.dice.values.1))?;

        // Players
        let players_dict = PyDict::new(py);
        for (id, player) in &self.game_state.players {
            let p_dict = PyDict::new(py);
            p_dict.set_item("color", format!("{:?}", player.color))?;
            p_dict.set_item("holes", player.holes)?;
            p_dict.set_item("points", player.points)?;
            p_dict.set_item("can_bredouille", player.can_bredouille)?;
            p_dict.set_item("dice_roll_count", player.dice_roll_count)?;
            players_dict.set_item(id, p_dict)?;
        }
        dict.set_item("players", players_dict)?;

        Ok(dict)
    }

    /// Lance les dés ou utilise la séquence prédéfinie
    fn roll_dice(&mut self) -> PyResult<(u8, u8)> {
        let player_id = self.game_state.active_player_id;

        if self.game_state.turn_stage != TurnStage::RollDice {
             return Err(pyo3::exceptions::PyRuntimeError::new_err("Not in RollDice stage"));
        }
        
        self.game_state.consume(&GameEvent::Roll { player_id });
        
        let dice = if self.current_dice_index < self.dice_roll_sequence.len() {
             let vals = self.dice_roll_sequence[self.current_dice_index];
             self.current_dice_index += 1;
             Dice { values: vals }
        } else {
             DiceRoller::default().roll()
        };
        
        self.game_state.consume(&GameEvent::RollResult { player_id, dice });
        
        Ok(dice.values)
    }

    /// Applique un mouvement (deux déplacements de dames)
    fn apply_move(&mut self, from1: usize, to1: usize, from2: usize, to2: usize) -> PyResult<()> {
        let player_id = self.game_state.active_player_id;
        
        let m1 = CheckerMove::new(from1, to1).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        let m2 = CheckerMove::new(from2, to2).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        
        let moves = (m1, m2);
        
        if !self.game_state.validate(&GameEvent::Move { player_id, moves }) {
             return Err(pyo3::exceptions::PyValueError::new_err("Invalid move"));
        }
        
        self.game_state.consume(&GameEvent::Move { player_id, moves });
        Ok(())
    }

    /// Obtenir l'état du jeu sous forme de chaîne de caractères compacte
    fn get_state_id(&self) -> String {
        self.game_state.to_string_id()
    }

    /// Renvoie les positions des pièces pour un joueur spécifique
    fn get_checker_positions(&self, color: Color) -> Vec<(usize, i8)> {
        self.game_state.board.get_color_fields(color)
    }

    /// Obtenir la liste des mouvements légaux sous forme de paires (from, to)
    fn get_available_moves(&self) -> Vec<((usize, usize), (usize, usize))> {
        // L'agent joue toujours le joueur actif
        let color = self
            .game_state
            .player_color_by_id(&self.game_state.active_player_id)
            .unwrap_or(Color::White);

        // Si ce n'est pas le moment de déplacer les pièces, retourner une liste vide
        if self.game_state.turn_stage != TurnStage::Move
            && self.game_state.turn_stage != TurnStage::HoldOrGoChoice
        {
            return vec![];
        }

        let rules = MoveRules::new(&color, &self.game_state.board, self.game_state.dice);
        let possible_moves = rules.get_possible_moves_sequences(true, vec![]);

        // Convertir les mouvements CheckerMove en tuples (from, to) pour Python
        possible_moves
            .into_iter()
            .map(|(move1, move2)| {
                (
                    (move1.get_from(), move1.get_to()),
                    (move2.get_from(), move2.get_to()),
                )
            })
            .collect()
    }

    /// Calcule les points maximaux que le joueur actif peut obtenir avec les dés actuels
    fn calculate_points(&self) -> u8 {
        let active_player = self
            .game_state
            .players
            .get(&self.game_state.active_player_id);

        if let Some(player) = active_player {
            let dice_roll_count = player.dice_roll_count;
            let color = player.color;

            let points_rules =
                PointsRules::new(&color, &self.game_state.board, self.game_state.dice);
            let (points, _) = points_rules.get_points(dice_roll_count);

            points
        } else {
            0
        }
    }

    /// Réinitialise la partie
    fn reset(&mut self) {
        self.game_state = GameState::new(false);

        // Initialiser 2 joueurs
        self.game_state.init_player("player1");
        self.game_state.init_player("player2");

        // Commencer la partie avec le joueur 1
        self.game_state
            .consume(&GameEvent::BeginGame { goes_first: 1 });

        // Réinitialiser l'index de la séquence de dés
        self.current_dice_index = 0;
    }

    /// Vérifie si la partie est terminée
    fn is_done(&self) -> bool {
        self.game_state.stage == Stage::Ended || self.game_state.determine_winner().is_some()
    }

    /// Obtenir le gagnant de la partie
    fn get_winner(&self) -> Option<PlayerId> {
        self.game_state.determine_winner()
    }

    /// Obtenir le score du joueur actif (nombre de trous)
    fn get_score(&self, player_id: PlayerId) -> i32 {
        if let Some(player) = self.game_state.players.get(&player_id) {
            player.holes as i32
        } else {
            -1
        }
    }

    /// Obtenir l'ID du joueur actif
    fn get_active_player_id(&self) -> PlayerId {
        self.game_state.active_player_id
    }

    /// Définir une séquence de dés à utiliser (pour la reproductibilité)
    fn set_dice_sequence(&mut self, sequence: Vec<(u8, u8)>) {
        self.dice_roll_sequence = sequence;
        self.current_dice_index = 0;
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
fn store(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<TricTrac>()?;

    Ok(())
}
