use backbone_lib::traits::{BackEndArchitecture, BackendCommand};
use trictrac_store::{Color, Dice, DiceRoller, GameEvent, GameState, Player, Stage, TurnStage};

use super::types::{GameDelta, PlayerAction, PreGameRollState, SerStage, SerTurnStage, ViewState};

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
    /// Die rolled by each player during the ceremony ([host, guest]).
    pre_game_dice: [Option<u8>; 2],
    /// Number of tied rounds so far.
    tie_count: u8,
    /// True while the first-player ceremony is running.
    ceremony_started: bool,
}

impl TrictracBackend {
    fn sync_view_state(&mut self) {
        let mut vs = ViewState::from_game_state(&self.game, HOST_PLAYER_ID, GUEST_PLAYER_ID);
        if self.ceremony_started {
            vs.stage = SerStage::PreGameRoll;
            vs.pre_game_roll = Some(PreGameRollState {
                host_die: self.pre_game_dice[0],
                guest_die: self.pre_game_dice[1],
                tie_count: self.tie_count,
            });
            // Both players roll independently; no single "active" player.
            vs.active_mp_player = None;
        }
        self.view_state = vs;
    }

    fn broadcast_state(&mut self) {
        self.sync_view_state();
        let delta = GameDelta {
            state: self.view_state.clone(),
        };
        self.commands.push(BackendCommand::Delta(delta));
    }

