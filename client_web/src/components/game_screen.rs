use futures::channel::mpsc::UnboundedSender;
use leptos::prelude::*;
use trictrac_store::CheckerMove;

use crate::app::{GameUiState, NetCommand};
use crate::trictrac::types::{JanEntry, PlayerAction, SerStage, SerTurnStage};

use super::board::Board;
use super::die::Die;
use super::score_panel::PlayerScorePanel;

#[allow(dead_code)]
/// Returns (d0_used, d1_used) by matching each staged move's distance to a die.
/// Falls back to position order for exit moves (distance doesn't match any die).
fn matched_dice_used(staged: &[(u8, u8)], dice: (u8, u8)) -> (bool, bool) {
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

/// Split `dice_jans` into (viewer_jans, opponent_jans).
/// Entries where the active player scores (total >= 0) go to the active player.
/// Entries where the active player loses (total < 0) go to the opponent, with signs flipped.
fn split_jans(
    dice_jans: &[JanEntry],
    viewer_is_active: bool,
) -> (Vec<JanEntry>, Vec<JanEntry>) {
    let mut mine = Vec::new();
    let mut theirs = Vec::new();
    for e in dice_jans {
        if viewer_is_active {
            if e.total >= 0 {
                mine.push(e.clone());
            } else {
                theirs.push(JanEntry { total: -e.total, points_per: -e.points_per, ..e.clone() });
            }
        } else {
            if e.total >= 0 {
                theirs.push(e.clone());
            } else {
                mine.push(JanEntry { total: -e.total, points_per: -e.points_per, ..e.clone() });
            }
        }
    }
    (mine, theirs)
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
    let show_dice = dice != (0, 0);

    // ── Button senders ─────────────────────────────────────────────────────────
    let cmd_tx_roll = cmd_tx.clone();
    let cmd_tx_go = cmd_tx.clone();
    let cmd_tx_quit = cmd_tx.clone();
    let show_roll = is_my_turn && vs.turn_stage == SerTurnStage::RollDice;
    let show_hold_go = is_my_turn && vs.turn_stage == SerTurnStage::HoldOrGoChoice;

    // ── Jan split: viewer_jans / opponent_jans ─────────────────────────────────
    let (my_jans, opp_jans) = split_jans(&vs.dice_jans, is_my_turn);

    // ── Scores: index = mp_player_id ──────────────────────────────────────────
    let my_score = vs.scores[player_id as usize].clone();
    let opp_score = vs.scores[1 - player_id as usize].clone();

    view! {
        <div class="game-container">
            // ── Top bar ──────────────────────────────────────────────────────
            <div class="top-bar">
                <span>Room: {state.room_id}</span>
                <a class="quit-link" href="#" on:click=move |e| {
                    e.prevent_default();
                    cmd_tx_quit.unbounded_send(NetCommand::Disconnect).ok();
                }>Quit</a>
            </div>

            // ── Opponent score (above board) ─────────────────────────────────
            <PlayerScorePanel score=opp_score jans=opp_jans is_you=false />

            // ── Status ───────────────────────────────────────────────────────
            <div class="status-bar">
                <span>{move || {
                    if is_move_stage {
                        let n = staged_moves.get().len();
                        format!("Select move {} of 2", n + 1)
                    } else {
                        status.clone()
                    }
                }}</span>
            </div>

            // ── Opponent dice (top) ──────────────────────────────────────────
            {(!is_my_turn && show_dice).then(|| view! {
                <div class="dice-bar dice-bar-opponent">
                    <Die value=dice.0 used=true />
                    <Die value=dice.1 used=true />
                </div>
            })}

            // ── Board ────────────────────────────────────────────────────────
            <Board
                view_state=vs
                player_id=player_id
                selected_origin=selected_origin
                staged_moves=staged_moves
            />

            // ── Player action bar (bottom) ───────────────────────────────────
            {is_my_turn.then(|| view! {
                <div class="dice-bar dice-bar-player">
                    // Dice (reactive greying as moves are staged)
                    {move || {
                        let (d0, d1) = if is_move_stage {
                            matched_dice_used(&staged_moves.get(), dice)
                        } else {
                            (false, false)
                        };
                        view! {
                            <Die value=dice.0 used=d0 />
                            <Die value=dice.1 used=d1 />
                        }
                    }}
                    // Roll button (shown next to the dice during RollDice stage)
                    {show_roll.then(|| view! {
                        <button class="btn btn-primary" on:click=move |_| {
                            cmd_tx_roll.unbounded_send(NetCommand::Action(PlayerAction::Roll)).ok();
                        }>"Roll dice"</button>
                    })}
                    // Go button (HoldOrGoChoice)
                    {show_hold_go.then(|| view! {
                        <button class="btn btn-primary" on:click=move |_| {
                            cmd_tx_go.unbounded_send(NetCommand::Action(PlayerAction::Go)).ok();
                        }>"Go"</button>
                    })}
                    // Empty move button
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
            })}

            // ── Player score (below board) ────────────────────────────────────
            <PlayerScorePanel score=my_score jans=my_jans is_you=true />
        </div>
    }
}
