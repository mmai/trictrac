# Backlog

## DONE

## TODO

### stack overflow

- <https://crates.io/crates/backtrace-on-stack-overflow>
- <https://users.rust-lang.org/t/how-to-diagnose-a-stack-overflow-issues-cause/17320/11>
- <https://www.reddit.com/r/rust/comments/1d8lxtd/debugging_stack_overflows/>

Méthodes pour limiter la stack : réduire la taille de la pile avant de lancer ton binaire en ligne de commande :

```sh
ulimit -s 6144  # Limite la pile à 6Mo
# just trainbot
RUST_BACKTRACE=1 LD_LIBRARY_PATH=./target/debug  ./target/debug/train_dqn_burn
ulimit -s unlimited # Pour revenir à la normale
```

- bot burn
  - train = `just trainbot`
    - durée d'entrainement selon params ?
  - save
  - load and run against default bot
  - many configs, save models selon config
  - retrain against himself ?

### Doc

Cheatsheet : arbre des situations et priorité des règles

### Epic : jeu simple

- déplacements autorisés par les règles (pourront être validés physiquement si jeu avec écoles)
- calcul des points automatique (pas d'écoles)

Server

-

Client

- client tui (ratatui)
- client desktop (bevy)
- client web

### Epic : jeu avec écoles

- déplacement de fiches points : validation physique
- évenements de déclaration d'école & contre école

### Epic : Bot

- OpenAi gym
  - doc gymnasium <https://gymnasium.farama.org/introduction/basic_usage/>
  - Rust implementation for OpenAi gym <https://github.com/MathisWellmann/gym-rs>
  - Backgammon (?) <https://github.com/dellalibera/gym-backgammon>
