use futures::channel::mpsc;
use leptos::prelude::*;

use backbone_lib::traits::{BackEndArchitecture, BackendCommand};

use crate::app::{GameUiState, NetCommand, PauseReason, Screen};
use crate::game::trictrac::backend::TrictracBackend;
use crate::game::trictrac::bot_local::bot_decide;
use crate::game::trictrac::types::{
    JanEntry, ScoredEvent, SerStage, SerTurnStage, ViewState,
};
use trictrac_store::CheckerMove;

use std::collections::VecDeque;

/// Runs one local bot game. Returns `true` if the player wants to play again.
pub async fn run_local_bot_game(
    screen: RwSignal<Screen>,
    cmd_rx: &mut mpsc::UnboundedReceiver<NetCommand>,
    pending: RwSignal<VecDeque<GameUiState>>,
    player_name: String,
) -> bool {
    let mut backend = TrictracBackend::new(0);
    backend.player_arrival(0);
    backend.player_arrival(1);

    let mut vs = ViewState::default_with_names(&player_name, "Bot");
    for cmd in backend.drain_commands() {
        match cmd {
            BackendCommand::ResetViewState => {
                vs = backend.get_view_state().clone();
            }
            BackendCommand::Delta(delta) => {
                vs.apply_delta(&delta);
            }
            _ => {}
        }
    }
    patch_bot_names(&mut vs, &player_name);
    screen.set(Screen::Playing(GameUiState {
        view_state: vs.clone(),
        player_id: 0,
        room_id: String::new(),
        is_bot_game: true,
        waiting_for_confirm: false,
        pause_reason: None,
        my_scored_event: None,
        opp_scored_event: None,
        last_moves: None,
        suppress_dice_anim: false,
    }));

    use futures::StreamExt;
    loop {
        match cmd_rx.next().await {
            Some(NetCommand::Action(action)) => {
                let prev_vs = vs.clone();
                backend.inform_rpc(0, action);
                for cmd in backend.drain_commands() {
                    if let BackendCommand::Delta(delta) = cmd {
                        vs.apply_delta(&delta);
                    }
                }
                patch_bot_names(&mut vs, &player_name);
                let scored = compute_scored_event(&prev_vs, &vs, 0);
                let opp_scored = compute_scored_event(&prev_vs, &vs, 1);
                screen.set(Screen::Playing(GameUiState {
                    view_state: vs.clone(),
                    player_id: 0,
                    room_id: String::new(),
                    is_bot_game: true,
                    waiting_for_confirm: false,
                    pause_reason: None,
                    my_scored_event: scored,
                    opp_scored_event: opp_scored,
                    last_moves: compute_last_moves(&prev_vs, &vs, true),
                    suppress_dice_anim: false,
                }));
            }
            Some(NetCommand::PlayVsBot) => return true,
            _ => return false,
        }

        loop {
            let pgr = backend.get_view_state().pre_game_roll.clone();
            match bot_decide(backend.get_game(), pgr.as_ref()) {
                None => break,
                Some(action) => {
                    backend.inform_rpc(1, action);
                    for cmd in backend.drain_commands() {
                        if let BackendCommand::Delta(delta) = cmd {
                            let delta_prev_vs = vs.clone();
                            vs.apply_delta(&delta);
                            patch_bot_names(&mut vs, &player_name);
                            push_or_show(
                                &delta_prev_vs,
                                GameUiState {
                                    view_state: vs.clone(),
                                    player_id: 0,
                                    room_id: String::new(),
                                    is_bot_game: true,
                                    waiting_for_confirm: false,
                                    pause_reason: None,
                                    my_scored_event: None,
                                    opp_scored_event: None,
                                    last_moves: compute_last_moves(&delta_prev_vs, &vs, false),
                                    suppress_dice_anim: false,
                                },
                                pending,
                                screen,
                            );
                        }
                    }
                }
            }
        }
    }
}

/// Patches the player names in a ViewState after a backend delta (bot game: slot 0 = human, 1 = Bot).
pub fn patch_bot_names(vs: &mut ViewState, player_name: &str) {
    vs.scores[0].name = player_name.to_string();
    vs.scores[1].name = "Bot".to_string();
}

/// Patches the local player's name in a ViewState after a backend delta (multiplayer).
pub fn patch_player_name(vs: &mut ViewState, player_id: u16, name: &str) {
    vs.scores[player_id as usize].name = name.to_string();
}

