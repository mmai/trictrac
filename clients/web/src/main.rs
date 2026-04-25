leptos_i18n::load_locales!();

mod api;
mod app;
mod game;
mod nav;
mod portal;

use app::App;
use i18n::I18nContextProvider;
use leptos::prelude::*;

fn main() {
    mount_to_body(|| view! {
        <I18nContextProvider>
            <App />
        </I18nContextProvider>
    })
}
