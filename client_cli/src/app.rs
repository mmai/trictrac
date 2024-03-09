use pretty_assertions::assert_eq;
use store::{CheckerMove, GameEvent, GameState, PlayerId};

// Application.
#[derive(Debug, Default)]
pub struct App {
    // should the application exit?
    pub should_quit: bool,
    pub game: GameState,
    first_move: Option<CheckerMove>,
    player_id: Option<PlayerId>,
}

impl App {
    // Constructs a new instance of [`App`].
    pub fn new() -> Self {
        // Self::default()

        let mut state = GameState::default();
        // local : player
        let player_id: Option<PlayerId> = state.init_player("myself");
        state.init_player("adversary");
        println!("player_id ? {:?}", player_id);
        Self {
            game: state,
            should_quit: false,
            first_move: None,
            player_id,
        }
    }

    fn get_my_player(&mut self) {}

    // Constructs a new instance of [`App`].
    pub fn start(&mut self) {
        self.game = GameState::new();
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

    // Set running to false to quit the application.
    fn roll_dice(&mut self) {}

    fn add_move(&mut self, input: &str) {
        if self.player_id.is_none() {
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
                if self.first_move.is_some() {
                    self.game.consume(&GameEvent::Move {
                        player_id: self.player_id.unwrap(),
                        moves: (self.first_move.unwrap(), checker_move.unwrap()),
                    });
                    self.first_move = None;
                } else {
                    self.first_move = Some(checker_move.unwrap());
                }
                return;
            }
        }
        println!("invalid move : {}", input);
    }

    pub fn display(&mut self) -> String {
        let mut board = "".to_owned();
        board = board + &self.game.board.to_display_grid(9);
        board
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display() {
        let expected = "
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
        let expected = "
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
 |                              | |         O    O              O |
  ----------------------------------------------------------------
    12   11   10    9    8    7        6    5    4    3    2    1   
";
        let mut app = App::new();
        app.input("1 4");
        app.input("1 5");
        self::assert_eq!(app.display(), expected);
    }
}