/// Returns the checker moves to animate when the board changed between two ViewStates.
pub fn compute_last_moves(
    prev: &ViewState,
    next: &ViewState,
    own_move: bool,
) -> Option<(CheckerMove, CheckerMove)> {
    if prev.board == next.board {
        return None;
    }
    let (m1, m2) = next.dice_moves;
    if m1 == CheckerMove::default() && m2 == CheckerMove::default() {
        return None;
    }
    if own_move {
        if m2 == CheckerMove::default() {
            return None;
        }
        return Some((m2, CheckerMove::default()));
    }
    Some((m1, m2))
}

/// Computes a scoring event for `player_id` by comparing the previous and next ViewState.
pub fn compute_scored_event(prev: &ViewState, next: &ViewState, player_id: u16) -> Option<ScoredEvent> {
    let prev_score = &prev.scores[player_id as usize];
    let next_score = &next.scores[player_id as usize];

    let holes_gained = next_score.holes.saturating_sub(prev_score.holes);
    if holes_gained == 0 && prev_score.points == next_score.points {
        return None;
    }

    let bredouille = holes_gained > 0 && prev_score.can_bredouille;

    let my_jans: Vec<JanEntry> = if next.active_mp_player == Some(player_id)
        && prev.active_mp_player == Some(player_id)
    {
        next.dice_jans
            .iter()
            .filter(|e| e.total > 0)
            .cloned()
            .collect()
    } else if next.active_mp_player == Some(player_id) && prev.active_mp_player != Some(player_id) {
        next.dice_jans
            .iter()
            .filter(|e| e.total < 0)
            .map(|e| JanEntry {
                total: -e.total,
                points_per: -e.points_per,
                ..e.clone()
            })
            .collect()
    } else {
        return None;
    };

    let points_earned: u8 = my_jans
        .iter()
        .fold(0u8, |acc, e| acc.saturating_add(e.total.unsigned_abs()));

    if points_earned == 0 && holes_gained == 0 {
        return None;
    }

    Some(ScoredEvent {
        points_earned,
        holes_gained,
        holes_total: next_score.holes,
        bredouille,
        jans: my_jans,
    })
}

/// Either queues the state as a confirmation step or shows it immediately.
pub fn push_or_show(
    prev_vs: &ViewState,
    new_state: GameUiState,
    pending: RwSignal<VecDeque<GameUiState>>,
    screen: RwSignal<Screen>,
) {
    let scored = compute_scored_event(prev_vs, &new_state.view_state, new_state.player_id);
    let opp_scored = compute_scored_event(prev_vs, &new_state.view_state, 1 - new_state.player_id);

    if let Some(reason) = infer_pause_reason(prev_vs, &new_state.view_state, new_state.player_id) {
        pending.update(|q| {
            q.push_back(GameUiState {
                waiting_for_confirm: true,
                pause_reason: Some(reason),
                my_scored_event: scored,
                opp_scored_event: opp_scored,
                ..new_state.clone()
            });
        });
        screen.set(Screen::Playing(GameUiState {
            last_moves: None,
            suppress_dice_anim: true,
            ..new_state
        }));
    } else {
        screen.set(Screen::Playing(GameUiState {
            my_scored_event: scored,
            opp_scored_event: opp_scored,
            ..new_state
        }));
    }
}

/// Compares the previous and next ViewState to decide whether the transition
/// warrants a confirmation pause.
pub fn infer_pause_reason(prev: &ViewState, next: &ViewState, player_id: u16) -> Option<PauseReason> {
    let opponent_id = 1 - player_id;

    if next.stage == SerStage::PreGameRoll {
        if let (Some(prev_pgr), Some(next_pgr)) = (&prev.pre_game_roll, &next.pre_game_roll) {
            let both_now = next_pgr.host_die.is_some() && next_pgr.guest_die.is_some();
            let both_before = prev_pgr.host_die.is_some() && prev_pgr.guest_die.is_some();
            if both_now && !both_before {
                return Some(PauseReason::AfterOpponentPreGameRoll);
            }
        }
        return None;
    }

    if prev.stage == SerStage::PreGameRoll {
        return None;
    }

    if next.active_mp_player == Some(opponent_id) {
        if next.dice != prev.dice {
            return Some(PauseReason::AfterOpponentRoll);
        }
        if prev.turn_stage == SerTurnStage::HoldOrGoChoice && next.turn_stage == SerTurnStage::Move {
            return Some(PauseReason::AfterOpponentGo);
        }
    }

    if next.active_mp_player == Some(player_id) && prev.active_mp_player == Some(opponent_id) {
        return Some(PauseReason::AfterOpponentMove);
    }

    None
}
