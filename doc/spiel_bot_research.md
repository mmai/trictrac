# spiel_bot: Rust-native AlphaZero Training Crate for Trictrac

## 0. Context and Scope

The existing `bot` crate already uses **Burn 0.20** with the `burn-rl` library
(DQN, PPO, SAC) against a random opponent. It uses the old 36-value `to_vec()`
encoding and handles only the `Move`/`HoldOrGoChoice` stages, outsourcing every
other stage to an inline random-opponent loop.

`spiel_bot` is a new workspace crate that replaces the OpenSpiel C++ dependency
for **self-play training**. Its goals:

- Provide a minimal, clean **game-environment abstraction** (the "Rust OpenSpiel")
  that works with Trictrac's multi-stage turn model and stochastic dice.
- Implement **AlphaZero** (MCTS + policy-value network + self-play replay buffer)
  as the first algorithm.
- Remain **modular**: adding DQN or PPO later requires only a new
  `impl Algorithm for Dqn` without touching the environment or network layers.
- Use the 217-value `to_tensor()` encoding and `get_valid_actions()` from
  `trictrac-store`.

---

## 1. Library Landscape

### 1.1 Neural Network Frameworks

| Crate           | Autodiff           | GPU                   | Pure Rust                    | Maturity                         | Notes                              |
| --------------- | ------------------ | --------------------- | ---------------------------- | -------------------------------- | ---------------------------------- |
| **Burn 0.20**   | yes                | wgpu / CUDA (via tch) | yes                          | active, breaking API every minor | already used in `bot/`             |
| **tch-rs 0.17** | yes (via LibTorch) | CUDA / MPS            | no (requires LibTorch ~2 GB) | very mature                      | full PyTorch; best raw performance |
| **Candle 0.8**  | partial            | CUDA                  | yes                          | stable, HuggingFace-backed       | better for inference than training |
| ndarray alone   | no                 | no                    | yes                          | mature                           | array ops only; no autograd        |

**Recommendation: Burn** — consistent with the existing `bot/` crate, no C++
runtime needed, the `ndarray` backend is sufficient for CPU training and can
switch to `wgpu` (GPU without CUDA driver) or `tch` (LibTorch, fastest) by
changing one type alias.

`tch-rs` would be the best choice for raw training throughput (it is the most
battle-tested backend for RL) but adds a 2 GB LibTorch download and breaks the
pure-Rust constraint. If training speed becomes the bottleneck after prototyping,
switching `spiel_bot` to `tch-rs` is a one-line backend swap.

### 1.2 Other Key Crates

| Crate                | Role                                                              |
| -------------------- | ----------------------------------------------------------------- |
| `rand 0.9`           | dice sampling, replay buffer shuffling (already in store)         |
| `rayon`              | parallel self-play: `(0..n_games).into_par_iter().map(play_game)` |
| `crossbeam-channel`  | optional producer/consumer pipeline (self-play workers → trainer) |
| `serde / serde_json` | replay buffer snapshots, checkpoint metadata                      |
| `anyhow`             | error propagation (already used everywhere)                       |
| `indicatif`          | training progress bars                                            |
| `tracing`            | structured logging per episode/iteration                          |

### 1.3 What `burn-rl` Provides (and Does Not)

The external `burn-rl` crate (from `github.com/yunjhongwu/burn-rl-examples`)
provides DQN, PPO, SAC agents via a `burn_rl::base::{Environment, State, Action}`
trait. It does **not** provide:

- MCTS or any tree-search algorithm
- Two-player self-play
- Legal action masking during training
- Chance-node handling

For AlphaZero, `burn-rl` is not useful. The `spiel_bot` crate will define its
own (simpler, more targeted) traits and implement MCTS from scratch.

---

## 2. Trictrac-Specific Design Constraints

### 2.1 Multi-Stage Turn Model

A Trictrac turn passes through up to six `TurnStage` values. Only two involve
genuine player choice:

| TurnStage        | Node type                       | Handler                         |
| ---------------- | ------------------------------- | ------------------------------- |
| `RollDice`       | Forced (player initiates roll)  | Auto-apply `GameEvent::Roll`    |
| `RollWaiting`    | **Chance** (dice outcome)       | Sample dice, apply `RollResult` |
| `MarkPoints`     | Forced (score is deterministic) | Auto-apply `GameEvent::Mark`    |
| `HoldOrGoChoice` | **Player decision**             | MCTS / policy network           |
| `Move`           | **Player decision**             | MCTS / policy network           |
| `MarkAdvPoints`  | Forced                          | Auto-apply `GameEvent::Mark`    |

