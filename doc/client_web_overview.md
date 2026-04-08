# client_web Crate Overview

A Leptos-based WASM frontend for trictrac. Builds to a single-page app served by Trunk on port 9092.

---

## File Structure

```
client_web/
├── Cargo.toml             # Dependencies and i18n locale config
├── Trunk.toml             # Serve port 9092
├── index.html             # Shell: mounts WASM + links CSS
├── assets/style.css       # All styles (~472 lines, no framework)
├── locales/
│   ├── en.json            # 52 English keys
│   └── fr.json            # 52 French keys
└── src/
    ├── main.rs            # load_locales!() macro + mount_to_body
    ├── app.rs             # Root App component, state, network loop (571 lines)
    ├── components/
    │   ├── mod.rs
    │   ├── game_screen.rs  # Main in-game UI, move staging (324 lines)
    │   ├── board.rs        # Board rendering and click handling (372 lines)
    │   ├── die.rs          # SVG die face
    │   ├── score_panel.rs  # Points/holes bar for one player
    │   ├── scoring.rs      # Jan-by-jan scoring notification panel
    │   ├── login_screen.rs # Room create/join
    │   └── connecting_screen.rs
    └── trictrac/
        ├── mod.rs
        ├── types.rs        # Protocol types: ViewState, JanEntry, PlayerAction, … (217 lines)
        ├── backend.rs      # BackEndArchitecture impl, engine bridge (332 lines)
        └── bot_local.rs    # Local bot: random moves, always Go (34 lines)
```

---

## Component Tree

```
App                          ← manages screen, pending queue, network task
└─ I18nContextProvider
   ├─ LoginScreen             ← room name input, create/join/bot buttons
   ├─ ConnectingScreen        ← spinner while connecting
   └─ GameScreen              ← in-game UI; receives GameUiState prop
      ├─ PlayerScorePanel     ← opponent score (above board)
      ├─ Board                ← 24 interactive fields; SVG arrow overlay
      ├─ side panel
      │  ├─ status bar        ← localised turn/action prompt
      │  ├─ dice bar          ← two Die components
      │  ├─ ScoringPanel (me)       ← my jans this turn, hold/go buttons
      │  ├─ ScoringPanel (opponent) ← opponent jans (shown during pause)
      │  └─ action buttons    ← Continue / Go / Empty Move
      └─ PlayerScorePanel     ← my score (below board)
      [game-over overlay modal]
```

---

## Screens and Transitions

```
Login ──(connect)──→ Connecting ──(game start)──→ Playing
                                    ↑                 │
                                    └──(reconnect)─────┘
Playing ──(disconnect / game over)──→ Login
```

`app.rs` drives transitions via `RwSignal<Screen>`.

---

## State Management

### Root signals (live in `App`, provided via Leptos context)

| Signal | Type | Purpose |
|--------|------|---------|
| `screen` | `RwSignal<Screen>` | Which screen is shown |
| `pending` | `RwSignal<VecDeque<GameUiState>>` | Buffered states awaiting "Continue" |
| `cmd_tx` | `UnboundedSender<NetCommand>` | UI → network command channel |

Both `pending` and `cmd_tx` are provided as context so any descendant can read/write them without prop-drilling.

### GameScreen-local signals

| Signal | Type | Purpose |
|--------|------|---------|
| `selected_origin` | `RwSignal<Option<u8>>` | First clicked field during move staging |
| `staged_moves` | `RwSignal<Vec<(u8, u8)>>` | Accumulated (origin, dest) pairs for this turn |
| `hovered_jan_moves` | `RwSignal<Vec<(CheckerMove, CheckerMove)>>` | Moves to draw arrows for on hover |

### Data flow

```
Network task (async in App)
    ↓  SessionEvent::Update
push_or_show() → pending queue or screen.set()
    ↓
GameScreen re-renders (GameUiState prop)
    ↓
User clicks field → staged_moves effect → NetCommand::Action(Move)
User clicks Go/Continue → cmd_tx.send or pending.pop_front()
```

