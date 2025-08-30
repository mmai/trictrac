# Inspirations

tools

- config clippy ?
- bacon : tests runner (ou loom ?)

## Rust libs

cf. <https://blessed.rs/crates>

nombres aléatoires avec seed : <https://richard.dallaway.com/posts/2021-01-04-repeat-resume/>

- cli : <https://lib.rs/crates/pico-args> ( ou clap )
- reseau async : tokio
- web serveur : axum (uses tokio)
  - <https://fasterthanli.me/series/updating-fasterthanli-me-for-2022/part-2#the-opinions-of-axum-also-nice-error-handling>
- db : sqlx

- eyre, color-eyre (Results)
- tracing (logging)
- rayon ( sync <-> parallel )

- front : yew + tauri
  - egui

- <https://docs.rs/board-game/latest/board_game/>

## network games

- <https://www.mattkeeter.com/projects/pont/>
- <https://github.com/jackadamson/onitama> (wasm, rooms)
- <https://github.com/UkoeHB/renet2>

## Others

- plugins avec <https://github.com/extism/extism>

## Backgammon existing projects

- go : <https://bgammon.org/blog/20240101-hello-world/>
  - protocole de communication : <https://code.rocket9labs.com/tslocum/bgammon/src/branch/main/PROTOCOL.md>
- ocaml : <https://github.com/jacobhilton/backgammon?tab=readme-ov-file>
  cli example : <https://www.jacobh.co.uk/backgammon/>
- lib rust backgammon
  - <https://github.com/carlostrub/backgammon>
  - <https://github.com/marktani/backgammon>
- network webtarot
- front ?

## cli examples

### GnuBackgammon

    (No game) new game
     gnubg rolls 3, anthon rolls 1.

     GNU Backgammon  Positions ID: 4HPwATDgc/ABMA
     Match ID   : MIEFAAAAAAAA
      +12-11-10--9--8--7-------6--5--4--3--2--1-+     O: gnubg
      | X           O    |   | O              X |     0 points
      | X           O    |   | O              X |     Rolled 31
      | X           O    |   | O                |
      | X                |   | O                |
      | X                |   | O                |
     ^|                  |BAR|                  |     (Cube: 1)
      | O                |   | X                |
      | O                |   | X                |
      | O           X    |   | X                |
      | O           X    |   | X              O |
      | O           X    |   | X              O |     0 points
      +13-14-15-16-17-18------19-20-21-22-23-24-+     X: anthon

     gnubg moves 8/5 6/5.

### jacobh

Move 11: player O rolls a 6-2.
Player O estimates that they have a 90.6111% chance of winning.

Os borne off: none  
 24 23 22 21 20 19 18 17 16 15 14 13

---

| v v v v v v | | v v v v v v |
| | | |
| X O O O | | O O O |
| X O O O | | O O |
| O | | |
| | X | |
| | | |
| | | |
| | | |
| | | |
|------------------------------| |------------------------------|
| | | |
| | | |
| | | |
| | | |
| X | | |
| X X | | X |
| X X X | | X O |
| X X X | | X O O |
| | | |
| ^ ^ ^ ^ ^ ^ | | ^ ^ ^ ^ ^ ^ |

---

1 2 3 4 5 6 7 8 9 10 11 12  
Xs borne off: none

Move 12: player X rolls a 6-3.
Your move (? for help): bar/22
Illegal move: it is possible to move more.
Your move (? for help): ?
Enter the start and end positions, separated by a forward slash (or any non-numeric character), of each counter you want to move.
Each position should be number from 1 to 24, "bar" or "off".
Unlike in standard notation, you should enter each counter movement individually. For example:
24/18 18/13
bar/3 13/10 13/10 8/5
2/off 1/off
You can also enter these commands:
p - show the previous move
n - show the next move
<enter> - toggle between showing the current and last moves
help - show this help text
quit - abandon game
