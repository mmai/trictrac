use crate::{BotStrategy, CheckerMove, Color, GameState, PlayerId, PointsRules};
use store::MoveRules;
use std::process::Command;
use std::io::Write;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use serde::{Serialize, Deserialize};

#[derive(Debug)]
pub struct StableBaselines3Strategy {
    pub game: GameState,
    pub player_id: PlayerId,
    pub color: Color,
    pub model_path: String,
}

impl Default for StableBaselines3Strategy {
    fn default() -> Self {
        let game = GameState::default();
        Self {
            game,
            player_id: 2,
            color: Color::Black,
            model_path: "models/trictrac_ppo.zip".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct GameStateJson {
    board: Vec<i8>,
    active_player: u8,
    dice: [u8; 2],
    white_points: u8,
    white_holes: u8,
    black_points: u8,
    black_holes: u8,
    turn_stage: u8,
}

#[derive(Deserialize)]
struct ActionJson {
    action_type: u8,
    from1: usize,
    to1: usize,
    from2: usize,
    to2: usize,
}

impl StableBaselines3Strategy {
    pub fn new(model_path: &str) -> Self {
        let game = GameState::default();
        Self {
            game,
            player_id: 2,
            color: Color::Black,
            model_path: model_path.to_string(),
        }
    }

    fn get_state_as_json(&self) -> GameStateJson {
        // Convertir l'état du jeu en un format compatible avec notre modèle Python
        let mut board = vec![0; 24];
        
        // Remplir les positions des pièces blanches (valeurs positives)
        for (pos, count) in self.game.board.get_color_fields(Color::White) {
            if pos < 24 {
                board[pos] = count as i8;
            }
        }
        
        // Remplir les positions des pièces noires (valeurs négatives)
        for (pos, count) in self.game.board.get_color_fields(Color::Black) {
            if pos < 24 {
                board[pos] = -(count as i8);
            }
        }
        
        // Convertir l'étape du tour en entier
        let turn_stage = match self.game.turn_stage {
            store::TurnStage::RollDice => 0,
            store::TurnStage::RollWaiting => 1,
            store::TurnStage::MarkPoints => 2,
            store::TurnStage::HoldOrGoChoice => 3,
            store::TurnStage::Move => 4,
            store::TurnStage::MarkAdvPoints => 5,
            _ => 0,
        };
        
        // Récupérer les points et trous des joueurs
        let white_points = self.game.players.get(&1).map_or(0, |p| p.points);
        let white_holes = self.game.players.get(&1).map_or(0, |p| p.holes);
        let black_points = self.game.players.get(&2).map_or(0, |p| p.points);
        let black_holes = self.game.players.get(&2).map_or(0, |p| p.holes);
        
        // Créer l'objet JSON
        GameStateJson {
            board,
            active_player: self.game.active_player_id as u8,
            dice: [self.game.dice.values.0, self.game.dice.values.1],
            white_points,
            white_holes,
            black_points,
            black_holes,
            turn_stage,
        }
    }

    fn predict_action(&self) -> Option<ActionJson> {
        // Convertir l'état du jeu en JSON
        let state_json = self.get_state_as_json();
        let state_str = serde_json::to_string(&state_json).unwrap();
        
        // Écrire l'état dans un fichier temporaire
        let temp_input_path = "temp_state.json";
        let mut file = File::create(temp_input_path).ok()?;
        file.write_all(state_str.as_bytes()).ok()?;
        
        // Exécuter le script Python pour faire une prédiction
        let output_path = "temp_action.json";
        let python_script = format!(
            r#"
import sys
import json
import numpy as np
from stable_baselines3 import PPO
import torch

# Charger le modèle
model = PPO.load("{}")

# Lire l'état du jeu
with open("temp_state.json", "r") as f:
    state_dict = json.load(f)

# Convertir en format d'observation attendu par le modèle
observation = {{
    'board': np.array(state_dict['board'], dtype=np.int8),
    'active_player': state_dict['active_player'],
    'dice': np.array(state_dict['dice'], dtype=np.int32),
    'white_points': state_dict['white_points'],
    'white_holes': state_dict['white_holes'],
    'black_points': state_dict['black_points'],
    'black_holes': state_dict['black_holes'],
    'turn_stage': state_dict['turn_stage'],
}}

# Prédire l'action
action, _ = model.predict(observation)

# Convertir l'action en format lisible
action_dict = {{
    'action_type': int(action[0]),
    'from1': int(action[1]),
    'to1': int(action[2]),
    'from2': int(action[3]),
    'to2': int(action[4]),
}}

# Écrire l'action dans un fichier
with open("{}", "w") as f:
    json.dump(action_dict, f)
"#,
            self.model_path, output_path
        );
        
        let temp_script_path = "temp_predict.py";
        let mut script_file = File::create(temp_script_path).ok()?;
        script_file.write_all(python_script.as_bytes()).ok()?;
        
        // Exécuter le script Python
        let status = Command::new("python")
            .arg(temp_script_path)
            .status()
            .ok()?;
            
        if !status.success() {
            return None;
        }
        
        // Lire la prédiction
        if Path::new(output_path).exists() {
            let mut file = File::open(output_path).ok()?;
            let mut contents = String::new();
            file.read_to_string(&mut contents).ok()?;
            
            // Nettoyer les fichiers temporaires
            std::fs::remove_file(temp_input_path).ok();
            std::fs::remove_file(temp_script_path).ok();
            std::fs::remove_file(output_path).ok();
            
            // Analyser la prédiction
            let action: ActionJson = serde_json::from_str(&contents).ok()?;
            Some(action)
        } else {
            None
        }
    }
}

impl BotStrategy for StableBaselines3Strategy {
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
        // Utiliser la prédiction du modèle uniquement si c'est une action de type "mark" (1)
        if let Some(action) = self.predict_action() {
            if action.action_type == 1 {
                // Marquer les points calculés par le modèle (ici on utilise la somme des dés comme proxy)
                return self.game.dice.values.0 + self.game.dice.values.1;
            }
        }
        
        // Fallback vers la méthode standard si la prédiction échoue
        let dice_roll_count = self
            .get_game()
            .players
            .get(&self.player_id)
            .unwrap()
            .dice_roll_count;
        let points_rules = PointsRules::new(&Color::White, &self.game.board, self.game.dice);
        points_rules.get_points(dice_roll_count).0
    }

    fn calculate_adv_points(&self) -> u8 {
        self.calculate_points()
    }

    fn choose_go(&self) -> bool {
        // Utiliser la prédiction du modèle uniquement si c'est une action de type "go" (2)
        if let Some(action) = self.predict_action() {
            return action.action_type == 2;
        }
        
        // Fallback vers la méthode standard si la prédiction échoue
        true
    }

    fn choose_move(&self) -> (CheckerMove, CheckerMove) {
        // Utiliser la prédiction du modèle uniquement si c'est une action de type "move" (0)
        if let Some(action) = self.predict_action() {
            if action.action_type == 0 {
                let move1 = CheckerMove::new(action.from1, action.to1).unwrap_or_default();
                let move2 = CheckerMove::new(action.from2, action.to2).unwrap_or_default();
                return (move1, move2);
            }
        }
        
        // Fallback vers la méthode standard si la prédiction échoue
        let rules = MoveRules::new(&self.color, &self.game.board, self.game.dice);
        let possible_moves = rules.get_possible_moves_sequences(true, vec![]);
        let choosen_move = *possible_moves
            .first()
            .unwrap_or(&(CheckerMove::default(), CheckerMove::default()));
        
        if self.color == Color::White {
            choosen_move
        } else {
            (choosen_move.0.mirror(), choosen_move.1.mirror())
        }
    }
}