# Diagrammes d'architecture


## Diagramme de Classes / Structures

Ce diagramme montre les relations statiques entre les composants principaux.

@startuml

!theme vibrant

package "client_cli" {
  class GameRunner {
    - state: GameState
    - bots: Vec<Bot>
    + new(Vec<Box<dyn BotStrategy>>)
    + handle_event(&GameEvent)
  }
}

package "bot" {
  class Bot {
    - strategy: Box<dyn BotStrategy>
    + new(Box<dyn BotStrategy>)
    + handle_event(&GameEvent): Option<GameEvent>
  }

  interface BotStrategy {
    + choose_move(): (CheckerMove, CheckerMove)
    + get_game(): &GameState
    ' ... autres méthodes
  }

  class DefaultStrategy
  class DqnStrategy
  class ErroneousStrategy
}

package "store" {
  class GameState {
    + stage: Stage
    + turn_stage: TurnStage
    + board: Board
    + active_player_id: PlayerId
    ' ...
    + validate(&GameEvent): bool
    + consume(&GameEvent)
  }

  class GameEvent
}

GameRunner "1" *-- "1..2" Bot : contient
Bot "1" *-- "1" BotStrategy : utilise
BotStrategy <|.. DefaultStrategy : implémente
BotStrategy <|.. DqnStrategy : implémente
BotStrategy <|.. ErroneousStrategy : implémente

GameRunner ..> GameState : dépend de
GameRunner ..> GameEvent : gère
Bot ..> GameState : dépend de
Bot ..> GameEvent : traite et génère
BotStrategy ..> GameState : analyse
@enduml


## Diagramme de Séquence : Boucle de jeu d'un Bot

Ce diagramme montre les interactions dynamiques lors d'un tour de jeu où c'est à un bot de jouer.

@startuml
!theme vibrant
autonumber

participant "main (client_cli)" as Main
participant "runner: GameRunner" as Runner
participant "bot: Bot" as Bot
participant "strategy: BotStrategy" as Strategy
participant "state: GameState" as GameState

Main -> Runner: new(strategies, seed)
activate Runner
Runner -> GameState: new()
activate GameState
GameState --> Runner: state
deactivate GameState
Runner -> Main: runner
deactivate Runner

... Boucle de jeu principale ...

Main -> Runner: handle_event(event)
activate Runner

Runner -> GameState: validate(event)
activate GameState
GameState --> Runner: bool
deactivate GameState

Runner -> GameState: consume(event)
activate GameState
deactivate GameState

Runner -> Bot: handle_event(event)
activate Bot

note right of Bot: Le bot vérifie si c'est son tour de jouer

Bot -> Strategy: get_mut_game()
activate Strategy
Strategy --> Bot: &mut GameState
deactivate Strategy

' Supposons que c'est au bot de jouer un coup
Bot -> Strategy: choose_move()
activate Strategy
Strategy -> GameState: Accède à l'état (board, dice, etc.)
activate GameState
deactivate GameState
Strategy --> Bot: moves
deactivate Strategy

Bot --> Runner: Some(GameEvent::Move)
deactivate Bot

Runner -> Runner: handle_event(GameEvent::Move)
note right of Runner: Appel récursif pour traiter le coup du bot

Runner -> GameState: validate(GameEvent::Move)
activate GameState
GameState --> Runner: true
deactivate GameState

Runner -> GameState: consume(GameEvent::Move)
activate GameState
note right of GameState: L'état du jeu est mis à jour\n(pions déplacés, joueur actif changé)
deactivate GameState

Runner --> Main: Option<GameEvent> (ou None)
deactivate Runner

@enduml

