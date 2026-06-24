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

## Inspirations

The multiplayer game architecture, implemented in packages _clients/backbone-lib_, _clients/web/game_, _server/protocol_ and _server/relay-server_ is a [Leptos](https://leptos.dev/)-optimized adaptation of the macroquad-based [Carbonfreezer/multiplayer](https://github.com/Carbonfreezer/multiplayer) project.

The web client UX/UI is inspired by https://playtiao.com.
