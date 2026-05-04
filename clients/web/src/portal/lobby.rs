use futures::channel::mpsc::UnboundedSender;
use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_query_map;

use crate::app::{NetCommand, Screen};
use crate::i18n::*;

// ── Room/nickname generation ──────────────────────────────────────────────────

fn generate_room_code() -> String {
    use rand::Rng;
    let mut rng = rand::rng();
    const CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
    (0..6)
        .map(|_| CHARS[rng.random_range(0..CHARS.len())] as char)
        .collect()
}

fn generate_nickname() -> String {
    use rand::Rng;
    let mut rng = rand::rng();
    const ADJ: &[&str] = &[
        "swift", "brave", "noble", "fierce", "clever", "bold", "cunning",
        "agile", "sharp", "golden", "iron", "silver", "quick", "daring", "wild",
    ];
    const NOUN: &[&str] = &[
        "fox", "hawk", "wolf", "lion", "bear", "rook", "knight",
        "duke", "earl", "lance", "blade", "crown", "dame", "ace", "star",
    ];
    let adj = ADJ[rng.random_range(0..ADJ.len())];
    let noun = NOUN[rng.random_range(0..NOUN.len())];
    let num: u8 = rng.random_range(10..=99);
    format!("{adj}-{noun}-{num}")
}

// ── QR code SVG rendering ─────────────────────────────────────────────────────

pub(crate) fn qr_svg(text: &str) -> String {
    use qrcodegen::{QrCode, QrCodeEcc};
    let qr = match QrCode::encode_text(text, QrCodeEcc::Medium) {
        Ok(q) => q,
        Err(_) => return String::new(),
    };
    let size = qr.size();
    let border = 2;
    let total = size + 2 * border;
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {t} {t}\" shape-rendering=\"crispEdges\">",
        t = total,
    );
    svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"#f2e8d0\"/>");
    for y in 0..size {
        for x in 0..size {
            if qr.get_module(x, y) {
                svg.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"1\" height=\"1\" fill=\"#2a1508\"/>",
                    x + border,
                    y + border,
                ));
            }
        }
    }
    svg.push_str("</svg>");
    svg
}

