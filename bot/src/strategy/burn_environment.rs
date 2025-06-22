use burn::{backend::Backend, tensor::Tensor};
use burn_rl::base::{Action, Environment, Snapshot, State};
use crate::GameState;
use store::{Color, Game, PlayerId};
use std::collections::HashMap;

/// État du jeu Trictrac pour burn-rl
#[derive(Debug, Clone, Copy)]
pub struct TrictracState {
    pub data: [f32; 36], // Représentation vectorielle de l'état du jeu
}

impl State for TrictracState {
    type Data = [f32; 36];

    fn to_tensor<B: Backend>(&self) -> Tensor<B, 1> {
        Tensor::from_floats(self.data, &B::Device::default())
    }

    fn size() -> usize {
        36
    }
}

impl TrictracState {
    /// Convertit un GameState en TrictracState
    pub fn from_game_state(game_state: &GameState) -> Self {
        let state_vec = game_state.to_vec();
        let mut data = [0.0f32; 36];
        
        // Copier les données en s'assurant qu'on ne dépasse pas la taille
        let copy_len = state_vec.len().min(36);
        for i in 0..copy_len {
            data[i] = state_vec[i];
        }
        
        TrictracState { data }
    }
}

/// Actions possibles dans Trictrac pour burn-rl
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrictracAction {
    pub index: u32,
}

impl Action for TrictracAction {
    fn random() -> Self {
        use rand::{thread_rng, Rng};
        let mut rng = thread_rng();
        TrictracAction {
            index: rng.gen_range(0..Self::size() as u32),
        }
    }

    fn enumerate() -> Vec<Self> {
        (0..Self::size() as u32)
            .map(|index| TrictracAction { index })
            .collect()
    }

    fn size() -> usize {
        // Utiliser l'espace d'actions compactes pour réduire la complexité
        // Maximum estimé basé sur les actions contextuelles
        1000 // Estimation conservative, sera ajusté dynamiquement
    }
}

impl From<u32> for TrictracAction {
    fn from(index: u32) -> Self {
        TrictracAction { index }
    }
}

impl From<TrictracAction> for u32 {
    fn from(action: TrictracAction) -> u32 {
        action.index
    }
}

/// Environnement Trictrac pour burn-rl
#[derive(Debug)]
pub struct TrictracEnvironment {
    game: Game,
    active_player_id: PlayerId, 
    opponent_id: PlayerId,
    current_state: TrictracState,
    episode_reward: f32,
    step_count: usize,
    visualized: bool,
}

impl Environment for TrictracEnvironment {
    type StateType = TrictracState;
    type ActionType = TrictracAction;
    type RewardType = f32;

    const MAX_STEPS: usize = 1000; // Limite max pour éviter les parties infinies

    fn new(visualized: bool) -> Self {
        let mut game = Game::new();
        
        // Ajouter deux joueurs
        let player1_id = game.add_player("DQN Agent".to_string(), Color::White);
        let player2_id = game.add_player("Opponent".to_string(), Color::Black);
        
        game.start();
        
        let game_state = game.get_state();
        let current_state = TrictracState::from_game_state(&game_state);
        
        TrictracEnvironment {
            game,
            active_player_id: player1_id,
            opponent_id: player2_id,
            current_state,
            episode_reward: 0.0,
            step_count: 0,
            visualized,
        }
    }

    fn state(&self) -> Self::StateType {
        self.current_state
    }

    fn reset(&mut self) -> Snapshot<Self> {
        // Réinitialiser le jeu
        self.game = Game::new();
        self.active_player_id = self.game.add_player("DQN Agent".to_string(), Color::White);
        self.opponent_id = self.game.add_player("Opponent".to_string(), Color::Black);
        self.game.start();
        
        let game_state = self.game.get_state();
        self.current_state = TrictracState::from_game_state(&game_state);
        self.episode_reward = 0.0;
        self.step_count = 0;

        Snapshot {
            state: self.current_state,
            reward: 0.0,
            terminated: false,
        }
    }

