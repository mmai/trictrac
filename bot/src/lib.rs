pub mod strategy;

use store::{CheckerMove, Color, GameEvent, GameState, PlayerId, PointsRules, Stage, TurnStage};
pub use strategy::burn_dqn_strategy::{BurnDqnStrategy, create_burn_dqn_strategy};
pub use strategy::default::DefaultStrategy;
pub use strategy::dqn::DqnStrategy;
pub use strategy::erroneous_moves::ErroneousStrategy;
pub use strategy::stable_baselines3::StableBaselines3Strategy;

pub trait BotStrategy: std::fmt::Debug {
    fn get_game(&self) -> &GameState;
    fn get_mut_game(&mut self) -> &mut GameState;
    fn calculate_points(&self) -> u8;
    fn calculate_adv_points(&self) -> u8;
    fn choose_move(&self) -> (CheckerMove, CheckerMove);
    fn choose_go(&self) -> bool;
    fn set_player_id(&mut self, player_id: PlayerId);
    fn set_color(&mut self, color: Color);
    fn init_players(&mut self) {
        self.get_mut_game().init_player("p1");
        self.get_mut_game().init_player("p2");
    }
}

#[derive(Debug)]
pub struct Bot {
    pub player_id: PlayerId,
    strategy: Box<dyn BotStrategy>,
    // color: Color,
    // schools_enabled: bool,
}

impl Default for Bot {
    fn default() -> Self {
        let strategy = DefaultStrategy::default();
        Self {
            player_id: 2,
            strategy: Box::new(strategy),
            // color: Color::Black,
            // schools_enabled: false,
        }
    }
}

impl Bot {
    /// new initialize a bot
    // pub fn new(mut strategy: Box<dyn BotStrategy>, color: Color, schools_enabled: bool) -> Self {
    pub fn new(mut strategy: Box<dyn BotStrategy>, color: Color) -> Self {
        // let game = strategy.get_mut_game();
        strategy.init_players();
        let player_id = match color {
            Color::White => 1,
            Color::Black => 2,
        };
        strategy.set_player_id(player_id);
        strategy.set_color(color);
        Self {
            player_id,
            strategy,
            // color,
            // schools_enabled: false,
        }
    }

    pub fn handle_event(&mut self, event: &GameEvent) -> Option<GameEvent> {
        let game = self.strategy.get_mut_game();
        game.consume(event);
        if game.stage == Stage::Ended {
            return None;
        }
        if game.active_player_id == self.player_id {
            return match game.turn_stage {
                TurnStage::MarkAdvPoints => Some(GameEvent::Mark {
                    player_id: self.player_id,
                    points: self.strategy.calculate_adv_points(),
                }),
                TurnStage::RollDice => Some(GameEvent::Roll {
                    player_id: self.player_id,
                }),
                TurnStage::MarkPoints => Some(GameEvent::Mark {
                    player_id: self.player_id,
                    points: self.strategy.calculate_points(),
                }),
                TurnStage::Move => Some(GameEvent::Move {
                    player_id: self.player_id,
                    moves: self.strategy.choose_move(),
                }),
                TurnStage::HoldOrGoChoice => {
                    if self.strategy.choose_go() {
                        Some(GameEvent::Go {
                            player_id: self.player_id,
                        })
                    } else {
                        Some(GameEvent::Move {
                            player_id: self.player_id,
                            moves: self.strategy.choose_move(),
                        })
                    }
                }
                _ => None,
            };
        }
        None
    }

    pub fn get_state(&self) -> &GameState {
        self.strategy.get_game()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use store::{Dice, Stage};

    #[test]
    fn test_new() {
        let bot = Bot::new(Box::new(DefaultStrategy::default()), Color::Black);
        // let bot = Bot::new(Box::new(DefaultStrategy::default()), Color::Black, false);
        assert_eq!(bot.get_state().stage, Stage::PreGame);
    }

    #[test]
    fn test_consume() {
        let mut bot = Bot::new(Box::new(DefaultStrategy::default()), Color::Black);
        // let mut bot = Bot::new(Box::new(DefaultStrategy::default()), Color::Black, false);
        let mut event = bot.handle_event(&GameEvent::BeginGame { goes_first: 2 });
        assert_eq!(event, Some(GameEvent::Roll { player_id: 2 }));
        assert_eq!(bot.get_state().active_player_id, 2);

        event = bot.handle_event(&GameEvent::BeginGame { goes_first: 1 });
        assert_eq!(event, None);

        assert_eq!(bot.get_state().active_player_id, 1);
        bot.handle_event(&GameEvent::RollResult {
            player_id: 1,
            dice: Dice { values: (2, 3) },
        });
        assert_eq!(bot.get_state().turn_stage, TurnStage::Move);
    }
}
