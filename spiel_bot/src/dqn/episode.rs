//! DQN self-play episode generation.
//!
//! Both players share the same Q-network (the [`TrictracEnv`] handles board
//! mirroring so that each player always acts from "White's perspective").
//! Transitions for both players are stored in the returned sample list.
//!
//! # Reward
//!
//! After each full decision (action applied and the state has advanced through
//! any intervening chance nodes back to the same player's next turn), the
//! reward is:
//!
//! ```text
//! r = (my_total_score_now − my_total_score_then)
//!   − (opp_total_score_now − opp_total_score_then)
//! ```
//!
//! where `total_score = holes × 12 + points`.
//!
//! # Transition structure
//!
//! We use a "pending transition" per player.  When a player acts again, we
//! *complete* the previous pending transition by filling in `next_obs`,
//! `next_legal`, and computing `reward`.  Terminal transitions are completed
//! when the game ends.

use burn::tensor::{backend::Backend, Tensor, TensorData};
use rand::Rng;

use crate::env::{GameEnv, TrictracEnv};
use crate::network::QValueNet;
use super::DqnSample;

// ── Internals ─────────────────────────────────────────────────────────────────

struct PendingTransition {
    obs: Vec<f32>,
    action: usize,
    /// Score snapshot `[p1_total, p2_total]` at the moment of the action.
    score_before: [i32; 2],
}

