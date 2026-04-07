use futures::channel::mpsc;
use futures::{FutureExt, StreamExt};
use gloo_storage::{LocalStorage, Storage};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};

use backbone_lib::session::{ConnectError, GameSession, RoomConfig, RoomRole, SessionEvent};
use backbone_lib::traits::{BackEndArchitecture, BackendCommand, ViewStateUpdate};

use crate::components::{ConnectingScreen, GameScreen, LoginScreen};
use crate::i18n::I18nContextProvider;
use crate::trictrac::backend::TrictracBackend;
use crate::trictrac::bot_local::bot_decide;
use crate::trictrac::types::{GameDelta, JanEntry, PlayerAction, ScoredEvent, SerTurnStage, ViewState};

use std::collections::VecDeque;

const RELAY_URL: &str = "ws://127.0.0.1:8080/ws";
const GAME_ID: &str = "trictrac";
const STORAGE_KEY: &str = "trictrac_session";

/// The state the UI needs to render the game screen.
#[derive(Clone, PartialEq)]
pub struct GameUiState {
    pub view_state: ViewState,
    /// 0 = host, 1 = guest
    pub player_id: u16,
    pub room_id: String,
    pub is_bot_game: bool,
    /// True when this state is a buffered snapshot awaiting player confirmation.
    pub waiting_for_confirm: bool,
    /// Why we are paused — drives the status-bar message in GameScreen.
    pub pause_reason: Option<PauseReason>,
    /// Points scored by this player in the transition to this state (if any).
    pub my_scored_event: Option<ScoredEvent>,
    pub opp_scored_event: Option<ScoredEvent>,
}

/// Reason the UI is paused waiting for the player to click Continue.
#[derive(Clone, Debug, PartialEq)]
pub enum PauseReason {
    AfterOpponentRoll,
    AfterOpponentGo,
    AfterOpponentMove,
}

/// Which screen is currently shown.
#[derive(Clone, PartialEq)]
pub enum Screen {
    Login { error: Option<String> },
    Connecting,
    Playing(GameUiState),
}

/// Commands sent from UI event handlers into the network task.
pub enum NetCommand {
    CreateRoom {
        room: String,
    },
    JoinRoom {
        room: String,
    },
    Reconnect {
        relay_url: String,
        game_id: String,
        room_id: String,
        token: u64,
        host_state: Option<Vec<u8>>,
    },
    PlayVsBot,
    Action(PlayerAction),
    Disconnect,
}

/// Stored in localStorage to reconnect after a page refresh.
#[derive(Serialize, Deserialize)]
struct StoredSession {
    relay_url: String,
    game_id: String,
    room_id: String,
    token: u64,
    #[serde(default)]
    is_host: bool,
    #[serde(default)]
    view_state: Option<ViewState>,
}

fn save_session(session: &StoredSession) {
    LocalStorage::set(STORAGE_KEY, session).ok();
}

fn load_session() -> Option<StoredSession> {
    LocalStorage::get::<StoredSession>(STORAGE_KEY).ok()
}

fn clear_session() {
    LocalStorage::delete(STORAGE_KEY);
}

