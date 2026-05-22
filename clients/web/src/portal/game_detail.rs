use leptos::prelude::*;
use leptos_router::{components::A, hooks::use_params_map};

use crate::api::{self, GameDetail, Participant};
use crate::i18n::*;

#[component]
pub fn GameDetailPage() -> impl IntoView {
    let i18n = use_i18n();
    let params = use_params_map();
    let id_str = move || params.read().get("id").unwrap_or_default();

    let detail = LocalResource::new(move || {
        let s = id_str();
        async move {
            let id: i64 = s.parse().map_err(|_| "invalid game id".to_string())?;
            api::get_game_detail(id).await
        }
    });

    view! {
        <div class="portal-main">
            {move || match detail.get().map(|sw| sw.take()) {
                None => view! { <p class="portal-loading">{t!(i18n, loading)}</p> }.into_any(),
                Some(Err(e)) => view! { <p class="portal-error">{ e }</p> }.into_any(),
                Some(Ok(g)) => view! { <GameDetailView game=g /> }.into_any(),
            }}
        </div>
    }
}

#[component]
fn GameDetailView(game: GameDetail) -> impl IntoView {
    let i18n = use_i18n();
    let locale_tag = match i18n.get_locale() {
        Locale::en => "en-GB",
        Locale::fr => "fr-FR",
    };
    let started = api::format_ts(game.started_at, locale_tag, &api::DateFormatOptions::date_only());
    let ended = game.ended_at.map(|ts| api::format_ts(ts, locale_tag, &api::DateFormatOptions::date_only()))
        .unwrap_or_else(|| t_string!(i18n, game_ongoing).to_string());

    view! {
        <div class="portal-card">
            <h1>{t!(i18n, room_detail_title)} " " { game.room_code.clone() }</h1>
            <p class="portal-meta">
                {t!(i18n, started_label)} ": " { started.clone() }
                " · "
                {t!(i18n, ended_label)} ": " { ended }
            </p>

            <h2>{t!(i18n, players_header)}</h2>
            <table>
                <thead>
                    <tr>
                        <th>{t!(i18n, col_player)}</th>
                        <th>{t!(i18n, label_username)}</th>
                        <th>{t!(i18n, col_outcome)}</th>
                    </tr>
                </thead>
                <tbody>
                    {game.participants.iter().map(|p| {
                        view! { <ParticipantRow participant=p.clone() /> }
                    }).collect_view()}
                </tbody>
            </table>

            {game.result.as_ref().map(|r| view! {
                <div style="margin-top:1.5rem">
                    <h2>{t!(i18n, score_header)}</h2>
                    <p style="font-family:var(--font-display);font-size:1.1rem;color:var(--ui-ink)">
                        { r.clone() }
                    </p>
                </div>
            })}
        </div>
    }
}

#[component]
fn ParticipantRow(participant: Participant) -> impl IntoView {
    let i18n = use_i18n();
    let outcome_class = match participant.outcome.as_deref() {
        Some("win")  => "outcome-win",
        Some("loss") => "outcome-loss",
        Some("draw") => "outcome-draw",
        _            => "",
    };
    let outcome_text = move || match participant.outcome.as_deref() {
        Some("win")  => t_string!(i18n, outcome_win),
        Some("loss") => t_string!(i18n, outcome_loss),
        Some("draw") => t_string!(i18n, outcome_draw),
        _            => "—",
    };
    let name = participant.username.clone();

    view! {
        <tr>
            <td>{t!(i18n, col_player)} " " { participant.player_id }</td>
            <td>
                {match name {
                    Some(u) => view! {
                        <A href=format!("/profile/{u}")>{ u }</A>
                    }.into_any(),
                    None => view! {
                        <span style="color:#aa9070">{t!(i18n, anonymous_player)}</span>
                    }.into_any(),
                }}
            </td>
            <td class=outcome_class>{ outcome_text }</td>
        </tr>
    }
}
