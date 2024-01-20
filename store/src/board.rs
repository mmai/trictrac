use crate::player::{Color, Player};
use crate::Error;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Represents the Tric Trac board
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Board {
    board: [i8; 24],
}

impl Default for Board {
    fn default() -> Self {
        Board {
            board: [
                15, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, -15,
            ],
        }
    }
}

// implement Display trait
impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut s = String::new();
        s.push_str(&format!("{:?}", self.board));
        write!(f, "{}", s)
    }
}

impl Board {
    /// Create a new board
    pub fn new() -> Self {
        Board::default()
    }

    pub fn to_gnupg_pos_id(&self) -> String {
        // Pieces placement -> 77bits (24 + 23 + 30 max)
        // inspired by https://www.gnu.org/software/gnubg/manual/html_node/A-technical-description-of-the-Position-ID.html
        // - white positions
        let white_board = self.board.clone();
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
        let mut black_board = self.board.clone();
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
    pub fn set(&mut self, player: &Player, field: usize, amount: i8) -> Result<(), Error> {
        if field > 24 {
            return Err(Error::FieldInvalid);
        }

        if self.blocked(player, field)? {
            return Err(Error::FieldBlocked);
        }

        match player.color {
            Color::White => {
                let new = self.board[field - 1] + amount;
                if new < 0 {
                    return Err(Error::MoveInvalid);
                }
                self.board[field - 1] = new;

                Ok(())
            }
            Color::Black => {
                let new = self.board[24 - field] - amount;
                if new > 0 {
                    return Err(Error::MoveInvalid);
                }
                self.board[24 - field] = new;

                Ok(())
            }
        }
    }

    /// Check if a field is blocked for a player
    pub fn blocked(&self, player: &Player, field: usize) -> Result<bool, Error> {
        if field > 24 {
            return Err(Error::FieldInvalid);
        }

        match player.color {
            Color::White => {
                if self.board[field - 1] < 0 {
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            Color::Black => {
                if self.board[23 - field] > 1 {
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
        }
    }
}

/// Trait to move checkers
pub trait Move {
    /// Move a checker
    fn move_checker(&mut self, player: &Player, dice: u8, from: usize) -> Result<&mut Self, Error>
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
    fn blocked_player0() -> Result<(), Error> {
        let board = Board::new();
        assert!(board.blocked(
            &Player {
                name: "".into(),
                color: Color::White,
                holes: 0,
                points: 0,
                can_bredouille: true,
                can_big_bredouille: true
            },
            0
        )?);
        Ok(())
    }

    #[test]
    fn blocked_player1() -> Result<(), Error> {
        let board = Board::new();
        assert!(board.blocked(
            &Player {
                name: "".into(),
                color: Color::Black,
                holes: 0,
                points: 0,
                can_bredouille: true,
                can_big_bredouille: true
            },
            0
        )?);
        Ok(())
    }

    #[test]
    fn blocked_player0_a() -> Result<(), Error> {
        let mut board = Board::new();
        board.set(
            &Player {
                name: "".into(),
                color: Color::Black,
                holes: 0,
                points: 0,
                can_bredouille: true,
                can_big_bredouille: true
            },
            1,
            2,
        )?;
        assert!(board.blocked(
            &Player {
                name: "".into(),
                color: Color::White,
                holes: 0,
                points: 0,
                can_bredouille: true,
                can_big_bredouille: true
            },
            22
        )?);
        Ok(())
    }

    #[test]
    fn blocked_player1_a() -> Result<(), Error> {
        let mut board = Board::new();
        board.set(
            &Player {
                name: "".into(),
                color: Color::White,
                holes: 0,
                points: 0,
                can_bredouille: true,
                can_big_bredouille: true
            },
            1,
            2,
        )?;
        assert!(board.blocked(
            &Player {
                name: "".into(),
                color: Color::Black,
                holes: 0,
                points: 0,
                can_bredouille: true,
                can_big_bredouille: true
            },
            22
        )?);
        Ok(())
    }

    #[test]
    fn blocked_invalid_field() {
        let board = Board::new();
        assert!(board
            .blocked(
                &Player {
                    name: "".into(),
                    color: Color::White,
                    holes: 0,
                    points: 0,
                    can_bredouille: true,
                    can_big_bredouille: true
                },
                24
            )
            .is_err());
    }

    #[test]
    fn set_field_blocked() {
        let mut board = Board::new();
        assert!(board
            .set(
                &Player {
                    name: "".into(),
                    color: Color::White,
                holes: 0,
                points: 0,
                can_bredouille: true,
                can_big_bredouille: true
                },
                0,
                2
            )
            .is_err());
    }

    #[test]
    fn set_wrong_field1() {
        let mut board = Board::new();
        assert!(board
            .set(
                &Player {
                    name: "".into(),
                    color: Color::White,
                holes: 0,
                points: 0,
                can_bredouille: true,
                can_big_bredouille: true
                },
                50,
                2
            )
            .is_err());
    }

    #[test]
    fn set_wrong_amount0() {
        let mut board = Board::new();
        assert!(board
            .set(
                &Player {
                    name: "".into(),
                    color: Color::White,
                holes: 0,
                points: 0,
                can_bredouille: true,
                can_big_bredouille: true
                },
                23,
                -3
            )
            .is_err());
    }

    #[test]
    fn set_wrong_amount1() {
        let mut board = Board::new();
        assert!(board
            .set(
                &Player {
                    name: "".into(),
                    color: Color::Black,
                holes: 0,
                points: 0,
                can_bredouille: true,
                can_big_bredouille: true
                },
                23,
                -3
            )
            .is_err());
    }
}
