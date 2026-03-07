//! # Play a TricTrac Game
use crate::board::{Board, CheckerMove};
use crate::dice::Dice;
use crate::game_rules_moves::MoveRules;
use crate::game_rules_points::{PointsRules, PossibleJans, PossibleJansMethods};
use crate::player::{Color, Player, PlayerId};
// use anyhow::{Context, Result};
use log::{debug, error};

// use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::{fmt, str};

use base64::{engine::general_purpose, Engine as _};

/// The different stages a game can be in. (not to be confused with the entire "GameState")
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Stage {
    PreGame,
    InGame,
    Ended,
}

/// The different stages a game turn can be in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
        let empty_string = String::from("");
        s.push_str(&format!(
            "Who plays: {}\n",
            self.who_plays()
                .map(|player| &player.name)
                .unwrap_or_else(|| &empty_string)
        ));
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
impl Hash for GameState {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.to_string_id().hash(state);
    }
}

impl GameState {
    /// Create a new default game
    pub fn new(schools_enabled: bool) -> Self {
        let mut gs = GameState::default();
        gs.set_schools_enabled(schools_enabled);
        gs
    }

    pub fn new_with_players(p1_name: &str, p2_name: &str) -> Self {
        let mut game = Self::default();
        if let Some(p1) = game.init_player(p1_name) {
            game.init_player(p2_name);
            let _ = game
                .consume(&GameEvent::BeginGame { goes_first: p1 })
                .inspect_err(|e| error!("{}", e));
        }
        game
    }

