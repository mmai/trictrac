use leptos::prelude::*;
use leptos_router::hooks::use_navigate;

use crate::api;
use crate::i18n::*;

#[component]
pub fn AccountPage() -> impl IntoView {
    let i18n = use_i18n();
    let auth_username =
        use_context::<RwSignal<Option<String>>>().expect("auth_username context not found");
    let navigate = use_navigate();

    Effect::new(move |_| {
        if let Some(u) = auth_username.get() {
            navigate(&format!("/profile/{u}"), Default::default());
        }
    });

    let tab = RwSignal::new("login");

    view! {
        <div class="portal-main" style="display:flex;justify-content:center;padding-top:3rem">
            <div class="portal-card" style="max-width:420px;width:100%">
                <h1 style="font-family:var(--font-display);font-size:1.6rem;margin-bottom:1.5rem;text-align:center">
                    {t!(i18n, account_title)}
                </h1>
                <div class="portal-tabs">
                    <button
                        class=move || if tab.get() == "login" { "portal-tab-btn active" } else { "portal-tab-btn" }
                        on:click=move |_| tab.set("login")
                    >{t!(i18n, sign_in)}</button>
                    <button
                        class=move || if tab.get() == "register" { "portal-tab-btn active" } else { "portal-tab-btn" }
                        on:click=move |_| tab.set("register")
                    >{t!(i18n, create_account)}</button>
                </div>
                {move || if tab.get() == "login" {
                    view! { <LoginForm /> }.into_any()
                } else {
                    view! { <RegisterForm /> }.into_any()
                }}
            </div>
        </div>
    }
}

#[component]
fn LoginForm() -> impl IntoView {
    let i18n = use_i18n();
    let auth_username =
        use_context::<RwSignal<Option<String>>>().expect("auth_username context not found");
    let navigate = use_navigate();

    let username = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());
    let error = RwSignal::new(String::new());
    let pending = RwSignal::new(false);

    let submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        if pending.get() { return; }
        pending.set(true);
        error.set(String::new());
        let u = username.get();
        let p = password.get();
        let navigate = navigate.clone();
        wasm_bindgen_futures::spawn_local(async move {
            match api::post_login(&u, &p).await {
                Ok(me) => {
                    let dest = format!("/profile/{}", me.username);
                    auth_username.set(Some(me.username));
                    navigate(&dest, Default::default());
                }
                Err(e) => {
                    error.set(e);
                    pending.set(false);
                }
            }
        });
    };

    view! {
        <form on:submit=submit>
            <label class="portal-label">{t!(i18n, label_username)}</label>
            <input class="portal-input" type="text" required
                prop:value=move || username.get()
                on:input=move |ev| username.set(event_target_value(&ev)) />
            <label class="portal-label">{t!(i18n, label_password)}</label>
            <input class="portal-input" type="password" required
                prop:value=move || password.get()
                on:input=move |ev| password.set(event_target_value(&ev)) />
            <button class="portal-submit-btn" type="submit"
                disabled=move || pending.get()
            >{t!(i18n, sign_in)}</button>
            {move || if !error.get().is_empty() {
                view! { <p class="portal-error">{ error.get() }</p> }.into_any()
            } else {
                view! { <span /> }.into_any()
            }}
        </form>
    }
}

#[component]
fn RegisterForm() -> impl IntoView {
    let i18n = use_i18n();
    let auth_username =
        use_context::<RwSignal<Option<String>>>().expect("auth_username context not found");
    let navigate = use_navigate();

    let username = RwSignal::new(String::new());
    let email = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());
    let error = RwSignal::new(String::new());
    let pending = RwSignal::new(false);

    let submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        if pending.get() { return; }
        pending.set(true);
        error.set(String::new());
        let u = username.get();
        let e = email.get();
        let p = password.get();
        let navigate = navigate.clone();
        wasm_bindgen_futures::spawn_local(async move {
            match api::post_register(&u, &e, &p).await {
                Ok(me) => {
                    let dest = format!("/profile/{}", me.username);
                    auth_username.set(Some(me.username));
                    navigate(&dest, Default::default());
                }
                Err(err) => {
                    error.set(err);
                    pending.set(false);
                }
            }
        });
    };

    view! {
        <form on:submit=submit>
            <label class="portal-label">{t!(i18n, label_username)}</label>
            <input class="portal-input" type="text" required
                prop:value=move || username.get()
                on:input=move |ev| username.set(event_target_value(&ev)) />
            <label class="portal-label">{t!(i18n, label_email)}</label>
            <input class="portal-input" type="email" required
                prop:value=move || email.get()
                on:input=move |ev| email.set(event_target_value(&ev)) />
            <label class="portal-label">{t!(i18n, label_password)}</label>
            <input class="portal-input" type="password" required
                prop:value=move || password.get()
                on:input=move |ev| password.set(event_target_value(&ev)) />
            <button class="portal-submit-btn" type="submit"
                disabled=move || pending.get()
            >{t!(i18n, create_account)}</button>
            {move || if !error.get().is_empty() {
                view! { <p class="portal-error">{ error.get() }</p> }.into_any()
            } else {
                view! { <span /> }.into_any()
            }}
        </form>
    }
}
