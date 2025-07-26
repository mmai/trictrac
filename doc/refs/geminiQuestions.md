# Description du projet et question

Je développe un jeu de TricTrac (<https://fr.wikipedia.org/wiki/Trictrac>) dans le langage rust.
Pour le moment je me concentre sur l'application en ligne de commande simple, donc ne t'occupe pas des dossiers 'client_bevy', 'client_tui', et 'server' qui ne seront utilisés que pour de prochaines évolutions.

Les règles du jeu et l'état d'une partie sont implémentées dans 'store', l'application ligne de commande est implémentée dans 'client_cli', elle permet déjà de jouer contre un bot, ou de faire jouer deux bots l'un contre l'autre.
Les stratégies de bots sont implémentées dans le dossier 'bot'.

Plus précisément, l'état du jeu est défini par le struct GameState dans store/src/game.rs, la méthode to_string_id() permet de coder cet état de manière compacte dans une chaîne de caractères, mais il n'y a pas l'historique des coups joués. Il y a aussi fmt::Display d'implémenté pour une representation textuelle plus lisible.

'client_cli/src/game_runner.rs' contient la logique permettant de faire jouer deux bots l'un contre l'autre.
'bot/src/strategy/default.rs' contient le code d'une stratégie de bot basique : il détermine la liste des mouvements valides (avec la méthode get_possible_moves_sequences de store::MoveRules) et joue simplement le premier de la liste.

Je cherche maintenant à ajouter des stratégies de bot plus fortes en entrainant un agent/bot par reinforcement learning.

Une première version avec DQN fonctionne (entraînement avec `cargo run -bin=train_dqn`)
Il gagne systématiquement contre le bot par défaut 'dummy' : `cargo run --bin=client_cli -- --bot dqn:./models/dqn_model_final.json,dummy`.

Une version, toujours DQN, mais en utilisant la bibliothèque burn (<https://burn.dev/>) est en cours de développement.

L'entraînement du modèle se passe dans la fonction "main" du fichier bot/src/burnrl/main.rs. On peut lancer l'exécution avec 'just trainbot'.

Voici la sortie de l'entraînement lancé avec 'just trainbot' :

```
> Entraînement
> {"episode": 0, "reward": -1692.3148, "duration": 1000}
> {"episode": 1, "reward": -361.6962, "duration": 1000}
> {"episode": 2, "reward": -126.1013, "duration": 1000}
> {"episode": 3, "reward": -36.8000, "duration": 1000}
> {"episode": 4, "reward": -21.4997, "duration": 1000}
> {"episode": 5, "reward": -8.3000, "duration": 1000}
> {"episode": 6, "reward": 3.1000, "duration": 1000}
> {"episode": 7, "reward": -21.5998, "duration": 1000}
> {"episode": 8, "reward": -10.1999, "duration": 1000}
> {"episode": 9, "reward": 3.1000, "duration": 1000}
> {"episode": 10, "reward": 14.5002, "duration": 1000}
> {"episode": 11, "reward": 10.7000, "duration": 1000}
> {"episode": 12, "reward": -0.7000, "duration": 1000}

thread 'main' has overflowed its stack
fatal runtime error: stack overflow
error: Recipe `trainbot` was terminated on line 25 by signal 6
```

Au bout du 12ème épisode (plus de 6 heures sur ma machine), l'entraînement s'arrête avec une erreur stack overlow. Peux-tu m'aider à diagnostiquer d'où peut provenir le problème ? Y a-t-il des outils qui permettent de détecter les zones de code qui utilisent le plus la stack ? Pour information j'ai vu ce rapport de bug https://github.com/yunjhongwu/burn-rl-examples/issues/40, donc peut-être que le problème vient du paquet 'burl-rl'.
