use bot::{Bot, BotStrategy, DefaultStrategy};
use store::{CheckerMove, DiceRoller, GameEvent, GameState, PlayerId, TurnStage};

// Application Game
#[derive(Debug)]
pub struct Game {
    pub state: GameState,
    pub dice_roller: DiceRoller,
    pub first_move: Option<CheckerMove>,
    pub player_id: Option<PlayerId>,
    bot: Bot<DefaultStrategy>,
}

impl Default for Game {
    fn default() -> Self {
        Self {
            state: GameState::default(),
            dice_roller: DiceRoller::default(),
            first_move: None,
            player_id: None,
            bot: Bot::default(),
        }
    }
}

impl Game {
    // Constructs a new instance of [`App`].
    pub fn new(schools_enabled: bool, seed: Option<u64>) -> Self {
        let mut state = GameState::new(schools_enabled);
        // local : player
        let player_id: Option<PlayerId> = state.init_player("myself");
        // bot
        let bot_id: PlayerId = state.init_player("bot").unwrap();
        let bot_color = state.player_color_by_id(&bot_id).unwrap();
        let bot_strategy = DefaultStrategy::default();
        let bot: Bot<DefaultStrategy> = Bot::new(bot_strategy, bot_color, schools_enabled);

        let mut game = Self {
            state,
            dice_roller: DiceRoller::new(seed),
            first_move: None,
            player_id,
            bot,
        };
        game.handle_event(&GameEvent::BeginGame {
            goes_first: player_id.unwrap(),
        });
        game
    }

    pub fn handle_event(&mut self, event: &GameEvent) -> Option<GameEvent> {
        if !self.state.validate(event) {
            return None;
        }
        // println!("consuming {:?}", event);
        self.state.consume(event);
        // chain all successive bot actions
        let bot_event = self
            .bot
            .handle_event(event)
            .and_then(|evt| self.handle_event(&evt));
        // roll dice for bot if needed
        if self.bot_needs_dice_roll() {
            let dice = self.dice_roller.roll();
            self.handle_event(&GameEvent::RollResult {
                player_id: self.bot.player_id,
                dice,
            })
        } else {
            bot_event
        }
    }

    fn bot_needs_dice_roll(&self) -> bool {
        self.state.active_player_id == self.bot.player_id
            && self.state.turn_stage == TurnStage::RollWaiting
    }
}