The environment wrapper advances through forced/chance stages automatically so
that from the algorithm's perspective every node it sees is a genuine player
decision.

### 2.2 Stochastic Dice in MCTS

AlphaZero was designed for deterministic games (Chess, Go). For Trictrac, dice
introduce stochasticity. Three approaches exist:

**A. Outcome sampling (recommended)**
During each MCTS simulation, when a chance node is reached, sample one dice
outcome at random and continue. After many simulations the expected value
converges. This is the approach used by OpenSpiel's MCTS for stochastic games
and requires no changes to the standard PUCT formula.

**B. Chance-node averaging (expectimax)**
At each chance node, expand all 21 unique dice pairs weighted by their
probability (doublet: 1/36 each × 6; non-doublet: 2/36 each × 15). This is
exact but multiplies the branching factor by ~21 at every dice roll, making it
prohibitively expensive.

**C. Condition on dice in the observation (current approach)**
Dice values are already encoded at indices [192–193] of `to_tensor()`. The
network naturally conditions on the rolled dice when it evaluates a position.
MCTS only runs on player-decision nodes _after_ the dice have been sampled;
chance nodes are bypassed by the environment wrapper (approach A). The policy
and value heads learn to play optimally given any dice pair.

**Use approach A + C together**: the environment samples dice automatically
(chance node bypass), and the 217-dim tensor encodes the dice so the network
can exploit them.

### 2.3 Perspective / Mirroring

All move rules and tensor encoding are defined from White's perspective.
`to_tensor()` must always be called after mirroring the state for Black.
The environment wrapper handles this transparently: every observation returned
to an algorithm is already in the active player's perspective.

### 2.4 Legal Action Masking

A crucial difference from the existing `bot/` code: instead of penalizing
invalid actions with `ERROR_REWARD`, the policy head logits are **masked**
before softmax — illegal action logits are set to `-inf`. This prevents the
network from wasting capacity on illegal moves and eliminates the need for the
penalty-reward hack.

---

## 3. Proposed Crate Architecture

```
spiel_bot/
├── Cargo.toml
└── src/
    ├── lib.rs               # re-exports; feature flags: "alphazero", "dqn", "ppo"
    │
    ├── env/
    │   ├── mod.rs           # GameEnv trait — the minimal OpenSpiel interface
    │   └── trictrac.rs      # TrictracEnv: impl GameEnv using trictrac-store
    │
    ├── mcts/
    │   ├── mod.rs           # MctsConfig, run_mcts() entry point
    │   ├── node.rs          # MctsNode (visit count, W, prior, children)
    │   └── search.rs        # simulate(), backup(), select_action()
    │
    ├── network/
    │   ├── mod.rs           # PolicyValueNet trait
    │   └── resnet.rs        # Burn ResNet: Linear + residual blocks + two heads
    │
    ├── alphazero/
    │   ├── mod.rs           # AlphaZeroConfig
    │   ├── selfplay.rs      # generate_episode() -> Vec<TrainSample>
    │   ├── replay.rs        # ReplayBuffer (VecDeque, capacity, shuffle)
    │   └── trainer.rs       # training loop: selfplay → sample → loss → update
    │
    └── agent/
        ├── mod.rs           # Agent trait
        ├── random.rs        # RandomAgent (baseline)
        └── mcts_agent.rs    # MctsAgent: uses trained network for inference
```

Future algorithms slot in without touching the above:

```
    ├── dqn/                 # (future) DQN: impl Algorithm + own replay buffer
    └── ppo/                 # (future) PPO: impl Algorithm + rollout buffer
```

---

## 4. Core Traits

### 4.1 `GameEnv` — the minimal OpenSpiel interface

