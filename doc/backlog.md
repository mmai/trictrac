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

### Game state notation

#### History

Jollyvet : rien

1698 Le jeu de trictrac...
Noirs  T 1 2 .. 11
Blancs T 1 2 .. 11

1738 Le Grand Trictrac, Bernard Laurent Soumille
A B C D E F G H I K L M
& Z Y X V T S R Q P O N

1816 Guiton
Noirs  T 1 2 .. 11
Blancs T 1 2 .. 11

1818 Cours complet de Trictrac, Pierre Marie Michel Lepeintre
m n o p q r s t u v x y
l k j i h g f e d c b a

1852 Le jeu de trictrac rendu facile
Noirs  T 1 2 .. 11
Blancs T 1 2 .. 11

#### Références actuelles

https://salondesjeux.fr/trictrac.htm : Guiton
Noirs  T 1 2 .. 11
Blancs T 1 2 .. 11

http://trictrac.org/content/index2.html
N1 N2 .. N12
B1 B2 .. B12

Backgammon
13 14 .. 23 24
12 11 .. 2  1

=> utilisation de la notation Backgammon : uniformisation de la notation quelque soit le jeu de table. 
Non dénuée d'avantages : 
- on se débarrasse de la notation spéciale du talon
- on évite confusion entre côté noir et blanc.
- bien que l'orientation change par rapport à la plupart des traité, on suit celle du Lepeintre, et celle des vidéos de Philippe Lalanne

Backgammon notation : https://nymann.dev/2023/05/16/Introducing-the-Backgammon-Position-Notation-BPN-A-Standard-for-Representing-Game-State/

GnuBg : https://www.gnu.org/software/gnubg/manual/html_node/A-technical-description-of-the-Position-ID.html

#### State data
* piece placement -> 77bits (24 + 23 + 30 max)
  * dames
* active player -> 1 bit
* step  -> 2 bits
  * roll dice
  * mark points (jeton & fichet) & set bredouille markers (3rd jeton & pavillon)
  * move pieces
* dice roll -> 4bits 
* points 10bits x2 joueurs = 20bits
  * points -> 4bits
  * trous -> 4bits
  * bredouille possible 1bit
  * grande bredouille possible 1bit

Total : 77 + 1 + 2 + 4 + 20 = 104 bits = 13 * 8 (8^2 = 256) = 17.3333 * 6 (18 u64) -> 108 possible
