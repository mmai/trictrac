use futures::channel::mpsc::UnboundedSender;
use leptos::prelude::*;
use trictrac_store::CheckerMove;

use crate::app::{GameUiState, NetCommand};
use crate::i18n::*;
use crate::trictrac::types::{JanEntry, PlayerAction, SerStage, SerTurnStage};

use super::board::Board;
use super::die::Die;
use super::score_panel::PlayerScorePanel;

#[allow(dead_code)]
/// Returns (d0_used, d1_used) by matching each staged move's distance to a die.
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
fn split_jans(dice_jans: &[JanEntry], viewer_is_active: bool) -> (Vec<JanEntry>, Vec<JanEntry>) {
    let mut mine = Vec::new();
    let mut theirs = Vec::new();
    for e in dice_jans {
        if viewer_is_active {
            if e.total >= 0 {
                mine.push(e.clone());
            } else {
                theirs.push(JanEntry {
                    total: -e.total,
                    points_per: -e.points_per,
                    ..e.clone()
                });
            }
        } else if e.total >= 0 {
            theirs.push(e.clone());
        } else {
            mine.push(JanEntry {
                total: -e.total,
                points_per: -e.points_per,
                ..e.clone()
            });
        }
    }
    (mine, theirs)
}

#[component]
pub fn GameScreen(state: GameUiState) -> impl IntoView {
    let i18n = use_i18n();

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

    let dice = vs.dice;
    let show_dice = dice != (0, 0);

    // ── Button senders ─────────────────────────────────────────────────────────
    let cmd_tx_roll = cmd_tx.clone();
    let cmd_tx_go = cmd_tx.clone();
    let cmd_tx_quit = cmd_tx.clone();
    let cmd_tx_end_quit = cmd_tx.clone();
    let cmd_tx_end_replay = cmd_tx.clone();
    let show_roll = is_my_turn && vs.turn_stage == SerTurnStage::RollDice;
    let show_hold_go = is_my_turn && vs.turn_stage == SerTurnStage::HoldOrGoChoice;

    // ── Jan split: viewer_jans / opponent_jans ─────────────────────────────────
    let (my_jans, opp_jans) = split_jans(&vs.dice_jans, is_my_turn && !show_roll);

    // ── Scores ─────────────────────────────────────────────────────────────────
    let my_score = vs.scores[player_id as usize].clone();
    let opp_score = vs.scores[1 - player_id as usize].clone();

    // ── Capture for closures ───────────────────────────────────────────────────
    let stage = vs.stage.clone();
    let turn_stage = vs.turn_stage.clone();
    let room_id = state.room_id.clone();
    let is_bot_game = state.is_bot_game;

    // ── Game-over info ─────────────────────────────────────────────────────────
    let stage_is_ended = stage == SerStage::Ended;
    let winner_is_me = my_score.holes >= 12;
    let opp_name_end = opp_score.name.clone();

    view! {
        <div class="game-container">
            // ── Top bar ──────────────────────────────────────────────────────
            <div class="top-bar">
                <span>{move || if is_bot_game {
                    t_string!(i18n, vs_bot_label).to_owned()
                } else {
                    t_string!(i18n, room_label, id = room_id.as_str())
                }}</span>
                <div class="lang-switcher">
                    <button
                        class:lang-active=move || i18n.get_locale() == Locale::en
                        on:click=move |_| i18n.set_locale(Locale::en)
                    >"EN"</button>
                    <button
                        class:lang-active=move || i18n.get_locale() == Locale::fr
                        on:click=move |_| i18n.set_locale(Locale::fr)
                    >"FR"</button>
                </div>
                <a class="quit-link" href="#" on:click=move |e| {
                    e.prevent_default();
                    cmd_tx_quit.unbounded_send(NetCommand::Disconnect).ok();
                }>{t!(i18n, quit)}</a>
            </div>

            // ── Opponent score (above board) ─────────────────────────────────
            <PlayerScorePanel score=opp_score jans=opp_jans is_you=false />

            // ── Board + side panel ───────────────────────────────────────────
            <div class="board-and-panel">
                <Board
                    view_state=vs
                    player_id=player_id
                    selected_origin=selected_origin
                    staged_moves=staged_moves
                />

                // ── Side panel ───────────────────────────────────────────────
                <div class="side-panel">
                    // Status message
                    <div class="status-bar">
                        <span>{move || {
                            let n = staged_moves.get().len();
                            if is_move_stage {
                                t_string!(i18n, select_move, n = n + 1)
                            } else {
                                String::from(match (&stage, is_my_turn, &turn_stage) {
                                    (SerStage::Ended, _, _) => t_string!(i18n, game_over),
                                    (SerStage::PreGame, _, _) => t_string!(i18n, waiting_for_opponent),
                                    (SerStage::InGame, true, SerTurnStage::RollDice) => t_string!(i18n, your_turn_roll),
                                    (SerStage::InGame, true, SerTurnStage::HoldOrGoChoice) => t_string!(i18n, hold_or_go),
                                    (SerStage::InGame, true, _) => t_string!(i18n, your_turn),
                                    (SerStage::InGame, false, _) => t_string!(i18n, opponent_turn),
                                })
                            }
                        }}</span>
                    </div>

                    // Dice (always shown when rolled, used state depends on whose turn)
                    {show_dice.then(|| view! {
                        <div class="dice-bar">
                            {move || {
                                let (d0, d1) = if is_move_stage {
                                    matched_dice_used(&staged_moves.get(), dice)
                                } else {
                                    (true, true)
                                };
                                view! {
                                    <Die value=dice.0 used=d0 />
                                    <Die value=dice.1 used=d1 />
                                }
                            }}
                        </div>
                    })}

                    // Action buttons
                    <div class="action-buttons">
                        {show_roll.then(|| view! {
                            <button class="btn btn-primary" on:click=move |_| {
                                cmd_tx_roll.unbounded_send(NetCommand::Action(PlayerAction::Roll)).ok();
                            }>{t!(i18n, roll_dice)}</button>
                        })}
                        {show_hold_go.then(|| view! {
                            <button class="btn btn-primary" on:click=move |_| {
                                cmd_tx_go.unbounded_send(NetCommand::Action(PlayerAction::Go)).ok();
                            }>{t!(i18n, go)}</button>
                        })}
                        {is_move_stage.then(|| view! {
                            <button
                                class="btn btn-secondary"
                                disabled=move || 2 <= staged_moves.get().len()
                                on:click=move |_| {
                                    selected_origin.set(None);
                                    staged_moves.update(|v| v.push((0, 0)));
                                }
                            >{t!(i18n, empty_move)}</button>
                        })}
                    </div>
                </div>
            </div>

            // ── Player score (below board) ────────────────────────────────────
            <PlayerScorePanel score=my_score jans=my_jans is_you=true />

            // ── Game-over overlay ─────────────────────────────────────────────
            {stage_is_ended.then(|| {
                let winner_text = if winner_is_me {
                    t_string!(i18n, you_win).to_owned()
                } else {
                    t_string!(i18n, opp_wins, name = opp_name_end.as_str())
                };
                view! {
                    <div class="game-over-overlay">
                        <div class="game-over-box">
                            <h2>{t!(i18n, game_over)}</h2>
                            <p class="game-over-winner">{winner_text}</p>
                            <div class="game-over-actions">
                                <button class="btn btn-secondary" on:click=move |_| {
                                    cmd_tx_end_quit.unbounded_send(NetCommand::Disconnect).ok();
                                }>{t!(i18n, quit)}</button>
                                {is_bot_game.then(|| view! {
                                    <button class="btn btn-primary" on:click=move |_| {
                                        cmd_tx_end_replay.unbounded_send(NetCommand::PlayVsBot).ok();
                                    }>{t!(i18n, play_again)}</button>
                                })}
                            </div>
                        </div>
                    </div>
                }
            })}
        </div>
    }
}
