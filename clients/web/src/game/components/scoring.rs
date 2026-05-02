use futures::channel::mpsc::UnboundedSender;
#[cfg(target_arch = "wasm32")]
use gloo_timers::future::TimeoutFuture;
use leptos::prelude::*;
#[cfg(target_arch = "wasm32")]
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use trictrac_store::CheckerMove;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::spawn_local;

use crate::app::NetCommand;
use crate::game::trictrac::types::{JanEntry, PlayerAction, ScoredEvent, SerTurnStage};
use crate::i18n::*;

use super::score_panel::jan_label;

/// One row in the scoring panel. Sets the hovered-moves context on enter
/// (so board shows arrows for that jan's moves), but does NOT clear on
/// leave — clearing is handled by the outer wrapper's mouseleave so that
/// arrows persist while the pointer moves between rows.
fn scoring_jan_row(entry: JanEntry) -> impl IntoView {
    let i18n = use_i18n();
    let hovered = use_context::<RwSignal<Vec<(CheckerMove, CheckerMove)>>>();
    let jan = entry.jan;
    let is_double = entry.is_double;
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
        >
            <span class="jan-label">{move || jan_label(&jan)}</span>
            <span class="jan-tag">{move || if is_double {
                t_string!(i18n, jan_double).to_owned()
            } else {
                t_string!(i18n, jan_simple).to_owned()
            }}</span>
            <span class="jan-tag">{ways_tag}</span>
            <span class="jan-pts">{pts_str}</span>
        </div>
    }
}

/// Scoring detail panel, shown to the right of the hole counter in the merged
/// score panel area.
///
/// Lifecycle:
/// 1. Mounts expanded — shows all jan details and draws board arrows.
/// 2. After 3.4 s the arrows clear and the panel auto-minimises to a small "+"
///    button (unless Hold/Go buttons are still needed).
/// 3. The "+" / "−" buttons let the player toggle between states at any time.
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
    let panel_class = if is_opponent {
        "scoring-panel scoring-panel-opp"
    } else {
        "scoring-panel"
    };

    // minimized: starts false (expanded)
    let minimized = RwSignal::new(false);

    // Collect all moves from all jans for automatic arrow display.
    let all_moves: Vec<(CheckerMove, CheckerMove)> = event
        .jans
        .iter()
        .flat_map(|e| e.moves.iter().cloned())
        .collect();
    let all_moves_auto = all_moves.clone();
    let all_moves_expand = all_moves.clone();
    let all_moves_enter = all_moves.clone();

    let hovered_ctx = use_context::<RwSignal<Vec<(CheckerMove, CheckerMove)>>>();
    let jan_rows: Vec<_> = event.jans.into_iter().map(scoring_jan_row).collect();

    // On mount: show all this event's moves as board arrows immediately,
    // then after 3.4 s slide to peek and clear the arrows.
    //
    // Two important constraints:
    // 1. The initial hm.set() must be deferred (spawn_local, not sync in body)
    //    to avoid writing a reactive signal mid-render while Board reads it —
    //    that triggers Leptos's cycle guard → `unreachable` WASM panic.
    // 2. The cancellation flag must be Rc<Cell<bool>>, NOT RwSignal<bool>.
    //    RwSignal is a NodeId into Leptos's arena; the arena slot is freed
    //    when ScoringPanel's owner drops (on every GameScreen remount). If the
    //    3.4 s future outlives the component and calls is_alive.get_untracked()
    //    on a freed slot, that also panics with `unreachable`. Rc<Cell<bool>>
    //    is reference-counted outside the arena and stays valid for as long as
    //    the future holds onto it.
    #[cfg(target_arch = "wasm32")]
    if let Some(hm) = hovered_ctx {
        let is_alive = Arc::new(AtomicBool::new(true));
        let is_alive_cleanup = is_alive.clone();
        // on_cleanup requires Send + Sync; Arc<AtomicBool> satisfies both.
        on_cleanup(move || is_alive_cleanup.store(false, Ordering::Relaxed));

        spawn_local(async move {
            // Show arrows (runs in the next microtask, after render settles).
            hm.set(all_moves);

            TimeoutFuture::new(3_400).await;
            // Guard: component may have been destroyed while we were waiting.
            // is_alive was set to false by on_cleanup, which runs before Leptos
            // frees the signal arena slots — so peeked is still valid iff this
            // returns true.
            if !is_alive.load(Ordering::Relaxed) {
                return;
            }
            hm.set(vec![]);
        });
    }

    view! {
        <div
            class="scoring-panel-wrapper"
            class:scoring-minimized=move || minimized.get()
            on:mouseenter=move |_| {
                if let Some(hm) = hovered_ctx {
                    hm.set(all_moves_enter.clone());
                }
            }
            on:mouseleave=move |_| {
                if let Some(hm) = hovered_ctx {
                    hm.set(vec![]);
                }
            }
        >
            // "+" expand button — shown only when minimised (CSS hides it otherwise).
            <button
                class="scoring-expand-btn"
                title="Show scoring details"
                on:click=move |ev: leptos::web_sys::MouseEvent| {
                    ev.stop_propagation();
                    minimized.set(false);
                    if let Some(hm) = hovered_ctx {
                        hm.set(all_moves_expand.clone());
                    }
                }
            >
                "+"
            </button>

            // Full panel — hidden when minimised via CSS.
            <div class=panel_class>
                <div class="scoring-panel-head">
                    <div class="scoring-total">
                        {move || if is_opponent {
                            t_string!(i18n, opp_scored_pts, n = points_earned)
                        } else {
                            t_string!(i18n, scored_pts, n = points_earned)
                        }}
                    </div>
                    <button
                        class="scoring-collapse-btn"
                        title="Minimise"
                        on:click=move |ev: leptos::web_sys::MouseEvent| {
                            ev.stop_propagation();
                            minimized.set(true);
                            if let Some(hm) = hovered_ctx {
                                hm.set(vec![]);
                            }
                        }
                    >
                        "−"
                    </button>
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
                {show_hold_go.then(|| {
                    let dismissed = RwSignal::new(false);
                    view! {
                        <div class="hold-go-buttons" class:hidden=move || dismissed.get()>
                            <button class="btn btn-secondary"
                                on:click=move |ev: leptos::web_sys::MouseEvent| {
                                    ev.stop_propagation();
                                    dismissed.set(true);
                                }
                            >
                                {t!(i18n, hold)}
                            </button>
                            <button class="btn btn-primary"
                                on:click=move |ev: leptos::web_sys::MouseEvent| {
                                    ev.stop_propagation();
                                    cmd_tx
                                        .unbounded_send(NetCommand::Action(PlayerAction::Go))
                                        .ok();
                                }
                            >
                                {t!(i18n, go)}
                            </button>
                        </div>
                    }
                })}
            </div>
        </div>
    }
}
