use futures::channel::mpsc;
use futures::{FutureExt, StreamExt};
use gloo_storage::{LocalStorage, Storage};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::{Route, Router, Routes};
use leptos_router::hooks::use_location;
use leptos_router::path;
use serde::{Deserialize, Serialize};

use backbone_lib::session::{ConnectError, GameSession, RoomConfig, RoomRole, SessionEvent};
use backbone_lib::traits::ViewStateUpdate;

use crate::api;
use crate::game::components::{ConnectingScreen, GameScreen};
use crate::game::session::{
    compute_last_moves, patch_player_name, push_or_show, run_local_bot_game,
};
use crate::game::trictrac::backend::TrictracBackend;
use crate::game::trictrac::types::{GameDelta, PlayerAction, ScoredEvent, SerStage, ViewState};
use crate::i18n::*;
use crate::nav::SiteNav;
use crate::portal::{
    account::AccountPage, game_detail::GameDetailPage, lobby::LobbyPage, profile::ProfilePage,
};
use trictrac_store::CheckerMove;

use std::collections::VecDeque;

const RELAY_URL: &str = "ws://localhost:8080/ws";
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
    pub waiting_for_confirm: bool,
    pub pause_reason: Option<PauseReason>,
    pub my_scored_event: Option<ScoredEvent>,
    pub opp_scored_event: Option<ScoredEvent>,
    pub last_moves: Option<(CheckerMove, CheckerMove)>,
}

/// Reason the UI is paused waiting for the player to click Continue.
#[derive(Clone, Debug, PartialEq)]
pub enum PauseReason {
    AfterOpponentRoll,
    AfterOpponentGo,
    AfterOpponentMove,
    AfterOpponentPreGameRoll,
}

/// Which screen is currently shown (used to toggle game overlay).
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

async fn submit_game_result(room_code: String, game_state: ViewState) {
    let [score_pl1, score_pl2] = game_state.scores;
    let result_str = format!("{:?} - {:?}", score_pl1.holes, score_pl2.holes);
    let outcomes = if score_pl1.holes < score_pl2.holes {
        [("0", "loss"), ("1", "win")]
    } else if score_pl2.holes < score_pl1.holes {
        [("0", "win"), ("1", "loss")]
    } else {
        [("0", "draw"), ("1", "draw")]
    };
    let body = serde_json::json!({
        "room_code": room_code,
        "game_id":   GAME_ID,
        "result":    result_str,
        "outcomes":  std::collections::HashMap::from(outcomes),
    });
    let _ = gloo_net::http::Request::post(&format!("{}/games/result", api::HTTP_BASE))
        .credentials(web_sys::RequestCredentials::Include)
        .json(&body)
        .unwrap()
        .send()
        .await;
}

#[component]
pub fn App() -> impl IntoView {
    let i18n = use_i18n();
    let stored = load_session();
    let initial_screen = if stored.is_some() {
        Screen::Connecting
    } else {
        Screen::Login { error: None }
    };
    let screen: RwSignal<Screen> = RwSignal::new(initial_screen);
    provide_context(screen);

    // Auth: fetch once on load; shared by nav + game + portal components.
    let auth_username: RwSignal<Option<String>> = RwSignal::new(None);
    provide_context(auth_username);
    spawn_local(async move {
        if let Ok(me) = api::get_me().await {
            auth_username.set(Some(me.username));
        }
    });

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
                    _ => {}
                }
            };

            if remote_config.is_none() {
                let player_name = auth_username
                    .get_untracked()
                    .unwrap_or_else(|| untrack(|| t_string!(i18n, anonymous_name).to_string()));
                loop {
                    let restart =
                        run_local_bot_game(screen, &mut cmd_rx, pending, player_name.clone()).await;
                    if !restart {
                        break;
                    }
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
            let my_name = auth_username
                .get_untracked()
                .unwrap_or_else(|| t_string!(i18n, anonymous_name).to_string());
            let mut vs = ViewState::default_with_names("", "");
            let mut result_submitted = false;

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
                            patch_player_name(&mut vs, player_id, &my_name);

                            if is_host && !result_submitted && vs.stage == SerStage::Ended {
                                result_submitted = true;
                                let room = room_id_for_storage.clone();
                                let gs = vs.clone();
                                spawn_local(submit_game_result(room, gs));
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
                            let is_own_move = prev_vs.active_mp_player == Some(player_id);
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
                                    last_moves: compute_last_moves(&prev_vs, &vs, is_own_move),
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
        <Router>
            <SiteNav />

            <main>
                <Routes fallback=|| view! { <p class="portal-empty" style="padding:3rem;text-align:center">"Page not found."</p> }>
                    <Route path=path!("/") view=LobbyPage />
                    <Route path=path!("/account") view=AccountPage />
                    <Route path=path!("/profile/:username") view=ProfilePage />
                    <Route path=path!("/games/:id") view=GameDetailPage />
                </Routes>
            </main>

            <GameOverlay pending=pending screen=screen />
        </Router>
    }
}

/// Renders the full-screen game overlay, but only when the current route is "/".
/// This lets the user navigate to profile/account pages while a game is running.
#[component]
fn GameOverlay(
    pending: RwSignal<VecDeque<GameUiState>>,
    screen: RwSignal<Screen>,
) -> impl IntoView {
    let location = use_location();

    move || {
        if location.pathname.get() != "/" {
            return view! {}.into_any();
        }
        let q = pending.get();
        let front = q.front().cloned();
        if let Some(state) = front {
            return view! {
                <div class="game-overlay"><GameScreen state /></div>
            }
            .into_any();
        }
        match screen.get() {
            Screen::Playing(state) => view! {
                <div class="game-overlay"><GameScreen state /></div>
            }
            .into_any(),
            Screen::Connecting => view! {
                <div class="game-overlay"><ConnectingScreen /></div>
            }
            .into_any(),
            _ => view! {}.into_any(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::session::infer_pause_reason;
    use crate::game::trictrac::types::{PlayerScore, SerStage, SerTurnStage};

    fn score() -> PlayerScore {
        PlayerScore {
            name: String::new(),
            points: 0,
            holes: 0,
            can_bredouille: false,
        }
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
            dice_moves: (CheckerMove::default(), CheckerMove::default()),
            pre_game_roll: None,
        }
    }

    #[test]
    fn dice_change_is_after_roll() {
        let prev = vs((0, 0), SerTurnStage::RollDice, Some(1));
        let next = vs((3, 5), SerTurnStage::Move, Some(1));
        assert_eq!(
            infer_pause_reason(&prev, &next, 0),
            Some(PauseReason::AfterOpponentRoll)
        );
    }

    #[test]
    fn hold_to_move_is_after_go() {
        let prev = vs((3, 5), SerTurnStage::HoldOrGoChoice, Some(1));
        let next = vs((3, 5), SerTurnStage::Move, Some(1));
        assert_eq!(
            infer_pause_reason(&prev, &next, 0),
            Some(PauseReason::AfterOpponentGo)
        );
    }

    #[test]
    fn turn_switch_is_after_move() {
        let prev = vs((3, 5), SerTurnStage::Move, Some(1));
        let next = vs((3, 5), SerTurnStage::RollDice, Some(0));
        assert_eq!(
            infer_pause_reason(&prev, &next, 0),
            Some(PauseReason::AfterOpponentMove)
        );
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
