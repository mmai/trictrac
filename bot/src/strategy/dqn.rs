use crate::{BotStrategy, CheckerMove, Color, GameState, PlayerId};
use log::info;
use std::path::Path;
use store::MoveRules;

use crate::dqn::dqn_common::{
    get_valid_actions, sample_valid_action, SimpleNeuralNetwork, TrictracAction,
};

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
            player_id: 1,
            color: Color::White,
            model: None,
        }
    }
}

impl DqnStrategy {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_with_model<P: AsRef<Path> + std::fmt::Debug>(model_path: P) -> Self {
        let mut strategy = Self::new();
        if let Ok(model) = SimpleNeuralNetwork::load(&model_path) {
            info!("Loading model {model_path:?}");
            strategy.model = Some(model);
        }
        strategy
    }

    /// Utilise le modèle DQN pour choisir une action valide
    fn get_dqn_action(&self) -> Option<TrictracAction> {
        if let Some(ref model) = self.model {
            let state = self.game.to_vec_float();
            let valid_actions = get_valid_actions(&self.game);

            if valid_actions.is_empty() {
                return None;
            }

            // Obtenir les Q-values pour toutes les actions
            let q_values = model.forward(&state);

            // Trouver la meilleure action valide
            let mut best_action = &valid_actions[0];
            let mut best_q_value = f32::NEG_INFINITY;

            for action in &valid_actions {
                let action_index = action.to_action_index();
                if action_index < q_values.len() {
                    let q_value = q_values[action_index];
                    if q_value > best_q_value {
                        best_q_value = q_value;
                        best_action = action;
                    }
                }
            }

            Some(best_action.clone())
        } else {
            // Fallback : action aléatoire valide
            sample_valid_action(&self.game)
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
        self.game.dice_points.0
    }

    fn calculate_adv_points(&self) -> u8 {
        self.game.dice_points.1
    }

    fn choose_go(&self) -> bool {
        // Utiliser le DQN pour décider si on continue
        if let Some(action) = self.get_dqn_action() {
            matches!(action, TrictracAction::Go)
        } else {
            // Fallback : toujours continuer
            true
        }
    }

    fn choose_move(&self) -> (CheckerMove, CheckerMove) {
        // Utiliser le DQN pour choisir le mouvement
        if let Some(action) = self.get_dqn_action() {
            if let TrictracAction::Move {
                dice_order,
                from1,
                from2,
            } = action
            {
                let dicevals = self.game.dice.values;
                let (mut dice1, mut dice2) = if dice_order {
                    (dicevals.0, dicevals.1)
                } else {
                    (dicevals.1, dicevals.0)
                };

                if from1 == 0 {
                    // empty move
                    dice1 = 0;
                }
                let mut to1 = from1 + dice1 as usize;
                if 24 < to1 {
                    // sortie
                    to1 = 0;
                }
                if from2 == 0 {
                    // empty move
                    dice2 = 0;
                }
                let mut to2 = from2 + dice2 as usize;
                if 24 < to2 {
                    // sortie
                    to2 = 0;
                }

                let checker_move1 = CheckerMove::new(from1, to1).unwrap_or_default();
                let checker_move2 = CheckerMove::new(from2, to2).unwrap_or_default();

                let chosen_move = if self.color == Color::White {
                    (checker_move1, checker_move2)
                } else {
                    (checker_move1.mirror(), checker_move2.mirror())
                };

                return chosen_move;
            }
        }

        // Fallback : utiliser la stratégie par défaut
        let rules = MoveRules::new(&self.color, &self.game.board, self.game.dice);
        let possible_moves = rules.get_possible_moves_sequences(true, vec![]);

        let chosen_move = *possible_moves
            .first()
            .unwrap_or(&(CheckerMove::default(), CheckerMove::default()));

        if self.color == Color::White {
            chosen_move
        } else {
            (chosen_move.0.mirror(), chosen_move.1.mirror())
        }
    }
}
