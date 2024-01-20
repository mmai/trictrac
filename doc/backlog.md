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

* go : https://bgammon.org/blog/20240101-hello-world/
  - protocole de communication : https://code.rocket9labs.com/tslocum/bgammon/src/branch/main/PROTOCOL.md

* lib rust backgammon
  - https://github.com/carlostrub/backgammon
  - https://github.com/marktani/backgammon
* network webtarot
* front ?

## Specs

## Représentation des cases :

cf. ./blog/game-state-notation.md

13 14 .. 23 24
12 11 .. 2  1

Encodage efficace : https://www.gnu.org/software/gnubg/manual/html_node/A-technical-description-of-the-Position-ID.html

#### State data
* piece placement -> 77bits (24 + 23 + 30 max)
  * dames
* active player -> 1 bit
* step  -> 2 bits
  * roll dice
  * mark points (jeton & fichet) & set bredouille markers (3rd jeton & pavillon)
  * move pieces
* dice roll -> 6bits 
* points 10bits x2 joueurs = 20bits
  * points -> 4bits
  * trous -> 4bits
  * bredouille possible 1bit
  * grande bredouille possible 1bit

Total : 77 + 1 + 2 + 6 + 20 = 105 bits = 17.666 * 6 -> 18 u32 (108 possible)
