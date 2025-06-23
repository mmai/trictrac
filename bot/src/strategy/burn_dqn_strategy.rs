use crate::{BotStrategy, CheckerMove, Color, GameState, PlayerId};
use super::burn_dqn_agent::{DqnNetwork, DqnConfig, InferenceBackend};
use super::dqn_common::get_valid_actions;
use burn::{backend::ndarray::NdArrayDevice, tensor::Tensor};
use std::path::Path;

/// Stratégie utilisant un modèle DQN Burn entraîné
#[derive(Debug)]
pub struct BurnDqnStrategy {
    pub game: GameState,
    pub player_id: PlayerId,
    pub color: Color,
    network: Option<DqnNetwork<InferenceBackend>>,
    config: Option<DqnConfig>,
    device: NdArrayDevice,
}

impl Default for BurnDqnStrategy {
    fn default() -> Self {
        Self {
            game: GameState::default(),
            player_id: 0,
            color: Color::White,
            network: None,
            config: None,
            device: NdArrayDevice::default(),
        }
    }
}

impl BurnDqnStrategy {
    /// Crée une nouvelle stratégie avec un modèle chargé
    pub fn new(model_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let mut strategy = Self::default();
        strategy.load_model(model_path)?;
        Ok(strategy)
    }

    /// Charge un modèle DQN depuis un fichier
    pub fn load_model(&mut self, model_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        if !Path::new(&format!("{}_config.json", model_path)).exists() {
            return Err(format!("Modèle non trouvé : {}", model_path).into());
        }

        let (network, config) = super::burn_dqn_agent::BurnDqnAgent::load_model_for_inference(model_path)?;
        
        self.network = Some(network);
        self.config = Some(config);
        
        println!("Modèle DQN Burn chargé depuis : {}", model_path);
        Ok(())
    }

    /// Sélectionne la meilleure action selon le modèle DQN
    fn select_best_action(&self, valid_actions: &[super::dqn_common::TrictracAction]) -> Option<super::dqn_common::TrictracAction> {
        if valid_actions.is_empty() {
            return None;
        }

        // Si pas de réseau chargé, utiliser la première action valide
        let Some(network) = &self.network else {
            return Some(valid_actions[0].clone());
        };

        // Convertir l'état du jeu en tensor
        let state_vec = self.game.to_vec_float();
        let state_tensor = Tensor::<InferenceBackend, 2>::from_floats([state_vec], &self.device);

        // Faire une prédiction
        let q_values = network.forward(state_tensor);
        let q_data = q_values.into_data().convert::<f32>().value;

        // Trouver la meilleure action parmi les actions valides
        let mut best_action = &valid_actions[0];
        let mut best_q_value = f32::NEG_INFINITY;

        for (i, action) in valid_actions.iter().enumerate() {
            if i < q_data.len() && q_data[i] > best_q_value {
                best_q_value = q_data[i];
                best_action = action;
            }
        }

        Some(best_action.clone())
    }

    /// Convertit une TrictracAction en CheckerMove pour les mouvements
    fn trictrac_action_to_moves(&self, action: &super::dqn_common::TrictracAction) -> Option<(CheckerMove, CheckerMove)> {
        match action {
            super::dqn_common::TrictracAction::Move { dice_order, from1, from2 } => {
                let dice = self.game.dice;
                let (die1, die2) = if *dice_order { 
                    (dice.values.0, dice.values.1) 
                } else { 
                    (dice.values.1, dice.values.0) 
                };

                // Calculer les destinations selon la couleur
                let to1 = if self.color == Color::White {
                    from1 + die1 as usize
                } else {
                    from1.saturating_sub(die1 as usize)
                };
                let to2 = if self.color == Color::White {
                    from2 + die2 as usize
                } else {
                    from2.saturating_sub(die2 as usize)
                };

                // Créer les mouvements
                let move1 = CheckerMove::new(*from1, to1).ok()?;
                let move2 = CheckerMove::new(*from2, to2).ok()?;
                
                Some((move1, move2))
            }
            _ => None,
        }
    }
}

impl BotStrategy for BurnDqnStrategy {
    fn get_game(&self) -> &GameState {
        &self.game
    }

    fn get_mut_game(&mut self) -> &mut GameState {
        &mut self.game
    }

    fn calculate_points(&self) -> u8 {
        // Utiliser le modèle DQN pour décider des points à marquer
        let valid_actions = get_valid_actions(&self.game);
        
        // Chercher une action Mark dans les actions valides
        for action in &valid_actions {
            if let super::dqn_common::TrictracAction::Mark { points } = action {
                return *points;
            }
        }
        
        // Par défaut, marquer 0 points
        0
    }

    fn calculate_adv_points(&self) -> u8 {
        // Même logique que calculate_points pour les points d'avance
        self.calculate_points()
    }

    fn choose_move(&self) -> (CheckerMove, CheckerMove) {
        let valid_actions = get_valid_actions(&self.game);
        
        if let Some(best_action) = self.select_best_action(&valid_actions) {
            if let Some((move1, move2)) = self.trictrac_action_to_moves(&best_action) {
                return (move1, move2);
            }
        }

        // Fallback: utiliser la stratégie par défaut
        let default_strategy = super::default::DefaultStrategy::default();
        default_strategy.choose_move()
    }

    fn choose_go(&self) -> bool {
        let valid_actions = get_valid_actions(&self.game);
        
        if let Some(best_action) = self.select_best_action(&valid_actions) {
            match best_action {
                super::dqn_common::TrictracAction::Go => return true,
                super::dqn_common::TrictracAction::Move { .. } => return false,
                _ => {}
            }
        }

        // Par défaut, toujours choisir de continuer
        true
    }

    fn set_player_id(&mut self, player_id: PlayerId) {
        self.player_id = player_id;
    }

    fn set_color(&mut self, color: Color) {
        self.color = color;
    }
}

/// Factory function pour créer une stratégie DQN Burn depuis un chemin de modèle
pub fn create_burn_dqn_strategy(model_path: &str) -> Result<Box<dyn BotStrategy>, Box<dyn std::error::Error>> {
    let strategy = BurnDqnStrategy::new(model_path)?;
    Ok(Box::new(strategy))
}