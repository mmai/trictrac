# Trictrac — Research Notes: Engine & OpenSpiel Integration

> Generated from a deep read of `trictrac/store/src/` and `forks/open_spiel/open_spiel/python/games/trictrac.py`.

---

## 1. Architecture Overview

The project connects two codebases through a compiled Python extension:

```
┌─────────────────────────────────────┐
│  trictrac/store/  (Rust crate)      │
│  - full game rules engine           │
│  - pyengine.rs → PyO3 bindings      │
│  compiled by maturin → .whl         │
└──────────────┬──────────────────────┘
               │  import trictrac_store
┌──────────────▼──────────────────────┐
│  forks/open_spiel/.../trictrac.py   │
│  - TrictracGame  (pyspiel.Game)     │
│  - TrictracState (pyspiel.State)    │
│  registered as "python_trictrac"    │
└─────────────────────────────────────┘
```

Build pipeline:
- `just pythonlib` (in `trictrac/`) → `maturin build -m store/Cargo.toml --release` → `.whl` into `target/wheels/`
- `just installtrictrac` (in `forks/open_spiel/`) → `pip install --force-reinstall` the wheel into the devenv venv

The Rust crate is named `trictrac-store` (package) but produces a lib named `trictrac_store` (the Python module name, set in `Cargo.toml` `[lib] name`).

---

## 2. Rust Engine: Module Map

| Module | Responsibility |
|---|---|
| `board.rs` | Board representation, checker manipulation, quarter analysis |
| `dice.rs` | `Dice` struct, `DiceRoller`, bit encoding |
| `player.rs` | `Player` struct (score, bredouille), `Color`, `PlayerId`, `CurrentPlayer` |
| `game.rs` | `GameState` state machine, `GameEvent` enum, `Stage`/`TurnStage` |
| `game_rules_moves.rs` | `MoveRules`: move validation and generation |
| `game_rules_points.rs` | `PointsRules`: jan detection and scoring |
| `training_common.rs` | `TrictracAction` enum, action-space encoding (size 514) |
| `pyengine.rs` | PyO3 Python module exposing `TricTrac` class |
| `lib.rs` | Crate root, re-exports |

---

## 3. Board Representation

```rust
pub struct Board {
    positions: [i8; 24],
}
```

- 24 fields indexed 0–23 internally, 1–24 externally.
- Positive values = White checkers on that field; negative = Black.
- Initial state: `[15, 0, ..., 0, -15]` — all 15 white pieces on field 1, all 15 black pieces on field 24.
- Field 0 is a sentinel for "exited the board" (never stored in the array).

**Mirroring** is the central symmetry operation used throughout:

```rust
pub fn mirror(&self) -> Self {
    let mut positions = self.positions.map(|c| 0 - c);
    positions.reverse();
    Board { positions }
}
```

This negates all values (swapping who owns each checker) and reverses the array (swapping directions). The entire engine always reasons from White's perspective; Black's moves are handled by mirroring the board first.

**Quarter structure**: fields 1–6, 7–12, 13–18, 19–24. This maps to the four tables of Trictrac:
- 1–6: White's "petit jan" (own table)
- 7–12: White's "grand jan"
- 13–18: Black's "grand jan" (= White's opponent territory)
- 19–24: Black's "petit jan" / White's "jan de retour"

The "coin de repos" (rest corner) is field 12 for White, field 13 for Black.

---

## 4. Dice

```rust
pub struct Dice {
    pub values: (u8, u8),
}
```

Dice are always a pair (never quadrupled for doubles, unlike Backgammon). The `DiceRoller` uses `StdRng` seeded from OS entropy (or an optional fixed seed for tests). Bit encoding: `"{d1:0>3b}{d2:0>3b}"` — 3 bits each, 6 bits total.

---

## 5. Player State

```rust
pub struct Player {
    pub name: String,
    pub color: Color,       // White or Black
    pub points: u8,         // 0–11 (points within current hole)
    pub holes: u8,          // holes won (game ends at >12)
    pub can_bredouille: bool,
    pub can_big_bredouille: bool,
    pub dice_roll_count: u8, // rolls since last new_pick_up()
}
```

`PlayerId` is a `u64` alias. Player 1 = White, Player 2 = Black (set at init time; this is fixed for the session in pyengine).

---

## 6. Game State Machine

### Stages

