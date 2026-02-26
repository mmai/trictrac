# Plan: C++ OpenSpiel Game via cxx.rs

> Implementation plan for a native C++ OpenSpiel game for Trictrac, powered by the existing Rust engine through [cxx.rs](https://cxx.rs/) bindings.
>
> Base on reading: `store/src/pyengine.rs`, `store/src/training_common.rs`, `store/src/game.rs`, `store/src/board.rs`, `store/src/player.rs`, `store/src/game_rules_points.rs`, `forks/open_spiel/open_spiel/games/backgammon/backgammon.h`, `forks/open_spiel/open_spiel/games/backgammon/backgammon.cc`, `forks/open_spiel/open_spiel/spiel.h`, `forks/open_spiel/open_spiel/games/CMakeLists.txt`.

---

## 1. Overview

The Python binding (`pyengine.rs` + `trictrac.py`) wraps the Rust engine via PyO3. The goal here is an analogous C++ binding:

- **`store/src/cxxengine.rs`** — defines a `#[cxx::bridge]` exposing an opaque `TricTracEngine` Rust type with the same logical API as `pyengine.rs`.
- **`forks/open_spiel/open_spiel/games/trictrac/trictrac.h`** — C++ header for a `TrictracGame : public Game` and `TrictracState : public State`.
- **`forks/open_spiel/open_spiel/games/trictrac/trictrac.cc`** — C++ implementation that holds a `rust::Box<ffi::TricTracEngine>` and delegates all logic to Rust.
- Build wired together via **corrosion** (CMake-native Rust integration) and `cxx-build`.

The resulting C++ game registers itself as `"trictrac"` via `REGISTER_SPIEL_GAME` and is consumable by any OpenSpiel algorithm (AlphaZero, MCTS, etc.) that works with C++ games.

---

## 2. Files to Create / Modify

```
trictrac/
  store/
    Cargo.toml                   ← MODIFY: add cxx, cxx-build, staticlib crate-type
    build.rs                     ← CREATE: cxx-build bridge registration
    src/
      lib.rs                     ← MODIFY: add cxxengine module
      cxxengine.rs               ← CREATE: #[cxx::bridge] definition + impl

forks/open_spiel/
  CMakeLists.txt                 ← MODIFY: add Corrosion FetchContent
  open_spiel/
    games/
      CMakeLists.txt             ← MODIFY: add trictrac/ sources + test
      trictrac/                  ← CREATE directory
        trictrac.h               ← CREATE
        trictrac.cc              ← CREATE
        trictrac_test.cc         ← CREATE

  justfile                       ← MODIFY: add buildtrictrac target
trictrac/
  justfile                       ← MODIFY: add cxxlib target
```

---

## 3. Step 1 — Rust: `store/Cargo.toml`

Add `cxx` as a runtime dependency and `cxx-build` as a build dependency. Add `staticlib` to `crate-type` so CMake can link against the Rust code as a static library.

```toml
[package]
name = "trictrac-store"
version = "0.1.0"
edition = "2021"

[lib]
name = "trictrac_store"
# cdylib   → Python .so (used by maturin / pyengine)
# rlib     → used by other Rust crates in the workspace
# staticlib → used by C++ consumers (cxxengine)
crate-type = ["cdylib", "rlib", "staticlib"]

[dependencies]
base64 = "0.21.7"
cxx = "1.0"
log = "0.4.20"
merge = "0.1.0"
pyo3 = { version = "0.23", features = ["extension-module", "abi3-py38"] }
rand = "0.9"
serde = { version = "1.0", features = ["derive"] }
transpose = "0.2.2"

[build-dependencies]
cxx-build = "1.0"
```

> **Note on `staticlib` + `cdylib` coexistence.** Cargo will build all three types when asked. The static library is used by the C++ OpenSpiel build; the cdylib is used by maturin for the Python wheel. They do not interfere. The `rlib` is used internally by other workspace members (`bot`, `client_cli`).

---

## 4. Step 2 — Rust: `store/build.rs`

The `build.rs` script drives `cxx-build`, which compiles the C++ side of the bridge (the generated shim) and tells Cargo where to find the generated header.

```rust
fn main() {
    cxx_build::bridge("src/cxxengine.rs")
        .std("c++17")
        .compile("trictrac-cxx");

    // Re-run if the bridge source changes
    println!("cargo:rerun-if-changed=src/cxxengine.rs");
}
```

`cxx-build` will:

- Parse `src/cxxengine.rs` for the `#[cxx::bridge]` block.
- Generate `$OUT_DIR/cxxbridge/include/trictrac_store/src/cxxengine.rs.h` — the C++ header.
- Generate `$OUT_DIR/cxxbridge/sources/trictrac_store/src/cxxengine.rs.cc` — the C++ shim source.
- Compile the shim into `libtrictrac-cxx.a` (alongside the Rust `libtrictrac_store.a`).

---

## 5. Step 3 — Rust: `store/src/cxxengine.rs`

This is the heart of the C++ integration. It mirrors `pyengine.rs` in structure but uses `#[cxx::bridge]` instead of PyO3.

### Design decisions vs. `pyengine.rs`

| pyengine                  | cxxengine                    | Reason                                       |
| ------------------------- | ---------------------------- | -------------------------------------------- |
| `PyResult<()>` for errors | `Result<()>`                 | cxx.rs translates `Err` to a C++ exception   |
| `(u8, u8)` tuple for dice | `DicePair` shared struct     | cxx cannot cross tuples                      |
| `Vec<usize>` for actions  | `Vec<u64>`                   | cxx does not support `usize`                 |
| `[i32; 2]` for scores     | `PlayerScores` shared struct | cxx cannot cross fixed arrays                |
| Clone via PyO3 pickling   | `clone_engine()` method      | OpenSpiel's `State::Clone()` needs deep copy |

### File content

```rust
//! # C++ bindings for the TricTrac game engine via cxx.rs
//!
//! Exposes an opaque `TricTracEngine` type and associated functions
//! to C++. The C++ side (trictrac.cc) uses `rust::Box<ffi::TricTracEngine>`.
//!
//! The Rust engine always works from the perspective of White (player 1).
//! For Black (player 2), the board is mirrored before computing actions
//! and events are mirrored back before applying — exactly as in pyengine.rs.

use crate::dice::Dice;
use crate::game::{GameEvent, GameState, Stage, TurnStage};
use crate::training_common::{get_valid_action_indices, TrictracAction};

// ── cxx bridge declaration ────────────────────────────────────────────────────

#[cxx::bridge(namespace = "trictrac_engine")]
pub mod ffi {
    // ── Shared types (visible to both Rust and C++) ───────────────────────────

    /// Two dice values passed from C++ to Rust for a dice-roll event.
    struct DicePair {
        die1: u8,
        die2: u8,
    }

    /// Both players' scores: holes * 12 + points.
    struct PlayerScores {
        score_p1: i32,
        score_p2: i32,
    }

    // ── Opaque Rust type exposed to C++ ───────────────────────────────────────

    extern "Rust" {
        /// Opaque handle to a TricTrac game state.
        /// C++ accesses this only through `rust::Box<TricTracEngine>`.
        type TricTracEngine;

        /// Create a new engine, initialise two players, begin with player 1.
        fn new_trictrac_engine() -> Box<TricTracEngine>;

        /// Return a deep copy of the engine (needed for State::Clone()).
        fn clone_engine(self: &TricTracEngine) -> Box<TricTracEngine>;

        // ── Queries ───────────────────────────────────────────────────────────

        /// True when the game is in TurnStage::RollWaiting (OpenSpiel chance node).
        fn needs_roll(self: &TricTracEngine) -> bool;

        /// True when Stage::Ended.
        fn is_game_ended(self: &TricTracEngine) -> bool;

        /// Active player index: 0 (player 1 / White) or 1 (player 2 / Black).
        fn current_player_idx(self: &TricTracEngine) -> u64;

        /// Legal action indices for `player_idx`. Returns empty vec if it is
        /// not that player's turn. Indices are in [0, 513].
        fn get_legal_actions(self: &TricTracEngine, player_idx: u64) -> Vec<u64>;

        /// Human-readable action description, e.g. "0:Move { dice_order: true … }".
        fn action_to_string(self: &TricTracEngine, player_idx: u64, action_idx: u64) -> String;

        /// Both players' scores: holes * 12 + points.
        fn get_players_scores(self: &TricTracEngine) -> PlayerScores;

        /// 36-element state observation vector (i8).  Mirrored for player 1.
        fn get_tensor(self: &TricTracEngine, player_idx: u64) -> Vec<i8>;

        /// Human-readable state description for `player_idx`.
        fn get_observation_string(self: &TricTracEngine, player_idx: u64) -> String;

        /// Full debug representation of the current state.
        fn to_debug_string(self: &TricTracEngine) -> String;

        // ── Mutations ─────────────────────────────────────────────────────────

        /// Apply a dice roll result. Returns Err if not in RollWaiting stage.
        fn apply_dice_roll(self: &mut TricTracEngine, dice: DicePair) -> Result<()>;

        /// Apply a player action (move, go, roll). Returns Err if invalid.
        fn apply_action(self: &mut TricTracEngine, action_idx: u64) -> Result<()>;
    }
}

// ── Opaque type implementation ────────────────────────────────────────────────

pub struct TricTracEngine {
    game_state: GameState,
}

pub fn new_trictrac_engine() -> Box<TricTracEngine> {
    let mut game_state = GameState::new(false); // schools_enabled = false
    game_state.init_player("player1");
    game_state.init_player("player2");
    game_state.consume(&GameEvent::BeginGame { goes_first: 1 });
    Box::new(TricTracEngine { game_state })
}

impl TricTracEngine {
    fn clone_engine(&self) -> Box<TricTracEngine> {
        Box::new(TricTracEngine {
            game_state: self.game_state.clone(),
        })
    }

    fn needs_roll(&self) -> bool {
        self.game_state.turn_stage == TurnStage::RollWaiting
    }

    fn is_game_ended(&self) -> bool {
        self.game_state.stage == Stage::Ended
    }

    /// Returns 0 for player 1 (White) and 1 for player 2 (Black).
    fn current_player_idx(&self) -> u64 {
        self.game_state.active_player_id - 1
    }

    fn get_legal_actions(&self, player_idx: u64) -> Vec<u64> {
        if player_idx == self.current_player_idx() {
            if player_idx == 0 {
                get_valid_action_indices(&self.game_state)
                    .into_iter()
                    .map(|i| i as u64)
                    .collect()
            } else {
                let mirror = self.game_state.mirror();
                get_valid_action_indices(&mirror)
                    .into_iter()
                    .map(|i| i as u64)
                    .collect()
            }
        } else {
            vec![]
        }
    }

    fn action_to_string(&self, player_idx: u64, action_idx: u64) -> String {
        TrictracAction::from_action_index(action_idx as usize)
            .map(|a| format!("{}:{}", player_idx, a))
            .unwrap_or_else(|| "unknown action".into())
    }

    fn get_players_scores(&self) -> ffi::PlayerScores {
        ffi::PlayerScores {
            score_p1: self.score_for(1),
            score_p2: self.score_for(2),
        }
    }

    fn score_for(&self, player_id: u64) -> i32 {
        if let Some(player) = self.game_state.players.get(&player_id) {
            player.holes as i32 * 12 + player.points as i32
        } else {
            -1
        }
    }

    fn get_tensor(&self, player_idx: u64) -> Vec<i8> {
        if player_idx == 0 {
            self.game_state.to_vec()
        } else {
            self.game_state.mirror().to_vec()
        }
    }

    fn get_observation_string(&self, player_idx: u64) -> String {
        if player_idx == 0 {
            format!("{}", self.game_state)
        } else {
            format!("{}", self.game_state.mirror())
        }
    }

    fn to_debug_string(&self) -> String {
        format!("{}", self.game_state)
    }

    fn apply_dice_roll(&mut self, dice: ffi::DicePair) -> Result<(), String> {
        let player_id = self.game_state.active_player_id;
        if self.game_state.turn_stage != TurnStage::RollWaiting {
            return Err("Not in RollWaiting stage".into());
        }
        let dice = Dice {
            values: (dice.die1, dice.die2),
        };
        self.game_state
            .consume(&GameEvent::RollResult { player_id, dice });
        Ok(())
    }

    fn apply_action(&mut self, action_idx: u64) -> Result<(), String> {
        let action_idx = action_idx as usize;
        let needs_mirror = self.game_state.active_player_id == 2;

        let event = TrictracAction::from_action_index(action_idx)
            .and_then(|a| {
                let game_state = if needs_mirror {
                    &self.game_state.mirror()
                } else {
                    &self.game_state
                };
                a.to_event(game_state)
                    .map(|e| if needs_mirror { e.get_mirror(false) } else { e })
            });

        match event {
            Some(evt) if self.game_state.validate(&evt) => {
                self.game_state.consume(&evt);
                Ok(())
            }
            Some(_) => Err("Action is invalid".into()),
            None => Err("Could not build event from action index".into()),
        }
    }
}
```

> **Note on `Result<(), String>`**: cxx.rs requires the error type to implement `std::error::Error`. `String` does not implement it directly. Two options:
>
> - Use `anyhow::Error` (add `anyhow` dependency).
> - Define a thin newtype `struct EngineError(String)` that implements `std::error::Error`.
>
> The recommended approach is `anyhow`:
>
> ```toml
> [dependencies]
> anyhow = "1.0"
> ```
>
> Then `fn apply_action(...) -> Result<(), anyhow::Error>` — cxx.rs will convert this to a C++ exception of type `rust::Error` carrying the message.

---

## 6. Step 4 — Rust: `store/src/lib.rs`

Add the new module:

```rust
// existing modules …
mod pyengine;

// NEW: C++ bindings via cxx.rs
pub mod cxxengine;
```

---

## 7. Step 5 — C++: `trictrac/trictrac.h`

Modelled closely after `backgammon/backgammon.h`. The state holds a `rust::Box<ffi::TricTracEngine>` and delegates everything to it.

```cpp
// open_spiel/games/trictrac/trictrac.h
#ifndef OPEN_SPIEL_GAMES_TRICTRAC_H_
#define OPEN_SPIEL_GAMES_TRICTRAC_H_

#include <memory>
#include <string>
#include <vector>

#include "open_spiel/spiel.h"
#include "open_spiel/spiel_utils.h"

// Generated by cxx-build from store/src/cxxengine.rs.
// The include path is set by CMake (see CMakeLists.txt).
#include "trictrac_store/src/cxxengine.rs.h"

namespace open_spiel {
namespace trictrac {

inline constexpr int kNumPlayers          = 2;
inline constexpr int kNumChanceOutcomes   = 36;   // 6 × 6 dice outcomes
inline constexpr int kNumDistinctActions  = 514;  // matches ACTION_SPACE_SIZE in Rust
inline constexpr int kStateEncodingSize   = 36;   // matches to_vec() length in Rust
inline constexpr int kDefaultMaxTurns     = 1000;

class TrictracGame;

// ---------------------------------------------------------------------------
// TrictracState
// ---------------------------------------------------------------------------
class TrictracState : public State {
 public:
  explicit TrictracState(std::shared_ptr<const Game> game);
  TrictracState(const TrictracState& other);

  Player CurrentPlayer() const override;
  std::vector<Action> LegalActions() const override;
  std::string ActionToString(Player player, Action move_id) const override;
  std::vector<std::pair<Action, double>> ChanceOutcomes() const override;
  std::string ToString() const override;
  bool IsTerminal() const override;
  std::vector<double> Returns() const override;
  std::string ObservationString(Player player) const override;
  void ObservationTensor(Player player, absl::Span<float> values) const override;
  std::unique_ptr<State> Clone() const override;

 protected:
  void DoApplyAction(Action move_id) override;

 private:
  // Decode a chance action index [0,35] to (die1, die2).
  // Matches Python: [(i,j) for i in range(1,7) for j in range(1,7)][action]
  static trictrac_engine::DicePair DecodeChanceAction(Action action);

  // The Rust engine handle. Deep-copied via clone_engine() when cloning state.
  rust::Box<trictrac_engine::TricTracEngine> engine_;
};

// ---------------------------------------------------------------------------
// TrictracGame
// ---------------------------------------------------------------------------
class TrictracGame : public Game {
 public:
  explicit TrictracGame(const GameParameters& params);

  int NumDistinctActions() const override { return kNumDistinctActions; }
  std::unique_ptr<State> NewInitialState() const override;
  int MaxChanceOutcomes() const override { return kNumChanceOutcomes; }
  int NumPlayers() const override { return kNumPlayers; }
  double MinUtility() const override { return 0.0; }
  double MaxUtility() const override { return 200.0; }
  int MaxGameLength() const override { return 3 * max_turns_; }
  int MaxChanceNodesInHistory() const override { return MaxGameLength(); }
  std::vector<int> ObservationTensorShape() const override {
    return {kStateEncodingSize};
  }

 private:
  int max_turns_;
};

}  // namespace trictrac
}  // namespace open_spiel

#endif  // OPEN_SPIEL_GAMES_TRICTRAC_H_
```

---

## 8. Step 6 — C++: `trictrac/trictrac.cc`

```cpp
// open_spiel/games/trictrac/trictrac.cc
#include "open_spiel/games/trictrac/trictrac.h"

#include <memory>
#include <string>
#include <vector>

#include "open_spiel/abseil-cpp/absl/types/span.h"
#include "open_spiel/game_parameters.h"
#include "open_spiel/spiel.h"
#include "open_spiel/spiel_globals.h"
#include "open_spiel/spiel_utils.h"

namespace open_spiel {
namespace trictrac {
namespace {

// ── Game registration ────────────────────────────────────────────────────────

const GameType kGameType{
    /*short_name=*/"trictrac",
    /*long_name=*/"Trictrac",
    GameType::Dynamics::kSequential,
    GameType::ChanceMode::kExplicitStochastic,
    GameType::Information::kPerfectInformation,
    GameType::Utility::kGeneralSum,
    GameType::RewardModel::kRewards,
    /*min_num_players=*/kNumPlayers,
    /*max_num_players=*/kNumPlayers,
    /*provides_information_state_string=*/false,
    /*provides_information_state_tensor=*/false,
    /*provides_observation_string=*/true,
    /*provides_observation_tensor=*/true,
    /*parameter_specification=*/{
        {"max_turns", GameParameter(kDefaultMaxTurns)},
    }};

static std::shared_ptr<const Game> Factory(const GameParameters& params) {
  return std::make_shared<const TrictracGame>(params);
}

REGISTER_SPIEL_GAME(kGameType, Factory);

}  // namespace

// ── TrictracGame ─────────────────────────────────────────────────────────────

TrictracGame::TrictracGame(const GameParameters& params)
    : Game(kGameType, params),
      max_turns_(ParameterValue<int>("max_turns", kDefaultMaxTurns)) {}

std::unique_ptr<State> TrictracGame::NewInitialState() const {
  return std::make_unique<TrictracState>(shared_from_this());
}

// ── TrictracState ─────────────────────────────────────────────────────────────

TrictracState::TrictracState(std::shared_ptr<const Game> game)
    : State(game),
      engine_(trictrac_engine::new_trictrac_engine()) {}

// Copy constructor: deep-copy the Rust engine via clone_engine().
TrictracState::TrictracState(const TrictracState& other)
    : State(other),
      engine_(other.engine_->clone_engine()) {}

std::unique_ptr<State> TrictracState::Clone() const {
  return std::make_unique<TrictracState>(*this);
}

// ── Current player ────────────────────────────────────────────────────────────

Player TrictracState::CurrentPlayer() const {
  if (engine_->is_game_ended()) return kTerminalPlayerId;
  if (engine_->needs_roll())    return kChancePlayerId;
  return static_cast<Player>(engine_->current_player_idx());
}

// ── Legal actions ─────────────────────────────────────────────────────────────

std::vector<Action> TrictracState::LegalActions() const {
  if (IsChanceNode()) {
    // All 36 dice outcomes are equally likely; return indices 0–35.
    std::vector<Action> actions(kNumChanceOutcomes);
    for (int i = 0; i < kNumChanceOutcomes; ++i) actions[i] = i;
    return actions;
  }
  Player player = CurrentPlayer();
  rust::Vec<uint64_t> rust_actions =
      engine_->get_legal_actions(static_cast<uint64_t>(player));
  std::vector<Action> actions;
  actions.reserve(rust_actions.size());
  for (uint64_t a : rust_actions) actions.push_back(static_cast<Action>(a));
  return actions;
}

// ── Chance outcomes ───────────────────────────────────────────────────────────

std::vector<std::pair<Action, double>> TrictracState::ChanceOutcomes() const {
  SPIEL_CHECK_TRUE(IsChanceNode());
  const double p = 1.0 / kNumChanceOutcomes;
  std::vector<std::pair<Action, double>> outcomes;
  outcomes.reserve(kNumChanceOutcomes);
  for (int i = 0; i < kNumChanceOutcomes; ++i) outcomes.emplace_back(i, p);
  return outcomes;
}

// ── Apply action ──────────────────────────────────────────────────────────────

/*static*/
trictrac_engine::DicePair TrictracState::DecodeChanceAction(Action action) {
  // Matches: [(i,j) for i in range(1,7) for j in range(1,7)][action]
  return trictrac_engine::DicePair{
      /*die1=*/static_cast<uint8_t>(action / 6 + 1),
      /*die2=*/static_cast<uint8_t>(action % 6 + 1),
  };
}

void TrictracState::DoApplyAction(Action action) {
  if (IsChanceNode()) {
    engine_->apply_dice_roll(DecodeChanceAction(action));
  } else {
    engine_->apply_action(static_cast<uint64_t>(action));
  }
}

// ── Terminal & returns ────────────────────────────────────────────────────────

bool TrictracState::IsTerminal() const {
  return engine_->is_game_ended();
}

std::vector<double> TrictracState::Returns() const {
  trictrac_engine::PlayerScores scores = engine_->get_players_scores();
  return {static_cast<double>(scores.score_p1),
          static_cast<double>(scores.score_p2)};
}

// ── Observation ───────────────────────────────────────────────────────────────

std::string TrictracState::ObservationString(Player player) const {
  return std::string(engine_->get_observation_string(
      static_cast<uint64_t>(player)));
}

void TrictracState::ObservationTensor(Player player,
                                      absl::Span<float> values) const {
  SPIEL_CHECK_EQ(values.size(), kStateEncodingSize);
  rust::Vec<int8_t> tensor =
      engine_->get_tensor(static_cast<uint64_t>(player));
  SPIEL_CHECK_EQ(tensor.size(), static_cast<size_t>(kStateEncodingSize));
  for (int i = 0; i < kStateEncodingSize; ++i) {
    values[i] = static_cast<float>(tensor[i]);
  }
}

// ── Strings ───────────────────────────────────────────────────────────────────

std::string TrictracState::ToString() const {
  return std::string(engine_->to_debug_string());
}

std::string TrictracState::ActionToString(Player player, Action action) const {
  if (IsChanceNode()) {
    trictrac_engine::DicePair d = DecodeChanceAction(action);
    return "(" + std::to_string(d.die1) + ", " + std::to_string(d.die2) + ")";
  }
  return std::string(engine_->action_to_string(
      static_cast<uint64_t>(player), static_cast<uint64_t>(action)));
}

}  // namespace trictrac
}  // namespace open_spiel
```

---

## 9. Step 7 — C++: `trictrac/trictrac_test.cc`

```cpp
// open_spiel/games/trictrac/trictrac_test.cc
#include "open_spiel/games/trictrac/trictrac.h"

#include <iostream>
#include <memory>

#include "open_spiel/spiel.h"
#include "open_spiel/tests/basic_tests.h"
#include "open_spiel/utils/init.h"

namespace open_spiel {
namespace trictrac {
namespace {

void BasicTrictracTests() {
  testing::LoadGameTest("trictrac");
  testing::RandomSimTest(*LoadGame("trictrac"), /*num_sims=*/5);
}

}  // namespace
}  // namespace trictrac
}  // namespace open_spiel

int main(int argc, char** argv) {
  open_spiel::Init(&argc, &argv);
  open_spiel::trictrac::BasicTrictracTests();
  std::cout << "trictrac tests passed" << std::endl;
  return 0;
}
```

---

## 10. Step 8 — Build System: `forks/open_spiel/CMakeLists.txt`

The top-level `CMakeLists.txt` must be extended to bring in **Corrosion**, the standard CMake module for Rust. Add this block before the main `open_spiel` target is defined:

```cmake
# ── Corrosion: CMake integration for Rust ────────────────────────────────────
include(FetchContent)
FetchContent_Declare(
  Corrosion
  GIT_REPOSITORY https://github.com/corrosion-rs/corrosion.git
  GIT_TAG v0.5.1  # pin to a stable release
)
FetchContent_MakeAvailable(Corrosion)

# Import the trictrac-store Rust crate.
# This creates a CMake target named 'trictrac-store'.
corrosion_import_crate(
  MANIFEST_PATH ${CMAKE_CURRENT_SOURCE_DIR}/../../trictrac/store/Cargo.toml
  CRATES trictrac-store
)

# Generate the cxx bridge from cxxengine.rs.
# corrosion_add_cxxbridge:
#   - runs cxx-build as part of the Rust build
#   - creates a CMake target 'trictrac_cxx_bridge' that:
#       * compiles the generated C++ shim
#       * exposes INTERFACE include dirs for the generated .rs.h header
corrosion_add_cxxbridge(trictrac_cxx_bridge
  CRATE trictrac-store
  FILES src/cxxengine.rs
)
```

> **Where to insert**: After the `cmake_minimum_required` / `project()` lines and before `add_subdirectory(open_spiel)` (or wherever games are pulled in). Check the actual file structure before editing.

---

## 11. Step 9 — Build System: `open_spiel/games/CMakeLists.txt`

Two changes: add the new source files to `GAME_SOURCES`, and add a test target.

### 11.1 Add to `GAME_SOURCES`

Find the alphabetically correct position (after `tic_tac_toe`, before `trade_comm`) and add:

```cmake
set(GAME_SOURCES
  # ... existing games ...
  trictrac/trictrac.cc
  trictrac/trictrac.h
  # ... remaining games ...
)
```

### 11.2 Link cxx bridge into OpenSpiel objects

The `trictrac` sources need the Rust library and cxx bridge linked in. Since the existing build compiles all `GAME_SOURCES` into `${OPEN_SPIEL_OBJECTS}` as a single object library, you need to ensure the Rust library and cxx bridge are linked when that object library is consumed.

The cleanest approach is to add the link dependencies to the main `open_spiel` library target. Find where `open_spiel` is defined (likely in `open_spiel/CMakeLists.txt`) and add:

```cmake
target_link_libraries(open_spiel
  PUBLIC
    trictrac_cxx_bridge   # C++ shim generated by cxx-build
    trictrac-store        # Rust static library
)
```

If modifying the central `open_spiel` target is too disruptive, create an explicit object library for the trictrac game:

```cmake
add_library(trictrac_game OBJECT
  trictrac/trictrac.cc
  trictrac/trictrac.h
)
target_include_directories(trictrac_game
  PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/..
)
target_link_libraries(trictrac_game
  PUBLIC
    trictrac_cxx_bridge
    trictrac-store
    open_spiel_core  # or whatever the core target is called
)
```

Then reference `$<TARGET_OBJECTS:trictrac_game>` in relevant executables.

### 11.3 Add the test

```cmake
add_executable(trictrac_test
  trictrac/trictrac_test.cc
  ${OPEN_SPIEL_OBJECTS}
  $<TARGET_OBJECTS:tests>
)
target_link_libraries(trictrac_test
  PRIVATE
    trictrac_cxx_bridge
    trictrac-store
)
add_test(trictrac_test trictrac_test)
```

---

## 12. Step 10 — Justfile updates

### `trictrac/justfile` — add `cxxlib` target

Builds the Rust crate as a static library (for use by the C++ build) and confirms the generated header exists:

```just
cxxlib:
  cargo build --release -p trictrac-store
  @echo "Static lib: $(ls target/release/libtrictrac_store.a)"
  @echo "CXX header: $(find target -name 'cxxengine.rs.h' | head -1)"
```

### `forks/open_spiel/justfile` — add `buildtrictrac` and `testtrictrac`

```just
buildtrictrac:
  # Rebuild the Rust static lib first, then CMake
  cd ../../trictrac && cargo build --release -p trictrac-store
  mkdir -p build && cd build && \
    CXX=$(which clang++) cmake -DCMAKE_BUILD_TYPE=Release ../open_spiel && \
    make -j$(nproc) trictrac_test

testtrictrac: buildtrictrac
  ./build/trictrac_test

playtrictrac_cpp:
  ./build/examples/example --game=trictrac
```

---

## 13. Key Design Decisions

### 13.1 Opaque type with `clone_engine()`

OpenSpiel's `State::Clone()` must return a fully independent copy of the game state (used extensively by search algorithms). Since `TricTracEngine` is an opaque Rust type, C++ cannot copy it directly. The bridge exposes `clone_engine() -> Box<TricTracEngine>` which calls `.clone()` on the inner `GameState` (which derives `Clone`).

### 13.2 Action encoding: same 514-element space

The C++ game uses the same 514-action encoding as the Python version and the Rust training code. This means:

- The same `TrictracAction::to_action_index` / `from_action_index` mapping applies.
- Action 0 = Roll (used as the bridge between Move and the next chance node).
- Actions 2–513 = Move variants (checker ordinal pair + dice order).
- A trained C++ model and Python model share the same action space.

### 13.3 Chance outcome ordering

The dice outcome ordering is identical to the Python version:

```
action → (die1, die2)
0  → (1,1)   6  → (2,1)  ...  35 → (6,6)
```

(`die1 = action/6 + 1`, `die2 = action%6 + 1`)

This matches `_roll_from_chance_idx` in `trictrac.py` exactly, ensuring the two implementations are interchangeable in training pipelines.

### 13.4 `GameType::Utility::kGeneralSum` + `kRewards`

Consistent with the Python version. Trictrac is not zero-sum (both players can score positive holes). Intermediate hole rewards are returned by `Returns()` at every state, not just the terminal.

### 13.5 Mirror pattern preserved

`get_legal_actions` and `apply_action` in `TricTracEngine` mirror the board for player 2 exactly as `pyengine.rs` does. C++ never needs to know about the mirroring — it simply passes `player_idx` and the Rust engine handles the rest.

### 13.6 `rust::Box` vs `rust::UniquePtr`

`rust::Box<T>` (where `T` is an `extern "Rust"` type) is the correct choice for ownership of a Rust type from C++. It owns the heap allocation and drops it when the C++ destructor runs. `rust::UniquePtr<T>` is for C++ types held in Rust.

### 13.7 Separate struct from `pyengine.rs`

`TricTracEngine` in `cxxengine.rs` is a separate struct from `TricTrac` in `pyengine.rs`. They both wrap `GameState` but are independent. This avoids:

- PyO3 and cxx attributes conflicting on the same type.
- Changes to one binding breaking the other.
- Feature-flag complexity.

---

## 14. Known Challenges

### 14.1 Corrosion path resolution

`corrosion_import_crate(MANIFEST_PATH ...)` takes a path relative to the CMake source directory. Since the Rust crate lives outside the `forks/open_spiel/` directory, the path will be something like `${CMAKE_CURRENT_SOURCE_DIR}/../../trictrac/store/Cargo.toml`. Verify this resolves correctly on all developer machines (absolute paths are safer but less portable).

### 14.2 `staticlib` + `cdylib` in one crate

Rust allows `["cdylib", "rlib", "staticlib"]` in one crate, but there are subtle interactions:

- The `cdylib` build (for maturin) does not need `staticlib`, and building both doubles the compile time.
- Consider gating `staticlib` behind a Cargo feature: `crate-type` is not directly feature-gatable, but you can work around this with a separate `Cargo.toml` or a workspace profile.
- Alternatively, accept the extra compile time during development.

### 14.3 Linker symbols from Rust std

When linking a Rust `staticlib`, the C++ linker must pull in Rust's runtime and standard library symbols. Corrosion handles this automatically by reading the output of `rustc --print native-static-libs` and adding them to the link command. If not using Corrosion, these must be added manually (typically `-ldl -lm -lpthread -lc`).

### 14.4 `anyhow` for error types

cxx.rs requires the `Err` type in `Result<T, E>` to implement `std::error::Error + Send + Sync`. `String` does not satisfy this. Use `anyhow::Error` or define a thin newtype wrapper:

```rust
use std::fmt;

#[derive(Debug)]
struct EngineError(String);
impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "{}", self.0) }
}
impl std::error::Error for EngineError {}
```

On the C++ side, errors become `rust::Error` exceptions. Wrap `DoApplyAction` in a try-catch during development to surface Rust errors as `SpielFatalError`.

### 14.5 `UndoAction` not implemented

OpenSpiel algorithms that use tree search (e.g., MCTS) may call `UndoAction`. The Rust engine's `GameState` stores a full `history` vec of `GameEvent`s but does not implement undo — the history is append-only. To support undo, `Clone()` is the only reliable strategy (clone before applying, discard clone if undo needed). OpenSpiel's default `UndoAction` raises `SpielFatalError`, which is acceptable for RL training but blocks game-tree search. If search support is needed, the simplest approach is to store a stack of cloned states inside `TrictracState` and pop on undo.

### 14.6 Generated header path in `#include`

The `#include "trictrac_store/src/cxxengine.rs.h"` path used in `trictrac.h` must match the actual path that `cxx-build` (via corrosion) places the generated header. With `corrosion_add_cxxbridge`, this is typically handled by the `trictrac_cxx_bridge` target's `INTERFACE_INCLUDE_DIRECTORIES`, which CMake propagates automatically to any target that links against it. Verify by inspecting the generated build directory.

### 14.7 `rust::String` to `std::string` conversion

The bridge methods returning `String` (Rust) appear as `rust::String` in C++. The conversion `std::string(engine_->action_to_string(...))` is valid because `rust::String` is implicitly convertible to `std::string`. Verify this works with your cxx version; if not, use `engine_->action_to_string(...).c_str()` or `static_cast<std::string>(...)`.

---

## 15. Complete File Checklist

```
[ ] trictrac/store/Cargo.toml          — add cxx, cxx-build, staticlib
[ ] trictrac/store/build.rs            — new file: cxx_build::bridge(...)
[ ] trictrac/store/src/lib.rs          — add `pub mod cxxengine;`
[ ] trictrac/store/src/cxxengine.rs    — new file: full bridge implementation
[ ] trictrac/justfile                  — add `cxxlib` target
[ ] forks/open_spiel/CMakeLists.txt    — add Corrosion, corrosion_import_crate, corrosion_add_cxxbridge
[ ] forks/open_spiel/open_spiel/games/CMakeLists.txt  — add trictrac sources + test
[ ] forks/open_spiel/open_spiel/games/trictrac/trictrac.h    — new file
[ ] forks/open_spiel/open_spiel/games/trictrac/trictrac.cc   — new file
[ ] forks/open_spiel/open_spiel/games/trictrac/trictrac_test.cc — new file
[ ] forks/open_spiel/justfile          — add buildtrictrac / testtrictrac
```

---

## 16. Implementation Order

Implement in this sequence to get early feedback at each step:

1. **Rust bridge first** (`Cargo.toml` → `build.rs` → `cxxengine.rs`). Run `cargo build -p trictrac-store` and confirm the static library and generated header are produced.
2. **Verify generated header** by locating `$OUT_DIR/cxxbridge/include/trictrac_store/src/cxxengine.rs.h` and inspecting it. Confirm C++ signatures match expectations.
3. **CMake Corrosion integration** in `forks/open_spiel/CMakeLists.txt`. Confirm `cmake ..` succeeds and the `trictrac_cxx_bridge` and `trictrac-store` targets exist.
4. **Write `trictrac.h` and `trictrac.cc`**. Compile with `make trictrac_test` (or equivalent). Fix linker errors from Rust std symbols if Corrosion doesn't handle them.
5. **Run `trictrac_test`**. Use `testing::RandomSimTest` to exercise the full game loop, catch panics from `get_valid_actions` (the `TurnStage::RollWaiting` panic path), and verify game termination.
6. **Smoke-test with the example runner**: `./build/examples/example --game=trictrac`.

---

# Implementation complete

All files are in place and trictrac_test passes (168 84 final scores, all assertions pass).

What was done

Rust side (from previous session, already complete):

- trictrac/store/Cargo.toml — added staticlib, cxx, anyhow, cxx-build
- trictrac/store/build.rs — drives cxx-build
- trictrac/store/src/cxxengine.rs — full cxx bridge + TricTracEngine impl
- trictrac/store/src/lib.rs — added pub mod cxxengine;

C++ side (this session):

- forks/open_spiel/open_spiel/games/trictrac/trictrac.h — game header
- forks/open_spiel/open_spiel/games/trictrac/trictrac.cc — game implementation
- forks/open_spiel/open_spiel/games/trictrac/trictrac_test.cc — basic test

Build system:

- forks/open_spiel/open_spiel/CMakeLists.txt — Corrosion + corrosion_import_crate + corrosion_add_cxxbridge
- forks/open_spiel/open_spiel/games/CMakeLists.txt — trictrac_game OBJECT target + trictrac_test executable

Justfiles:

- trictrac/justfile — added cxxlib target
- forks/open_spiel/justfile — added buildtrictrac and testtrictrac

Fixes discovered during build

| Issue                                                                                           | Fix                                                        |
| ----------------------------------------------------------------------------------------------- | ---------------------------------------------------------- |
| Corrosion creates trictrac_store (underscore), not trictrac-store                               | Used trictrac_store in CRATE arg and target_link_libraries |
| FILES src/cxxengine.rs doubled src/src/                                                         | Changed to FILES cxxengine.rs (relative to crate's src/)   |
| Include path changed: not trictrac-store/src/cxxengine.rs.h but trictrac_cxx_bridge/cxxengine.h | Updated #include in trictrac.h                             |
| rust::Error not in inline cxx types                                                             | Added #include "rust/cxx.h" to trictrac.cc                 |
| Init() signature differs in this fork                                                           | Changed to Init(argv[0], &argc, &argv, true)               |
| libtrictrac_store.a contains PyO3 code → missing Python symbols                                 | Added Python3::Python to target_link_libraries             |
| LegalActions() not sorted (OpenSpiel requires ascending)                                        | Added std::sort                                            |
| Duplicate actions for doubles                                                                   | Added std::unique after sort                               |
| Returns() returned non-zero at intermediate states, violating invariant with default Rewards()  | Returns() now returns {0, 0} at non-terminal states        |
