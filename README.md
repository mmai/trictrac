# Trictrac

This is a game of [Trictrac](https://en.wikipedia.org/wiki/Trictrac) rust implementation.

## Usage

Install [devenv](https://devenv.sh/getting-started/), start a devenv shell `devenv shell`, and run the following commands.

```bash
# Run the relay server
just build-relay
just run-relay  # listens on :8080

# Run the game (separate terminal)
just dev
```

Open a browser window at `http://127.0.0.1:9091`. You can play against a very basic bot, or invite an other player to connect at the same address.

## Code structure

- game rules and game state are implemented in the _store/_ folder.
- a server for the network game is implemented in _server/relay-server_, which uses _server/protocol_
- the web client is in _clients/web_, it connects to the server using the _clients/backbone-lib_ library
- the command-line application is implemented in _clients/cli/_; it allows you to play against a bot, or to have two bots play against each other
- the bots algorithms and the training of their models are implemented in the _bot/_ and _spiel_bot_ folders. This is a work in progress, they are not performant at all.

## Inspirations

The multiplayer game architecture, implemented in packages _clients/backbone-lib_, _clients/web/game_, _server/protocol_, _server/relay-server_ is a Leptos-optimized adaptation of the macroquad-based [Carbonfreezer/multiplayer](https://github.com/Carbonfreezer/multiplayer) project. It is a multiplayer game system in Rust targeting browser-based board games compiled as WASM. The original project used Macroquad with a polling-based transport layer; this version replaces that with an async session API built for [Leptos](https://leptos.dev/).

The web client UX/UI is inspired by https://playtiao.com.
