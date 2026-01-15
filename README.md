# Trictrac

This is a game of [Trictrac](https://en.wikipedia.org/wiki/Trictrac) rust implementation.

The project is on its early stages.
Rules (without "schools") are implemented, as well as a rudimentary terminal interface which allow you to play against a bot which plays randomly.

Training of AI bots is the work in progress.

## Usage

`cargo run --bin=client_cli -- --bot random`

## Roadmap

- [x] rules
- [x] command line interface
- [x] basic bot (random play)
- [ ] AI bot
- [ ] network game
- [ ] web client

## Code structure

- game rules and game state are implemented in the _store/_ folder.
- the command-line application is implemented in _client_cli/_; it allows you to play against a bot, or to have two bots play against each other
- the bots algorithms and the training of their models are implemented in the _bot/_ folder

### _store_ package

The game state is defined by the `GameState` struct in _store/src/game.rs_. The `to_string_id()` method allows this state to be encoded compactly in a string (without the played moves history). For a more readable textual representation, the `fmt::Display` trait is implemented.

### _client_cli_ package

`client_cli/src/game_runner.rs` contains the logic to make two bots play against each other.

### _bot_ package

- `bot/src/strategy/default.rs` contains the code for a basic bot strategy: it determines the list of valid moves (using the `get_possible_moves_sequences` method of `store::MoveRules`) and simply executes the first move in the list.
- `bot/src/strategy/dqnburn.rs` is another bot strategy that uses a reinforcement learning trained model with the DQN algorithm via the burn library (<https://burn.dev/>).
- `bot/scripts/trains.sh` allows you to train agents using different algorithms (DQN, PPO, SAC).
