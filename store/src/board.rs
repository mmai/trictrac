use crate::player::Color;
use crate::Error;
use serde::{Deserialize, Serialize};
use std::cmp;
use std::fmt;

/// field (aka 'point') position on the board (from 0 to 24, 0 being 'outside')
pub type Field = usize;

#[derive(Debug, Copy, Clone, Serialize, PartialEq, Deserialize)]
pub struct CheckerMove {
    from: Field,
    to: Field,
}

pub const EMPTY_MOVE: CheckerMove = CheckerMove { from: 0, to: 0 };

fn transpose(matrix: Vec<Vec<String>>) -> Vec<Vec<String>> {
    let num_cols = matrix.first().unwrap().len();
    let mut row_iters: Vec<_> = matrix.into_iter().map(Vec::into_iter).collect();
    let mut out: Vec<Vec<_>> = (0..num_cols).map(|_| Vec::new()).collect();

    for out_row in out.iter_mut() {
        for it in row_iters.iter_mut() {
            out_row.push(it.next().unwrap());
        }
    }
    out
}

impl Default for CheckerMove {
    fn default() -> Self {
        EMPTY_MOVE
    }
}

impl CheckerMove {
    pub fn to_display_string(self) -> String {
        format!("{:?} ", self)
    }

    pub fn new(from: Field, to: Field) -> Result<Self, Error> {
        // println!("from {} to {}", from, to);
        // check if the field is on the board
        // we allow 0 for 'to', which represents the exit of a checker
        // and (0, 0) which represent the absence of a move (when there is only one checker left on the
        // board)
        if ((from, to) != (0, 0)) && (!(1..25).contains(&from) || 24 < to) {
            return Err(Error::FieldInvalid);
        }
        // check that the destination is after the origin field
        // --> not applicable for black moves
        // if to < from && to != 0 {
        //     return Err(Error::MoveInvalid);
        // }
        Ok(Self { from, to })
    }

    /// Get the mirrord CheckerMove (ie change colors)
    pub fn mirror(&self) -> Self {
        let from = if self.from == 0 { 0 } else { 25 - self.from };
        let to = if self.to == 0 { 0 } else { 25 - self.to };
        Self { from, to }
    }

    // Construct the move resulting of two successive moves
    pub fn chain(self, cmove: Self) -> Result<Self, Error> {
        if self.to != cmove.from {
            return Err(Error::MoveInvalid);
        }
        Ok(Self {
            from: self.from,
            to: cmove.to,
        })
    }

    pub fn get_from(&self) -> Field {
        self.from
    }

    pub fn get_to(&self) -> Field {
        self.to
    }

    pub fn is_exit(&self) -> bool {
        self.to == 0 && self != &EMPTY_MOVE
    }

