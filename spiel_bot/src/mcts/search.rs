//! Simulation, expansion, backup, and noise helpers.
//!
//! These are internal to the `mcts` module; the public entry points are
//! [`super::run_mcts`], [`super::mcts_policy`], and [`super::select_action`].

use rand::Rng;
use rand_distr::{Gamma, Distribution};

use crate::env::GameEnv;
use super::{Evaluator, MctsConfig};
use super::node::MctsNode;

// ── Masked softmax ─────────────────────────────────────────────────────────

/// Numerically stable softmax over `legal` actions only.
///
/// Illegal logits are treated as `-∞` and receive probability `0.0`.
/// Returns a probability vector of length `action_space`.
pub(super) fn masked_softmax(logits: &[f32], legal: &[usize], action_space: usize) -> Vec<f32> {
    let mut probs = vec![0.0f32; action_space];
    if legal.is_empty() {
        return probs;
    }
    let max_logit = legal
        .iter()
        .map(|&a| logits[a])
        .fold(f32::NEG_INFINITY, f32::max);
    let mut sum = 0.0f32;
    for &a in legal {
        let e = (logits[a] - max_logit).exp();
        probs[a] = e;
        sum += e;
    }
    if sum > 0.0 {
        for &a in legal {
            probs[a] /= sum;
        }
    } else {
        let uniform = 1.0 / legal.len() as f32;
        for &a in legal {
            probs[a] = uniform;
        }
    }
    probs
}

// ── Dirichlet noise ────────────────────────────────────────────────────────

/// Mix Dirichlet(α, …, α) noise into the root's children priors for exploration.
///
/// Standard AlphaZero parameters: `alpha = 0.3`, `eps = 0.25`.
/// Uses the Gamma-distribution trick: Dir(α,…,α) = Gamma(α,1)^n / sum.
pub(super) fn add_dirichlet_noise(
    node: &mut MctsNode,
    alpha: f32,
    eps: f32,
    rng: &mut impl Rng,
) {
    let n = node.children.len();
    if n == 0 {
        return;
    }
    let Ok(gamma) = Gamma::new(alpha as f64, 1.0_f64) else {
        return;
    };
    let samples: Vec<f32> = (0..n).map(|_| gamma.sample(rng) as f32).collect();
    let sum: f32 = samples.iter().sum();
    if sum <= 0.0 {
        return;
    }
    for (i, (_, child)) in node.children.iter_mut().enumerate() {
        let noise = samples[i] / sum;
        child.p = (1.0 - eps) * child.p + eps * noise;
    }
}

// ── Expansion ──────────────────────────────────────────────────────────────

/// Evaluate the network at `state` and populate `node` with children.
///
/// Sets `node.n = 1`, `node.w = value`, `node.expanded = true`.
/// Returns the network value estimate from `player_idx`'s perspective.
pub(super) fn expand<E: GameEnv>(
    node: &mut MctsNode,
    state: &E::State,
    env: &E,
    evaluator: &dyn Evaluator,
    player_idx: usize,
) -> f32 {
    let obs = env.observation(state, player_idx);
    let legal = env.legal_actions(state);
    let (logits, value) = evaluator.evaluate(&obs);
    let priors = masked_softmax(&logits, &legal, env.action_space());
    node.children = legal.iter().map(|&a| (a, MctsNode::new(priors[a]))).collect();
    node.expanded = true;
    node.n = 1;
    node.w = value;
    value
}

// ── Simulation ─────────────────────────────────────────────────────────────

/// One MCTS simulation from an **already-expanded** decision node.
///
/// Traverses the tree with PUCT selection, expands the first unvisited leaf,
/// and backs up the result.
///
/// * `player_idx` — the player (0 or 1) who acts at `state`.
/// * Returns the backed-up value **from `player_idx`'s perspective**.
pub(super) fn simulate<E: GameEnv>(
    node: &mut MctsNode,
    state: E::State,
    env: &E,
    evaluator: &dyn Evaluator,
    config: &MctsConfig,
    rng: &mut impl Rng,
    player_idx: usize,
) -> f32 {
    debug_assert!(node.expanded, "simulate called on unexpanded node");

    // ── Selection: child with highest PUCT ────────────────────────────────
    let parent_n = node.n;
    let best = node
        .children
        .iter()
        .enumerate()
        .max_by(|(_, (_, a)), (_, (_, b))| {
            a.puct(parent_n, config.c_puct)
                .partial_cmp(&b.puct(parent_n, config.c_puct))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(i, _)| i)
        .expect("expanded node must have at least one child");

    let (action, child) = &mut node.children[best];
    let action = *action;

    // ── Apply action + advance through any chance nodes ───────────────────
    let mut next_state = state;
    env.apply(&mut next_state, action);
    while env.current_player(&next_state).is_chance() {
        env.apply_chance(&mut next_state, rng);
    }

    let next_cp = env.current_player(&next_state);

    // ── Evaluate leaf or terminal ──────────────────────────────────────────
    // All values are converted to `player_idx`'s perspective before backup.
    let child_value = if next_cp.is_terminal() {
        let returns = env
            .returns(&next_state)
            .expect("terminal node must have returns");
        returns[player_idx]
    } else {
        let child_player = next_cp.index().unwrap();
        let v = if child.expanded {
            simulate(child, next_state, env, evaluator, config, rng, child_player)
        } else {
            expand::<E>(child, &next_state, env, evaluator, child_player)
        };
        // Negate when the child belongs to the opponent.
        if child_player == player_idx { v } else { -v }
    };

    // ── Backup ────────────────────────────────────────────────────────────
    node.n += 1;
    node.w += child_value;

    child_value
}
