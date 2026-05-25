use futures::channel::mpsc;
use futures::{FutureExt, StreamExt};
use gloo_storage::{LocalStorage, Storage};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::{Route, Router, Routes, A};
use leptos_router::hooks::use_location;
use leptos_router::path;
use serde::{Deserialize, Serialize};

use backbone_lib::session::{ConnectError, GameSession, RoomConfig, RoomRole, SessionEvent};
use backbone_lib::traits::ViewStateUpdate;

use crate::api;
use crate::game::components::{ConnectingScreen, GameScreen};
use crate::game::session::{
    compute_last_moves, patch_player_name, push_or_show, run_local_bot_game,
    run_local_bot_game_with_backend,
};
use crate::game::trictrac::backend::TrictracBackend;
use crate::game::trictrac::types::{GameDelta, PlayerAction, ScoredEvent, SerStage, ViewState};
use crate::i18n::*;
use crate::portal::{
    account::AccountPage, content_page::ContentPage, forgot_password::ForgotPasswordPage,
    game_detail::GameDetailPage, lobby::LobbyPage, profile::ProfilePage,
    reset_password::ResetPasswordPage, verify_email::VerifyEmailPage,
};
use trictrac_store::CheckerMove;

use std::collections::VecDeque;

/// Newtype wrappers so context lookup can distinguish signals of the same inner type.
#[derive(Clone, Copy)]
pub(crate) struct AnonNickname(pub RwSignal<Option<String>>);
#[derive(Clone, Copy)]
pub(crate) struct AuthEmailVerified(pub RwSignal<bool>);
/// One-shot message shown as a top banner and auto-dismissed after a few seconds.
#[derive(Clone, Copy)]
pub(crate) struct FlashMessage(pub RwSignal<Option<String>>);

