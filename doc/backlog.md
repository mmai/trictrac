# Backlog

## DONE

## TODO

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
