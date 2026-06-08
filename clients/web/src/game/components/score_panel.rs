#[cfg(target_arch = "wasm32")]
use gloo_timers::future::TimeoutFuture;
use leptos::prelude::*;
#[cfg(target_arch = "wasm32")]
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use trictrac_store::Jan;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::spawn_local;

use crate::game::trictrac::types::PlayerScore;
use crate::i18n::*;

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

/// Full-width player strip at the top of the game screen.
///
/// - Left side: me (right-aligned toward center): avatar → name → pegs → pts.
/// - Center: "Trictrac" italic title.
/// - Right side: opponent (left-aligned from center): pts → pegs → name → avatar.
/// - Active player zone gets a subtle rounded highlight.
/// - Points animate as a jackpot counter; new peg pops in with an animation.
#[component]
pub fn MergedScorePanel(
    my_score: PlayerScore,
    opp_score: PlayerScore,
    /// Points just earned this turn; 0 = no animation.
    #[prop(default = 0)]
    my_points_earned: u8,
    #[prop(default = 0)] opp_points_earned: u8,
    /// Non-zero when a new hole was just scored (triggers peg-pop animation).
    #[prop(default = 0)]
    my_holes_gained: u8,
    #[prop(default = 0)] opp_holes_gained: u8,
    /// True when my hole was scored under bredouille (shows ×2 in the flash).
    #[prop(default = false)]
    my_bredouille: bool,
    /// `Some(true)` = my turn active, `Some(false)` = opponent active, `None` = no active turn.
    #[prop(default = None)]
    active_player_is_me: Option<bool>,
) -> impl IntoView {
    let i18n = use_i18n();

    // ── Points counter signals ──────────────────────────────────────────────
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
                    TimeoutFuture::new(100).await;
                    if !is_alive.load(Ordering::Relaxed) {
                        return;
                    }
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
                    TimeoutFuture::new(100).await;
                    if !is_alive.load(Ordering::Relaxed) {
                        return;
                    }
                    opp_displayed_pts.set(p);
                    crate::game::sound::play_opp_points_tick();
                }
            });
        }
    }

    // ── Hole peg tracks ─────────────────────────────────────────────────────
    let my_holes = my_score.holes;
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
            }
            .into_any()
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
            }
            .into_any()
        })
        .collect();

    let my_name = my_score.name.clone();
    let opp_name = opp_score.name.clone();
    let my_can_bredouille = my_score.can_bredouille;
    let opp_can_bredouille = opp_score.can_bredouille;

    let my_active = active_player_is_me == Some(true);
    let opp_active = active_player_is_me == Some(false);

    view! {
        <div class="players-strip">

            // ── My player: left side, right-aligned toward center ───────────
            <div class="strip-player strip-player-left">
                <div class="strip-active-zone" class:active=my_active>
                    <div class="strip-avatar strip-avatar-me"></div>
                    <div class="score-row-name">
                        <span class="player-name">{my_name}</span>
                    </div>
                    {my_can_bredouille.then(|| view! {
                        <span class="bredouille-badge"
                              title=move || t_string!(i18n, bredouille_title).to_owned()>
                            "B"
                        </span>
                    })}
                    <div class="peg-track">{my_pegs}</div>
                    <div class="pts-counter-wrap">
                        <div class="pts-counter-row">
                            <span class="pts-counter">{move || my_displayed_pts.get()}</span>
                            <span class="pts-max">"/12"</span>
                        </div>
                    </div>
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
            </div>

            // ── Center title ────────────────────────────────────────────────
            <div class="strip-center">
                <span class="strip-title">"Trictrac"</span>
            </div>

            // ── Opponent: right side, left-aligned from center ──────────────
            <div class="strip-player strip-player-right">
                <div class="strip-active-zone" class:active=opp_active>
                    <div class="pts-counter-wrap">
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
                    <div class="score-row-name">
                        <span class="player-name">{opp_name}</span>
                    </div>
                    <div class="strip-avatar strip-avatar-opp"></div>
                </div>
            </div>

        </div>
    }
}
