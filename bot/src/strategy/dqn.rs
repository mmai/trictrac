use crate::{BotStrategy, CheckerMove, Color, GameState, PlayerId, PointsRules};
use store::MoveRules;
use rand::{thread_rng, Rng};
use std::collections::VecDeque;
use std::path::Path;
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
    weights1: Vec<Vec<f32>>,
    biases1: Vec<f32>,
    weights2: Vec<Vec<f32>>,
    biases2: Vec<f32>,
    weights3: Vec<Vec<f32>>,
    biases3: Vec<f32>,
}

impl SimpleNeuralNetwork {
    pub fn new(input_size: usize, hidden_size: usize, output_size: usize) -> Self {
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
}

/// Expérience pour le buffer de replay
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Experience {
    pub state: Vec<f32>,
    pub action: usize,
    pub reward: f32,
    pub next_state: Vec<f32>,
    pub done: bool,
}

/// Buffer de replay pour stocker les expériences
#[derive(Debug)]
pub struct ReplayBuffer {
    buffer: VecDeque<Experience>,
    capacity: usize,
}

impl ReplayBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn push(&mut self, experience: Experience) {
        if self.buffer.len() >= self.capacity {
            self.buffer.pop_front();
        }
        self.buffer.push_back(experience);
    }

    pub fn sample(&self, batch_size: usize) -> Vec<Experience> {
        let mut rng = thread_rng();
        let len = self.buffer.len();
        if len < batch_size {
            return self.buffer.iter().cloned().collect();
        }

        let mut batch = Vec::with_capacity(batch_size);
        for _ in 0..batch_size {
            let idx = rng.gen_range(0..len);
            batch.push(self.buffer[idx].clone());
        }
        batch
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }
}

/// Agent DQN pour l'apprentissage par renforcement
#[derive(Debug)]
pub struct DqnAgent {
    config: DqnConfig,
    model: SimpleNeuralNetwork,
    target_model: SimpleNeuralNetwork,
    replay_buffer: ReplayBuffer,
    epsilon: f64,
    step_count: usize,
}

impl DqnAgent {
    pub fn new(config: DqnConfig) -> Self {
        let model = SimpleNeuralNetwork::new(config.input_size, config.hidden_size, config.num_actions);
        let target_model = model.clone();
        let replay_buffer = ReplayBuffer::new(config.replay_buffer_size);
        let epsilon = config.epsilon;

        Self {
            config,
            model,
            target_model,
            replay_buffer,
            epsilon,
            step_count: 0,
        }
    }

    pub fn select_action(&mut self, state: &[f32]) -> usize {
        let mut rng = thread_rng();
        if rng.gen::<f64>() < self.epsilon {
            // Exploration : action aléatoire
            rng.gen_range(0..self.config.num_actions)
        } else {
            // Exploitation : meilleure action selon le modèle
            self.model.get_best_action(state)
        }
    }

    pub fn store_experience(&mut self, experience: Experience) {
        self.replay_buffer.push(experience);
    }

    pub fn train(&mut self) {
        if self.replay_buffer.len() < self.config.batch_size {
            return;
        }

        // Pour l'instant, on simule l'entraînement en mettant à jour epsilon
        // Dans une implémentation complète, ici on ferait la backpropagation
        self.epsilon = (self.epsilon * self.config.epsilon_decay).max(self.config.epsilon_min);
        self.step_count += 1;

        // Mise à jour du target model tous les 100 steps
        if self.step_count % 100 == 0 {
            self.target_model = self.model.clone();
        }
    }

    pub fn save_model<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>> {
        let data = serde_json::to_string_pretty(&self.model)?;
        std::fs::write(path, data)?;
        Ok(())
    }

    pub fn load_model<P: AsRef<Path>>(&mut self, path: P) -> Result<(), Box<dyn std::error::Error>> {
        let data = std::fs::read_to_string(path)?;
        self.model = serde_json::from_str(&data)?;
        self.target_model = self.model.clone();
        Ok(())
    }
}

/// Environnement Trictrac pour l'entraînement
#[derive(Debug)]
pub struct TrictracEnv {
    pub game_state: GameState,
    pub agent_player_id: PlayerId,
    pub opponent_player_id: PlayerId,
    pub agent_color: Color,
    pub max_steps: usize,
    pub current_step: usize,
}

impl TrictracEnv {
    pub fn new() -> Self {
        let mut game_state = GameState::new(false);
        game_state.init_player("agent");
        game_state.init_player("opponent");
        
        Self {
            game_state,
            agent_player_id: 1,
            opponent_player_id: 2,
            agent_color: Color::White,
            max_steps: 1000,
            current_step: 0,
        }
    }

