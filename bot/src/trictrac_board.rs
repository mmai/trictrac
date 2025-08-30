// https://docs.rs/board-game/ implementation
use crate::training_common::{get_valid_actions, TrictracAction};
use board_game::board::{
    Board as BoardGameBoard, BoardDone, BoardMoves, Outcome, PlayError, Player as BoardGamePlayer,
};
use board_game::impl_unit_symmetry_board;
use internal_iterator::InternalIterator;
use std::fmt;
use std::ops::ControlFlow;
use store::Color;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrictracBoard(crate::GameState);

impl Default for TrictracBoard {
    fn default() -> Self {
        TrictracBoard(crate::GameState::new_with_players("white", "black"))
    }
}

impl fmt::Display for TrictracBoard {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl_unit_symmetry_board!(TrictracBoard);

impl BoardGameBoard for TrictracBoard {
    // impl TrictracBoard {
    type Move = TrictracAction;

    fn next_player(&self) -> BoardGamePlayer {
        self.0
            .who_plays()
            .map(|p| {
                if p.color == Color::Black {
                    BoardGamePlayer::B
                } else {
                    BoardGamePlayer::A
                }
            })
            .unwrap_or(BoardGamePlayer::A)
    }

    fn is_available_move(&self, mv: Self::Move) -> Result<bool, BoardDone> {
        self.check_done()?;
        let is_valid = mv
            .to_event(&self.0)
            .map(|evt| self.0.validate(&evt))
            .unwrap_or(false);
        Ok(is_valid)
    }

    fn play(&mut self, mv: Self::Move) -> Result<(), PlayError> {
        self.check_can_play(mv)?;
        self.0.consume(&mv.to_event(&self.0).unwrap());
        Ok(())
    }

    fn outcome(&self) -> Option<Outcome> {
        if self.0.stage == crate::Stage::Ended {
            self.0.determine_winner().map(|player_id| {
                Outcome::WonBy(if player_id == 1 {
                    BoardGamePlayer::A
                } else {
                    BoardGamePlayer::B
                })
            })
        } else {
            None
        }
    }

    fn can_lose_after_move() -> bool {
        true
    }
}

impl<'a> BoardMoves<'a, TrictracBoard> for TrictracBoard {
    type AllMovesIterator = TrictracAllMovesIterator;
    type AvailableMovesIterator = TrictracAvailableMovesIterator<'a>;

    fn all_possible_moves() -> Self::AllMovesIterator {
        TrictracAllMovesIterator::default()
    }

    fn available_moves(&'a self) -> Result<Self::AvailableMovesIterator, BoardDone> {
        TrictracAvailableMovesIterator::new(self)
    }
}

#[derive(Debug, Clone)]
pub struct TrictracAllMovesIterator;

impl Default for TrictracAllMovesIterator {
    fn default() -> Self {
        TrictracAllMovesIterator
    }
}

impl InternalIterator for TrictracAllMovesIterator {
    type Item = TrictracAction;

    fn try_for_each<R, F: FnMut(Self::Item) -> ControlFlow<R>>(self, mut f: F) -> ControlFlow<R> {
        f(TrictracAction::Roll)?;
        f(TrictracAction::Go)?;
        for dice_order in [false, true] {
            for checker1 in 0..16 {
                for checker2 in 0..16 {
                    f(TrictracAction::Move {
                        dice_order,
                        checker1,
                        checker2,
                    })?;
                }
            }
        }

        ControlFlow::Continue(())
    }
}

#[derive(Debug, Clone)]
pub struct TrictracAvailableMovesIterator<'a> {
    board: &'a TrictracBoard,
}

impl<'a> TrictracAvailableMovesIterator<'a> {
    pub fn new(board: &'a TrictracBoard) -> Result<Self, BoardDone> {
        board.check_done()?;
        Ok(TrictracAvailableMovesIterator { board })
    }

    pub fn board(&self) -> &'a TrictracBoard {
        self.board
    }
}

impl InternalIterator for TrictracAvailableMovesIterator<'_> {
    type Item = TrictracAction;

    fn try_for_each<R, F>(self, f: F) -> ControlFlow<R>
    where
        F: FnMut(Self::Item) -> ControlFlow<R>,
    {
        get_valid_actions(&self.board.0).into_iter().try_for_each(f)
    }
}
