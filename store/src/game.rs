//! # Play a TricTrac Game
use crate::board::{Board, Move};
use crate::dice::{Dices, Roll};
use crate::player::{Color, Player, PlayerId};
use crate::Error;
use log::{error, info, trace, warn};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::{fmt, vec};

type TGPN = [u8];

/// The different stages a game can be in. (not to be confused with the entire "GameState")
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Stage {
    PreGame,
    InGame,
    Ended,
}

/// The different stages a game turn can be in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TurnStage {
    RollDice,
    MarkPoints,
    Move,
}

/// Represents a TricTrac game
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameState {
    pub stage: Stage,
    pub turn_stage: TurnStage,
    pub board: Board,
    pub active_player_id: PlayerId,
    pub players: HashMap<PlayerId, Player>,
    pub history: Vec<GameEvent>,
    /// last dice pair rolled
    pub dices: Dices,
    /// true if player needs to roll first
    roll_first: bool,
}

// implement Display trait
impl fmt::Display for GameState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut s = String::new();
        s.push_str(&format!("Dices: {:?}\n", self.dices));
        // s.push_str(&format!("Who plays: {}\n", self.who_plays().map(|player| &player.name ).unwrap_or("")));
        s.push_str(&format!("Board: {:?}\n", self.board));
        write!(f, "{}", s)
    }
}

impl Default for GameState {
    fn default() -> Self {
        Self {
            stage: Stage::PreGame,
            turn_stage: TurnStage::RollDice,
            board: Board::default(),
            active_player_id: 0,
            players: HashMap::new(),
            history: Vec::new(),
            dices: Dices::default(),
            roll_first: true,
        }
    }
}

impl GameState {
    /// Create a new default game
    pub fn new() -> Self {
        GameState::default()
    }

    /// Format to TGPN notation (Tables games position notation)
    // fn toTGPN(&self, f: &mut fmt::Formatter) -> TGPN {
    pub fn toTGPN(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut s = String::new();
        // s.push_str(&format!("Dices: {:?}\n", self.dices));
        write!(f, "{}", s)
    }

    /// Calculate game state id :
    pub fn to_string_id(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Pieces placement -> 77 bits (24 + 23 + 30 max)
        let mut pos_bits = self.board.toGnupgPosId();

        // active player -> 1 bit
        // white : 0 (false)
        // black : 1 (true)
        pos_bits.push(
            self.who_plays()
                .map(|player| player.color == Color::Black)
                .unwrap_or(false), // White by default
        );

        // step  -> 2 bits
        //   * roll dice
        //   * mark points (jeton & fichet) & set bredouille markers (3rd jeton & pavillon)
        //   * move pieces
        let mut step_bits = match self.turn_stage {
            TurnStage::RollDice => [false, false],
            TurnStage::MarkPoints => [false, true],
            TurnStage::Move => [true, false],
        };
        pos_bits.append(&mut step_bits.into());

        // dice roll -> 4 bits
        let mut dice_bits = match self.dices {
            TurnStage::RollDice => [false, false],
            TurnStage::MarkPoints => [false, true],
            TurnStage::Move => [true, false],
        };
        pos_bits.append(&mut step_bits.into());
        // points 10bits x2 joueurs = 20bits
        //   * points -> 4bits
        //   * trous -> 4bits
        //   * bredouille possible 1bit
        //   * grande bredouille possible 1bit

        let mut s = String::new();
        // s.push_str(&format!("Dices: {:?}\n", self.dices));
        write!(f, "{}", s)
    }

    pub fn who_plays(&self) -> Option<&Player> {
        self.players.get(&self.active_player_id)
    }

    pub fn switch_active_player(&mut self) {
        let other_player_id = self
            .players
            .iter()
            .filter(|(id, _player)| **id != self.active_player_id)
            .map(|(id, _player)| *id)
            .next();
        self.active_player_id = other_player_id.unwrap_or(0);
    }

    pub fn player_id_by_color(&self, color: Color) -> Option<&PlayerId> {
        self.players
            .iter()
            .filter(|(_id, player)| player.color == color)
            .map(|(id, _player)| id)
            .next()
    }

    pub fn player_id(&self, player: &Player) -> Option<&PlayerId> {
        self.players
            .iter()
            .filter(|(_id, candidate)| player.color == candidate.color)
            .map(|(id, _candidate)| id)
            .next()
    }

    /// Determines whether an event is valid considering the current GameState
    pub fn validate(&self, event: &GameEvent) -> bool {
        use GameEvent::*;
        match event {
            BeginGame { goes_first } => {
                // Check that the player supposed to go first exists
                if !self.players.contains_key(goes_first) {
                    return false;
                }

                // Check that the game hasn't started yet. (we don't want to double start a game)
                if self.stage != Stage::PreGame {
                    return false;
                }
            }
            EndGame { reason } => match reason {
                EndGameReason::PlayerWon { winner: _ } => {
                    // Check that the game has started before someone wins it
                    if self.stage != Stage::InGame {
                        return false;
                    }
                }
                _ => {}
            },
            PlayerJoined { player_id, name: _ } => {
                // Check that there isn't another player with the same id
                if self.players.contains_key(player_id) {
                    return false;
                }
            }
            PlayerDisconnected { player_id } => {
                // Check player exists
                if !self.players.contains_key(player_id) {
                    return false;
                }
            }
            Roll { player_id } => {
                // Check player exists
                if !self.players.contains_key(player_id) {
                    return false;
                }
                // Check player is currently the one making their move
                if self.active_player_id != *player_id {
                    return false;
                }
            }
            Move {
                player_id,
                from,
                to,
            } => {
                // Check player exists
                if !self.players.contains_key(player_id) {
                    error!("Player {} unknown", player_id);
                    return false;
                }
                // Check player is currently the one making their move
                if self.active_player_id != *player_id {
                    error!("Player not active : {}", self.active_player_id);
                    return false;
                }

                // Check that the tile index is inside the board
                if *to > 23 {
                    return false;
                }
                if *from > 23 {
                    return false;
                }
            }
        }

        // We couldn't find anything wrong with the event so it must be good
        true
    }

