//! # Play a TricTrac Game
use crate::board::{Board, CheckerMove, Field, Move};
use crate::dice::{Dice, DiceRoller, Roll};
use crate::player::{Color, Player, PlayerId};
use crate::Error;
use log::{error, info};
use std::cmp;

// use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::{fmt, str};

use base64::{engine::general_purpose, Engine as _};

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
    pub dice: Dice,
    /// true if player needs to roll first
    roll_first: bool,
}

// implement Display trait
impl fmt::Display for GameState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut s = String::new();
        s.push_str(&format!("Dice: {:?}\n", self.dice));
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
            dice: Dice::default(),
            roll_first: true,
        }
    }
}

impl GameState {
    /// Create a new default game
    pub fn new() -> Self {
        GameState::default()
    }

    // -------------------------------------------------------------------------
    //                        accessors
    // -------------------------------------------------------------------------

    /// Calculate game state id :
    pub fn to_string_id(&self) -> String {
        // Pieces placement -> 77 bits (24 + 23 + 30 max)
        let mut pos_bits = self.board.to_gnupg_pos_id();

        // active player -> 1 bit
        // white : 0 (false)
        // black : 1 (true)
        pos_bits.push(
            self.who_plays()
                .map(|player| {
                    if player.color == Color::Black {
                        '1'
                    } else {
                        '0'
                    }
                })
                .unwrap_or('0'), // White by default
        );

        // step  -> 2 bits
        let step_bits = match self.turn_stage {
            TurnStage::RollDice => "01",
            TurnStage::MarkPoints => "01",
            TurnStage::Move => "10",
        };
        pos_bits.push_str(step_bits);

        // dice roll -> 6 bits
        let dice_bits = self.dice.to_bits_string();
        pos_bits.push_str(&dice_bits);

        // points 10bits x2 joueurs = 20bits
        let white_bits = self.get_white_player().unwrap().to_bits_string();
        let black_bits = self.get_black_player().unwrap().to_bits_string();
        pos_bits.push_str(&white_bits);
        pos_bits.push_str(&black_bits);

        pos_bits = format!("{:0>108}", pos_bits);
        // println!("{}", pos_bits);
        let pos_u8 = pos_bits
            .as_bytes()
            .chunks(6)
            .map(|chunk| str::from_utf8(chunk).unwrap())
            .map(|chunk| u8::from_str_radix(chunk, 2).unwrap())
            .collect::<Vec<u8>>();
        general_purpose::STANDARD.encode(pos_u8)
    }

    pub fn who_plays(&self) -> Option<&Player> {
        self.players.get(&self.active_player_id)
    }

    pub fn get_white_player(&self) -> Option<&Player> {
        self.players
            .iter()
            .filter(|(_id, player)| player.color == Color::White)
            .map(|(_id, player)| player)
            .next()
    }

    pub fn get_black_player(&self) -> Option<&Player> {
        self.players
            .iter()
            .filter(|(_id, player)| player.color == Color::Black)
            .map(|(_id, player)| player)
            .next()
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

    pub fn player_color_by_id(&self, player_id: &PlayerId) -> Option<Color> {
        self.players
            .iter()
            .filter(|(id, _)| *id == player_id)
            .map(|(_, player)| player.color)
            .next()
    }

    // ----------------------------------------------------------------------------------
    //                          Rules checks
    // ----------------------------------------------------------------------------------

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
            Roll { player_id } | RollResult { player_id, dice: _ } => {
                // Check player exists
                if !self.players.contains_key(player_id) {
                    return false;
                }
                // Check player is currently the one making their move
                if self.active_player_id != *player_id {
                    return false;
                }
            }
            Mark { player_id, points } => {
                // Check player exists
                if !self.players.contains_key(player_id) {
                    return false;
                }
                // Check player is currently the one making their move
                if self.active_player_id != *player_id {
                    return false;
                }
            }
            Move { player_id, moves } => {
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
                let color = &self.players[player_id].color;

                // Check moves possibles on the board
                if !self.moves_possible(color, moves) {
                    return false;
                }

                // Check moves conforms to the dice
                if !self.moves_follows_dices(color, moves) {
                    return false;
                }

                // Check move is allowed by the rules (to desactivate when playing with schools)
                if !self.moves_allowed(color, moves) {
                    return false;
                }
            }
        }

