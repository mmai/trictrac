use leptos::prelude::*;
use trictrac_store::Jan;
#[cfg(target_arch = "wasm32")]
use gloo_timers::future::TimeoutFuture;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::spawn_local;
#[cfg(target_arch = "wasm32")]
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

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

/// Merged scoreboard showing both players above the board.
///
/// - Two stacked rows for a clear race-to-12 visual comparison.
/// - Points shown as an animated jackpot counter (ticks up on each new point).
/// - Hole pegs are larger and use green (me) / red (opponent) instead of gold.
/// - When a hole is gained, the new peg pops in and a brief non-blocking label
///   appears instead of the old blocking toast popup.
#[component]
pub fn MergedScorePanel(
    my_score: PlayerScore,
    opp_score: PlayerScore,
    /// Points just earned this turn; 0 = no animation. Set to 0 when a hole
    /// was gained (points wrap around 12, counter stays at end value).
    #[prop(default = 0)] my_points_earned: u8,
    #[prop(default = 0)] opp_points_earned: u8,
    /// Non-zero when a new hole was just scored (triggers peg-pop animation).
    #[prop(default = 0)] my_holes_gained: u8,
    #[prop(default = 0)] opp_holes_gained: u8,
    /// True when my hole was scored under bredouille (shows ×2 in the flash).
    #[prop(default = false)] my_bredouille: bool,
) -> impl IntoView {
    let i18n = use_i18n();

    // ── Points counter signals ──────────────────────────────────────────────
    // When no hole was gained: start from (current - earned) and tick up.
    // When a hole was gained: points wrapped around 12, so skip the animation.
    // On non-WASM there is no animation; start directly at the final value.
    // Suppress the unused-variable warning for animation-only params.
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (my_points_earned, opp_points_earned);
    #[cfg(not(target_arch = "wasm32"))]
    let my_pts_start = my_score.points;
    #[cfg(target_arch = "wasm32")]
    let my_pts_start = if my_holes_gained == 0 {
        my_score.points.saturating_sub(my_points_earned)
    } else {
        my_score.points
    };
    let my_displayed_pts: RwSignal<u8> = RwSignal::new(my_pts_start);

    #[cfg(not(target_arch = "wasm32"))]
    let opp_pts_start = opp_score.points;
    #[cfg(target_arch = "wasm32")]
    let opp_pts_start = if opp_holes_gained == 0 {
        opp_score.points.saturating_sub(opp_points_earned)
    } else {
        opp_score.points
    };
    let opp_displayed_pts: RwSignal<u8> = RwSignal::new(opp_pts_start);

    // ── Jackpot counter animation (WASM only) ───────────────────────────────
    #[cfg(target_arch = "wasm32")]
    {
        let my_pts_end = my_score.points;
        if my_pts_start < my_pts_end {
            let is_alive = Arc::new(AtomicBool::new(true));
            let alive_c = is_alive.clone();
            on_cleanup(move || alive_c.store(false, Ordering::Relaxed));
            spawn_local(async move {
                for p in (my_pts_start + 1)..=my_pts_end {
                    TimeoutFuture::new(200).await;
                    if !is_alive.load(Ordering::Relaxed) { return; }
                    my_displayed_pts.set(p);
                    crate::game::sound::play_points_tick();
                }
            });
        }
        let opp_pts_end = opp_score.points;
        if opp_pts_start < opp_pts_end {
            let is_alive = Arc::new(AtomicBool::new(true));
            let alive_c = is_alive.clone();
            on_cleanup(move || alive_c.store(false, Ordering::Relaxed));
            spawn_local(async move {
                for p in (opp_pts_start + 1)..=opp_pts_end {
                    TimeoutFuture::new(200).await;
                    if !is_alive.load(Ordering::Relaxed) { return; }
                    opp_displayed_pts.set(p);
                }
            });
        }
    }

    // ── Ghost bar widths (show the end value immediately — static reference) ─
    let my_bar_style  = format!("width:{}%", (my_score.points  as u32 * 100 / 12).min(100));
    let opp_bar_style = format!("width:{}%", (opp_score.points as u32 * 100 / 12).min(100));

    // ── Hole peg tracks ─────────────────────────────────────────────────────
    let my_holes  = my_score.holes;
    let opp_holes = opp_score.holes;

    let my_pegs: Vec<AnyView> = (1u8..=12)
        .map(|i| {
            let filled = i <= my_holes;
            let is_new = filled && i == my_holes && my_holes_gained > 0;
            view! {
                <div class="peg-hole"
                     class:filled=filled
                     class:peg-new=is_new>
                </div>
            }.into_any()
        })
        .collect();

    let opp_pegs: Vec<AnyView> = (1u8..=12)
        .map(|i| {
            let filled = i <= opp_holes;
            let is_new = filled && i == opp_holes && opp_holes_gained > 0;
            view! {
                <div class="peg-hole peg-opp"
                     class:filled=filled
                     class:peg-new=is_new>
                </div>
            }.into_any()
        })
        .collect();

    let my_name  = my_score.name.clone();
    let opp_name = opp_score.name.clone();
    let my_can_bredouille  = my_score.can_bredouille;
    let opp_can_bredouille = opp_score.can_bredouille;

    view! {
        <div class="merged-score-panel">

            // ── My player row ───────────────────────────────────────────
            <div class="score-row score-row-me">
                <div class="score-row-name">
                    <span class="player-name">{my_name}</span>
                    <span class="you-tag">{t!(i18n, you_suffix)}</span>
                </div>
                <div class="pts-counter-wrap">
                    <div class="pts-ghost-bar-track">
                        <div class="pts-ghost-bar-fill" style=my_bar_style></div>
                    </div>
                    <div class="pts-counter-row">
                        <span class="pts-counter">{move || my_displayed_pts.get()}</span>
                        <span class="pts-max">"/12"</span>
                    </div>
                </div>
                <div class="peg-track">{my_pegs}</div>
                {my_can_bredouille.then(|| view! {
                    <span class="bredouille-badge"
                          title=move || t_string!(i18n, bredouille_title).to_owned()>
                        "B"
                    </span>
                })}
                // Flash sits in the free space to the right of the pegs.
                // margin-left:auto keeps it right-aligned inside the flex row
                // without adding a new row, so the board never shifts down.
                {(my_holes_gained > 0).then(|| {
                    let label = if my_bredouille {
                        format!("Trou {} · ×2 bredouille", my_holes)
                    } else {
                        format!("Trou {}", my_holes)
                    };
                    view! {
                        <div class="hole-flash"
                             class:hole-flash-bredouille=my_bredouille>
                            {label}
                        </div>
                    }
                })}
            </div>

            <div class="score-row-sep"></div>

            // ── Opponent row ────────────────────────────────────────────
            <div class="score-row score-row-opp">
                <div class="score-row-name">
                    <span class="player-name">{opp_name}</span>
                </div>
                <div class="pts-counter-wrap">
                    <div class="pts-ghost-bar-track">
                        <div class="pts-ghost-bar-fill pts-ghost-bar-opp" style=opp_bar_style></div>
                    </div>
                    <div class="pts-counter-row">
                        <span class="pts-counter">{move || opp_displayed_pts.get()}</span>
                        <span class="pts-max">"/12"</span>
                    </div>
                </div>
                <div class="peg-track">{opp_pegs}</div>
                {opp_can_bredouille.then(|| view! {
                    <span class="bredouille-badge"
                          title=move || t_string!(i18n, bredouille_title).to_owned()>
                        "B"
                    </span>
                })}
            </div>
        </div>
    }
}
