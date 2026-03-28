use leptos::prelude::*;
use trictrac_store::{CheckerMove, Jan};

use crate::trictrac::types::{JanEntry, PlayerScore};

fn jan_label(jan: &Jan) -> &'static str {
    match jan {
        Jan::FilledQuarter => "Remplissage",
        Jan::TrueHitSmallJan => "Battage à vrai (petit jan)",
        Jan::TrueHitBigJan => "Battage à vrai (grand jan)",
        Jan::TrueHitOpponentCorner => "Battage coin adverse",
        Jan::FirstPlayerToExit => "Premier sorti",
        Jan::SixTables => "Six tables",
        Jan::TwoTables => "Deux tables",
        Jan::Mezeas => "Mezeas",
        Jan::FalseHitSmallJan => "Battage à faux (petit jan)",
        Jan::FalseHitBigJan => "Battage à faux (grand jan)",
        Jan::ContreTwoTables => "Contre deux tables",
        Jan::ContreMezeas => "Contre mezeas",
        Jan::HelplessMan => "Dame impuissante",
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
    let row_class = if entry.total >= 0 {
        "jan-row jan-expandable jan-positive"
    } else {
        "jan-row jan-expandable jan-negative"
    };
    let label = jan_label(&entry.jan);
    let double_tag = if entry.is_double { "double" } else { "simple" };
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

/// One player's score panel: name, progress bars (points & holes), bredouille indicator,
/// and the list of jans scored by this player in the last roll.
/// `jans` should already be filtered and sign-corrected for this player's perspective.
#[component]
pub fn PlayerScorePanel(score: PlayerScore, jans: Vec<JanEntry>, is_you: bool) -> impl IntoView {
    let label = if is_you { " (vous)" } else { "" };
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
                <span class="player-name">{score.name}{label}</span>
            </div>
            <div class="score-bars">
                <div class="score-bar-row">
                    <span class="score-bar-label">"Points"</span>
                    <div class="score-bar">
                        <div class="score-bar-fill score-bar-points" style=format!("width:{points_pct}")></div>
                    </div>
                    <span class="score-bar-value">{points_val}</span>
                    {can_bredouille.then(|| view! {
                        <span class="bredouille-badge" title="Peut faire bredouille">"B"</span>
                    })}
                </div>
                <div class="score-bar-row">
                    <span class="score-bar-label">"Trous"</span>
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
