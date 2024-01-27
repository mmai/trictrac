use crate::player::{Color, Player};
use crate::Error;
use serde::{Deserialize, Serialize};
use std::fmt;

/// field (aka 'point') position on the board (from 1 to 24)
pub type Field = usize;

#[derive(Debug, Copy, Clone, Serialize, PartialEq, Deserialize)]
pub struct CheckerMove {
    from: Field,
    to: Field,
}

impl CheckerMove {
    pub fn new(from: Field, to: Field) -> Result<Self, Error> {
        if from < 1 || 24 < from || to < 1 || 24 < to {
            return Err(Error::FieldInvalid);
        }
        Ok(CheckerMove { from, to })
    }

    pub fn get_to(&self) -> Field {
        self.to
    }
}

/// Represents the Tric Trac board
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Board {
    positions: [i8; 24],
}

impl Default for Board {
    fn default() -> Self {
        Board {
            positions: [
                15, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, -15,
            ],
        }
    }
}

// implement Display trait
impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut s = String::new();
        s.push_str(&format!("{:?}", self.positions));
        write!(f, "{}", s)
    }
}

impl Board {
    /// Create a new board
    pub fn new() -> Self {
        Board::default()
    }

    // maybe todo : operate on bits (cf. https://github.com/bungogood/bkgm/blob/a2fb3f395243bcb0bc9f146df73413f73f5ea1e0/src/position.rs#L217)
    pub fn to_gnupg_pos_id(&self) -> String {
        // Pieces placement -> 77bits (24 + 23 + 30 max)
        // inspired by https://www.gnu.org/software/gnubg/manual/html_node/A-technical-description-of-the-Position-ID.html
        // - white positions
        let white_board = self.positions.clone();
        let mut pos_bits = white_board.iter().fold(vec![], |acc, nb| {
            let mut new_acc = acc.clone();
            if *nb > 0 {
                // add as many `true` as there are pieces on the arrow
                new_acc.append(&mut vec!['1'; *nb as usize]);
            }
            new_acc.push('0'); // arrow separator
            new_acc
        });

        // - black positions
        let mut black_board = self.positions.clone();
        black_board.reverse();
        let mut pos_black_bits = black_board.iter().fold(vec![], |acc, nb| {
            let mut new_acc = acc.clone();
            if *nb < 0 {
                // add as many `true` as there are pieces on the arrow
                new_acc.append(&mut vec!['1'; (0 - *nb) as usize]);
            }
            new_acc.push('0'); // arrow separator
            new_acc
        });

        pos_bits.append(&mut pos_black_bits);

        // fill with 0 bits until 77
        pos_bits.resize(77, '0');
        pos_bits.iter().collect::<String>()
    }

    /// Set checkers for a player on a field
    ///
    /// This method adds the amount of checkers for a player on a field. The field is numbered from
    /// 1 to 24, starting from the first field of each player in the home board, the most far away
    /// field for each player is number 24.
    ///
    /// If the field is blocked for the player, an error is returned. If the field is not blocked,
    /// but there is already one checker from the other player on the field, that checker is hit and
    /// moved to the bar.
    pub fn set(&mut self, color: &Color, field: Field, amount: i8) -> Result<(), Error> {
        if field > 24 {
            return Err(Error::FieldInvalid);
        }

        if self.blocked(color, field)? {
            return Err(Error::FieldBlocked);
        }

        match color {
            Color::White => {
                let new = self.positions[field - 1] + amount;
                if new < 0 {
                    return Err(Error::MoveInvalid);
                }
                self.positions[field - 1] = new;

                Ok(())
            }
            Color::Black => {
                let new = self.positions[24 - field] - amount;
                if new > 0 {
                    return Err(Error::MoveInvalid);
                }
                self.positions[24 - field] = new;

                Ok(())
            }
        }
    }