    pub fn mirror(&self) -> GameState {
        let mirrored_active_player = if self.active_player_id == 1 { 2 } else { 1 };
        let mut mirrored_players = HashMap::new();
        if let Some(p2) = self.players.get(&2) {
            mirrored_players.insert(1, p2.mirror());
        }
        if let Some(p1) = self.players.get(&1) {
            mirrored_players.insert(2, p1.mirror());
        }
        let (move1, move2) = self.dice_moves;
        GameState {
            stage: self.stage,
            turn_stage: self.turn_stage,
            board: self.board.mirror(),
            active_player_id: mirrored_active_player,
            // active_player_id: self.active_player_id,
            players: mirrored_players,
            history: Vec::new(),
            dice: self.dice,
            dice_points: self.dice_points,
            dice_moves: (move1.mirror(), move2.mirror()),
            dice_jans: self.dice_jans.mirror(),
            roll_first: self.roll_first,
            schools_enabled: self.schools_enabled,
        }
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

    /// Get state as a tensor for neural network training (Option B, TD-Gammon style).
    /// Returns 217 f32 values, all normalized to [0, 1].
    ///
    /// Must be called from the active player's perspective: callers should mirror
    /// the GameState for Black before calling so that "own" always means White.
    ///
    /// Layout:
    ///   [0..95]    own (White) checkers: 4 values per field × 24 fields
    ///   [96..191]  opp (Black) checkers: 4 values per field × 24 fields
    ///   [192..193] dice values / 6
    ///   [194]      active player color (0=White, 1=Black)
    ///   [195]      turn_stage / 5
    ///   [196..199] White player: points/12, holes/12, can_bredouille, can_big_bredouille
    ///   [200..203] Black player: same
    ///   [204..207] own quarter filled (quarters 1-4)
    ///   [208..211] opp quarter filled (quarters 1-4)
    ///   [212]      own checkers all in exit zone (fields 19-24)
    ///   [213]      opp checkers all in exit zone (fields 1-6)
    ///   [214]      own coin de repos taken (field 12 has ≥2 own checkers)
    ///   [215]      opp coin de repos taken (field 13 has ≥2 opp checkers)
    ///   [216]      own dice_roll_count / 3, clamped to 1
    pub fn to_tensor(&self) -> Vec<f32> {
        let mut t = Vec::with_capacity(217);
        let pos: Vec<i8> = self.board.to_vec(); // 24 elements, positive=White, negative=Black

        // [0..95] own (White) checkers, TD-Gammon encoding.
        // Each field contributes 4 values:
        //   (count==1), (count==2), (count==3), (count-3)/12  ← all in [0,1]
        // The overflow term is divided by 12 because the maximum excess is
        // 15 (all checkers) − 3 = 12.
        for &c in &pos {
            let own = c.max(0) as u8;
            t.push((own == 1) as u8 as f32);
            t.push((own == 2) as u8 as f32);
            t.push((own == 3) as u8 as f32);
            t.push(own.saturating_sub(3) as f32 / 12.0);
        }

        // [96..191] opp (Black) checkers, same encoding.
        for &c in &pos {
            let opp = (-c).max(0) as u8;
            t.push((opp == 1) as u8 as f32);
            t.push((opp == 2) as u8 as f32);
            t.push((opp == 3) as u8 as f32);
            t.push(opp.saturating_sub(3) as f32 / 12.0);
        }

        // [192..193] dice
        t.push(self.dice.values.0 as f32 / 6.0);
        t.push(self.dice.values.1 as f32 / 6.0);

        // [194] active player color
        t.push(
            self.who_plays()
                .map(|p| if p.color == Color::Black { 1.0f32 } else { 0.0 })
                .unwrap_or(0.0),
        );

        // [195] turn stage
        t.push(u8::from(self.turn_stage) as f32 / 5.0);

        // [196..199] White player stats
        let wp = self.get_white_player();
        t.push(wp.map_or(0.0, |p| p.points as f32 / 12.0));
        t.push(wp.map_or(0.0, |p| p.holes as f32 / 12.0));
        t.push(wp.map_or(0.0, |p| p.can_bredouille as u8 as f32));
        t.push(wp.map_or(0.0, |p| p.can_big_bredouille as u8 as f32));

        // [200..203] Black player stats
        let bp = self.get_black_player();
        t.push(bp.map_or(0.0, |p| p.points as f32 / 12.0));
        t.push(bp.map_or(0.0, |p| p.holes as f32 / 12.0));
        t.push(bp.map_or(0.0, |p| p.can_bredouille as u8 as f32));
        t.push(bp.map_or(0.0, |p| p.can_big_bredouille as u8 as f32));

        // [204..207] own (White) quarter fill status
        for &start in &[1usize, 7, 13, 19] {
            t.push(self.board.is_quarter_filled(Color::White, start) as u8 as f32);
        }

        // [208..211] opp (Black) quarter fill status
        for &start in &[1usize, 7, 13, 19] {
            t.push(self.board.is_quarter_filled(Color::Black, start) as u8 as f32);
        }

        // [212] can_exit_own: no own checker in fields 1-18
        t.push(pos[0..18].iter().all(|&c| c <= 0) as u8 as f32);

        // [213] can_exit_opp: no opp checker in fields 7-24
        t.push(pos[6..24].iter().all(|&c| c >= 0) as u8 as f32);

        // [214] own coin de repos taken (field 12 = index 11, ≥2 own checkers)
        t.push((pos[11] >= 2) as u8 as f32);

        // [215] opp coin de repos taken (field 13 = index 12, ≥2 opp checkers)
        t.push((pos[12] <= -2) as u8 as f32);

        // [216] own dice_roll_count / 3, clamped to 1
        t.push((wp.map_or(0, |p| p.dice_roll_count) as f32 / 3.0).min(1.0));

        debug_assert_eq!(t.len(), 217, "to_tensor length mismatch");
        t
    }

    /// Get state as a vector (to be used for bot training input) :
    /// length = 36
    /// i8 for board positions with negative values for blacks
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
            .map(|p| p.to_vec().iter().map(|&x| x as i8).collect())
            .unwrap_or(vec![0; 10]);
        state.extend(white_player);
        let black_player: Vec<i8> = self
            .get_black_player()
            .map(|p| p.to_vec().iter().map(|&x| x as i8).collect())
            .unwrap_or(vec![0; 10]);
        state.extend(black_player);

        // ensure state has length state_len
        state.truncate(state_len);
        while state.len() < state_len {
            state.push(0);
        }
        state
    }

