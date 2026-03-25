use leptos::prelude::*;

use crate::trictrac::types::PlayerScore;

#[component]
pub fn ScorePanel(scores: [PlayerScore; 2], player_id: u16) -> impl IntoView {
    let rows: Vec<_> = scores
        .into_iter()
        .enumerate()
        .map(|(i, score)| {
            let label = if i as u16 == player_id { " (you)" } else { "" };
            view! {
                <div class="score-row">
                    <span class="score-name">{score.name}{label}</span>
                    <span class="score-points">"Points: "{score.points}</span>
                    <span class="score-holes">"Holes: "{score.holes}</span>
                </div>
            }
        })
        .collect();

    view! {
        <div class="score-panel">{rows}</div>
    }
}
