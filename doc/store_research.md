# Store Crate: Deep Research & Performance Analysis

This document covers the Rust `trictrac-store` crate that backs the OpenSpiel C++ game.
It traces the full data-flow from a C++ `get_legal_actions()` call down to individual
board operations, documents design decisions, and identifies performance bottlenecks
relevant to MCTS training throughput.

---

## 1. Module Map

| File | Responsibility |
|---|---|
| `board.rs` | `Board` (`[i8;24]`), `CheckerMove`, low-level move primitives |
| `dice.rs` | `Dice` (two `u8` values), `DiceRoller` |
| `player.rs` | `Color`, `Player` (score / holes / bredouille flags) |
| `game.rs` | `GameState` (the full game state machine + serialisation) |
| `game_rules_moves.rs` | `MoveRules` — legal move generation and validation |
| `game_rules_points.rs` | `PointsRules` — jan (scoring) computation |
| `training_common.rs` | Action encoding/decoding (`TrictracAction`, 514-element space) |
| `cxxengine.rs` | FFI bridge (cxx.rs) — the sole entry point from C++ |

---

## 2. Data Model

### 2.1 Board

```
positions: [i8; 24]   // index i → field i+1 (fields are 1-indexed)
positive  → white checkers
negative  → black checkers
0         → empty
```

Fields 1–6 = player 1's home quarter ("petit jan"), 7–12 = big jan, 13–18 = opponent
big jan, 19–24 = opponent home quarter.
Field 12 = White rest corner, field 13 = Black rest corner.

