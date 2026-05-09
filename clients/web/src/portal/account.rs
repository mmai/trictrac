use leptos::prelude::*;
use leptos_router::hooks::use_navigate;

use crate::api;
use crate::i18n::*;

#[component]
pub fn AccountPage() -> impl IntoView {
    let i18n = use_i18n();
    let auth_username =
        use_context::<RwSignal<Option<String>>>().expect("auth_username context not found");
    let auth_email_verified =
        use_context::<RwSignal<bool>>().expect("auth_email_verified context not found");
    let navigate = use_navigate();

    // Only redirect to profile when the email is actually verified.
    Effect::new(move |_| {
        if let Some(u) = auth_username.get() {
            if auth_email_verified.get() {
                navigate(&format!("/profile/{u}"), Default::default());
            }
        }
    });

    let tab = RwSignal::new("login");

    view! {
        <div class="portal-main" style="display:flex;justify-content:center;padding-top:3rem">
            <div class="portal-card" style="max-width:420px;width:100%">
                <h1 style="font-family:var(--font-display);font-size:1.6rem;margin-bottom:1.5rem;text-align:center">
                    {t!(i18n, account_title)}
                </h1>
                {move || {
                    let username = auth_username.get();
                    let verified = auth_email_verified.get();
                    if username.is_some() && !verified {
                        view! { <VerificationBanner /> }.into_any()
                    } else if username.is_none() {
                        view! {
                            <div>
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
                        }.into_any()
                    } else {
                        view! { <span /> }.into_any()
                    }
                }}
            </div>
        </div>
    }
}

#[component]
fn VerificationBanner() -> impl IntoView {
    let i18n = use_i18n();
    let pending = RwSignal::new(false);
    let sent = RwSignal::new(false);
    let error = RwSignal::new(String::new());

    let resend = move |_| {
        if pending.get() { return; }
        pending.set(true);
        sent.set(false);
        error.set(String::new());
        wasm_bindgen_futures::spawn_local(async move {
            match api::post_resend_verification().await {
                Ok(()) => { sent.set(true); }
                Err(e) => { error.set(e); }
            }
            pending.set(false);
        });
    };

    view! {
        <div class="portal-verification-banner">
            <p>{t!(i18n, email_not_verified_banner)}</p>
            <button class="portal-submit-btn" on:click=resend disabled=move || pending.get()>
                {t!(i18n, resend_verification)}
            </button>
            {move || if sent.get() {
                view! { <p class="portal-success">{ t_string!(i18n, verification_email_resent).to_string() }</p> }.into_any()
            } else if !error.get().is_empty() {
                view! { <p class="portal-error">{ error.get() }</p> }.into_any()
            } else {
                view! { <span /> }.into_any()
            }}
        </div>
    }
}

#[component]
fn LoginForm() -> impl IntoView {
    let i18n = use_i18n();
    let auth_username =
        use_context::<RwSignal<Option<String>>>().expect("auth_username context not found");
    let auth_email_verified =
        use_context::<RwSignal<bool>>().expect("auth_email_verified context not found");
    let navigate = use_navigate();

    let login = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());
    let error = RwSignal::new(String::new());
    let pending = RwSignal::new(false);

    let submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        if pending.get() { return; }
        pending.set(true);
        error.set(String::new());
        let u = login.get();
        let p = password.get();
        let navigate = navigate.clone();
        wasm_bindgen_futures::spawn_local(async move {
            match api::post_login(&u, &p).await {
                Ok(me) => {
                    auth_username.set(Some(me.username.clone()));
                    auth_email_verified.set(me.email_verified);
                    if me.email_verified {
                        navigate(&format!("/profile/{}", me.username), Default::default());
                    }
                    // If not verified, the AccountPage Effect will show the banner.
                }
                Err(e) => {
                    let msg = if e.is_empty() {
                        t_string!(i18n, login_failed).to_string()
                    } else {
                        e
                    };
                    error.set(msg);
                    pending.set(false);
                }
            }
        });
    };

    view! {
        <form on:submit=submit>
            <label class="portal-label">{t!(i18n, label_username_or_email)}</label>
            <input class="portal-input" type="text" required autocomplete="username"
                prop:value=move || login.get()
                on:input=move |ev| login.set(event_target_value(&ev)) />
            <label class="portal-label">{t!(i18n, label_password)}</label>
            <input class="portal-input" type="password" required autocomplete="current-password"
                prop:value=move || password.get()
                on:input=move |ev| password.set(event_target_value(&ev)) />
            <div style="text-align:right;margin-bottom:0.75rem">
                <a href="/forgot-password" class="portal-link">{t!(i18n, forgot_password_link)}</a>
            </div>
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
    let auth_email_verified =
        use_context::<RwSignal<bool>>().expect("auth_email_verified context not found");

    let username = RwSignal::new(String::new());
    let email = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());
    let confirm_password = RwSignal::new(String::new());
    let error = RwSignal::new(String::new());
    let pending = RwSignal::new(false);

    let submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        if pending.get() { return; }

        if password.get() != confirm_password.get() {
            error.set(t_string!(i18n, passwords_do_not_match).to_string());
            return;
        }

        pending.set(true);
        error.set(String::new());
        let u = username.get();
        let e = email.get();
        let p = password.get();
        wasm_bindgen_futures::spawn_local(async move {
            match api::post_register(&u, &e, &p).await {
                Ok(me) => {
                    auth_username.set(Some(me.username));
                    auth_email_verified.set(me.email_verified);
                    // AccountPage shows verification banner when email_verified = false.
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
            <input class="portal-input" type="text" required autocomplete="username"
                prop:value=move || username.get()
                on:input=move |ev| username.set(event_target_value(&ev)) />
            <label class="portal-label">{t!(i18n, label_email)}</label>
            <input class="portal-input" type="email" required autocomplete="email"
                prop:value=move || email.get()
                on:input=move |ev| email.set(event_target_value(&ev)) />
            <label class="portal-label">{t!(i18n, label_password)}</label>
            <input class="portal-input" type="password" required autocomplete="new-password"
                prop:value=move || password.get()
                on:input=move |ev| password.set(event_target_value(&ev)) />
            <label class="portal-label">{t!(i18n, label_confirm_password)}</label>
            <input class="portal-input" type="password" required autocomplete="new-password"
                prop:value=move || confirm_password.get()
                on:input=move |ev| confirm_password.set(event_target_value(&ev)) />
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