```rust
pub enum Stage { PreGame, InGame, Ended }

pub enum TurnStage {
    RollDice,       // 1 — player must request a roll
    RollWaiting,    // 0 — waiting for dice result from outside
    MarkPoints,     // 2 — points are being marked (schools mode only)
    HoldOrGoChoice, // 3 — player won a hole; choose to Go or Hold
    Move,           // 4 — player must move checkers
    MarkAdvPoints,  // 5 — mark opponent's points after the move (schools mode)
}
```

### Turn lifecycle (schools disabled — the default in pyengine)

```
RollWaiting
    │ RollResult → auto-mark points
    ├─[no hole]──→ Move
    │                │ Move → mark opponent's points → switch player
    │                └───────────────────────────────→ RollDice (next player)
    └─[hole won]─→ HoldOrGoChoice
                    ├─ Go  ──→ new_pick_up() → RollDice (same player)
                    └─ Move ──→ mark opponent's points → switch player → RollDice
```

In schools mode (`schools_enabled = true`), the player explicitly marks their own points (`Mark` event) and then the opponent's points after moving (`MarkAdvPoints` stage).

### Key events

```rust
pub enum GameEvent {
    BeginGame { goes_first: PlayerId },
    EndGame { reason: EndGameReason },
    PlayerJoined { player_id, name },
    PlayerDisconnected { player_id },
    Roll { player_id },         // triggers RollWaiting
    RollResult { player_id, dice }, // provides dice values
    Mark { player_id, points }, // explicit point marking (schools mode)
    Go { player_id },           // choose to restart position after hole
    Move { player_id, moves: (CheckerMove, CheckerMove) },
    PlayError,
}
```

### Initialization in pyengine

```rust
fn new() -> Self {
    let mut game_state = GameState::new(false); // schools_enabled = false
    game_state.init_player("player1");
    game_state.init_player("player2");
    game_state.consume(&GameEvent::BeginGame { goes_first: 1 });
    TricTrac { game_state }
}
```

Player 1 (White) always goes first. `active_player_id` uses 1-based indexing; pyengine converts to 0-based for the Python side with `active_player_id - 1`.

---

## 7. Scoring System (Jans)

Points are awarded after each dice roll based on "jans" (scoring events) detected by `PointsRules`. All computation assumes White's perspective (board is mirrored for Black before calling).

### Jan types

| Jan | Points (normal / doublet) | Direction |
|---|---|---|
| `TrueHitSmallJan` | 4 / 6 | → active player |
| `TrueHitBigJan` | 2 / 4 | → active player |
| `TrueHitOpponentCorner` | 4 / 6 | → active player |
| `FilledQuarter` | 4 / 6 | → active player |
| `FirstPlayerToExit` | 4 / 6 | → active player |
| `SixTables` | 4 / 6 | → active player |
| `TwoTables` | 4 / 6 | → active player |
| `Mezeas` | 4 / 6 | → active player |
| `FalseHitSmallJan` | −4 / −6 | → opponent |
| `FalseHitBigJan` | −2 / −4 | → opponent |
| `ContreTwoTables` | −4 / −6 | → opponent |
| `ContreMezeas` | −4 / −6 | → opponent |
| `HelplessMan` | −2 / −4 | → opponent |

A single roll can trigger multiple jans, each scored independently. The jan detection process:
1. Try both dice orderings
2. Detect "tout d'une" (combined dice move as a virtual single die)
3. Prefer true hits over false hits for the same move
4. Check quarter-filling opportunities
5. Check rare jans (SixTables at roll 3, TwoTables, Mezeas) given specific board positions and talon counts

### Hole scoring

```rust
fn mark_points(&mut self, player_id: PlayerId, points: u8) -> bool {
    let sum_points = p.points + points;
    let jeux = sum_points / 12;          // number of completed holes
    let holes = match (jeux, p.can_bredouille) {
        (0, _) => 0,
        (_, false) => 2 * jeux - 1,     // no bredouille bonus
        (_, true)  => 2 * jeux,          // bredouille doubles the holes
    };
    p.points = sum_points % 12;
    p.holes += holes;
    ...
}
```

- 12 points = 1 "jeu", which yields 1 or 2 holes depending on bredouille status.
- Scoring any points clears the opponent's `can_bredouille`.
- Completing a hole resets `can_bredouille` for the scorer.
- Game ends when `holes > 12`.
- Score reported to OpenSpiel: `holes * 12 + points`.

### Points from both rolls

After a roll, the active player's points (`dice_points.0`) are auto-marked immediately. After the Move, the opponent's points (`dice_points.1`) are marked (they were computed at roll-time from the pre-move board).