    fn step(&mut self, action: Self::ActionType) -> Snapshot<Self> {
        self.step_count += 1;
        
        let game_state = self.game.get_state();
        
        // Convertir l'action burn-rl vers une action Trictrac
        let trictrac_action = self.convert_action(action, &game_state);
        
        let mut reward = 0.0;
        let mut terminated = false;
        
        // Exécuter l'action si c'est le tour de l'agent DQN
        if game_state.active_player_id == self.active_player_id {
            if let Some(action) = trictrac_action {
                match self.execute_action(action) {
                    Ok(action_reward) => {
                        reward = action_reward;
                    }
                    Err(_) => {
                        // Action invalide, pénalité
                        reward = -1.0;
                    }
                }
            } else {
                // Action non convertible, pénalité
                reward = -0.5;
            }
        }
        
        // Jouer l'adversaire si c'est son tour
        self.play_opponent_if_needed();
        
        // Vérifier fin de partie
        let updated_state = self.game.get_state();
        if updated_state.is_finished() || self.step_count >= Self::MAX_STEPS {
            terminated = true;
            
            // Récompense finale basée sur le résultat
            if let Some(winner_id) = updated_state.winner {
                if winner_id == self.active_player_id {
                    reward += 10.0; // Victoire
                } else {
                    reward -= 10.0; // Défaite
                }
            }
        }
        
        // Mettre à jour l'état
        self.current_state = TrictracState::from_game_state(&updated_state);
        self.episode_reward += reward;
        
        if self.visualized && terminated {
            println!("Episode terminé. Récompense totale: {:.2}, Étapes: {}", 
                     self.episode_reward, self.step_count);
        }

        Snapshot {
            state: self.current_state,
            reward,
            terminated,
        }
    }
}

impl TrictracEnvironment {
    /// Convertit une action burn-rl vers une action Trictrac
    fn convert_action(&self, action: TrictracAction, game_state: &GameState) -> Option<super::dqn_common::TrictracAction> {
        use super::dqn_common::{get_valid_compact_actions, CompactAction};
        
        // Obtenir les actions valides dans le contexte actuel
        let valid_actions = get_valid_compact_actions(game_state);
        
        if valid_actions.is_empty() {
            return None;
        }
        
        // Mapper l'index d'action sur une action valide
        let action_index = (action.index as usize) % valid_actions.len();
        let compact_action = &valid_actions[action_index];
        
        // Convertir l'action compacte vers une action Trictrac complète
        compact_action.to_trictrac_action(game_state)
    }
    
    /// Exécute une action Trictrac dans le jeu
    fn execute_action(&mut self, action: super::dqn_common::TrictracAction) -> Result<f32, Box<dyn std::error::Error>> {
        use super::dqn_common::TrictracAction;
        
        let mut reward = 0.0;
        
        match action {
            TrictracAction::Roll => {
                self.game.roll_dice_for_player(&self.active_player_id)?;
                reward = 0.1; // Petite récompense pour une action valide
            }
            TrictracAction::Mark { points } => {
                self.game.mark_points_for_player(&self.active_player_id, points)?;
                reward = points as f32 * 0.1; // Récompense proportionnelle aux points
            }
            TrictracAction::Go => {
                self.game.go_for_player(&self.active_player_id)?;
                reward = 0.2; // Récompense pour continuer
            }
            TrictracAction::Move { move1, move2 } => {
                let checker_move1 = store::CheckerMove::new(move1.0, move1.1)?;
                let checker_move2 = store::CheckerMove::new(move2.0, move2.1)?;
                self.game.move_checker_for_player(&self.active_player_id, checker_move1, checker_move2)?;
                reward = 0.3; // Récompense pour un mouvement réussi
            }
        }
        
        Ok(reward)
    }
    
    /// Fait jouer l'adversaire avec une stratégie simple
    fn play_opponent_if_needed(&mut self) {
        let game_state = self.game.get_state();
        
        // Si c'est le tour de l'adversaire, jouer automatiquement
        if game_state.active_player_id == self.opponent_id && !game_state.is_finished() {
            // Utiliser une stratégie simple pour l'adversaire (dummy bot)
            if let Ok(_) = crate::strategy::dummy::get_dummy_action(&mut self.game, &self.opponent_id) {
                // L'action a été exécutée par get_dummy_action
            }
        }
    }
}