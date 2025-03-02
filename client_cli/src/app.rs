use bot::{BotStrategy, DefaultStrategy, StableBaselines3Strategy};
use itertools::Itertools;

use crate::game_runner::GameRunner;
use store::{CheckerMove, GameEvent, GameState, Stage, TurnStage};

#[derive(Debug, Default)]
pub struct AppArgs {
    pub seed: Option<u32>,
    pub bot: Option<String>,
}

// Application.
#[derive(Debug, Default)]
pub struct App {
    // should the application exit?
    pub should_quit: bool,
    pub schools_enabled: bool,
    pub game: GameRunner,
}

impl App {
    // Constructs a new instance of [`App`].
    pub fn new(args: AppArgs) -> Self {
        let bot_strategies: Vec<Box<dyn BotStrategy>> = args
            .bot
            .as_deref()
            .map(|str_bots| {
                str_bots
                    .split(",")
                    .filter_map(|s| match s.trim() {
                        "dummy" => {
                            Some(Box::new(DefaultStrategy::default()) as Box<dyn BotStrategy>)
                        }
                        "ai" => {
                            Some(Box::new(StableBaselines3Strategy::default()) as Box<dyn BotStrategy>)
                        }
                        s if s.starts_with("ai:") => {
                            let path = s.trim_start_matches("ai:");
                            Some(Box::new(StableBaselines3Strategy::new(path)) as Box<dyn BotStrategy>)
                        }
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_default();
        let schools_enabled = false;
        let should_quit = bot_strategies.len() > 1;
        Self {
            game: GameRunner::new(schools_enabled, bot_strategies, args.seed.map(|s| s as u64)),
            should_quit,
            schools_enabled,
        }
    }

    pub fn start(&mut self) {
        self.game.state = GameState::new(self.schools_enabled);
    }

    pub fn input(&mut self, input: &str) {
        // println!("'{}'", input);
        match input {
            "state" => self.show_state(),
            "history" => self.show_history(),
            "quit" => self.quit(),
            // run bots game (when two bots)
            "bots" => self.bots_all(),
            "" => self.bots_next_step(),
            // play (when one bot)
            "roll" => self.roll_dice(),
            "go" => self.go(),
            _ => self.add_move(input),
        }
        println!("{}", self.display());
    }

    // --- 2 bots game actions

    fn bots_all(&mut self) {}

    fn bots_next_step(&mut self) {}

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

        // get correct points for these board and dice
        // let points_rules = PointsRules::new(
        //     &self
        //         .game
        //         .state
        //         .player_color_by_id(&self.game.player_id.unwrap())
        //         .unwrap(),
        //     &self.game.state.board,
        //     dice,
        // );
        self.game.handle_event(&GameEvent::RollResult {
            player_id: self.game.player_id.unwrap(),
            dice,
        });
    }

    fn go(&mut self) {
        if self.game.player_id.is_none() {
            println!("player_id not set ");
            return;
        }
        if self.game.state.turn_stage != TurnStage::HoldOrGoChoice {
            println!("Not in position to go");
            return;
        }
        self.game.handle_event(&GameEvent::Go {
            player_id: self.game.player_id.unwrap(),
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
        let winner = self
            .game
            .state
            .determine_winner()
            .and_then(|id| self.game.state.players.get(&id));
        let str_won: String = winner
            .map(|p| {
                let mut name = " winner: ".to_owned();
                name.push_str(&p.name);
                name
            })
            .unwrap_or("".to_owned());
        let mut output = "-------------------------------".to_owned();
        output += format!(
            "\n{:?}{} > {} > {:?}",
            self.game.state.stage,
            str_won,
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
            output = output + "\nRolled dice jans : " + &format!("{:?}", self.game.state.dice_jans);
            output = output
                + "\nLast move : "
                + &self.game.state.dice_moves.0.to_display_string()
                + ", "
                + &self.game.state.dice_moves.1.to_display_string();

            // display players points
            output += format!("\n\n{:<11} :: {:<5} :: {}", "Player", "holes", "points").as_str();

            for player_id in self.game.state.players.keys().sorted() {
                let player = &self.game.state.players[player_id];
                output += format!(
                    "\n{}. {:<8} :: {:<5} :: {}",
                    &player_id, &player.name, &player.holes, &player.points,
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
    use pretty_assertions::assert_eq;

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
Rolled dice jans : {}
Last move : CheckerMove { from: 24, to: 18 } , CheckerMove { from: 24, to: 20 } 

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
        let mut app = App::new(AppArgs {
            seed: Some(1327),
            bot: Some("dummy".into()),
        });
        app.input("roll");
        app.input("1 3");
        app.input("1 4");
        self::assert_eq!(app.display(), expected);
    }
}
