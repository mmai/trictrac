use serde::{Deserialize, Serialize};
use std::fmt;

// This just makes it easier to dissern between a player id and any ol' u64
pub type PlayerId = u64;

#[derive(Copy, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Color {
    White,
    Black,
}

/// Struct for storing player related data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Player {
    pub name: String,
    pub color: Color,
    pub points: u8,
    pub holes: u8,
    pub can_bredouille: bool,
    pub can_big_bredouille: bool,
}

impl Player {
    pub fn new(name: String, color: Color) -> Self {
        Player {
            name,
            color,
            points: 0,
            holes: 0,
            can_bredouille: true,
            can_big_bredouille: true,
        }
    }

    pub fn to_bits_string(&self) -> String {
        format!(
            "{:0>4b}{:0>4b}{:b}{:b}",
            self.points, self.holes, self.can_bredouille as u8, self.can_big_bredouille as u8
        )
    }
}

/// Represents a player in the game.
///
/// Part of the rules of the game is that this game is for only two players, we call them Player 0
/// and Player 1. The labels are chosen arbitrarily and do not affect the game at all, however, it
/// is convenient here to use 0 and 1 as labels because we sometimes use Rust tuples which we can
/// then address the same way. There is a special case where nobody is allowed to move or act, for
/// example when a game begins or ends, thus we define this as the default.
#[derive(
    Debug, Clone, Copy, Eq, Ord, PartialEq, PartialOrd, Hash, Serialize, Deserialize, Default,
)]
pub enum CurrentPlayer {
    /// None of the two players, e.g. at start or end of game.
    #[default]
    Nobody,
    /// Player 0
    Player0,
    /// Player 1
    Player1,
}

impl CurrentPlayer {
    /// Returns the other player, i.e. the player who is not the current player.
    pub fn other(&self) -> Self {
        match *self {
            CurrentPlayer::Nobody => CurrentPlayer::Nobody,
            CurrentPlayer::Player0 => CurrentPlayer::Player1,
            CurrentPlayer::Player1 => CurrentPlayer::Player0,
        }
    }
}

// Implement Display trait for Player
impl fmt::Display for CurrentPlayer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            CurrentPlayer::Nobody => write!(f, "Nobody"),
            CurrentPlayer::Player0 => write!(f, "Player 0"),
            CurrentPlayer::Player1 => write!(f, "Player 1"),
        }
    }
}

// Test Display trait for Player
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_other() {
        assert_eq!(CurrentPlayer::Nobody.other(), CurrentPlayer::Nobody);
        assert_eq!(CurrentPlayer::Player0.other(), CurrentPlayer::Player1);
        assert_eq!(CurrentPlayer::Player1.other(), CurrentPlayer::Player0);
    }

    #[test]
    fn test_to_bits_string() {
        let player = Player {
            name: "Edgar".into(),
            color: Color::White,
            points: 11,
            holes: 3,
            can_bredouille: true,
            can_big_bredouille: false,
        };
        println!("{}", player.to_bits_string());
        assert!(player.to_bits_string() == "1011001110");
    }
}
