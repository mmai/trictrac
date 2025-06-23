use burn::{
    backend::{ndarray::NdArrayDevice, Autodiff, NdArray},
    nn::{Linear, LinearConfig, loss::MseLoss},
    module::Module,
    tensor::Tensor,
    optim::{AdamConfig, Optimizer},
    record::{CompactRecorder, Recorder},
};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// Backend utilisé pour l'entraînement (Autodiff + NdArray)
pub type MyBackend = Autodiff<NdArray>;
/// Backend utilisé pour l'inférence (NdArray)
pub type InferenceBackend = NdArray;
pub type MyDevice = NdArrayDevice;

/// Réseau de neurones pour DQN
#[derive(Module, Debug)]
pub struct DqnNetwork<B: burn::prelude::Backend> {
    fc1: Linear<B>,
    fc2: Linear<B>,
    fc3: Linear<B>,
}

impl<B: burn::prelude::Backend> DqnNetwork<B> {
    /// Crée un nouveau réseau DQN
    pub fn new(input_size: usize, hidden_size: usize, output_size: usize, device: &B::Device) -> Self {
        let fc1 = LinearConfig::new(input_size, hidden_size).init(device);
        let fc2 = LinearConfig::new(hidden_size, hidden_size).init(device);
        let fc3 = LinearConfig::new(hidden_size, output_size).init(device);
        
        Self { fc1, fc2, fc3 }
    }

    /// Forward pass du réseau
    pub fn forward(&self, input: Tensor<B, 2>) -> Tensor<B, 2> {
        let x = self.fc1.forward(input);
        let x = burn::tensor::activation::relu(x);
        let x = self.fc2.forward(x);
        let x = burn::tensor::activation::relu(x);
        self.fc3.forward(x)
    }
}

/// Configuration pour l'entraînement DQN
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DqnConfig {
    pub state_size: usize,
    pub action_size: usize,
    pub hidden_size: usize,
    pub learning_rate: f64,
    pub gamma: f32,
    pub epsilon: f32,
    pub epsilon_decay: f32,
    pub epsilon_min: f32,
    pub replay_buffer_size: usize,
    pub batch_size: usize,
    pub target_update_freq: usize,
}

impl Default for DqnConfig {
    fn default() -> Self {
        Self {
            state_size: 36,
            action_size: 1000,
            hidden_size: 256,
            learning_rate: 0.001,
            gamma: 0.99,
            epsilon: 1.0,
            epsilon_decay: 0.995,
            epsilon_min: 0.01,
            replay_buffer_size: 10000,
            batch_size: 32,
            target_update_freq: 100,
        }
    }
}

/// Experience pour le replay buffer
#[derive(Debug, Clone)]
pub struct Experience {
    pub state: Vec<f32>,
    pub action: usize,
    pub reward: f32,
    pub next_state: Option<Vec<f32>>,
    pub done: bool,
}

/// Agent DQN utilisant Burn
pub struct BurnDqnAgent {
    config: DqnConfig,
    device: MyDevice,
    q_network: DqnNetwork<MyBackend>,
    target_network: DqnNetwork<MyBackend>,
    optimizer: burn::optim::Adam<MyBackend>,
    replay_buffer: VecDeque<Experience>,
    epsilon: f32,
    step_count: usize,
}

impl BurnDqnAgent {
    /// Crée un nouvel agent DQN
    pub fn new(config: DqnConfig) -> Self {
        let device = MyDevice::default();
        
        let q_network = DqnNetwork::new(
            config.state_size,
            config.hidden_size,
            config.action_size,
            &device,
        );
        
        let target_network = DqnNetwork::new(
            config.state_size,
            config.hidden_size,
            config.action_size,
            &device,
        );
        
        let optimizer = AdamConfig::new().init();

        Self {
            config: config.clone(),
            device,
            q_network,
            target_network,
            optimizer,
            replay_buffer: VecDeque::new(),
            epsilon: config.epsilon,
            step_count: 0,
        }
    }

    /// Sélectionne une action avec epsilon-greedy
    pub fn select_action(&mut self, state: &[f32], valid_actions: &[usize]) -> usize {
        if valid_actions.is_empty() {
            return 0;
        }

        // Exploration epsilon-greedy
        if rand::random::<f32>() < self.epsilon {
            let random_index = rand::random::<usize>() % valid_actions.len();
            return valid_actions[random_index];
        }

        // Exploitation : choisir la meilleure action selon le Q-network
        let state_tensor = Tensor::<MyBackend, 2>::from_floats([state], &self.device);
        let q_values = self.q_network.forward(state_tensor);
        
        // Convertir en vecteur pour traitement
        let q_data = q_values.into_data().convert::<f32>().value;
        
        // Trouver la meilleure action parmi les actions valides
        let mut best_action = valid_actions[0];
        let mut best_q_value = f32::NEG_INFINITY;
        
        for &action in valid_actions {
            if action < q_data.len() && q_data[action] > best_q_value {
                best_q_value = q_data[action];
                best_action = action;
            }
        }
        
        best_action
    }