        // We couldn't find anything wrong with the event so it must be good
        true
    }

    fn moves_possible(&self, color: &Color, moves: &(CheckerMove, CheckerMove)) -> bool {
        // Check move is physically possible
        if !self.board.move_possible(color, &moves.0) {
            return false;
        }

        // Chained_move : "Tout d'une"
        let chained_move = moves.0.chain(moves.1);
        if chained_move.is_ok() {
            if !self.board.move_possible(color, &chained_move.unwrap()) {
                return false;
            }
        } else if !self.board.move_possible(color, &moves.1) {
            return false;
        }
        true
    }

    fn moves_follows_dices(&self, color: &Color, moves: &(CheckerMove, CheckerMove)) -> bool {
        let (dice1, dice2) = self.dice.values;
        let (move1, move2): &(CheckerMove, CheckerMove) = moves.into();
        let dist1 = (move1.get_to() - move1.get_from()) as u8;
        let dist2 = (move2.get_to() - move2.get_from()) as u8;
        print!("{}, {}, {}, {}", dist1, dist2, dice1, dice2);
        // basic : same number
        if cmp::min(dist1, dist2) != cmp::min(dice1, dice2)
            || cmp::max(dist1, dist2) != cmp::max(dice1, dice2)
        {
            return false;
        }
        // prise de coin par puissance
        // sorties
        // no rule was broken
        true
    }

    fn moves_allowed(&self, color: &Color, moves: &(CheckerMove, CheckerMove)) -> bool {
        // ------- corner rules ----------
        let corner_field: Field = self.board.get_color_corner(color);
        let (corner_count, _color) = self.board.get_field_checkers(corner_field).unwrap();
        let (from0, to0, from1, to1) = (
            moves.0.get_from(),
            moves.0.get_to(),
            moves.1.get_from(),
            moves.1.get_to(),
        );
        // 2 checkers must go at the same time on an empty corner
        if (to0 == corner_field || to1 == corner_field) && (to0 != to1) && corner_count == 0 {
            return false;
        }

        // the lat 2 checkers of a corner must leave at the same time
        if (from0 == corner_field || from1 == corner_field) && (from0 != from1) && corner_count == 2
        {
            return false;
        }

        // ------- exit rules ----------
        // -- toutes les dames doivent être dans le jan de retour
        // -- si on peut sortir, on doit sortir
        // -- priorité :
        //  - dame se trouvant sur la flêche correspondant au dé
        //  - dame se trouvant plus loin de la sortie que la flêche (point défaillant)
        //  - dame se trouvant plus près que la flêche (point exédant)

        // --- cadran rempli si possible ----
        // --- interdit de jouer dans cadran que l'adversaire peut encore remplir ----
        // no rule was broken
        true
    }

    // ----------------------------------------------------------------------------------
    //                   State updates
    // ----------------------------------------------------------------------------------

    pub fn init_player(&mut self, player_name: &str) -> Option<PlayerId> {
        if self.players.len() > 2 {
            println!("more than two players");
            return None;
        }

        let player_id = self.players.len() + 1;
        println!("player_id {}", player_id);
        let color = if player_id == 1 {
            Color::White
        } else {
            Color::Black
        };
        let player = Player::new(player_name.into(), color);
        self.players.insert(player_id as PlayerId, player);
        Some(player_id as PlayerId)
    }

    fn add_player(&mut self, player_id: PlayerId, player: Player) {
        self.players.insert(player_id, player);
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
    /// Consumes an event, modifying the GameState and adding the event to its history
    /// NOTE: consume assumes the event to have already been validated and will accept *any* event passed to it
    pub fn consume(&mut self, valid_event: &GameEvent) {
        use GameEvent::*;
        match valid_event {
            BeginGame { goes_first } => {
                self.active_player_id = *goes_first;
                // if self.who_plays().is_none() {
                //     let active_color = match self.dice.coin() {
                //         false => Color::Black,
                //         true => Color::White,
                //     };
                //     let color_player_id = self.player_id_by_color(active_color);
                //     if color_player_id.is_some() {
                //         self.active_player_id = *color_player_id.unwrap();
                //     }
                // }
                self.stage = Stage::InGame;
                self.turn_stage = TurnStage::RollDice;
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
                        holes: 0,
                        points: 0,
                        can_bredouille: true,
                        can_big_bredouille: true,
                    },
                );
            }
            PlayerDisconnected { player_id } => {
                self.players.remove(player_id);
            }
            Roll { player_id: _ } => {}
            RollResult { player_id: _, dice } => {
                self.dice = *dice;
                self.turn_stage = TurnStage::MarkPoints;
            }
            Mark { player_id, points } => {
                self.mark_points(*player_id, *points);
                if self.stage != Stage::Ended {
                    self.turn_stage = TurnStage::Move;
                }
            }
            Move { player_id, moves } => {
                let player = self.players.get(player_id).unwrap();
                self.board.move_checker(&player.color, moves.0).unwrap();
                self.board.move_checker(&player.color, moves.1).unwrap();
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

    fn mark_points(&mut self, player_id: PlayerId, points: u8) {
        todo!()
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
    RollResult {
        player_id: PlayerId,
        dice: Dice,
    },
    Mark {
        player_id: PlayerId,
        points: u8,
    },
    Move {
        player_id: PlayerId,
        moves: (CheckerMove, CheckerMove),
    },
}

impl Move for GameState {
    fn move_checker(&mut self, player: &Player, dice: u8, from: usize) -> Result<&mut Self, Error> {
        // check if move is permitted
        let _ = self.move_permitted(player, dice)?;

        // remove checker from old position
        self.board.set(&player.color, from, -1)?;

        // move checker to new position, in case it is reaching the off position, set it off
        let new_position = from as i8 - dice as i8;
        if new_position < 0 {
            // self.board.set_off(player, 1)?;
        } else {
            // self.board.set(player, new_position as usize, 1)?;
        }

        // switch to other player if all dice have been consumed
        self.switch_active_player();
        self.roll_first = true;

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
        if dice != self.dice.values.0 && dice != self.dice.values.1 {
            return Err(Error::DiceInvalid);
        }

        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_string_id() {
        let mut state = GameState::default();
        state.add_player(1, Player::new("player1".into(), Color::White));
        state.add_player(2, Player::new("player2".into(), Color::Black));
        let string_id = state.to_string_id();
        // println!("string_id : {}", string_id);
        assert!(string_id == "Dz8+AAAAAT8/MAAAAAQAADAD");
    }

    #[test]
    fn test_moves_possible() {
        let mut state = GameState::default();
        let player1 = Player::new("player1".into(), Color::White);
        let player_id = 1;
        state.add_player(player_id, player1);
        state.add_player(2, Player::new("player2".into(), Color::Black));
        state.consume(&GameEvent::BeginGame {
            goes_first: player_id,
        });

        // Chained moves
        let moves = (
            CheckerMove::new(1, 5).unwrap(),
            CheckerMove::new(5, 9).unwrap(),
        );
        assert!(state.moves_possible(&Color::White, &moves));

        // not chained moves
        let moves = (
            CheckerMove::new(1, 5).unwrap(),
            CheckerMove::new(6, 9).unwrap(),
        );
        assert!(!state.moves_possible(&Color::White, &moves));
    }

    #[test]
    fn test_moves_follow_dices() {
        let mut state = GameState::default();
        let player1 = Player::new("player1".into(), Color::White);
        let player_id = 1;
        state.add_player(player_id, player1);
        state.add_player(2, Player::new("player2".into(), Color::Black));
        state.consume(&GameEvent::BeginGame {
            goes_first: player_id,
        });
        state.consume(&GameEvent::Roll { player_id });
        let dice = state.dice.values;
        let moves = (
            CheckerMove::new(1, (1 + dice.0).into()).unwrap(),
            CheckerMove::new((1 + dice.0).into(), (1 + dice.0 + dice.1).into()).unwrap(),
        );
        assert!(state.moves_follows_dices(&Color::White, &moves));

        let badmoves = (
            CheckerMove::new(1, (2 + dice.0).into()).unwrap(),
            CheckerMove::new((1 + dice.0).into(), (1 + dice.0 + dice.1).into()).unwrap(),
        );
        assert!(!state.moves_follows_dices(&Color::White, &badmoves));
    }
}
