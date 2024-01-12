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

    pub fn toGnupgPosId(&self) -> Vec<bool> {
        // Pieces placement -> 77bits (24 + 23 + 30 max)
        // inspired by https://www.gnu.org/software/gnubg/manual/html_node/A-technical-description-of-the-Position-ID.html
        // - white positions
        let white_board = self.board.clone();
        let mut posBits = white_board.iter().fold(vec![], |acc, nb| {
            let mut newAcc = acc.clone();
            if *nb as usize > 0 {
                // add as many `true` as there are pieces on the arrow
                newAcc.append(&mut vec![true; *nb as usize]);
            }
            newAcc.push(false); // arrow separator
            newAcc
        });

        // - black positions
        let mut black_board = self.board.clone();
        black_board.reverse();
        let mut posBlackBits = black_board.iter().fold(vec![], |acc, nb| {
            let mut newAcc = acc.clone();
            if (*nb as usize) < 0 {
                // add as many `true` as there are pieces on the arrow
                newAcc.append(&mut vec![true; 0 - *nb as usize]);
            }
            newAcc.push(false); // arrow separator
            newAcc
        });

        posBits.append(&mut posBlackBits);

        // fill with 0 bits until 77
        posBits.resize(77, false);
        posBits
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
                if self.raw_board.0.board[23 - field] > 1 {
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
    fn default_player_board() {
        assert_eq!(
            PlayerBoard::default(),
            PlayerBoard {
                board: [0, 0, 0, 0, 0, 5, 0, 3, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2,],
                off: 0
            }
        );
    }

    #[test]
    fn get_board() {
        let board = Board::new();
        assert_eq!(
            board.get(),
            BoardDisplay {
                board: [
                    -2, 0, 0, 0, 0, 5, 0, 3, 0, 0, 0, -5, 5, 0, 0, 0, -3, 0, -5, 0, 0, 0, 0, 2,
                ],
                off: (0, 0)
            }
        );
    }

    #[test]
    fn get_off() {
        let board = Board::new();
        assert_eq!(board.get_off(), (0, 0));
    }

    #[test]
    fn set_player0() -> Result<(), Error> {
        let mut board = Board::new();
        let player = Player {
            name: "".into(),
            color: Color::White,
        };
        board.set(&player, 1, 1)?;
        assert_eq!(board.get().board[1], 1);
        Ok(())
    }

    #[test]
    fn set_player1() -> Result<(), Error> {
        let mut board = Board::new();
        let player = Player {
            name: "".into(),
            color: Color::Black,
        };
        board.set(&player, 2, 1)?;
        assert_eq!(board.get().board[21], -1);
        Ok(())
    }

    #[test]
    fn set_player0_off() -> Result<(), Error> {
        let mut board = Board::new();
        let player = Player {
            name: "".into(),
            color: Color::White,
        };
        board.set_off(player, 1)?;
        assert_eq!(board.get().off.0, 1);
        Ok(())
    }

    #[test]
    fn set_player1_off() -> Result<(), Error> {
        let mut board = Board::new();
        let player = Player {
            name: "".into(),
            color: Color::Black,
        };
        board.set_off(player, 1)?;
        assert_eq!(board.get().off.1, 1);
        Ok(())
    }

    #[test]
    fn set_player1_off1() -> Result<(), Error> {
        let mut board = Board::new();
        let player = Player {
            name: "".into(),
            color: Color::Black,
        };
        board.set_off(player, 1)?;
        board.set_off(player, 1)?;
        assert_eq!(board.get().off.1, 2);
        Ok(())
    }

    #[test]
    fn blocked_player0() -> Result<(), Error> {
        let board = Board::new();
        assert!(board.blocked(
            &Player {
                name: "".into(),
                color: Color::White
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
                color: Color::Black
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
            },
            1,
            2,
        )?;
        assert!(board.blocked(
            &Player {
                name: "".into(),
                color: Color::White
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
            },
            1,
            2,
        )?;
        assert!(board.blocked(
            &Player {
                name: "".into(),
                color: Color::Black
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
                    color: Color::White
                },
                24
            )
            .is_err());
    }

    #[test]
    fn set_field_with_1_checker_player0_a() -> Result<(), Error> {
        let mut board = Board::new();
        board.set(
            &Player {
                name: "".into(),
                color: Color::White,
            },
            1,
            1,
        )?;
        board.set(
            &Player {
                name: "".into(),
                color: Color::Black,
            },
            22,
            1,
        )?;
        assert_eq!(board.get().board[1], -1);
        Ok(())
    }

    #[test]
    fn set_field_with_1_checker_player0_b() -> Result<(), Error> {
        let mut board = Board::new();
        board.set(
            &Player {
                name: "".into(),
                color: Color::White,
            },
            1,
            1,
        )?;
        board.set(
            &Player {
                name: "".into(),
                color: Color::Black,
            },
            22,
            1,
        )?;
        assert_eq!(board.get().board[1], -1);
        Ok(())
    }

    #[test]
    fn set_field_with_1_checker_player1_a() -> Result<(), Error> {
        let mut board = Board::new();
        board.set(
            &Player {
                name: "".into(),
                color: Color::Black,
            },
            1,
            1,
        )?;
        board.set(
            &Player {
                name: "".into(),
                color: Color::White,
            },
            22,
            1,
        )?;
        assert_eq!(board.get().board[22], 1);
        Ok(())
    }

    #[test]
    fn set_field_with_1_checker_player1_b() -> Result<(), Error> {
        let mut board = Board::new();
        board.set(
            &Player {
                name: "".into(),
                color: Color::Black,
            },
            1,
            1,
        )?;
        board.set(
            &Player {
                name: "".into(),
                color: Color::White,
            },
            22,
            1,
        )?;
        assert_eq!(board.get().board[22], 1);
        Ok(())
    }

    #[test]
    fn set_field_with_2_checkers_player0_a() -> Result<(), Error> {
        let mut board = Board::new();
        board.set(
            &Player {
                name: "".into(),
                color: Color::White,
            },
            23,
            2,
        )?;
        assert_eq!(board.get().board[23], 4);
        Ok(())
    }

    #[test]
    fn set_field_with_2_checkers_player0_b() -> Result<(), Error> {
        let mut board = Board::new();
        board.set(
            &Player {
                name: "".into(),
                color: Color::White,
            },
            23,
            -1,
        )?;
        assert_eq!(board.get().board[23], 1);
        Ok(())
    }

    #[test]
    fn set_field_blocked() {
        let mut board = Board::new();
        assert!(board
            .set(
                &Player {
                    name: "".into(),
                    color: Color::White
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
                    color: Color::White
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
                    color: Color::White
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
                    color: Color::Black
                },
                23,
                -3
            )
            .is_err());
    }
}
