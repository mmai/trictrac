use backbone_lib::traits::{BackEndArchitecture, BackendCommand};
use trictrac_store::{CheckerMove, DiceRoller, GameEvent, GameState, TurnStage};

use crate::trictrac::types::{GameDelta, PlayerAction, ViewState};

// Store PlayerId (u64) values used for the two players.
const HOST_PLAYER_ID: u64 = 1;
const GUEST_PLAYER_ID: u64 = 2;

pub struct TrictracBackend {
    game: GameState,
    dice_roller: DiceRoller,
    commands: Vec<BackendCommand<GameDelta>>,
    view_state: ViewState,
    /// Arrival flags: have host (index 0) and guest (index 1) joined?
    arrived: [bool; 2],
    /// First move of the current pair, waiting for the second.
    pending_first_move: Option<CheckerMove>,
}

impl TrictracBackend {
    fn sync_view_state(&mut self) {
        self.view_state =
            ViewState::from_game_state(&self.game, HOST_PLAYER_ID, GUEST_PLAYER_ID);
    }

    fn broadcast_state(&mut self) {
        self.sync_view_state();
        let delta = GameDelta { state: self.view_state.clone() };
        self.commands.push(BackendCommand::Delta(delta));
    }

    /// Roll dice using the store's DiceRoller and fire Roll + RollResult events.
    fn do_roll(&mut self) {
        let dice = self.dice_roller.roll();
        let player_id = self.game.active_player_id;
        let _ = self.game.consume(&GameEvent::Roll { player_id });
        let _ = self.game.consume(&GameEvent::RollResult { player_id, dice });

        // Drive automatic stages that require no player input.
        self.drive_automatic_stages();
    }

    /// Advance through stages that can be resolved without player input
    /// (MarkPoints, MarkAdvPoints).
    fn drive_automatic_stages(&mut self) {
        loop {
            let player_id = self.game.active_player_id;
            match self.game.turn_stage {
                TurnStage::MarkPoints | TurnStage::MarkAdvPoints => {
                    let _ = self.game.consume(&GameEvent::Mark {
                        player_id,
                        points: self.game.dice_points.0.max(self.game.dice_points.1),
                    });
                }
                _ => break,
            }
        }
    }
}

impl BackEndArchitecture<PlayerAction, GameDelta, ViewState> for TrictracBackend {
    fn new(_rule_variation: u16) -> Self {
        let mut game = GameState::new(false);
        game.init_player("Host");
        game.init_player("Guest");

        let view_state =
            ViewState::from_game_state(&game, HOST_PLAYER_ID, GUEST_PLAYER_ID);

        TrictracBackend {
            game,
            dice_roller: DiceRoller::default(),
            commands: Vec::new(),
            view_state,
            arrived: [false; 2],
            pending_first_move: None,
        }
    }

    fn from_bytes(_rule_variation: u16, bytes: &[u8]) -> Option<Self> {
        let view_state: ViewState = serde_json::from_slice(bytes).ok()?;
        // Reconstruct a fresh game; full state restore is not yet implemented.
        let mut backend = Self::new(_rule_variation);
        backend.view_state = view_state;
        Some(backend)
    }

    fn player_arrival(&mut self, mp_player: u16) {
        if mp_player > 1 {
            self.commands.push(BackendCommand::KickPlayer { player: mp_player });
            return;
        }
        self.arrived[mp_player as usize] = true;

        // Cancel any reconnect timer for this player.
        self.commands.push(BackendCommand::CancelTimer { timer_id: mp_player });

        // Start the game once both players have arrived.
        if self.arrived[0] && self.arrived[1] && self.game.stage == trictrac_store::Stage::PreGame
        {
            let _ = self.game.consume(&GameEvent::BeginGame { goes_first: HOST_PLAYER_ID });
            self.commands.push(BackendCommand::ResetViewState);
        } else {
            self.broadcast_state();
        }
    }

    fn player_departure(&mut self, mp_player: u16) {
        if mp_player > 1 {
            return;
        }
        self.arrived[mp_player as usize] = false;
        // Give 60 seconds to reconnect before terminating the room.
        self.commands.push(BackendCommand::SetTimer {
            timer_id: mp_player,
            duration: 60.0,
        });
    }

    fn inform_rpc(&mut self, mp_player: u16, action: PlayerAction) {
        if self.game.stage == trictrac_store::Stage::Ended {
            return;
        }

        let store_id = if mp_player == 0 { HOST_PLAYER_ID } else { GUEST_PLAYER_ID };

        // Only the active player may act (except during Chance-like waiting stages).
        if self.game.active_player_id != store_id {
            return;
        }

        match action {
            PlayerAction::Roll => {
                if self.game.turn_stage == TurnStage::RollDice {
                    self.do_roll();
                }
            }
            PlayerAction::Move { from, to } => {
                if self.game.turn_stage != TurnStage::Move {
                    return;
                }
                let Ok(cmove) = CheckerMove::new(from as usize, to as usize) else {
                    return;
                };
                if let Some(first) = self.pending_first_move.take() {
                    let event = GameEvent::Move {
                        player_id: store_id,
                        moves: (first, cmove),
                    };
                    if self.game.validate(&event) {
                        let _ = self.game.consume(&event);
                        self.drive_automatic_stages();
                    }
                    // Whether valid or not, clear pending so the player can retry.
                } else {
                    self.pending_first_move = Some(cmove);
                    // No state broadcast yet — wait for the second move.
                    return;
                }
            }
            PlayerAction::Go => {
                if self.game.turn_stage == TurnStage::HoldOrGoChoice {
                    let _ = self.game.consume(&GameEvent::Go { player_id: store_id });
                }
            }
            PlayerAction::Mark => {
                if matches!(
                    self.game.turn_stage,
                    TurnStage::MarkPoints | TurnStage::MarkAdvPoints
                ) {
                    self.drive_automatic_stages();
                }
            }
        }

        self.broadcast_state();
    }

    fn timer_triggered(&mut self, timer_id: u16) {
        match timer_id {
            0 | 1 => {
                // Reconnect grace period expired for host (0) or guest (1).
                self.commands.push(BackendCommand::TerminateRoom);
            }
            _ => {}
        }
    }

    fn get_view_state(&self) -> &ViewState {
        &self.view_state
    }

    fn drain_commands(&mut self) -> Vec<BackendCommand<GameDelta>> {
        std::mem::take(&mut self.commands)
    }
}