```rust
use rand::Rng;

/// Who controls the current node.
pub enum Player {
    P1,       // player index 0
    P2,       // player index 1
    Chance,   // dice roll
    Terminal, // game over
}

pub trait GameEnv: Clone + Send + Sync + 'static {
    type State: Clone + Send + Sync;

    /// Fresh game state.
    fn new_game(&self) -> Self::State;

    /// Who acts at this node.
    fn current_player(&self, s: &Self::State) -> Player;

    /// Legal action indices (always in [0, action_space())).
    /// Empty only at Terminal nodes.
    fn legal_actions(&self, s: &Self::State) -> Vec<usize>;

    /// Apply a player action (must be legal).
    fn apply(&self, s: &mut Self::State, action: usize);

    /// Advance a Chance node by sampling dice; no-op at non-Chance nodes.
    fn apply_chance(&self, s: &mut Self::State, rng: &mut impl Rng);

    /// Observation tensor from `pov`'s perspective (0 or 1).
    /// Returns 217 f32 values for Trictrac.
    fn observation(&self, s: &Self::State, pov: usize) -> Vec<f32>;

    /// Flat observation size (217 for Trictrac).
    fn obs_size(&self) -> usize;

    /// Total action-space size (514 for Trictrac).
    fn action_space(&self) -> usize;

    /// Game outcome per player, or None if not Terminal.
    /// Values in [-1, 1]: +1 = win, -1 = loss, 0 = draw.
    fn returns(&self, s: &Self::State) -> Option<[f32; 2]>;
}
```

### 4.2 `PolicyValueNet` — neural network interface

```rust
use burn::prelude::*;

pub trait PolicyValueNet<B: Backend>: Send + Sync {
    /// Forward pass.
    /// `obs`: [batch, obs_size] tensor.
    /// Returns: (policy_logits [batch, action_space], value [batch]).
    fn forward(&self, obs: Tensor<B, 2>) -> (Tensor<B, 2>, Tensor<B, 1>);

    /// Save weights to `path`.
    fn save(&self, path: &std::path::Path) -> anyhow::Result<()>;

    /// Load weights from `path`.
    fn load(path: &std::path::Path) -> anyhow::Result<Self>
    where
        Self: Sized;
}
```

### 4.3 `Agent` — player policy interface

```rust
pub trait Agent: Send {
    /// Select an action index given the current game state observation.
    /// `legal`: mask of valid action indices.
    fn select_action(&mut self, obs: &[f32], legal: &[usize]) -> usize;
}
```

---

## 5. MCTS Implementation

### 5.1 Node

```rust
pub struct MctsNode {
    n: u32,                                // visit count N(s, a)
    w: f32,                                // sum of backed-up values W(s, a)
    p: f32,                                // prior from policy head P(s, a)
    children: Vec<(usize, MctsNode)>,      // (action_idx, child)
    is_expanded: bool,
}

impl MctsNode {
    pub fn q(&self) -> f32 {
        if self.n == 0 { 0.0 } else { self.w / self.n as f32 }
    }

    /// PUCT score used for selection.
    pub fn puct(&self, parent_n: u32, c_puct: f32) -> f32 {
        self.q() + c_puct * self.p * (parent_n as f32).sqrt() / (1.0 + self.n as f32)
    }
}
```

### 5.2 Simulation Loop

One MCTS simulation (for deterministic decision nodes):

```
1. SELECTION   — traverse from root, always pick child with highest PUCT,
                 auto-advancing forced/chance nodes via env.apply_chance().
2. EXPANSION   — at first unvisited leaf: call network.forward(obs) to get
                 (policy_logits, value).  Mask illegal actions, softmax
                 the remaining logits → priors P(s,a) for each child.
3. BACKUP      — propagate -value up the tree (negate at each level because
                 perspective alternates between P1 and P2).
```

After `n_simulations` iterations, action selection at the root:

```rust
// During training: sample proportional to N^(1/temperature)
// During evaluation: argmax N
fn select_action(root: &MctsNode, temperature: f32) -> usize { ... }
```

### 5.3 Configuration

```rust
pub struct MctsConfig {
    pub n_simulations: usize,   // e.g. 200
    pub c_puct: f32,            // exploration constant, e.g. 1.5
    pub dirichlet_alpha: f32,   // root noise for exploration, e.g. 0.3
    pub dirichlet_eps: f32,     // noise weight, e.g. 0.25
    pub temperature: f32,       // action sampling temperature (anneals to 0)
}
```

### 5.4 Handling Chance Nodes Inside MCTS

When simulation reaches a Chance node (dice roll), the environment automatically
samples dice and advances to the next decision node. The MCTS tree does **not**
branch on dice outcomes — it treats the sampled outcome as the state. This
corresponds to "outcome sampling" (approach A from §2.2). Because each
simulation independently samples dice, the Q-values at player nodes converge to
their expected value over many simulations.

---

## 6. Network Architecture

### 6.1 ResNet Policy-Value Network

A single trunk with residual blocks, then two heads:

