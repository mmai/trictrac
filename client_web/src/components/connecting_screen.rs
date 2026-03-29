use leptos::prelude::*;

use crate::i18n::*;

#[component]
pub fn ConnectingScreen() -> impl IntoView {
    let i18n = use_i18n();
    view! { <p class="connecting">{t!(i18n, connecting)}</p> }
}
