use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::A;

use crate::api;
use crate::i18n::*;

#[component]
pub fn SiteNav() -> impl IntoView {
    let i18n = use_i18n();
    let auth_username =
        use_context::<RwSignal<Option<String>>>().expect("auth_username context not found");

    let logout = move |_| {
        spawn_local(async move {
            let _ = api::post_logout().await;
            auth_username.set(None);
        });
    };

    view! {
        <nav class="site-nav">
            <A href="/" attr:class="site-nav-brand">"Trictrac"</A>
            <div class="site-nav-spacer" />
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
            {move || match auth_username.get() {
                Some(u) => view! {
                    <A href=format!("/profile/{u}")>{ u.clone() }</A>
                    <button class="site-nav-btn" on:click=logout>{t!(i18n, sign_out)}</button>
                }.into_any(),
                None => view! {
                    <A href="/account">{t!(i18n, sign_in)}</A>
                }.into_any(),
            }}
        </nav>
    }
}
