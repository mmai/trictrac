use leptos::prelude::*;
use trictrac_store::CheckerMove;

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
        0 => seqs.iter()
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

/// Valid destinations for a selected origin given already-staged moves.
/// May include 0 (exit); callers handle that case.
fn valid_dests_for(seqs: &[(CheckerMove, CheckerMove)], staged: &[(u8, u8)], origin: u8) -> Vec<u8> {
    let mut v: Vec<u8> = match staged.len() {
        0 => seqs.iter()
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
) -> impl IntoView {
    let board = view_state.board;
    let is_move_stage = view_state.active_mp_player == Some(player_id)
        && matches!(
            view_state.turn_stage,
            SerTurnStage::Move | SerTurnStage::HoldOrGoChoice
        );
    let is_white = player_id == 0;

    // `valid_sequences` is cloned per field (the Vec is small; Send-safe unlike Rc).
    let fields_from = |nums: &[u8], is_top_row: bool| -> Vec<AnyView> {
        nums.iter()
            .map(|&field_num| {
                // Each reactive closure gets its own owned clone — Vec<(CheckerMove,CheckerMove)>
                // is Send, which Leptos requires for reactive attribute functions.
                let seqs_c = valid_sequences.clone();
                let seqs_k = valid_sequences.clone();
                view! {
                    <div
                        class=move || {
                            let staged = staged_moves.get();
                            let val = displayed_value(board, &staged, is_white, field_num);
                            let is_mine = if is_white { val > 0 } else { val < 0 };
                            let can_stage = is_move_stage && staged.len() < 2;
                            let sel = selected_origin.get();

                            let mut cls = "field".to_string();

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
                            (count > 0).then(|| {
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
                                    view! {
                                        <div class=format!("checker {color}")>{label}</div>
                                    }.into_any()
                                }).collect();
                                view! { <div class="checker-stack">{chips}</div> }
                            })
                        }}
                    </div>
                }
                .into_any()
            })
            .collect()
    };

    let (tl, tr, bl, br) = if is_white {
        (&TOP_LEFT_W, &TOP_RIGHT_W, &BOT_LEFT_W, &BOT_RIGHT_W)
    } else {
        (&TOP_LEFT_B, &TOP_RIGHT_B, &BOT_LEFT_B, &BOT_RIGHT_B)
    };

    view! {
        <div class="board">
            <div class="board-row top-row">
                <div class="board-quarter">{fields_from(tl, true)}</div>
                <div class="board-bar"></div>
                <div class="board-quarter">{fields_from(tr, true)}</div>
            </div>
            <div class="board-center-bar"></div>
            <div class="board-row bot-row">
                <div class="board-quarter">{fields_from(bl, false)}</div>
                <div class="board-bar"></div>
                <div class="board-quarter">{fields_from(br, false)}</div>
            </div>
        </div>
    }
}
