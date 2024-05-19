//! # Play a TricTrac Game
use crate::board::{Board, CheckerMove, Field, EMPTY_MOVE};
use crate::dice::Dice;
use crate::player::{Color, Player, PlayerId};
use log::error;
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
    RollWaiting,
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
        s.push_str(&format!(
            "Stage: {:?} / {:?}\n",
            self.stage, self.turn_stage
        ));
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
            TurnStage::RollWaiting => "00",
            TurnStage::RollDice => "01",
            TurnStage::MarkPoints => "10",
            TurnStage::Move => "11",
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
            EndGame { reason } => {
                if let EndGameReason::PlayerWon { winner: _ } = reason {
                    // Check that the game has started before someone wins it
                    if self.stage != Stage::InGame {
                        return false;
                    }
                }
            }
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
            Mark {
                player_id,
                points: _,
            } => {
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
        if let Ok(chained_move) = moves.0.chain(moves.1) {
            if !self.board.move_possible(color, &chained_move) {
                return false;
            }
        } else if !self.board.move_possible(color, &moves.1) {
            return false;
        }
        true
    }

    fn get_move_compatible_dices(&self, color: &Color, cmove: &CheckerMove) -> Vec<u8> {
        let (dice1, dice2) = self.dice.values;

        let mut move_dices = Vec::new();
        if cmove.get_to() == 0 {
            // handle empty move (0, 0) only one checker left, exiting with the first die.
            if cmove.get_from() == 0 {
                move_dices.push(dice1);
                move_dices.push(dice2);
                return move_dices;
            }

            // Exits
            let min_dist = match color {
                Color::White => 25 - cmove.get_from(),
                Color::Black => cmove.get_from(),
            };
            if dice1 as usize >= min_dist {
                move_dices.push(dice1);
            }
            if dice2 as usize >= min_dist {
                move_dices.push(dice2);
            }
        } else {
            let dist = (cmove.get_to() as i8 - cmove.get_from() as i8).unsigned_abs();
            if dice1 == dist {
                move_dices.push(dice1);
            }
            if dice2 == dist {
                move_dices.push(dice2);
            }
        }
        move_dices
    }

    fn moves_follows_dices(&self, color: &Color, moves: &(CheckerMove, CheckerMove)) -> bool {
        // Prise de coin par puissance
        if self.is_move_by_puissance(color, moves) {
            return true;
        }

        let (dice1, dice2) = self.dice.values;
        let (move1, move2): &(CheckerMove, CheckerMove) = moves;

        let move1_dices = self.get_move_compatible_dices(color, move1);
        if move1_dices.is_empty() {
            return false;
        }
        let move2_dices = self.get_move_compatible_dices(color, move2);
        if move2_dices.is_empty() {
            return false;
        }
        if move1_dices.len() == 1
            && move2_dices.len() == 1
            && move1_dices[0] == move2_dices[0]
            && dice1 != dice2
        {
            return false;
        }

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

        // the last 2 checkers of a corner must leave at the same time
        if (from0 == corner_field || from1 == corner_field) && (from0 != from1) && corner_count == 2
        {
            return false;
        }

        if self.is_move_by_puissance(color, moves) && self.can_take_corner_by_effect(color) {
            return false;
        }

        // check exit rules
        if moves.0.get_to() == 0 || moves.1.get_to() == 0 {
            // toutes les dames doivent être dans le jan de retour
            let has_outsiders = !self
                .board
                .get_color_fields(*color)
                .iter()
                .filter(|(field, _count)| {
                    (*color == Color::White && *field < 19)
                        || (*color == Color::Black && *field > 6)
                })
                .collect::<Vec<&(usize, i8)>>()
                .is_empty();
            if has_outsiders {
                return false;
            }

            // toutes les sorties directes sont autorisées, ainsi que les nombre défaillants
            let possible_moves_sequences = self.get_possible_moves_sequences(color);
            if !possible_moves_sequences.contains(moves) {
                // À ce stade au moins un des déplacements concerne un nombre en excédant
                // - si d'autres séquences de mouvements sans nombre en excédant étaient possibles, on
                // refuse cette séquence
                if !possible_moves_sequences.is_empty() {
                    return false;
                }

                // - la dame choisie doit être la plus éloignée de la sortie
                let mut checkers = self.board.get_color_fields(*color);
                checkers.sort_by(|a, b| {
                    if *color == Color::White {
                        b.0.cmp(&a.0)
                    } else {
                        a.0.cmp(&b.0)
                    }
                });
                let mut farthest = if *color == Color::White { 24 } else { 1 };
                let mut next_farthest = if *color == Color::White { 24 } else { 1 };
                let mut has_two_checkers = false;
                if let Some((field, count)) = checkers.first() {
                    farthest = *field;
                    if *count > 1 {
                        next_farthest = *field;
                        has_two_checkers = true;
                    } else if let Some((field, _count)) = checkers.get(1) {
                        next_farthest = *field;
                        has_two_checkers = true;
                    }
                }

                // s'il reste au moins deux dames, on vérifie que les plus éloignées soint choisies
                if has_two_checkers {
                    if moves.0.get_to() == 0 && moves.1.get_to() == 0 {
                        // Deux coups sortants en excédant
                        if *color == Color::White {
                            if cmp::max(moves.0.get_from(), moves.1.get_from()) > next_farthest {
                                return false;
                            }
                        } else if cmp::min(moves.0.get_from(), moves.1.get_from()) < next_farthest {
                            return false;
                        }
                    } else {
                        // Un seul coup sortant en excédant le coup sortant doit concerner la plus éloignée du bord
                        let exit_move_field = if moves.0.get_to() == 0 {
                            moves.0.get_from()
                        } else {
                            moves.1.get_from()
                        };
                        if exit_move_field != farthest {
                            return false;
                        }
                    }
                }
            }
        }

        // --- interdit de jouer dans cadran que l'adversaire peut encore remplir ----
        let farthest = if *color == Color::White {
            cmp::max(moves.0.get_to(), moves.1.get_to())
        } else {
            cmp::min(moves.0.get_to(), moves.1.get_to())
        };
        let in_opponent_side = if *color == Color::White {
            farthest > 12
        } else {
            farthest < 13
        };

        if in_opponent_side
            && self
                .board
                .is_quarter_fillable(color.opponent_color(), farthest)
        {
            return false;
        }

        // --- remplir cadran si possible & conserver cadran rempli si possible ----
        let filling_moves_sequences = self.get_quarter_filling_moves_sequences(color);
        if !filling_moves_sequences.contains(moves) && !filling_moves_sequences.is_empty() {
            return false;
        }
        // no rule was broken
        true
    }

    fn get_possible_moves_sequences(&self, color: &Color) -> Vec<(CheckerMove, CheckerMove)> {
        let (dice1, dice2) = self.dice.values;
        let mut moves_seqs = self.get_possible_moves_sequences_by_dices(color, dice1, dice2);
        let mut moves_seqs_order2 = self.get_possible_moves_sequences_by_dices(color, dice1, dice2);
        moves_seqs.append(&mut moves_seqs_order2);
        moves_seqs
    }

    fn get_quarter_filling_moves_sequences(
        &self,
        color: &Color,
    ) -> Vec<(CheckerMove, CheckerMove)> {
        let mut moves_seqs = Vec::new();
        for moves in self.get_possible_moves_sequences(color) {
            let mut board = self.board.clone();
            board.move_checker(color, moves.0).unwrap();
            board.move_checker(color, moves.1).unwrap();
            if board.any_quarter_filled(*color) {
                moves_seqs.push(moves);
            }
        }
        moves_seqs
    }

    fn get_possible_moves_sequences_by_dices(
        &self,
        color: &Color,
        dice1: u8,
        dice2: u8,
    ) -> Vec<(CheckerMove, CheckerMove)> {
        let mut moves_seqs = Vec::new();
        for first_move in self.board.get_possible_moves(*color, dice1, false) {
            let mut board2 = self.board.clone();
            if board2.move_checker(color, first_move).is_err() {
                println!("err move");
                continue;
            }
            if board2.get_color_fields(*color).is_empty() {
                // no checkers left : empty move
                println!("empty move");
                moves_seqs.push((first_move, EMPTY_MOVE));
            } else {
                for second_move in board2.get_possible_moves(*color, dice2, false) {
                    moves_seqs.push((first_move, second_move));
                }
            }
        }
        moves_seqs
    }

    fn get_direct_exit_moves(&self, color: &Color) -> Vec<CheckerMove> {
        let mut moves = Vec::new();
        let (dice1, dice2) = self.dice.values;

        // sorties directes simples
        let (field1_candidate, field2_candidate) = if color == &Color::White {
            (25 - dice1 as usize, 25 - dice2 as usize)
        } else {
            (dice1 as usize, dice2 as usize)
        };
        let (count1, col1) = self.board.get_field_checkers(field1_candidate).unwrap();
        let (count2, col2) = self.board.get_field_checkers(field2_candidate).unwrap();
        if count1 > 0 {
            moves.push(CheckerMove::new(field1_candidate, 0).unwrap());
        }
        if dice2 != dice1 {
            if count2 > 0 {
                moves.push(CheckerMove::new(field2_candidate, 0).unwrap());
            }
        } else if count1 > 1 {
            // doublet et deux dames disponibles
            moves.push(CheckerMove::new(field1_candidate, 0).unwrap());
        }

        // sortie directe tout d'une
        let fieldall_candidate = if color == &Color::White {
            25 - dice1 - dice2
        } else {
            dice1 + dice2
        } as usize;
        let (countall, _col) = self.board.get_field_checkers(fieldall_candidate).unwrap();
        if countall > 0 {
            if col1.is_none() || col1 == Some(color) {
                moves.push(CheckerMove::new(fieldall_candidate, field1_candidate).unwrap());
                moves.push(CheckerMove::new(field1_candidate, 0).unwrap());
            }
            if col2.is_none() || col2 == Some(color) {
                moves.push(CheckerMove::new(fieldall_candidate, field2_candidate).unwrap());
                moves.push(CheckerMove::new(field2_candidate, 0).unwrap());
            }
        }
        moves
    }

    fn is_move_by_puissance(&self, color: &Color, moves: &(CheckerMove, CheckerMove)) -> bool {
        let (dice1, dice2) = self.dice.values;
        let (move1, move2): &(CheckerMove, CheckerMove) = moves;
        let dist1 = (move1.get_to() as i8 - move1.get_from() as i8).unsigned_abs();
        let dist2 = (move2.get_to() as i8 - move2.get_from() as i8).unsigned_abs();

        // Both corners must be empty
        let (count1, _color) = self.board.get_field_checkers(12).unwrap();
        let (count2, _color2) = self.board.get_field_checkers(13).unwrap();
        if count1 > 0 || count2 > 0 {
            return false;
        }

        move1.get_to() == move2.get_to()
            && move1.get_to() == self.board.get_color_corner(color)
            && ((*color == Color::White
                && cmp::min(dist1, dist2) == cmp::min(dice1, dice2) - 1
                && cmp::max(dist1, dist2) == cmp::max(dice1, dice2) - 1)
                || (*color == Color::Black
                    && cmp::min(dist1, dist2) == cmp::min(dice1, dice2) + 1
                    && cmp::max(dist1, dist2) == cmp::max(dice1, dice2) + 1))
    }

    fn can_take_corner_by_effect(&self, color: &Color) -> bool {
        // return false if corner already taken
        let corner_field: Field = self.board.get_color_corner(color);
        let (count, _col) = self.board.get_field_checkers(corner_field).unwrap();
        if count > 0 {
            return false;
        }

        let (dice1, dice2) = self.dice.values;
        let (field1, field2) = match color {
            Color::White => (12 - dice1, 12 - dice2),
            Color::Black => (13 + dice1, 13 + dice2),
        };
        let res1 = self.board.get_field_checkers(field1.into());
        let res2 = self.board.get_field_checkers(field2.into());
        if res1.is_err() || res2.is_err() {
            return false;
        }
        let (count1, opt_color1) = res1.unwrap();
        let (count2, opt_color2) = res2.unwrap();
        count1 > 0 && count2 > 0 && opt_color1 == Some(color) && opt_color2 == Some(color)
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
                let color = if !self.players.is_empty() {
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
            Roll { player_id: _ } => {
                self.turn_stage = TurnStage::RollWaiting;
            }
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
                self.active_player_id = *self.players.keys().find(|id| *id != player_id).unwrap();
                self.turn_stage = TurnStage::RollDice;
            }
        }

        self.history.push(valid_event.clone());
    }

    /// Determines if someone has won the game
    pub fn determine_winner(&self) -> Option<PlayerId> {
        None
    }

    fn mark_points(&mut self, player_id: PlayerId, points: u8) {
        self.players.get_mut(&player_id).map(|p| {
            p.points += points;
            p
        });
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_string_id() {
        let mut state = GameState::default();
        state.add_player(1, Player::new("player1".into(), Color::White));
        state.add_player(2, Player::new("player2".into(), Color::Black));
        let string_id = state.to_string_id();
        // println!("string_id : {}", string_id);
        assert!(string_id == "Dz8+AAAAAT8/MAAAAAQAADAD");
    }

    #[test]
    fn moves_possible() {
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

        // black moves
        let moves = (
            CheckerMove::new(24, 20).unwrap(),
            CheckerMove::new(20, 19).unwrap(),
        );
        assert!(state.moves_possible(&Color::Black, &moves));
    }

    #[test]
    fn moves_follow_dices() {
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

    #[test]
    fn can_take_corner_by_effect() {
        let mut state = GameState::default();
        let player1 = Player::new("player1".into(), Color::White);
        let player_id = 1;
        state.add_player(player_id, player1);
        state.add_player(2, Player::new("player2".into(), Color::Black));
        state.consume(&GameEvent::BeginGame {
            goes_first: player_id,
        });
        state.consume(&GameEvent::Roll { player_id });

        state.board.set_positions([
            10, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, -15,
        ]);
        state.dice.values = (4, 4);
        assert!(state.can_take_corner_by_effect(&Color::White));

        state.dice.values = (5, 5);
        assert!(!state.can_take_corner_by_effect(&Color::White));

        state.board.set_positions([
            10, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, -15,
        ]);
        state.dice.values = (4, 4);
        assert!(!state.can_take_corner_by_effect(&Color::White));

        state.board.set_positions([
            10, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, -2, 0, 0, 0, 0, 0, 0, 0, 0, 0, -13,
        ]);
        state.dice.values = (1, 1);
        assert!(state.can_take_corner_by_effect(&Color::Black));
    }

    #[test]
    fn prise_en_puissance() {
        let mut state = GameState::default();
        let player1 = Player::new("player1".into(), Color::White);
        let player_id = 1;
        state.add_player(player_id, player1);
        state.add_player(2, Player::new("player2".into(), Color::Black));
        state.consume(&GameEvent::BeginGame {
            goes_first: player_id,
        });
        state.consume(&GameEvent::Roll { player_id });

        // prise par puissance ok
        state.board.set_positions([
            10, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, -15,
        ]);
        state.dice.values = (5, 5);
        let moves = (
            CheckerMove::new(8, 12).unwrap(),
            CheckerMove::new(8, 12).unwrap(),
        );
        assert!(state.is_move_by_puissance(&Color::White, &moves));
        assert!(state.moves_follows_dices(&Color::White, &moves));
        assert!(state.moves_allowed(&Color::White, &moves));

        // opponent corner must be empty
        state.board.set_positions([
            10, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, -2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, -13,
        ]);
        assert!(!state.is_move_by_puissance(&Color::White, &moves));
        assert!(!state.moves_follows_dices(&Color::White, &moves));

        // Si on a la possibilité de prendre son coin à la fois par effet, c'est à dire naturellement, et aussi par puissance, on doit le prendre par effet
        state.board.set_positions([
            5, 0, 0, 0, 0, 0, 5, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, -15,
        ]);
        assert!(!state.moves_allowed(&Color::White, &moves));

        // on a déjà pris son coin : on ne peux plus y deplacer des dames par puissance
        state.board.set_positions([
            8, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, -15,
        ]);
        assert!(!state.is_move_by_puissance(&Color::White, &moves));
        assert!(!state.moves_follows_dices(&Color::White, &moves));
    }

    #[test]
    fn exit() {
        let mut state = GameState::default();
        let player1 = Player::new("player1".into(), Color::White);
        let player_id = 1;
        state.add_player(player_id, player1);
        state.add_player(2, Player::new("player2".into(), Color::Black));
        state.consume(&GameEvent::BeginGame {
            goes_first: player_id,
        });
        state.consume(&GameEvent::Roll { player_id });

        // exit ok
        state.board.set_positions([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0,
        ]);
        state.dice.values = (5, 5);
        let moves = (
            CheckerMove::new(20, 0).unwrap(),
            CheckerMove::new(20, 0).unwrap(),
        );
        assert!(state.moves_possible(&Color::White, &moves));
        assert!(state.moves_follows_dices(&Color::White, &moves));
        assert!(state.moves_allowed(&Color::White, &moves));

        // toutes les dames doivent être dans le jan de retour
        state.board.set_positions([
            0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0,
        ]);
        state.dice.values = (5, 5);
        let moves = (
            CheckerMove::new(20, 0).unwrap(),
            CheckerMove::new(20, 0).unwrap(),
        );
        assert!(!state.moves_allowed(&Color::White, &moves));

        // on ne peut pas sortir une dame avec un nombre excédant si on peut en jouer une avec un nombre défaillant
        state.board.set_positions([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 3, 0, 0, 2, 0,
        ]);
        state.dice.values = (5, 5);
        let moves = (
            CheckerMove::new(20, 0).unwrap(),
            CheckerMove::new(23, 0).unwrap(),
        );
        assert!(!state.moves_allowed(&Color::White, &moves));

        // on doit jouer le nombre excédant le plus éloigné
        state.board.set_positions([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 1, 0,
        ]);
        state.dice.values = (5, 5);
        let moves = (
            CheckerMove::new(20, 0).unwrap(),
            CheckerMove::new(23, 0).unwrap(),
        );
        assert!(!state.moves_allowed(&Color::White, &moves));
        let moves = (
            CheckerMove::new(20, 0).unwrap(),
            CheckerMove::new(20, 0).unwrap(),
        );
        assert!(state.moves_allowed(&Color::White, &moves));

        // Cas de la dernière dame
        state.board.set_positions([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0,
        ]);
        state.dice.values = (5, 5);
        let moves = (
            CheckerMove::new(23, 0).unwrap(),
            CheckerMove::new(0, 0).unwrap(),
        );
        assert!(state.moves_possible(&Color::White, &moves));
        assert!(state.moves_follows_dices(&Color::White, &moves));
        assert!(state.moves_allowed(&Color::White, &moves));
    }

    #[test]
    fn move_check_oponnent_fillable_quarter() {
        let mut state = GameState::default();
        state.board.set_positions([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 1, 0,
        ]);
        state.dice.values = (5, 5);
        let moves = (
            CheckerMove::new(11, 16).unwrap(),
            CheckerMove::new(11, 16).unwrap(),
        );
        assert!(state.moves_allowed(&Color::White, &moves));

        state.board.set_positions([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, -12, 0, 0, 0, 0, 1, 0,
        ]);
        state.dice.values = (5, 5);
        let moves = (
            CheckerMove::new(11, 16).unwrap(),
            CheckerMove::new(11, 16).unwrap(),
        );
        assert!(!state.moves_allowed(&Color::White, &moves));
    }

    #[test]
    fn move_check_fillable_quarter() {
        let mut state = GameState::default();
        state.board.set_positions([
            3, 3, 2, 2, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 1, 0,
        ]);
        state.dice.values = (5, 4);
        let moves = (
            CheckerMove::new(1, 6).unwrap(),
            CheckerMove::new(2, 6).unwrap(),
        );
        assert!(state.moves_allowed(&Color::White, &moves));
        let moves = (
            CheckerMove::new(1, 5).unwrap(),
            CheckerMove::new(2, 7).unwrap(),
        );
        assert!(!state.moves_allowed(&Color::White, &moves));

        state.board.set_positions([
            2, 3, 2, 2, 3, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ]);
        state.dice.values = (2, 3);
        let moves = (
            CheckerMove::new(6, 8).unwrap(),
            CheckerMove::new(6, 9).unwrap(),
        );
        assert!(!state.moves_allowed(&Color::White, &moves));
        let moves = (
            CheckerMove::new(2, 4).unwrap(),
            CheckerMove::new(5, 8).unwrap(),
        );
        assert!(state.moves_allowed(&Color::White, &moves));
    }
}