---

## Network and Session

The multiplayer layer is provided by `backbone-lib` (local fork at `../../forks/multiplayer/`). `App` spawns an async task (via `spawn_local`) that multiplexes:
- `cmd_rx`: commands from UI components
- `session.next_event()`: updates from the server

### StoredSession (localStorage key: `"trictrac_session"`)

```rust
struct StoredSession {
    relay_url: String,
    game_id: String,
    room_id: String,
    token: u64,          // reconnect token issued by server
    is_host: bool,
    view_state: Option<ViewState>,  // host saves last known state; guest saves None
}
```

On page load, if a stored session exists, App goes directly to Connecting and sends `NetCommand::Reconnect`. Failed reconnects clear the session and return to Login.

---

## Pause / Confirmation Flow

Certain opponent events are paused so the local player can see what happened before their turn starts.

Pause triggers (`infer_pause_reason()` in `app.rs`):

| Reason | Condition |
|--------|-----------|
| `AfterOpponentRoll` | Opponent is active; dice values changed |
| `AfterOpponentGo` | Opponent chose Go (HoldOrGoChoice→Move transition) |
| `AfterOpponentMove` | Turn switched to us |

While a state is in the pending queue, `GameScreen` shows a "Continue" button. Clicking it calls `pending.pop_front()`; if the queue empties, the live state is displayed.

---

## Game Engine Integration

**File**: `src/trictrac/backend.rs`

`TrictracBackend` implements the `BackEndArchitecture` trait. It owns a `GameState` from `trictrac-store` and translates between the UI protocol and the engine's event model.

### PlayerAction → GameEvent mapping

| PlayerAction | GameEvents emitted |
|---|---|
| `Roll` | `GameEvent::Roll`, `GameEvent::RollResult(d1, d2)` |
| `Move(m1, m2)` | `GameEvent::Move` (after validation) |
| `Go` | `GameEvent::Go` |
| `Mark` | internal; drives `MarkPoints`/`MarkAdvPoints` loop automatically |

`drive_automatic_stages()` loops through scoring stages without waiting for player input — these are not interactive in the current implementation (schools are not implemented).

### ViewState construction

`ViewState::from_game_state()` in `types.rs` converts the engine state to the serialisable snapshot sent to clients:
- `board: [i8; 24]` — direct copy of `Board::positions`
- `dice: [u8; 2]` — current dice values
- `stage / turn_stage` — serialisable enums (`SerStage`, `SerTurnStage`)
- `scores: [PlayerScore; 2]` — points, holes, `can_bredouille`
- `dice_jans: Vec<JanEntry>` — scoring events for the current turn, sorted descending by points
- `active_player_index: usize` — 0 = host, 1 = guest

### Bot

`bot_local.rs` runs in the browser (no server call). It inspects `GameState` directly and returns a `PlayerAction`:
- **RollDice**: always Roll
- **HoldOrGoChoice**: always Go
- **Move**: picks a random legal sequence from `MoveRules::get_possible_moves_sequences()`; mirrors moves because Black's board is mirrored

---

## Board Rendering (`board.rs`)

### Layout

The 24 fields are split into 4 quarters of 6. Each player sees the board from their own perspective:

```
White's view:
  TOP-LEFT  [13–18]  |  TOP-RIGHT  [19–24]
  ─────────────────────────────────────────
  BOT-LEFT  [12–7]   |  BOT-RIGHT  [6–1]

Black's view (mirror):
  TOP-LEFT  [1–6]    |  TOP-RIGHT  [7–12]
  ─────────────────────────────────────────
  BOT-LEFT  [24–19]  |  BOT-RIGHT  [18–13]
```

Fields are 60 × 180 px, alternating gold (`#d4a843` / `#c49030`). Checkers are 40 px SVG circles (radial gradient). Up to 4 are stacked visually; a text label is shown when count > 4.

### Highlighting

