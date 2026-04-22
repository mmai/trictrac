use leptos::prelude::*;
use trictrac_store::CheckerMove;

use super::die::Die;
use crate::trictrac::types::{SerTurnStage, ViewState};

/// Field numbers in visual display order (left-to-right for each quarter), white's perspective.
const TOP_LEFT_W: [u8; 6] = [13, 14, 15, 16, 17, 18];
const TOP_RIGHT_W: [u8; 6] = [19, 20, 21, 22, 23, 24];
const BOT_LEFT_W: [u8; 6] = [12, 11, 10, 9, 8, 7];
const BOT_RIGHT_W: [u8; 6] = [6, 5, 4, 3, 2, 1];

/// 180° rotation of white's layout: black's pieces (field 24) appear at the bottom.
const TOP_LEFT_B: [u8; 6] = [1, 2, 3, 4, 5, 6];
const TOP_RIGHT_B: [u8; 6] = [7, 8, 9, 10, 11, 12];
const BOT_LEFT_B: [u8; 6] = [24, 23, 22, 21, 20, 19];
const BOT_RIGHT_B: [u8; 6] = [18, 17, 16, 15, 14, 13];

/// The rest corner is field 12 (White) or field 13 (Black) in the store's coordinate system.
/// Returns true when `field_num` is the rest corner for this perspective.
#[allow(dead_code)]
fn is_rest_corner(field_num: u8, is_white: bool) -> bool {
    if is_white {
        field_num == 12
    } else {
        field_num == 13
    }
}

/// Zone CSS class for a field number (field coordinates are always White's 1-24).
fn field_zone_class(field_num: u8) -> &'static str {
    match field_num {
        1..=6 => "zone-petit",
        7..=12 => "zone-grand",
        13..=18 => "zone-opponent",
        19..=24 => "zone-retour",
        _ => "",
    }
}

/// Returns (d0_used, d1_used) for the bar dice display.
fn bar_matched_dice_used(staged: &[(u8, u8)], dice: (u8, u8)) -> (bool, bool) {
    let mut d0 = false;
    let mut d1 = false;
    for &(from, to) in staged {
        let dist = if from < to {
            to.saturating_sub(from)
        } else {
            from.saturating_sub(to)
        };
        if !d0 && dist == dice.0 {
            d0 = true;
        } else if !d1 && dist == dice.1 {
            d1 = true;
        } else if !d0 {
            d0 = true;
        } else {
            d1 = true;
        }
    }
    (d0, d1)
}

/// Returns the displayed board value for `field_num` after applying `staged_moves`.
/// Field numbers are always in white's coordinate system (1–24).
fn displayed_value(
    base_board: [i8; 24],
    staged_moves: &[(u8, u8)],
    is_white: bool,
    field_num: u8,
) -> i8 {
    let mut val = base_board[(field_num - 1) as usize];
    let delta: i8 = if is_white { 1 } else { -1 };
    for &(from, to) in staged_moves {
        if from == field_num {
            val -= delta;
        }
        if to == field_num {
            val += delta;
        }
    }
    val
}

/// Fields whose checkers may be selected as the next origin given already-staged moves.
fn valid_origins_for(seqs: &[(CheckerMove, CheckerMove)], staged: &[(u8, u8)]) -> Vec<u8> {
    let mut v: Vec<u8> = match staged.len() {
        0 => seqs
            .iter()
            .map(|(m1, _)| m1.get_from() as u8)
            .filter(|&f| f != 0)
            .collect(),
        1 => {
            let (f0, t0) = staged[0];
            seqs.iter()
                .filter(|(m1, _)| m1.get_from() as u8 == f0 && m1.get_to() as u8 == t0)
                .map(|(_, m2)| m2.get_from() as u8)
                .filter(|&f| f != 0)
                .collect()
        }
        _ => vec![],
    };
    v.sort_unstable();
    v.dedup();
    v
}

