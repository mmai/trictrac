use futures::channel::mpsc::UnboundedSender;
use leptos::prelude::*;

use crate::app::{NetCommand, Screen};
use crate::i18n::*;

#[component]
pub fn LobbyPage() -> impl IntoView {
    let i18n = use_i18n();
    let (room_name, set_room_name) = signal(String::new());

    let screen = use_context::<RwSignal<Screen>>().expect("Screen context not found");
    let cmd_tx = use_context::<UnboundedSender<NetCommand>>()
        .expect("UnboundedSender<NetCommand> not found in context");

    let cmd_tx_create = cmd_tx.clone();
    let cmd_tx_join = cmd_tx.clone();
    let cmd_tx_bot = cmd_tx;

    // Extract connection error from screen state.
    let error = move || match screen.get() {
        Screen::Login { error } => error,
        _ => None,
    };

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

                    <input
                        class="login-input"
                        type="text"
                        placeholder=move || t_string!(i18n, room_name_placeholder)
                        prop:value=move || room_name.get()
                        on:input=move |ev| set_room_name.set(event_target_value(&ev))
                    />

                    <div class="login-actions">
                        <button
                            class="login-btn login-btn-primary"
                            disabled=move || room_name.get().is_empty()
                            on:click=move |_| {
                                cmd_tx_create
                                    .unbounded_send(NetCommand::CreateRoom { room: room_name.get() })
                                    .ok();
                            }
                        >
                            {t!(i18n, create_room)}
                        </button>

                        <button
                            class="login-btn login-btn-secondary"
                            disabled=move || room_name.get().is_empty()
                            on:click=move |_| {
                                cmd_tx_join
                                    .unbounded_send(NetCommand::JoinRoom { room: room_name.get() })
                                    .ok();
                            }
                        >
                            {t!(i18n, join_room)}
                        </button>

                        <button
                            class="login-btn login-btn-bot"
                            on:click=move |_| {
                                cmd_tx_bot.unbounded_send(NetCommand::PlayVsBot).ok();
                            }
                        >
                            {t!(i18n, play_vs_bot)}
                        </button>
                    </div>
                </div>
            </div>
        </div>
    }
}
