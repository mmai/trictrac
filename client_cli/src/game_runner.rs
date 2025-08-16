use bot::{Bot, BotStrategy};
use log::{debug, error};
use store::{CheckerMove, DiceRoller, GameEvent, GameState, PlayerId, TurnStage};

// Application Game
#[derive(Debug, Default)]
pub struct GameRunner {
    pub state: GameState,
    pub dice_roller: DiceRoller,
    pub first_move: Option<CheckerMove>,
    pub player_id: Option<PlayerId>,
    bots: Vec<Bot>,
}

impl GameRunner {
    // Constructs a new instance of [`App`].
    pub fn new(
        schools_enabled: bool,
        bot_strategies: Vec<Box<dyn BotStrategy>>,
        seed: Option<u64>,
    ) -> Self {
        let mut state = GameState::new(schools_enabled);
        // local : player
        let player_id: Option<PlayerId> = if bot_strategies.len() > 1 {
            None
        } else {
            state.init_player("myself")
        };

        // bots
        let bots: Vec<Bot> = bot_strategies
            .into_iter()
            .map(|strategy| {
                let bot_id: PlayerId = state.init_player("bot").unwrap();
                let bot_color = state.player_color_by_id(&bot_id).unwrap();
                Bot::new(strategy, bot_color)
            })
            .collect();
        // let bot_strategy = Box::new(DefaultStrategy::default());
        // let bot: Bot = Bot::new(bot_strategy, bot_color, schools_enabled);
        // let bot: Bot = Bot::new(bot_strategy, bot_color);

        let first_player_id = if bots.len() > 1 {
            bots[0].player_id
        } else {
            player_id.unwrap()
        };
        let mut game = Self {
            state,
            dice_roller: DiceRoller::new(seed),
            first_move: None,
            player_id,
            bots,
        };
        game.handle_event(&GameEvent::BeginGame {
            goes_first: first_player_id,
        });
        game
    }

    pub fn handle_event(&mut self, event: &GameEvent) -> Option<GameEvent> {
        if event == &GameEvent::PlayError {
            return None;
        }
        let valid_event = if self.state.validate(event) {
            debug!(
                "--------------- new valid event {event:?} (stage {:?}) -----------",
                self.state.turn_stage
            );
            self.state.consume(event);
            debug!(
                " --> stage {:?} ; active player points {:?}",
                self.state.turn_stage,
                self.state.who_plays().map(|p| p.points)
            );
            event
        } else {
            debug!("{}", self.state);
            error!("event not valid : {event:?}");
            panic!("crash and burn {} \nevt not valid {event:?}", self.state);
            &GameEvent::PlayError
        };

        // chain all successive bot actions
        if self.bots.is_empty() {
            return None;
        }

        // Collect bot actions to avoid borrow conflicts
        let bot_events: Vec<GameEvent> = self
            .bots
            .iter_mut()
            .filter_map(|bot| bot.handle_event(valid_event))
            .collect();

        // if bot_events.len() > 1 {
        //     println!(
        //         "There might be a problem : 2 bots events : {:?}",
        //         bot_events
        //     );
        // }

        let mut next_event = None;
        for bot_event in bot_events {
            let bot_result_event = self.handle_event(&bot_event);
            if let Some(bot_id) = bot_event.player_id() {
                next_event = if self.bot_needs_dice_roll(bot_id) {
                    let dice = self.dice_roller.roll();
                    self.handle_event(&GameEvent::RollResult {
                        player_id: bot_id,
                        dice,
                    })
                } else {
                    bot_result_event
                };
            }
        }

        if let Some(winner) = self.state.determine_winner() {
            next_event = Some(store::GameEvent::EndGame {
                reason: store::EndGameReason::PlayerWon { winner },
            });
        }

        next_event
    }

    fn bot_needs_dice_roll(&self, bot_id: PlayerId) -> bool {
        self.state.active_player_id == bot_id && self.state.turn_stage == TurnStage::RollWaiting
    }
}