    /// Calculate game state id :
    pub fn to_string_id_slow(&self) -> String {
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
        let white_bits = self
            .get_white_player()
            .map(|p| p.to_bits_string())
            .unwrap_or("0000000000".into());
        let black_bits = self
            .get_black_player()
            .map(|p| p.to_bits_string())
            .unwrap_or("0000000000".into());
        pos_bits.push_str(&white_bits);
        pos_bits.push_str(&black_bits);

        pos_bits = format!("{pos_bits:0<108}");
        // println!("{}", pos_bits);
        // let pos_u8 = pos_bits
        //     .as_bytes()
        //     .chunks(6)
        //     .map(|chunk| str::from_utf8(chunk).unwrap())
        //     .map(|chunk| u8::from_str_radix(chunk, 2).unwrap())
        //     .collect::<Vec<u8>>();

        let pos_u8 = pos_bits
            .as_bytes()
            .chunks(6)
            .map(|chunk| chunk.iter().fold(0u8, |acc, &b| (acc << 1) | (b - b'0')))
            .collect::<Vec<u8>>();

        general_purpose::STANDARD.encode(pos_u8)
    }

    pub fn to_string_id(&self) -> String {
        const TOTAL_BITS: usize = 108;
        const TOTAL_BYTES: usize = TOTAL_BITS / 6; // 18 bytes

        let mut output = Vec::with_capacity(TOTAL_BYTES);

        let mut current: u8 = 0;
        let mut bit_count: u8 = 0;

        // helper to push a single bit
        let push_bit = |bit: u8, output: &mut Vec<u8>, current: &mut u8, bit_count: &mut u8| {
            *current = (*current << 1) | (bit & 1);
            *bit_count += 1;

            if *bit_count == 6 {
                output.push(*current);
                *current = 0;
                *bit_count = 0;
            }
        };

        // helper to push a string of '0'/'1'
        let push_bits_str =
            |bits: &str, output: &mut Vec<u8>, current: &mut u8, bit_count: &mut u8| {
                for b in bits.bytes() {
                    push_bit(b - b'0', output, current, bit_count);
                }
            };

        // --------------------------------------------------
        // 1️⃣ Board position bits
        // --------------------------------------------------
        push_bits_str(
            &self.board.to_gnupg_pos_id(),
            &mut output,
            &mut current,
            &mut bit_count,
        );

        // --------------------------------------------------
        // 2️⃣ Active player (1 bit)
        // --------------------------------------------------
        let active_bit = self
            .who_plays()
            .map(|player| (player.color == Color::Black) as u8)
            .unwrap_or(0);

        push_bit(active_bit, &mut output, &mut current, &mut bit_count);

        // --------------------------------------------------
        // 3️⃣ Turn stage (3 bits)
        // --------------------------------------------------
        let stage_bits: u8 = match self.turn_stage {
            TurnStage::RollWaiting => 0b000,
            TurnStage::RollDice => 0b001,
            TurnStage::MarkPoints => 0b010,
            TurnStage::HoldOrGoChoice => 0b011,
            TurnStage::Move => 0b100,
            TurnStage::MarkAdvPoints => 0b101,
        };

        for i in (0..3).rev() {
            push_bit(
                (stage_bits >> i) & 1,
                &mut output,
                &mut current,
                &mut bit_count,
            );
        }

        // --------------------------------------------------
        // 4️⃣ Dice (6 bits)
        // --------------------------------------------------
        push_bits_str(
            &self.dice.to_bits_string(),
            &mut output,
            &mut current,
            &mut bit_count,
        );

        // --------------------------------------------------
        // 5️⃣ Players points (10 bits each)
        // --------------------------------------------------
        let white_bits = self
            .get_white_player()
            .map(|p| p.to_bits_string())
            .unwrap_or_else(|| "0000000000".to_string());

        let black_bits = self
            .get_black_player()
            .map(|p| p.to_bits_string())
            .unwrap_or_else(|| "0000000000".to_string());

        push_bits_str(&white_bits, &mut output, &mut current, &mut bit_count);
        push_bits_str(&black_bits, &mut output, &mut current, &mut bit_count);

        // --------------------------------------------------
        // 6️⃣ Pad remaining bits (if needed)
        // --------------------------------------------------
        while output.len() < TOTAL_BYTES {
            push_bit(0, &mut output, &mut current, &mut bit_count);
        }

        base64::engine::general_purpose::STANDARD.encode(output)
    }

