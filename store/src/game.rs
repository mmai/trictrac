//! # Play a TricTrac Game
use crate::board::{Board, CheckerMove};
use crate::dice::Dice;
use crate::game_rules_moves::MoveRules;
use crate::game_rules_points::{PointsRules, PossibleJans};
use crate::player::{Color, Player, PlayerId};
use log::{error, info};

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
    HoldOrGoChoice,
    Move,
    MarkAdvPoints,
}

impl From<u8> for TurnStage {
    fn from(item: u8) -> Self {
        match item {
            0 => TurnStage::RollWaiting,
            1 => TurnStage::RollDice,
            2 => TurnStage::MarkPoints,
            3 => TurnStage::HoldOrGoChoice,
            4 => TurnStage::Move,
            5 => TurnStage::MarkAdvPoints,
            _ => TurnStage::RollWaiting,
        }
    }
}

impl From<TurnStage> for u8 {
    fn from(stage: TurnStage) -> u8 {
        match stage {
            TurnStage::RollWaiting => 0,
            TurnStage::RollDice => 1,
            TurnStage::MarkPoints => 2,
            TurnStage::HoldOrGoChoice => 3,
            TurnStage::Move => 4,
            TurnStage::MarkAdvPoints => 5,
        }
    }
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
    /// players points computed for the last dice pair rolled
    pub dice_points: (u8, u8),
    pub dice_moves: (CheckerMove, CheckerMove),
    pub dice_jans: PossibleJans,
    /// true if player needs to roll first
    roll_first: bool,
    // NOTE: add to a Setting struct if other fields needed
    pub schools_enabled: bool,
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
        // s.push_str(&format!("History: {:?}\n", self.history));
        write!(f, "{s}")
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
            dice_points: (0, 0),
            dice_moves: (CheckerMove::default(), CheckerMove::default()),
            dice_jans: PossibleJans::default(),
            roll_first: true,
            schools_enabled: false,
        }
    }
}

impl GameState {
    /// Create a new default game
    pub fn new(schools_enabled: bool) -> Self {
        let mut gs = GameState::default();
        gs.set_schools_enabled(schools_enabled);
        gs
    }

    fn set_schools_enabled(&mut self, schools_enabled: bool) {
        self.schools_enabled = schools_enabled;
    }

    fn get_active_player(&self) -> Option<&Player> {
        self.players.get(&self.active_player_id)
    }

    fn get_opponent_id(&self) -> Option<PlayerId> {
        self.players
            .keys()
            .copied()
            .filter(|k| k != &self.active_player_id)
            .collect::<Vec<PlayerId>>()
            .first()
            .copied()
    }

    // -------------------------------------------------------------------------
    //                        accessors
    // -------------------------------------------------------------------------

    pub fn to_vec_float(&self) -> Vec<f32> {
        self.to_vec().iter().map(|&x| x as f32).collect()
    }

    /// Get state as a vector (to be used for bot training input) :
    /// length = 36
    pub fn to_vec(&self) -> Vec<i8> {
        let state_len = 36;
        let mut state = Vec::with_capacity(state_len);

        // length = 24
        state.extend(self.board.to_vec());

        // active player -> length = 1
        // white : 0 (false)
        // black : 1 (true)
        state.push(
            self.who_plays()
                .map(|player| if player.color == Color::Black { 1 } else { 0 })
                .unwrap_or(0), // White by default
        );

        // step  -> length = 1
        let turn_stage: u8 = self.turn_stage.into();
        state.push(turn_stage as i8);

        // dice roll -> length = 2
        state.push(self.dice.values.0 as i8);
        state.push(self.dice.values.1 as i8);

        // points, trous, bredouille, grande bredouille length=4 x2 joueurs = 8
        let white_player: Vec<i8> = self
            .get_white_player()
            .unwrap()
            .to_vec()
            .iter()
            .map(|&x| x as i8)
            .collect();
        state.extend(white_player);
        let black_player: Vec<i8> = self
            .get_black_player()
            .unwrap()
            .to_vec()
            .iter()
            .map(|&x| x as i8)
            .collect();
        // .iter().map(|&x| x as i8) .collect()
        state.extend(black_player);

        // ensure state has length state_len
        state.truncate(state_len);
        while state.len() < state_len {
            state.push(0);
        }
        state
    }

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

