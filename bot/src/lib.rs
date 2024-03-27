mod bot;

use store::{CheckerMove, Color, Dice, GameEvent, GameState, Player, PlayerId, Stage, TurnStage};

#[derive(Debug)]
pub struct Bot {
    pub game: GameState,
    pub player_id: PlayerId,
    color: Color,
}

impl Default for Bot {
    fn default() -> Bot {
        Bot {
            game: GameState::default(),
            player_id: 1,
            color: Color::Black,
        }
    }
}

// impl PlayerEngine for Bot {}

impl Bot {
    /// new initialize a bot
    /// # Examples
    /// ```let mut bot = Bot::new(Color::Black);
    ///    assert_eq!(bot.game.stage, Stage::PreGame);
    /// ```
    pub fn new(color: Color) -> Self {
        let mut game = GameState::default();
        game.init_player("p1");
        game.init_player("p2");

        let player_id = match color {
            Color::White => 1,
            Color::Black => 2,
        };

        Self {
            game,
            player_id,
            color,
        }
    }

    pub fn consume(&mut self, event: &GameEvent) -> Option<GameEvent> {
        self.game.consume(event);
        println!("bot game {:?}", self.game);
        println!("bot player_id {:?}", self.player_id);
        if self.game.active_player_id == self.player_id {
            return match self.game.turn_stage {
                TurnStage::RollDice => Some(GameEvent::Roll {
                    player_id: self.player_id,
                }),
                TurnStage::MarkPoints => Some(GameEvent::Mark {
                    player_id: self.player_id,
                    points: 0,
                }),
                TurnStage::Move => Some(GameEvent::Move {
                    player_id: self.player_id,
                    moves: self.choose_move(),
                }),
                _ => None,
            };
        }
        None
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let bot = Bot::new(Color::Black);
        assert_eq!(bot.game.stage, Stage::PreGame);
    }

    #[test]
    fn test_consume() {
        let mut bot = Bot::new(Color::Black);
        let mut event = bot.consume(&GameEvent::BeginGame { goes_first: 2 });
        assert_eq!(event, Some(GameEvent::Roll { player_id: 2 }));

        event = bot.consume(&GameEvent::BeginGame { goes_first: 1 });
        assert_eq!(event, None);

        event = bot.consume(&GameEvent::RollResult {
            player_id: 2,
            dice: Dice { values: (2, 3) },
        });
        assert_eq!(bot.game.turn_stage, TurnStage::MarkPoints);
    }
}