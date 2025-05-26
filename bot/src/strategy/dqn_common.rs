use serde::{Deserialize, Serialize};

/// Configuration pour l'agent DQN
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DqnConfig {
    pub input_size: usize,
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
            input_size: 32,
            hidden_size: 256,
            num_actions: 3,
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
            .map(|_| (0..input_size).map(|_| rng.gen_range(-scale1..scale1)).collect())
            .collect();
        let biases1 = vec![0.0; hidden_size];
        
        let scale2 = (2.0 / hidden_size as f32).sqrt();
        let weights2 = (0..hidden_size)
            .map(|_| (0..hidden_size).map(|_| rng.gen_range(-scale2..scale2)).collect())
            .collect();
        let biases2 = vec![0.0; hidden_size];
        
        let scale3 = (2.0 / hidden_size as f32).sqrt();
        let weights3 = (0..output_size)
            .map(|_| (0..hidden_size).map(|_| rng.gen_range(-scale3..scale3)).collect())
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

    pub fn save<P: AsRef<std::path::Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>> {
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

/// Convertit l'état du jeu en vecteur d'entrée pour le réseau de neurones
pub fn game_state_to_vector(game_state: &crate::GameState) -> Vec<f32> {
    use crate::Color;
    
    let mut state = Vec::with_capacity(32);

    // Plateau (24 cases)
    let white_positions = game_state.board.get_color_fields(Color::White);
    let black_positions = game_state.board.get_color_fields(Color::Black);
    
    let mut board = vec![0.0; 24];
    for (pos, count) in white_positions {
        if pos < 24 {
            board[pos] = count as f32;
        }
    }
    for (pos, count) in black_positions {
        if pos < 24 {
            board[pos] = -(count as f32);
        }
    }
    state.extend(board);

    // Informations supplémentaires limitées pour respecter input_size = 32
    state.push(game_state.active_player_id as f32);
    state.push(game_state.dice.values.0 as f32);
    state.push(game_state.dice.values.1 as f32);

    // Points et trous des joueurs
    if let Some(white_player) = game_state.get_white_player() {
        state.push(white_player.points as f32);
        state.push(white_player.holes as f32);
    } else {
        state.extend(vec![0.0, 0.0]);
    }

    // Assurer que la taille est exactement input_size
    state.truncate(32);
    while state.len() < 32 {
        state.push(0.0);
    }

    state
}