use serde::{Deserialize, Serialize};

/// Types d'actions possibles dans le jeu
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TrictracAction {
    /// Lancer les dés
    Roll,
    /// Marquer des points
    Mark { points: u8 },
    /// Continuer après avoir gagné un trou
    Go,
    /// Effectuer un mouvement de pions
    Move {
        move1: (usize, usize), // (from, to) pour le premier pion
        move2: (usize, usize), // (from, to) pour le deuxième pion
    },
}

impl TrictracAction {
    /// Encode une action en index pour le réseau de neurones
    pub fn to_action_index(&self) -> usize {
        match self {
            TrictracAction::Roll => 0,
            TrictracAction::Mark { points } => {
                1 + (*points as usize).min(12) // Indices 1-13 pour 0-12 points
            }
            TrictracAction::Go => 14,
            TrictracAction::Move { move1, move2 } => {
                // Encoder les mouvements dans l'espace d'actions
                // Indices 15+ pour les mouvements
                15 + encode_move_pair(*move1, *move2)
            }
        }
    }

    /// Décode un index d'action en TrictracAction
    pub fn from_action_index(index: usize) -> Option<TrictracAction> {
        match index {
            0 => Some(TrictracAction::Roll),
            1..=13 => Some(TrictracAction::Mark {
                points: (index - 1) as u8,
            }),
            14 => Some(TrictracAction::Go),
            i if i >= 15 => {
                let move_code = i - 15;
                let (move1, move2) = decode_move_pair(move_code);
                Some(TrictracAction::Move { move1, move2 })
            }
            _ => None,
        }
    }

    /// Retourne la taille de l'espace d'actions total
    pub fn action_space_size() -> usize {
        // 1 (Roll) + 13 (Mark 0-12) + 1 (Go) + mouvements possibles
        // Pour les mouvements : 25*25*25*25 = 390625 (position 0-24 pour chaque from/to)
        // Mais on peut optimiser en limitant aux positions valides (1-24)
        15 + (24 * 24 * 24 * 24) // = 331791
    }
}

/// Encode une paire de mouvements en un seul entier
fn encode_move_pair(move1: (usize, usize), move2: (usize, usize)) -> usize {
    let (from1, to1) = move1;
    let (from2, to2) = move2;
    // Assurer que les positions sont dans la plage 0-24
    let from1 = from1.min(24);
    let to1 = to1.min(24);
    let from2 = from2.min(24);
    let to2 = to2.min(24);

    from1 * (25 * 25 * 25) + to1 * (25 * 25) + from2 * 25 + to2
}

/// Décode un entier en paire de mouvements
fn decode_move_pair(code: usize) -> ((usize, usize), (usize, usize)) {
    let from1 = code / (25 * 25 * 25);
    let remainder = code % (25 * 25 * 25);
    let to1 = remainder / (25 * 25);
    let remainder = remainder % (25 * 25);
    let from2 = remainder / 25;
    let to2 = remainder % 25;

    ((from1, to1), (from2, to2))
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
    use store::{MoveRules, TurnStage};

    let mut valid_actions = Vec::new();

    let active_player_id = game_state.active_player_id;
    let player_color = game_state.player_color_by_id(&active_player_id);

    if let Some(color) = player_color {
        match game_state.turn_stage {
            TurnStage::RollDice | TurnStage::RollWaiting => {
                valid_actions.push(TrictracAction::Roll);
            }
            TurnStage::MarkPoints | TurnStage::MarkAdvPoints => {
                // Calculer les points possibles
                if let Some(player) = game_state.players.get(&active_player_id) {
                    let dice_roll_count = player.dice_roll_count;
                    let points_rules = PointsRules::new(&color, &game_state.board, game_state.dice);
                    let (max_points, _) = points_rules.get_points(dice_roll_count);

                    // Permettre de marquer entre 0 et max_points
                    for points in 0..=max_points {
                        valid_actions.push(TrictracAction::Mark { points });
                    }
                }
            }
            TurnStage::HoldOrGoChoice => {
                valid_actions.push(TrictracAction::Go);

                // Ajouter aussi les mouvements possibles
                let rules = MoveRules::new(&color, &game_state.board, game_state.dice);
                let possible_moves = rules.get_possible_moves_sequences(true, vec![]);

                for (move1, move2) in possible_moves {
                    valid_actions.push(TrictracAction::Move {
                        move1: (move1.get_from(), move1.get_to()),
                        move2: (move2.get_from(), move2.get_to()),
                    });
                }
            }
            TurnStage::Move => {
                let rules = MoveRules::new(&color, &game_state.board, game_state.dice);
                let possible_moves = rules.get_possible_moves_sequences(true, vec![]);

                for (move1, move2) in possible_moves {
                    valid_actions.push(TrictracAction::Move {
                        move1: (move1.get_from(), move1.get_to()),
                        move2: (move2.get_from(), move2.get_to()),
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
