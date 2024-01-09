# Journal

```sh
devenv init
cargo init
cargo add pico-args
```

Organisation store / server / client selon https://herluf-ba.github.io/making-a-turn-based-multiplayer-game-in-rust-01-whats-a-turn-based-game-anyway

_store_ est la bibliothèque contenant le _reducer_ qui transforme l'état du jeu en fonction les évènements. Elle est utilisée par le _server_ et le _client_. Seuls les évènements sont transmis entre clients et serveur.

## Organisation du store

lib
  - game::GameState
    - error
    - dice
    - board
      - user
    - user
