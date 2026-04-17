use backbone_lib::traits::{BackEndArchitecture, BackendCommand};
use trictrac_store::{DiceRoller, GameEvent, GameState, TurnStage};

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
}

impl TrictracBackend {
    fn sync_view_state(&mut self) {
        self.view_state = ViewState::from_game_state(&self.game, HOST_PLAYER_ID, GUEST_PLAYER_ID);
    }

    fn broadcast_state(&mut self) {
        self.sync_view_state();
        let delta = GameDelta {
            state: self.view_state.clone(),
        };
        self.commands.push(BackendCommand::Delta(delta));
    }

    /// Roll dice using the store's DiceRoller and fire Roll + RollResult events.
    fn do_roll(&mut self) {
        let dice = self.dice_roller.roll();
        let player_id = self.game.active_player_id;
        let _ = self.game.consume(&GameEvent::Roll { player_id });
        let _ = self
            .game
            .consume(&GameEvent::RollResult { player_id, dice });

        // Drive automatic stages that require no player input.
        self.drive_automatic_stages();
    }

    /// Advance through stages that can be resolved without player input
    /// (MarkPoints, MarkAdvPoints).
    fn drive_automatic_stages(&mut self) {
        loop {
            // Stop if the game has already ended (stage transitions to Ended but
            // turn_stage may still be MarkPoints when schools_enabled=false, which
            // makes consume(Mark) a no-op and would cause an infinite loop).
            if self.game.stage == trictrac_store::Stage::Ended {
                break;
            }
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

impl TrictracBackend {
    pub fn get_game(&self) -> &GameState {
        &self.game
    }
}

impl BackEndArchitecture<PlayerAction, GameDelta, ViewState> for TrictracBackend {
    fn new(_rule_variation: u16) -> Self {
        let mut game = GameState::new(false);
        game.init_player("Host");
        game.init_player("Guest");

        let view_state = ViewState::from_game_state(&game, HOST_PLAYER_ID, GUEST_PLAYER_ID);

        TrictracBackend {
            game,
            dice_roller: DiceRoller::default(),
            commands: Vec::new(),
            view_state,
            arrived: [false; 2],
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
            self.commands
                .push(BackendCommand::KickPlayer { player: mp_player });
            return;
        }
        self.arrived[mp_player as usize] = true;

        // Cancel any reconnect timer for this player.
        self.commands.push(BackendCommand::CancelTimer {
            timer_id: mp_player,
        });

        // Start the game once both players have arrived.
        if self.arrived[0] && self.arrived[1] && self.game.stage == trictrac_store::Stage::PreGame {
            let _ = self.game.consume(&GameEvent::BeginGame {
                goes_first: HOST_PLAYER_ID,
            });
            self.sync_view_state();
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

        let store_id = if mp_player == 0 {
            HOST_PLAYER_ID
        } else {
            GUEST_PLAYER_ID
        };

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
            PlayerAction::Move(m1, m2) => {
                if self.game.turn_stage != TurnStage::Move
                    && self.game.turn_stage != TurnStage::HoldOrGoChoice
                {
                    return;
                }
                let event = GameEvent::Move {
                    player_id: store_id,
                    moves: (m1, m2),
                };
                if self.game.validate(&event) {
                    // let message = format!("Event {:?} validated on {:?}", event, self.game);
                    // console_log(message);
                    let _ = self.game.consume(&event);
                    self.drive_automatic_stages();
                }
            }
            PlayerAction::Go => {
                if self.game.turn_stage == TurnStage::HoldOrGoChoice {
                    let _ = self.game.consume(&GameEvent::Go {
                        player_id: store_id,
                    });
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

#[cfg(test)]
mod tests {
    use super::*;
    use backbone_lib::traits::BackEndArchitecture;

    fn make_backend() -> TrictracBackend {
        TrictracBackend::new(0)
    }

    /// Helper: drain and return only Delta commands, extracting their ViewStates.
    fn drain_deltas(b: &mut TrictracBackend) -> Vec<ViewState> {
        b.drain_commands()
            .into_iter()
            .filter_map(|cmd| match cmd {
                BackendCommand::Delta(d) => Some(d.state),
                BackendCommand::ResetViewState => Some(b.view_state.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn both_players_arrive_starts_game() {
        let mut b = make_backend();
        b.player_arrival(0); // host
        b.drain_commands();
        b.player_arrival(1); // guest
        let cmds = b.drain_commands();

        // ResetViewState should have been issued after BeginGame.
        let has_reset = cmds
            .iter()
            .any(|c| matches!(c, BackendCommand::ResetViewState));
        assert!(
            has_reset,
            "expected ResetViewState after both players arrive"
        );

        // Game should now be InGame.
        use crate::trictrac::types::SerStage;
        assert_eq!(b.get_view_state().stage, SerStage::InGame);
    }

    #[test]
    fn unknown_player_kicked() {
        let mut b = make_backend();
        b.player_arrival(99);
        let cmds = b.drain_commands();
        assert!(cmds
            .iter()
            .any(|c| matches!(c, BackendCommand::KickPlayer { player: 99 })));
    }

    #[test]
    fn roll_advances_to_move_or_hold() {
        let mut b = make_backend();
        b.player_arrival(0);
        b.player_arrival(1);
        b.drain_commands();

        // Host rolls (player_id 0, whose store id == HOST_PLAYER_ID == active after BeginGame).
        b.inform_rpc(0, PlayerAction::Roll);
        let states = drain_deltas(&mut b);
        assert!(!states.is_empty(), "expected a state broadcast after roll");

        use crate::trictrac::types::SerTurnStage;
        let last = states.last().unwrap();
        assert!(
            matches!(
                last.turn_stage,
                SerTurnStage::Move | SerTurnStage::HoldOrGoChoice
            ),
            "expected Move or HoldOrGoChoice after roll, got {:?}",
            last.turn_stage
        );
        assert_eq!(last.dice, b.get_view_state().dice);
        assert!(last.dice.0 >= 1 && last.dice.0 <= 6);
        assert!(last.dice.1 >= 1 && last.dice.1 <= 6);
    }

    #[test]
    fn wrong_player_roll_ignored() {
        let mut b = make_backend();
        b.player_arrival(0);
        b.player_arrival(1);
        b.drain_commands();

        // Guest tries to roll when it's the host's turn.
        b.inform_rpc(1, PlayerAction::Roll);
        let cmds = b.drain_commands();
        assert!(
            cmds.is_empty(),
            "guest roll should be ignored when it's host's turn"
        );
    }

    #[test]
    fn departure_sets_reconnect_timer() {
        let mut b = make_backend();
        b.player_arrival(0);
        b.drain_commands();
        b.player_departure(0);
        let cmds = b.drain_commands();
        assert!(
            cmds.iter()
                .any(|c| matches!(c, BackendCommand::SetTimer { timer_id: 0, .. })),
            "expected reconnect timer after host departure"
        );
    }

    #[test]
    fn timer_triggers_terminate_room() {
        let mut b = make_backend();
        b.timer_triggered(0);
        let cmds = b.drain_commands();
        assert!(cmds
            .iter()
            .any(|c| matches!(c, BackendCommand::TerminateRoom)));
    }
}

// ── Public API: WASM delegates to `inner`, other targets are no-ops ───────────

#[cfg(target_arch = "wasm32")]
mod inner {
    use web_sys::console;

    pub fn console_log(message: String) {
        console::log_1(&message.into());
    }
}

#[cfg(target_arch = "wasm32")]
pub use inner::console_log;

#[cfg(not(target_arch = "wasm32"))]
pub fn console_log(message: String) {}