Field CSS classes are computed reactively inside the `view!` macro closure:

| Class | Meaning |
|-------|---------|
| `.clickable` | Valid origin during Move stage (lime green) |
| `.selected` | Currently selected origin (darker green + outline) |
| `.dest` | Valid destination for the selected origin |

`valid_sequences` (from `MoveRules`) is computed once per render and used to derive `valid_origins_for()` and `valid_dests_for()`. The displayed checker count (`displayed_value()`) accounts for staged-but-not-yet-sent moves so the board previews the move visually.

### SVG arrow overlay

When the player hovers a row in the ScoringPanel, the corresponding checker moves are drawn as gold arrows over the board. `field_center()` maps field numbers to pixel coordinates; `arrow_svg()` renders the path with a drop-shadow.

---

## Scoring Display (`scoring.rs`, `score_panel.rs`)

`compute_scored_event()` in `app.rs` diffs consecutive `ViewState` snapshots to produce a `ScoredEvent`:
- `points_earned: i32`
- `holes_gained: u8`
- `jans: Vec<JanEntry>` — only events relevant to the beneficiary

`ScoringPanel` renders one `JanEntry` per row. Hovering a row writes that entry's moves into `hovered_jan_moves`, triggering the arrow overlay on the board.

`PlayerScorePanel` shows a colour-filled bar (animated via CSS `transition: width 0.3s`) for points (0–12) and holes (0–12). Bredouille state is shown with a small indicator.

---

## Internationalisation

`leptos_i18n::load_locales!()` is a compile-time macro that reads `locales/en.json` and `locales/fr.json` and generates a typed `i18n` module. There are 52 keys covering UI labels, game-state prompts, jan names, and status messages.

Usage in components:
```rust
let i18n = use_i18n();
t!(i18n, your_turn_roll)                       // → reactive View
t_string!(i18n, scored_pts, pts = 4)           // → String with interpolation
```

The language switcher (top bar and login screen) calls `i18n.set_locale(Locale::en | Locale::fr)`, which triggers a full reactive re-render.

---

## Styling

`assets/style.css` is a single hand-written stylesheet (~472 lines). No CSS framework.

Key design tokens:
- Body background: `#c8b084` (tan)
- Board background: `#2e6b2e` (dark green)
- Fields: `#d4a843` / `#c49030` (gold alternating)
- Interactive fields: `#aad060` (lime, clickable) / `#709a20` (darker, selected)
- UI panels: `#f5edd8` (cream)

Layout uses Flexbox and CSS Grid throughout. Score bars animate with `transition: width 0.3s`. Field clicks give immediate feedback via `transition: background 0.1s`. No media queries — the layout is designed for desktop/tablet.

---

## Protocol Types (`types.rs`)

| Type | Role |
|------|------|
| `PlayerAction` | `Roll \| Move(CheckerMove, CheckerMove) \| Go \| Mark` — UI → backend |
| `GameDelta` | `{ state: ViewState }` — broadcast to all clients on every change |
| `ViewState` | Full serialisable snapshot of engine state |
| `JanEntry` | One scoring event: jan type, points, ways, moves, is_double |
| `ScoredEvent` | Points/holes delta + jan list for one player in one turn |
| `PlayerScore` | name, points (0–11), holes (0–12), can_bredouille |
| `SerStage` | `PreGame \| InGame \| Ended` |
| `SerTurnStage` | `RollDice \| RollWaiting \| MarkPoints \| HoldOrGoChoice \| Move \| MarkAdvPoints` |

`CheckerMove` comes directly from `trictrac-store`; fields are 1-indexed (0 = stack/exit).

---

## Build

```bash
trunk serve          # dev server at http://localhost:9092
trunk build --release  # WASM release bundle
```

`index.html` uses Trunk's `data-trunk` attributes: `rel="rust"` compiles `src/main.rs` to WASM; `rel="css"` copies `assets/style.css`. The WASM binary and generated JS glue land in `dist/`.
