pub mod burn_environment;
pub mod client;
pub mod default;
pub mod dqn;
pub mod dqn_common;
pub mod dqn_trainer;
pub mod erroneous_moves;
pub mod stable_baselines3;

pub mod dummy {
    use store::{Color, Game, PlayerId};
    
    /// Action simple pour l'adversaire dummy
    pub fn get_dummy_action(game: &mut Game, player_id: &PlayerId) -> Result<(), Box<dyn std::error::Error>> {
        let game_state = game.get_state();
        
        match game_state.turn_stage {
            store::TurnStage::RollDice => {
                game.roll_dice_for_player(player_id)?;
            }
            store::TurnStage::MarkPoints | store::TurnStage::MarkAdvPoints => {
                // Marquer 0 points (stratégie conservatrice)
                game.mark_points_for_player(player_id, 0)?;
            }
            store::TurnStage::HoldOrGoChoice => {
                // Toujours choisir "Go" (stratégie simple)
                game.go_for_player(player_id)?;
            }
            store::TurnStage::Move => {
                // Utiliser la logique de mouvement par défaut
                use super::default::DefaultStrategy;
                use crate::BotStrategy;
                
                let mut default_strategy = DefaultStrategy::default();
                default_strategy.set_player_id(*player_id);
                default_strategy.set_color(game_state.player_color_by_id(player_id).unwrap_or(Color::White));
                *default_strategy.get_mut_game() = game_state.clone();
                
                let (move1, move2) = default_strategy.choose_move();
                game.move_checker_for_player(player_id, move1, move2)?;
            }
            _ => {}
        }
        
        Ok(())
    }
}