Mirror (for Black's perspective): negate every element and reverse the array.

### 2.2 CheckerMove

```
CheckerMove { from: Field, to: Field }
from ∈ [1,24], to ∈ [0,24]   (to==0 means bear off)
(0,0) = EMPTY_MOVE (no-op, used when a player cannot move one die)
```

### 2.3 Action Space (training_common.rs)

514 discrete actions:

| Index | Meaning |
|---|---|
| 0 | `Roll` — trigger a dice roll |
| 1 | `Go` — take the hole and reset (instead of holding) |
| 2–257 | `Move` with `dice_order=true` (die1 first): `2 + checker1*16 + checker2` |
| 258–513 | `Move` with `dice_order=false` (die2 first): `258 + checker1*16 + checker2` |

`checker1` and `checker2` are 1-indexed ordinal positions of checkers from the
starting field (not the field index). This avoids a per-position bijection and keeps
the space fixed-size regardless of where checkers are.

### 2.4 GameState

Key fields that affect performance:

```rust
pub struct GameState {
    pub board:            Board,            // 24 bytes
    pub active_player_id: u64,              // 1 or 2
    pub players:          HashMap<u64, Player>,   // 2 entries
    pub history:          Vec<GameEvent>,   // grows unboundedly
    pub dice:             Dice,             // 2 bytes
    pub dice_points:      (u8, u8),
    pub dice_moves:       (CheckerMove, CheckerMove),
    pub dice_jans:        PossibleJans,     // HashMap<Jan, Vec<(CM,CM)>>
    pub turn_stage:       TurnStage,
    pub stage:            Stage,
    ...
}
```

`history` is the largest field and grows ~3–4 entries per turn.
A 200-turn game holds ~600 `GameEvent` values.

---

## 3. Call Chain: get_legal_actions (C++ → Rust)

```
C++ LegalActions()
  └─ engine_->get_legal_actions(player_idx)          [cxxengine.rs]
       └─ get_valid_action_indices(&game_state or &mirrored)
            └─ get_valid_actions(state)               [training_common.rs]
                 └─ MoveRules::get_possible_moves_sequences(true, [])
                      ├─ get_possible_moves_sequences_by_dices(dice_max, dice_min, ...)
                      │    ├─ board.get_possible_moves(dice1)     [loop over fields]
                      │    └─ for each first_move:
                      │         ├─ board.clone()                  [24-byte copy]
                      │         ├─ board2.get_possible_moves(dice2)
                      │         └─ for each second_move:
                      │              ├─ check_corner_rules()
                      │              ├─ check_opponent_can_fill_quarter_rule()
                      │              ├─ check_exit_rules()        [may recurse!]
                      │              └─ check_must_fill_quarter_rule()  [recurses!]
                      └─ get_possible_moves_sequences_by_dices(dice_min, dice_max, ...)
                           [same structure as above]
```

Then for each valid `(CheckerMove, CheckerMove)` pair, `checker_moves_to_trictrac_action()`
maps it back to a `TrictracAction`:

```
checker_moves_to_trictrac_action(move1, move2, color, state)
  └─ white_checker_moves_to_trictrac_action(...)
       ├─ board.get_field_checker(White, from1)    [O(24) scan]
       ├─ board.clone()
       ├─ board.move_checker(White, move1)         [board mutation]
       └─ board.get_field_checker(White, from2)    [O(24) scan]
```

For **player 2**, an extra `GameState::mirror()` is called before all of this,
cloning the full state including history.

---

## 4. Move Generation Deep Dive

### 4.1 get_possible_moves_sequences

```rust
pub fn get_possible_moves_sequences(
    &self,
    with_excedents: bool,
    ignored_rules: Vec<TricTracRule>,
) -> Vec<(CheckerMove, CheckerMove)> {
    // called TWICE, once per dice order (max-first, then min-first)
    let mut seqs = self.get_possible_moves_sequences_by_dices(dice_max, dice_min, ...);
    let mut seqs2 = self.get_possible_moves_sequences_by_dices(dice_min, dice_max, ...);
    seqs.append(&mut seqs2);
    // deduplication via HashSet
}
```

The function is **correct but called recursively** through rule validation:

- `check_must_fill_quarter_rule()` calls `get_quarter_filling_moves_sequences()`
- `get_quarter_filling_moves_sequences()` calls `get_possible_moves_sequences(true, [Exit, MustFillQuarter])`
- This inner call's `check_must_fill_quarter_rule()` does **not** recurse further (because `MustFillQuarter` is in ignored_rules)

So there are at most **2 levels of recursion**, but the second level is invoked once per
candidate move pair at the outer level. If there are N first-moves × M second-moves,
`get_quarter_filling_moves_sequences()` is called N×M times and each call triggers a
full second-level move generation pass.

### 4.2 get_possible_moves_sequences_by_dices

```rust
for first_move in board.get_possible_moves(dice1, ...) {
    let mut board2 = self.board.clone();          // ← clone per first move
    board2.move_checker(color, first_move);
    for second_move in board2.get_possible_moves(dice2, ...) {
        // 4 rule checks, each potentially expensive
        if all_pass {
            moves_seqs.push((first_move, second_move));
        }
    }
    if !has_second {
        // also push (first_move, EMPTY_MOVE) after same 4 checks
    }
}
```

**Board clones**: one per first-move candidate. With 15 checkers on 24 fields, a
typical position has 5–15 valid first moves → 5–15 board clones per call.

### 4.3 check_must_fill_quarter_rule

```rust
fn check_must_fill_quarter_rule(&self, moves) -> Result<(), MoveError> {
    let filling_moves_sequences = self.get_quarter_filling_moves_sequences();
    if !filling_moves_sequences.contains(moves) && !filling_moves_sequences.is_empty() {
        return Err(MoveError::MustFillQuarter);
    }
    Ok(())
}
```

`get_quarter_filling_moves_sequences()` runs a full pass of move generation and
applies both checker moves to a board clone for each candidate:

```rust
pub fn get_quarter_filling_moves_sequences(&self) -> Vec<(CheckerMove, CheckerMove)> {
    for moves in self.get_possible_moves_sequences(true, [Exit, MustFillQuarter]) {
        let mut board = self.board.clone();       // ← clone per candidate
        board.move_checker(color, moves.0).unwrap();
        board.move_checker(color, moves.1).unwrap();
        if board.any_quarter_filled(Color::White) {
            moves_seqs.push(moves);
        }
    }
    moves_seqs
}
```

If quarter-filling is not relevant to the current position (the common case early in
the game), this entire function still runs a full move generation pass before returning
an empty vec.

### 4.4 check_exit_rules

```rust
fn check_exit_rules(&self, moves) -> Result<(), MoveError> {
    if !moves.0.is_exit() && !moves.1.is_exit() { return Ok(()); }
    if self.has_checkers_outside_last_quarter() { return Err(...); }
    let non_excedent_seqs = self.get_possible_moves_sequences(false, [Exit]);  // ← full pass
    if non_excedent_seqs.contains(moves) { return Ok(()); }
    if !non_excedent_seqs.is_empty() { return Err(ExitByEffectPossible); }
    // check farthest checker rule ...
    Ok(())
}
```

Called per candidate move pair during the inner loop. Triggers another full
`get_possible_moves_sequences(false, [Exit])` pass whenever a move involves bearing off.

---

## 5. Jan (Points) Computation

### 5.1 get_jans (game_rules_points.rs)

Called once per dice roll via `game.rs: consume(RollResult)`. Not on the MCTS hot path
(MCTS does not need to compute points, only moves). However it is called during
`get_possible_moves_sequences()` indirectly via `get_scoring_quarter_filling_moves_sequences()`.

`PossibleJans = HashMap<Jan, Vec<(CheckerMove, CheckerMove)>>` — with only 13 possible
enum keys this is overkill. A fixed-size array would be faster.

`PossibleJansMethods::push()` uses `ways.contains(&cmoves)` — O(n) linear search on
the existing moves list to avoid duplicates. For small lists this is fine, but it can
be called dozens of times per position.

`PossibleJansMethods::merge()` for `TrueHitBigJan`/`TrueHitSmallJan` does two O(n)
scans per entry (one `contains`, one `retain`).

---

## 6. State Encoding

### 6.1 to_vec() — neural-network input (36 i8 values)

```
[0..23]  board positions (i8, negative = black)
[24]     active player color (0=white, 1=black)
[25]     turn stage (u8 cast to i8)
[26]     dice.values.0
[27]     dice.values.1
[28..31] white player: points, holes, can_bredouille, can_big_bredouille
[32..35] black player: same
```

Simple, allocation-heavy (returns `Vec<i8>`). For the MCTS hot path, returning a
`[i8; 36]` stack array would avoid the heap allocation entirely.

### 6.2 GameState::mirror()

```rust
pub fn mirror(&self) -> GameState {
    // Mirrors board (O(24))
    // Swaps and mirrors two player entries in a new HashMap
    // Clones and mirrors the entire history Vec (O(history.len()))  ← expensive
    // Mirrors dice_moves (O(1))
    // Mirrors dice_jans (clone + HashMap iteration)
    ...
}
```

`mirror()` is called on every `get_legal_actions()` invocation for player 2 (Black).
Cloning the history is **O(history.len())** and history grows through the entire game.

---

## 7. Action Encoding/Decoding

### 7.1 TrictracAction::to_event() — decode action index → GameEvent

For `Move` actions, `to_event()`:
1. Reads `checker1` / `checker2` ordinal positions
2. Calls `board.get_checker_field(color, checker1)` — O(24) scan
3. Clones the board
4. Applies the first move on the clone
5. Calls `board.get_checker_field(color, checker2)` — O(24) scan on the clone
6. Adjusts for "prise par puissance"
7. Constructs the `GameEvent::Move`

This is called once per `apply_action()` call from C++, so it is not as hot as legal
action generation.

### 7.2 white_checker_moves_to_trictrac_action() — encode (CheckerMove, CheckerMove) → TrictracAction

Called for **every valid move** during `get_valid_actions()`:
1. Computes `diff_move1` to identify which die was used first
2. Calls `board.get_field_checker(White, from1)` — O(24) scan
3. Clones the board
4. Calls `board.move_checker(White, move1)` — mutation on clone
5. Calls `board.get_field_checker(White, from2)` — O(24) scan on clone

With 20 valid moves, this is 20 board clones + 40 O(24) scans per
`get_valid_actions()` call.

---

## 8. Identified Performance Bottlenecks

Ordered by estimated impact on MCTS training throughput:

### 8.1 [HIGH] Recursive move generation in check_must_fill_quarter_rule

**Problem**: `get_possible_moves_sequences()` is called O(F₁ × F₂) times where F₁ and F₂ are
the number of first- and second-move candidates. Each inner call runs a complete move
generation pass.

**When it triggers**: Only when at least one valid move would fill/preserve a quarter
of the board. This is a common situation especially in mid/late game.

**Fix direction**: Precompute `get_quarter_filling_moves_sequences()` once per
`get_possible_moves_sequences()` call and pass it in, instead of recomputing it for
each candidate pair.

### 8.2 [HIGH] GameState::mirror() copies history

**Problem**: `mirror()` clones + iterates `self.history` on every `get_legal_actions()`
call for Black. The history grows proportionally to game length and serves no purpose
for action generation.

**Fix direction**: Either skip history in `mirror()` (pass an empty `Vec` for that
field) or refactor `cxxengine.rs` to mirror only the board and thread color perspective
through the functions that need it.

### 8.3 [MEDIUM] Board clones in get_possible_moves_sequences_by_dices

**Problem**: One `board.clone()` per first-move candidate (5–15 per call). Each clone
is 24 bytes, but the allocator round-trip costs more than the copy.

**Fix direction**: Apply and undo moves on a single mutable board (move + undo pattern)
rather than cloning. Board mutation is O(1) and undoing is symmetric.

### 8.4 [MEDIUM] Board clones in get_quarter_filling_moves_sequences

**Problem**: One `board.clone()` per candidate move sequence, plus two `move_checker()`
calls (which can return Err — currently `.unwrap()`ed). These clones are nested inside
the already-expensive recursive path described in 8.1.

**Fix direction**: Same move + undo pattern as 8.3.

### 8.5 [MEDIUM] Board clones and O(24) scans in checker_moves_to_trictrac_action

**Problem**: Called for every valid move in `get_valid_actions()`. With 20 valid moves,
this is 20 clones + 40 O(24) scans.

**Fix direction**: Pass the checker ordinal index directly from `get_board_exit_farthest`
or store it alongside the CheckerMove when generating moves. This avoids re-scanning
to convert back.

### 8.6 [MEDIUM] check_exit_rules triggers a full move generation pass

**Problem**: Called for every candidate move pair that involves bearing off. Runs
`get_possible_moves_sequences(false, [Exit])` → another full move generation.

**Fix direction**: Precompute non-excedant moves once if any move in the candidate set
involves bearing off, and pass the result in.

### 8.7 [LOW] PossibleJans backed by HashMap with linear-search deduplication

**Problem**: Only 13 `Jan` variants exist. A `HashMap` adds hashing overhead and
pointer indirection. `Vec::contains()` for deduplication is O(n).

**Fix direction**: Use `[Option<SmallVec<...>>; 13]` with `Jan as usize` index. For
deduplication use a `BTreeSet` or just sort + dedup (the lists are short).

### 8.8 [LOW] to_vec() returns a heap-allocated Vec

**Problem**: Called from C++ via `get_tensor()` on every MCTS rollout observation.

**Fix direction**: Return `[i8; 36]` or write directly into a pre-allocated C++-owned
buffer (possible with cxx.rs via `Pin<&mut CxxVector<i8>>`).

### 8.9 [LOW] get_color_fields() allocates a Vec on every call

**Problem**: Called repeatedly in move generation (once per first-move enumeration,
once in is_quarter_fillable per field, etc.).

**Fix direction**: Return a fixed-size `ArrayVec<(Field, i8), 24>` (using the
`arrayvec` crate) or add a small-vec optimisation.

### 8.10 [LOW] unwrap() calls in hot paths (correctness concern)

Currently `.unwrap()` in:
- `get_quarter_filling_moves_sequences()` line 529-530: `board.move_checker(...).unwrap()`
- Multiple places in `game_rules_points.rs`

With `catch_unwind` wrapping at the FFI boundary these panics are now caught, but
they still abort the move and propagate an error to C++ rather than producing a clean
`SpielFatalError` message. These should be `?` with proper `Result` propagation or
`.expect()` with descriptive messages.

---

## 9. Key Correctness Observations

### 9.1 Game ends during chance action (fixed in alpha_zero.cc)

`game.rs::consume(RollResult)` sets `stage = Stage::Ended` (line 795) when a player's
jan score grants their 13th hole on the dice roll itself. This makes a **chance action**
terminal. The OpenSpiel `PlayGame` loop in `alpha_zero.cc` did not check `IsTerminal()`
after chance actions — this was the root cause of the SIGSEGV crash. Fixed by adding:
```cpp
state->ApplyAction(action);  // chance action
if (state->IsTerminal()) { trajectory.returns = state->Returns(); break; }
```

### 9.2 current_player_idx() panics on underflow

```rust
fn current_player_idx(&self) -> u64 {
    self.game_state.active_player_id - 1  // u64 underflow if id == 0
}
```

`active_player_id` is always 1 or 2 during normal play (set by `BeginGame`), so this
is safe in practice. However it should be wrapped in `catch_unwind` or guarded by an
explicit check for robustness:
```rust
self.game_state.active_player_id.saturating_sub(1)
```

### 9.3 Mirror is always from White's perspective

`MoveRules` and `PointsRules` always reason from White's point of view. For Black, the
caller must mirror the board beforehand. The comment at the top of `game_rules_moves.rs`
documents this. `cxxengine.rs` correctly mirrors the `GameState` for player 2 before
calling `get_valid_action_indices()`.

---

## 10. Summary Table

| Bottleneck | Location | Estimated Severity | Fix Complexity |
|---|---|---|---|
| Recursive `get_possible_moves_sequences` in `check_must_fill_quarter_rule` | `game_rules_moves.rs:272` | High | Medium |
| `GameState::mirror()` clones history | `game.rs:159` | High | Low |
| Board clones per first-move in `get_possible_moves_sequences_by_dices` | `game_rules_moves.rs:555` | Medium | Medium |
| Board clones in `get_quarter_filling_moves_sequences` | `game_rules_moves.rs:528` | Medium | Medium |
| `check_exit_rules` triggers full move generation | `game_rules_moves.rs:335` | Medium | Medium |
| `checker_moves_to_trictrac_action` clones per valid move | `training_common.rs:326` | Medium | Low–Medium |
| `PossibleJans` HashMap + linear dedup | `game_rules_points.rs:67` | Low | Low |
| `to_vec()` heap allocation | `game.rs:213` | Low | Low |
| `get_color_fields()` Vec allocation | `board.rs:405` | Low | Medium |
| `unwrap()` in hot paths | multiple | Correctness | Low |
