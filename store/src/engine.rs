//! # Expose trictrac game state and rules in a python module
use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::board::CheckerMove;
use crate::dice::Dice;
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
        game_state.init_player("bot");

        // Commencer la partie avec le joueur 1
        game_state.consume(&GameEvent::BeginGame { goes_first: 1 });

        TricTrac {
            game_state,
            dice_roll_sequence: Vec::new(),
            current_dice_index: 0,
        }
    }

    /// Obtenir l'état du jeu sous forme de chaîne de caractères compacte
    fn get_state_id(&self) -> String {
        self.game_state.to_string_id()
    }

    /// Obtenir l'état du jeu sous forme de dictionnaire pour faciliter l'entrainement
    fn get_state_dict(&self) -> PyResult<Py<PyDict>> {
        Python::with_gil(|py| {
            let state_dict = PyDict::new(py);

        // Informations essentielles sur l'état du jeu
        state_dict.set_item("active_player", self.game_state.active_player_id)?;
        state_dict.set_item("stage", format!("{:?}", self.game_state.stage))?;
        state_dict.set_item("turn_stage", format!("{:?}", self.game_state.turn_stage))?;

        // Dés
        let (dice1, dice2) = self.game_state.dice.values;
        state_dict.set_item("dice", (dice1, dice2))?;

        // Points des joueurs
        if let Some(white_player) = self.game_state.get_white_player() {
            state_dict.set_item("white_points", white_player.points)?;
            state_dict.set_item("white_holes", white_player.holes)?;
        }

        if let Some(black_player) = self.game_state.get_black_player() {
            state_dict.set_item("black_points", black_player.points)?;
            state_dict.set_item("black_holes", black_player.holes)?;
        }

        // Positions des pièces
        let white_positions = self.get_checker_positions(Color::White);
        let black_positions = self.get_checker_positions(Color::Black);

        state_dict.set_item("white_positions", white_positions)?;
        state_dict.set_item("black_positions", black_positions)?;

        // État compact pour la comparaison d'états
        state_dict.set_item("state_id", self.game_state.to_string_id())?;

            Ok(state_dict.into())
        })
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

    /// Jouer un coup ((from1, to1), (from2, to2))
    fn play_move(&mut self, moves: ((usize, usize), (usize, usize))) -> bool {
        let ((from1, to1), (from2, to2)) = moves;

        // Vérifier que c'est au tour du joueur de jouer
        if self.game_state.turn_stage != TurnStage::Move
            && self.game_state.turn_stage != TurnStage::HoldOrGoChoice
        {
            return false;
        }

        let move1 = CheckerMove::new(from1, to1).unwrap_or_default();
        let move2 = CheckerMove::new(from2, to2).unwrap_or_default();

        let event = GameEvent::Move {
            player_id: self.game_state.active_player_id,
            moves: (move1, move2),
        };

        // Vérifier si le mouvement est valide
        if !self.game_state.validate(&event) {
            return false;
        }

        // Exécuter le mouvement
        self.game_state.consume(&event);

        // Si l'autre joueur doit lancer les dés maintenant, simuler ce lancement
        if self.game_state.turn_stage == TurnStage::RollDice {
            self.roll_dice();
        }

        true
    }

    /// Lancer les dés (soit aléatoirement, soit en utilisant une séquence prédéfinie)
    fn roll_dice(&mut self) -> (u8, u8) {
        // Vérifier que c'est au bon moment pour lancer les dés
        if self.game_state.turn_stage != TurnStage::RollDice
            && self.game_state.turn_stage != TurnStage::RollWaiting
        {
            return self.game_state.dice.values;
        }

        // Simuler un lancer de dés
        let dice_values = if !self.dice_roll_sequence.is_empty()
            && self.current_dice_index < self.dice_roll_sequence.len()
        {
            // Utiliser la séquence prédéfinie
            let dice = self.dice_roll_sequence[self.current_dice_index];
            self.current_dice_index += 1;
            dice
        } else {
            // Générer aléatoirement
            (
                (1 + (rand::random::<u8>() % 6)),
                (1 + (rand::random::<u8>() % 6)),
            )
        };

        // Envoyer les événements appropriés
        let roll_event = GameEvent::Roll {
            player_id: self.game_state.active_player_id,
        };

        if self.game_state.validate(&roll_event) {
            self.game_state.consume(&roll_event);
        }

        let roll_result_event = GameEvent::RollResult {
            player_id: self.game_state.active_player_id,
            dice: Dice {
                values: dice_values,
            },
        };

        if self.game_state.validate(&roll_result_event) {
            self.game_state.consume(&roll_result_event);
        }

        dice_values
    }

    /// Marquer des points
    fn mark_points(&mut self, points: u8) -> bool {
        // Vérifier que c'est au bon moment pour marquer des points
        if self.game_state.turn_stage != TurnStage::MarkPoints
            && self.game_state.turn_stage != TurnStage::MarkAdvPoints
        {
            return false;
        }

        let event = GameEvent::Mark {
            player_id: self.game_state.active_player_id,
            points,
        };

        // Vérifier si l'événement est valide
        if !self.game_state.validate(&event) {
            return false;
        }

        // Exécuter l'événement
        self.game_state.consume(&event);

        // Si l'autre joueur doit lancer les dés maintenant, simuler ce lancement
        if self.game_state.turn_stage == TurnStage::RollDice {
            self.roll_dice();
        }

        true
    }

    /// Choisir de "continuer" (Go) après avoir gagné un trou
    fn choose_go(&mut self) -> bool {
        // Vérifier que c'est au bon moment pour choisir de continuer
        if self.game_state.turn_stage != TurnStage::HoldOrGoChoice {
            return false;
        }

        let event = GameEvent::Go {
            player_id: self.game_state.active_player_id,
        };

        // Vérifier si l'événement est valide
        if !self.game_state.validate(&event) {
            return false;
        }

        // Exécuter l'événement
        self.game_state.consume(&event);

        // Simuler le lancer de dés pour le prochain tour
        self.roll_dice();

        true
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
        self.game_state.init_player("bot");

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
