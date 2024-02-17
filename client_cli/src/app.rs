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
        let mut board = "
  24   23   22   21   20   19        18   17   16   15   14   13  
-------------------------------------------------------------------"
            .to_owned();
        board = board
            + "-------------------------------------------------------------------
   1    2    3    4    5    6         7    8    9   10   11   12   ";

        // ligne 1 à 8 : positions 24 à 13
        // ligne 9 nombre exact
        // ligne 10 ---
        // lignes 11 à 18 : positions 1 à 12
        board
        // self.game.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display() {
        let expected = "
  24   23   22   21   20   19        18   17   16   15   14   13  
-------------------------------------------------------------------
|                               | |                            X  |
|                               | |                            X  |
|                               | |                            X  |
|                               | |                            X  |
|                               | |                            X  |
|                               | |                            X  |
|                               | |                            X  |
|                               | |                            X  |
|                               | |                           15  |
|------------------------------ | | ------------------------------|
|                               | |                           15  |
|                               | |                            O  |
|                               | |                            O  |
|                               | |                            O  |
|                               | |                            O  |
|                               | |                            O  |
|                               | |                            O  |
|                               | |                            O  |
|                               | |                            O  |
-------------------------------------------------------------------
   1    2    3    4    5    6         7    8    9   10   11   12   ";
        let mut app = App::default();
        assert_eq!(app.display(), expected);
    }
}