    /// Check if a field is blocked for a player
    pub fn blocked(&self, color: &Color, field: Field) -> Result<bool, Error> {
        if field < 1 || 24 < field {
            return Err(Error::FieldInvalid);
        }

        // the square is blocked on the opponent rest corner or if there are opponent's men on the square
        match color {
            Color::White => {
                if field == 13 || self.positions[field - 1] < 0 {
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            Color::Black => {
                if field == 12 || self.positions[23 - field] > 1 {
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
        }
    }

    pub fn get_checkers_color(&self, field: Field) -> Result<Option<&Color>, Error> {
        if field < 1 || field > 24 {
            return Err(Error::FieldInvalid);
        }
        let checkers_count = self.positions[field - 1]; 
        let color = if checkers_count < 0 {
            Some(&Color::Black)
        } else if checkers_count > 0 {
            Some(&Color::White)
        } else {
            None
        };
        Ok(color)
    }

    pub fn move_possible(&self, color: &Color, cmove: CheckerMove) -> bool {
        let blocked = self.blocked(color, cmove.to).unwrap_or(true);
        // Check if there is a player's checker on the 'from' square
        let has_checker = self.get_checkers_color(cmove.from).unwrap_or(None) == Some(color);
        has_checker && !blocked
    }

    pub fn move_checker(&mut self, color: &Color, cmove: CheckerMove) -> Result<(), Error> {
        self.remove_checker(color, cmove.from)?;
        self.add_checker(color, cmove.to)?;
        Ok(())
    }

    pub fn remove_checker(&mut self, color: &Color, field: Field) -> Result<(), Error> {
        let checker_color = self.get_checkers_color(field)?;
        if Some(color) != checker_color {
            return Err(Error::FieldInvalid);
        }
        self.positions[field] -= 1;
        Ok(())
    }

    pub fn add_checker(&mut self, color: &Color, field: Field) -> Result<(), Error> {
        let checker_color = self.get_checkers_color(field)?;
        // error if the case contains the other color
        if None != checker_color && Some(color) != checker_color {
            return Err(Error::FieldInvalid);
        }
        self.positions[field] += 1;
        Ok(())
    }
}

/// Trait to move checkers
pub trait Move {
    /// Move a checker
    fn move_checker(&mut self, player: &Player, dice: u8, from: Field) -> Result<&mut Self, Error>
    where
        Self: Sized;

    /// Move permitted
    fn move_permitted(&mut self, player: &Player, dice: u8) -> Result<&mut Self, Error>
    where
        Self: Sized;
}

// Unit Tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_board() {
        assert_eq!(Board::new(), Board::default());
    }

    #[test]
    fn blocked_outofrange() -> Result<(), Error> {
        let board = Board::new();
        assert!(board.blocked( &Color::White, 0).is_err());
        assert!(board.blocked( &Color::White, 28).is_err());
        Ok(())
    }

    #[test]
    fn blocked_otherplayer() -> Result<(), Error> {
        let board = Board::new();
        assert!(board.blocked( &Color::White, 24)?);
        Ok(())
    }

    #[test]
    fn blocked_notblocked() -> Result<(), Error> {
        let board = Board::new();
        assert!(!board.blocked( &Color::White, 6)?);
        Ok(())
    }


    #[test]
    fn set_field_blocked() {
        let mut board = Board::new();
        assert!(
            board.set( &Color::White, 0, 24)
            .is_err()
            );
    }

    #[test]
    fn set_wrong_field1() {
        let mut board = Board::new();
        assert!(board
            .set( &Color::White, 50, 2)
            .is_err());
    }

    #[test]
    fn set_wrong_amount0() {
        let mut board = Board::new();
        assert!(board
            .set(&Color::White , 23, -3)
            .is_err());
    }

    #[test]
    fn set_wrong_amount1() {
        let mut board = Board::new();
        let player = Player::new("".into(), Color::White);
        assert!(board
            .set( &Color::White, 23, -3)
            .is_err());
    }
}