---

## 8. Move Rules

`MoveRules` always works from White's perspective. Key constraints enforced by `moves_allowed()`:

1. **Opponent's corner forbidden**: Cannot land on field 13 (opponent's rest corner for White).
2. **Corner needs two checkers**: The rest corner (field 12) must be taken or vacated with exactly 2 checkers simultaneously.
3. **Corner by effect vs. by power**: If the corner can be taken directly ("par effet"), you cannot take it "par puissance" (using combined dice).
4. **Exit preconditions**: All checkers must be in fields 19–24 before any exit is allowed.
5. **Exit by effect priority**: If a normal exit is possible, exceedant moves (using overflow) are forbidden.
6. **Farthest checker first**: When exiting with exceedant, must exit the checker at the highest field.
7. **Must play all dice**: If both dice can be played, playing only one is invalid.
8. **Must play strongest die**: If only one die can be played, it must be the higher value die.
9. **Must fill quarter**: If a quarter can be completed, the move must complete it.
10. **Cannot block opponent's fillable quarter**: Cannot move into a quarter the opponent can still fill.

The board state after each die application is simulated to check two-step sequences.

---

## 9. Action Space (training_common.rs)

Total size: **514 actions**.

| Index | Action | Description |
|---|---|---|
| 0 | `Roll` | Request dice roll (not used in OpenSpiel mode) |
| 1 | `Go` | After winning hole: reset board and continue |
| 2–257 | `Move { dice_order: true, checker1, checker2 }` | Move with die[0] first |
| 258–513 | `Move { dice_order: false, checker1, checker2 }` | Move with die[1] first |

Move encoding: `index = 2 + (0 if dice_order else 256) + checker1 * 16 + checker2`

`checker1` and `checker2` are **ordinal positions** (1-based) of specific checkers counted left-to-right across all White-occupied fields, not field indices. Checker 0 = "no move" (empty move). Range: 0–15 (16 values each).

### Mirror pattern in get_legal_actions / apply_action

For player 2 (Black):
```rust
// get_legal_actions: mirror game state before computing
let mirror = self.game_state.mirror();
get_valid_action_indices(&mirror)

// apply_action: convert action → event on mirrored state, then mirror the event back
a.to_event(&self.game_state.mirror())
 .map(|e| e.get_mirror(false))
```

This ensures Black's actions are computed as if Black were White on a mirrored board, then translated back to real-board coordinates.

---

## 10. Python Bindings (pyengine.rs)

The `TricTrac` PyO3 class exposes:

| Method | Signature | Description |
|---|---|---|
| `new()` | `→ TricTrac` | Create game, init 2 players, begin with player 1 |
| `needs_roll()` | `→ bool` | True when in `RollWaiting` stage |
| `is_game_ended()` | `→ bool` | True when `Stage::Ended` |
| `current_player_idx()` | `→ u64` | 0 or 1 (active_player_id − 1) |
| `get_legal_actions(player_idx)` | `→ Vec<usize>` | Action indices for player; empty if not their turn |
| `action_to_string(player_idx, action_idx)` | `→ String` | Human-readable action description |
| `apply_dice_roll(dices: (u8, u8))` | `→ PyResult<()>` | Inject dice result; errors if not in RollWaiting |
| `apply_action(action_idx)` | `→ PyResult<()>` | Apply a game action; validates before applying |
| `get_score(player_id)` | `→ i32` | `holes * 12 + points` for player (1-indexed!) |
| `get_players_scores()` | `→ [i32; 2]` | `[score_p1, score_p2]` |
| `get_tensor(player_idx)` | `→ Vec<i8>` | 36-element state vector (mirrored for player 1) |
| `get_observation_string(player_idx)` | `→ String` | Human-readable state (mirrored for player 1) |
| `__str__()` | `→ String` | Debug representation of game state |

Note: `get_score(player_id)` takes a 1-based player ID (1 or 2), unlike `current_player_idx()` which returns 0-based.

---

## 11. State Tensor Encoding (36 bytes)

```
[0..23]  Board positions (i8): +N white / −N black checkers per field
[24]     Active player: 0=White, 1=Black
[25]     TurnStage: 0=RollWaiting, 1=RollDice, 2=MarkPoints, 3=HoldOrGoChoice,
                   4=Move, 5=MarkAdvPoints
[26]     Dice value 1 (i8)
[27]     Dice value 2 (i8)
[28]     White: points (0–11)
[29]     White: holes (0–12)
[30]     White: can_bredouille (0 or 1)
[31]     White: can_big_bredouille (0 or 1)
[32]     Black: points
[33]     Black: holes
[34]     Black: can_bredouille
[35]     Black: can_big_bredouille
```