#[component]
pub fn App() -> impl IntoView {
    let stored = load_session();
    let initial_screen = if stored.is_some() {
        Screen::Connecting
    } else {
        Screen::Login { error: None }
    };
    let screen = RwSignal::new(initial_screen);

    let (cmd_tx, mut cmd_rx) = mpsc::unbounded::<NetCommand>();
    let pending: RwSignal<VecDeque<GameUiState>> = RwSignal::new(VecDeque::new());
    provide_context(pending);
    provide_context(cmd_tx.clone());

    if let Some(s) = stored {
        let host_state = s
            .view_state
            .as_ref()
            .and_then(|vs| serde_json::to_vec(vs).ok());
        cmd_tx
            .unbounded_send(NetCommand::Reconnect {
                relay_url: s.relay_url,
                game_id: s.game_id,
                room_id: s.room_id,
                token: s.token,
                host_state,
            })
            .ok();
    }

    spawn_local(async move {
        loop {
            // Wait for a connect/reconnect command (or PlayVsBot).
            // None means "play vs bot"; Some((config, is_reconnect)) means "connect to relay".
            let remote_config: Option<(RoomConfig, bool)> = loop {
                match cmd_rx.next().await {
                    Some(NetCommand::PlayVsBot) => break None,
                    Some(NetCommand::CreateRoom { room }) => {
                        break Some((
                            RoomConfig {
                                relay_url: RELAY_URL.to_string(),
                                game_id: GAME_ID.to_string(),
                                room_id: room,
                                rule_variation: 0,
                                role: RoomRole::Create,
                                reconnect_token: None,
                                host_state: None,
                            },
                            false,
                        ));
                    }
                    Some(NetCommand::JoinRoom { room }) => {
                        break Some((
                            RoomConfig {
                                relay_url: RELAY_URL.to_string(),
                                game_id: GAME_ID.to_string(),
                                room_id: room,
                                rule_variation: 0,
                                role: RoomRole::Join,
                                reconnect_token: None,
                                host_state: None,
                            },
                            false,
                        ));
                    }
                    Some(NetCommand::Reconnect {
                        relay_url,
                        game_id,
                        room_id,
                        token,
                        host_state,
                    }) => {
                        break Some((
                            RoomConfig {
                                relay_url,
                                game_id,
                                room_id,
                                rule_variation: 0,
                                role: RoomRole::Join,
                                reconnect_token: Some(token),
                                host_state,
                            },
                            true,
                        ));
                    }
                    _ => {} // Ignore game commands while disconnected.
                }
            };

            if remote_config.is_none() {
                loop {
                    let restart = run_local_bot_game(screen, &mut cmd_rx, pending).await;
                    if !restart { break; }
                }
                pending.update(|q| q.clear());
                screen.set(Screen::Login { error: None });
                continue;
            }
            let (config, is_reconnect) = remote_config.unwrap();

            screen.set(Screen::Connecting);

            let room_id_for_storage = config.room_id.clone();
            let mut session: GameSession<PlayerAction, GameDelta, ViewState> =
                match GameSession::connect::<TrictracBackend>(config).await {
                    Ok(s) => s,
                    Err(ConnectError::WebSocket(e) | ConnectError::Handshake(e)) => {
                        if is_reconnect {
                            clear_session();
                        }
                        screen.set(Screen::Login { error: Some(e) });
                        continue;
                    }
                };

            if !session.is_host {
                save_session(&StoredSession {
                    relay_url: RELAY_URL.to_string(),
                    game_id: GAME_ID.to_string(),
                    room_id: room_id_for_storage.clone(),
                    token: session.reconnect_token,
                    is_host: false,
                    view_state: None,
                });
            }

            let is_host = session.is_host;
            let player_id = session.player_id;
            let reconnect_token = session.reconnect_token;
            let mut vs = ViewState::default_with_names("Host", "Guest");

            loop {
                futures::select! {
                    cmd = cmd_rx.next().fuse() => match cmd {
                        Some(NetCommand::Action(action)) => {
                            session.send_action(action);
                        }
                        _ => {
                            clear_session();
                            session.disconnect();
                            pending.update(|q| q.clear());
                            screen.set(Screen::Login { error: None });
                            break;
                        }
                    },
                    event = session.next_event().fuse() => match event {
                        Some(SessionEvent::Update(u)) => {
                            let prev_vs = vs.clone();
                            match u {
                                ViewStateUpdate::Full(state) => vs = state,
                                ViewStateUpdate::Incremental(delta) => vs.apply_delta(&delta),
                            }
                            if is_host {
                                save_session(&StoredSession {
                                    relay_url: RELAY_URL.to_string(),
                                    game_id: GAME_ID.to_string(),
                                    room_id: room_id_for_storage.clone(),
                                    token: reconnect_token,
                                    is_host: true,
                                    view_state: Some(vs.clone()),
                                });
                            }
                            push_or_show(
                                &prev_vs,
                                GameUiState {
                                    view_state: vs.clone(),
                                    player_id,
                                    room_id: room_id_for_storage.clone(),
                                    is_bot_game: false,
                                    waiting_for_confirm: false,
                                    pause_reason: None,
                                    my_scored_event: None,
                                    opp_scored_event: None,
                                },
                                pending,
                                screen,
                            );
                        }
                        Some(SessionEvent::Disconnected(reason)) => {
                            pending.update(|q| q.clear());
                            screen.set(Screen::Login { error: reason });
                            break;
                        }
                        None => {
                            pending.update(|q| q.clear());
                            screen.set(Screen::Login { error: None });
                            break;
                        }
                    }
                }
            }
        }
    });

    view! {
        <I18nContextProvider>
            {move || {
                let q = pending.get();
                if let Some(front) = q.front() {
                    view! { <GameScreen state=front.clone() /> }.into_any()
                } else {
                    match screen.get() {
                        Screen::Login { error } => view! { <LoginScreen error=error /> }.into_any(),
                        Screen::Connecting => view! { <ConnectingScreen /> }.into_any(),
                        Screen::Playing(state) => view! { <GameScreen state=state /> }.into_any(),
                    }
                }
            }}
        </I18nContextProvider>
    }
}

