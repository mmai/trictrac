# DQN avec burn-rl

## Paramètre d'entraînement dans dqn/burnrl/dqn_model.rs

Ces constantes sont des hyperparamètres, c'est-à-dire des réglages que l'on fixe avant l'entraînement et qui conditionnent la manière dont le modèle va apprendre.

MEMORY_SIZE

- Ce que c'est : La taille de la "mémoire de rejeu" (Replay Memory/Buffer).
- À quoi ça sert : L'agent interagit avec l'environnement (le jeu de TricTrac) et stocke ses expériences (un état, l'action prise, la récompense obtenue, et l'état suivant) dans cette mémoire. Pour s'entraîner, au
  lieu d'utiliser uniquement la dernière expérience, il pioche un lot (batch) d'expériences aléatoires dans cette mémoire.
- Pourquoi c'est important :
  1.  Décorrélation : Ça casse la corrélation entre les expériences successives, ce qui rend l'entraînement plus stable et efficace.
  2.  Réutilisation : Une même expérience peut être utilisée plusieurs fois pour l'entraînement, ce qui améliore l'efficacité des données.
- Dans votre code : const MEMORY_SIZE: usize = 4096; signifie que l'agent gardera en mémoire les 4096 dernières transitions.

DENSE_SIZE

- Ce que c'est : La taille des couches cachées du réseau de neurones. "Dense" signifie que chaque neurone d'une couche est connecté à tous les neurones de la couche suivante.
- À quoi ça sert : C'est la "capacité de réflexion" de votre agent. Le réseau de neurones (ici, Net) prend l'état du jeu en entrée, le fait passer à travers des couches de calcul (de taille DENSE_SIZE), et sort une
  estimation de la qualité de chaque action possible.
- Pourquoi c'est important :
  - Une valeur trop petite : le modèle ne sera pas assez "intelligent" pour apprendre les stratégies complexes du TricTrac.
  - Une valeur trop grande : l'entraînement sera plus lent et le modèle pourrait "sur-apprendre" (overfitting), c'est-à-dire devenir très bon sur les situations vues en entraînement mais incapable de généraliser
    sur de nouvelles situations.
- Dans votre code : const DENSE_SIZE: usize = 128; définit que les couches cachées du réseau auront 128 neurones.

EPS_START, EPS_END et EPS_DECAY

Ces trois constantes gèrent la stratégie d'exploration de l'agent, appelée "epsilon-greedy". Le but est de trouver un équilibre entre :

- L'Exploitation : Jouer le coup que le modèle pense être le meilleur.
- L'Exploration : Jouer un coup au hasard pour découvrir de nouvelles stratégies, potentiellement meilleures.

epsilon (ε) est la probabilité de faire un choix aléatoire (explorer).

- `EPS_START` (Epsilon de départ) :

  - Ce que c'est : La valeur d'epsilon au tout début de l'entraînement.
  - Rôle : Au début, le modèle ne sait rien. Il est donc crucial qu'il explore beaucoup pour accumuler des expériences variées. Une valeur élevée (proche de 1.0) est typique.
  - Dans votre code : const EPS_START: f64 = 0.9; signifie qu'au début, l'agent a 90% de chances de jouer un coup au hasard.

- `EPS_END` (Epsilon final) :

  - Ce que c'est : La valeur minimale d'epsilon, atteinte après un certain nombre d'étapes.
  - Rôle : Même après un long entraînement, on veut conserver une petite part d'exploration pour éviter que l'agent ne se fige dans une stratégie sous-optimale.
  - Dans votre code : const EPS_END: f64 = 0.05; signifie qu'à la fin, l'agent explorera encore avec 5% de probabilité.

- `EPS_DECAY` (Décroissance d'epsilon) :
  - Ce que c'est : Contrôle la vitesse à laquelle epsilon passe de EPS_START à EPS_END.
  - Rôle : C'est un facteur de "lissage" dans la formule de décroissance exponentielle. Plus cette valeur est élevée, plus la décroissance est lente, et donc plus l'agent passera de temps à explorer.
  - Dans votre code : const EPS_DECAY: f64 = 1000.0; est utilisé dans la formule EPS_END + (EPS_START - EPS_END) \* f64::exp(-(step as f64) / EPS_DECAY); pour faire diminuer progressivement la valeur d'epsilon à
    chaque étape (step) de l'entraînement.

En résumé, ces constantes définissent l'architecture du "cerveau" de votre bot (DENSE*SIZE), sa mémoire à court terme (MEMORY_SIZE), et comment il apprend à équilibrer entre suivre sa stratégie et en découvrir de
nouvelles (EPS*\*).
