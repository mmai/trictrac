use leptos::prelude::*;
use leptos_router::hooks::use_query_map;

use crate::api;
use crate::i18n::*;

#[component]
pub fn ResetPasswordPage() -> impl IntoView {
    let i18n = use_i18n();
    let query = use_query_map();
    // Read token once — not reactive, just a plain String.
    let token = query.with(|m| m.get("token").map(|s| s.to_string()).unwrap_or_default());

    let new_password = RwSignal::new(String::new());
    let confirm_password = RwSignal::new(String::new());
    let pending = RwSignal::new(false);
    let success = RwSignal::new(false);
    let error = RwSignal::new(String::new());

    if token.is_empty() {
        error.set(t_string!(i18n, reset_password_invalid).to_string());
    }

    // `submit` moves `token: String` — it is FnMut (clones token each call) but not Copy.
    // Keep it off of reactive closures: put it directly on <form on:submit=submit>.
    let submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        if pending.get() { return; }

        if new_password.get() != confirm_password.get() {
            error.set(t_string!(i18n, passwords_do_not_match).to_string());
            return;
        }

        pending.set(true);
        error.set(String::new());
        let tok = token.clone();
        let pw = new_password.get();
        let invalid_msg = t_string!(i18n, reset_password_invalid).to_string();
        wasm_bindgen_futures::spawn_local(async move {
            match api::post_reset_password(&tok, &pw).await {
                Ok(()) => { success.set(true); }
                Err(_) => { error.set(invalid_msg); }
            }
            pending.set(false);
        });
    };

    view! {
        <div class="portal-main" style="display:flex;justify-content:center;padding-top:3rem">
            <div class="portal-card" style="max-width:420px;width:100%">
                <h1 style="font-family:var(--font-display);font-size:1.6rem;margin-bottom:1.5rem;text-align:center">
                    {t!(i18n, reset_password_title)}
                </h1>

                // Success message — only captures `success` (Copy RwSignal)
                {move || success.get().then(|| view! {
                    <p class="portal-success" style="text-align:center">
                        {t!(i18n, reset_password_success)}
                    </p>
                    <div style="margin-top:1rem;text-align:center">
                        <a href="/account" class="portal-link">{t!(i18n, sign_in)}</a>
                    </div>
                })}

                // Form — `submit` lives directly on the element, not inside a reactive closure
                <form on:submit=submit
                      style:display=move || if success.get() { "none" } else { "" }>
                    <label class="portal-label">{t!(i18n, new_password_label)}</label>
                    <input class="portal-input" type="password" required autocomplete="new-password"
                        prop:value=move || new_password.get()
                        on:input=move |ev| new_password.set(event_target_value(&ev)) />
                    <label class="portal-label">{t!(i18n, label_confirm_password)}</label>
                    <input class="portal-input" type="password" required autocomplete="new-password"
                        prop:value=move || confirm_password.get()
                        on:input=move |ev| confirm_password.set(event_target_value(&ev)) />
                    <button class="portal-submit-btn" type="submit"
                        prop:disabled=move || pending.get()
                    >{t!(i18n, reset_password_submit)}</button>
                    {move || (!error.get().is_empty()).then(|| view! {
                        <p class="portal-error">{ error.get() }</p>
                    })}
                </form>
            </div>
        </div>
    }
}
