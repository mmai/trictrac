use crate::{Color, GameState, PlayerId};
use store::{GameEvent, MoveRules, PointsRules, Stage, TurnStage};
use rand::{thread_rng, Rng};
use std::collections::VecDeque;
use serde::{Deserialize, Serialize};

use super::dqn_common::{DqnConfig, SimpleNeuralNetwork, game_state_to_vector};

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

    pub fn save_model<P: AsRef<std::path::Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>> {
        self.model.save(path)
    }

    pub fn get_epsilon(&self) -> f64 {
        self.epsilon
    }

    pub fn get_step_count(&self) -> usize {
        self.step_count
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
        
        // Commencer la partie
        self.game_state.consume(&GameEvent::BeginGame { goes_first: self.agent_player_id });
        
        self.current_step = 0;
        game_state_to_vector(&self.game_state)
    }

    pub fn step(&mut self, action: usize) -> (Vec<f32>, f32, bool) {
        let mut reward = 0.0;
        
        // Appliquer l'action de l'agent
        if self.game_state.active_player_id == self.agent_player_id {
            reward += self.apply_agent_action(action);
        }
        
        // Faire jouer l'adversaire (stratégie simple)
        while self.game_state.active_player_id == self.opponent_player_id 
            && self.game_state.stage != Stage::Ended {
            self.play_opponent_turn();
        }
        
        // Vérifier si la partie est terminée
        let done = self.game_state.stage == Stage::Ended || 
                   self.game_state.determine_winner().is_some() ||
                   self.current_step >= self.max_steps;

        // Récompense finale si la partie est terminée
        if done {
            if let Some(winner) = self.game_state.determine_winner() {
                if winner == self.agent_player_id {
                    reward += 10.0; // Bonus pour gagner
                } else {
                    reward -= 5.0; // Pénalité pour perdre
                }
            }
        }

        self.current_step += 1;
        let next_state = game_state_to_vector(&self.game_state);
        
        (next_state, reward, done)
    }

    fn apply_agent_action(&mut self, action: usize) -> f32 {
        let mut reward = 0.0;
        
        match self.game_state.turn_stage {
            TurnStage::RollDice => {
                // Lancer les dés
                let event = GameEvent::Roll { player_id: self.agent_player_id };
                if self.game_state.validate(&event) {
                    self.game_state.consume(&event);
                    
                    // Simuler le résultat des dés
                    let mut rng = thread_rng();
                    let dice_values = (rng.gen_range(1..=6), rng.gen_range(1..=6));
                    let dice_event = GameEvent::RollResult {
                        player_id: self.agent_player_id,
                        dice: store::Dice { values: dice_values },
                    };
                    if self.game_state.validate(&dice_event) {
                        self.game_state.consume(&dice_event);
                    }
                    reward += 0.1;
                }
            }
            TurnStage::Move => {
                // Choisir un mouvement selon l'action
                let rules = MoveRules::new(&self.agent_color, &self.game_state.board, self.game_state.dice);
                let possible_moves = rules.get_possible_moves_sequences(true, vec![]);
                
                if !possible_moves.is_empty() {
                    let move_index = if action == 0 {
                        0
                    } else if action == 1 && possible_moves.len() > 1 {
                        possible_moves.len() / 2
                    } else {
                        possible_moves.len().saturating_sub(1)
                    };
                    
                    let moves = *possible_moves.get(move_index).unwrap_or(&possible_moves[0]);
                    let event = GameEvent::Move {
                        player_id: self.agent_player_id,
                        moves,
                    };
                    
                    if self.game_state.validate(&event) {
                        self.game_state.consume(&event);
                        reward += 0.2;
                    } else {
                        reward -= 1.0; // Pénalité pour mouvement invalide
                    }
                }
            }
            TurnStage::MarkPoints => {
                // Calculer et marquer les points
                let dice_roll_count = self.game_state.players.get(&self.agent_player_id).unwrap().dice_roll_count;
                let points_rules = PointsRules::new(&self.agent_color, &self.game_state.board, self.game_state.dice);
                let points = points_rules.get_points(dice_roll_count).0;
                
                let event = GameEvent::Mark {
                    player_id: self.agent_player_id,
                    points,
                };
                
                if self.game_state.validate(&event) {
                    self.game_state.consume(&event);
                    reward += 0.1 * points as f32; // Récompense proportionnelle aux points
                }
            }
            TurnStage::HoldOrGoChoice => {
                // Décider de continuer ou pas selon l'action
                if action == 2 { // Action "go"
                    let event = GameEvent::Go { player_id: self.agent_player_id };
                    if self.game_state.validate(&event) {
                        self.game_state.consume(&event);
                        reward += 0.1;
                    }
                } else {
                    // Passer son tour en jouant un mouvement
                    let rules = MoveRules::new(&self.agent_color, &self.game_state.board, self.game_state.dice);
                    let possible_moves = rules.get_possible_moves_sequences(true, vec![]);
                    
                    if !possible_moves.is_empty() {
                        let moves = possible_moves[0];
                        let event = GameEvent::Move {
                            player_id: self.agent_player_id,
                            moves,
                        };
                        
                        if self.game_state.validate(&event) {
                            self.game_state.consume(&event);
                        }
                    }
                }
            }
            _ => {}
        }
        
        reward
    }

    fn play_opponent_turn(&mut self) {
        match self.game_state.turn_stage {
            TurnStage::RollDice => {
                let event = GameEvent::Roll { player_id: self.opponent_player_id };
                if self.game_state.validate(&event) {
                    self.game_state.consume(&event);
                    
                    let mut rng = thread_rng();
                    let dice_values = (rng.gen_range(1..=6), rng.gen_range(1..=6));
                    let dice_event = GameEvent::RollResult {
                        player_id: self.opponent_player_id,
                        dice: store::Dice { values: dice_values },
                    };
                    if self.game_state.validate(&dice_event) {
                        self.game_state.consume(&dice_event);
                    }
                }
            }
            TurnStage::Move => {
                let opponent_color = self.agent_color.opponent_color();
                let rules = MoveRules::new(&opponent_color, &self.game_state.board, self.game_state.dice);
                let possible_moves = rules.get_possible_moves_sequences(true, vec![]);
                
                if !possible_moves.is_empty() {
                    let moves = possible_moves[0]; // Stratégie simple : premier mouvement
                    let event = GameEvent::Move {
                        player_id: self.opponent_player_id,
                        moves,
                    };
                    
                    if self.game_state.validate(&event) {
                        self.game_state.consume(&event);
                    }
                }
            }
            TurnStage::MarkPoints => {
                let opponent_color = self.agent_color.opponent_color();
                let dice_roll_count = self.game_state.players.get(&self.opponent_player_id).unwrap().dice_roll_count;
                let points_rules = PointsRules::new(&opponent_color, &self.game_state.board, self.game_state.dice);
                let points = points_rules.get_points(dice_roll_count).0;
                
                let event = GameEvent::Mark {
                    player_id: self.opponent_player_id,
                    points,
                };
                
                if self.game_state.validate(&event) {
                    self.game_state.consume(&event);
                }
            }
            TurnStage::HoldOrGoChoice => {
                // Stratégie simple : toujours continuer
                let event = GameEvent::Go { player_id: self.opponent_player_id };
                if self.game_state.validate(&event) {
                    self.game_state.consume(&event);
                }
            }
            _ => {}
        }
    }
}

