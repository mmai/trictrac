leptos_i18n::load_locales!();

mod app;
mod components;
mod sound;
mod trictrac;

use app::App;
use leptos::prelude::*;

fn main() {
    mount_to_body(|| view! { <App /> })
}