    /// Consumes an event, modifying the GameState and adding the event to its history
    /// NOTE: consume assumes the event to have already been validated and will accept *any* event passed to it
    pub fn consume(&mut self, valid_event: &GameEvent) {
        use GameEvent::*;
        match valid_event {
            BeginGame { goes_first } => {
                self.active_player_id = *goes_first;
                self.stage = Stage::InGame;
            }
            EndGame { reason: _ } => self.stage = Stage::Ended,
            PlayerJoined { player_id, name } => {
                let color = if self.players.len() > 0 {
                    Color::White
                } else {
                    Color::Black
                };
                self.players.insert(
                    *player_id,
                    Player {
                        name: name.to_string(),
                        color,
                    },
                );
            }
            PlayerDisconnected { player_id } => {
                self.players.remove(player_id);
            }
            Roll { player_id } => {}
            Move {
                player_id,
                from,
                to,
            } => {
                let player = self.players.get(player_id).unwrap();
                self.board.set(player, *from, 0 as i8).unwrap();
                self.board.set(player, *to, 1 as i8).unwrap();
                self.active_player_id = self
                    .players
                    .keys()
                    .find(|id| *id != player_id)
                    .unwrap()
                    .clone();
            }
        }

        self.history.push(valid_event.clone());
    }

    /// Determines if someone has won the game
    pub fn determine_winner(&self) -> Option<PlayerId> {
        None
    }
}

/// The reasons why a game could end
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Deserialize)]
pub enum EndGameReason {
    // In tic tac toe it doesn't make sense to keep playing when one of the players disconnect.
    // Note that it might make sense to keep playing in some other game (like Team Fight Tactics for instance).
    PlayerLeft { player_id: PlayerId },
    PlayerWon { winner: PlayerId },
}

/// An event that progresses the GameState forward
#[derive(Debug, Clone, Serialize, PartialEq, Deserialize)]
pub enum GameEvent {
    BeginGame {
        goes_first: PlayerId,
    },
    EndGame {
        reason: EndGameReason,
    },
    PlayerJoined {
        player_id: PlayerId,
        name: String,
    },
    PlayerDisconnected {
        player_id: PlayerId,
    },
    Roll {
        player_id: PlayerId,
    },
    Move {
        player_id: PlayerId,
        from: usize,
        to: usize,
    },
}

impl Roll for GameState {
    fn roll(&mut self) -> Result<&mut Self, Error> {
        if !self.dices.consumed.0 && !self.dices.consumed.1 {
            return Err(Error::MoveFirst);
        }

        self.dices = self.dices.roll();
        if self.who_plays().is_none() {
            let diff = self.dices.values.0 - self.dices.values.1;
            let active_color = if diff < 0 { Color::Black } else { Color::White };
            let color_player_id = self.player_id_by_color(active_color);
            if color_player_id.is_some() {
                self.active_player_id = *color_player_id.unwrap();
            }
        }
        Ok(self)
    }
}

impl Move for GameState {
    fn move_checker(&mut self, player: &Player, dice: u8, from: usize) -> Result<&mut Self, Error> {
        // check if move is permitted
        let _ = self.move_permitted(player, dice)?;

        // check if the dice value has been consumed
        if (dice == self.dices.values.0 && self.dices.consumed.0)
            || (dice == self.dices.values.1 && self.dices.consumed.1)
        {
            return Err(Error::MoveInvalid);
        }

        // remove checker from old position
        self.board.set(player, from, -1)?;

        // move checker to new position, in case it is reaching the off position, set it off
        let new_position = from as i8 - dice as i8;
        if new_position < 0 {
            self.board.set_off(player, 1)?;
        } else {
            self.board.set(player, new_position as usize, 1)?;
        }

        // set dice value to consumed
        if dice == self.dices.values.0 && !self.dices.consumed.0 {
            self.dices.consumed.0 = true;
        } else if dice == self.dices.values.1 && !self.dices.consumed.1 {
            self.dices.consumed.1 = true;
        }

        // switch to other player if all dices have been consumed
        if self.dices.consumed.0 && self.dices.consumed.1 {
            self.switch_active_player();
            self.roll_first = true;
        }

        Ok(self)
    }

    /// Implements checks to validate if the player is allowed to move
    fn move_permitted(&mut self, player: &Player, dice: u8) -> Result<&mut Self, Error> {
        let maybe_player_id = self.player_id(&player);
        // check if player is allowed to move
        if maybe_player_id != Some(&self.active_player_id) {
            return Err(Error::NotYourTurn);
        }

        // if player is nobody, you can not play and have to roll first
        if maybe_player_id.is_none() {
            return Err(Error::RollFirst);
        }

        // check if player has to roll first
        if self.roll_first {
            return Err(Error::RollFirst);
        }

        // check if dice value has actually been rolled
        if dice != self.dices.values.0 && dice != self.dices.values.1 {
            return Err(Error::DiceInvalid);
        }

        Ok(self)
    }
}