    pub fn doable_with_dice(&self, dice: usize) -> bool {
        (self.to == 0 && 25 - self.from <= dice)
            || (self.from < self.to && self.to - self.from == dice)
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

    /// Get the mirrord board (ie change colors)
    pub fn mirror(&self) -> Self {
        let mut positions = self.positions.map(|c| 0 - c);
        positions.reverse();
        Board { positions }
    }

    /// Globally set pieces on board ( for tests )
    pub fn set_positions(&mut self, positions: [i8; 24]) {
        self.positions = positions;
    }

    pub fn count_checkers(&self, color: Color, from: Field, to: Field) -> u8 {
        if to == 0 || from == 0 {
            return 0;
        }
        self.positions[(from - 1)..to]
            .iter()
            .filter(|count| {
                if color == Color::White {
                    **count > 0 as i8
                } else {
                    **count < 0 as i8
                }
            })
            .sum::<i8>()
            .unsigned_abs()
    }

    // maybe todo : operate on bits (cf. https://github.com/bungogood/bkgm/blob/a2fb3f395243bcb0bc9f146df73413f73f5ea1e0/src/position.rs#L217)
    pub fn to_gnupg_pos_id(&self) -> String {
        // Pieces placement -> 77bits (24 + 23 + 30 max)
        // inspired by https://www.gnu.org/software/gnubg/manual/html_node/A-technical-description-of-the-Position-ID.html
        // - white positions
        let white_board = self.positions;
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
        let mut black_board = self.positions;
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

    /// format positions to a grid of symbols
    pub fn to_display_grid(&self, col_size: usize) -> String {
        // convert numbers to columns of chars
        let mut columns: Vec<Vec<String>> = self
            .positions
            .iter()
            .map(|count| {
                let char = if *count > 0 { "O" } else { "X" };
                let men_count = count.abs();
                let mut cells = vec!["".to_owned(); col_size];
                cells[0..(cmp::min(men_count, col_size as i8) as usize)].fill(char.to_owned());
                if men_count as usize > col_size {
                    cells[col_size - 1] = men_count.to_string();
                }
                cells
            })
            .collect();

        // upper columns (13 to 24)
        let upper_positions: Vec<Vec<String>> = columns.split_off(12).into_iter().collect();

        // lower columns (12 to 1)
        let mut lower_positions: Vec<Vec<String>> = columns
            .into_iter()
            .map(|mut col| {
                col.reverse();
                col
            })
            .collect();
        lower_positions.reverse();

        // display board columns
        let upper: Vec<String> = transpose(upper_positions)
            .into_iter()
            .map(|cells| {
                cells
                    .into_iter()
                    .map(|cell| format!("{:>5}", cell))
                    .collect::<Vec<String>>()
                    .join("")
            })
            .collect();

        let lower: Vec<String> = transpose(lower_positions)
            .into_iter()
            .map(|cells| {
                cells
                    .into_iter()
                    .map(|cell| format!("{:>5}", cell))
                    .collect::<Vec<String>>()
                    .join("")
            })
            .collect();

        let mut output = "
     13   14   15   16   17   18      19   20   21   22   23   24  
  ----------------------------------------------------------------\n"
            .to_owned();
        for mut line in upper {
            // add middle bar
            line.replace_range(31..31, "| |");
            output = output + " |" + &line + " |\n";
        }
        output += " |------------------------------ | | -----------------------------|\n";
        for mut line in lower {
            // add middle bar
            line.replace_range(31..31, "| |");
            output = output + " |" + &line + " |\n";
        }
        output += "  ----------------------------------------------------------------
    12   11   10    9    8    7        6    5    4    3    2    1   \n";
        output
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

        // the exit : no checker added to the board
        if field == 0 {
            return Ok(());
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
        // the square is blocked on the opponent rest corner
        let opp_corner_field = if color == &Color::White { 13 } else { 12 };
        self.passage_blocked(color, field)
        // .map(|blocked| blocked || opp_corner_field == field)
    }

    pub fn passage_blocked(&self, color: &Color, field: Field) -> Result<bool, Error> {
        if 24 < field {
            return Err(Error::FieldInvalid);
        }

        // the exit is never 'blocked'
        if field == 0 {
            return Ok(false);
        }

        // the square is blocked if there are opponent's men on the square
        let blocked = if color == &Color::White {
            self.positions[field - 1] < 0
        } else {
            self.positions[field - 1] > 0
        };
        Ok(blocked)
    }

    pub fn get_field_checkers(&self, field: Field) -> Result<(u8, Option<&Color>), Error> {
        if !(1..25).contains(&field) {
            return Err(Error::FieldInvalid);
        }
        let checkers_count = self.positions[field - 1];
        let color = match checkers_count.cmp(&0) {
            cmp::Ordering::Less => Some(&Color::Black),
            cmp::Ordering::Greater => Some(&Color::White),
            cmp::Ordering::Equal => None,
        };
        Ok((checkers_count.unsigned_abs(), color))
    }

    pub fn get_checkers_color(&self, field: Field) -> Result<Option<&Color>, Error> {
        self.get_field_checkers(field).map(|(_ount, color)| color)
    }

    pub fn is_field_in_small_jan(field: Field) -> bool {
        field < 7 || field > 18
    }

    /// returns the list of Fields containing Checkers of the Color
    pub fn get_color_fields(&self, color: Color) -> Vec<(usize, i8)> {
        match color {
            Color::White => self
                .positions
                .iter()
                .enumerate()
                .filter(|&(_, count)| *count > 0)
                .map(|(i, count)| (i + 1, *count))
                .collect(),
            Color::Black => self
                .positions
                .iter()
                .enumerate()
                .filter(|&(_, count)| *count < 0)
                .rev()
                .map(|(i, count)| (i + 1, (0 - count)))
                .collect(),
        }
    }

    // Get the corner field for the color
    pub fn get_color_corner(&self, color: &Color) -> Field {
        if color == &Color::White {
            12
        } else {
            13
        }
    }

    pub fn get_possible_moves(
        &self,
        color: Color,
        dice: u8,
        with_excedants: bool,
        check_rest_corner_exit: bool,
        forbid_exits: bool,
    ) -> Vec<CheckerMove> {
        let mut moves = Vec::new();

        let get_dest = |from| {
            if color == Color::White {
                if from + dice as i32 == 25 {
                    0
                } else {
                    from + dice as i32
                }
            } else {
                from - dice as i32
            }
        };

        for (field, count) in self.get_color_fields(color) {
            // check rest corner exit
            if field == self.get_color_corner(&color) && count == 2 && check_rest_corner_exit {
                continue;
            }
            let mut dest = get_dest(field as i32);
            if dest == 0 && forbid_exits {
                continue;
            }
            if !(0..25).contains(&dest) {
                if with_excedants && !forbid_exits {
                    dest = 0;
                } else {
                    continue;
                }
            }
            if let Ok(cmove) = CheckerMove::new(field, dest.unsigned_abs() as usize) {
                if let Ok(false) = self.blocked(&color, dest.unsigned_abs() as usize) {
                    moves.push(cmove);
                }
            }
        }
        moves
    }

    pub fn passage_possible(&self, color: &Color, cmove: &CheckerMove) -> bool {
        !self.passage_blocked(color, cmove.to).unwrap_or(true)
    }

    pub fn move_possible(&self, color: &Color, cmove: &CheckerMove) -> bool {
        let blocked = self.blocked(color, cmove.to).unwrap_or(true);
        // Check if there is a player's checker on the 'from' square
        let has_checker = self.get_checkers_color(cmove.from).unwrap_or(None) == Some(color);
        has_checker && !blocked
    }

    /// Return if there is a quarter filled by the color
    pub fn any_quarter_filled(&self, color: Color) -> bool {
        [1, 7, 13, 19]
            .iter()
            .any(|field| self.is_quarter_filled(color, *field))
    }

    /// Return if the quarter containing `field` is filled by the `color`
    pub fn is_quarter_filled(&self, color: Color, field: Field) -> bool {
        let fields = self.get_quarter_fields(field);
        !fields.iter().any(|field| {
            if color == Color::White {
                self.positions[field - 1] < 2
            } else {
                self.positions[field - 1] > -2
            }
        })
    }

    pub fn get_quarter_filling_candidate(&self, color: Color) -> Vec<Field> {
        let mut missing = vec![];
        // first quarter
        for quarter in [1..7, 7..13, 13..19, 19..25] {
            missing = vec![];
            for field in quarter {
                let field_count = if color == Color::Black {
                    0 - self.positions[field - 1]
                } else {
                    self.positions[field - 1]
                };
                if field_count < 0 {
                    // opponent checker found : this quarter cannot be filled
                    missing = vec![];
                    continue;
                }
                if field_count == 0 {
                    missing.push(field);
                    missing.push(field);
                } else if field_count == 1 {
                    missing.push(field);
                }
            }
            if missing.len() < 3 {
                // fillable quarter found (no more than two missing checkers)
                if let Some(field) = missing.first() {
                    // We check that there are sufficient checkers left to fill the quarter
                    if !self.is_quarter_fillable(color, *field) {
                        missing = vec![];
                    }
                }
                // there will be no other fillable quarter
                break;
            }
        }
        missing
    }

    /// Returns whether the `color` player can still fill the quarter containing the `field`
    /// * `color` - color of the player
    /// * `field` - field belonging to the quarter
    pub fn is_quarter_fillable(&self, color: Color, field: Field) -> bool {
        let fields = self.get_quarter_fields(field);

        // opponent rest corner
        if color == Color::White && fields.contains(&13)
            || color == Color::Black && fields.contains(&12)
        {
            return false;
        }

        // is there a sufficient number of checkers on or before each fields ?
        for field in fields {
            // Number of checkers needed before this field (included) :
            // 2 checkers by field, from the begining of the quarter
            let mut field_pos = field % 6;
            if field_pos == 0 {
                field_pos = 6;
            }
            if color == Color::Black {
                field_pos = 7 - field_pos;
            }
            let needed = 2 * field_pos;

            let (from, to) = if color == Color::White {
                (1, field)
            } else {
                (field, 24)
            };
            if self.count_checkers(color, from, to) < needed as u8 {
                return false;
            }
        }
        true
    }

    /// Returns the 6 fields of the quarter containing the `field`
    fn get_quarter_fields(&self, field: Field) -> [Field; 6] {
        if field == 0 {
            return [0; 6];
        }
        let min = 1 + ((field - 1) / 6) * 6;
        core::array::from_fn(|i| i + min)
    }

    pub fn move_checker(&mut self, color: &Color, cmove: CheckerMove) -> Result<(), Error> {
        self.remove_checker(color, cmove.from)?;
        self.add_checker(color, cmove.to)?;
        Ok(())
    }

    pub fn remove_checker(&mut self, color: &Color, field: Field) -> Result<(), Error> {
        if field == 0 {
            return Ok(());
        }
        let checker_color = self.get_checkers_color(field)?;
        if Some(color) != checker_color {
            return Err(Error::FieldInvalid);
        }
        let unit = match color {
            Color::White => 1,
            Color::Black => -1,
        };
        self.positions[field - 1] -= unit;
        Ok(())
    }

    pub fn add_checker(&mut self, color: &Color, field: Field) -> Result<(), Error> {
        // Sortie
        if field == 0 {
            return Ok(());
        }

        // let checker_color = self.get_checkers_color(field)?;
        let (count, checker_color) = self.get_field_checkers(field)?;
        // error if the case contains the other color
        if checker_color.is_some() && Some(color) != checker_color {
            return if count > 1 {
                Err(Error::FieldBlocked)
            } else {
                Err(Error::FieldBlockedByOne)
            };
        }
        let unit = match color {
            Color::White => 1,
            Color::Black => -1,
        };
        self.positions[field - 1] += unit;
        Ok(())
    }
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
        assert!(board.blocked(&Color::White, 0).is_ok());
        assert!(board.blocked(&Color::White, 28).is_err());
        Ok(())
    }

    #[test]
    fn blocked_otherplayer() -> Result<(), Error> {
        let board = Board::new();
        assert!(board.blocked(&Color::White, 24)?);
        Ok(())
    }

    #[test]
    fn blocked_notblocked() -> Result<(), Error> {
        let board = Board::new();
        assert!(!board.blocked(&Color::White, 6)?);
        Ok(())
    }

    #[test]
    fn set_field_blocked() {
        let mut board = Board::new();
        assert!(board.set(&Color::White, 24, 2).is_err());
    }

    #[test]
    fn set_wrong_field1() {
        let mut board = Board::new();
        assert!(board.set(&Color::White, 50, 2).is_err());
    }

    #[test]
    fn set_wrong_amount0() {
        let mut board = Board::new();
        assert!(board.set(&Color::White, 23, -3).is_err());
    }

    #[test]
    fn set_wrong_amount1() {
        let mut board = Board::new();
        assert!(board.set(&Color::White, 23, -3).is_err());
    }

    #[test]
    fn get_color_fields() {
        let board = Board::new();
        assert_eq!(board.get_color_fields(Color::White), vec![(1, 15)]);
        assert_eq!(board.get_color_fields(Color::Black), vec![(24, 15)]);
    }

    #[test]
    fn is_quarter_fillable() {
        let mut board = Board::new();
        board.set_positions([
            15, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, -15,
        ]);
        assert!(board.is_quarter_fillable(Color::Black, 1));
        assert!(!board.is_quarter_fillable(Color::Black, 12));
        assert!(board.is_quarter_fillable(Color::Black, 13));
        assert!(board.is_quarter_fillable(Color::Black, 24));
        assert!(board.is_quarter_fillable(Color::White, 1));
        assert!(board.is_quarter_fillable(Color::White, 12));
        assert!(!board.is_quarter_fillable(Color::White, 13));
        assert!(board.is_quarter_fillable(Color::White, 24));
        board.set_positions([
            5, 0, 0, 0, 0, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, -8, 0, 0, 0, 0, 0, -5,
        ]);
        assert!(board.is_quarter_fillable(Color::Black, 13));
        assert!(!board.is_quarter_fillable(Color::Black, 24));
        assert!(!board.is_quarter_fillable(Color::White, 1));
        assert!(board.is_quarter_fillable(Color::White, 12));
        board.set_positions([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, -12, 0, 0, 0, 0, 1, 0,
        ]);
        assert!(board.is_quarter_fillable(Color::Black, 16));
    }

    #[test]
    fn get_quarter_filling_candidate() {
        let mut board = Board::new();
        board.set_positions([
            3, 1, 2, 2, 3, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ]);
        assert_eq!(vec![2], board.get_quarter_filling_candidate(Color::White));
    }
}
