use leptos::prelude::*;

use crate::trictrac::types::{SerTurnStage, ViewState};

/// Field numbers in visual display order (left-to-right for each quarter), white's perspective.
const TOP_LEFT_W:  [u8; 6] = [13, 14, 15, 16, 17, 18];
const TOP_RIGHT_W: [u8; 6] = [19, 20, 21, 22, 23, 24];
const BOT_LEFT_W:  [u8; 6] = [12, 11, 10,  9,  8,  7];
const BOT_RIGHT_W: [u8; 6] = [ 6,  5,  4,  3,  2,  1];

/// 180° rotation of white's layout: black's pieces (field 24) appear at the bottom.
const TOP_LEFT_B:  [u8; 6] = [ 1,  2,  3,  4,  5,  6];
const TOP_RIGHT_B: [u8; 6] = [ 7,  8,  9, 10, 11, 12];
const BOT_LEFT_B:  [u8; 6] = [24, 23, 22, 21, 20, 19];
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
        if from == field_num { val -= delta; }
        if to == field_num  { val += delta; }
    }
    val
}

#[component]
pub fn Board(
    view_state: ViewState,
    player_id: u16,
    /// Pending origin selection (first click of a move pair).
    selected_origin: RwSignal<Option<u8>>,
    /// Moves staged so far this turn (max 2). Each entry is (from, to), 0 = empty move.
    staged_moves: RwSignal<Vec<(u8, u8)>>,
) -> impl IntoView {
    let board = view_state.board;
    let is_move_stage = view_state.active_mp_player == Some(player_id)
        && matches!(view_state.turn_stage, SerTurnStage::Move | SerTurnStage::HoldOrGoChoice);
    let is_white = player_id == 0;

    let fields_from = |nums: &[u8]| -> Vec<AnyView> {
        nums.iter().map(|&field_num| {
            view! {
                <div
                    class=move || {
                        let moves = staged_moves.get();
                        let val = displayed_value(board, &moves, is_white, field_num);
                        let is_mine = if is_white { val > 0 } else { val < 0 };
                        let can_stage = is_move_stage && moves.len() < 2;
                        let sel = selected_origin.get();

                        let mut cls = "field".to_string();
                        if can_stage && (sel.is_some() || is_mine) {
                            cls.push_str(" clickable");
                        }
                        if sel == Some(field_num) { cls.push_str(" selected"); }
                        if can_stage && sel.is_some() && sel != Some(field_num) {
                            cls.push_str(" dest");
                        }
                        cls
                    }
                    on:click=move |_| {
                        if !is_move_stage { return; }
                        if staged_moves.get_untracked().len() >= 2 { return; }

                        let moves = staged_moves.get_untracked();
                        let val = displayed_value(board, &moves, is_white, field_num);
                        let is_mine = if is_white { val > 0 } else { val < 0 };

                        match selected_origin.get_untracked() {
                            Some(origin) if origin == field_num => {
                                selected_origin.set(None);
                            }
                            Some(origin) => {
                                staged_moves.update(|v| v.push((origin, field_num)));
                                selected_origin.set(None);
                            }
                            None if is_mine => selected_origin.set(Some(field_num)),
                            None => {}
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
                            view! { <span class=format!("checkers {color}")>{count}</span> }
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
                <div class="board-quarter">{fields_from(tl)}</div>
                <div class="board-bar"></div>
                <div class="board-quarter">{fields_from(tr)}</div>
            </div>
            <div class="board-center-bar"></div>
            <div class="board-row bot-row">
                <div class="board-quarter">{fields_from(bl)}</div>
                <div class="board-bar"></div>
                <div class="board-quarter">{fields_from(br)}</div>
            </div>
        </div>
    }
}
