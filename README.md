# Trictrac

This is a game of [Trictrac](https://en.wikipedia.org/wiki/Trictrac) rust implementation.

The project is on its early stages.
Rules (without "schools") are implemented, as well as a rudimentary terminal interface which allow you to play against a bot which plays randomly.

Training of AI bots is the work in progress.

## Usage

Install [devenv](https://devenv.sh/getting-started/), start a devenv shell `devenv shell`, and run the following commands.

```bash
# Run the relay server
just build-relay
just run-relay  # listens on :8080

# Run the game (separate terminal)
just dev-game
```

Open two browser windows at `http://127.0.0.1:9091`. In one, create a room; in the other, join with the same room name.

Playing with the cli against the 'random' bot: `cargo run --bin=client_cli -- --bot random`

## Roadmap

- [x] rules
- [x] command line interface
- [x] basic bot (random play)
- [ ] AI bot
- [ ] network game
- [ ] web client

## Code structure

- game rules and game state are implemented in the _store/_ folder.
- the command-line application is implemented in _clients/cli/_; it allows you to play against a bot, or to have two bots play against each other
- the bots algorithms and the training of their models are implemented in the _bot/_ and _spiel_bot_ folders.

### _store_ package

The game state is defined by the `GameState` struct in _store/src/game.rs_. The `to_string_id()` method allows this state to be encoded compactly in a string (without the played moves history). For a more readable textual representation, the `fmt::Display` trait is implemented.

### _clients/cli_ package

`clients/cli/src/game_runner.rs` contains the logic to make two bots play against each other.

### _bot_ package

- `bot/src/strategy/default.rs` contains the code for a basic bot strategy: it determines the list of valid moves (using the `get_possible_moves_sequences` method of `store::MoveRules`) and simply executes the first move in the list.
- `bot/src/strategy/dqnburn.rs` is another bot strategy that uses a reinforcement learning trained model with the DQN algorithm via the burn library (<https://burn.dev/>).
- `bot/scripts/trains.sh` allows you to train agents using different algorithms (DQN, PPO, SAC).

### multiplayer game

Pagckages "clients/backbone-lib", "clients/web-game", "server/protocol", "server/relay-server" are a Leptos-optimized adaptation of the macroquad-based [Carbonfreezer/multiplayer](https://github.com/Carbonfreezer/multiplayer) project. It is a multiplayer game system in Rust targeting browser-based board games compiled as WASM. The original project used Macroquad with a polling-based transport layer; this version replaces that with an async session API built for [Leptos](https://leptos.dev/).

The system consists of:

- A **relay server** (Axum/Tokio) that routes messages between players and manages rooms, without knowing anything about game rules.
- A **backbone library** that handles WebSocket connection, handshake, and message routing, exposing an async API to the game frontend.
- Game-specific **backend logic** implementing the `BackEndArchitecture` trait, which runs only on the hosting client.
- A **Leptos frontend** that connects to a session and reacts to state updates.

There is no dedicated game server. One of the players acts as the host: their browser runs the game backend locally. The relay server only forwards messages — it never touches game state.

```
┌─────────────────────────────────────────────────────────────┐
│                        Host Client                          │
│  ┌─────────────┐    ┌──────────────────┐    ┌────────────┐  │
│  │  Leptos UI  │◄──►│  GameSession     │◄──►│  Backend   │  │
│  └─────────────┘    └────────┬─────────┘    └────────────┘  │
└───────────────────────────── │ ────────────────────────────┘
                                │  WebSocket
                         ┌──────▼──────┐
                         │ Relay Server│
                         └──────┬──────┘
                                │  WebSocket
┌───────────────────────────────│────────────────────────────┐
│  ┌─────────────┐    ┌─────────▼────────┐                   │
│  │  Leptos UI  │◄──►│  GameSession     │  (no backend)     │
│  └─────────────┘    └──────────────────┘                   │
│                        Remote Client                        │
└─────────────────────────────────────────────────────────────┘
```

#### Data flow

- **Actions** (e.g. "place stone at B3") flow from the UI to the host backend via `GameSession::send_action()`.
- **State updates** flow back as `ViewStateUpdate::Full` (full snapshot, on join or reset) or `ViewStateUpdate::Incremental` (delta, for animations).
- **Timers** are managed by the host's background task (wall-clock, no polling required from the game).

#### backbone-lib session API

The key design choice: `backbone-lib` owns a background async task per session. The Leptos app never drives a loop — it just awaits on events.

#### Workspace

**server/protocol**

Shared message-type constants and the `JoinRequest` struct used during the WebSocket handshake.

**server/relay-server**

Listens on port 8080. Loads `GameConfig.json` on startup to know which games exist and their player limits:

```json
[{ "name": "trictrac", "max_players": 10 }]
```

Games can be added at runtime via the `/reload` endpoint. `/enlist` lists active rooms. A watchdog cleans up inactive rooms every 20 minutes.

For production, put it behind a reverse proxy with SSL (the browser requires `wss://` on HTTPS pages). Example Caddy config:

```
your-domain.com {
    handle_path /api/* {
        reverse_proxy localhost:8080
    }
    file_server
}
```

**clients/backbone-lib**

Modules:

| Module     | Purpose                                                                                                    |
| ---------- | ---------------------------------------------------------------------------------------------------------- |
| `session`  | `GameSession`, `connect()`, `SessionEvent`, `RoomConfig`                                                   |
| `host`     | Background async task for the hosting client (drives `BackEndArchitecture`, manages timers)                |
| `client`   | Background async task for non-hosting clients                                                              |
| `protocol` | Wire encoding/decoding helpers (postcard + message-type bytes)                                             |
| `platform` | `spawn_task` / `sleep_ms` abstractions (WASM: `spawn_local` + gloo-timers; native: thread + thread::sleep) |
| `traits`   | `BackEndArchitecture`, `BackendCommand`, `ViewStateUpdate`, `SerializationCap`                             |
