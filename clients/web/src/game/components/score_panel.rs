use leptos::prelude::*;
use trictrac_store::Jan;

use crate::i18n::*;
use crate::game::trictrac::types::PlayerScore;

pub fn jan_label(jan: &Jan) -> String {
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

#[component]
pub fn PlayerScorePanel(score: PlayerScore, is_you: bool) -> impl IntoView {
    let i18n = use_i18n();

    let points_pct = format!("{}%", (score.points as u32 * 100 / 12).min(100));
    let points_val = format!("{}/12", score.points);
    let holes = score.holes;
    let can_bredouille = score.can_bredouille;

    // 12 peg holes; filled up to `holes`
    let pegs: Vec<AnyView> = (1u8..=12)
        .map(|i| {
            let cls = if i <= holes { "peg-hole filled" } else { "peg-hole" };
            view! { <div class=cls></div> }.into_any()
        })
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
                    <div class="peg-track">{pegs}</div>
                    <span class="score-bar-value">{format!("{holes}/12")}</span>
                </div>
            </div>
        </div>
    }
}
