use pretty_assertions::assert_eq;
use store::{CheckerMove, Dice, DiceRoller, GameEvent, GameState, PlayerId};

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
}

impl Game {
    // Constructs a new instance of [`App`].
    pub fn new(seed: Option<u64>) -> Self {
        let mut state = GameState::default();
        // local : player
        let player_id: Option<PlayerId> = state.init_player("myself");
        state.init_player("adversary");
        state.consume(&GameEvent::BeginGame {
            goes_first: player_id.unwrap(),
        });
        Self {
            state,
            dice_roller: DiceRoller::new(seed),
            first_move: None,
            player_id,
        }
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

    fn get_my_player(&mut self) {}

    pub fn start(&mut self) {
        self.game.state = GameState::new();
    }

    pub fn input(&mut self, input: &str) {
        println!("'{}'", input);
        match input {
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

    fn roll_dice(&mut self) {
        if self.game.player_id.is_none() {
            println!("player_id not set ");
            return;
        }
        let dice = self.game.dice_roller.roll();
        self.game.state.consume(&GameEvent::RollResult {
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
            let checker_move = CheckerMove::new(positions[0], positions[1]);
            if checker_move.is_ok() {
                if self.game.first_move.is_some() {
                    let move_event = GameEvent::Move {
                        player_id: self.game.player_id.unwrap(),
                        moves: (self.game.first_move.unwrap(), checker_move.unwrap()),
                    };
                    if !self.game.state.validate(&move_event) {
                        println!("Move invalid");
                        self.game.first_move = None;
                        return;
                    }
                    self.game.state.consume(&move_event);
                    self.game.first_move = None;
                } else {
                    self.game.first_move = Some(checker_move.unwrap());
                }
                return;
            }
        }
        println!("invalid move : {}", input);
    }

    pub fn display(&mut self) -> String {
        let mut output = "-------------------------------".to_owned();
        output = output + "\nRolled dice : " + &self.game.state.dice.to_display_string();
        output = output + "\n-------------------------------";
        output = output + "\n" + &self.game.state.board.to_display_grid(9);
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display() {
        let expected = "-------------------------------
Rolled dice : 0 & 0
-------------------------------

    13   14   15   16   17   18       19   20   21   22   23   24  
  ----------------------------------------------------------------
 |                              | |                             X |
 |                              | |                             X |
 |                              | |                             X |
 |                              | |                             X |
 |                              | |                             X |
 |                              | |                             X |
 |                              | |                             X |
 |                              | |                             X |
 |                              | |                            15 |
 |----------------------------- | | ------------------------------|
 |                              | |                            15 |
 |                              | |                             O |
 |                              | |                             O |
 |                              | |                             O |
 |                              | |                             O |
 |                              | |                             O |
 |                              | |                             O |
 |                              | |                             O |
 |                              | |                             O |
  ----------------------------------------------------------------
    12   11   10    9    8    7        6    5    4    3    2    1   
";
        let mut app = App::default();
        self::assert_eq!(app.display(), expected);
    }

    #[test]
    fn test_move() {
        let expected = "-------------------------------
Rolled dice : 2 & 3
-------------------------------

    13   14   15   16   17   18       19   20   21   22   23   24  
  ----------------------------------------------------------------
 |                              | |                             X |
 |                              | |                             X |
 |                              | |                             X |
 |                              | |                             X |
 |                              | |                             X |
 |                              | |                             X |
 |                              | |                             X |
 |                              | |                             X |
 |                              | |                            15 |
 |----------------------------- | | ------------------------------|
 |                              | |                            13 |
 |                              | |                             O |
 |                              | |                             O |
 |                              | |                             O |
 |                              | |                             O |
 |                              | |                             O |
 |                              | |                             O |
 |                              | |                             O |
 |                              | |              O    O         O |
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