// ── Share URL helper ──────────────────────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
pub(crate) fn room_url(code: &str) -> String {
    let origin = web_sys::window()
        .and_then(|w| w.location().origin().ok())
        .unwrap_or_default();
    format!("{}/?room={}", origin, code)
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn room_url(code: &str) -> String {
    format!("http://localhost:9091/?room={}", code)
}

// ── Lobby state ───────────────────────────────────────────────────────────────

/// Action to execute once the anonymous player has chosen their nickname.
#[derive(Clone)]
enum PendingLobbyAction {
    Create { code: String },
    Join { code: String },
}

#[derive(Clone)]
enum LobbyView {
    Idle,
    Waiting { code: String },
}

// ── LobbyPage ─────────────────────────────────────────────────────────────────

#[component]
pub fn LobbyPage() -> impl IntoView {
    let screen = use_context::<RwSignal<Screen>>().expect("Screen context");
    let cmd_tx = use_context::<UnboundedSender<NetCommand>>().expect("NetCommand sender");
    let auth_username = use_context::<RwSignal<Option<String>>>().expect("auth_username context");
    let auth_loaded = use_context::<RwSignal<bool>>().expect("auth_loaded context");
    let anon_nickname = use_context::<RwSignal<Option<String>>>().expect("anon_nickname context");
    let query = use_query_map();

    let view_state: RwSignal<LobbyView> = RwSignal::new(LobbyView::Idle);
    // Non-None while the nickname-chooser modal is open.
    let pending_action: RwSignal<Option<PendingLobbyAction>> = RwSignal::new(None);

    // ── Auto-join when URL has ?room=CODE ──────────────────────────────────
    // Wait for auth to resolve so we join directly when already logged in,
    // or show the nickname modal when anonymous.
    let join_processed = StoredValue::new(false);
    let cmd_tx_q = cmd_tx.clone();
    Effect::new(move |_| {
        if join_processed.get_value() || !auth_loaded.get() {
            return;
        }
        let Some(code) = query.read().get("room").filter(|s| !s.is_empty()) else {
            return;
        };
        join_processed.set_value(true);
        if auth_username.get_untracked().is_some() {
            cmd_tx_q.unbounded_send(NetCommand::JoinRoom { room: code }).ok();
        } else {
            pending_action.set(Some(PendingLobbyAction::Join { code }));
        }
    });

    let error = move || match screen.get() {
        Screen::Login { error } => error,
        _ => None,
    };

    let cmd_idle = cmd_tx.clone();
    let cmd_modal = cmd_tx;

    view! {
        <div class="portal-main" style="display:flex;justify-content:center;align-items:flex-start;padding-top:5vh">
            <div class="login-card">
                <div class="login-card-header">
                    <div class="login-board-stripe"></div>
                </div>
                <div class="login-card-body">
                    <h1 class="login-title">"Trictrac"</h1>
                    <p class="login-subtitle">
                        <em>"Une interprétation numérique"</em>
                    </p>
                    <div class="login-ornament">"✦"</div>

                    {move || error().map(|err| view! { <p class="error-msg">{err}</p> })}

                    {move || match view_state.get() {
                        LobbyView::Idle => view! {
                            <IdleCard
                                cmd_tx=cmd_idle.clone()
                                auth_username=auth_username
                                view_state=view_state
                                pending_action=pending_action
                            />
                        }.into_any(),
                        LobbyView::Waiting { code } => view! {
                            <WaitingCard code=code />
                        }.into_any(),
                    }}
                </div>
            </div>

            // Fixed-position modal overlay; rendered here but escapes layout.
            {move || pending_action.get().map(|action| view! {
                <NicknameModal
                    pending=action
                    cmd_tx=cmd_modal.clone()
                    view_state=view_state
                    pending_action=pending_action
                    anon_nickname=anon_nickname
                />
            })}
        </div>
    }
}

// ── IdleCard: Create + vs Bot + hidden join-by-code ──────────────────────────

#[component]
fn IdleCard(
    cmd_tx: UnboundedSender<NetCommand>,
    auth_username: RwSignal<Option<String>>,
    view_state: RwSignal<LobbyView>,
    pending_action: RwSignal<Option<PendingLobbyAction>>,
) -> impl IntoView {
    let i18n = use_i18n();
    let join_open = RwSignal::new(false);
    let join_code = RwSignal::new(String::new());

    let cmd_bot = cmd_tx.clone();
    let cmd_create = cmd_tx.clone();
    let cmd_join = cmd_tx;

    let on_create = move |_: leptos::ev::MouseEvent| {
        let code = generate_room_code();
        if auth_username.get_untracked().is_some() {
            cmd_create.unbounded_send(NetCommand::CreateRoom { room: code.clone() }).ok();
            view_state.set(LobbyView::Waiting { code });
        } else {
            pending_action.set(Some(PendingLobbyAction::Create { code }));
        }
    };

    view! {
        <div class="login-actions">
            <button
                class="login-btn login-btn-bot"
                on:click=move |_| { cmd_bot.unbounded_send(NetCommand::PlayVsBot).ok(); }
            >
                {t!(i18n, play_vs_bot)}
            </button>
            <button class="login-btn login-btn-primary" on:click=on_create>
                {t!(i18n, create_room)}
            </button>
        </div>

        // Hidden "join by code" fallback
        <div style="margin-top:1.25rem;text-align:center">
            <button
                class="portal-page-btn"
                style="font-size:0.75rem;opacity:0.7"
                on:click=move |_| join_open.update(|v| *v = !*v)
            >
                {move || if join_open.get() { "▲ " } else { "▼ " }}
                {t!(i18n, join_code_label)}
            </button>
            {move || {
                let cmd = cmd_join.clone();
                join_open.get().then(|| view! {
                    <div style="margin-top:0.75rem;display:flex;gap:0.5rem">
                        <input
                            class="login-input"
                            style="margin:0"
                            type="text"
                            placeholder=move || t_string!(i18n, join_code_placeholder)
                            prop:value=move || join_code.get()
                            on:input=move |ev| join_code.set(event_target_value(&ev))
                        />
                        <button
                            class="login-btn login-btn-secondary"
                            style="margin:0;padding:0 1rem"
                            disabled=move || join_code.get().is_empty()
                            on:click=move |_| {
                                let code = join_code.get();
                                if auth_username.get_untracked().is_some() {
                                    cmd.unbounded_send(NetCommand::JoinRoom { room: code }).ok();
                                } else {
                                    pending_action.set(Some(PendingLobbyAction::Join { code }));
                                }
                            }
                        >
                            {t!(i18n, join_room)}
                        </button>
                    </div>
                })
            }}
        </div>
    }
}

// ── NicknameModal ─────────────────────────────────────────────────────────────

#[component]
fn NicknameModal(
    pending: PendingLobbyAction,
    cmd_tx: UnboundedSender<NetCommand>,
    view_state: RwSignal<LobbyView>,
    pending_action: RwSignal<Option<PendingLobbyAction>>,
    anon_nickname: RwSignal<Option<String>>,
) -> impl IntoView {
    let i18n = use_i18n();
    // Pre-fill with a random nickname; the player can edit it.
    let nick = RwSignal::new(generate_nickname());

    let on_play = move |_: leptos::ev::MouseEvent| {
        let chosen = nick.get().trim().to_string();
        let chosen = if chosen.is_empty() { generate_nickname() } else { chosen };
        anon_nickname.set(Some(chosen));
        match &pending {
            PendingLobbyAction::Create { code } => {
                cmd_tx.unbounded_send(NetCommand::CreateRoom { room: code.clone() }).ok();
                view_state.set(LobbyView::Waiting { code: code.clone() });
            }
            PendingLobbyAction::Join { code } => {
                cmd_tx.unbounded_send(NetCommand::JoinRoom { room: code.clone() }).ok();
            }
        }
        pending_action.set(None);
    };

    view! {
        <div class="nickname-backdrop">
            <div class="nickname-modal">
                <h2 class="nickname-modal-title">{t!(i18n, nickname_modal_title)}</h2>
                <p class="nickname-modal-hint">{t!(i18n, nickname_modal_hint)}</p>
                <input
                    class="login-input"
                    type="text"
                    style="margin:0"
                    prop:value=move || nick.get()
                    on:input=move |ev| nick.set(event_target_value(&ev))
                />
                <button
                    class="login-btn login-btn-primary"
                    disabled=move || nick.get().trim().is_empty()
                    on:click=on_play
                >
                    {t!(i18n, nickname_modal_play)}
                </button>
                <p class="nickname-modal-alt">
                    {t!(i18n, nickname_modal_or)}
                    " "
                    <A href="/account">{t!(i18n, nickname_modal_sign_in)}</A>
                    " · "
                    <A href="/account">{t!(i18n, nickname_modal_register)}</A>
                </p>
            </div>
        </div>
    }
}

// ── WaitingCard: URL + copy + QR ─────────────────────────────────────────────

#[component]
fn WaitingCard(code: String) -> impl IntoView {
    let i18n = use_i18n();
    let url = room_url(&code);
    let svg = qr_svg(&url);
    let copied = RwSignal::new(false);

    let on_copy = {
        let url = url.clone();
        move |_| {
            #[cfg(target_arch = "wasm32")]
            {
                let url = url.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    if let Some(clipboard) = web_sys::window().map(|w| w.navigator().clipboard()) {
                        let _ =
                            wasm_bindgen_futures::JsFuture::from(clipboard.write_text(&url)).await;
                        copied.set(true);
                        gloo_timers::future::TimeoutFuture::new(2000).await;
                        copied.set(false);
                    }
                });
            }
        }
    };

    view! {
        <p style="font-size:0.85rem;color:rgba(242,232,208,0.75);margin-bottom:1rem;text-align:center">
            {t!(i18n, waiting_for_opponent)}
        </p>

        <p style="font-size:0.8rem;color:rgba(242,232,208,0.6);margin-bottom:0.5rem;text-align:center">
            {t!(i18n, share_link)}
        </p>

        <div class="share-url-row">
            <span class="share-url-text">{ url.clone() }</span>
            <button class="share-copy-btn" on:click=on_copy>
                {move || if copied.get() {
                    t_string!(i18n, link_copied)
                } else {
                    t_string!(i18n, copy_link)
                }}
            </button>
        </div>

        <p style="font-size:0.75rem;color:rgba(242,232,208,0.45);margin:1rem 0 0.5rem;text-align:center">
            {t!(i18n, scan_qr)}
        </p>

        <div class="qr-container" inner_html=svg />
    }
}