        // step  -> 3 bits
        let step_bits = match self.turn_stage {
            TurnStage::RollWaiting => "000",
            TurnStage::RollDice => "001",
            TurnStage::MarkPoints => "010",
            TurnStage::HoldOrGoChoice => "011",
            TurnStage::Move => "100",
            TurnStage::MarkAdvPoints => "101",
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
        self.get_active_player()
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

                // Check points are correct
                // let (board, moves) = if *color == Color::Black {
                //     (board.mirror(), (moves.0.mirror(), moves.1.mirror()))
                // } else {
                //     (board.clone(), *moves)
                // };
                // let rules_points: u8 = self.get_points().iter().map(|r| r.0).sum();
                // if rules_points != *points {
                //     return false;
                // }
            }
            Go { player_id } => {
                if !self.players.contains_key(player_id) {
                    error!("Player {player_id} unknown");
                    return false;
                }
                // Check player is currently the one making their move
                if self.active_player_id != *player_id {
                    error!("Player not active : {}", self.active_player_id);
                    return false;
                }
                // Check the player can leave (ie the game is in the KeepOrLeaveChoice stage)
                if self.turn_stage != TurnStage::HoldOrGoChoice {
                    error!("bad stage {:?}", self.turn_stage);
                    error!(
                        "black player points : {:?}",
                        self.get_black_player()
                            .map(|player| (player.points, player.holes))
                    );
                    // error!("history {:?}", self.history);
                    return false;
                }
            }
            Move { player_id, moves } => {
                // Check player exists
                if !self.players.contains_key(player_id) {
                    error!("Player {player_id} unknown");
                    return false;
                }
                // Check player is currently the one making their move
                if self.active_player_id != *player_id {
                    error!("Player not active : {}", self.active_player_id);
                    return false;
                }
                // Check the turn stage
                if self.turn_stage != TurnStage::Move
                    && self.turn_stage != TurnStage::HoldOrGoChoice
                {
                    error!("bad stage {:?}", self.turn_stage);
                    return false;
                }
                let color = &self.players[player_id].color;

                let rules = MoveRules::new(color, &self.board, self.dice);
                let moves = if *color == Color::Black {
                    (moves.0.mirror(), moves.1.mirror())
                } else {
                    *moves
                };
                if !rules.moves_follow_rules(&moves) {
                    error!("rules not followed ");
                    return false;
                }
            }
            PlayError => {
                return true;
            }
        }

        // We couldn't find anything wrong with the event so it must be good
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
        let color = if player_id == 1 {
            Color::White
        } else {
            Color::Black
        };
        let player = Player::new(player_name.into(), color);
        self.players.insert(player_id as PlayerId, player);
        Some(player_id as PlayerId)
    }

    #[cfg(test)]
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
            EndGame { reason: _ } => {
                self.stage = Stage::Ended;
            }
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
                        dice_roll_count: 0,
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
                self.inc_roll_count(self.active_player_id);
                self.turn_stage = TurnStage::MarkPoints;
                (self.dice_jans, self.dice_points) = self.get_rollresult_jans(dice);
                info!("points from result : {:?}", self.dice_points);
                if !self.schools_enabled {
                    // Schools are not enabled. We mark points automatically
                    // the points earned by the opponent will be marked on its turn
                    let new_hole = self.mark_points(self.active_player_id, self.dice_points.0);
                    if new_hole {
                        let holes_count = self.get_active_player().unwrap().holes;
                        info!("new hole  -> {holes_count:?}");
                        if holes_count > 12 {
                            self.stage = Stage::Ended;
                        } else {
                            self.turn_stage = TurnStage::HoldOrGoChoice;
                        }
                    } else {
                        self.turn_stage = TurnStage::Move;
                    }
                }
            }
            Mark { player_id, points } => {
                if self.schools_enabled {
                    let new_hole = self.mark_points(*player_id, *points);
                    if new_hole {
                        if self.get_active_player().unwrap().holes > 12 {
                            self.stage = Stage::Ended;
                        } else {
                            self.turn_stage = if self.turn_stage == TurnStage::MarkAdvPoints {
                                TurnStage::RollDice
                            } else {
                                TurnStage::HoldOrGoChoice
                            };
                        }
                    } else {
                        self.turn_stage = if self.turn_stage == TurnStage::MarkAdvPoints {
                            TurnStage::RollDice
                        } else {
                            TurnStage::Move
                        };
                    }
                }
            }
            Go { player_id: _ } => self.new_pick_up(),
            Move { player_id, moves } => {
                let player = self.players.get(player_id).unwrap();
                self.board.move_checker(&player.color, moves.0).unwrap();
                self.board.move_checker(&player.color, moves.1).unwrap();
                self.dice_moves = *moves;
                self.active_player_id = *self.players.keys().find(|id| *id != player_id).unwrap();
                self.turn_stage = if self.schools_enabled {
                    TurnStage::MarkAdvPoints
                } else {
                    // The player has moved, we can mark its opponent's points (which is now the current player)
                    let new_hole = self.mark_points(self.active_player_id, self.dice_points.1);
                    if new_hole && self.get_active_player().unwrap().holes > 12 {
                        self.stage = Stage::Ended;
                    }
                    TurnStage::RollDice
                };
            }
            PlayError => {}
        }
        self.history.push(valid_event.clone());
    }

    /// Set a new pick up ('relevé') after a player won a hole and choose to 'go',
    /// or after a player has bore off (took of his men off the board)
    fn new_pick_up(&mut self) {
        self.players.iter_mut().for_each(|(_id, p)| {
            // reset points
            p.points = 0;
            // reset dice_roll_count
            p.dice_roll_count = 0;
            // reset bredouille
            p.can_bredouille = true;
            // XXX : switch colors
            // désactivé pour le moment car la vérification des mouvements échoue, cf. https://code.rhumbs.fr/henri/trictrac/issues/31
            // p.color = p.color.opponent_color();
        });
        // joueur actif = joueur ayant sorti ses dames ou est parti (donc deux jeux successifs)
        self.turn_stage = TurnStage::RollDice;
        // reset board
        self.board = Board::new();
    }

    fn get_rollresult_jans(&self, dice: &Dice) -> (PossibleJans, (u8, u8)) {
        let player = &self.players.get(&self.active_player_id).unwrap();
        info!(
            "get rollresult for {:?} {:?} {:?} (roll count {:?})",
            player.color, self.board, dice, player.dice_roll_count
        );
        let points_rules = PointsRules::new(&player.color, &self.board, *dice);
        points_rules.get_result_jans(player.dice_roll_count)
    }

    /// Determines if someone has won the game
    pub fn determine_winner(&self) -> Option<PlayerId> {
        // A player has won if he has got 12 holes
        self.players
            .iter()
            .filter(|(_, p)| p.holes > 11)
            .map(|(id, _)| *id)
            .next()
    }

    fn inc_roll_count(&mut self, player_id: PlayerId) {
        self.players.get_mut(&player_id).map(|p| {
            if p.dice_roll_count < u8::MAX {
                p.dice_roll_count += 1;
            }
            p
        });
    }

    fn mark_points(&mut self, player_id: PlayerId, points: u8) -> bool {
        // Update player points and holes
        let mut new_hole = false;
        self.players.get_mut(&player_id).map(|p| {
            let sum_points = p.points + points;
            let jeux = sum_points / 12;
            let holes = match (jeux, p.can_bredouille) {
                (0, _) => 0,
                (_, false) => 2 * jeux - 1,
                (_, true) => 2 * jeux,
            };

            new_hole = holes > 0;
            if new_hole {
                p.can_bredouille = true;
            }
            p.points = sum_points % 12;
            p.holes += holes;

            // if points > 0 && p.holes > 15 {
            if points > 0 {
                info!(
                    "player {player_id:?}  holes : {:?} (+{holes:?}) points : {:?} (+{points:?} - {jeux:?})",
                    p.holes, p.points
                )
            }
            p
        });

        // Opponent updates
        let maybe_op = if player_id == self.active_player_id {
            self.get_opponent_id()
        } else {
            Some(player_id)
        };
        if let Some(opp_id) = maybe_op {
            if points > 0 {
                self.players.get_mut(&opp_id).map(|opponent| {
                    // Cancel opponent bredouille
                    opponent.can_bredouille = false;
                    // Reset opponent points if the player finished a hole
                    if new_hole {
                        opponent.points = 0;
                        opponent.can_bredouille = true;
                    }
                    opponent
                });
            }
        }

        new_hole
    }
}