```
Input: [batch, 217]
   ↓
Linear(217 → 512) + ReLU
   ↓
ResBlock × 4   (Linear(512→512) + BN + ReLU + Linear(512→512) + BN + skip + ReLU)
   ↓ trunk output [batch, 512]
   ├─ Policy head: Linear(512 → 514) → logits  (masked softmax at use site)
   └─ Value head:  Linear(512 → 1)   → tanh    (output in [-1, 1])
```

Burn implementation sketch:

```rust
#[derive(Module, Debug)]
pub struct TrictracNet<B: Backend> {
    input:       Linear<B>,
    res_blocks:  Vec<ResBlock<B>>,
    policy_head: Linear<B>,
    value_head:  Linear<B>,
}

impl<B: Backend> TrictracNet<B> {
    pub fn forward(&self, obs: Tensor<B, 2>)
        -> (Tensor<B, 2>, Tensor<B, 1>)
    {
        let x = activation::relu(self.input.forward(obs));
        let x = self.res_blocks.iter().fold(x, |x, b| b.forward(x));
        let policy = self.policy_head.forward(x.clone()); // raw logits
        let value  = activation::tanh(self.value_head.forward(x))
                         .squeeze(1);
        (policy, value)
    }
}
```

A simpler MLP (no residual blocks) is sufficient for a first version and much
faster to train: `Linear(217→512) + ReLU + Linear(512→256) + ReLU` then two
heads.

### 6.2 Loss Function

```
L = MSE(value_pred, z)
  + CrossEntropy(policy_logits_masked, π_mcts)
  - c_l2 * L2_regularization
```

Where:

- `z` = game outcome (±1) from the active player's perspective
- `π_mcts` = normalized MCTS visit counts at the root (the policy target)
- Legal action masking is applied before computing CrossEntropy

---

## 7. AlphaZero Training Loop

```
INIT
  network ← random weights
  replay  ← empty ReplayBuffer(capacity = 100_000)

LOOP forever:
  ── Self-play phase ──────────────────────────────────────────────
  (parallel with rayon, n_workers games at once)
  for each game:
    state ← env.new_game()
    samples = []
    while not terminal:
      advance forced/chance nodes automatically
      obs ← env.observation(state, current_player)
      legal ← env.legal_actions(state)
      π, root_value ← mcts.run(state, network, config)
      action ← sample from π (with temperature)
      samples.push((obs, π, current_player))
      env.apply(state, action)
    z ← env.returns(state)          // final scores
    for (obs, π, player) in samples:
      replay.push(TrainSample { obs, policy: π, value: z[player] })

  ── Training phase ───────────────────────────────────────────────
  for each gradient step:
    batch ← replay.sample(batch_size)
    (policy_logits, value_pred) ← network.forward(batch.obs)
    loss ← mse(value_pred, batch.value) + xent(policy_logits, batch.policy)
    optimizer.step(loss.backward())

  ── Evaluation (every N iterations) ─────────────────────────────
  win_rate ← evaluate(network_new vs network_prev, n_eval_games)
  if win_rate > 0.55: save checkpoint
```

### 7.1 Replay Buffer

```rust
pub struct TrainSample {
    pub obs:    Vec<f32>,  // 217 values
    pub policy: Vec<f32>,  // 514 values (normalized MCTS visit counts)
    pub value:  f32,       // game outcome ∈ {-1, 0, +1}
}

pub struct ReplayBuffer {
    data:     VecDeque<TrainSample>,
    capacity: usize,
}

impl ReplayBuffer {
    pub fn push(&mut self, s: TrainSample) {
        if self.data.len() == self.capacity { self.data.pop_front(); }
        self.data.push_back(s);
    }

    pub fn sample(&self, n: usize, rng: &mut impl Rng) -> Vec<&TrainSample> {
        // sample without replacement
    }
}
```

### 7.2 Parallelism Strategy

Self-play is embarrassingly parallel (each game is independent):

```rust
let samples: Vec<TrainSample> = (0..n_games)
    .into_par_iter()                          // rayon
    .flat_map(|_| generate_episode(&env, &network, &mcts_config))
    .collect();
```

Note: Burn's `NdArray` backend is not `Send` by default when using autodiff.
Self-play uses inference-only (no gradient tape), so a `NdArray<f32>` backend
(without `Autodiff` wrapper) is `Send`. Training runs on the main thread with
`Autodiff<NdArray<f32>>`.

