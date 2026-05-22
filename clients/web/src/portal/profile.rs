use leptos::prelude::*;
use leptos_router::{components::A, hooks::use_params_map};

use crate::api::{self, GameSummary, UserProfile};
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
    // let joined = api::format_ts(profile.created_at, locale_tag, &api::DateFormatOptions::date_only());

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