When called for player 1 (Black), the entire state is mirrored first (`game_state.mirror().to_vec()`).

### State ID (base64 string for hashing)

108 bits packed as 18 base64 characters:
- 77 bits: GNUbg-inspired board position encoding (run-length with separators)
- 1 bit: active player color
- 3 bits: turn stage
- 6 bits: dice (3 bits per die)
- 10 bits: white player (4 pts + 4 holes + 2 flags)
- 10 bits: black player
- Padded to 108 bits, grouped as 18 × 6-bit base64 chunks

---

## 12. OpenSpiel Integration (trictrac.py)

### Game registration

```python
pyspiel.register_game(_GAME_TYPE, TrictracGame)
```

Key parameters:
- `short_name = "python_trictrac"`
- `dynamics = SEQUENTIAL`
- `chance_mode = EXPLICIT_STOCHASTIC`
- `information = PERFECT_INFORMATION`
- `utility = GENERAL_SUM` (both players can score positive; no zero-sum constraint)
- `reward_model = REWARDS` (intermediate rewards, not just terminal)
- `num_distinct_actions = 514`
- `max_chance_outcomes = 36`
- `min_utility = 0.0`, `max_utility = 200.0`
- `max_game_length = 3000` (rough estimate)

### Chance node handling

When `needs_roll()` is true, the state is a chance node. OpenSpiel samples one of 36 outcomes (uniform):

```python
def _roll_from_chance_idx(self, action):
    return [(i,j) for i in range(1,7) for j in range(1,7)][action]

def chance_outcomes(self):
    p = 1.0 / 36
    return [(i, p) for i in range(0, 36)]
```

Action 0 → (1,1), action 1 → (1,2), …, action 35 → (6,6). The chance action is then passed to `apply_dice_roll((d1, d2))` on the Rust side.

### Player action handling

When not a chance node:
```python
def _legal_actions(self, player):
    return self._store.get_legal_actions(player)

def _apply_action(self, action):
    self._store.apply_action(action)
```

The `Roll` action (index 0) is never returned by `get_legal_actions` in this mode because the Rust side only returns Roll actions from `TurnStage::RollDice`, which is bypassed in the pyengine flow (the RollWaiting→chance node path takes over).

### Returns

```python
def returns(self):
    return self._store.get_players_scores()
# → [holes_p1 * 12 + points_p1, holes_p2 * 12 + points_p2]
```

These are cumulative scores available at any point during the game (not just terminal), consistent with `reward_model = REWARDS`.

---

## 13. Known Issues and Inconsistencies

### 13.1 `observation_string` missing return (trictrac.py:156)

```python
def observation_string(self, player):
    self._store.get_observation_string(player)  # result discarded, returns None
```

Should be `return self._store.get_observation_string(player)`.

### 13.2 `observation_tensor` not populating buffer (trictrac.py:159)

```python
def observation_tensor(self, player, values):
    self._store.get_tensor(player)  # result discarded, values not filled
```

OpenSpiel's API expects `values` (a mutable buffer, typically a flat numpy array) to be filled in-place. The returned `Vec<i8>` from `get_tensor()` is discarded. Should copy data into `values`.

### 13.3 Debug print statement active (trictrac.py:140)

```python
print("in apply action", self.is_chance_node(), action)
```

This fires on every action application. Should be removed or guarded.

### 13.4 Color swap on new_pick_up disabled

In `game.rs:new_pick_up()`:

```rust
// XXX : switch colors
// désactivé pour le moment car la vérification des mouvements échoue,
// cf. https://code.rhumbs.fr/henri/trictrac/issues/31
// p.color = p.color.opponent_color();
```

In authentic Trictrac, players swap colors between "relevés" (pick-ups after a hole is won with Go). This is commented out, so the same player always plays White and the same always plays Black throughout the entire game.

### 13.5 `can_big_bredouille` tracked but not implemented

The `can_big_bredouille` flag is stored in `Player` and serialized in state encoding, but the scoring logic never reads it. Grande bredouille (a rare extra bonus) is not implemented.

### 13.6 `Roll` action in action space but unused in OpenSpiel mode

