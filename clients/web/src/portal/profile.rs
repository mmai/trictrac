use leptos::prelude::*;
use leptos_router::{components::A, hooks::use_navigate, hooks::use_params_map};

use crate::api::{self, GameSummary, UserProfile};
use crate::app::{AuthEmailVerified, FlashMessage};
use crate::i18n::*;

#[component]
pub fn ProfilePage() -> impl IntoView {
    let params = use_params_map();
    let username = move || params.read().get("username").unwrap_or_default();

    let profile = LocalResource::new(move || {
        let u = username();
        async move { api::get_user_profile(&u).await }
    });

    let i18n = use_i18n();

    view! {
        <div class="portal-main">
            {move || match profile.get().map(|sw| sw.take()) {
                None => view! { <p class="portal-loading">{t!(i18n, loading)}</p> }.into_any(),
                Some(Err(e)) => view! { <p class="portal-error">{ e }</p> }.into_any(),
                Some(Ok(p)) => view! { <ProfileContent profile=p username=username() /> }.into_any(),
            }}
        </div>
    }
}

#[component]
fn ProfileContent(profile: UserProfile, username: String) -> impl IntoView {
    let i18n = use_i18n();
    let auth_username =
        use_context::<RwSignal<Option<String>>>().expect("auth_username context not found");
    let page = RwSignal::new(0i64);
    let games = LocalResource::new(move || {
        let u = username.clone();
        let p = page.get();
        async move { api::get_user_games(&u, p).await }
    });

    let locale_tag = match i18n.get_locale() {
        Locale::en => "en-GB",
        Locale::fr => "fr-FR",
    };
    let date_format = api::DateFormatOptions {
        date_style: Some("long"),
        time_style: None,
    };
    let joined = api::format_ts(profile.created_at, locale_tag, &date_format);

    let profile_username = profile.username.clone();
    let is_own_profile = move || auth_username.get().as_deref() == Some(&profile_username);

    view! {
        <div class="portal-card">
            <h1>{ profile.username.clone() }</h1>
            <p class="portal-meta">{t!(i18n, member_since)} " " { joined }</p>

            <div class="stats-grid">
                <div class="stat-box">
                    <div class="value">{ profile.total_games }</div>
                    <div class="label">{t!(i18n, stat_games)}</div>
                </div>
                <div class="stat-box">
                    <div class="value outcome-win">{ profile.wins }</div>
                    <div class="label">{t!(i18n, stat_wins)}</div>
                </div>
                <div class="stat-box">
                    <div class="value outcome-loss">{ profile.losses }</div>
                    <div class="label">{t!(i18n, stat_losses)}</div>
                </div>
            </div>
        </div>

        <div class="portal-card">
            <h2>{t!(i18n, game_history_title)}</h2>
            {move || match games.get().map(|sw| sw.take()) {
                None => view! { <p class="portal-loading">{t!(i18n, loading)}</p> }.into_any(),
                Some(Err(e)) => view! { <p class="portal-error">{ e }</p> }.into_any(),
                Some(Ok(r)) => {
                    if r.games.is_empty() {
                        view! { <p class="portal-empty">{t!(i18n, no_games)}</p> }.into_any()
                    } else {
                        view! { <GamesTable games=r.games page=page /> }.into_any()
                    }
                }
            }}
        </div>

        {move || if is_own_profile() {
            let uname = profile.username.clone();
            view! { <DeleteAccountSection username=uname /> }.into_any()
        } else {
            view! { <span /> }.into_any()
        }}
    }
}