    /// Process one ceremony die-roll for `mp_player` (0 = host, 1 = guest).
    fn handle_pre_game_roll(&mut self, mp_player: u16) {
        let idx = mp_player as usize;
        // Ignore if this player already rolled.
        if self.pre_game_dice[idx].is_some() {
            return;
        }
        let single = self.dice_roller.roll().values.0;
        self.pre_game_dice[idx] = Some(single);

        if let [Some(h), Some(g)] = self.pre_game_dice {
            // Both have rolled — broadcast both dice before resolving.
            self.broadcast_state();
            if h == g {
                // Tie: reset for another round.
                self.tie_count += 1;
                self.pre_game_dice = [None; 2];
                self.broadcast_state();
            } else {
                // Highest die goes first.
                let goes_first = if h > g {
                    HOST_PLAYER_ID
                } else {
                    GUEST_PLAYER_ID
                };
                self.ceremony_started = false;
                let _ = self.game.consume(&GameEvent::BeginGame { goes_first });
                // Use pre-game dice roll for the first move
                let _ = self.game.consume(&GameEvent::Roll {
                    player_id: goes_first,
                });
                let _ = self.game.consume(&GameEvent::RollResult {
                    player_id: goes_first,
                    dice: Dice { values: (g, h) },
                });
                self.broadcast_state();
            }
        } else {
            // Only one die rolled so far — broadcast the partial result.
            self.broadcast_state();
        }
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

    /// Build a backend pre-loaded with the given `ViewState` snapshot so a bot
    /// game can resume from an arbitrary position (debug feature).
    pub fn from_view_state(vs: ViewState, player_name: &str) -> Self {
        let mut game = GameState::new(false);

        game.board.set_positions(&Color::White, vs.board);

        game.stage = match vs.stage {
            SerStage::InGame => Stage::InGame,
            SerStage::Ended => Stage::Ended,
            _ => Stage::InGame,
        };

        game.turn_stage = match vs.turn_stage {
            SerTurnStage::RollDice => TurnStage::RollDice,
            SerTurnStage::RollWaiting => TurnStage::RollWaiting,
            SerTurnStage::MarkPoints => TurnStage::MarkPoints,
            SerTurnStage::HoldOrGoChoice => TurnStage::HoldOrGoChoice,
            SerTurnStage::Move => TurnStage::Move,
            SerTurnStage::MarkAdvPoints => TurnStage::MarkAdvPoints,
        };

        game.dice = Dice { values: vs.dice };

        game.active_player_id = match vs.active_mp_player {
            Some(0) => HOST_PLAYER_ID,
            Some(1) => GUEST_PLAYER_ID,
            _ => HOST_PLAYER_ID,
        };

        let build_player = |score: &crate::game::trictrac::types::PlayerScore,
                             color: Color|
         -> Player {
            let mut p = Player::new(score.name.clone(), color);
            p.points = score.points;
            p.holes = score.holes;
            p.can_bredouille = score.can_bredouille;
            p
        };

        game.players.insert(HOST_PLAYER_ID, build_player(&vs.scores[0], Color::White));
        game.players.insert(GUEST_PLAYER_ID, build_player(&vs.scores[1], Color::Black));

        let mut view_state = ViewState::from_game_state(&game, HOST_PLAYER_ID, GUEST_PLAYER_ID);
        view_state.scores[0].name = player_name.to_string();
        view_state.scores[1].name = "Bot".to_string();

        TrictracBackend {
            game,
            dice_roller: DiceRoller::default(),
            commands: Vec::new(),
            view_state,
            arrived: [true, true],
            pre_game_dice: [None; 2],
            tie_count: 0,
            ceremony_started: false,
        }
    }
}

impl BackEndArchitecture<PlayerAction, GameDelta, ViewState> for TrictracBackend {
    fn new(_rule_variation: u16) -> Self {
        let mut game = GameState::new(false);
        game.init_player("Blancs");
        game.init_player("Noirs");

        let view_state = ViewState::from_game_state(&game, HOST_PLAYER_ID, GUEST_PLAYER_ID);

        TrictracBackend {
            game,
            dice_roller: DiceRoller::default(),
            commands: Vec::new(),
            view_state,
            arrived: [false; 2],
            pre_game_dice: [None; 2],
            tie_count: 0,
            ceremony_started: false,
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

        // Start the ceremony once both players have arrived.
        if self.arrived[0]
            && self.arrived[1]
            && self.game.stage == trictrac_store::Stage::PreGame
            && !self.ceremony_started
        {
            self.ceremony_started = true;
            self.pre_game_dice = [None; 2];
            self.tie_count = 0;
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
        // SetName is always accepted regardless of game stage or whose turn it is.
        if let PlayerAction::SetName(name) = action {
            let store_id = if mp_player == 0 { HOST_PLAYER_ID } else { GUEST_PLAYER_ID };
            if let Some(p) = self.game.players.get_mut(&store_id) {
                p.name = name;
            }
            self.broadcast_state();
            return;
        }

        // During the first-player ceremony only PreGameRoll actions are accepted.
        if self.ceremony_started {
            if matches!(action, PlayerAction::PreGameRoll) {
                self.handle_pre_game_roll(mp_player);
            }
            return;
        }

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
            PlayerAction::PreGameRoll => {} // ignored outside ceremony
            PlayerAction::SetName(_) => {}  // handled at the top of inform_rpc
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
    use super::{SerStage, SerTurnStage};
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

    /// Drive the ceremony to completion (both players roll until one wins).
    fn complete_ceremony(b: &mut TrictracBackend) {
        loop {
            if b.get_view_state().stage != SerStage::PreGameRoll {
                break;
            }
            let pgr = b.get_view_state().pre_game_roll.clone().unwrap_or_default();
            let host_needs = pgr.host_die.is_none();
            let guest_needs = pgr.guest_die.is_none();
            if !host_needs && !guest_needs {
                break; // both rolled but stage not yet resolved — shouldn't happen
            }
            if host_needs {
                b.inform_rpc(0, PlayerAction::PreGameRoll);
            }
            if guest_needs {
                b.inform_rpc(1, PlayerAction::PreGameRoll);
            }
            b.drain_commands();
        }
    }

    #[test]
    fn both_players_arrive_starts_ceremony() {
        let mut b = make_backend();
        b.player_arrival(0); // host
        b.drain_commands();
        b.player_arrival(1); // guest
        let cmds = b.drain_commands();

        // ResetViewState should have been issued to start the ceremony.
        let has_reset = cmds
            .iter()
            .any(|c| matches!(c, BackendCommand::ResetViewState));
        assert!(
            has_reset,
            "expected ResetViewState after both players arrive"
        );

        // Stage should now be PreGameRoll, not InGame.
        assert_eq!(b.get_view_state().stage, SerStage::PreGameRoll);
    }

    #[test]
    fn ceremony_resolves_to_in_game() {
        let mut b = make_backend();
        b.player_arrival(0);
        b.player_arrival(1);
        b.drain_commands();

        complete_ceremony(&mut b);

        assert_eq!(b.get_view_state().stage, SerStage::InGame);
    }

    #[test]
    fn ceremony_any_order_allowed() {
        let mut b = make_backend();
        b.player_arrival(0);
        b.player_arrival(1);
        b.drain_commands();

        // Guest may roll before host.
        b.inform_rpc(1, PlayerAction::PreGameRoll);
        let states = drain_deltas(&mut b);
        assert!(
            !states.is_empty(),
            "guest PreGameRoll should broadcast a state"
        );
        let pgr = states.last().unwrap().pre_game_roll.as_ref().unwrap();
        assert!(
            pgr.guest_die.is_some(),
            "guest die should be set after guest rolls"
        );
        assert!(pgr.host_die.is_none(), "host die should still be blank");
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

        // Complete ceremony before rolling.
        complete_ceremony(&mut b);

        // Roll for whoever won the ceremony (either player could go first).
        let first_player = b
            .get_view_state()
            .active_mp_player
            .expect("someone should be active");
        b.inform_rpc(first_player, PlayerAction::Roll);
        let states = drain_deltas(&mut b);
        assert!(!states.is_empty(), "expected a state broadcast after roll");

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
        complete_ceremony(&mut b);

        // Identify who goes first and have the OTHER player try to roll.
        let active = b.get_view_state().active_mp_player;
        let wrong_player = if active == Some(0) { 1u16 } else { 0u16 };
        b.inform_rpc(wrong_player, PlayerAction::Roll);
        let cmds = b.drain_commands();
        assert!(cmds.is_empty(), "wrong player roll should be ignored");
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