/// Pixel center of a board field in the SVG overlay coordinate space.
/// Geometry: field 60×180px, board padding 4px, gap 4px, bar 20px, center-bar 12px.
/// With triangular flèches, arrows target the WIDE BASE of each triangle —
/// that is where the checker stack actually sits.
fn field_center(f: usize, is_white: bool) -> Option<(f32, f32)> {
    if f == 0 || f > 24 {
        return None;
    }
    let (qi, right, top): (usize, bool, bool) = if is_white {
        match f {
            13..=18 => (f - 13, false, true),
            19..=24 => (f - 19, true, true),
            7..=12 => (12 - f, false, false),
            1..=6 => (6 - f, true, false),
            _ => return None,
        }
    } else {
        match f {
            1..=6 => (f - 1, false, true),
            7..=12 => (f - 7, true, true),
            19..=24 => (24 - f, false, false),
            13..=18 => (18 - f, true, false),
            _ => return None,
        }
    };
    // Left-quarter field i center x:  4(pad) + i*62 + 30(half field) = 34 + 62i
    // Right-quarter:  4 + 370(quarter) + 4(gap) + 68(bar) + 4(gap) + i*62 + 30 = 480 + 62i
    let x = if right {
        480.0 + qi as f32 * 62.0
    } else {
        34.0 + qi as f32 * 62.0
    };
    // Top row triangle base (wide end) ≈ y=30; bot row triangle base ≈ y=358.
    // (Top base: 4pad + 4field-pad + 20half-checker ≈ 28; Bot base: 388 − 4pad − 4field-pad − 20 ≈ 360)
    let y = if top { 30.0 } else { 358.0 };
    Some((x, y))
}

/// SVG `<g>` element drawing one arrow (shadow + gold) from `fp` to `tp`.
fn arrow_svg(fp: (f32, f32), tp: (f32, f32)) -> AnyView {
    let (x1, y1) = fp;
    let (x2, y2) = tp;
    let dx = x2 - x1;
    let dy = y2 - y1;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 10.0 {
        return view! { <g /> }.into_any();
    }
    let nx = dx / len;
    let ny = dy / len;
    let px = -ny;
    let py = nx;

    // Shrink line ends so arrows don't overlap the checker stack
    let lx1 = x1 + nx * 20.0;
    let ly1 = y1 + ny * 20.0;
    let lx2 = x2 - nx * 15.0;
    let ly2 = y2 - ny * 15.0;

    // Arrowhead triangle at (x2, y2)
    let ah = 15.0_f32;
    let aw = 7.0_f32;
    let bx = x2 - nx * ah;
    let bary = y2 - ny * ah;
    let pts = format!(
        "{:.1},{:.1} {:.1},{:.1} {:.1},{:.1}",
        x2,
        y2,
        bx + px * aw,
        bary + py * aw,
        bx - px * aw,
        bary - py * aw,
    );
    let shadow_pts = format!(
        "{:.1},{:.1} {:.1},{:.1} {:.1},{:.1}",
        x2,
        y2,
        bx + px * (aw + 1.5),
        bary + py * (aw + 1.5),
        bx - px * (aw + 1.5),
        bary - py * (aw + 1.5),
    );

    view! {
        <g>
            // Drop-shadow for readability on coloured fields
            <line
                x1=format!("{lx1:.1}") y1=format!("{ly1:.1}")
                x2=format!("{lx2:.1}") y2=format!("{ly2:.1}")
                style="stroke:rgba(0,0,0,0.45);stroke-width:5;stroke-linecap:round"
            />
            <polygon points=shadow_pts style="fill:rgba(0,0,0,0.45)" />
            // Gold arrow
            <line
                x1=format!("{lx1:.1}") y1=format!("{ly1:.1}")
                x2=format!("{lx2:.1}") y2=format!("{ly2:.1}")
                style="stroke:rgba(255,215,0,0.9);stroke-width:3;stroke-linecap:round"
            />
            <polygon points=pts style="fill:rgba(255,215,0,0.9)" />
        </g>
    }
    .into_any()
}

/// Valid destinations for a selected origin given already-staged moves.
/// May include 0 (exit); callers handle that case.
fn valid_dests_for(
    seqs: &[(CheckerMove, CheckerMove)],
    staged: &[(u8, u8)],
    origin: u8,
) -> Vec<u8> {
    let mut v: Vec<u8> = match staged.len() {
        0 => seqs
            .iter()
            .filter(|(m1, _)| m1.get_from() as u8 == origin)
            .map(|(m1, _)| m1.get_to() as u8)
            .collect(),
        1 => {
            let (f0, t0) = staged[0];
            seqs.iter()
                .filter(|(m1, m2)| {
                    m1.get_from() as u8 == f0
                        && m1.get_to() as u8 == t0
                        && m2.get_from() as u8 == origin
                })
                .map(|(_, m2)| m2.get_to() as u8)
                .collect()
        }
        _ => vec![],
    };
    v.sort_unstable();
    v.dedup();
    v
}