/// Runs one local bot game. Returns `true` if the player wants to play again.
async fn run_local_bot_game(
    screen: RwSignal<Screen>,
    cmd_rx: &mut futures::channel::mpsc::UnboundedReceiver<NetCommand>,
    pending: RwSignal<VecDeque<GameUiState>>,
) -> bool {
    let mut backend = TrictracBackend::new(0);
    backend.player_arrival(0);
    backend.player_arrival(1);

    let mut vs = ViewState::default_with_names("You", "Bot");
    for cmd in backend.drain_commands() {
        match cmd {
            BackendCommand::ResetViewState => { vs = backend.get_view_state().clone(); }
            BackendCommand::Delta(delta) => { vs.apply_delta(&delta); }
            _ => {}
        }
    }
    screen.set(Screen::Playing(GameUiState {
        view_state: vs.clone(),
        player_id: 0,
        room_id: String::new(),
        is_bot_game: true,
        waiting_for_confirm: false,
        pause_reason: None,
        my_scored_event: None,
        opp_scored_event: None,
    }));

    loop {
        match cmd_rx.next().await {
            Some(NetCommand::Action(action)) => {
                let prev_vs = vs.clone();
                backend.inform_rpc(0, action);
                for cmd in backend.drain_commands() {
                    if let BackendCommand::Delta(delta) = cmd {
                        vs.apply_delta(&delta);
                    }
                }
                let scored = compute_scored_event(&prev_vs, &vs, 0);
                let opp_scored = compute_scored_event(&prev_vs, &vs, 1);
                screen.set(Screen::Playing(GameUiState {
                    view_state: vs.clone(),
                    player_id: 0,
                    room_id: String::new(),
                    is_bot_game: true,
                    waiting_for_confirm: false,
                    pause_reason: None,
                    my_scored_event: scored,
                    opp_scored_event: opp_scored,
                }));
            }
            Some(NetCommand::PlayVsBot) => return true,
            _ => return false,
        }

        loop {
            match bot_decide(backend.get_game()) {
                None => break,
                Some(action) => {
                    let prev_vs = vs.clone();
                    backend.inform_rpc(1, action);
                    for cmd in backend.drain_commands() {
                        if let BackendCommand::Delta(delta) = cmd {
                            vs.apply_delta(&delta);
                        }
                    }
                    push_or_show(
                        &prev_vs,
                        GameUiState {
                            view_state: vs.clone(),
                            player_id: 0,
                            room_id: String::new(),
                            is_bot_game: true,
                            waiting_for_confirm: false,
                            pause_reason: None,
                            my_scored_event: None,
                            opp_scored_event: None,
                        },
                        pending,
                        screen,
                    );
                }
            }
        }
    }
}

/// Computes a scoring event for `player_id` by comparing the previous and next
/// ViewState. Returns `None` when no points changed for that player.
fn compute_scored_event(prev: &ViewState, next: &ViewState, player_id: u16) -> Option<ScoredEvent> {
    let prev_score = &prev.scores[player_id as usize];
    let next_score = &next.scores[player_id as usize];

    let holes_gained = next_score.holes.saturating_sub(prev_score.holes);
    if holes_gained == 0 && prev_score.points == next_score.points {
        return None;
    }

    let bredouille = holes_gained > 0 && prev_score.can_bredouille;

    // Determine which dice_jans are "mine" depending on who was the active roller.
    let my_jans: Vec<JanEntry> = if next.active_mp_player == Some(player_id)
        && prev.active_mp_player == Some(player_id)
    {
        // My own roll: positive totals are mine.
        next.dice_jans.iter().filter(|e| e.total > 0).cloned().collect()
    } else if next.active_mp_player == Some(player_id)
        && prev.active_mp_player != Some(player_id)
    {
        // Opponent just moved: negative totals (their penalty) are scored for me.
        next.dice_jans
            .iter()
            .filter(|e| e.total < 0)
            .map(|e| JanEntry { total: -e.total, points_per: -e.points_per, ..e.clone() })
            .collect()
    } else {
        return None;
    };

    let points_earned: u8 = my_jans
        .iter()
        .fold(0u8, |acc, e| acc.saturating_add(e.total.unsigned_abs()));

    if points_earned == 0 && holes_gained == 0 {
        return None;
    }

    Some(ScoredEvent {
        points_earned,
        holes_gained,
        holes_total: next_score.holes,
        bredouille,
        jans: my_jans,
    })
}

