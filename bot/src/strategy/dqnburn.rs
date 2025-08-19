use burn::backend::NdArray;
use burn::tensor::cast::ToElement;
use burn_rl::base::{ElemType, Model, State};

use crate::{BotStrategy, CheckerMove, Color, GameState, PlayerId};
use log::info;
use store::MoveRules;

use crate::burnrl::dqn::{dqn_model, utils};
use crate::burnrl::environment;
use crate::training_common::{get_valid_action_indices, sample_valid_action, TrictracAction};

type DqnBurnNetwork = dqn_model::Net<NdArray<ElemType>>;

/// Stratégie DQN pour le bot - ne fait que charger et utiliser un modèle pré-entraîné
#[derive(Debug)]
pub struct DqnBurnStrategy {
    pub game: GameState,
    pub player_id: PlayerId,
    pub color: Color,
    pub model: Option<DqnBurnNetwork>,
}

impl Default for DqnBurnStrategy {
    fn default() -> Self {
        Self {
            game: GameState::default(),
            player_id: 1,
            color: Color::White,
            model: None,
        }
    }
}

impl DqnBurnStrategy {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_with_model(model_path: &String) -> Self {
        info!("Loading model {model_path:?}");
        let mut strategy = Self::new();
        strategy.model = utils::load_model(256, model_path);
        strategy
    }

    /// Utilise le modèle DQN pour choisir une action valide
    fn get_dqn_action(&self) -> Option<TrictracAction> {
        if let Some(ref model) = self.model {
            let state = environment::TrictracState::from_game_state(&self.game);
            let valid_actions_indices = get_valid_action_indices(&self.game);
            if valid_actions_indices.is_empty() {
                return None; // No valid actions, end of episode
            }

            // Obtenir les Q-values pour toutes les actions
            let q_values = model.infer(state.to_tensor().unsqueeze());

            // Set non valid actions q-values to lowest
            let mut masked_q_values = q_values.clone();
            let q_values_vec: Vec<f32> = q_values.into_data().into_vec().unwrap();
            for (index, q_value) in q_values_vec.iter().enumerate() {
                if !valid_actions_indices.contains(&index) {
                    masked_q_values = masked_q_values.clone().mask_fill(
                        masked_q_values.clone().equal_elem(*q_value),
                        f32::NEG_INFINITY,
                    );
                }
            }
            // Get best action (highest q-value)
            let action_index = masked_q_values.argmax(1).into_scalar().to_u32();
            environment::TrictracEnvironment::convert_action(environment::TrictracAction::from(
                action_index,
            ))
        } else {
            // Fallback : action aléatoire valide
            sample_valid_action(&self.game)
        }
    }
}

impl BotStrategy for DqnBurnStrategy {
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
        if let Some(TrictracAction::Move {
            dice_order,
            checker1,
            checker2,
        }) = self.get_dqn_action()
        {
            let dicevals = self.game.dice.values;
            let (mut dice1, mut dice2) = if dice_order {
                (dicevals.0, dicevals.1)
            } else {
                (dicevals.1, dicevals.0)
            };

            assert_eq!(self.color, Color::White);
            let from1 = self
                .game
                .board
                .get_checker_field(&self.color, checker1 as u8)
                .unwrap_or(0);

            if from1 == 0 {
                // empty move
                dice1 = 0;
            }
            let mut to1 = from1;
            if self.color == Color::White {
                to1 += dice1 as usize;
                if 24 < to1 {
                    // sortie
                    to1 = 0;
                }
            } else {
                let fto1 = to1 as i16 - dice1 as i16;
                to1 = if fto1 < 0 { 0 } else { fto1 as usize };
            }

            let checker_move1 = store::CheckerMove::new(from1, to1).unwrap_or_default();

            let mut tmp_board = self.game.board.clone();
            let move_res = tmp_board.move_checker(&self.color, checker_move1);
            if move_res.is_err() {
                panic!("could not move {move_res:?}");
            }
            let from2 = tmp_board
                .get_checker_field(&self.color, checker2 as u8)
                .unwrap_or(0);
            if from2 == 0 {
                // empty move
                dice2 = 0;
            }
            let mut to2 = from2;
            if self.color == Color::White {
                to2 += dice2 as usize;
                if 24 < to2 {
                    // sortie
                    to2 = 0;
                }
            } else {
                let fto2 = to2 as i16 - dice2 as i16;
                to2 = if fto2 < 0 { 0 } else { fto2 as usize };
            }

            // Gestion prise de coin par puissance
            let opp_rest_field = if self.color == Color::White { 13 } else { 12 };
            if to1 == opp_rest_field && to2 == opp_rest_field {
                if self.color == Color::White {
                    to1 -= 1;
                    to2 -= 1;
                } else {
                    to1 += 1;
                    to2 += 1;
                }
            }

            let checker_move1 = CheckerMove::new(from1, to1).unwrap_or_default();
            let checker_move2 = CheckerMove::new(from2, to2).unwrap_or_default();

            let chosen_move = if self.color == Color::White {
                (checker_move1, checker_move2)
            } else {
                // XXX : really ?
                (checker_move1.mirror(), checker_move2.mirror())
            };

            return chosen_move;
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
