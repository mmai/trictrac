use leptos::prelude::*;

use crate::api;
use crate::i18n::*;

#[component]
pub fn ForgotPasswordPage() -> impl IntoView {
    let i18n = use_i18n();

    let email = RwSignal::new(String::new());
    let pending = RwSignal::new(false);
    let sent = RwSignal::new(false);
    let error = RwSignal::new(String::new());

    let submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        if pending.get() { return; }
        pending.set(true);
        error.set(String::new());
        let e = email.get();
        wasm_bindgen_futures::spawn_local(async move {
            match api::post_forgot_password(&e).await {
                Ok(()) => { sent.set(true); }
                Err(e) => { error.set(e); }
            }
            pending.set(false);
        });
    };

    view! {
        <div class="portal-main" style="display:flex;justify-content:center;padding-top:3rem">
            <div class="portal-card" style="max-width:420px;width:100%">
                <h1 style="font-family:var(--font-display);font-size:1.6rem;margin-bottom:1.5rem;text-align:center">
                    {t!(i18n, forgot_password_title)}
                </h1>
                {move || if sent.get() {
                    view! {
                        <p class="portal-success" style="text-align:center">
                            {t!(i18n, forgot_password_sent)}
                        </p>
                    }.into_any()
                } else {
                    view! {
                        <form on:submit=submit>
                            <label class="portal-label">{t!(i18n, forgot_password_email_label)}</label>
                            <input class="portal-input" type="email" required autocomplete="email"
                                prop:value=move || email.get()
                                on:input=move |ev| email.set(event_target_value(&ev)) />
                            <button class="portal-submit-btn" type="submit"
                                disabled=move || pending.get()
                            >{t!(i18n, forgot_password_submit)}</button>
                            {move || if !error.get().is_empty() {
                                view! { <p class="portal-error">{ error.get() }</p> }.into_any()
                            } else {
                                view! { <span /> }.into_any()
                            }}
                        </form>
                    }.into_any()
                }}
                <div style="margin-top:1rem;text-align:center">
                    <a href="/account" class="portal-link">{t!(i18n, sign_in)}</a>
                </div>
            </div>
        </div>
    }
}
