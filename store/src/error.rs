/// This module contains the error definition for the Trictrac game.
use std::fmt;

/// Holds all possible errors that can occur during a Trictrac game.
#[derive(Debug)]
pub enum Error {
    /// Game has already started
    GameStarted,
    /// Game has already ended
    GameEnded,
    /// Doubling not permitted
    DoublingNotPermitted,
    /// Invalid player
    PlayerInvalid,
    /// Field blocked
    FieldBlocked,
    /// Invalid field
    FieldInvalid,
    /// Not your turn
    NotYourTurn,
    /// Invalid move
    MoveInvalid,
    /// Move first
    MoveFirst,
    /// Roll first
    RollFirst,
    /// Dice Invalid
    DiceInvalid,
}

// implement Error trait
impl std::error::Error for Error {}

// implement Display trait
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::GameStarted => write!(f, "Game has already started"),
            Error::GameEnded => write!(f, "Game has already ended"),
            Error::PlayerInvalid => write!(f, "Invalid player"),
            Error::DoublingNotPermitted => write!(f, "Doubling not permitted"),
            Error::FieldBlocked => write!(f, "Field blocked"),
            Error::FieldInvalid => write!(f, "Invalid field"),
            Error::NotYourTurn => write!(f, "Not your turn"),
            Error::MoveInvalid => write!(f, "Invalid move"),
            Error::MoveFirst => write!(f, "Move first"),
            Error::RollFirst => write!(f, "Roll first"),
            Error::DiceInvalid => write!(f, "Invalid dice"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        assert_eq!(
            format!("{}", Error::GameStarted),
            "Game has already started"
        );
        assert_eq!(format!("{}", Error::GameEnded), "Game has already ended");
        assert_eq!(format!("{}", Error::PlayerInvalid), "Invalid player");
        assert_eq!(format!("{}", Error::FieldBlocked), "Field blocked");
        assert_eq!(format!("{}", Error::FieldInvalid), "Invalid field");
        assert_eq!(format!("{}", Error::NotYourTurn), "Not your turn");
        assert_eq!(format!("{}", Error::MoveInvalid), "Invalid move");
        assert_eq!(format!("{}", Error::MoveFirst), "Move first");
        assert_eq!(format!("{}", Error::RollFirst), "Roll first");
        assert_eq!(format!("{}", Error::DiceInvalid), "Invalid dice");
    }
}
