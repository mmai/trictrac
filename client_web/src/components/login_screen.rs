use futures::channel::mpsc::UnboundedSender;
use leptos::prelude::*;

use crate::app::NetCommand;
use crate::i18n::*;

#[component]
pub fn LoginScreen(error: Option<String>) -> impl IntoView {
    let i18n = use_i18n();
    let (room_name, set_room_name) = signal(String::new());

    let cmd_tx = use_context::<UnboundedSender<NetCommand>>()
        .expect("UnboundedSender<NetCommand> not found in context");

    let cmd_tx_create = cmd_tx.clone();
    let cmd_tx_join = cmd_tx.clone();
    let cmd_tx_bot = cmd_tx;

    view! {
        <div class="login-container">
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

            <h1>"Trictrac"</h1>

            {error.map(|err| view! { <p class="error-msg">{err}</p> })}

            <input
                type="text"
                placeholder=move || t_string!(i18n, room_name_placeholder)
                prop:value=move || room_name.get()
                on:input=move |ev| set_room_name.set(event_target_value(&ev))
            />

            <button
                class="btn btn-primary"
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
                class="btn btn-secondary"
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
                class="btn btn-bot"
                on:click=move |_| {
                    cmd_tx_bot.unbounded_send(NetCommand::PlayVsBot).ok();
                }
            >
                {t!(i18n, play_vs_bot)}
            </button>
        </div>
    }
}
