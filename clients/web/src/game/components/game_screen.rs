use std::cell::Cell;
use std::collections::VecDeque;

use futures::channel::mpsc::UnboundedSender;
use leptos::prelude::*;
use trictrac_store::{Board as StoreBoard, CheckerMove, Color, Dice as StoreDice, Jan, MoveRules};

use super::die::Die;
use crate::app::{GameUiState, NetCommand, PauseReason};
use crate::game::trictrac::types::{PlayerAction, PreGameRollState, SerStage, SerTurnStage};
use crate::i18n::*;
use crate::portal::lobby::{qr_svg, room_url};

use super::board::Board;
use super::score_panel::MergedScorePanel;
use super::scoring::ScoringPanel;

#[component]
pub fn GameScreen(state: GameUiState) -> impl IntoView {
    let i18n = use_i18n();

    let auth_username =
        use_context::<RwSignal<Option<String>>>().unwrap_or_else(|| RwSignal::new(None));
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
    let pending =
        use_context::<RwSignal<VecDeque<GameUiState>>>().expect("pending not found in context");
    let cmd_tx_effect = cmd_tx.clone();
    // Non-reactive counter so we can detect when staged_moves grows without
    // returning a value from the Effect (which causes a Leptos reactive loop
    // when the Effect also writes to the same signal it reads).
    let prev_staged_len = Cell::new(0usize);

    Effect::new(move |_| {
        let moves = staged_moves.get();
        let n = moves.len();
        // Play checker sound whenever a move is added (own moves, immediate feedback).
        if n > prev_staged_len.get() {
            crate::game::sound::play_checker_move();
        }
        prev_staged_len.set(n);
        if n == 2 {
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
            // Reset the counter so the next turn starts clean.
            prev_staged_len.set(0);
        }
    });

    // ── Auto-roll effect ─────────────────────────────────────────────────────
    // GameScreen is fully re-mounted on every ViewState update (state is a
    // plain prop, not a signal), so this effect fires exactly once per
    // RollDice phase entry and will not double-send.
    // Guard: suppressed while waiting_for_confirm — the AfterOpponentMove
    // buffered state shows the human's RollDice turn but the auto-roll must
    // wait until the buffer is drained and the live screen state is shown.
    // Guard: never auto-roll during the pre-game ceremony (the ceremony overlay
    // has its own Roll button for PlayerAction::PreGameRoll).
    let show_roll =
        is_my_turn && vs.turn_stage == SerTurnStage::RollDice && vs.stage != SerStage::PreGameRoll;
    if show_roll && !waiting_for_confirm {
        let cmd_tx_auto = cmd_tx.clone();
        Effect::new(move |_| {
            cmd_tx_auto
                .unbounded_send(NetCommand::Action(PlayerAction::Roll))
                .ok();
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
        let color = if player_id == 0 {
            Color::White
        } else {
            Color::Black
        };
        let rules = MoveRules::new(&color, &store_board, store_dice);
        let raw = rules.get_possible_moves_sequences(true, vec![]);
        if player_id == 0 {
            raw
        } else {
            raw.into_iter()
                .map(|(m1, m2)| (m1.mirror(), m2.mirror()))
                .collect()
        }
    } else {
        vec![]
    };
    // Clone for the empty-move button reactive closure (Board consumes the original).
    let valid_seqs_empty = valid_sequences.clone();

    // ── Scores ─────────────────────────────────────────────────────────────────
    let my_score = vs.scores[player_id as usize].clone();
    let opp_score = vs.scores[1 - player_id as usize].clone();

    // ── Ceremony state (extracted before vs is moved into Board) ────────────────
    let is_ceremony = vs.stage == SerStage::PreGameRoll;
    let pre_game_roll_data: Option<PreGameRollState> = vs.pre_game_roll.clone();
    let my_name_ceremony = my_score.name.clone();
    let opp_name_ceremony = opp_score.name.clone();
    let cmd_tx_ceremony = cmd_tx.clone();

    // ── Scoring notifications ──────────────────────────────────────────────────
    let my_scored_event = state.my_scored_event.clone();
    let opp_scored_event = state.opp_scored_event.clone();

    // Values for MergedScorePanel — extracted before events are consumed.
    // Don't animate points when a hole was gained (points wrap around 12).
    let my_pts_earned: u8 = my_scored_event.as_ref().map_or(0, |e| {
        if e.holes_gained == 0 {
            e.points_earned
        } else {
            0
        }
    });
    let opp_pts_earned: u8 = opp_scored_event.as_ref().map_or(0, |e| {
        if e.holes_gained == 0 {
            e.points_earned
        } else {
            0
        }
    });
    let my_holes_gained_score: u8 = my_scored_event.as_ref().map_or(0, |e| e.holes_gained);
    let opp_holes_gained_score: u8 = opp_scored_event.as_ref().map_or(0, |e| e.holes_gained);
    let my_bredouille_flash: bool = my_scored_event
        .as_ref()
        .map_or(false, |e| e.bredouille && e.holes_gained > 0);

    let is_double_dice = dice.0 == dice.1 && dice.0 != 0;

    let last_moves = state.last_moves;

    // fields where a battue (hit) was scored; ripple animation shown there.
    let hit_fields: Vec<u8> = {
        let is_hit_jan = |jan: &Jan| {
            matches!(
                jan,
                Jan::TrueHitSmallJan
                    | Jan::TrueHitBigJan
                    | Jan::TrueHitOpponentCorner
                    | Jan::FalseHitSmallJan
                    | Jan::FalseHitBigJan
            )
        };
        let mut fields: Vec<u8> = vec![];
        for event_opt in [&my_scored_event, &opp_scored_event] {
            if let Some(event) = event_opt {
                for entry in &event.jans {
                    if is_hit_jan(&entry.jan) {
                        for (m1, m2) in &entry.moves {
                            for m in [m1, m2] {
                                let to = m.get_to() as u8;
                                if to != 0 && !fields.contains(&to) {
                                    fields.push(to);
                                }
                            }
                        }
                    }
                }
            }
        }
        fields
    };

    // ── Sound effects (fire once on mount = once per state snapshot) ──────────
    // Dice roll: dice just appeared (no preceding moves in this snapshot).
    if show_dice && last_moves.is_none() {
        crate::game::sound::play_dice_roll();
    }
    // Checker move: moves were committed in the preceding action.
    if last_moves.is_some() {
        crate::game::sound::play_checker_move();
    }
    // Scoring: hole fanfare plays immediately; per-point ticks are driven by
    // MergedScorePanel's counter animation so play_points_scored is not called here.
    if let Some(ref ev) = my_scored_event {
        if ev.holes_gained > 0 {
            crate::game::sound::play_hole_scored();
        }
    }
    if let Some(ref ev) = opp_scored_event {
        if ev.holes_gained > 0 {
            crate::game::sound::play_opp_hole_scored();
        }
    }

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

    // Open by default for the room creator while waiting for an opponent.
    // When the opponent joins the stage becomes PreGameRoll, so the next
    // re-mount will initialise this to false — auto-closing the popover.
    let share_open = RwSignal::new(!is_bot_game && player_id == 0 && stage == SerStage::PreGame);
    let share_copied = RwSignal::new(false);
    let share_url = if !is_bot_game {
        room_url(&room_id)
    } else {
        String::new()
    };
    let share_svg = if !is_bot_game {
        qr_svg(&share_url)
    } else {
        String::new()
    };

    view! {
        <div class="game-container">
            // ── Top bar ──────────────────────────────────────────────────────
            <div class="top-bar">
                <span>{move || if is_bot_game {
                    t_string!(i18n, vs_bot_label).to_owned()
                } else {
                    t_string!(i18n, room_label, id = room_id.as_str())
                }}</span>

                {move || (!is_bot_game).then(|| view! {
                    <button
                        class="quit-link"
                        style="border:none;background:transparent;cursor:pointer"
                        on:click=move |_| share_open.update(|v| *v = !*v)
                    >
                        {move || if share_open.get() { "✕ " } else { "" }}
                        {t!(i18n, share_btn)}
                    </button>
                })}

                {move || auth_username.get().map(|u| view! {
                    <span class="playing-as">"▶ " <strong>{u}</strong></span>
                })}

                <a class="quit-link" href="#" on:click=move |e| {
                    e.prevent_default();
                    cmd_tx_quit.unbounded_send(NetCommand::Disconnect).ok();
                }>{t!(i18n, quit)}</a>
            </div>

            // ── Share popover ─────────────────────────────────────────────────
            {move || share_open.get().then(|| {
                let url = share_url.clone();
                let url_copy = share_url.clone();
                view! {
                    <div class="share-popover">
                        <p class="share-popover-label">{t!(i18n, share_link)}</p>
                        <div class="share-url-row">
                            <span class="share-url-text">{url}</span>
                            <button class="share-copy-btn" on:click=move |_| {
                                #[cfg(target_arch = "wasm32")]
                                {
                                    let u = url_copy.clone();
                                    wasm_bindgen_futures::spawn_local(async move {
                                        if let Some(cb) = web_sys::window()
                                            .map(|w| w.navigator().clipboard())
                                        {
                                            let _ = wasm_bindgen_futures::JsFuture::from(
                                                cb.write_text(&u),
                                            )
                                            .await;
                                            share_copied.set(true);
                                            gloo_timers::future::TimeoutFuture::new(2000).await;
                                            share_copied.set(false);
                                        }
                                    });
                                }
                            }>
                                {move || if share_copied.get() {
                                    t_string!(i18n, link_copied)
                                } else {
                                    t_string!(i18n, copy_link)
                                }}
                            </button>
                        </div>
                        <p class="share-popover-label" style="margin-top:0.75rem">
                            {t!(i18n, scan_qr)}
                        </p>
                        <div class="qr-container" inner_html=share_svg.clone() />
                    </div>
                }
            })}

            // ── Merged scoreboard + scoring panels (above board) ─────────────
            // score-area is position:relative so the scoring-panels-container
            // can be absolute-positioned at the right of the hole counter.
            <div class="score-area">
                <MergedScorePanel
                    my_score=my_score
                    opp_score=opp_score
                    my_points_earned=my_pts_earned
                    opp_points_earned=opp_pts_earned
                    my_holes_gained=my_holes_gained_score
                    opp_holes_gained=opp_holes_gained_score
                    my_bredouille=my_bredouille_flash
                />
                // Scoring detail panels — stacked at the right, overlapping if needed.
                <div class="scoring-panels-container">
                    {my_scored_event.map(|event| view! {
                        <ScoringPanel event=event turn_stage=turn_stage_for_panel />
                    })}
                    {opp_scored_event.map(|event| view! {
                        <ScoringPanel event=event turn_stage=SerTurnStage::RollDice is_opponent=true />
                    })}
                </div>
            </div>

            // ── Board ────────────────────────────────────────────────────────
            <Board
                view_state=vs
                player_id=player_id
                selected_origin=selected_origin
                staged_moves=staged_moves
                valid_sequences=valid_sequences
                bar_dice=show_dice.then_some(dice)
                bar_is_move=is_move_stage
                is_my_turn=is_my_turn
                bar_is_double=is_double_dice
                last_moves=last_moves
                hit_fields=hit_fields
            />

            // ── Status, hints, and actions — cream strip below board ─
            <div class="game-bottom-strip">
                <div class="game-status">
                    {move || {
                        if let Some(ref reason) = pause_reason {
                            return String::from(match reason {
                                PauseReason::AfterOpponentRoll => t_string!(i18n, after_opponent_roll),
                                PauseReason::AfterOpponentGo   => t_string!(i18n, after_opponent_go),
                                PauseReason::AfterOpponentMove => t_string!(i18n, after_opponent_move),
                                PauseReason::AfterOpponentPreGameRoll => t_string!(i18n, after_opponent_pre_game_roll),
                            });
                        }
                        let n = staged_moves.get().len();
                        if is_move_stage {
                            t_string!(i18n, select_move, n = n + 1)
                        } else {
                            String::from(match (&stage, is_my_turn, &turn_stage) {
                                (SerStage::Ended, _, _) => t_string!(i18n, game_over),
                                (SerStage::PreGame, _, _) | (SerStage::PreGameRoll, _, _) => t_string!(i18n, waiting_for_opponent),
                                (SerStage::InGame, true, SerTurnStage::RollDice) => t_string!(i18n, your_turn_roll),
                                (SerStage::InGame, true, SerTurnStage::HoldOrGoChoice) => t_string!(i18n, hold_or_go),
                                (SerStage::InGame, true, _) => t_string!(i18n, your_turn),
                                (SerStage::InGame, false, _) => t_string!(i18n, opponent_turn),
                            })
                        }
                    }}
                </div>
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
            </div>

            // ── Pre-game ceremony overlay ─────────────────────────────────────
            {is_ceremony.then(|| {
                let pgr = pre_game_roll_data.unwrap_or(PreGameRollState {
                    host_die: None,
                    guest_die: None,
                    tie_count: 0,
                });
                if pgr.host_die != None {
                    crate::game::sound::play_dice_roll();
                }

                let my_die = if player_id == 0 { pgr.host_die } else { pgr.guest_die };
                let opp_die = if player_id == 0 { pgr.guest_die } else { pgr.host_die };
                let can_roll = my_die.is_none() && !waiting_for_confirm;
                let show_tie = pgr.tie_count > 0;
                view! {
                    <div class="ceremony-overlay">
                        <div class="ceremony-box">
                            <h2>{t!(i18n, pre_game_roll_title)}</h2>
                            {show_tie.then(|| view! {
                                <p class="ceremony-tie">{t!(i18n, pre_game_roll_tie)}</p>
                            })}
                            <div class="ceremony-dice">
                                <div class="ceremony-die-slot">
                                    <span class="ceremony-die-label">{my_name_ceremony}{t!(i18n, you_suffix)}</span>
                                    <Die value=my_die.unwrap_or(0) used=false />
                                </div>
                                <div class="ceremony-die-slot">
                                    <span class="ceremony-die-label">{opp_name_ceremony}</span>
                                    <Die value=opp_die.unwrap_or(0) used=false />
                                </div>
                            </div>
                            {waiting_for_confirm.then(|| {
                                let pending_c = pending;
                                view! {
                                    <button class="btn btn-primary" on:click=move |_| {
                                        pending_c.update(|q| { q.pop_front(); });
                                    }>{t!(i18n, continue_btn)}</button>
                                }
                            })}
                            {can_roll.then(|| {
                                let cmd_tx_c = cmd_tx_ceremony.clone();
                                view! {
                                    <button class="btn btn-primary" on:click=move |_| {
                                        cmd_tx_c.unbounded_send(NetCommand::Action(PlayerAction::PreGameRoll)).ok();
                                    }>{t!(i18n, pre_game_roll_btn)}</button>
                                }
                            })}
                        </div>
                    </div>
                }
            })}

            // ── Game-over overlay ─────────────────────────────────────────────
            {stage_is_ended.then(|| {
                if winner_is_me {
                    crate::game::sound::play_victory();
                } else {
                    crate::game::sound::play_defeat();
                }
                let opp_name_end_clone = opp_name_end.clone();
                let winner_text = move || if winner_is_me {
                    t_string!(i18n, you_win).to_owned()
                } else {
                    t_string!(i18n, opp_wins, name = opp_name_end_clone.as_str())
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

        </div>
    }
}
