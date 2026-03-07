//! Monte Carlo Tree Search with PUCT selection and policy-value network guidance.
//!
//! # Algorithm
//!
//! The implementation follows AlphaZero's MCTS:
//!
//! 1. **Expand root** — run the network once to get priors and a value
//!    estimate; optionally add Dirichlet noise for training-time exploration.
//! 2. **Simulate** `n_simulations` times:
//!    - *Selection* — traverse the tree with PUCT until an unvisited leaf.
//!    - *Chance bypass* — call [`GameEnv::apply_chance`] at chance nodes;
//!      chance nodes are **not** stored in the tree (outcome sampling).
//!    - *Expansion* — evaluate the network at the leaf; populate children.
//!    - *Backup* — propagate the value upward; negate at each player boundary.
//! 3. **Policy** — normalized visit counts at the root ([`mcts_policy`]).
//! 4. **Action** — greedy (temperature = 0) or sampled ([`select_action`]).
//!
//! # Perspective convention
//!
//! Every [`MctsNode::w`] is stored **from the perspective of the player who
//! acts at that node**.  The backup negates the child value whenever the
//! acting player differs between parent and child.
//!
//! # Stochastic games
//!
//! When [`GameEnv::current_player`] returns [`Player::Chance`], the
//! simulation calls [`GameEnv::apply_chance`] to sample a random outcome and
//! continues.  Chance nodes are skipped transparently; Q-values converge to
//! their expectation over many simulations (outcome sampling).

pub mod node;
mod search;

pub use node::MctsNode;

use rand::Rng;

use crate::env::GameEnv;

// ── Evaluator trait ────────────────────────────────────────────────────────

/// Evaluates a game position for use in MCTS.
///
/// Implementations typically wrap a [`PolicyValueNet`](crate::network::PolicyValueNet)
/// but the `mcts` module itself does **not** depend on Burn.
pub trait Evaluator: Send + Sync {
    /// Evaluate `obs` (flat observation vector of length `obs_size`).
    ///
    /// Returns:
    /// - `policy_logits`: one raw logit per action (`action_space` entries).
    ///   Illegal action entries are masked inside the search — no need to
    ///   zero them here.
    /// - `value`: scalar in `(-1, 1)` from **the current player's** perspective.
    fn evaluate(&self, obs: &[f32]) -> (Vec<f32>, f32);
}

// ── Configuration ─────────────────────────────────────────────────────────

/// Hyperparameters for [`run_mcts`].
#[derive(Debug, Clone)]
pub struct MctsConfig {
    /// Number of MCTS simulations per move.  Typical: 50–800.
    pub n_simulations: usize,
    /// PUCT exploration constant `c_puct`.  Typical: 1.0–2.0.
    pub c_puct: f32,
    /// Dirichlet noise concentration α.  Set to `0.0` to disable.
    /// Typical: `0.3` for Chess, `0.1` for large action spaces.
    pub dirichlet_alpha: f32,
    /// Weight of Dirichlet noise mixed into root priors.  Typical: `0.25`.
    pub dirichlet_eps: f32,
    /// Action sampling temperature.  `> 0` = proportional sample, `0` = argmax.
    pub temperature: f32,
}

impl Default for MctsConfig {
    fn default() -> Self {
        Self {
            n_simulations: 200,
            c_puct: 1.5,
            dirichlet_alpha: 0.3,
            dirichlet_eps: 0.25,
            temperature: 1.0,
        }
    }
}

// ── Public interface ───────────────────────────────────────────────────────

/// Run MCTS from `state` and return the populated root [`MctsNode`].
///
/// `state` must be a player-decision node (`P1` or `P2`).
/// Use [`mcts_policy`] and [`select_action`] on the returned root.
///
/// # Panics
///
/// Panics if `env.current_player(state)` is not `P1` or `P2`.
pub fn run_mcts<E: GameEnv>(
    env: &E,
    state: &E::State,
    evaluator: &dyn Evaluator,
    config: &MctsConfig,
    rng: &mut impl Rng,
) -> MctsNode {
    let player_idx = env
        .current_player(state)
        .index()
        .expect("run_mcts called at a non-decision node");

    // ── Expand root (network called once here, not inside the loop) ────────
    let mut root = MctsNode::new(1.0);
    search::expand::<E>(&mut root, state, env, evaluator, player_idx);

    // ── Optional Dirichlet noise for training exploration ──────────────────
    if config.dirichlet_alpha > 0.0 && config.dirichlet_eps > 0.0 {
        search::add_dirichlet_noise(&mut root, config.dirichlet_alpha, config.dirichlet_eps, rng);
    }

    // ── Simulations ────────────────────────────────────────────────────────
    for _ in 0..config.n_simulations {
        search::simulate::<E>(
            &mut root,
            state.clone(),
            env,
            evaluator,
            config,
            rng,
            player_idx,
        );
    }

    root
}

