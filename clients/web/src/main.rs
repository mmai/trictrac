leptos_i18n::load_locales!();

mod api;
mod app;
mod game;
mod nav;
mod portal;

use app::App;
use leptos::prelude::*;

fn main() {
    mount_to_body(|| view! { <App /> })
}