#[component]
pub fn Board(
    view_state: ViewState,
    player_id: u16,
    /// Pending origin selection (first click of a move pair).
    selected_origin: RwSignal<Option<u8>>,
    /// Moves staged so far this turn (max 2). Each entry is (from, to), 0 = empty move.
    staged_moves: RwSignal<Vec<(u8, u8)>>,
    /// All valid two-move sequences for this turn (empty when not in move stage).
    valid_sequences: Vec<(CheckerMove, CheckerMove)>,
    /// Dice to display in the center bars; None means dice not yet rolled (cups shown upright).
    #[prop(default = None)]
    bar_dice: Option<(u8, u8)>,
    /// Whether we're in the move stage (determines used/unused die appearance).
    #[prop(default = false)]
    bar_is_move: bool,
    #[prop(default = false)] is_my_turn: bool,
    /// Whether the dice are a double (golden glow).
    #[prop(default = false)]
    bar_is_double: bool,
    /// Checker moves to animate on mount (None when board unchanged).
    #[prop(default = None)]
    last_moves: Option<(CheckerMove, CheckerMove)>,
    /// Fields where a hit (battue) was scored this turn — show ripple animation.
    #[prop(default = vec![])]
    hit_fields: Vec<u8>,
) -> impl IntoView {
    let board = view_state.board;
    let is_move_stage = view_state.active_mp_player == Some(player_id)
        && matches!(
            view_state.turn_stage,
            SerTurnStage::Move | SerTurnStage::HoldOrGoChoice
        );
    let is_white = player_id == 0;
    let hovered_moves = use_context::<RwSignal<Vec<(CheckerMove, CheckerMove)>>>();

    // Exit-eligible (§8c): all the player's checkers are in their last jan.
    // White last jan = fields 19-24 (board indices 18-23, positive values).
    // Black last jan = fields 1-6  (board indices 0-5, negative values).
    let board_snapshot = view_state.board;
    let all_in_exit: bool;
    let exit_field_test: fn(u8) -> bool;
    if is_white {
        let in_exit: i8 = board_snapshot[18..24].iter().map(|&v| v.max(0)).sum();
        let total: i8 = board_snapshot.iter().map(|&v| v.max(0)).sum();
        all_in_exit = total > 0 && in_exit == total;
        exit_field_test = |f| matches!(f, 19..=24);
    } else {
        let in_exit: i8 = board_snapshot[0..6].iter().map(|&v| (-v).max(0)).sum();
        let total: i8 = board_snapshot.iter().map(|&v| (-v).max(0)).sum();
        all_in_exit = total > 0 && in_exit == total;
        exit_field_test = |f| matches!(f, 1..=6);
    }

    // `valid_sequences` is cloned per field (the Vec is small; Send-safe unlike Rc).
    let fields_from = |nums: &[u8], is_top_row: bool| -> Vec<AnyView> {
        nums.iter()
            .map(|&field_num| {
                // Each reactive closure gets its own owned clone — Vec<(CheckerMove,CheckerMove)>
                // is Send, which Leptos requires for reactive attribute functions.
                let seqs_c = valid_sequences.clone();
                let seqs_k = valid_sequences.clone();
                let corner_title = if is_rest_corner(field_num, is_white) {
                    Some("Coin de repos — must enter and leave with 2 checkers")
                } else {
                    None
                };
                // §4a — slide delta for the arriving checker at this field.
                // Computed once per field at render time; Option<(f32,f32)> is Copy.
                let slide_delta: Option<(f32, f32)> = last_moves.and_then(|(m1, m2)| {
                    [m1, m2].iter().find_map(|m| {
                        if m.get_to() != field_num as usize || m.get_from() == m.get_to() {
                            return None;
                        }
                        let (fx, fy) = field_center(m.get_from(), is_white)?;
                        let (tx, ty) = field_center(m.get_to(), is_white)?;
                        let dx = fx - tx;
                        let dy = fy - ty;
                        (dx.abs() >= 1.0 || dy.abs() >= 1.0).then_some((dx, dy))
                    })
                });
                // §6e — ripple on hit fields (battue).
                let is_hit_field = hit_fields.contains(&field_num);
                view! {
                    <div
                        id={format!("field-{field_num}")}
                        title=corner_title
                        class=move || {
                            let staged = staged_moves.get();
                            let val = displayed_value(board, &staged, is_white, field_num);
                            let is_mine = if is_white { val > 0 } else { val < 0 };
                            let can_stage = is_move_stage && staged.len() < 2;
                            let sel = selected_origin.get();

                            let mut cls = format!("field {}", field_zone_class(field_num));
                            if is_rest_corner(field_num, is_white) {
                                cls.push_str(" corner");
                                // Pulse when the corner can be reached this turn
                                if !seqs_c.is_empty() && seqs_c.iter().any(|(m1, m2)| {
                                    m1.get_to() as u8 == field_num
                                        || m2.get_to() as u8 == field_num
                                }) {
                                    cls.push_str(" corner-available");
                                }
                            }
                            if is_rest_corner(field_num, !is_white) {
                                cls.push_str(" corner");
                            }
                            if all_in_exit && exit_field_test(field_num) {
                                cls.push_str(" exit-eligible");
                            }

                            if seqs_c.is_empty() {
                                // No restriction (dice not rolled or not move stage)
                                if can_stage && (sel.is_some() || is_mine) {
                                    cls.push_str(" clickable");
                                }
                                if sel == Some(field_num) { cls.push_str(" selected"); }
                                if can_stage && sel.is_some() && sel != Some(field_num) {
                                    cls.push_str(" dest");
                                }
                            } else if can_stage {
                                if let Some(origin) = sel {
                                    if origin == field_num {
                                        cls.push_str(" selected clickable");
                                    } else {
                                        let dests = valid_dests_for(&seqs_c, &staged, origin);
                                        // Only highlight non-exit destinations (field 0 = exit has no tile)
                                        if dests.iter().any(|&d| d == field_num && d != 0) {
                                            cls.push_str(" clickable dest");
                                        }
                                    }
                                } else {
                                    let origins = valid_origins_for(&seqs_c, &staged);
                                    if origins.iter().any(|&o| o == field_num) {
                                        cls.push_str(" clickable");
                                    }
                                }
                            }

                            // §6c: highlight fields touched by the hovered jan
                            if let Some(hm) = hovered_moves {
                                let pairs = hm.get();
                                let f = field_num as usize;
                                let highlighted = pairs.iter().any(|(m1, m2)| {
                                    (m1.get_from() != 0 && m1.get_from() == f)
                                        || (m1.get_to() != 0 && m1.get_to() == f)
                                        || (m2.get_from() != 0 && m2.get_from() == f)
                                        || (m2.get_to() != 0 && m2.get_to() == f)
                                });
                                if highlighted {
                                    cls.push_str(" jan-hovered");
                                }
                            }

                            cls
                        }
                        on:click=move |_| {
                            if !is_move_stage { return; }
                            let staged = staged_moves.get_untracked();
                            if staged.len() >= 2 { return; }

                            match selected_origin.get_untracked() {
                                Some(origin) if origin == field_num => {
                                    selected_origin.set(None);
                                }
                                Some(origin) => {
                                    let valid = if seqs_k.is_empty() {
                                        true
                                    } else {
                                        valid_dests_for(&seqs_k, &staged, origin)
                                            .iter()
                                            .any(|&d| d == field_num)
                                    };
                                    if valid {
                                        staged_moves.update(|v| v.push((origin, field_num)));
                                        selected_origin.set(None);
                                    }
                                }
                                None => {
                                    if seqs_k.is_empty() {
                                        let val = displayed_value(board, &staged, is_white, field_num);
                                        if is_white && val > 0 || !is_white && val < 0 {
                                            selected_origin.set(Some(field_num));
                                        }
                                    } else {
                                        let origins = valid_origins_for(&seqs_k, &staged);
                                        if origins.iter().any(|&o| o == field_num) {
                                            let dests = valid_dests_for(&seqs_k, &staged, field_num);
                                            if !dests.is_empty() && dests.iter().all(|&d| d == 0) {
                                                // All destinations are exits: auto-stage
                                                staged_moves.update(|v| v.push((field_num, 0)));
                                            } else {
                                                selected_origin.set(Some(field_num));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    >
                        <span class="field-num">{field_num}</span>
                        {move || {
                            let moves = staged_moves.get();
                            let val = displayed_value(board, &moves, is_white, field_num);
                            let count = val.unsigned_abs();
                            // §6e — ripple on hit (battue) fields; must be inside the
                            // reactive closure so Leptos uses the same direct rendering
                            // path as .arriving (avoids node-move that resets animation).
                            let ripple = is_hit_field.then(|| {
                                let cls = if is_top_row { "hit-ripple hit-ripple-top" } else { "hit-ripple hit-ripple-bot" };
                                view! { <div class=cls></div> }.into_any()
                            });
                            let stack = (count > 0).then(|| {
                                let color = if val > 0 { "white" } else { "black" };
                                let display_n = (count as usize).min(4);
                                // outermost index: last for top rows, first for bottom rows.
                                let outer_idx = if is_top_row { display_n - 1 } else { 0 };
                                let chips: Vec<AnyView> = (0..display_n).map(|i| {
                                    let label = if i == outer_idx && count >= 5 {
                                        count.to_string()
                                    } else {
                                        String::new()
                                    };
                                    if i == outer_idx {
                                        if let Some((dx, dy)) = slide_delta {
                                            return view! {
                                                <div
                                                    class=format!("checker {color} arriving")
                                                    style=format!("--slide-dx:{dx:.1}px;--slide-dy:{dy:.1}px")
                                                >{label}</div>
                                            }.into_any();
                                        }
                                    }
                                    view! {
                                        <div class=format!("checker {color}")>{label}</div>
                                    }.into_any()
                                }).collect();
                                view! { <div class="checker-stack">{chips}</div> }.into_any()
                            });
                            (ripple, stack)
                        }}
                    </div>
                }
                .into_any()
            })
            .collect()
    };

    // ── Bar content: die in the center bar (die_idx 0 = top bar, 1 = bottom bar) ──
    let bar_content = move |die_idx: u8| -> AnyView {
        match bar_dice {
            None => view! { <div class="bar-die-slot"></div> }.into_any(),
            Some(dice_vals) => {
                let die_val = if die_idx == 0 {
                    dice_vals.0
                } else {
                    dice_vals.1
                };
                view! {
                    <div class="bar-die-slot">
                        {move || {
                            let staged = staged_moves.get();
                            let (u0, u1) = if bar_is_move {
                                bar_matched_dice_used(&staged, dice_vals)
                            } else if is_my_turn {
                                (true, true)
                            } else {
                                (false, false)
                            };
                            let used = if die_idx == 0 { u0 } else { u1 };
                            view! { <Die value=die_val used=used is_double=bar_is_double /> }
                        }}
                    </div>
                }
                .into_any()
            }
        }
    };

    let (tl, tr, bl, br) = if is_white {
        (&TOP_LEFT_W, &TOP_RIGHT_W, &BOT_LEFT_W, &BOT_RIGHT_W)
    } else {
        (&TOP_LEFT_B, &TOP_RIGHT_B, &BOT_LEFT_B, &BOT_RIGHT_B)
    };

    // Zone label pairs (top-left, top-right, bot-left, bot-right) per perspective.
    let (label_tl, label_tr, label_bl, label_br) = if is_white {
        ("", "jan de retour", "grand jan", "petit jan")
    } else {
        ("petit jan", "grand jan", "jan de retour", "")
    };

    view! {
        // board-wrapper keeps zone labels outside .board so the SVG overlay
        // inside .board stays correctly positioned (position:absolute top:0 left:0
        // is relative to .board, not the wrapper).
        <div class="board-wrapper">
            <div class="zone-labels-row">
                <div class="zone-label zone-label-quarter">{label_tl}</div>
                <div class="zone-label zone-label-bar"></div>
                <div class="zone-label zone-label-quarter">{label_tr}</div>
            </div>
            <div class="board">
                <div class="board-row top-row">
                    <div class="board-quarter">{fields_from(tl, true)}</div>
                    <div class="board-bar">{bar_content(0)}</div>
                    <div class="board-quarter">{fields_from(tr, true)}</div>
                </div>
                <div class="board-center-bar"></div>
                <div class="board-row bot-row">
                    <div class="board-quarter">{fields_from(bl, false)}</div>
                    <div class="board-bar">{bar_content(1)}</div>
                    <div class="board-quarter">{fields_from(br, false)}</div>
                </div>
                // SVG overlay: arrows for hovered jan moves
                <svg
                    width="824" height="388"
                    style="position:absolute;top:0;left:0;pointer-events:none;overflow:visible"
                >
                    {move || {
                        let Some(hm) = hovered_moves else { return vec![]; };
                        let pairs = hm.get();
                        if pairs.is_empty() { return vec![]; }
                        // Collect unique individual (from, to) moves; skip empty/exit.
                        let mut moves: Vec<(usize, usize)> = pairs.iter()
                            .flat_map(|(m1, m2)| [
                                (m1.get_from(), m1.get_to()),
                                (m2.get_from(), m2.get_to()),
                            ])
                            .filter(|&(f, t)| f != 0 && t != 0)
                            .collect();
                        moves.sort_unstable();
                        moves.dedup();
                        moves.into_iter()
                            .filter_map(|(from, to)| {
                                let p1 = field_center(from, is_white)?;
                                let p2 = field_center(to, is_white)?;
                                Some(arrow_svg(p1, p2))
                            })
                            .collect()
                    }}
                </svg>
            </div>
            <div class="zone-labels-row">
                <div class="zone-label zone-label-quarter">{label_bl}</div>
                <div class="zone-label zone-label-bar"></div>
                <div class="zone-label zone-label-quarter">{label_br}</div>
            </div>
        </div>
    }
}
