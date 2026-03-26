use leptos::prelude::*;

use crate::trictrac::types::{SerTurnStage, ViewState};

/// Field numbers in visual display order (left-to-right for each quarter).
const TOP_LEFT:  [u8; 6] = [13, 14, 15, 16, 17, 18];
const TOP_RIGHT: [u8; 6] = [19, 20, 21, 22, 23, 24];
const BOT_LEFT:  [u8; 6] = [12, 11, 10,  9,  8,  7];
const BOT_RIGHT: [u8; 6] = [ 6,  5,  4,  3,  2,  1];

/// Returns the displayed board value for `field_num` after applying `staged_moves`.
/// `is_white`: true when the local player's checkers are positive (host = white).
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

    view! {
        <div class="board">
            <div class="board-row top-row">
                <div class="board-quarter">{fields_from(&TOP_LEFT)}</div>
                <div class="board-bar"></div>
                <div class="board-quarter">{fields_from(&TOP_RIGHT)}</div>
            </div>
            <div class="board-center-bar"></div>
            <div class="board-row bot-row">
                <div class="board-quarter">{fields_from(&BOT_LEFT)}</div>
                <div class="board-bar"></div>
                <div class="board-quarter">{fields_from(&BOT_RIGHT)}</div>
            </div>
        </div>
    }
}