    pub fn reset(&mut self) -> Vec<f32> {
        self.game_state = GameState::new(false);
        self.game_state.init_player("agent");
        self.game_state.init_player("opponent");
        self.current_step = 0;
        self.get_state_vector()
    }

    pub fn step(&mut self, _action: usize) -> (Vec<f32>, f32, bool) {
        let reward = 0.0; // Simplifié pour l'instant
        let done = self.game_state.stage == store::Stage::Ended || 
                   self.game_state.determine_winner().is_some() ||
                   self.current_step >= self.max_steps;

        self.current_step += 1;
        
        // Retourner l'état suivant
        let next_state = self.get_state_vector();
        
        (next_state, reward, done)
    }

    pub fn get_state_vector(&self) -> Vec<f32> {
        let mut state = Vec::with_capacity(32);

        // Plateau (24 cases)
        let white_positions = self.game_state.board.get_color_fields(Color::White);
        let black_positions = self.game_state.board.get_color_fields(Color::Black);
        
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
        state.push(self.game_state.active_player_id as f32);
        state.push(self.game_state.dice.values.0 as f32);
        state.push(self.game_state.dice.values.1 as f32);

        // Points et trous des joueurs
        if let Some(white_player) = self.game_state.get_white_player() {
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
}

/// Stratégie DQN pour le bot
#[derive(Debug)]
pub struct DqnStrategy {
    pub game: GameState,
    pub player_id: PlayerId,
    pub color: Color,
    pub agent: Option<DqnAgent>,
    pub env: TrictracEnv,
}

impl Default for DqnStrategy {
    fn default() -> Self {
        let game = GameState::default();
        let config = DqnConfig::default();
        let agent = DqnAgent::new(config);
        let env = TrictracEnv::new();
        
        Self {
            game,
            player_id: 2,
            color: Color::Black,
            agent: Some(agent),
            env,
        }
    }
}

impl DqnStrategy {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_with_model(model_path: &str) -> Self {
        let mut strategy = Self::new();
        if let Some(ref mut agent) = strategy.agent {
            let _ = agent.load_model(model_path);
        }
        strategy
    }

    pub fn train_episode(&mut self) -> f32 {
        let mut total_reward = 0.0;
        let mut state = self.env.reset();
        
        loop {
            let action = if let Some(ref mut agent) = self.agent {
                agent.select_action(&state)
            } else {
                0
            };

            let (next_state, reward, done) = self.env.step(action);
            total_reward += reward;

            if let Some(ref mut agent) = self.agent {
                let experience = Experience {
                    state: state.clone(),
                    action,
                    reward,
                    next_state: next_state.clone(),
                    done,
                };
                agent.store_experience(experience);
                agent.train();
            }

            if done {
                break;
            }
            state = next_state;
        }

        total_reward
    }

    pub fn save_model(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(ref agent) = self.agent {
            agent.save_model(path)?;
        }
        Ok(())
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
        // Pour l'instant, utilisation de la méthode standard
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
        // Utiliser le DQN pour décider (simplifié pour l'instant)
        if let Some(ref agent) = self.agent {
            let state = self.env.get_state_vector();
            // Action 2 = "go", on vérifie si c'est la meilleure action
            let q_values = agent.model.forward(&state);
            if q_values.len() > 2 {
                return q_values[2] > q_values[0] && q_values[2] > *q_values.get(1).unwrap_or(&0.0);
            }
        }
        true // Fallback
    }

    fn choose_move(&self) -> (CheckerMove, CheckerMove) {
        // Pour l'instant, utiliser la stratégie par défaut
        // Plus tard, on pourrait utiliser le DQN pour choisir parmi les mouvements valides
        let rules = MoveRules::new(&self.color, &self.game.board, self.game.dice);
        let possible_moves = rules.get_possible_moves_sequences(true, vec![]);
        
        let chosen_move = if let Some(ref agent) = self.agent {
            // Utiliser le DQN pour choisir le meilleur mouvement
            let state = self.env.get_state_vector();
            let action = agent.model.get_best_action(&state);
            
            // Pour l'instant, on mappe simplement l'action à un mouvement
            // Dans une implémentation complète, on aurait un espace d'action plus sophistiqué
            let move_index = action.min(possible_moves.len().saturating_sub(1));
            *possible_moves.get(move_index).unwrap_or(&(CheckerMove::default(), CheckerMove::default()))
        } else {
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