/// Pick an action ε-greedily: random with probability `epsilon`, greedy otherwise.
fn epsilon_greedy<B: Backend, Q: QValueNet<B>>(
    q_net: &Q,
    obs: &[f32],
    legal: &[usize],
    epsilon: f32,
    rng: &mut impl Rng,
    device: &B::Device,
) -> usize {
    debug_assert!(!legal.is_empty(), "epsilon_greedy: no legal actions");
    if rng.random::<f32>() < epsilon {
        legal[rng.random_range(0..legal.len())]
    } else {
        let obs_tensor = Tensor::<B, 2>::from_data(
            TensorData::new(obs.to_vec(), [1, obs.len()]),
            device,
        );
        let q_values: Vec<f32> = q_net.forward(obs_tensor).into_data().to_vec().unwrap();
        legal
            .iter()
            .copied()
            .max_by(|&a, &b| {
                q_values[a].partial_cmp(&q_values[b]).unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap()
    }
}

/// Reward for `player_idx` (0 = P1, 1 = P2) given score snapshots before/after.
fn compute_reward(player_idx: usize, score_before: &[i32; 2], score_after: &[i32; 2]) -> f32 {
    let opp_idx = 1 - player_idx;
    ((score_after[player_idx] - score_before[player_idx])
        - (score_after[opp_idx] - score_before[opp_idx])) as f32
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Play one full game and return all transitions for both players.
///
/// - `q_net` uses the **inference backend** (no autodiff wrapper).
/// - `epsilon` in `[0, 1]`: probability of taking a random action.
/// - `reward_scale`: reward divisor (e.g. `12.0` to map one hole → `±1`).
pub fn generate_dqn_episode<B: Backend, Q: QValueNet<B>>(
    env: &TrictracEnv,
    q_net: &Q,
    epsilon: f32,
    rng: &mut impl Rng,
    device: &B::Device,
    reward_scale: f32,
) -> Vec<DqnSample> {
    let obs_size = env.obs_size();
    let mut state = env.new_game();
    let mut pending: [Option<PendingTransition>; 2] = [None, None];
    let mut samples: Vec<DqnSample> = Vec::new();

    loop {
        // ── Advance past chance nodes ──────────────────────────────────────
        while env.current_player(&state).is_chance() {
            env.apply_chance(&mut state, rng);
        }

        let score_now = TrictracEnv::score_snapshot(&state);

        if env.current_player(&state).is_terminal() {
            // Complete all pending transitions as terminal.
            for player_idx in 0..2 {
                if let Some(prev) = pending[player_idx].take() {
                    let reward =
                        compute_reward(player_idx, &prev.score_before, &score_now) / reward_scale;
                    samples.push(DqnSample {
                        obs: prev.obs,
                        action: prev.action,
                        reward,
                        next_obs: vec![0.0; obs_size],
                        next_legal: vec![],
                        done: true,
                    });
                }
            }
            break;
        }

        let player_idx = env.current_player(&state).index().unwrap();
        let legal = env.legal_actions(&state);
        let obs = env.observation(&state, player_idx);

        // ── Complete the previous transition for this player ───────────────
        if let Some(prev) = pending[player_idx].take() {
            let reward =
                compute_reward(player_idx, &prev.score_before, &score_now) / reward_scale;
            samples.push(DqnSample {
                obs: prev.obs,
                action: prev.action,
                reward,
                next_obs: obs.clone(),
                next_legal: legal.clone(),
                done: false,
            });
        }

        // ── Pick and apply action ──────────────────────────────────────────
        let action = epsilon_greedy(q_net, &obs, &legal, epsilon, rng, device);
        env.apply(&mut state, action);

        // ── Record new pending transition ──────────────────────────────────
        pending[player_idx] = Some(PendingTransition {
            obs,
            action,
            score_before: score_now,
        });
    }

    samples
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use burn::backend::NdArray;
    use rand::{SeedableRng, rngs::SmallRng};

    use crate::network::{QNet, QNetConfig};

    type B = NdArray<f32>;

    fn device() -> <B as Backend>::Device { Default::default() }
    fn rng() -> SmallRng { SmallRng::seed_from_u64(7) }

    fn tiny_q() -> QNet<B> {
        QNet::new(&QNetConfig::default(), &device())
    }

    #[test]
    fn episode_terminates_and_produces_samples() {
        let env = TrictracEnv;
        let q = tiny_q();
        let samples = generate_dqn_episode(&env, &q, 1.0, &mut rng(), &device(), 1.0);
        assert!(!samples.is_empty(), "episode must produce at least one sample");
    }

    #[test]
    fn episode_obs_size_correct() {
        let env = TrictracEnv;
        let q = tiny_q();
        let samples = generate_dqn_episode(&env, &q, 1.0, &mut rng(), &device(), 1.0);
        for s in &samples {
            assert_eq!(s.obs.len(), 217, "obs size mismatch");
            if s.done {
                assert_eq!(s.next_obs.len(), 217, "done next_obs should be zeros of obs_size");
                assert!(s.next_legal.is_empty());
            } else {
                assert_eq!(s.next_obs.len(), 217, "next_obs size mismatch");
                assert!(!s.next_legal.is_empty());
            }
        }
    }

    #[test]
    fn episode_actions_within_action_space() {
        let env = TrictracEnv;
        let q = tiny_q();
        let samples = generate_dqn_episode(&env, &q, 1.0, &mut rng(), &device(), 1.0);
        for s in &samples {
            assert!(s.action < 514, "action {} out of bounds", s.action);
        }
    }

    #[test]
    fn greedy_episode_also_terminates() {
        let env = TrictracEnv;
        let q = tiny_q();
        let samples = generate_dqn_episode(&env, &q, 0.0, &mut rng(), &device(), 1.0);
        assert!(!samples.is_empty());
    }

    #[test]
    fn at_least_one_done_sample() {
        let env = TrictracEnv;
        let q = tiny_q();
        let samples = generate_dqn_episode(&env, &q, 1.0, &mut rng(), &device(), 1.0);
        let n_done = samples.iter().filter(|s| s.done).count();
        // Two players, so 1 or 2 terminal transitions.
        assert!(n_done >= 1 && n_done <= 2, "expected 1-2 done samples, got {n_done}");
    }

    #[test]
    fn compute_reward_correct() {
        // P1 gains 4 points (2 holes 10 pts → 3 holes 2 pts), opp unchanged.
        let before = [2 * 12 + 10, 0];
        let after  = [3 * 12 + 2,  0];
        let r = compute_reward(0, &before, &after);
        assert!((r - 4.0).abs() < 1e-6, "expected 4.0, got {r}");
    }

    #[test]
    fn compute_reward_with_opponent_scoring() {
        // P1 gains 2, opp gains 3 → net = -1 from P1's perspective.
        let before = [0, 0];
        let after  = [2, 3];
        let r = compute_reward(0, &before, &after);
        assert!((r - (-1.0)).abs() < 1e-6, "expected -1.0, got {r}");
    }
}
