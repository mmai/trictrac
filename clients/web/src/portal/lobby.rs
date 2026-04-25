use futures::channel::mpsc::UnboundedSender;
use leptos::prelude::*;
use leptos_router::hooks::use_query_map;

use crate::app::{NetCommand, Screen};
use crate::i18n::*;

// ── Room code generation ──────────────────────────────────────────────────────

fn generate_room_code() -> String {
    use rand::Rng;
    let mut rng = rand::rng();
    const CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
    (0..6)
        .map(|_| CHARS[rng.random_range(0..CHARS.len())] as char)
        .collect()
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

// ── Lobby component ───────────────────────────────────────────────────────────

#[derive(Clone)]
enum LobbyView {
    Idle,
    Waiting { code: String },
}

#[component]
pub fn LobbyPage() -> impl IntoView {
    let screen = use_context::<RwSignal<Screen>>().expect("Screen context not found");
    let cmd_tx = use_context::<UnboundedSender<NetCommand>>()
        .expect("UnboundedSender<NetCommand> not found in context");
    let query = use_query_map();

    let view_state: RwSignal<LobbyView> = RwSignal::new(LobbyView::Idle);

    // Auto-join when the URL contains ?room=CODE
    let cmd_tx_query = cmd_tx.clone();
    Effect::new(move |_| {
        if let Some(code) = query.read().get("room") {
            if !code.is_empty() {
                cmd_tx_query
                    .unbounded_send(NetCommand::JoinRoom { room: code })
                    .ok();
            }
        }
    });

    let error = move || match screen.get() {
        Screen::Login { error } => error,
        _ => None,
    };

    let cmd_tx_idle = cmd_tx;

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
                        LobbyView::Idle => {
                            // Create fresh closures each render so they are FnMut-compatible
                            let cmd_tx_create = cmd_tx_idle.clone();
                            let cmd_tx_bot = cmd_tx_idle.clone();
                            let on_create = move |_: leptos::ev::MouseEvent| {
                                let code = generate_room_code();
                                cmd_tx_create
                                    .unbounded_send(NetCommand::CreateRoom { room: code.clone() })
                                    .ok();
                                view_state.set(LobbyView::Waiting { code });
                            };
                            view! {
                                <IdleCard on_create=on_create cmd_tx_bot=cmd_tx_bot />
                            }.into_any()
                        }
                        LobbyView::Waiting { code } => view! {
                            <WaitingCard code=code />
                        }.into_any(),
                    }}
                </div>
            </div>
        </div>
    }
}

// ── Idle card: Create + vs Bot + hidden join-by-code ─────────────────────────

#[component]
fn IdleCard(
    on_create: impl Fn(leptos::ev::MouseEvent) + 'static,
    cmd_tx_bot: UnboundedSender<NetCommand>,
) -> impl IntoView {
    let i18n = use_i18n();
    let join_open = RwSignal::new(false);
    let join_code = RwSignal::new(String::new());
    let cmd_tx_join = cmd_tx_bot.clone();

    view! {
        <div class="login-actions">
            <button class="login-btn login-btn-primary" on:click=on_create>
                {t!(i18n, create_room)}
            </button>
            <button
                class="login-btn login-btn-bot"
                on:click=move |_| { cmd_tx_bot.unbounded_send(NetCommand::PlayVsBot).ok(); }
            >
                {t!(i18n, play_vs_bot)}
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
                // Clone the sender on each reactive run to keep the outer closure FnMut
                let cmd = cmd_tx_join.clone();
                join_open.get().then(|| view! {
                    <div style="margin-top:0.75rem;display:flex;gap:0.5rem">
                        <input
                            class="login-input"
                            style="flex:1;margin:0"
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
                                cmd.unbounded_send(NetCommand::JoinRoom { room: join_code.get() })
                                    .ok();
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

// ── Waiting card: URL + copy + QR ────────────────────────────────────────────

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
                    if let Some(clipboard) = web_sys::window()
                        .map(|w| w.navigator().clipboard())
                    {
                        let _ = wasm_bindgen_futures::JsFuture::from(
                            clipboard.write_text(&url)
                        ).await;
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