fn relay_url() -> String {
    #[cfg(debug_assertions)]
    {
        "ws://localhost:8080/ws".to_string()
    }
    #[cfg(not(debug_assertions))]
    {
        let location = web_sys::window().and_then(|w| Some(w.location())).unwrap();
        let protocol = location.protocol().unwrap_or_default();
        let host = location.host().unwrap_or_default();
        let ws_protocol = if protocol == "https:" { "wss" } else { "ws" };
        format!("{ws_protocol}://{host}/ws")
    }
}
const GAME_ID: &str = "trictrac";
const STORAGE_KEY: &str = "trictrac_session";
const VERSION: &str = env!("CARGO_PKG_VERSION");

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
    /// True on the echo screen state set alongside a pending item — suppresses dice
    /// roll animation and sound since they already played on the pending screen.
    pub suppress_dice_anim: bool,
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
    /// Start a bot game with the board/score position from a previously taken snapshot.
    ReplaySnapshot(ViewState),
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
    let auth_email_verified: RwSignal<bool> = RwSignal::new(false);
    provide_context(auth_username);
    provide_context(AuthEmailVerified(auth_email_verified));
    // Set to true once get_me resolves (success or failure) so lobby can
    // decide immediately whether to show the nickname modal.
    let auth_loaded: RwSignal<bool> = RwSignal::new(false);
    provide_context(auth_loaded);
    // Nickname chosen by an anonymous player; used instead of "Anonymous".
    let anon_nickname: RwSignal<Option<String>> = RwSignal::new(None);
    provide_context(AnonNickname(anon_nickname));
    let flash: RwSignal<Option<String>> = RwSignal::new(None);
    provide_context(FlashMessage(flash));
    spawn_local(async move {
        if let Ok(me) = api::get_me().await {
            auth_username.set(Some(me.username));
            auth_email_verified.set(me.email_verified);
        }
        auth_loaded.set(true);
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
            let mut snapshot_init: Option<ViewState> = None;
            let remote_config: Option<(RoomConfig, bool)> = loop {
                match cmd_rx.next().await {
                    Some(NetCommand::PlayVsBot) => break None,
                    Some(NetCommand::ReplaySnapshot(vs)) => {
                        snapshot_init = Some(vs);
                        break None;
                    }
                    Some(NetCommand::CreateRoom { room }) => {
                        break Some((
                            RoomConfig {
                                relay_url: relay_url(),
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
                                relay_url: relay_url(),
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
                    .or_else(|| anon_nickname.get_untracked())
                    .unwrap_or_else(|| untrack(|| t_string!(i18n, anonymous_name).to_string()));
                loop {
                    let restart = match snapshot_init.take() {
                        Some(vs) => {
                            let backend = TrictracBackend::from_view_state(vs, &player_name);
                            run_local_bot_game_with_backend(
                                screen,
                                &mut cmd_rx,
                                pending,
                                player_name.clone(),
                                backend,
                            )
                            .await
                        }
                        None => {
                            run_local_bot_game(screen, &mut cmd_rx, pending, player_name.clone())
                                .await
                        }
                    };
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
                    relay_url: relay_url(),
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
                .or_else(|| anon_nickname.get_untracked())
                .unwrap_or_else(|| t_string!(i18n, anonymous_name).to_string());
            // Announce our name to the host backend so it can broadcast it to
            // the opponent. Done once immediately after connecting.
            session.send_action(PlayerAction::SetName(my_name.clone()));
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
                                    relay_url: relay_url(),
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
                                    suppress_dice_anim: false,
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
            <SiteHamburger />
            <FlashBanner />
            <main>
                <Routes fallback=|| view! { <p class="portal-empty" style="padding:3rem;text-align:center">"Page not found."</p> }>
                    <Route path=path!("/") view=LobbyPage />
                    <Route path=path!("/account") view=AccountPage />
                    <Route path=path!("/profile/:username") view=ProfilePage />
                    <Route path=path!("/games/:id") view=GameDetailPage />
                    <Route path=path!("/verify-email") view=VerifyEmailPage />
                    <Route path=path!("/forgot-password") view=ForgotPasswordPage />
                    <Route path=path!("/reset-password") view=ResetPasswordPage />
                    <Route path=path!("/page/:slug") view=ContentPage />
                </Routes>
            </main>

            <GameOverlay pending=pending screen=screen />
        </Router>
    }
}

/// Fixed top banner that shows a flash message and auto-dismisses after 5 seconds.
#[component]
fn FlashBanner() -> impl IntoView {
    let flash = use_context::<FlashMessage>().expect("FlashMessage context not found").0;

    Effect::new(move |_| {
        if flash.get().is_some() {
            spawn_local(async move {
                gloo_timers::future::TimeoutFuture::new(5_000).await;
                flash.set(None);
            });
        }
    });

    move || flash.get().map(|msg| view! {
        <div class="flash-banner">
            <span>{ msg }</span>
            <button class="flash-dismiss" on:click=move |_| flash.set(None)>"✕"</button>
        </div>
    })
}

/// Renders the full-screen game overlay, but only when the current route is "/".
/// This lets the user navigate to profile/account pages while a game is running.
#[component]
fn GameOverlay(
    pending: RwSignal<VecDeque<GameUiState>>,
    screen: RwSignal<Screen>,
) -> impl IntoView {
    let location = use_location();

    // Memoize the front of the pending queue so that pushing a new item to the back
    // does not re-mount GameScreen (and replay dice animation/sound) when the displayed
    // state (the front) hasn't changed.
    let pending_front = Memo::new(move |_| pending.with(|q| q.front().cloned()));

    move || {
        if location.pathname.get() != "/" {
            return view! {}.into_any();
        }
        if let Some(state) = pending_front.get() {
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

/// Persistent hamburger button + left sidebar — visible on every page.
#[component]
fn SiteHamburger() -> impl IntoView {
    let i18n = use_i18n();
    let auth_username =
        use_context::<RwSignal<Option<String>>>().unwrap_or_else(|| RwSignal::new(None));
    let screen = use_context::<RwSignal<Screen>>().expect("Screen context not found");
    let cmd_tx = use_context::<futures::channel::mpsc::UnboundedSender<NetCommand>>()
        .expect("cmd_tx not found in context");

    let sidebar_open = RwSignal::new(false);
    let snapshot_copied = RwSignal::new(false);
    let replay_open = RwSignal::new(false);
    let replay_text = RwSignal::new(String::new());
    let replay_error = RwSignal::new(false);

    let cmd_tx_newgame = cmd_tx.clone();
    let cmd_tx_snapshot = cmd_tx.clone();
    let cmd_tx_replay = cmd_tx.clone();

    view! {
        // ── Hamburger button (☰ → ✕ animation) ───────────────────────────────
        <button
            class="game-hamburger"
            class:game-hamburger-open=move || sidebar_open.get()
            on:click=move |_| sidebar_open.update(|v| *v = !*v)
            aria-label="Menu"
        >
            <span class="hb-bar hb-top"></span>
            <span class="hb-bar hb-mid"></span>
            <span class="hb-bar hb-bot"></span>
        </button>

        // ── Left sidebar ──────────────────────────────────────────────────────
        <div class="game-sidebar" class:game-sidebar-open=move || sidebar_open.get()>

            <div class="game-sidebar-header">
                <span class="game-sidebar-brand">"Trictrac"</span>

                <div class="lang-switcher">
                    <button
                        class:lang-active=move || i18n.get_locale() == Locale::en
                        on:click=move |_| i18n.set_locale(Locale::en)
                    >"EN"</button>
                    <button
                        class:lang-active=move || i18n.get_locale() == Locale::fr
                        on:click=move |_| i18n.set_locale(Locale::fr)
                    >"FR"</button>
                </div>
            </div>

            // Language switcher
            // <div class="game-sidebar-section">
            //     <svg class="icon" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 640 640">
            //         <path fill="currentColor" d="M192 64C209.7 64 224 78.3 224 96L224 128L352 128C369.7 128 384 142.3 384 160C384 177.7 369.7 192 352 192L342.4 192L334 215.1C317.6 260.3 292.9 301.6 261.8 337.1C276 345.9 290.8 353.7 306.2 360.6L356.6 383L418.8 243C423.9 231.4 435.4 224 448 224C460.6 224 472.1 231.4 477.2 243L605.2 531C612.4 547.2 605.1 566.1 589 573.2C572.9 580.3 553.9 573.1 546.8 557L526.8 512L369.3 512L349.3 557C342.1 573.2 323.2 580.4 307.1 573.2C291 566 283.7 547.1 290.9 531L330.7 441.5L280.3 419.1C257.3 408.9 235.3 396.7 214.5 382.7C193.2 399.9 169.9 414.9 145 427.4L110.3 444.6C94.5 452.5 75.3 446.1 67.4 430.3C59.5 414.5 65.9 395.3 81.7 387.4L116.2 370.1C132.5 361.9 148 352.4 162.6 341.8C148.8 329.1 135.8 315.4 123.7 300.9L113.6 288.7C102.3 275.1 104.1 254.9 117.7 243.6C131.3 232.3 151.5 234.1 162.8 247.7L173 259.9C184.5 273.8 197.1 286.7 210.4 298.6C237.9 268.2 259.6 232.5 273.9 193.2L274.4 192L64.1 192C46.3 192 32 177.7 32 160C32 142.3 46.3 128 64 128L160 128L160 96C160 78.3 174.3 64 192 64zM448 334.8L397.7 448L498.3 448L448 334.8z"/>
            //     </svg>
            //     <span> {t!(i18n, language)}</span>
            //     <div class="lang-switcher">
            //         <button
            //             class:lang-active=move || i18n.get_locale() == Locale::en
            //             on:click=move |_| i18n.set_locale(Locale::en)
            //         >"EN"</button>
            //         <button
            //             class:lang-active=move || i18n.get_locale() == Locale::fr
            //             on:click=move |_| i18n.set_locale(Locale::fr)
            //         >"FR"</button>
            //     </div>
            // </div>

            <div class="game-sidebar-section">
                <svg class="icon" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 640 640">
                    <path fill="currentColor" d="M304 70.1C313.1 61.9 326.9 61.9 336 70.1L568 278.1C577.9 286.9 578.7 302.1 569.8 312C560.9 321.9 545.8 322.7 535.9 313.8L527.9 306.6L527.9 511.9C527.9 547.2 499.2 575.9 463.9 575.9L175.9 575.9C140.6 575.9 111.9 547.2 111.9 511.9L111.9 306.6L103.9 313.8C94 322.6 78.9 321.8 70 312C61.1 302.2 62 287 71.8 278.1L304 70.1zM320 120.2L160 263.7L160 512C160 520.8 167.2 528 176 528L224 528L224 424C224 384.2 256.2 352 296 352L344 352C383.8 352 416 384.2 416 424L416 528L464 528C472.8 528 480 520.8 480 512L480 263.7L320 120.3zM272 528L368 528L368 424C368 410.7 357.3 400 344 400L296 400C282.7 400 272 410.7 272 424L272 528z"/>
               </svg>
            {move || {
                let tx = cmd_tx_newgame.clone();
                Some(view! {
                    <A href="/" attr:class="game-sidebar-link"
                        on:click=move |_| { tx.unbounded_send(NetCommand::Disconnect).ok(); sidebar_open.set(false); }>
                        {t!(i18n, new_game)}
                    </A>
                })
            }}
            </div>

            // Auth
                {move || match auth_username.get() {
                    Some(u) => {
                        let href = format!("/profile/{u}");
                        view! {
                        <div class="game-sidebar-section">
                            <svg class="icon" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 640 640">
                                <path fill="currentColor" d="M240 192C240 147.8 275.8 112 320 112C364.2 112 400 147.8 400 192C400 236.2 364.2 272 320 272C275.8 272 240 236.2 240 192zM448 192C448 121.3 390.7 64 320 64C249.3 64 192 121.3 192 192C192 262.7 249.3 320 320 320C390.7 320 448 262.7 448 192zM144 544C144 473.3 201.3 416 272 416L368 416C438.7 416 496 473.3 496 544L496 552C496 565.3 506.7 576 520 576C533.3 576 544 565.3 544 552L544 544C544 446.8 465.2 368 368 368L272 368C174.8 368 96 446.8 96 544L96 552C96 565.3 106.7 576 120 576C133.3 576 144 565.3 144 552L144 544z"/>
                            </svg>

                            <A href=href attr:class="game-sidebar-link"
                               on:click=move |_| sidebar_open.set(false)>
                               {u}
                            </A>
                        </div>
                        <div class="game-sidebar-section">
                            <svg class="icon" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 640 640">
                                <path fill="currentColor" d="M224 160C241.7 160 256 145.7 256 128C256 110.3 241.7 96 224 96L160 96C107 96 64 139 64 192L64 448C64 501 107 544 160 544L224 544C241.7 544 256 529.7 256 512C256 494.3 241.7 480 224 480L160 480C142.3 480 128 465.7 128 448L128 192C128 174.3 142.3 160 160 160L224 160zM566.6 342.6C579.1 330.1 579.1 309.8 566.6 297.3L438.6 169.3C426.1 156.8 405.8 156.8 393.3 169.3C380.8 181.8 380.8 202.1 393.3 214.6L466.7 288L256 288C238.3 288 224 302.3 224 320C224 337.7 238.3 352 256 352L466.7 352L393.3 425.4C380.8 437.9 380.8 458.2 393.3 470.7C405.8 483.2 426.1 483.2 438.6 470.7L566.6 342.7z"/>
                            </svg>
                            <a class="game-sidebar-link" on:click=move |_| {
                                spawn_local(async move {
                                    let _ = api::post_logout().await;
                                    auth_username.set(None);
                                });
                            }>{t!(i18n, sign_out)}</a>
                        </div>
                        }.into_any()
                    },
                    None => view! {
                        <div class="game-sidebar-section">
                            <svg class="icon" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 640 640">
                                <path fill="currentColor" d="M416 160L480 160C497.7 160 512 174.3 512 192L512 448C512 465.7 497.7 480 480 480L416 480C398.3 480 384 494.3 384 512C384 529.7 398.3 544 416 544L480 544C533 544 576 501 576 448L576 192C576 139 533 96 480 96L416 96C398.3 96 384 110.3 384 128C384 145.7 398.3 160 416 160zM406.6 342.6C419.1 330.1 419.1 309.8 406.6 297.3L278.6 169.3C266.1 156.8 245.8 156.8 233.3 169.3C220.8 181.8 220.8 202.1 233.3 214.6L306.7 288L96 288C78.3 288 64 302.3 64 320C64 337.7 78.3 352 96 352L306.7 352L233.3 425.4C220.8 437.9 220.8 458.2 233.3 470.7C245.8 483.2 266.1 483.2 278.6 470.7L406.6 342.7z"/>
                            </svg>
                        <A href="/account" attr:class="game-sidebar-link"
                           on:click=move |_| sidebar_open.set(false)>
                            {t!(i18n, sign_in)}
                        </A>
                        </div>
                    }.into_any(),
                }}

            <div class="sidebar-footer">
            // ── Debug section ─────────────────────────────────────────────────
                // "Take snapshot" — only visible while a game is in progress
                {move || {
                    let Screen::Playing(ref state) = screen.get() else { return None; };
                    let vs = state.view_state.clone();
                    let tx = cmd_tx_snapshot.clone();
                    Some(view! {

                    <div class="game-sidebar-section">
                        <svg class="icon" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 640 640">
                            <path fill="currentColor" d="M257.1 96C238.4 96 220.9 105.4 210.5 120.9L184.5 160L128 160C92.7 160 64 188.7 64 224L64 480C64 515.3 92.7 544 128 544L512 544C547.3 544 576 515.3 576 480L576 224C576 188.7 547.3 160 512 160L455.5 160L429.5 120.9C419.1 105.4 401.6 96 382.9 96L257.1 96zM250.4 147.6C251.9 145.4 254.4 144 257.1 144L382.8 144C385.5 144 388 145.3 389.5 147.6L422.7 197.4C427.2 204.1 434.6 208.1 442.7 208.1L512 208.1C520.8 208.1 528 215.3 528 224.1L528 480.1C528 488.9 520.8 496.1 512 496.1L128 496C119.2 496 112 488.8 112 480L112 224C112 215.2 119.2 208 128 208L197.3 208C205.3 208 212.8 204 217.3 197.3L250.5 147.5zM320 448C381.9 448 432 397.9 432 336C432 274.1 381.9 224 320 224C258.1 224 208 274.1 208 336C208 397.9 258.1 448 320 448zM256 336C256 300.7 284.7 272 320 272C355.3 272 384 300.7 384 336C384 371.3 355.3 400 320 400C284.7 400 256 371.3 256 336z"/>
                        </svg>
                        <a class="game-sidebar-link" on:click=move |_| {
                            if let Ok(json) = serde_json::to_string(&vs) {
                                #[cfg(target_arch = "wasm32")]
                                {
                                    let json_c = json.clone();
                                    spawn_local(async move {
                                        if let Some(cb) = web_sys::window()
                                            .map(|w| w.navigator().clipboard())
                                        {
                                            let _ = wasm_bindgen_futures::JsFuture::from(
                                                cb.write_text(&json_c),
                                            ).await;
                                            snapshot_copied.set(true);
                                            gloo_timers::future::TimeoutFuture::new(2000).await;
                                            snapshot_copied.set(false);
                                        }
                                    });
                                }
                                let _ = tx; // suppress unused warning on non-wasm
                            }
                        }>
                            {move || if snapshot_copied.get() {
                                t_string!(i18n, snapshot_copied).to_owned()
                            } else {
                                t_string!(i18n, take_snapshot).to_owned()
                            }}
                        </a>
                    </div>
                    })
                }}

                // "Replay snapshot" — always visible
            <div class="game-sidebar-section">
                    <svg class="icon" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 640 640">
                        <path fill="currentColor" d="M534.6 182.6C547.1 170.1 547.1 149.8 534.6 137.3L470.6 73.3C461.4 64.1 447.7 61.4 435.7 66.4C423.7 71.4 416 83.1 416 96L416 128L256 128C150 128 64 214 64 320C64 337.7 78.3 352 96 352C113.7 352 128 337.7 128 320C128 249.3 185.3 192 256 192L416 192L416 224C416 236.9 423.8 248.6 435.8 253.6C447.8 258.6 461.5 255.8 470.7 246.7L534.7 182.7zM105.4 457.4C92.9 469.9 92.9 490.2 105.4 502.7L169.4 566.7C178.6 575.9 192.3 578.6 204.3 573.6C216.3 568.6 224 556.9 224 544L224 512L384 512C490 512 576 426 576 320C576 302.3 561.7 288 544 288C526.3 288 512 302.3 512 320C512 390.7 454.7 448 384 448L224 448L224 416C224 403.1 216.2 391.4 204.2 386.4C192.2 381.4 178.5 384.2 169.3 393.3L105.3 457.3z"/>
                    </svg>
                <a class="game-sidebar-link" on:click=move |_| {
                    replay_text.set(String::new());
                    replay_error.set(false);
                    replay_open.set(true);
                    sidebar_open.set(false);
                }>{t!(i18n, replay_snapshot)}</a>
            </div>
            <div>
                <span class="site-nav-version">"v" {VERSION}</span>
            </div>
            </div>
        </div>

        // ── Replay snapshot modal ─────────────────────────────────────────────
        <div class="ceremony-overlay" style="z-index:300"
            style:display=move || if replay_open.get() { "" } else { "none" }
            on:click=move |_| replay_open.set(false)>
            <div class="ceremony-box" style="min-width:340px;max-width:480px;width:90vw"
                 on:click=|e| e.stop_propagation()>
                <h2 style="font-size:1.3rem">{t!(i18n, replay_snapshot)}</h2>
                <p class="game-sub-prompt" style="margin:0;text-align:center">
                    {t!(i18n, replay_paste_hint)}
                </p>
                <textarea
                    style="width:100%;min-height:120px;background:rgba(0,0,0,0.25);border:1px solid rgba(200,164,72,0.35);border-radius:4px;color:var(--ui-parchment);font-family:var(--font-ui);font-size:0.75rem;padding:0.5rem;resize:vertical;box-sizing:border-box"
                    placeholder="{ \"board\": [...], ... }"
                    prop:value=move || replay_text.get()
                    on:input=move |e| {
                        use leptos::prelude::event_target_value;
                        replay_text.set(event_target_value(&e));
                        replay_error.set(false);
                    }
                />
                {move || replay_error.get().then(|| view! {
                    <p style="color:var(--ui-red-accent);font-size:0.8rem;margin:0">
                        {t!(i18n, replay_invalid_state)}
                    </p>
                })}
                <div style="display:flex;gap:0.75rem;justify-content:center">
                    <button class="btn btn-secondary" on:click=move |_| replay_open.set(false)>
                        {t!(i18n, cancel)}
                    </button>
                    <button class="btn btn-primary" on:click=move |_| {
                        let text = replay_text.get_untracked();
                        match serde_json::from_str::<ViewState>(&text) {
                            Ok(vs) => {
                                cmd_tx_replay
                                    .unbounded_send(NetCommand::ReplaySnapshot(vs))
                                    .ok();
                                replay_open.set(false);
                            }
                            Err(_) => replay_error.set(true),
                        }
                    }>{t!(i18n, replay_start)}</button>
                </div>
            </div>
        </div>
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
