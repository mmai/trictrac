use futures::channel::mpsc::UnboundedSender;
use leptos::prelude::*;

use crate::app::{GameUiState, NetCommand};
use crate::trictrac::types::{PlayerAction, SerStage, SerTurnStage};

use super::board::Board;
use super::score_panel::ScorePanel;

#[component]
pub fn GameScreen(state: GameUiState) -> impl IntoView {
    let vs = state.view_state.clone();
    let player_id = state.player_id;
    let is_my_turn = vs.active_mp_player == Some(player_id);

    let status = match &vs.stage {
        SerStage::Ended => "Game over".to_string(),
        SerStage::PreGame => "Waiting for opponent…".to_string(),
        SerStage::InGame => match (is_my_turn, &vs.turn_stage) {
            (true, SerTurnStage::RollDice) => "Your turn — roll the dice".to_string(),
            (true, SerTurnStage::HoldOrGoChoice) => "Hold or Go?".to_string(),
            (true, SerTurnStage::Move) => "Your turn — move a checker".to_string(),
            (true, _) => "Your turn".to_string(),
            (false, _) => "Opponent's turn".to_string(),
        },
    };

    let dice_text = if vs.dice != (0, 0) {
        format!("Dice: {} & {}", vs.dice.0, vs.dice.1)
    } else {
        String::new()
    };

    let cmd_tx = use_context::<UnboundedSender<NetCommand>>()
        .expect("UnboundedSender<NetCommand> not found in context");
    let cmd_tx2 = cmd_tx.clone();

    let show_roll = is_my_turn && vs.turn_stage == SerTurnStage::RollDice;
    let show_hold_go = is_my_turn && vs.turn_stage == SerTurnStage::HoldOrGoChoice;

    view! {
        <div class="game-container">
            <ScorePanel scores=vs.scores.clone() player_id=player_id />
            <div class="status-bar">
                <span>{status}</span>
                {(!dice_text.is_empty()).then(|| view! { <span class="dice">{dice_text}</span> })}
            </div>
            <div class="action-bar">
                {show_roll.then(|| view! {
                    <button class="btn btn-primary" on:click=move |_| {
                        cmd_tx.unbounded_send(NetCommand::Action(PlayerAction::Roll)).ok();
                    }>"Roll dice"</button>
                })}
                {show_hold_go.then(|| view! {
                    <button class="btn btn-secondary" on:click=move |_| {
                        cmd_tx2.unbounded_send(NetCommand::Action(PlayerAction::Mark)).ok();
                    }>"Hold"</button>
                })}
                {show_hold_go.then(|| {
                    let cmd_tx3 = use_context::<UnboundedSender<NetCommand>>()
                        .expect("UnboundedSender<NetCommand> not found in context");
                    view! {
                        <button class="btn btn-primary" on:click=move |_| {
                            cmd_tx3.unbounded_send(NetCommand::Action(PlayerAction::Go)).ok();
                        }>"Go"</button>
                    }
                })}
            </div>
            <Board view_state=vs player_id=player_id />
        </div>
    }
}
