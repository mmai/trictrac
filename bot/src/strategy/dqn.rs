use crate::{BotStrategy, CheckerMove, Color, GameState, PlayerId, PointsRules};
use std::path::Path;
use store::MoveRules;

use super::dqn_common::{DqnConfig, SimpleNeuralNetwork};

/// Stratégie DQN pour le bot - ne fait que charger et utiliser un modèle pré-entraîné
#[derive(Debug)]
pub struct DqnStrategy {
    pub game: GameState,
    pub player_id: PlayerId,
    pub color: Color,
    pub model: Option<SimpleNeuralNetwork>,
}

impl Default for DqnStrategy {
    fn default() -> Self {
        Self {
            game: GameState::default(),
            player_id: 2,
            color: Color::Black,
            model: None,
        }
    }
}

impl DqnStrategy {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_with_model<P: AsRef<Path>>(model_path: P) -> Self {
        let mut strategy = Self::new();
        if let Ok(model) = SimpleNeuralNetwork::load(model_path) {
            strategy.model = Some(model);
        }
        strategy
    }

    /// Utilise le modèle DQN pour choisir une action
    fn get_dqn_action(&self) -> Option<usize> {
        if let Some(ref model) = self.model {
            let state = self.game.to_vec_float();
            Some(model.get_best_action(&state))
        } else {
            None
        }
    }
}

impl BotStrategy for DqnStrategy {
    fn get_game(&self) -> &GameState {
        &self.game
    }

    fn get_mut_game(&mut self) -> &mut GameState {
        &mut self.game
    }

    fn set_color(&mut self, color: Color) {
        self.color = color;
    }

    fn set_player_id(&mut self, player_id: PlayerId) {
        self.player_id = player_id;
    }

    fn calculate_points(&self) -> u8 {
        let dice_roll_count = self
            .get_game()
            .players
            .get(&self.player_id)
            .unwrap()
            .dice_roll_count;
        let points_rules = PointsRules::new(&self.color, &self.game.board, self.game.dice);
        points_rules.get_points(dice_roll_count).0
    }

    fn calculate_adv_points(&self) -> u8 {
        self.calculate_points()
    }

    fn choose_go(&self) -> bool {
        // Utiliser le DQN pour décider si on continue (action 2 = "go")
        if let Some(action) = self.get_dqn_action() {
            // Si le modèle prédit l'action "go" (2), on continue
            action == 2
        } else {
            // Fallback : toujours continuer
            true
        }
    }

    fn choose_move(&self) -> (CheckerMove, CheckerMove) {
        let rules = MoveRules::new(&self.color, &self.game.board, self.game.dice);
        let possible_moves = rules.get_possible_moves_sequences(true, vec![]);

        let chosen_move = if let Some(action) = self.get_dqn_action() {
            // Utiliser l'action DQN pour choisir parmi les mouvements valides
            // Action 0 = premier mouvement, action 1 = mouvement moyen, etc.
            let move_index = if action == 0 {
                0 // Premier mouvement
            } else if action == 1 && possible_moves.len() > 1 {
                possible_moves.len() / 2 // Mouvement du milieu
            } else {
                possible_moves.len().saturating_sub(1) // Dernier mouvement
            };
            *possible_moves
                .get(move_index)
                .unwrap_or(&(CheckerMove::default(), CheckerMove::default()))
        } else {
            // Fallback : premier mouvement valide
            *possible_moves
                .first()
                .unwrap_or(&(CheckerMove::default(), CheckerMove::default()))
        };

        if self.color == Color::White {
            chosen_move
        } else {
            (chosen_move.0.mirror(), chosen_move.1.mirror())
        }
    }
}

