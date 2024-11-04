mod bot;

use store::{CheckerMove, Color, GameEvent, GameState, PlayerId, PointsRules, TurnStage};

pub trait BotStrategy: std::fmt::Debug {
    fn get_game(&self) -> &GameState;
    fn get_mut_game(&mut self) -> &mut GameState;
    fn calculate_points(&self) -> u8;
    fn calculate_adv_points(&self) -> u8;
    fn choose_move(&self) -> (CheckerMove, CheckerMove);
    fn set_player_id(&mut self, player_id: PlayerId);
    fn init_players(&mut self) {
        self.get_mut_game().init_player("p1");
        self.get_mut_game().init_player("p2");
    }
}

#[derive(Debug)]
pub struct DefaultStrategy {
    pub game: GameState,
    pub player_id: PlayerId,
    pub color: Color,
}

impl Default for DefaultStrategy {
    fn default() -> Self {
        let game = GameState::default();
        Self {
            game,
            player_id: 2,
            color: Color::Black,
        }
    }
}

impl BotStrategy for DefaultStrategy {
    fn get_game(&self) -> &GameState {
        &self.game
    }
    fn get_mut_game(&mut self) -> &mut GameState {
        &mut self.game
    }

    fn set_player_id(&mut self, player_id: PlayerId) {
        self.player_id = player_id;
    }

    fn calculate_points(&self) -> u8 {
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
        let dice_roll_count = self
            .get_game()
            .players
            .get(&self.player_id)
            .unwrap()
            .dice_roll_count;
        let points_rules = PointsRules::new(&Color::White, &self.game.board, self.game.dice);
        points_rules.get_points(dice_roll_count).0
    }

    fn choose_move(&self) -> (CheckerMove, CheckerMove) {
        let (dice1, dice2) = match self.color {
            Color::White => (self.game.dice.values.0 as i8, self.game.dice.values.1 as i8),
            Color::Black => (
                0 - self.game.dice.values.0 as i8,
                0 - self.game.dice.values.1 as i8,
            ),
        };

        let fields = self.game.board.get_color_fields(self.color);
        let first_field = fields.first().unwrap();
        (
            CheckerMove::new(first_field.0, (first_field.0 as i8 + dice1) as usize).unwrap(),
            CheckerMove::new(first_field.0, (first_field.0 as i8 + dice2) as usize).unwrap(),
        )
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
    /// # Examples
    /// ```let mut bot = Bot::new(Color::Black);
    ///    assert_eq!(bot.game.stage, Stage::PreGame);
    /// ```
    // pub fn new(mut strategy: Box<dyn BotStrategy>, color: Color, schools_enabled: bool) -> Self {
    pub fn new(mut strategy: Box<dyn BotStrategy>, color: Color) -> Self {
        // let game = strategy.get_mut_game();
        strategy.init_players();
        let player_id = match color {
            Color::White => 1,
            Color::Black => 2,
        };
        strategy.set_player_id(player_id);
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
        // println!("bot game {:?}", self.game);
        // println!("bot player_id {:?}", self.player_id);
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
