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
        "swift", "brave", "noble", "fierce", "clever", "bold", "cunning", "agile", "sharp",
        "golden", "iron", "silver", "quick", "daring", "wild",
    ];
    const NOUN: &[&str] = &[
        "fox", "hawk", "wolf", "lion", "bear", "rook", "knight", "duke", "earl", "lance", "blade",
        "crown", "dame", "ace", "star",
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
            cmd_tx_q
                .unbounded_send(NetCommand::JoinRoom { room: code })
                .ok();
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
            cmd_create
                .unbounded_send(NetCommand::CreateRoom { room: code.clone() })
                .ok();
            view_state.set(LobbyView::Waiting { code });
        } else {
            pending_action.set(Some(PendingLobbyAction::Create { code }));
        }
    };

    view! {
        <div class="login-actions">
            <button
                class="login-btn login-btn-secondary"
                on:click=move |_| { cmd_bot.unbounded_send(NetCommand::PlayVsBot).ok(); }
            >
                <svg class="icon" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 640 640">
                    <path fill="currentColor" d="M352 64C352 46.3 337.7 32 320 32C302.3 32 288 46.3 288 64L288 128L192 128C139 128 96 171 96 224L96 448C96 501 139 544 192 544L448 544C501 544 544 501 544 448L544 224C544 171 501 128 448 128L352 128L352 64zM160 432C160 418.7 170.7 408 184 408L216 408C229.3 408 240 418.7 240 432C240 445.3 229.3 456 216 456L184 456C170.7 456 160 445.3 160 432zM280 432C280 418.7 290.7 408 304 408L336 408C349.3 408 360 418.7 360 432C360 445.3 349.3 456 336 456L304 456C290.7 456 280 445.3 280 432zM400 432C400 418.7 410.7 408 424 408L456 408C469.3 408 480 418.7 480 432C480 445.3 469.3 456 456 456L424 456C410.7 456 400 445.3 400 432zM224 240C250.5 240 272 261.5 272 288C272 314.5 250.5 336 224 336C197.5 336 176 314.5 176 288C176 261.5 197.5 240 224 240zM368 288C368 261.5 389.5 240 416 240C442.5 240 464 261.5 464 288C464 314.5 442.5 336 416 336C389.5 336 368 314.5 368 288zM64 288C64 270.3 49.7 256 32 256C14.3 256 0 270.3 0 288L0 384C0 401.7 14.3 416 32 416C49.7 416 64 401.7 64 384L64 288zM608 256C590.3 256 576 270.3 576 288L576 384C576 401.7 590.3 416 608 416C625.7 416 640 401.7 640 384L640 288C640 270.3 625.7 256 608 256z"/>
                </svg>
                {t!(i18n, play_vs_bot)}
            </button>
            <button class="login-btn login-btn-primary" on:click=on_create>
                <svg class="icon" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 640 640">
                    <path  fill="currentColor" d="M598.1 139.4C608.8 131.6 611.2 116.6 603.4 105.9C595.6 95.2 580.6 92.8 569.9 100.6L495.4 154.8L485.5 148.2C465.8 135 442.6 128 418.9 128L359.7 128L359.3 128L215.7 128C189 128 163.2 136.9 142.3 153.1L70.1 100.6C59.4 92.8 44.4 95.2 36.6 105.9C28.8 116.6 31.2 131.6 41.9 139.4L129.9 203.4C139.5 210.3 152.6 209.3 161 201L164.9 197.1C178.4 183.6 196.7 176 215.8 176L262.1 176L170.4 267.7C154.8 283.3 154.8 308.6 170.4 324.3L171.2 325.1C218 372 294 372 340.9 325.1L368 298L465.8 395.8C481.4 411.4 481.4 436.7 465.8 452.4L456 462.2L425 431.2C415.6 421.8 400.4 421.8 391.1 431.2C381.8 440.6 381.7 455.8 391.1 465.1L419.1 493.1C401.6 503.5 381.9 509.8 361.5 511.6L313 463C303.6 453.6 288.4 453.6 279.1 463C269.8 472.4 269.7 487.6 279.1 496.9L294.1 511.9L290.3 511.9C254.2 511.9 219.6 497.6 194.1 472.1L65 343C55.6 333.6 40.4 333.6 31.1 343C21.8 352.4 21.7 367.6 31.1 376.9L160.2 506.1C194.7 540.6 241.5 560 290.3 560L342.1 560L343.1 561L344.1 560L349.8 560C398.6 560 445.4 540.6 479.9 506.1L499.8 486.2C501 485 502.1 483.9 503.2 482.7C503.9 482.2 504.5 481.6 505.1 481L609 377C618.4 367.6 618.4 352.4 609 343.1C599.6 333.8 584.4 333.7 575.1 343.1L521.3 396.9C517.1 384.1 510 372 499.8 361.8L385 247C375.6 237.6 360.4 237.6 351.1 247L307 291.1C280.5 317.6 238.5 319.1 210.3 295.7L309 197C322.4 183.6 340.6 176 359.6 175.9L368.1 175.9L368.3 175.9L419.1 175.9C433.3 175.9 447.2 180.1 459 188L482.7 204C491.1 209.6 502 209.3 510.1 203.4L598.1 139.4z"/>
                </svg>
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
        let chosen = if chosen.is_empty() {
            generate_nickname()
        } else {
            chosen
        };
        anon_nickname.set(Some(chosen));
        match &pending {
            PendingLobbyAction::Create { code } => {
                cmd_tx
                    .unbounded_send(NetCommand::CreateRoom { room: code.clone() })
                    .ok();
                view_state.set(LobbyView::Waiting { code: code.clone() });
            }
            PendingLobbyAction::Join { code } => {
                cmd_tx
                    .unbounded_send(NetCommand::JoinRoom { room: code.clone() })
                    .ok();
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
