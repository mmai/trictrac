use futures::channel::mpsc;
use futures::{FutureExt, StreamExt};
use gloo_storage::{LocalStorage, Storage};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};

use backbone_lib::session::{ConnectError, GameSession, RoomConfig, RoomRole, SessionEvent};
use backbone_lib::traits::ViewStateUpdate;

use crate::components::{ConnectingScreen, GameScreen, LoginScreen};
use crate::trictrac::backend::TrictracBackend;
use crate::trictrac::types::{GameDelta, PlayerAction, ViewState};

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
            // Wait for a connect/reconnect command.
            let (config, is_reconnect) = loop {
                match cmd_rx.next().await {
                    Some(NetCommand::CreateRoom { room }) => {
                        break (
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
                        );
                    }
                    Some(NetCommand::JoinRoom { room }) => {
                        break (
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
                        );
                    }
                    Some(NetCommand::Reconnect {
                        relay_url,
                        game_id,
                        room_id,
                        token,
                        host_state,
                    }) => {
                        break (
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
                        );
                    }
                    _ => {} // Ignore game commands while disconnected.
                }
            };

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
                            screen.set(Screen::Login { error: None });
                            break;
                        }
                    },
                    event = session.next_event().fuse() => match event {
                        Some(SessionEvent::Update(u)) => {
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
                            screen.set(Screen::Playing(GameUiState {
                                view_state: vs.clone(),
                                player_id,
                                room_id: room_id_for_storage.clone(),
                            }));
                        }
                        Some(SessionEvent::Disconnected(reason)) => {
                            screen.set(Screen::Login { error: reason });
                            break;
                        }
                        None => {
                            screen.set(Screen::Login { error: None });
                            break;
                        }
                    }
                }
            }
        }
    });

    view! {
        {move || match screen.get() {
            Screen::Login { error } => view! { <LoginScreen error=error /> }.into_any(),
            Screen::Connecting => view! { <ConnectingScreen /> }.into_any(),
            Screen::Playing(state) => view! { <GameScreen state=state /> }.into_any(),
        }}
    }
}
