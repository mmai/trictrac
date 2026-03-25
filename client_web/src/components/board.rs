use futures::channel::mpsc::UnboundedSender;
use leptos::prelude::*;

use crate::app::NetCommand;
use crate::trictrac::types::{PlayerAction, SerTurnStage, ViewState};

/// Field numbers in visual display order (left-to-right for each quarter).
const TOP_LEFT:  [u8; 6] = [13, 14, 15, 16, 17, 18];
const TOP_RIGHT: [u8; 6] = [19, 20, 21, 22, 23, 24];
const BOT_LEFT:  [u8; 6] = [12, 11, 10,  9,  8,  7];
const BOT_RIGHT: [u8; 6] = [ 6,  5,  4,  3,  2,  1];

#[component]
pub fn Board(view_state: ViewState, player_id: u16) -> impl IntoView {
    let selected: RwSignal<Option<u8>> = RwSignal::new(None);
    let cmd_tx = use_context::<UnboundedSender<NetCommand>>()
        .expect("UnboundedSender<NetCommand> not found in context");

    let board = view_state.board;
    let is_move_stage = view_state.active_mp_player == Some(player_id)
        && view_state.turn_stage == SerTurnStage::Move;

    // Build a Vec<AnyView> for a slice of field numbers.
    // `fields_from` borrows `board`, `cmd_tx` and copies `selected`, `is_move_stage`, `player_id`.
    let fields_from = |nums: &[u8]| -> Vec<AnyView> {
        nums.iter().map(|&field_num| {
            let value: i8 = board[(field_num - 1) as usize];
            let count = value.unsigned_abs();
            let checker_color = if value > 0 { "white" } else { "black" };
            let is_my_checker = if player_id == 0 { value > 0 } else { value < 0 };
            let cmd = cmd_tx.clone();

            view! {
                <div
                    class=move || {
                        let sel = selected.get();
                        let mut cls = "field".to_string();
                        let clickable = is_move_stage
                            && (sel.is_some() || is_my_checker);
                        if clickable { cls.push_str(" clickable"); }
                        if sel == Some(field_num) { cls.push_str(" selected"); }
                        if is_move_stage && sel.is_some() && sel != Some(field_num) {
                            cls.push_str(" dest");
                        }
                        cls
                    }
                    on:click=move |_| {
                        if !is_move_stage { return; }
                        match selected.get() {
                            Some(origin) if origin == field_num => selected.set(None),
                            Some(origin) => {
                                cmd.unbounded_send(NetCommand::Action(
                                    PlayerAction::Move { from: origin, to: field_num },
                                )).ok();
                                selected.set(None);
                            }
                            None if is_my_checker => selected.set(Some(field_num)),
                            None => {}
                        }
                    }
                >
                    <span class="field-num">{field_num}</span>
                    {(count > 0).then(|| view! {
                        <span class=format!("checkers {checker_color}")>{count}</span>
                    })}
                </div>
            }
            .into_any()
        })
        .collect()
    };

    let top_left  = fields_from(&TOP_LEFT);
    let top_right = fields_from(&TOP_RIGHT);
    let bot_left  = fields_from(&BOT_LEFT);
    let bot_right = fields_from(&BOT_RIGHT);

    view! {
        <div class="board">
            <div class="board-row top-row">
                <div class="board-quarter">{top_left}</div>
                <div class="board-bar"></div>
                <div class="board-quarter">{top_right}</div>
            </div>
            <div class="board-center-bar"></div>
            <div class="board-row bot-row">
                <div class="board-quarter">{bot_left}</div>
                <div class="board-bar"></div>
                <div class="board-quarter">{bot_right}</div>
            </div>
        </div>
    }
}
