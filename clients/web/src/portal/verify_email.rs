use leptos::prelude::*;
use leptos_router::hooks::use_query_map;

use crate::api;
use crate::i18n::*;

#[derive(Clone, PartialEq)]
enum VerifyStatus {
    Checking,
    Success,
    Error,
}

#[component]
pub fn VerifyEmailPage() -> impl IntoView {
    let i18n = use_i18n();
    let auth_username =
        use_context::<RwSignal<Option<String>>>().expect("auth_username context not found");
    let auth_email_verified =
        use_context::<RwSignal<bool>>().expect("auth_email_verified context not found");

    let query = use_query_map();
    let token = query.with(|m| m.get("token").map(|s| s.to_string()).unwrap_or_default());

    let status = RwSignal::new(VerifyStatus::Checking);

    let tok = token.clone();
    wasm_bindgen_futures::spawn_local(async move {
        let s = if tok.is_empty() {
            VerifyStatus::Error
        } else {
            match api::get_verify_email(&tok).await {
                Ok(()) => {
                    // Update the current session if the user is already logged in.
                    auth_email_verified.set(true);
                    VerifyStatus::Success
                }
                Err(_) => VerifyStatus::Error,
            }
        };
        status.set(s);
    });

    let profile_href = move || {
        auth_username
            .get()
            .map(|u| format!("/profile/{u}"))
            .unwrap_or_else(|| "/account".to_string())
    };

    view! {
        <div class="portal-main" style="display:flex;justify-content:center;padding-top:3rem">
            <div class="portal-card" style="max-width:420px;width:100%;text-align:center">
                <h1 style="font-family:var(--font-display);font-size:1.6rem;margin-bottom:1.5rem">
                    {t!(i18n, verify_email_title)}
                </h1>
                {move || match status.get() {
                    VerifyStatus::Checking => view! {
                        <p class="portal-empty">{t!(i18n, verify_email_checking)}</p>
                    }.into_any(),
                    VerifyStatus::Success => view! {
                        <div>
                            <p class="portal-success">{t!(i18n, verify_email_success)}</p>
                            <div style="margin-top:1rem">
                                <a href=profile_href class="portal-link">
                                    {t!(i18n, sign_in)}
                                </a>
                            </div>
                        </div>
                    }.into_any(),
                    VerifyStatus::Error => view! {
                        <div>
                            <p class="portal-error">{t!(i18n, verify_email_invalid)}</p>
                            <div style="margin-top:1rem">
                                <a href="/account" class="portal-link">{t!(i18n, sign_in)}</a>
                            </div>
                        </div>
                    }.into_any(),
                }}
            </div>
        </div>
    }
}