    /// Ajoute une expérience au replay buffer
    pub fn add_experience(&mut self, experience: Experience) {
        if self.replay_buffer.len() >= self.config.replay_buffer_size {
            self.replay_buffer.pop_front();
        }
        self.replay_buffer.push_back(experience);
    }

    /// Entraîne le réseau sur un batch d'expériences
    pub fn train_step(&mut self) -> Option<f32> {
        if self.replay_buffer.len() < self.config.batch_size {
            return None;
        }

        // Échantillonner un batch d'expériences
        let batch = self.sample_batch();
        
        // Préparer les tenseurs d'état
        let states: Vec<&[f32]> = batch.iter().map(|exp| exp.state.as_slice()).collect();
        let state_tensor = Tensor::<MyBackend, 2>::from_floats(states, &self.device);
        
        // Calculer les Q-values actuelles
        let current_q_values = self.q_network.forward(state_tensor);
        
        // Pour l'instant, version simplifiée sans calcul de target
        let target_q_values = current_q_values.clone();
        
        // Calculer la loss MSE
        let loss = MseLoss::new().forward(
            current_q_values, 
            target_q_values, 
            burn::nn::loss::Reduction::Mean
        );
        
        // Backpropagation (version simplifiée)
        let grads = loss.backward();
        self.q_network = self.optimizer.step(self.config.learning_rate, self.q_network, grads);
        
        // Mise à jour du réseau cible
        self.step_count += 1;
        if self.step_count % self.config.target_update_freq == 0 {
            self.update_target_network();
        }
        
        // Décroissance d'epsilon
        if self.epsilon > self.config.epsilon_min {
            self.epsilon *= self.config.epsilon_decay;
        }
        
        Some(loss.into_scalar())
    }

    /// Échantillonne un batch d'expériences du replay buffer
    fn sample_batch(&self) -> Vec<Experience> {
        let mut batch = Vec::new();
        let buffer_size = self.replay_buffer.len();
        
        for _ in 0..self.config.batch_size.min(buffer_size) {
            let index = rand::random::<usize>() % buffer_size;
            if let Some(exp) = self.replay_buffer.get(index) {
                batch.push(exp.clone());
            }
        }
        
        batch
    }

    /// Met à jour le réseau cible avec les poids du réseau principal
    fn update_target_network(&mut self) {
        // Copie simple des poids
        self.target_network = self.q_network.clone();
    }

    /// Sauvegarde le modèle
    pub fn save_model(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Sauvegarder la configuration
        let config_path = format!("{}_config.json", path);
        let config_json = serde_json::to_string_pretty(&self.config)?;
        std::fs::write(config_path, config_json)?;
        
        // Sauvegarder le réseau pour l'inférence (conversion vers NdArray backend)
        let inference_network = self.q_network.clone().into_record();
        let recorder = CompactRecorder::new();
        
        let model_path = format!("{}_model.burn", path);
        recorder.record(inference_network, model_path.into())?;
        
        println!("Modèle sauvegardé : {}", path);
        Ok(())
    }

    /// Charge un modèle pour l'inférence
    pub fn load_model_for_inference(path: &str) -> Result<(DqnNetwork<InferenceBackend>, DqnConfig), Box<dyn std::error::Error>> {
        // Charger la configuration
        let config_path = format!("{}_config.json", path);
        let config_json = std::fs::read_to_string(config_path)?;
        let config: DqnConfig = serde_json::from_str(&config_json)?;
        
        // Créer le réseau pour l'inférence
        let device = NdArrayDevice::default();
        let network = DqnNetwork::<InferenceBackend>::new(
            config.state_size,
            config.hidden_size,
            config.action_size,
            &device,
        );
        
        // Charger les poids
        let model_path = format!("{}_model.burn", path);
        let recorder = CompactRecorder::new();
        let record = recorder.load(model_path.into(), &device)?;
        let network = network.load_record(record);
        
        Ok((network, config))
    }

    /// Retourne l'epsilon actuel
    pub fn get_epsilon(&self) -> f32 {
        self.epsilon
    }

    /// Retourne la taille du replay buffer
    pub fn get_buffer_size(&self) -> usize {
        self.replay_buffer.len()
    }
}