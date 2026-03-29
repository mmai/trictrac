use leptos::prelude::*;
use trictrac_store::{CheckerMove, Jan};

use crate::i18n::*;
use crate::trictrac::types::{JanEntry, PlayerScore};

fn jan_label(jan: &Jan) -> String {
    let i18n = use_i18n();
    match jan {
        Jan::FilledQuarter => t_string!(i18n, jan_filled_quarter).to_owned(),
        Jan::TrueHitSmallJan => t_string!(i18n, jan_true_hit_small).to_owned(),
        Jan::TrueHitBigJan => t_string!(i18n, jan_true_hit_big).to_owned(),
        Jan::TrueHitOpponentCorner => t_string!(i18n, jan_true_hit_corner).to_owned(),
        Jan::FirstPlayerToExit => t_string!(i18n, jan_first_exit).to_owned(),
        Jan::SixTables => t_string!(i18n, jan_six_tables).to_owned(),
        Jan::TwoTables => t_string!(i18n, jan_two_tables).to_owned(),
        Jan::Mezeas => t_string!(i18n, jan_mezeas).to_owned(),
        Jan::FalseHitSmallJan => t_string!(i18n, jan_false_hit_small).to_owned(),
        Jan::FalseHitBigJan => t_string!(i18n, jan_false_hit_big).to_owned(),
        Jan::ContreTwoTables => t_string!(i18n, jan_contre_two).to_owned(),
        Jan::ContreMezeas => t_string!(i18n, jan_contre_mezeas).to_owned(),
        Jan::HelplessMan => t_string!(i18n, jan_helpless_man).to_owned(),
    }
}

fn format_move_pair(m1: CheckerMove, m2: CheckerMove) -> String {
    let fmt = |m: CheckerMove| -> String {
        let (f, t) = (m.get_from(), m.get_to());
        if f == 0 && t == 0 {
            "—".to_string()
        } else if t == 0 {
            format!("{f}↑")
        } else {
            format!("{f}→{t}")
        }
    };
    format!("{} & {}", fmt(m1), fmt(m2))
}

fn jan_row(idx: usize, entry: JanEntry, expanded: RwSignal<Option<usize>>) -> impl IntoView {
    let i18n = use_i18n();
    let row_class = if entry.total >= 0 {
        "jan-row jan-expandable jan-positive"
    } else {
        "jan-row jan-expandable jan-negative"
    };
    let label = jan_label(&entry.jan);
    let double_tag = if entry.is_double {
        t_string!(i18n, jan_double).to_owned()
    } else {
        t_string!(i18n, jan_simple).to_owned()
    };
    let ways_tag = format!("×{}", entry.ways);
    let pts_str = if entry.total >= 0 {
        format!("+{}", entry.total)
    } else {
        format!("{}", entry.total)
    };

    let moves = entry.moves.clone();

    view! {
        <div>
            <div
                class=row_class
                on:click=move |_| {
                    expanded.update(|s| {
                        *s = if *s == Some(idx) { None } else { Some(idx) };
                    });
                }
            >
                <span class="jan-label">{label}</span>
                <span class="jan-tag">{double_tag}</span>
                <span class="jan-tag">{ways_tag}</span>
                <span class="jan-pts">{pts_str}</span>
            </div>
            {
                let move_lines: Vec<_> = moves.iter()
                    .map(|&(m1, m2)| {
                        let text = format_move_pair(m1, m2);
                        view! { <div class="jan-move-line">{text}</div> }
                    })
                    .collect();
                view! {
                    <div class="jan-moves" class:hidden=move || expanded.get() != Some(idx)>
                        {move_lines}
                    </div>
                }
            }
        </div>
    }
}

#[component]
pub fn PlayerScorePanel(score: PlayerScore, jans: Vec<JanEntry>, is_you: bool) -> impl IntoView {
    let i18n = use_i18n();

    let points_pct = format!("{}%", (score.points as u32 * 100 / 12).min(100));
    let holes_pct = format!("{}%", (score.holes as u32 * 100 / 12).min(100));
    let points_val = format!("{}/12", score.points);
    let holes_val = format!("{}/12", score.holes);
    let can_bredouille = score.can_bredouille;

    let expanded: RwSignal<Option<usize>> = RwSignal::new(None);
    let jan_rows: Vec<_> = jans
        .into_iter()
        .enumerate()
        .map(|(i, entry)| jan_row(i, entry, expanded))
        .collect();

    view! {
        <div class="player-score-panel">
            <div class="player-score-header">
                <span class="player-name">
                    {score.name}
                    {is_you.then(|| t!(i18n, you_suffix))}
                </span>
            </div>
            <div class="score-bars">
                <div class="score-bar-row">
                    <span class="score-bar-label">{t!(i18n, points_label)}</span>
                    <div class="score-bar">
                        <div class="score-bar-fill score-bar-points" style=format!("width:{points_pct}")></div>
                    </div>
                    <span class="score-bar-value">{points_val}</span>
                    {can_bredouille.then(|| view! {
                        <span class="bredouille-badge" title=move || t_string!(i18n, bredouille_title).to_owned()>"B"</span>
                    })}
                </div>
                <div class="score-bar-row">
                    <span class="score-bar-label">{t!(i18n, holes_label)}</span>
                    <div class="score-bar">
                        <div class="score-bar-fill score-bar-holes" style=format!("width:{holes_pct}")></div>
                    </div>
                    <span class="score-bar-value">{holes_val}</span>
                </div>
            </div>
            {(!jan_rows.is_empty()).then(|| view! {
                <div class="player-jans">{jan_rows}</div>
            })}
        </div>
    }
}
