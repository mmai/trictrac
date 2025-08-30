# DQN avec burn-rl

## Paramètre d'entraînement dans dqn/burnrl/dqn_model.rs

Ces constantes sont des hyperparamètres, c'est-à-dire des réglages que l'on fixe avant l'entraînement et qui conditionnent la manière dont le modèle va apprendre.

MEMORY_SIZE

- Ce que c'est : La taille de la "mémoire de rejeu" (Replay Memory/Buffer).
- À quoi ça sert : L'agent interagit avec l'environnement (le jeu de TricTrac) et stocke ses expériences (un état, l'action prise, la récompense obtenue, et l'état suivant) dans cette mémoire. Pour s'entraîner, au
  lieu d'utiliser uniquement la dernière expérience, il pioche un lot (batch) d'expériences aléatoires dans cette mémoire.
- Pourquoi c'est important :
  1. Décorrélation : Ça casse la corrélation entre les expériences successives, ce qui rend l'entraînement plus stable et efficace.
  2. Réutilisation : Une même expérience peut être utilisée plusieurs fois pour l'entraînement, ce qui améliore l'efficacité des données.
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

## Paramètres DQNTrainingConfig

1. `gamma` (Facteur d'actualisation / _Discount Factor_)

   - À quoi ça sert ? Ça détermine l'importance des récompenses futures. Une valeur proche de 1 (ex: 0.99)
     indique à l'agent qu'une récompense obtenue dans le futur est presque aussi importante qu'une
     récompense immédiate. Il sera donc "patient" et capable de faire des sacrifices à court terme pour un
     gain plus grand plus tard.
   - Intuition : Un gamma de 0 rendrait l'agent "myope", ne se souciant que du prochain coup. Un gamma de
     0.99 l'encourage à élaborer des stratégies à long terme.

2. `tau` (Taux de mise à jour douce / _Soft Update Rate_)

   - À quoi ça sert ? Pour stabiliser l'apprentissage, les algorithmes DQN utilisent souvent deux réseaux
     : un réseau principal qui apprend vite et un "réseau cible" (copie du premier) qui évolue lentement.
     tau contrôle la vitesse à laquelle les connaissances du réseau principal sont transférées vers le
     réseau cible.
   - Intuition : Une petite valeur (ex: 0.005) signifie que le réseau cible, qui sert de référence stable,
     ne se met à jour que très progressivement. C'est comme un "mentor" qui n'adopte pas immédiatement
     toutes les nouvelles idées de son "élève", ce qui évite de déstabiliser tout l'apprentissage sur un
     coup de chance (ou de malchance).

3. `learning_rate` (Taux d'apprentissage)

   - À quoi ça sert ? C'est peut-être le plus classique des hyperparamètres. Il définit la "taille du
     pas" lors de la correction des erreurs. Après chaque prédiction, l'agent compare le résultat à ce
     qui s'est passé et ajuste ses poids. Le learning_rate détermine l'ampleur de cet ajustement.
   - Intuition : Trop élevé, et l'agent risque de sur-corriger et de ne jamais converger (comme chercher
     le fond d'une vallée en faisant des pas de géant). Trop bas, et l'apprentissage sera extrêmement
     lent.

4. `batch_size` (Taille du lot)

   - À quoi ça sert ? L'agent apprend de ses expériences passées, qu'il stocke dans une "mémoire". Pour
     chaque session d'entraînement, au lieu d'apprendre d'une seule expérience, il en pioche un lot
     (batch) au hasard (ex: 32 expériences). Il calcule l'erreur moyenne sur ce lot pour mettre à jour
     ses poids.
   - Intuition : Apprendre sur un lot plutôt que sur une seule expérience rend l'apprentissage plus
     stable et plus général. L'agent se base sur une "moyenne" de situations plutôt que sur un cas
     particulier qui pourrait être une anomalie.

5. `clip_grad` (Plafonnement du gradient / _Gradient Clipping_)
   - À quoi ça sert ? C'est une sécurité pour éviter le problème des "gradients qui explosent". Parfois,
     une expérience très inattendue peut produire une erreur de prédiction énorme, ce qui entraîne une
     correction (un "gradient") démesurément grande. Une telle correction peut anéantir tout ce que le
     réseau a appris.
   - Intuition : clip_grad impose une limite. Si la correction à apporter dépasse un certain seuil, elle
     est ramenée à cette valeur maximale. C'est un garde-fou qui dit : "OK, on a fait une grosse erreur,
     mais on va corriger calmement, sans tout casser".