/// Either queues the state as a buffered confirmation step (when the transition
/// warrants a pause) or shows it immediately. Always updates `screen` to the
/// live state so the UI falls through to the right content once pending drains.
fn push_or_show(
    prev_vs: &ViewState,
    new_state: GameUiState,
    pending: RwSignal<VecDeque<GameUiState>>,
    screen: RwSignal<Screen>,
) {
    let scored = compute_scored_event(prev_vs, &new_state.view_state, new_state.player_id);
    let opp_scored = compute_scored_event(prev_vs, &new_state.view_state, 1 - new_state.player_id);

    if let Some(reason) = infer_pause_reason(prev_vs, &new_state.view_state, new_state.player_id) {
        // Scoring notifications go on the buffered (paused) state only.
        pending.update(|q| {
            q.push_back(GameUiState {
                waiting_for_confirm: true,
                pause_reason: Some(reason),
                my_scored_event: scored,
                opp_scored_event: opp_scored,
                ..new_state.clone()
            });
        });
        screen.set(Screen::Playing(new_state));
    } else {
        // No pause: show scoring directly on the live state.
        screen.set(Screen::Playing(GameUiState {
            my_scored_event: scored,
            opp_scored_event: opp_scored,
            ..new_state
        }));
    }
}

/// Compares the previous and next ViewState to decide whether the transition
/// warrants a confirmation pause. Returns None when it is the local player's
/// own action (no pause needed).
fn infer_pause_reason(prev: &ViewState, next: &ViewState, player_id: u16) -> Option<PauseReason> {
    let opponent_id = 1 - player_id;

    if next.active_mp_player == Some(opponent_id) {
        // Dice changed → opponent just rolled.
        if next.dice != prev.dice {
            return Some(PauseReason::AfterOpponentRoll);
        }
        // Was at HoldOrGoChoice, now Move, opponent still active → opponent went.
        if prev.turn_stage == SerTurnStage::HoldOrGoChoice
            && next.turn_stage == SerTurnStage::Move
        {
            return Some(PauseReason::AfterOpponentGo);
        }
    }

    // Turn switched to us → opponent moved.
    if next.active_mp_player == Some(player_id) && prev.active_mp_player == Some(opponent_id) {
        return Some(PauseReason::AfterOpponentMove);
    }

    None
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::trictrac::types::{PlayerScore, SerStage, SerTurnStage};

    fn score() -> PlayerScore {
        PlayerScore { name: String::new(), points: 0, holes: 0, can_bredouille: false }
    }

    fn vs(dice: (u8, u8), turn_stage: SerTurnStage, active: Option<u16>) -> ViewState {
        ViewState {
            board: [0i8; 24],
            stage: SerStage::InGame,
            turn_stage,
            active_mp_player: active,
            scores: [score(), score()],
            dice,
            dice_jans: Vec::new(),
        }
    }

    #[test]
    fn dice_change_is_after_roll() {
        let prev = vs((0, 0), SerTurnStage::RollDice, Some(1));
        let next = vs((3, 5), SerTurnStage::Move, Some(1));
        assert_eq!(infer_pause_reason(&prev, &next, 0), Some(PauseReason::AfterOpponentRoll));
    }

    #[test]
    fn hold_to_move_is_after_go() {
        let prev = vs((3, 5), SerTurnStage::HoldOrGoChoice, Some(1));
        let next = vs((3, 5), SerTurnStage::Move, Some(1));
        assert_eq!(infer_pause_reason(&prev, &next, 0), Some(PauseReason::AfterOpponentGo));
    }

    #[test]
    fn turn_switch_is_after_move() {
        let prev = vs((3, 5), SerTurnStage::Move, Some(1));
        let next = vs((3, 5), SerTurnStage::RollDice, Some(0));
        assert_eq!(infer_pause_reason(&prev, &next, 0), Some(PauseReason::AfterOpponentMove));
    }

    #[test]
    fn own_action_returns_none() {
        let prev = vs((0, 0), SerTurnStage::RollDice, Some(0));
        let next = vs((2, 4), SerTurnStage::Move, Some(0));
        assert_eq!(infer_pause_reason(&prev, &next, 0), None);
    }

    #[test]
    fn no_active_player_returns_none() {
        let mut prev = vs((0, 0), SerTurnStage::RollDice, None);
        prev.stage = SerStage::PreGame;
        let mut next = prev.clone();
        next.active_mp_player = Some(0);
        assert_eq!(infer_pause_reason(&prev, &next, 0), None);
    }
}
