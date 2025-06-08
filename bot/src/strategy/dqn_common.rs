use std::cmp::max;

use serde::{Deserialize, Serialize};
use store::{CheckerMove, Dice, GameEvent, PlayerId};

/// Types d'actions possibles dans le jeu
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TrictracAction {
    /// Lancer les dés
    Roll,
    /// Marquer les points
    Mark,
    /// Continuer après avoir gagné un trou
    Go,
    /// Effectuer un mouvement de pions
    Move {
        dice_order: bool, // true = utiliser dice[0] en premier, false = dice[1] en premier
        from1: usize,     // position de départ du premier pion (0-24)
        from2: usize,     // position de départ du deuxième pion (0-24)
    },
}

impl TrictracAction {
    /// Encode une action en index pour le réseau de neurones
    pub fn to_action_index(&self) -> usize {
        match self {
            TrictracAction::Roll => 0,
            TrictracAction::Mark => 1,
            TrictracAction::Go => 2,
            TrictracAction::Move {
                dice_order,
                from1,
                from2,
            } => {
                // Encoder les mouvements dans l'espace d'actions
                // Indices 3+ pour les mouvements
                let mut start = 3;
                if !dice_order {
                    // 25 * 25 = 625
                    start += 625;
                }
                start + from1 * 25 + from2
            }
        }
    }

    /// Décode un index d'action en TrictracAction
    pub fn from_action_index(index: usize) -> Option<TrictracAction> {
        match index {
            0 => Some(TrictracAction::Roll),
            1 => Some(TrictracAction::Mark),
            2 => Some(TrictracAction::Go),
            i if i >= 3 => {
                let move_code = i - 3;
                let (dice_order, from1, from2) = Self::decode_move(move_code);
                Some(TrictracAction::Move {
                    dice_order,
                    from1,
                    from2,
                })
            }
            _ => None,
        }
    }

    /// Décode un entier en paire de mouvements
    fn decode_move(code: usize) -> (bool, usize, usize) {
        let mut encoded = code;
        let dice_order = code < 626;
        if !dice_order {
            encoded -= 625
        }
        let from1 = encoded / 25;
        let from2 = encoded % 25;
        (dice_order, from1, from2)
    }

    /// Retourne la taille de l'espace d'actions total
    pub fn action_space_size() -> usize {
        // 1 (Roll) + 1 (Mark) + 1 (Go) + mouvements possibles
        // Pour les mouvements : 2*25*25 = 1250 (choix du dé + position 0-24 pour chaque from)
        // Mais on peut optimiser en limitant aux positions valides (1-24)
        3 + (2 * 25 * 25) // = 1253
    }

    // pub fn to_game_event(&self, player_id: PlayerId, dice: Dice) -> GameEvent {
    //     match action {
    //         TrictracAction::Roll => Some(GameEvent::Roll { player_id }),
    //         TrictracAction::Mark => Some(GameEvent::Mark { player_id, points }),
    //         TrictracAction::Go => Some(GameEvent::Go { player_id }),
    //         TrictracAction::Move {
    //             dice_order,
    //             from1,
    //             from2,
    //         } => {
    //             // Effectuer un mouvement
    //             let checker_move1 = store::CheckerMove::new(move1.0, move1.1).unwrap_or_default();
    //             let checker_move2 = store::CheckerMove::new(move2.0, move2.1).unwrap_or_default();
    //
    //             Some(GameEvent::Move {
    //                 player_id: self.agent_player_id,
    //                 moves: (checker_move1, checker_move2),
    //             })
    //         }
    //     };
    // }
}

/// Configuration pour l'agent DQN
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DqnConfig {
    pub state_size: usize,
    pub hidden_size: usize,
    pub num_actions: usize,
    pub learning_rate: f64,
    pub gamma: f64,
    pub epsilon: f64,
    pub epsilon_decay: f64,
    pub epsilon_min: f64,
    pub replay_buffer_size: usize,
    pub batch_size: usize,
}

impl Default for DqnConfig {
    fn default() -> Self {
        Self {
            state_size: 36,
            hidden_size: 512, // Augmenter la taille pour gérer l'espace d'actions élargi
            num_actions: TrictracAction::action_space_size(),
            learning_rate: 0.001,
            gamma: 0.99,
            epsilon: 0.1,
            epsilon_decay: 0.995,
            epsilon_min: 0.01,
            replay_buffer_size: 10000,
            batch_size: 32,
        }
    }
}

/// Réseau de neurones DQN simplifié (matrice de poids basique)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleNeuralNetwork {
    pub weights1: Vec<Vec<f32>>,
    pub biases1: Vec<f32>,
    pub weights2: Vec<Vec<f32>>,
    pub biases2: Vec<f32>,
    pub weights3: Vec<Vec<f32>>,
    pub biases3: Vec<f32>,
}

