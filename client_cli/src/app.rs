use itertools::Itertools;

use bot::Bot;
use pretty_assertions::assert_eq;
use store::{CheckerMove, DiceRoller, GameEvent, GameState, PlayerId, Stage, TurnStage};

#[derive(Debug, Default)]
pub struct AppArgs {
    pub seed: Option<u32>,
}

// Application Game
#[derive(Debug, Default)]
pub struct Game {
    pub state: GameState,
    pub dice_roller: DiceRoller,
    first_move: Option<CheckerMove>,
    player_id: Option<PlayerId>,
    bot: Bot,
}

impl Game {
    // Constructs a new instance of [`App`].
    pub fn new(seed: Option<u64>) -> Self {
        let mut state = GameState::default();
        // local : player
        let player_id: Option<PlayerId> = state.init_player("myself");
        // bot
        let bot_id: PlayerId = state.init_player("bot").unwrap();
        let bot_color = state.player_color_by_id(&bot_id).unwrap();
        let bot: Bot = Bot::new(bot_color);

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

// Application.
#[derive(Debug, Default)]
pub struct App {
    // should the application exit?
    pub should_quit: bool,
    pub game: Game,
}

impl App {
    // Constructs a new instance of [`App`].
    pub fn new(args: AppArgs) -> Self {
        Self {
            game: Game::new(args.seed.map(|s| s as u64)),
            should_quit: false,
        }
    }

    pub fn start(&mut self) {
        self.game.state = GameState::new();
    }

    pub fn input(&mut self, input: &str) {
        // println!("'{}'", input);
        match input {
            "state" => self.show_state(),
            "history" => self.show_history(),
            "quit" => self.quit(),
            "roll" => self.roll_dice(),
            _ => self.add_move(input),
        }
        println!("{}", self.display());
    }

    // Set running to false to quit the application.
    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    pub fn show_state(&self) {
        println!("{:?}", self.game.state)
    }

    pub fn show_history(&self) {
        for hist in self.game.state.history.iter() {
            println!("{:?}\n", hist);
        }
    }

    fn roll_dice(&mut self) {
        if self.game.player_id.is_none() {
            println!("player_id not set ");
            return;
        }
        if self.game.state.turn_stage != TurnStage::RollDice {
            println!("Not in the dice roll stage");
            return;
        }
        let dice = self.game.dice_roller.roll();
        self.game.handle_event(&GameEvent::RollResult {
            player_id: self.game.player_id.unwrap(),
            dice,
        });
    }

    fn add_move(&mut self, input: &str) {
        if self.game.player_id.is_none() {
            println!("player_id not set ");
            return;
        }
        let positions: Vec<usize> = input
            .split(' ')
            .map(|str| str.parse().unwrap_or(0))
            .collect();
        if positions.len() == 2 && positions[0] != 0 && positions[1] != 0 {
            if let Ok(checker_move) = CheckerMove::new(positions[0], positions[1]) {
                // if checker_move.is_ok() {
                if self.game.first_move.is_some() {
                    let move_event = GameEvent::Move {
                        player_id: self.game.player_id.unwrap(),
                        moves: (self.game.first_move.unwrap(), checker_move),
                    };
                    if !self.game.state.validate(&move_event) {
                        println!("Move invalid");
                        self.game.first_move = None;
                        return;
                    }
                    self.game.handle_event(&move_event);
                    self.game.first_move = None;
                } else {
                    self.game.first_move = Some(checker_move);
                }
                return;
            }
        }
        println!("invalid move : {}", input);
    }

    pub fn display(&mut self) -> String {
        let mut output = "-------------------------------".to_owned();
        output += format!(
            "\n{:?} > {} > {:?}",
            self.game.state.stage,
            self.game
                .state
                .who_plays()
                .map(|pl| &pl.name)
                .unwrap_or(&"?".to_owned()),
            self.game.state.turn_stage
        )
        .as_str();

        output = output + "\nRolled dice : " + &self.game.state.dice.to_display_string();

        if self.game.state.stage != Stage::PreGame {
            // display players points
            output += format!("\n\n{:<11} :: {:<5} :: {}", "Player", "holes", "points").as_str();

            for player_id in self.game.state.players.keys().sorted() {
                let player = &self.game.state.players[player_id];
                output += format!(
                    "\n{}. {:<8} :: {:<5} :: {}",
                    &player_id, &player.name, &player.holes, &player.points
                )
                .as_str();
            }
        }

        output += "\n-------------------------------\n";
        output += &self.game.state.board.to_display_grid(9);
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display() {
        let expected = "-------------------------------
PreGame > ? > RollDice
Rolled dice : 0 & 0
-------------------------------

     13   14   15   16   17   18      19   20   21   22   23   24  
  ----------------------------------------------------------------
 |                               | |                            X |
 |                               | |                            X |
 |                               | |                            X |
 |                               | |                            X |
 |                               | |                            X |
 |                               | |                            X |
 |                               | |                            X |
 |                               | |                            X |
 |                               | |                           15 |
 |------------------------------ | | -----------------------------|
 |                               | |                           15 |
 |                               | |                            O |
 |                               | |                            O |
 |                               | |                            O |
 |                               | |                            O |
 |                               | |                            O |
 |                               | |                            O |
 |                               | |                            O |
 |                               | |                            O |
  ----------------------------------------------------------------
    12   11   10    9    8    7        6    5    4    3    2    1   
";
        let mut app = App::default();
        self::assert_eq!(app.display(), expected);
    }

    #[test]
    fn test_move() {
        let expected = "-------------------------------
InGame > myself > RollDice
Rolled dice : 4 & 6

Player      :: holes :: points
1. myself   :: 0     :: 0
2. bot      :: 0     :: 0
-------------------------------

     13   14   15   16   17   18      19   20   21   22   23   24  
  ----------------------------------------------------------------
 |                             X | |        X                   X |
 |                               | |                            X |
 |                               | |                            X |
 |                               | |                            X |
 |                               | |                            X |
 |                               | |                            X |
 |                               | |                            X |
 |                               | |                            X |
 |                               | |                           13 |
 |------------------------------ | | -----------------------------|
 |                               | |                           13 |
 |                               | |                            O |
 |                               | |                            O |
 |                               | |                            O |
 |                               | |                            O |
 |                               | |                            O |
 |                               | |                            O |
 |                               | |                            O |
 |                               | |             O    O         O |
  ----------------------------------------------------------------
    12   11   10    9    8    7        6    5    4    3    2    1   
";
        let mut app = App::new(AppArgs { seed: Some(1327) });
        app.input("roll");
        app.input("1 3");
        app.input("1 4");
        self::assert_eq!(app.display(), expected);
    }
}
