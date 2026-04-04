# Trictrac — Research Notes

## 1. Rust Engine: Module Map

| Module                 | Responsibility                                                            |
| ---------------------- | ------------------------------------------------------------------------- |
| `board.rs`             | Board representation, checker manipulation, quarter analysis              |
| `dice.rs`              | `Dice` struct, `DiceRoller`, bit encoding                                 |
| `player.rs`            | `Player` struct (score, bredouille), `Color`, `PlayerId`, `CurrentPlayer` |
| `game.rs`              | `GameState` state machine, `GameEvent` enum, `Stage`/`TurnStage`          |
| `game_rules_moves.rs`  | `MoveRules`: move validation and generation                               |
| `game_rules_points.rs` | `PointsRules`: jan detection and scoring                                  |
| `training_common.rs`   | `TrictracAction` enum, action-space encoding (size 514)                   |
| `lib.rs`               | Crate root, re-exports                                                    |

---

## 2. Board Representation

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

## 3. Dice

```rust
pub struct Dice {
    pub values: (u8, u8),
}
```

Dice are always a pair (never quadrupled for doubles, unlike Backgammon). The `DiceRoller` uses `StdRng` seeded from OS entropy (or an optional fixed seed for tests). Bit encoding: `"{d1:0>3b}{d2:0>3b}"` — 3 bits each, 6 bits total.

---

## 4. Player State

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

## 5. Game State Machine

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

### Turn lifecycle (schools disabled — the default)

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

Player 1 (White) always goes first. `active_player_id` uses 1-based indexing

---

## 6. Scoring System (Jans)

Points are awarded after each dice roll based on "jans" (scoring events) detected by `PointsRules`. All computation assumes White's perspective (board is mirrored for Black before calling).

### Jan types

| Jan                     | Points (normal / doublet) | Direction       |
| ----------------------- | ------------------------- | --------------- |
| `TrueHitSmallJan`       | 4 / 6                     | → active player |
| `TrueHitBigJan`         | 2 / 4                     | → active player |
| `TrueHitOpponentCorner` | 4 / 6                     | → active player |
| `FilledQuarter`         | 4 / 6                     | → active player |
| `FirstPlayerToExit`     | 4 / 6                     | → active player |
| `SixTables`             | 4 / 6                     | → active player |
| `TwoTables`             | 4 / 6                     | → active player |
| `Mezeas`                | 4 / 6                     | → active player |
| `FalseHitSmallJan`      | −4 / −6                   | → opponent      |
| `FalseHitBigJan`        | −2 / −4                   | → opponent      |
| `ContreTwoTables`       | −4 / −6                   | → opponent      |
| `ContreMezeas`          | −4 / −6                   | → opponent      |
| `HelplessMan`           | −2 / −4                   | → opponent      |

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
- Game ends when `holes >= 12`.

### Points from both rolls

After a roll, the active player's points (`dice_points.0`) are auto-marked immediately. After the Move, the opponent's points (`dice_points.1`) are marked (they were computed at roll-time from the pre-move board).

---

## 7. Move Rules

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

## 8. Action Space (training_common.rs)

Total size: **514 actions**.

| Index   | Action                                           | Description                                  |
| ------- | ------------------------------------------------ | -------------------------------------------- |
| 0       | `Roll`                                           | Request dice roll                            |
| 1       | `Go`                                             | After winning hole: reset board and continue |
| 2–257   | `Move { dice_order: true, checker1, checker2 }`  | Move with die[0] first                       |
| 258–513 | `Move { dice_order: false, checker1, checker2 }` | Move with die[1] first                       |

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

## 9. Known Issues and Inconsistencies

### 9.1 Color swap on new_pick_up disabled

In `game.rs:new_pick_up()`:

```rust
// XXX : switch colors
// désactivé pour le moment car la vérification des mouvements échoue,
// cf. https://code.rhumbs.fr/henri/trictrac/issues/31
// p.color = p.color.opponent_color();
```

In authentic Trictrac, players swap colors between "relevés" (pick-ups after a hole is won with Go). This is commented out, so the same player always plays White and the same always plays Black throughout the entire game.

### 9.2 `can_big_bredouille` tracked but not implemented

The `can_big_bredouille` flag is stored in `Player` and serialized in state encoding, but the scoring logic never reads it. Grande bredouille (a rare extra bonus) is not implemented.

### 9.3 `get_valid_actions` panics on `RollWaiting`

```rust
TurnStage::MarkPoints | TurnStage::MarkAdvPoints | TurnStage::RollWaiting => {
    panic!("get_valid_actions not implemented for turn stage {:?}", ...)
}
```

If `get_legal_actions` were ever called while `needs_roll()` is true, this would panic.

### 9.4 Opponent points marked at pre-move board state

The opponent's `dice_points.1` is computed at roll time (before the active player moves), but applied to the opponent after the move. This means the opponent's scoring is evaluated on the board position that existed before the active player moved — which is per the rules of Trictrac (points are based on where pieces could be hit at the moment of the roll), but it's worth noting this subtlety.
