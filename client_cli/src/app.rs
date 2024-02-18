use pretty_assertions::assert_eq;
use store::GameState;

// Application.
#[derive(Debug, Default)]
pub struct App {
    // should the application exit?
    pub should_quit: bool,
    pub game: GameState,
}

impl App {
    // Constructs a new instance of [`App`].
    pub fn new() -> Self {
        Self::default()
    }

    // Constructs a new instance of [`App`].
    pub fn start(&mut self) {
        self.game = GameState::new();
    }

    pub fn input(&mut self, input: &str) {
        println!("'{}'", input);
        println!("'{}'", self.display());
        if input == "quit" {
            self.quit();
        }
    }

    // Set running to false to quit the application.
    pub fn quit(&mut self) {
        self.should_quit = true;
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
}