For larger scale, a producer-consumer architecture (crossbeam-channel) separates
self-play workers from the training thread, allowing continuous data generation
while the GPU trains.

---

## 8. `TrictracEnv` Implementation Sketch

```rust
use trictrac_store::{
    training_common::{get_valid_actions, TrictracAction, ACTION_SPACE_SIZE},
    Dice, DiceRoller, GameEvent, GameState, Stage, TurnStage,
};

#[derive(Clone)]
pub struct TrictracEnv;

impl GameEnv for TrictracEnv {
    type State = GameState;

    fn new_game(&self) -> GameState {
        GameState::new_with_players("P1", "P2")
    }

    fn current_player(&self, s: &GameState) -> Player {
        match s.stage {
            Stage::Ended => Player::Terminal,
            _ => match s.turn_stage {
                TurnStage::RollWaiting => Player::Chance,
                _ => if s.active_player_id == 1 { Player::P1 } else { Player::P2 },
            },
        }
    }

    fn legal_actions(&self, s: &GameState) -> Vec<usize> {
        let view = if s.active_player_id == 2 { s.mirror() } else { s.clone() };
        get_valid_action_indices(&view).unwrap_or_default()
    }

    fn apply(&self, s: &mut GameState, action_idx: usize) {
        // advance all forced/chance nodes first, then apply the player action
        self.advance_forced(s);
        let needs_mirror = s.active_player_id == 2;
        let view = if needs_mirror { s.mirror() } else { s.clone() };
        if let Some(event) = TrictracAction::from_action_index(action_idx)
            .and_then(|a| a.to_event(&view))
            .map(|e| if needs_mirror { e.get_mirror(false) } else { e })
        {
            let _ = s.consume(&event);
        }
        // advance any forced stages that follow
        self.advance_forced(s);
    }

    fn apply_chance(&self, s: &mut GameState, rng: &mut impl Rng) {
        // RollDice → RollWaiting
        let _ = s.consume(&GameEvent::Roll { player_id: s.active_player_id });
        // RollWaiting → next stage
        let dice = Dice { values: (rng.random_range(1u8..=6), rng.random_range(1u8..=6)) };
        let _ = s.consume(&GameEvent::RollResult { player_id: s.active_player_id, dice });
        self.advance_forced(s);
    }

    fn observation(&self, s: &GameState, pov: usize) -> Vec<f32> {
        if pov == 0 { s.to_tensor() } else { s.mirror().to_tensor() }
    }

    fn obs_size(&self) -> usize { 217 }
    fn action_space(&self) -> usize { ACTION_SPACE_SIZE }

    fn returns(&self, s: &GameState) -> Option<[f32; 2]> {
        if s.stage != Stage::Ended { return None; }
        // Convert hole+point scores to ±1 outcome
        let s1 = s.players.get(&1).map(|p| p.holes as i32 * 12 + p.points as i32).unwrap_or(0);
        let s2 = s.players.get(&2).map(|p| p.holes as i32 * 12 + p.points as i32).unwrap_or(0);
        Some(match s1.cmp(&s2) {
            std::cmp::Ordering::Greater => [ 1.0, -1.0],
            std::cmp::Ordering::Less    => [-1.0,  1.0],
            std::cmp::Ordering::Equal   => [ 0.0,  0.0],
        })
    }
}

impl TrictracEnv {
    /// Advance through all forced (non-decision, non-chance) stages.
    fn advance_forced(&self, s: &mut GameState) {
        use trictrac_store::PointsRules;
        loop {
            match s.turn_stage {
                TurnStage::MarkPoints | TurnStage::MarkAdvPoints => {
                    // Scoring is deterministic; compute and apply automatically.
                    let color = s.player_color_by_id(&s.active_player_id)
                        .unwrap_or(trictrac_store::Color::White);
                    let drc = s.players.get(&s.active_player_id)
                        .map(|p| p.dice_roll_count).unwrap_or(0);
                    let pr = PointsRules::new(&color, &s.board, s.dice);
                    let pts = pr.get_points(drc);
                    let points = if s.turn_stage == TurnStage::MarkPoints { pts.0 } else { pts.1 };
                    let _ = s.consume(&GameEvent::Mark {
                        player_id: s.active_player_id, points,
                    });
                }
                TurnStage::RollDice => {
                    // RollDice is a forced "initiate roll" action with no real choice.
                    let _ = s.consume(&GameEvent::Roll { player_id: s.active_player_id });
                }
                _ => break,
            }
        }
    }
}
```