`TrictracAction::Roll` (index 0) exists in the 514-action space and in `get_valid_actions()` (for `TurnStage::RollDice`). However, in pyengine, the game starts at `RollWaiting` (dice have been requested but not yet rolled), so `TurnStage::RollDice` is never reached from OpenSpiel's perspective. The chance node mechanism replaces the Roll action entirely. The action space slot 0 is permanently wasted from OpenSpiel's point of view.

### 13.7 `get_valid_actions` panics on `RollWaiting`

```rust
TurnStage::MarkPoints | TurnStage::MarkAdvPoints | TurnStage::RollWaiting => {
    panic!("get_valid_actions not implemented for turn stage {:?}", ...)
}
```

If `get_legal_actions` were ever called while `needs_roll()` is true, this would panic. OpenSpiel's turn logic avoids this because chance nodes are handled separately, but it is a latent danger.

### 13.8 PPO training script uses wrong model name

`trictrac_ppo.py` saves to `ppo_backgammon_model.ckpt` — clearly copied from a backgammon example without renaming. Also uses `tensorflow.compat.v1` despite the PyTorch PPO import.

### 13.9 Opponent points marked at pre-move board state

The opponent's `dice_points.1` is computed at roll time (before the active player moves), but applied to the opponent after the move. This means the opponent's scoring is evaluated on the board position that existed before the active player moved — which is per the rules of Trictrac (points are based on where pieces could be hit at the moment of the roll), but it's worth noting this subtlety.

---

## 14. Data Flow: A Complete Turn

```
Python (OpenSpiel)          →  Rust (trictrac_store)
─────────────────────────────────────────────────────
is_chance_node()            ←  needs_roll() [TurnStage == RollWaiting]
                                (true at game start)

chance_outcomes() → [(0,p)..(35,p)]

_apply_action(chance_idx)
  _roll_from_chance_idx(idx) → (d1, d2)
  apply_dice_roll((d1, d2))  →  consume(RollResult{dice})
                                  → auto-mark active player's points
                                  → if hole: TurnStage=HoldOrGoChoice
                                  → else: TurnStage=Move

current_player() → 0 or 1

_legal_actions(player)      ←  get_legal_actions(player_idx)
                                  → get_valid_actions on (possibly mirrored) state
                                  → Vec<usize> of valid action indices

_apply_action(action_idx)   →  apply_action(action_idx)
                                  → TrictracAction::from_action_index
                                  → to_event on (mirrored) state
                                  → mirror event back if player==2
                                  → validate → consume
                                  → mark opponent points
                                  → switch active player
                                  → TurnStage=RollDice (→ pyengine starts next turn)

Wait — pyengine starts at RollWaiting, not RollDice!
The next is_chance_node() call will be true again.
```

Note on turn transition: After a `Move` event in `game.rs`, turn stage becomes `RollDice` (not `RollWaiting`). The pyengine `needs_roll()` checks for `RollWaiting`. So after a move, `is_chance_node()` returns false — OpenSpiel will ask for a regular player action. But `get_valid_actions` at `TurnStage::RollDice` returns only `Roll` (index 0), which is **not** the chance path.

This reveals a subtlety: after the Move event, the active player has already been switched, so `current_player()` returns the new active player, and `get_legal_actions` returns `[0]` (Roll). OpenSpiel then applies action 0, which calls `apply_action(0)` → `TrictracAction::Roll` → `GameEvent::Roll` → TurnStage becomes `RollWaiting`. Then the next call to `is_chance_node()` returns true, and the chance mechanism kicks in again.

So the full sequence in OpenSpiel terms is:
```
[Chance] dice roll → [Player] move → [Player] Roll action → [Chance] dice roll → ...
```

The `Roll` action IS used — it is the bridge between Move completion and the next chance node.

---

## 15. Summary of Design Choices

| Choice | Rationale |
|---|---|
| All rules engine in Rust | Performance, correctness, can be used in other contexts (CLI, native bots) |
| Mirror pattern for Black | Avoids duplicating all rule logic for both colors |
| Schools disabled by default | Simpler turn structure for RL training; full protocol for human play |
| GENERAL_SUM + REWARDS | Trictrac is not strictly zero-sum; intermediate hole rewards are informative for training |
| Action index for checkers (not fields) | Reduces action space; ordinal checker numbering is compact |
| 514 action slots | 1 Roll + 1 Go + 256 × 2 move combinations (ordered by die priority × 16 × 16 checker pairs) |
| Chance node = dice roll | Standard OpenSpiel pattern for stochastic games |
