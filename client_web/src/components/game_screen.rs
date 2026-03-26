use futures::channel::mpsc::UnboundedSender;
use leptos::prelude::*;
use trictrac_store::{CheckerMove, Jan};

use crate::app::{GameUiState, NetCommand};
use crate::trictrac::types::{PlayerAction, SerStage, SerTurnStage};

use super::board::Board;
use super::score_panel::ScorePanel;

#[allow(dead_code)]
/// Returns (d0_used, d1_used) by matching each staged move's distance to a die.
/// Falls back to position order for exit moves (distance doesn't match any die).
fn matched_dice_used(staged: &[(u8, u8)], dice: (u8, u8)) -> (bool, bool) {
    let mut d0 = false;
    let mut d1 = false;
    for &(from, to) in staged {
        let dist = to.saturating_sub(from); // 0 for empty/same-field moves
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

fn jan_label(jan: &Jan) -> &'static str {
    match jan {
        Jan::FilledQuarter => "Rempli",
        Jan::TrueHitSmallJan => "Atteinte vraie (petit jan)",
        Jan::TrueHitBigJan => "Atteinte vraie (grand jan)",
        Jan::TrueHitOpponentCorner => "Atteinte coin adverse",
        Jan::FirstPlayerToExit => "Premier sorti",
        Jan::SixTables => "Six tables",
        Jan::TwoTables => "Deux tables",
        Jan::Mezeas => "Mezeas",
        Jan::FalseHitSmallJan => "Faux (petit jan)",
        Jan::FalseHitBigJan => "Faux (grand jan)",
        Jan::ContreTwoTables => "Contre deux tables",
        Jan::ContreMezeas => "Contre mezeas",
        Jan::HelplessMan => "Homme en route",
    }
}

#[component]
pub fn GameScreen(state: GameUiState) -> impl IntoView {
    let vs = state.view_state.clone();
    let player_id = state.player_id;
    let is_my_turn = vs.active_mp_player == Some(player_id);
    let is_move_stage = is_my_turn
        && matches!(
            vs.turn_stage,
            SerTurnStage::Move | SerTurnStage::HoldOrGoChoice
        );

    // ── Staged move state ──────────────────────────────────────────────────────
    let selected_origin: RwSignal<Option<u8>> = RwSignal::new(None);
    let staged_moves: RwSignal<Vec<(u8, u8)>> = RwSignal::new(Vec::new());

    // When both move slots are filled, send the action to the backend.
    let cmd_tx = use_context::<UnboundedSender<NetCommand>>()
        .expect("UnboundedSender<NetCommand> not found in context");
    let cmd_tx_effect = cmd_tx.clone();
    Effect::new(move |_| {
        let moves = staged_moves.get();
        if moves.len() == 2 {
            let to_cm = |&(from, to): &(u8, u8)| {
                CheckerMove::new(from as usize, to as usize).unwrap_or_default()
            };
            cmd_tx_effect
                .unbounded_send(NetCommand::Action(PlayerAction::Move(
                    to_cm(&moves[0]),
                    to_cm(&moves[1]),
                )))
                .ok();
            staged_moves.set(vec![]);
            selected_origin.set(None);
        }
    });

    // ── Status text ────────────────────────────────────────────────────────────
    let status = match &vs.stage {
        SerStage::Ended => "Game over".to_string(),
        SerStage::PreGame => "Waiting for opponent…".to_string(),
        SerStage::InGame => match (is_my_turn, &vs.turn_stage) {
            (true, SerTurnStage::RollDice) => "Your turn — roll the dice".to_string(),
            (true, SerTurnStage::HoldOrGoChoice) => "Hold or Go?".to_string(),
            (true, SerTurnStage::Move) => "Select move 1 of 2".to_string(),
            (true, _) => "Your turn".to_string(),
            (false, _) => "Opponent's turn".to_string(),
        },
    };

    let dice = vs.dice;

    // ── Action bar buttons ─────────────────────────────────────────────────────
    let cmd_tx2 = cmd_tx.clone();
    let cmd_tx_quit = cmd_tx.clone();
    let show_roll = is_my_turn && vs.turn_stage == SerTurnStage::RollDice;
    let show_hold_go = is_my_turn && vs.turn_stage == SerTurnStage::HoldOrGoChoice;

    view! {
        <div class="game-container">
            <div class="top-bar">
                <span>{state.room_id}</span>
                <a class="quit-link" href="#" on:click=move |e| {
                    e.prevent_default();
                    cmd_tx_quit.unbounded_send(NetCommand::Disconnect).ok();
                }>"Quit"</a>
            </div>
            <ScorePanel scores=vs.scores.clone() player_id=player_id />
            <div class="status-bar">
                <span>{move || {
                    if is_move_stage {
                        let n = staged_moves.get().len();
                        format!("Select move {} of 2", n + 1)
                    } else {
                        status.clone()
                    }
                }}</span>
                {(dice != (0, 0)).then(|| view! {
                    <span class="dice-label">"Dice: "</span>
                    <span class={move || {
                        let (d0, _) = if is_move_stage {
                            matched_dice_used(&staged_moves.get(), dice)
                        } else {
                            (!is_my_turn || !is_move_stage, !is_my_turn || !is_move_stage)
                        };
                        if d0 { "dice dice-used" } else { "dice" }
                    }}>{dice.0}</span>
                    <span class="dice-sep">" & "</span>
                    <span class={move || {
                        let (_, d1) = if is_move_stage {
                            matched_dice_used(&staged_moves.get(), dice)
                        } else {
                            (!is_my_turn || !is_move_stage, !is_my_turn || !is_move_stage)
                        };
                        if d1 { "dice dice-used" } else { "dice" }
                    }}>{dice.1}</span>
                })}
            </div>
            <div class="action-bar">
                {show_roll.then(|| view! {
                    <button class="btn btn-primary" on:click=move |_| {
                        cmd_tx.unbounded_send(NetCommand::Action(PlayerAction::Roll)).ok();
                    }>"Roll dice"</button>
                })}
                {show_hold_go.then(|| view! {
                    <button class="btn btn-primary" on:click=move |_| {
                        cmd_tx2.unbounded_send(NetCommand::Action(PlayerAction::Go)).ok();
                    }>"Go"</button>
                })}
                {is_move_stage.then(|| view! {
                    <button
                        class="btn btn-secondary"
                        disabled=move || 2 <= staged_moves.get().len()
                        on:click=move |_| {
                            selected_origin.set(None);
                            staged_moves.update(|v| v.push((0, 0)));
                        }
                    >"Empty move"</button>
                })}
            </div>
            {(!vs.dice_jans.is_empty()).then(|| {
                let rows: Vec<_> = vs.dice_jans.iter().map(|(jan, pts)| {
                    let label = jan_label(jan);
                    let pts_str = if *pts >= 0 {
                        format!("+{}", pts)
                    } else {
                        format!("{}", pts)
                    };
                    let row_class = if *pts >= 0 { "jan-row jan-positive" } else { "jan-row jan-negative" };
                    view! {
                        <div class=row_class>
                            <span class="jan-label">{label}</span>
                            <span class="jan-pts">{pts_str}</span>
                        </div>
                    }
                }).collect();
                view! { <div class="jan-panel">{rows}</div> }
            })}
            <Board
                view_state=vs
                player_id=player_id
                selected_origin=selected_origin
                staged_moves=staged_moves
            />
        </div>
    }
}