/// Entraîneur pour le modèle DQN
pub struct DqnTrainer {
    agent: DqnAgent,
    env: TrictracEnv,
}

impl DqnTrainer {
    pub fn new(config: DqnConfig) -> Self {
        Self {
            agent: DqnAgent::new(config),
            env: TrictracEnv::new(),
        }
    }

    pub fn train_episode(&mut self) -> f32 {
        let mut total_reward = 0.0;
        let mut state = self.env.reset();
        
        loop {
            let action = self.agent.select_action(&state);
            let (next_state, reward, done) = self.env.step(action);
            total_reward += reward;

            let experience = Experience {
                state: state.clone(),
                action,
                reward,
                next_state: next_state.clone(),
                done,
            };
            self.agent.store_experience(experience);
            self.agent.train();

            if done {
                break;
            }
            state = next_state;
        }

        total_reward
    }

    pub fn train(&mut self, episodes: usize, save_every: usize, model_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        println!("Démarrage de l'entraînement DQN pour {} épisodes", episodes);
        
        for episode in 1..=episodes {
            let reward = self.train_episode();
            
            if episode % 100 == 0 {
                println!(
                    "Épisode {}/{}: Récompense = {:.2}, Epsilon = {:.3}, Steps = {}",
                    episode, episodes, reward, self.agent.get_epsilon(), self.agent.get_step_count()
                );
            }
            
            if episode % save_every == 0 {
                let save_path = format!("{}_episode_{}.json", model_path, episode);
                self.agent.save_model(&save_path)?;
                println!("Modèle sauvegardé : {}", save_path);
            }
        }
        
        // Sauvegarder le modèle final
        let final_path = format!("{}_final.json", model_path);
        self.agent.save_model(&final_path)?;
        println!("Modèle final sauvegardé : {}", final_path);
        
        Ok(())
    }
}