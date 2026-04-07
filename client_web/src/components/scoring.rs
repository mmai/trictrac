use futures::channel::mpsc::UnboundedSender;
use leptos::prelude::*;
use trictrac_store::CheckerMove;

use crate::app::NetCommand;
use crate::i18n::*;
use crate::trictrac::types::{JanEntry, PlayerAction, ScoredEvent, SerTurnStage};

use super::score_panel::jan_label;

fn scoring_jan_row(entry: JanEntry) -> impl IntoView {
    let i18n = use_i18n();
    let hovered = use_context::<RwSignal<Vec<(CheckerMove, CheckerMove)>>>();
    let label = jan_label(&entry.jan);
    let double_tag = if entry.is_double {
        t_string!(i18n, jan_double).to_owned()
    } else {
        t_string!(i18n, jan_simple).to_owned()
    };
    let ways_tag = format!("×{}", entry.ways);
    let pts_str = format!("+{}", entry.total);
    let moves_hover = entry.moves.clone();

    view! {
        <div
            class="scoring-jan-row"
            on:mouseenter=move |_| {
                if let Some(h) = hovered {
                    h.set(moves_hover.clone());
                }
            }
            on:mouseleave=move |_| {
                if let Some(h) = hovered {
                    h.set(vec![]);
                }
            }
        >
            <span class="jan-label">{label}</span>
            <span class="jan-tag">{double_tag}</span>
            <span class="jan-tag">{ways_tag}</span>
            <span class="jan-pts">{pts_str}</span>
        </div>
    }
}

#[component]
pub fn ScoringPanel(
    event: ScoredEvent,
    turn_stage: SerTurnStage,
    #[prop(default = false)] is_opponent: bool,
) -> impl IntoView {
    let i18n = use_i18n();
    let cmd_tx = use_context::<UnboundedSender<NetCommand>>()
        .expect("UnboundedSender<NetCommand> not found in context");

    let points_earned = event.points_earned;
    let holes_gained = event.holes_gained;
    let holes_total = event.holes_total;
    let bredouille = event.bredouille;
    let show_hold_go = !is_opponent && turn_stage == SerTurnStage::HoldOrGoChoice;
    let panel_class = if is_opponent { "scoring-panel scoring-panel-opp" } else { "scoring-panel" };

    let jan_rows: Vec<_> = event.jans.into_iter().map(scoring_jan_row).collect();

    view! {
        <div class=panel_class>
            <div class="scoring-total">
                {move || if is_opponent {
                    t_string!(i18n, opp_scored_pts, n = points_earned)
                } else {
                    t_string!(i18n, scored_pts, n = points_earned)
                }}
            </div>
            {jan_rows}
            {(holes_gained > 0).then(|| view! {
                <div class="scoring-hole">
                    <span>{move || if is_opponent {
                        t_string!(i18n, opp_hole_made, holes = holes_total)
                    } else {
                        t_string!(i18n, hole_made, holes = holes_total)
                    }}</span>
                    {bredouille.then(|| view! {
                        <span class="bredouille-badge">
                            {move || t_string!(i18n, bredouille_applied)}
                        </span>
                    })}
                </div>
            })}
            {show_hold_go.then(|| view! {
                <div class="hold-go-buttons">
                    <button class="btn btn-secondary">
                        {t!(i18n, hold)}
                    </button>
                    <button class="btn btn-primary" on:click=move |_| {
                        cmd_tx.unbounded_send(NetCommand::Action(PlayerAction::Go)).ok();
                    }>
                        {t!(i18n, go)}
                    </button>
                </div>
            })}
        </div>
    }
}
