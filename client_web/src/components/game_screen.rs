use std::collections::VecDeque;

use futures::channel::mpsc::UnboundedSender;
use leptos::prelude::*;
use trictrac_store::{Board as StoreBoard, CheckerMove, Color, Dice as StoreDice, MoveRules};

use crate::app::{GameUiState, NetCommand, PauseReason};
use crate::i18n::*;
use crate::trictrac::types::{PlayerAction, SerStage, SerTurnStage};

use super::board::Board;
use super::score_panel::PlayerScorePanel;
use super::scoring::ScoringPanel;

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
    let waiting_for_confirm = state.waiting_for_confirm;
    let pause_reason = state.pause_reason.clone();

    // ── Hovered jan moves (shown as arrows on the board) ──────────────────────
    let hovered_jan_moves: RwSignal<Vec<(CheckerMove, CheckerMove)>> = RwSignal::new(vec![]);
    provide_context(hovered_jan_moves);

    // ── Staged move state ──────────────────────────────────────────────────────
    let selected_origin: RwSignal<Option<u8>> = RwSignal::new(None);
    let staged_moves: RwSignal<Vec<(u8, u8)>> = RwSignal::new(Vec::new());

    let cmd_tx = use_context::<UnboundedSender<NetCommand>>()
        .expect("UnboundedSender<NetCommand> not found in context");
    let pending = use_context::<RwSignal<VecDeque<GameUiState>>>()
        .expect("pending not found in context");
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

    // ── Auto-roll effect ─────────────────────────────────────────────────────
    // GameScreen is fully re-mounted on every ViewState update (state is a
    // plain prop, not a signal), so this effect fires exactly once per
    // RollDice phase entry and will not double-send.
    // Guard: suppressed while waiting_for_confirm — the AfterOpponentMove
    // buffered state shows the human's RollDice turn but the auto-roll must
    // wait until the buffer is drained and the live screen state is shown.
    let show_roll = is_my_turn && vs.turn_stage == SerTurnStage::RollDice;
    if show_roll && !waiting_for_confirm {
        let cmd_tx_auto = cmd_tx.clone();
        Effect::new(move |_| {
            cmd_tx_auto.unbounded_send(NetCommand::Action(PlayerAction::Roll)).ok();
        });
    }

    let dice = vs.dice;
    let show_dice = dice != (0, 0);

    // ── Button senders ─────────────────────────────────────────────────────────
    let cmd_tx_go = cmd_tx.clone();
    let cmd_tx_quit = cmd_tx.clone();
    let cmd_tx_end_quit = cmd_tx.clone();
    let cmd_tx_end_replay = cmd_tx.clone();
    // Only show the fallback Go button when there is no ScoringPanel showing it.
    let show_hold_go = is_my_turn
        && vs.turn_stage == SerTurnStage::HoldOrGoChoice
        && state.my_scored_event.is_none();

    // ── Valid move sequences for this turn ─────────────────────────────────────
    // Computed once per ViewState snapshot; used by Board (highlighting) and the
    // empty-move button (visibility).
    let valid_sequences: Vec<(CheckerMove, CheckerMove)> = if is_move_stage && dice != (0, 0) {
        let mut store_board = StoreBoard::new();
        store_board.set_positions(&Color::White, vs.board);
        let store_dice = StoreDice { values: dice };
        let color = if player_id == 0 { Color::White } else { Color::Black };
        let rules = MoveRules::new(&color, &store_board, store_dice);
        let raw = rules.get_possible_moves_sequences(true, vec![]);
        if player_id == 0 {
            raw
        } else {
            raw.into_iter().map(|(m1, m2)| (m1.mirror(), m2.mirror())).collect()
        }
    } else {
        vec![]
    };
    // Clone for the empty-move button reactive closure (Board consumes the original).
    let valid_seqs_empty = valid_sequences.clone();

    // ── Scores ─────────────────────────────────────────────────────────────────
    let my_score = vs.scores[player_id as usize].clone();
    let opp_score = vs.scores[1 - player_id as usize].clone();

    // ── Scoring notifications ──────────────────────────────────────────────────
    let my_scored_event = state.my_scored_event.clone();
    let opp_scored_event = state.opp_scored_event.clone();
    let hole_toast_info = my_scored_event.as_ref()
        .filter(|e| e.holes_gained > 0)
        .map(|e| (e.holes_total, e.bredouille));

    let is_double_dice = dice.0 == dice.1 && dice.0 != 0;

    let last_moves = state.last_moves;

    // ── Capture for closures ───────────────────────────────────────────────────
    let stage = vs.stage.clone();
    let turn_stage = vs.turn_stage.clone();
    let turn_stage_for_panel = turn_stage.clone();
    let turn_stage_for_sub = turn_stage.clone();
    let room_id = state.room_id.clone();
    let is_bot_game = state.is_bot_game;

    // ── Game-over info ─────────────────────────────────────────────────────────
    let stage_is_ended = stage == SerStage::Ended;
    let winner_is_me = my_score.holes >= 12;
    let my_name_end = my_score.name.clone();
    let my_holes_end = my_score.holes;
    let opp_name_end = opp_score.name.clone();
    let opp_holes_end = opp_score.holes;

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
            <PlayerScorePanel score=opp_score is_you=false />

            // ── Status bar — full width, above board (§10b) ──────────────────
            <div class="game-status">
                {move || {
                    if let Some(ref reason) = pause_reason {
                        return String::from(match reason {
                            PauseReason::AfterOpponentRoll => t_string!(i18n, after_opponent_roll),
                            PauseReason::AfterOpponentGo   => t_string!(i18n, after_opponent_go),
                            PauseReason::AfterOpponentMove => t_string!(i18n, after_opponent_move),
                        });
                    }
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
                }}
            </div>

            // ── Contextual sub-prompt (§8a) ──────────────────────────────────
            {move || {
                let hint: String = if waiting_for_confirm {
                    t_string!(i18n, hint_continue).to_owned()
                } else if is_move_stage {
                    t_string!(i18n, hint_move).to_owned()
                } else if is_my_turn && turn_stage_for_sub == SerTurnStage::HoldOrGoChoice {
                    t_string!(i18n, hint_hold_or_go).to_owned()
                } else {
                    String::new()
                };
                (!hint.is_empty()).then(|| view! { <p class="game-sub-prompt">{hint}</p> })
            }}

            // ── Board + side panel ───────────────────────────────────────────
            <div class="board-and-panel">
                <Board
                    view_state=vs
                    player_id=player_id
                    selected_origin=selected_origin
                    staged_moves=staged_moves
                    valid_sequences=valid_sequences
                    bar_dice=show_dice.then_some(dice)
                    bar_is_move=is_move_stage
                    bar_is_double=is_double_dice
                    last_moves=last_moves
                />

                // ── Side panel (scoring panels only) ─────────────────────────
                <div class="side-panel">
                    {my_scored_event.map(|event| view! {
                        <ScoringPanel event=event turn_stage=turn_stage_for_panel />
                    })}
                    {opp_scored_event.map(|event| view! {
                        <ScoringPanel event=event turn_stage=SerTurnStage::RollDice is_opponent=true />
                    })}
                </div>
            </div>

            // ── Action buttons below board (§10c) ────────────────────────────
            <div class="board-actions">
                {waiting_for_confirm.then(|| view! {
                    <button class="btn btn-primary" on:click=move |_| {
                        pending.update(|q| { q.pop_front(); });
                    }>{t!(i18n, continue_btn)}</button>
                })}
                // Fallback Go button when no scoring panel (e.g. after reconnect)
                {show_hold_go.then(|| view! {
                    <button class="btn btn-primary" on:click=move |_| {
                        cmd_tx_go.unbounded_send(NetCommand::Action(PlayerAction::Go)).ok();
                    }>{t!(i18n, go)}</button>
                })}
                {move || {
                    // Show the empty-move button only when (0,0) is a valid
                    // first or second move given what has already been staged.
                    let staged = staged_moves.get();
                    let show = is_move_stage && staged.len() < 2 && (
                        valid_seqs_empty.is_empty() || match staged.len() {
                            0 => valid_seqs_empty.iter().any(|(m1, _)| m1.get_from() == 0),
                            1 => {
                                let (f0, t0) = staged[0];
                                valid_seqs_empty.iter()
                                    .filter(|(m1, _)| {
                                        m1.get_from() as u8 == f0
                                            && m1.get_to() as u8 == t0
                                    })
                                    .any(|(_, m2)| m2.get_from() == 0)
                            }
                            _ => false,
                        }
                    );
                    show.then(|| view! {
                        <button
                            class="btn btn-secondary"
                            on:click=move |_| {
                                selected_origin.set(None);
                                staged_moves.update(|v| v.push((0, 0)));
                            }
                        >{t!(i18n, empty_move)}</button>
                    })
                }}
            </div>

            // ── Player score (below board) ────────────────────────────────────
            <PlayerScorePanel score=my_score is_you=true />

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
                            <div class="game-over-score">
                                <span class="game-over-score-name">{my_name_end}</span>
                                <span class="game-over-score-nums">
                                    {format!("{my_holes_end} — {opp_holes_end}")}
                                </span>
                                <span class="game-over-score-name">{opp_name_end.clone()}</span>
                            </div>
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

            // ── Hole toast (§6a) — board-centered overlay when a hole is won ──
            {hole_toast_info.map(|(holes_total, bredouille)| view! {
                <div class="hole-toast" class:hole-toast-bredouille=bredouille>
                    <div class="hole-toast-title">"Trou !"</div>
                    <div class="hole-toast-count">{format!("{holes_total} / 12")}</div>
                    {bredouille.then(|| view! {
                        <div class="hole-toast-bredouille">"× 2 bredouille"</div>
                    })}
                </div>
            })}
        </div>
    }
}