#[component]
fn DeleteAccountSection(username: String) -> impl IntoView {
    let i18n = use_i18n();
    let auth_username =
        use_context::<RwSignal<Option<String>>>().expect("auth_username context not found");
    let auth_email_verified = use_context::<AuthEmailVerified>()
        .expect("auth_email_verified context not found").0;
    let flash = use_context::<FlashMessage>().expect("FlashMessage context not found").0;
    let navigate = use_navigate();

    let confirming = RwSignal::new(false);
    let confirm_input = RwSignal::new(String::new());
    let error = RwSignal::new(String::new());
    let pending = RwSignal::new(false);

    view! {
        <div class="portal-card portal-danger-zone">
            <h2>{t!(i18n, delete_account_title)}</h2>
            {move || if !confirming.get() {
                view! {
                    <div>
                        <p class="portal-meta" style="margin-bottom:1rem">
                            {t!(i18n, delete_account_warning)}
                        </p>
                        <button class="portal-danger-btn"
                            on:click=move |_| confirming.set(true)
                        >{t!(i18n, delete_account_btn)}</button>
                    </div>
                }.into_any()
            } else {
                // Define submit fresh each reactive call so the closure is FnMut-compatible.
                let expected = username.clone();
                let nav = navigate.clone();
                let submit = move |ev: leptos::ev::SubmitEvent| {
                    ev.prevent_default();
                    if pending.get() { return; }
                    error.set(String::new());

                    if confirm_input.get() != expected {
                        error.set(t_string!(i18n, delete_account_mismatch).to_string());
                        return;
                    }

                    pending.set(true);
                    let nav = nav.clone();
                    wasm_bindgen_futures::spawn_local(async move {
                        match api::delete_account().await {
                            Ok(()) => {
                                auth_username.set(None);
                                auth_email_verified.set(false);
                                flash.set(Some(t_string!(i18n, account_deleted).to_string()));
                                nav("/", Default::default());
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
                        <p class="portal-meta" style="margin-bottom:1rem">
                            {t!(i18n, delete_account_warning)}
                        </p>
                        <label class="portal-label">{t!(i18n, delete_account_confirm_label)}</label>
                        <input class="portal-input" type="text" required
                            prop:value=move || confirm_input.get()
                            on:input=move |ev| confirm_input.set(event_target_value(&ev)) />
                        {move || if !error.get().is_empty() {
                            view! { <p class="portal-error">{ error.get() }</p> }.into_any()
                        } else {
                            view! { <span /> }.into_any()
                        }}
                        <div style="display:flex;gap:0.75rem;margin-top:1rem">
                            <button class="portal-danger-btn" type="submit"
                                disabled=move || pending.get()
                            >{t!(i18n, delete_account_confirm_btn)}</button>
                            <button class="portal-page-btn" type="button"
                                on:click=move |_| {
                                    confirming.set(false);
                                    confirm_input.set(String::new());
                                    error.set(String::new());
                                }
                            >{t!(i18n, cancel)}</button>
                        </div>
                    </form>
                }.into_any()
            }}
        </div>
    }
}

#[component]
fn GamesTable(games: Vec<GameSummary>, page: RwSignal<i64>) -> impl IntoView {
    let i18n = use_i18n();
    let locale_tag = match i18n.get_locale() {
        Locale::en => "en-GB",
        Locale::fr => "fr-FR",
    };
    let rows = games.clone();
    let has_next = games.len() == 20;

    view! {
        <table>
            <thead>
                <tr>
                    <th>{t!(i18n, col_room)}</th>
                    <th>{t!(i18n, col_started)}</th>
                    <th>{t!(i18n, col_ended)}</th>
                    <th>{t!(i18n, col_outcome)}</th>
                    <th>{t!(i18n, col_detail)}</th>
                </tr>
            </thead>
            <tbody>
                {rows.into_iter().map(|g| {
                    let started = api::format_ts(g.started_at, locale_tag, &api::DateFormatOptions::date_only());
                    let ended = g.ended_at.map(|ts| api::format_ts(ts, locale_tag, &api::DateFormatOptions::date_only())).unwrap_or_else(|| "—".into());
                    let outcome_class = match g.outcome.as_deref() {
                        Some("win")  => "outcome-win",
                        Some("loss") => "outcome-loss",
                        Some("draw") => "outcome-draw",
                        _            => "",
                    };
                    let outcome_text = move || match g.outcome.as_deref() {
                        Some("win")  => t_string!(i18n, outcome_win),
                        Some("loss") => t_string!(i18n, outcome_loss),
                        Some("draw") => t_string!(i18n, outcome_draw),
                        _            => "—",
                    };
                    view! {
                        <tr>
                            <td>{ g.room_code.clone() }</td>
                            <td>{ started }</td>
                            <td>{ ended }</td>
                            <td class=outcome_class>{ outcome_text }</td>
                            <td>
                                <A href=format!("/games/{}", g.id)>{t!(i18n, view_link)}</A>
                            </td>
                        </tr>
                    }
                }).collect_view()}
            </tbody>
        </table>
        <div style="display:flex;gap:0.75rem;margin-top:1.25rem;align-items:center">
            {move || if page.get() > 0 {
                view! {
                    <button class="portal-page-btn"
                        on:click=move |_| page.update(|p| *p -= 1)
                    >{t!(i18n, prev_page)}</button>
                }.into_any()
            } else {
                view! { <span /> }.into_any()
            }}
            <span class="portal-meta" style="margin:0">{t!(i18n, page_label)} " " { move || page.get() + 1 }</span>
            {if has_next {
                view! {
                    <button class="portal-page-btn"
                        on:click=move |_| page.update(|p| *p += 1)
                    >{t!(i18n, next_page)}</button>
                }.into_any()
            } else {
                view! { <span /> }.into_any()
            }}
        </div>
    }
}
