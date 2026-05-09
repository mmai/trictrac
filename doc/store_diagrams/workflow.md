# Workflow

@startuml

state c <<choice>>
state haswon <<choice>>
state MarkPoints #lightblue
state MarkAdvPoints #lightblue
note right of MarkPoints : automatic 'Mark' transition\nwhen no school
note right of MarkAdvPoints : automatic 'Mark' transition\nwhen no school

[*] -> RollDice : BeginGame
RollDice --> RollWaiting : Roll (current player)
RollWaiting --> MarkPoints : RollResult (engine)
MarkPoints --> c : Mark (current player)
c --> HoldHorGoChoice : [new hole]
c --> [*] : [has won]
c --> Move : [not new hole]
HoldHorGoChoice --> RollDice : Go
HoldHorGoChoice --> MarkAdvPoints : Move
Move --> MarkAdvPoints : Move
MarkAdvPoints --> haswon : Mark (adversary)
haswon --> RollDice : [has not won]
haswon --> [*] : [has won]
@enduml
