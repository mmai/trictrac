# Backlog

tools
  - config clippy ?
  - bacon : tests runner (ou loom ?)

## Rust libs

cf. https://blessed.rs/crates

- cli : https://lib.rs/crates/pico-args ( ou clap )
- reseau async : tokio
- web serveur : axum (uses tokio)
  - https://fasterthanli.me/series/updating-fasterthanli-me-for-2022/part-2#the-opinions-of-axum-also-nice-error-handling
- db : sqlx


- eyre, color-eyre (Results)
- tracing (logging)
- rayon ( sync <-> parallel )

- front : yew + tauri 
  - egui

- https://docs.rs/board-game/latest/board_game/

## Others
- plugins avec https://github.com/extism/extism

## Backgammon existing projects

* lib rust backgammon
  - https://docs.rs/crate/backgammon/0.4.0
  - https://github.com/marktani/backgammon
* network webtarot
* front ?

## Specs

### Game state notation

Backgammon notation : https://nymann.dev/2023/05/16/Introducing-the-Backgammon-Position-Notation-BPN-A-Standard-for-Representing-Game-State/

* piece placement
  * dames
* active player
* step 
  * roll dice
  * mark points (jeton & fichet) & set bredouille markers (3rd jeton & pavillon)
  * move pieces
* dice roll
* points
  * points
  * trous
  * bredouille possible 
  * grande bredouille possible 
