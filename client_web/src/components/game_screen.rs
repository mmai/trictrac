use futures::channel::mpsc::UnboundedSender;
use leptos::prelude::*;
use trictrac_store::{CheckerMove, Jan};

use crate::app::{GameUiState, NetCommand};
use crate::trictrac::types::{JanEntry, PlayerAction, SerStage, SerTurnStage};

use super::board::Board;
use super::die::Die;
use super::score_panel::ScorePanel;

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

fn jan_label(jan: &Jan) -> &'static str {
    match jan {
        Jan::FilledQuarter          => "Remplissage",
        Jan::TrueHitSmallJan        => "Battage à vrai (petit jan)",
        Jan::TrueHitBigJan          => "Battage à vrai (grand jan)",
        Jan::TrueHitOpponentCorner  => "Battage coin adverse",
        Jan::FirstPlayerToExit      => "Premier sorti",
        Jan::SixTables              => "Six tables",
        Jan::TwoTables              => "Deux tables",
        Jan::Mezeas                 => "Mezeas",
        Jan::FalseHitSmallJan       => "Battage à faux (petit jan)",
        Jan::FalseHitBigJan         => "Battage à faux (grand jan)",
        Jan::ContreTwoTables        => "Contre deux tables",
        Jan::ContreMezeas           => "Contre mezeas",
        Jan::HelplessMan            => "Dame impuissante",
    }
}

fn format_move_pair(m1: CheckerMove, m2: CheckerMove) -> String {
    let fmt = |m: CheckerMove| -> String {
        let (f, t) = (m.get_from(), m.get_to());
        if f == 0 && t == 0 { "—".to_string() }
        else if t == 0      { format!("{f}↑") }  // exit
        else                { format!("{f}→{t}") }
    };
    format!("{} + {}", fmt(m1), fmt(m2))
}

fn jan_row(idx: usize, entry: JanEntry, expanded: RwSignal<Option<usize>>) -> impl IntoView {
    let row_class = if entry.total >= 0 { "jan-row jan-positive" } else { "jan-row jan-negative" };
    let label = jan_label(&entry.jan);
    let double_tag = if entry.is_double { "double" } else { "simple" };
    let ways_tag = format!("×{}", entry.ways);
    let pts_str = if entry.total >= 0 { format!("+{}", entry.total) } else { format!("{}", entry.total) };

    let can_expand = entry.ways > 1;
    let moves = entry.moves.clone();

    view! {
        <div>
            <div
                class=row_class
                class:jan-expandable=can_expand
                on:click=move |_| {
                    if can_expand {
                        expanded.update(|s| {
                            *s = if *s == Some(idx) { None } else { Some(idx) };
                        });
                    }
                }
            >
                <span class="jan-label">{label}</span>
                <span class="jan-tag">{double_tag}</span>
                <span class="jan-tag">{ways_tag}</span>
                <span class="jan-pts">{pts_str}</span>
            </div>
            {can_expand.then(|| {
                let move_lines: Vec<_> = moves.iter()
                    .map(|&(m1, m2)| {
                        let text = format_move_pair(m1, m2);
                        view! { <div class="jan-move-line">{text}</div> }
                    })
                    .collect();
                view! {
                    <div class="jan-moves" class:hidden=move || expanded.get() != Some(idx)>
                        {move_lines}
                    </div>
                }
            })}
        </div>
    }
}

#[component]
pub fn GameScreen(state: GameUiState) -> impl IntoView {
    let vs = state.view_state.clone();
    let player_id = state.player_id;
    let is_my_turn = vs.active_mp_player == Some(player_id);
    let is_move_stage = is_my_turn
        && matches!(vs.turn_stage, SerTurnStage::Move | SerTurnStage::HoldOrGoChoice);

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
        SerStage::Ended   => "Game over".to_string(),
        SerStage::PreGame => "Waiting for opponent…".to_string(),
        SerStage::InGame  => match (is_my_turn, &vs.turn_stage) {
            (true, SerTurnStage::RollDice)      => "Your turn — roll the dice".to_string(),
            (true, SerTurnStage::HoldOrGoChoice) => "Hold or Go?".to_string(),
            (true, SerTurnStage::Move)           => "Select move 1 of 2".to_string(),
            (true, _)                            => "Your turn".to_string(),
            (false, _)                           => "Opponent's turn".to_string(),
        },
    };

    let dice = vs.dice;
    let show_dice = dice != (0, 0);

    // ── Button senders ─────────────────────────────────────────────────────────
    let cmd_tx_roll = cmd_tx.clone();
    let cmd_tx_go   = cmd_tx.clone();
    let cmd_tx_quit = cmd_tx.clone();
    let show_roll     = is_my_turn && vs.turn_stage == SerTurnStage::RollDice;
    let show_hold_go  = is_my_turn && vs.turn_stage == SerTurnStage::HoldOrGoChoice;

    view! {
        <div class="game-container">
            // ── Top bar ──────────────────────────────────────────────────────
            <div class="top-bar">
                <span>Room: {state.room_id}</span>
                <a class="quit-link" href="#" on:click=move |e| {
                    e.prevent_default();
                    cmd_tx_quit.unbounded_send(NetCommand::Disconnect).ok();
                }>"Quit"</a>
            </div>

            <ScorePanel scores=vs.scores.clone() player_id=player_id />

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

            // ── Jan panel ────────────────────────────────────────────────────
            {(!vs.dice_jans.is_empty()).then(|| {
                let expanded: RwSignal<Option<usize>> = RwSignal::new(None);
                let rows: Vec<_> = vs.dice_jans.iter().enumerate().map(|(i, entry)| {
                    jan_row(i, entry.clone(), expanded)
                }).collect();
                view! { <div class="jan-panel">{rows}</div> }
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
        </div>
    }
}
