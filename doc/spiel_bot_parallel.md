Part B — Batched MCTS leaf evaluation

Goal: during a single game's MCTS, accumulate eval_batch_size leaf observations and call the network once with a [B, obs_size] tensor instead of B separate [1, obs_size] calls.

Step B1 — Add evaluate_batch to the Evaluator trait (mcts/mod.rs)

pub trait Evaluator: Send + Sync {
fn evaluate(&self, obs: &[f32]) -> (Vec<f32>, f32);

      /// Evaluate a batch of observations at once.  Default falls back to
      /// sequential calls; backends override this for efficiency.
      fn evaluate_batch(&self, obs_batch: &[&[f32]]) -> Vec<(Vec<f32>, f32)> {
          obs_batch.iter().map(|obs| self.evaluate(obs)).collect()
      }

}

Step B2 — Implement evaluate_batch in BurnEvaluator (selfplay.rs)

Stack all observations into one [B, obs_size] tensor, call model.forward once, split the output tensors back into B rows.

fn evaluate_batch(&self, obs_batch: &[&[f32]]) -> Vec<(Vec<f32>, f32)> {
let b = obs_batch.len();
let obs_size = obs_batch[0].len();
let flat: Vec<f32> = obs_batch.iter().flat_map(|o| o.iter().copied()).collect();
let obs_tensor = Tensor::<B, 2>::from_data(TensorData::new(flat, [b, obs_size]), &self.device);
let (policy_tensor, value_tensor) = self.model.forward(obs_tensor);
let policies: Vec<f32> = policy_tensor.into_data().to_vec().unwrap();
let values: Vec<f32> = value_tensor.into_data().to_vec().unwrap();
let action_size = policies.len() / b;
(0..b).map(|i| {
(policies[i * action_size..(i + 1) * action_size].to_vec(), values[i])
}).collect()
}

Step B3 — Add eval_batch_size to MctsConfig

pub struct MctsConfig {
// ... existing fields ...
/// Number of leaves to batch per network call. 1 = no batching (current behaviour).
pub eval_batch_size: usize,
}

Default: 1 (backwards-compatible).

Step B4 — Make the simulation iterative (mcts/search.rs)

The current simulate is recursive. For batching we need to split it into two phases:

descend (pure tree traversal — no network call):

- Traverse from root following PUCT, advancing through chance nodes with apply_chance.
- Stop when reaching: an unvisited leaf, a terminal node, or a node whose child was already selected by another in-flight descent (virtual loss in effect).
- Return a LeafWork { path: Vec<usize>, state: E::State, player_idx: usize, kind: LeafKind } where path is the sequence of child indices taken from the root and kind is NeedsEval | Terminal(value) | CrossedChance.
- Apply virtual loss along the path during descent: n += 1, w -= 1 at every node traversed. This steers the next concurrent descent away from the same path.

ascend (backup — no network call):

- Given the path and the evaluated value, walk back up the path re-negating at player-boundary transitions.
- Undo the virtual loss: n -= 1, w += 1, then add the real update: n += 1, w += value.

Step B5 — Add run_mcts_batched to mcts/mod.rs

The new entry point, called by run_mcts when config.eval_batch_size > 1:

expand root (1 network call)
optionally add Dirichlet noise

for round in 0..(n*simulations / batch_size):
leaves = []
for * in 0..batch_size:
leaf = descend(root, state, env, rng)
leaves.push(leaf)

      obs_batch = [env.observation(leaf.state, leaf.player) for leaf in leaves
                   where leaf.kind == NeedsEval]
      results = evaluator.evaluate_batch(obs_batch)

      for (leaf, result) in zip(leaves, results):
          expand the leaf node (insert children from result.policy)
          ascend(root, leaf.path, result.value, leaf.player_idx)
          // ascend also handles terminal and crossed-chance leaves

// handle remainder: n_simulations % batch_size

run_mcts becomes a thin dispatcher:
if config.eval_batch_size <= 1 {
// existing path (unchanged)
} else {
run_mcts_batched(...)
}

Step B6 — CLI flag in az_train.rs

--eval-batch N default: 8 Leaf batch size for MCTS network calls

---

Summary of file changes

┌───────────────────────────┬──────────────────────────────────────────────────────────────────────────┐
│ File │ Changes │
├───────────────────────────┼──────────────────────────────────────────────────────────────────────────┤
│ spiel_bot/Cargo.toml │ add rayon │
├───────────────────────────┼──────────────────────────────────────────────────────────────────────────┤
│ src/mcts/mod.rs │ evaluate_batch on trait; eval_batch_size in MctsConfig; run_mcts_batched │
├───────────────────────────┼──────────────────────────────────────────────────────────────────────────┤
│ src/mcts/search.rs │ descend (iterative, virtual loss); ascend (backup path); expand_at_path │
├───────────────────────────┼──────────────────────────────────────────────────────────────────────────┤
│ src/alphazero/selfplay.rs │ BurnEvaluator::evaluate_batch │
├───────────────────────────┼──────────────────────────────────────────────────────────────────────────┤
│ src/bin/az_train.rs │ parallel game loop (rayon); --eval-batch flag │
└───────────────────────────┴──────────────────────────────────────────────────────────────────────────┘

Key design constraint

Parts A and B are independent and composable:

- A only touches the outer game loop.
- B only touches the inner MCTS per game.
- Together: each of the N parallel games runs its own batched MCTS tree entirely independently with no shared state.
