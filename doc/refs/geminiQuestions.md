# Description du projet

Je développe un jeu de TricTrac (<https://fr.wikipedia.org/wiki/Trictrac>) dans le langage rust.
Pour le moment je me concentre sur l'application en ligne de commande simple, donc ne t'occupe pas des dossiers 'client_bevy', 'client_tui', et 'server' qui ne seront utilisés que pour de prochaines évolutions.

Les règles du jeu et l'état d'une partie sont implémentées dans 'store', l'application ligne de commande est implémentée dans 'client_cli', elle permet déjà de jouer contre un bot, ou de faire jouer deux bots l'un contre l'autre.
Les stratégies de bots sont implémentées dans le dossier 'bot'.

Plus précisément, l'état du jeu est défini par le struct GameState dans store/src/game.rs, la méthode to_string_id() permet de coder cet état de manière compacte dans une chaîne de caractères, mais il n'y a pas l'historique des coups joués. Il y a aussi fmt::Display d'implémenté pour une representation textuelle plus lisible.

'client_cli/src/game_runner.rs' contient la logique permettant de faire jouer deux bots l'un contre l'autre.
'bot/src/strategy/default.rs' contient le code d'une stratégie de bot basique : il détermine la liste des mouvements valides (avec la méthode get_possible_moves_sequences de store::MoveRules) et joue simplement le premier de la liste.

Je cherche maintenant à ajouter des stratégies de bot plus fortes en entrainant un agent/bot par reinforcement learning.
J'utilise la bibliothèque burn (<https://burn.dev/>).

Une version utilisant l'algorithme DQN peut être lancée avec `cargo run --bin=burn_train -- dqn`). Elle effectue un entraînement, sauvegarde les données du modèle obtenu puis recharge le modèle depuis le disque pour tester l'agent. L'entraînement est fait dans la fonction 'run' du fichier bot/src/burnrl/dqn_model.rs, la sauvegarde du modèle dans la fonction 'save_model' et le chargement dans la fonction 'load_model'.

J'essaie de faire l'équivalent avec les algorithmes PPO (fichier bot/src/burnrl/ppo_model.rs) et SAC (fichier bot/src/burnrl/sac_model.rs) : les fonctions 'run' sont implémentées mais pas les fonctions 'save_model' et 'load_model'. Peux-tu les implémenter ?