---

## 9. Cargo.toml Changes

### 9.1 Add `spiel_bot` to the workspace

```toml
# Cargo.toml (workspace root)
[workspace]
resolver = "2"
members = ["client_cli", "bot", "store", "spiel_bot"]
```

### 9.2 `spiel_bot/Cargo.toml`

```toml
[package]
name    = "spiel_bot"
version = "0.1.0"
edition = "2021"

[features]
default    = ["alphazero"]
alphazero  = []
# dqn      = []   # future
# ppo      = []   # future

[dependencies]
trictrac-store = { path = "../store" }
anyhow  = "1"
rand    = "0.9"
rayon   = "1"
serde   = { version = "1", features = ["derive"] }
serde_json = "1"

# Burn: NdArray for pure-Rust CPU training
# Replace NdArray with Wgpu or Tch for GPU.
burn = { version = "0.20", features = ["ndarray", "autodiff"] }

# Optional: progress display and structured logging
indicatif = "0.17"
tracing   = "0.1"

[[bin]]
name = "az_train"
path = "src/bin/az_train.rs"

[[bin]]
name = "az_eval"
path = "src/bin/az_eval.rs"
```

---

## 10. Comparison: `bot` crate vs `spiel_bot`

| Aspect           | `bot` (existing)            | `spiel_bot` (proposed)                       |
| ---------------- | --------------------------- | -------------------------------------------- |
| State encoding   | 36 i8 `to_vec()`            | 217 f32 `to_tensor()`                        |
| Algorithms       | DQN, PPO, SAC via `burn-rl` | AlphaZero (MCTS)                             |
| Opponent         | hardcoded random            | self-play                                    |
| Invalid actions  | penalise with reward        | legal action mask (no penalty)               |
| Dice handling    | inline sampling in step()   | `Chance` node in `GameEnv` trait             |
| Stochastic turns | manual per-stage code       | `advance_forced()` in env wrapper            |
| Burn dep         | yes (0.20)                  | yes (0.20), same backend                     |
| `burn-rl` dep    | yes                         | no                                           |
| C++ dep          | no                          | no                                           |
| Python dep       | no                          | no                                           |
| Modularity       | one entry point per algo    | `GameEnv` + `Agent` traits; algo is a plugin |

The two crates are **complementary**: `bot` is a working DQN/PPO baseline;
`spiel_bot` adds MCTS-based self-play on top of a cleaner abstraction. The
`TrictracEnv` in `spiel_bot` can also back-fill into `bot` if desired (just
replace `TrictracEnvironment` with `TrictracEnv`).

---

## 11. Implementation Order

1. **`env/`**: `GameEnv` trait + `TrictracEnv` + unit tests (run a random game
   through the trait, verify terminal state and returns).
2. **`network/`**: `PolicyValueNet` trait + MLP stub (no residual blocks yet) +
   Burn forward/backward pass test with dummy data.
3. **`mcts/`**: `MctsNode` + `simulate()` + `select_action()` + property tests
   (visit counts sum to `n_simulations`, legal mask respected).
4. **`alphazero/`**: `generate_episode()` + `ReplayBuffer` + training loop stub
   (one iteration, check loss decreases).
5. **Integration test**: run 100 self-play games with a tiny network (1 res block,
   64 hidden units), verify the training loop completes without panics.
6. **Benchmarks**: measure games/second, steps/second (target: ≥ 500 games/s
   on CPU, consistent with `random_game` throughput).
7. **Upgrade network**: 4 residual blocks, 512 hidden units; schedule
   hyperparameter sweep.
8. **`az_eval` binary**: play `MctsAgent` (trained) vs `RandomAgent`, report
   win rate every checkpoint.

---

## 12. Key Open Questions

1. **Scoring as returns**: Trictrac scores (holes × 12 + points) are unbounded.
   AlphaZero needs ±1 returns. Simple option: win/loss at game end (whoever
   scored more holes). Better option: normalize the score margin. The final
   choice affects how the value head is trained.

2. **Episode length**: Trictrac games average ~600 steps (`random_game` data).
   MCTS with 200 simulations per step means ~120k network evaluations per game.
   At batch inference this is feasible on CPU; on GPU it becomes fast. Consider
   limiting `n_simulations` to 50–100 for early training.