/// Compute the MCTS policy: normalized visit counts at the root.
///
/// Returns a vector of length `action_space` where `policy[a]` is the
/// fraction of simulations that visited action `a`.
pub fn mcts_policy(root: &MctsNode, action_space: usize) -> Vec<f32> {
    let total: f32 = root.children.iter().map(|(_, c)| c.n as f32).sum();
    let mut policy = vec![0.0f32; action_space];
    if total > 0.0 {
        for (a, child) in &root.children {
            policy[*a] = child.n as f32 / total;
        }
    } else if !root.children.is_empty() {
        // n_simulations = 0: uniform over legal actions.
        let uniform = 1.0 / root.children.len() as f32;
        for (a, _) in &root.children {
            policy[*a] = uniform;
        }
    }
    policy
}

/// Select an action index from the root after MCTS.
///
/// * `temperature = 0` — greedy argmax of visit counts.
/// * `temperature > 0` — sample proportionally to `N^(1 / temperature)`.
///
/// # Panics
///
/// Panics if the root has no children.
pub fn select_action(root: &MctsNode, temperature: f32, rng: &mut impl Rng) -> usize {
    assert!(!root.children.is_empty(), "select_action called on a root with no children");
    if temperature <= 0.0 {
        root.children
            .iter()
            .max_by_key(|(_, c)| c.n)
            .map(|(a, _)| *a)
            .unwrap()
    } else {
        let weights: Vec<f32> = root
            .children
            .iter()
            .map(|(_, c)| (c.n as f32).powf(1.0 / temperature))
            .collect();
        let total: f32 = weights.iter().sum();
        let mut r: f32 = rng.random::<f32>() * total;
        for (i, (a, _)) in root.children.iter().enumerate() {
            r -= weights[i];
            if r <= 0.0 {
                return *a;
            }
        }
        root.children.last().map(|(a, _)| *a).unwrap()
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{SeedableRng, rngs::SmallRng};
    use crate::env::Player;

    // ── Minimal deterministic test game ───────────────────────────────────
    //
    // "Countdown" — two players alternate subtracting 1 or 2 from a counter.
    // The player who brings the counter to 0 wins.
    // No chance nodes, two legal actions (0 = -1, 1 = -2).

    #[derive(Clone, Debug)]
    struct CState {
        remaining: u8,
        to_move: usize, // at terminal: last mover (winner)
    }

    #[derive(Clone)]
    struct CountdownEnv;

    impl crate::env::GameEnv for CountdownEnv {
        type State = CState;

        fn new_game(&self) -> CState {
            CState { remaining: 6, to_move: 0 }
        }

        fn current_player(&self, s: &CState) -> Player {
            if s.remaining == 0 {
                Player::Terminal
            } else if s.to_move == 0 {
                Player::P1
            } else {
                Player::P2
            }
        }

        fn legal_actions(&self, s: &CState) -> Vec<usize> {
            if s.remaining >= 2 { vec![0, 1] } else { vec![0] }
        }

        fn apply(&self, s: &mut CState, action: usize) {
            let sub = (action as u8) + 1;
            if s.remaining <= sub {
                s.remaining = 0;
                // to_move stays as winner
            } else {
                s.remaining -= sub;
                s.to_move = 1 - s.to_move;
            }
        }

        fn apply_chance<R: rand::Rng>(&self, _s: &mut CState, _rng: &mut R) {}

        fn observation(&self, s: &CState, _pov: usize) -> Vec<f32> {
            vec![s.remaining as f32 / 6.0, s.to_move as f32]
        }

        fn obs_size(&self) -> usize { 2 }
        fn action_space(&self) -> usize { 2 }

        fn returns(&self, s: &CState) -> Option<[f32; 2]> {
            if s.remaining != 0 { return None; }
            let mut r = [-1.0f32; 2];
            r[s.to_move] = 1.0;
            Some(r)
        }
    }

    // Uniform evaluator: all logits = 0, value = 0.
    // `action_space` must match the environment's `action_space()`.
    struct ZeroEval(usize);
    impl Evaluator for ZeroEval {
        fn evaluate(&self, _obs: &[f32]) -> (Vec<f32>, f32) {
            (vec![0.0f32; self.0], 0.0)
        }
    }

    fn rng() -> SmallRng {
        SmallRng::seed_from_u64(42)
    }

    fn config_n(n: usize) -> MctsConfig {
        MctsConfig {
            n_simulations: n,
            c_puct: 1.5,
            dirichlet_alpha: 0.0, // off for reproducibility
            dirichlet_eps: 0.0,
            temperature: 1.0,
        }
    }

    // ── Visit count tests ─────────────────────────────────────────────────

    #[test]
    fn visit_counts_sum_to_n_simulations() {
        let env = CountdownEnv;
        let state = env.new_game();
        let root = run_mcts(&env, &state, &ZeroEval(2), &config_n(50), &mut rng());
        let total: u32 = root.children.iter().map(|(_, c)| c.n).sum();
        assert_eq!(total, 50, "visit counts must sum to n_simulations");
    }

    #[test]
    fn all_root_children_are_legal() {
        let env = CountdownEnv;
        let state = env.new_game();
        let legal = env.legal_actions(&state);
        let root = run_mcts(&env, &state, &ZeroEval(2), &config_n(30), &mut rng());
        for (a, _) in &root.children {
            assert!(legal.contains(a), "child action {a} is not legal");
        }
    }

    // ── Policy tests ─────────────────────────────────────────────────────

    #[test]
    fn policy_sums_to_one() {
        let env = CountdownEnv;
        let state = env.new_game();
        let root = run_mcts(&env, &state, &ZeroEval(2), &config_n(20), &mut rng());
        let policy = mcts_policy(&root, env.action_space());
        let sum: f32 = policy.iter().sum();
        assert!((sum - 1.0).abs() < 1e-5, "policy sums to {sum}, expected 1.0");
    }

    #[test]
    fn policy_zero_for_illegal_actions() {
        let env = CountdownEnv;
        // remaining = 1 → only action 0 is legal
        let state = CState { remaining: 1, to_move: 0 };
        let root = run_mcts(&env, &state, &ZeroEval(2), &config_n(10), &mut rng());
        let policy = mcts_policy(&root, env.action_space());
        assert_eq!(policy[1], 0.0, "illegal action must have zero policy mass");
    }

    // ── Action selection tests ────────────────────────────────────────────

    #[test]
    fn greedy_selects_most_visited() {
        let env = CountdownEnv;
        let state = env.new_game();
        let root = run_mcts(&env, &state, &ZeroEval(2), &config_n(60), &mut rng());
        let greedy = select_action(&root, 0.0, &mut rng());
        let most_visited = root.children.iter().max_by_key(|(_, c)| c.n).map(|(a, _)| *a).unwrap();
        assert_eq!(greedy, most_visited);
    }

    #[test]
    fn temperature_sampling_stays_legal() {
        let env = CountdownEnv;
        let state = env.new_game();
        let legal = env.legal_actions(&state);
        let mut r = rng();
        let root = run_mcts(&env, &state, &ZeroEval(2), &config_n(30), &mut r);
        for _ in 0..20 {
            let a = select_action(&root, 1.0, &mut r);
            assert!(legal.contains(&a), "sampled action {a} is not legal");
        }
    }

    // ── Zero-simulation edge case ─────────────────────────────────────────

    #[test]
    fn zero_simulations_uniform_policy() {
        let env = CountdownEnv;
        let state = env.new_game();
        let root = run_mcts(&env, &state, &ZeroEval(2), &config_n(0), &mut rng());
        let policy = mcts_policy(&root, env.action_space());
        // With 0 simulations, fallback is uniform over the 2 legal actions.
        let sum: f32 = policy.iter().sum();
        assert!((sum - 1.0).abs() < 1e-5);
    }

    // ── Root value ────────────────────────────────────────────────────────

    #[test]
    fn root_q_in_valid_range() {
        let env = CountdownEnv;
        let state = env.new_game();
        let root = run_mcts(&env, &state, &ZeroEval(2), &config_n(40), &mut rng());
        let q = root.q();
        assert!(q >= -1.0 && q <= 1.0, "root Q={q} outside [-1, 1]");
    }

    // ── Integration: run on a real Trictrac game ──────────────────────────

    #[test]
    fn no_panic_on_trictrac_state() {
        use crate::env::TrictracEnv;

        let env = TrictracEnv;
        let mut state = env.new_game();
        let mut r = rng();

        // Advance past the initial chance node to reach a decision node.
        while env.current_player(&state).is_chance() {
            env.apply_chance(&mut state, &mut r);
        }

        if env.current_player(&state).is_terminal() {
            return; // unlikely but safe
        }

        let config = MctsConfig {
            n_simulations: 5, // tiny for speed
            dirichlet_alpha: 0.0,
            dirichlet_eps: 0.0,
            ..MctsConfig::default()
        };

        let root = run_mcts(&env, &state, &ZeroEval(514), &config, &mut r);
        assert!(root.n > 0);
        let total: u32 = root.children.iter().map(|(_, c)| c.n).sum();
        assert_eq!(total, 5);
    }
}
