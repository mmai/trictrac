use leptos::prelude::*;

use crate::app::GameUiState;
use crate::trictrac::types::SerStage;

#[component]
pub fn GameScreen(state: GameUiState) -> impl IntoView {
    let status = match state.view_state.stage {
        SerStage::Ended => "Game over",
        SerStage::PreGame => "Waiting for players…",
        SerStage::InGame => match state.view_state.active_mp_player {
            Some(id) if id == state.player_id => "Your turn",
            Some(_) => "Opponent's turn",
            None => "…",
        },
    };

    view! {
        <div class="game-container">
            <p class="status-bar">{status}</p>
            // Board and score panel will be added in a subsequent step.
            <p>"Board placeholder"</p>
        </div>
    }
}