impl SimpleNeuralNetwork {
    pub fn new(input_size: usize, hidden_size: usize, output_size: usize) -> Self {
        use rand::{thread_rng, Rng};
        let mut rng = thread_rng();

        // Initialisation aléatoire des poids avec Xavier/Glorot
        let scale1 = (2.0 / input_size as f32).sqrt();
        let weights1 = (0..hidden_size)
            .map(|_| {
                (0..input_size)
                    .map(|_| rng.gen_range(-scale1..scale1))
                    .collect()
            })
            .collect();
        let biases1 = vec![0.0; hidden_size];

        let scale2 = (2.0 / hidden_size as f32).sqrt();
        let weights2 = (0..hidden_size)
            .map(|_| {
                (0..hidden_size)
                    .map(|_| rng.gen_range(-scale2..scale2))
                    .collect()
            })
            .collect();
        let biases2 = vec![0.0; hidden_size];

        let scale3 = (2.0 / hidden_size as f32).sqrt();
        let weights3 = (0..output_size)
            .map(|_| {
                (0..hidden_size)
                    .map(|_| rng.gen_range(-scale3..scale3))
                    .collect()
            })
            .collect();
        let biases3 = vec![0.0; output_size];

        Self {
            weights1,
            biases1,
            weights2,
            biases2,
            weights3,
            biases3,
        }
    }

    pub fn forward(&self, input: &[f32]) -> Vec<f32> {
        // Première couche
        let mut layer1: Vec<f32> = self.biases1.clone();
        for (i, neuron_weights) in self.weights1.iter().enumerate() {
            for (j, &weight) in neuron_weights.iter().enumerate() {
                if j < input.len() {
                    layer1[i] += input[j] * weight;
                }
            }
            layer1[i] = layer1[i].max(0.0); // ReLU
        }

        // Deuxième couche
        let mut layer2: Vec<f32> = self.biases2.clone();
        for (i, neuron_weights) in self.weights2.iter().enumerate() {
            for (j, &weight) in neuron_weights.iter().enumerate() {
                if j < layer1.len() {
                    layer2[i] += layer1[j] * weight;
                }
            }
            layer2[i] = layer2[i].max(0.0); // ReLU
        }

        // Couche de sortie
        let mut output: Vec<f32> = self.biases3.clone();
        for (i, neuron_weights) in self.weights3.iter().enumerate() {
            for (j, &weight) in neuron_weights.iter().enumerate() {
                if j < layer2.len() {
                    output[i] += layer2[j] * weight;
                }
            }
        }

        output
    }

    pub fn get_best_action(&self, input: &[f32]) -> usize {
        let q_values = self.forward(input);
        q_values
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .map(|(index, _)| index)
            .unwrap_or(0)
    }

    pub fn save<P: AsRef<std::path::Path>>(
        &self,
        path: P,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let data = serde_json::to_string_pretty(self)?;
        std::fs::write(path, data)?;
        Ok(())
    }

    pub fn load<P: AsRef<std::path::Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let data = std::fs::read_to_string(path)?;
        let network = serde_json::from_str(&data)?;
        Ok(network)
    }
}

/// Obtient les actions valides pour l'état de jeu actuel
pub fn get_valid_actions(game_state: &crate::GameState) -> Vec<TrictracAction> {
    use crate::PointsRules;
    use store::TurnStage;

    let mut valid_actions = Vec::new();

    let active_player_id = game_state.active_player_id;
    let player_color = game_state.player_color_by_id(&active_player_id);

    if let Some(color) = player_color {
        match game_state.turn_stage {
            TurnStage::RollDice | TurnStage::RollWaiting => {
                valid_actions.push(TrictracAction::Roll);
            }
            TurnStage::MarkPoints | TurnStage::MarkAdvPoints => {
                valid_actions.push(TrictracAction::Mark);
            }
            TurnStage::HoldOrGoChoice => {
                valid_actions.push(TrictracAction::Go);

                // Ajouter aussi les mouvements possibles
                let rules = store::MoveRules::new(&color, &game_state.board, game_state.dice);
                let possible_moves = rules.get_possible_moves_sequences(true, vec![]);

                for (move1, move2) in possible_moves {
                    let diff_move1 = move1.get_to() - move1.get_from();
                    valid_actions.push(TrictracAction::Move {
                        dice_order: diff_move1 == game_state.dice.values.0 as usize,
                        from1: move1.get_from(),
                        from2: move2.get_from(),
                    });
                }
            }
            TurnStage::Move => {
                let rules = store::MoveRules::new(&color, &game_state.board, game_state.dice);
                let possible_moves = rules.get_possible_moves_sequences(true, vec![]);

                for (move1, move2) in possible_moves {
                    let diff_move1 = move1.get_to() - move1.get_from();
                    valid_actions.push(TrictracAction::Move {
                        dice_order: diff_move1 == game_state.dice.values.0 as usize,
                        from1: move1.get_from(),
                        from2: move2.get_from(),
                    });
                }
            }
        }
    }

    valid_actions
}

/// Retourne les indices des actions valides
pub fn get_valid_action_indices(game_state: &crate::GameState) -> Vec<usize> {
    get_valid_actions(game_state)
        .into_iter()
        .map(|action| action.to_action_index())
        .collect()
}

/// Sélectionne une action valide aléatoire
pub fn sample_valid_action(game_state: &crate::GameState) -> Option<TrictracAction> {
    use rand::{seq::SliceRandom, thread_rng};

    let valid_actions = get_valid_actions(game_state);
    let mut rng = thread_rng();
    valid_actions.choose(&mut rng).cloned()
}