    pub fn from_string_id(id: &str) -> Result<Self, String> {
        let bytes = general_purpose::STANDARD
            .decode(id)
            .map_err(|e| e.to_string())?;

        let bits_str: String = bytes.iter().map(|byte| format!("{:06b}", byte)).collect();

        // The original string was padded to 108 bits.
        let bits = if bits_str.len() >= 108 {
            &bits_str[..108]
        } else {
            return Err("Invalid decoded string length".to_string());
        };

        let board_bits = &bits[0..77];
        let board = Board::from_gnupg_pos_id(board_bits)?;

        let Some(active_player_bit) = bits.chars().nth(77) else {
            return Err("No bit at 77th position".to_string());
        };
        let active_player_color = if active_player_bit == '1' {
            Color::Black
        } else {
            Color::White
        };

        let turn_stage_bits = &bits[78..81];
        let turn_stage = match turn_stage_bits {
            "000" => TurnStage::RollWaiting,
            "001" => TurnStage::RollDice,
            "010" => TurnStage::MarkPoints,
            "011" => TurnStage::HoldOrGoChoice,
            "100" => TurnStage::Move,
            "101" => TurnStage::MarkAdvPoints,
            _ => return Err(format!("Invalid bits for turn stage : {turn_stage_bits}")),
        };

        let dice_bits = &bits[81..87];
        let dice = Dice::from_bits_string(dice_bits).map_err(|e| e.to_string())?;

        let white_player_bits = &bits[87..97];
        let black_player_bits = &bits[97..107];

        let white_player =
            Player::from_bits_string(white_player_bits, "Player 1".to_string(), Color::White)
                .map_err(|e| e.to_string())?;
        let black_player =
            Player::from_bits_string(black_player_bits, "Player 2".to_string(), Color::Black)
                .map_err(|e| e.to_string())?;

        let mut players = HashMap::new();
        players.insert(1, white_player);
        players.insert(2, black_player);

        let active_player_id = if active_player_color == Color::White {
            1
        } else {
            2
        };

        // Some fields are not in the ID, so we use defaults.
        Ok(GameState {
            stage: Stage::InGame, // Assume InGame from ID
            turn_stage,
            board,
            active_player_id,
            players,
            history: Vec::new(),
            dice,
            dice_points: (0, 0),
            dice_moves: (CheckerMove::default(), CheckerMove::default()),
            dice_jans: PossibleJans::default(),
            roll_first: false,      // Assume not first roll
            schools_enabled: false, // Assume disabled
        })
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
            Roll { player_id } => {
                // Check player exists
                if !self.players.contains_key(player_id) {
                    error!("unknown player_id");
                    return false;
                }
                // Check player is currently the one making their move
                if self.active_player_id != *player_id {
                    error!("not active player_id");
                    return false;
                }
                // Check the turn stage
                if self.turn_stage != TurnStage::RollDice {
                    error!("bad stage {:?}", self.turn_stage);
                    return false;
                }
            }
            RollResult { player_id, dice: _ } => {
                // Check player exists
                if !self.players.contains_key(player_id) {
                    return false;
                }
                // Check player is currently the one making their move
                if self.active_player_id != *player_id {
                    return false;
                }
                // Check the turn stage
                if self.turn_stage != TurnStage::RollWaiting {
                    error!("bad stage {:?}", self.turn_stage);
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
                    // println!(">>> rules not followed ");
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
            // println!("more than two players");
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
    pub fn consume(&mut self, valid_event: &GameEvent) -> Result<(), String> {
        use GameEvent::*;
        match valid_event {
            BeginGame { goes_first } => {
                self.active_player_id = *goes_first;
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
                (self.dice_jans, self.dice_points) = self.get_rollresult_jans(dice)?;
                debug!("points from result : {:?}", self.dice_points);
                if !self.schools_enabled {
                    // Schools are not enabled. We mark points automatically
                    // the points earned by the opponent will be marked on its turn
                    let new_hole = self.mark_points(self.active_player_id, self.dice_points.0);
                    if new_hole {
                        let Some(holes_count) = self.get_active_player().map(|p| p.holes) else {
                            return Err("No active player".into());
                        };
                        debug!("new hole  -> {holes_count:?}");
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
                        let Some(holes) = self.get_active_player().map(|p| p.holes) else {
                            return Err("No active player".into());
                        };
                        if holes > 12 {
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
                let Some(player) = self.players.get(player_id) else {
                    return Err("unknown player {player_id}".into());
                };
                self.board
                    .move_checker(&player.color, moves.0)
                    .map_err(|e| e.to_string())?;
                self.board
                    .move_checker(&player.color, moves.1)
                    .map_err(|e| e.to_string())?;
                self.dice_moves = *moves;
                let Some(active_player_id) = self.players.keys().find(|id| *id != player_id) else {
                    return Err("Can't find player id {id}".into());
                };
                self.active_player_id = *active_player_id;
                self.turn_stage = if self.schools_enabled {
                    TurnStage::MarkAdvPoints
                } else {
                    // The player has moved, we can mark its opponent's points (which is now the current player)
                    let new_hole = self.mark_points(self.active_player_id, self.dice_points.1);
                    if new_hole && self.get_active_player().map(|p| p.holes).unwrap_or(0) > 12 {
                        self.stage = Stage::Ended;
                    }
                    TurnStage::RollDice
                };
            }
            PlayError => {}
        }
        self.history.push(valid_event.clone());
        Ok(())
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

    fn get_rollresult_jans(&self, dice: &Dice) -> Result<(PossibleJans, (u8, u8)), String> {
        let Some(player) = &self.players.get(&self.active_player_id) else {
            return Err("No active player".into());
        };
        debug!(
            "get rollresult for {:?} {:?} {:?} (roll count {:?})",
            player.color, self.board, dice, player.dice_roll_count
        );
        let points_rules = PointsRules::new(&player.color, &self.board, *dice);
        Ok(points_rules.get_result_jans(player.dice_roll_count))
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
            p.dice_roll_count = p.dice_roll_count.saturating_add(1);
            p
        });
    }

    pub fn mark_points_for_bot_training(&mut self, player_id: PlayerId, points: u8) -> bool {
        self.mark_points(player_id, points)
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
                debug!(
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
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, Deserialize)]
pub enum EndGameReason {
    PlayerLeft { player_id: PlayerId },
    PlayerWon { winner: PlayerId },
}

/// An event that progresses the GameState forward
#[derive(Debug, Clone, Serialize, PartialEq, Eq, Deserialize)]
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

    pub fn get_mirror(&self, preserve_player: bool) -> Self {
        // let mut mirror = self.clone();
        let mirror_player_id = if let Some(player_id) = self.player_id() {
            if preserve_player {
                player_id
            } else if player_id == 1 {
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
        assert_eq!(string_id, "Pz84AAAABz8/AAAAAAgAASAG");
        let new_state = GameState::from_string_id(&string_id).unwrap();
        assert_eq!(state.board, new_state.board);
        assert_eq!(state.active_player_id, new_state.active_player_id);
        assert_eq!(state.turn_stage, new_state.turn_stage);
        assert_eq!(state.dice, new_state.dice);
        assert_eq!(
            state.get_white_player().unwrap().points,
            new_state.get_white_player().unwrap().points
        );
    }

    #[test]
    fn hold_or_go() {
        let mut game_state = init_test_gamestate(TurnStage::MarkPoints);
        game_state.schools_enabled = true;
        let pid = game_state.active_player_id;
        let _ = game_state.consume(
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
        let _ = game_state.consume(
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
        let _ = game_state.consume(
            &(GameEvent::Mark {
                player_id: pid,
                points: 13,
            }),
        );
        let moves = (
            CheckerMove::new(1, 3).unwrap(),
            CheckerMove::new(1, 3).unwrap(),
        );
        let _ = game_state.consume(
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