3. **`HoldOrGoChoice` strategy**: The `Go` action resets the board (new relevé).
   This is a long-horizon decision that AlphaZero handles naturally via MCTS
   lookahead, but needs careful value normalization (a "Go" restarts scoring
   within the same game).

4. **`burn-rl` reuse**: The existing DQN/PPO code in `bot/` could be migrated
   to use `TrictracEnv` from `spiel_bot`, consolidating the environment logic.
   This is optional but reduces code duplication.

5. **Dirichlet noise parameters**: Standard AlphaZero uses α = 0.3 for Chess,
   0.03 for Go. For Trictrac with action space 514, empirical tuning is needed.
   A reasonable starting point: α = 10 / mean_legal_actions ≈ 0.1.

## Implementation results

All benchmarks compile and run. Here's the complete results table:

| Group   | Benchmark               | Time                  |
| ------- | ----------------------- | --------------------- |
| env     | apply_chance            | 3.87 µs               |
|         | legal_actions           | 1.91 µs               |
|         | observation (to_tensor) | 341 ns                |
|         | random_game (baseline)  | 3.55 ms → 282 games/s |
| network | mlp_b1 hidden=64        | 94.9 µs               |
|         | mlp_b32 hidden=64       | 141 µs                |
|         | mlp_b1 hidden=256       | 352 µs                |
|         | mlp_b32 hidden=256      | 479 µs                |
| mcts    | zero_eval n=1           | 6.8 µs                |
|         | zero_eval n=5           | 23.9 µs               |
|         | zero_eval n=20          | 90.9 µs               |
|         | mlp64 n=1               | 203 µs                |
|         | mlp64 n=5               | 622 µs                |
|         | mlp64 n=20              | 2.30 ms               |
| episode | trictrac n=1            | 51.8 ms → 19 games/s  |
|         | trictrac n=2            | 145 ms → 7 games/s    |
| train   | mlp64 Adam b=16         | 1.93 ms               |
|         | mlp64 Adam b=64         | 2.68 ms               |

Key observations:

- random_game baseline: 282 games/s (short of the ≥ 500 target — game state ops dominate at 3.9 µs/apply_chance, ~600 steps/game)
- observation (217-value tensor): only 341 ns — not a bottleneck
- legal_actions: 1.9 µs — well optimised
- Network (MLP hidden=64): 95 µs per call — the dominant MCTS cost; with n=1 each episode step costs ~200 µs
- Tree traversal (zero_eval): only 6.8 µs for n=1 — MCTS overhead is minimal
- Full episode n=1: 51.8 ms (19 games/s); the 95 µs × ~2 calls × ~600 moves accounts for most of it
- Training: 2.7 ms/step at batch=64 → 370 steps/s

### Summary of Step 8

spiel_bot/src/bin/az_eval.rs — a self-contained evaluation binary:

- CLI flags: --checkpoint, --arch mlp|resnet, --hidden, --n-games, --n-sim, --seed, --c-puct
- No checkpoint → random weights (useful as a sanity baseline — should converge toward 50%)
- Game loop: alternates MctsAgent as P1 / P2 against a RandomAgent, n_games per side
- MctsAgent: run_mcts + greedy select_action (temperature=0, no Dirichlet noise)
- Output: win/draw/loss per side + combined decisive win rate

Typical usage after training:
cargo run -p spiel_bot --bin az_eval --release -- \
 --checkpoint checkpoints/iter_100.mpk --arch resnet --n-games 200 --n-sim 100

### az_train

#### Fresh MLP training (default: 100 iters, 10 games, 100 sims, save every 10)

cargo run -p spiel_bot --bin az_train --release

#### ResNet, more games, custom output dir

cargo run -p spiel_bot --bin az_train --release -- \
 --arch resnet --n-iter 200 --n-games 20 --n-sim 100 \
 --save-every 20 --out checkpoints/

#### Resume from iteration 50

cargo run -p spiel_bot --bin az_train --release -- \
 --resume checkpoints/iter_0050.mpk --arch mlp --n-iter 50

What the binary does each iteration:

1. Calls model.valid() to get a zero-overhead inference copy for self-play
2. Runs n_games episodes via generate_episode (temperature=1 for first --temp-drop moves, then greedy)
3. Pushes samples into a ReplayBuffer (capacity --replay-cap)
4. Runs n_train gradient steps via train_step with cosine LR annealing from --lr down to --lr-min
5. Saves a .mpk checkpoint every --save-every iterations and always on the last