/// The reasons why a game could end
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Deserialize)]
pub enum EndGameReason {
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
    Go {
        player_id: PlayerId,
    },
    Move {
        player_id: PlayerId,
        moves: (CheckerMove, CheckerMove),
    },
    PlayError,
}

impl GameEvent {
    pub fn player_id(&self) -> Option<PlayerId> {
        match self {
            Self::PlayerJoined { player_id, name: _ } => Some(*player_id),
            Self::PlayerDisconnected { player_id } => Some(*player_id),
            Self::Roll { player_id } => Some(*player_id),
            Self::RollResult { player_id, dice: _ } => Some(*player_id),
            Self::Mark {
                player_id,
                points: _,
            } => Some(*player_id),
            Self::Go { player_id } => Some(*player_id),
            Self::Move {
                player_id,
                moves: _,
            } => Some(*player_id),
            _ => None,
        }
    }

    pub fn get_mirror(&self) -> Self {
        // let mut mirror = self.clone();
        let mirror_player_id = if let Some(player_id) = self.player_id() {
            if player_id == 1 {
                2
            } else {
                1
            }
        } else {
            0
        };

        match self {
            Self::PlayerJoined { player_id: _, name } => Self::PlayerJoined {
                player_id: mirror_player_id,
                name: name.clone(),
            },
            Self::PlayerDisconnected { player_id: _ } => GameEvent::PlayerDisconnected {
                player_id: mirror_player_id,
            },
            Self::Roll { player_id: _ } => GameEvent::Roll {
                player_id: mirror_player_id,
            },
            Self::RollResult { player_id: _, dice } => GameEvent::RollResult {
                player_id: mirror_player_id,
                dice: *dice,
            },
            Self::Mark {
                player_id: _,
                points,
            } => GameEvent::Mark {
                player_id: mirror_player_id,
                points: *points,
            },
            Self::Go { player_id: _ } => GameEvent::Go {
                player_id: mirror_player_id,
            },
            Self::Move {
                player_id: _,
                moves: (move1, move2),
            } => Self::Move {
                player_id: mirror_player_id,
                moves: (move1.mirror(), move2.mirror()),
            },
            Self::BeginGame { goes_first } => GameEvent::BeginGame {
                goes_first: (if *goes_first == 1 { 2 } else { 1 }),
            },
            Self::EndGame { reason } => GameEvent::EndGame { reason: *reason },
            Self::PlayError => GameEvent::PlayError,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn init_test_gamestate(turn: TurnStage) -> GameState {
        let mut state = GameState::default();
        state.add_player(1, Player::new("player1".into(), Color::White));
        state.add_player(2, Player::new("player2".into(), Color::Black));
        state.active_player_id = 1;
        state.turn_stage = turn;
        state
    }

    #[test]
    fn to_string_id() {
        let state = init_test_gamestate(TurnStage::RollDice);
        let string_id = state.to_string_id();
        // println!("string_id : {}", string_id);
        assert_eq!(string_id, "Hz88AAAAAz8/IAAAAAQAADAD");
    }

    #[test]
    fn hold_or_go() {
        let mut game_state = init_test_gamestate(TurnStage::MarkPoints);
        game_state.schools_enabled = true;
        let pid = game_state.active_player_id;
        game_state.consume(
            &(GameEvent::Mark {
                player_id: pid,
                points: 13,
            }),
        );
        let player = game_state.get_active_player().unwrap();
        assert_eq!(player.points, 1);
        assert_eq!(player.holes, 2); // because can bredouille
        assert_eq!(game_state.turn_stage, TurnStage::HoldOrGoChoice);

        // Go
        game_state.consume(
            &(GameEvent::Go {
                player_id: game_state.active_player_id,
            }),
        );
        assert_eq!(game_state.active_player_id, pid);
        let player = game_state.get_active_player().unwrap();
        assert_eq!(player.points, 0);
        assert_eq!(game_state.turn_stage, TurnStage::RollDice);

        // Hold
        let mut game_state = init_test_gamestate(TurnStage::MarkPoints);
        game_state.schools_enabled = true;
        let pid = game_state.active_player_id;
        game_state.consume(
            &(GameEvent::Mark {
                player_id: pid,
                points: 13,
            }),
        );
        let moves = (
            CheckerMove::new(1, 3).unwrap(),
            CheckerMove::new(1, 3).unwrap(),
        );
        game_state.consume(
            &(GameEvent::Move {
                player_id: game_state.active_player_id,
                moves,
            }),
        );
        assert_ne!(game_state.active_player_id, pid);
        assert_eq!(game_state.players.get(&pid).unwrap().points, 1);
        assert_eq!(game_state.get_active_player().unwrap().points, 0);
        assert_eq!(game_state.turn_stage, TurnStage::MarkAdvPoints);
    }
}
