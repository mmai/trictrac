mod game;
mod game_rules_moves;
pub use game_rules_moves::{MoveError, MoveRules};
mod game_rules_points;
pub use game::{EndGameReason, GameEvent, GameState, Stage, TurnStage};
pub use game_rules_points::{Jan, PointsRules};

mod player;
pub use player::{Color, Player, PlayerId};

mod error;
pub use error::Error;

mod board;
pub use board::{Board, CheckerMove};

mod dice;
pub use dice::{Dice, DiceRoller};

pub mod training_common;
