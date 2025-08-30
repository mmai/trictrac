# Description

Je développe un jeu de TricTrac (<https://fr.wikipedia.org/wiki/Trictrac>) dans le langage rust.
Pour le moment je me concentre sur l'application en ligne de commande simple, donc ne t'occupe pas des dossiers 'client_bevy', 'client_tui', et 'server' qui ne seront utilisés que pour de prochaines évolutions.

Les règles du jeu et l'état d'une partie sont implémentées dans 'store', l'application ligne de commande est implémentée dans 'client_cli', elle permet déjà de jouer contre un bot, ou de faire jouer deux bots l'un contre l'autre.
Les stratégies de bots sont implémentées dans le dossier 'bot'.

Plus précisément, l'état du jeu est défini par le struct GameState dans store/src/game.rs, la méthode to_string_id() permet de coder cet état de manière compacte dans une chaîne de caractères, mais il n'y a pas l'historique des coups joués. Il y a aussi fmt::Display d'implémenté pour une representation textuelle plus lisible.

'client_cli/src/game_runner.rs' contient la logique permettant de faire jouer deux bots l'un contre l'autre.
'bot/src/default.rs' contient le code d'une stratégie de bot basique : il détermine la liste des mouvements valides (avec la méthode get_possible_moves_sequences de store::MoveRules) et joue simplement le premier de la liste.

J'aimerais maintenant ajouter une stratégie de bot plus forte en entrainant un agent/bot avec OpenAI Gym.

Pour bénéficier des bibliothèques disponibles en python, comme 'Stable-Baselines3', j'ai utilisé PyO3 pour appeler mon code rust depuis python. Le code rust exposé se trouve dans 'store/src/engine.rs', il ne contient pour le moment que du code d'exemple pour m'assurer que la passerelle entre python et rust fonctionne, mais tout est à implémenter : l'état du jeu, les mouvements disponibles, etc.

La classe python 'store/python/trictracEnv.py' est elle aussi presque vide, elle ne contient pour le moment qu'un squelette avec des fonctions à implémenter.

Peux-tu m'aider à modifier engine.rs et trictracenv.py pour permettre d'entrainer un agent (bot) ?

J'aimerais utiliser ce que j'ai déjà implémenté en rust : l'état d'une partie et les règles, et la fonction renvoyant la liste des coups disponibles pour un joueur à partir d'un état du jeu (get_possible_moves_sequences), mais je ne sais pas trop comment m'y prendre.

## Réponse de Claude

Je vois que je n'ai pas l'autorisation d'utiliser l'outil Edit. Je vais vous montrer les modifications nécessaires pour les deux fichiers principaux.

Voici le contenu à utiliser pour `store/src/engine.rs` :

```rust
//! # Expose trictrac game state and rules in a python module
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyTuple};

use crate::board::CheckerMove;
use crate::dice::Dice;
use crate::game::{GameEvent, GameState, Stage, TurnStage};
use crate::game_rules_moves::MoveRules;
use crate::game_rules_points::{PointsRules, PossibleJans};
use crate::player::{Color, Player, PlayerId};
use std::collections::HashMap;

#[pyclass]
struct TricTrac {
    game_state: GameState,
    dice_roll_sequence: Vec<(u8, u8)>,
    current_dice_index: usize,
}

#[pymethods]
impl TricTrac {
    #[new]
    fn new() -> Self {
        let mut game_state = GameState::new(false); // schools_enabled = false

        // Initialiser 2 joueurs
        game_state.init_player("player1");
        game_state.init_player("bot");

        // Commencer la partie avec le joueur 1
        game_state.consume(&GameEvent::BeginGame { goes_first: 1 });

        TricTrac {
            game_state,
            dice_roll_sequence: Vec::new(),
            current_dice_index: 0,
        }
    }

    /// Obtenir l'état du jeu sous forme de chaîne de caractères compacte
    fn get_state_id(&self) -> String {
        self.game_state.to_string_id()
    }

    /// Obtenir l'état du jeu sous forme de dictionnaire pour faciliter l'entrainement
    fn get_state_dict(&self) -> PyResult<Py<PyDict>> {
        let gil = Python::acquire_gil();
        let py = gil.python();

        let state_dict = PyDict::new(py);

        // Informations essentielles sur l'état du jeu
        state_dict.set_item("active_player", self.game_state.active_player_id)?;
        state_dict.set_item("stage", format!("{:?}", self.game_state.stage))?;
        state_dict.set_item("turn_stage", format!("{:?}", self.game_state.turn_stage))?;

        // Dés
        let (dice1, dice2) = self.game_state.dice.values;
        state_dict.set_item("dice", (dice1, dice2))?;

        // Points des joueurs
        if let Some(white_player) = self.game_state.get_white_player() {
            state_dict.set_item("white_points", white_player.points)?;
            state_dict.set_item("white_holes", white_player.holes)?;
        }

        if let Some(black_player) = self.game_state.get_black_player() {
            state_dict.set_item("black_points", black_player.points)?;
            state_dict.set_item("black_holes", black_player.holes)?;
        }

        // Positions des pièces
        let white_positions = self.get_checker_positions(Color::White);
        let black_positions = self.get_checker_positions(Color::Black);

        state_dict.set_item("white_positions", white_positions)?;
        state_dict.set_item("black_positions", black_positions)?;

        // État compact pour la comparaison d'états
        state_dict.set_item("state_id", self.game_state.to_string_id())?;

        Ok(state_dict.into())
    }

    /// Renvoie les positions des pièces pour un joueur spécifique
    fn get_checker_positions(&self, color: Color) -> Vec<(usize, i8)> {
        self.game_state.board.get_color_fields(color)
    }

    /// Obtenir la liste des mouvements légaux sous forme de paires (from, to)
    fn get_available_moves(&self) -> Vec<((usize, usize), (usize, usize))> {
        // L'agent joue toujours le joueur actif
        let color = self.game_state.player_color_by_id(&self.game_state.active_player_id).unwrap_or(Color::White);

        // Si ce n'est pas le moment de déplacer les pièces, retourner une liste vide
        if self.game_state.turn_stage != TurnStage::Move && self.game_state.turn_stage != TurnStage::HoldOrGoChoice {
            return vec![];
        }

        let rules = MoveRules::new(&color, &self.game_state.board, self.game_state.dice);
        let possible_moves = rules.get_possible_moves_sequences(true, vec![]);

        // Convertir les mouvements CheckerMove en tuples (from, to) pour Python
        possible_moves.into_iter()
            .map(|(move1, move2)| (
                (move1.get_from(), move1.get_to()),
                (move2.get_from(), move2.get_to())
            )).collect()
    }

    /// Jouer un coup ((from1, to1), (from2, to2))
    fn play_move(&mut self, moves: ((usize, usize), (usize, usize))) -> bool {
        let ((from1, to1), (from2, to2)) = moves;

        // Vérifier que c'est au tour du joueur de jouer
        if self.game_state.turn_stage != TurnStage::Move && self.game_state.turn_stage != TurnStage::HoldOrGoChoice {
            return false;
        }

        let move1 = CheckerMove::new(from1, to1).unwrap_or_default();
        let move2 = CheckerMove::new(from2, to2).unwrap_or_default();

        let event = GameEvent::Move {
            player_id: self.game_state.active_player_id,
            moves: (move1, move2),
        };

        // Vérifier si le mouvement est valide
        if !self.game_state.validate(&event) {
            return false;
        }

        // Exécuter le mouvement
        self.game_state.consume(&event);

        // Si l'autre joueur doit lancer les dés maintenant, simuler ce lancement
        if self.game_state.turn_stage == TurnStage::RollDice {
            self.roll_dice();
        }

        true
    }

    /// Lancer les dés (soit aléatoirement, soit en utilisant une séquence prédéfinie)
    fn roll_dice(&mut self) -> (u8, u8) {
        // Vérifier que c'est au bon moment pour lancer les dés
        if self.game_state.turn_stage != TurnStage::RollDice && self.game_state.turn_stage != TurnStage::RollWaiting {
            return self.game_state.dice.values;
        }

        // Simuler un lancer de dés
        let dice_values = if !self.dice_roll_sequence.is_empty() && self.current_dice_index < self.dice_roll_sequence.len() {
            // Utiliser la séquence prédéfinie
            let dice = self.dice_roll_sequence[self.current_dice_index];
            self.current_dice_index += 1;
            dice
        } else {
            // Générer aléatoirement
            (
                (1 + (rand::random::<u8>() % 6)),
                (1 + (rand::random::<u8>() % 6))
            )
        };

        // Envoyer les événements appropriés
        let roll_event = GameEvent::Roll {
            player_id: self.game_state.active_player_id,
        };

        if self.game_state.validate(&roll_event) {
            self.game_state.consume(&roll_event);
        }

        let roll_result_event = GameEvent::RollResult {
            player_id: self.game_state.active_player_id,
            dice: Dice { values: dice_values },
        };

        if self.game_state.validate(&roll_result_event) {
            self.game_state.consume(&roll_result_event);
        }

        dice_values
    }

    /// Marquer des points
    fn mark_points(&mut self, points: u8) -> bool {
        // Vérifier que c'est au bon moment pour marquer des points
        if self.game_state.turn_stage != TurnStage::MarkPoints && self.game_state.turn_stage != TurnStage::MarkAdvPoints {
            return false;
        }

        let event = GameEvent::Mark {
            player_id: self.game_state.active_player_id,
            points,
        };

        // Vérifier si l'événement est valide
        if !self.game_state.validate(&event) {
            return false;
        }

        // Exécuter l'événement
        self.game_state.consume(&event);

        // Si l'autre joueur doit lancer les dés maintenant, simuler ce lancement
        if self.game_state.turn_stage == TurnStage::RollDice {
            self.roll_dice();
        }

        true
    }

    /// Choisir de "continuer" (Go) après avoir gagné un trou
    fn choose_go(&mut self) -> bool {
        // Vérifier que c'est au bon moment pour choisir de continuer
        if self.game_state.turn_stage != TurnStage::HoldOrGoChoice {
            return false;
        }

        let event = GameEvent::Go {
            player_id: self.game_state.active_player_id,
        };

        // Vérifier si l'événement est valide
        if !self.game_state.validate(&event) {
            return false;
        }

        // Exécuter l'événement
        self.game_state.consume(&event);

        // Simuler le lancer de dés pour le prochain tour
        self.roll_dice();

        true
    }

    /// Calcule les points maximaux que le joueur actif peut obtenir avec les dés actuels
    fn calculate_points(&self) -> u8 {
        let active_player = self.game_state.players.get(&self.game_state.active_player_id);

        if let Some(player) = active_player {
            let dice_roll_count = player.dice_roll_count;
            let color = player.color;

            let points_rules = PointsRules::new(&color, &self.game_state.board, self.game_state.dice);
            let (points, _) = points_rules.get_points(dice_roll_count);

            points
        } else {
            0
        }
    }

    /// Réinitialise la partie
    fn reset(&mut self) {
        self.game_state = GameState::new(false);

        // Initialiser 2 joueurs
        self.game_state.init_player("player1");
        self.game_state.init_player("bot");

        // Commencer la partie avec le joueur 1
        self.game_state.consume(&GameEvent::BeginGame { goes_first: 1 });

        // Réinitialiser l'index de la séquence de dés
        self.current_dice_index = 0;
    }

    /// Vérifie si la partie est terminée
    fn is_done(&self) -> bool {
        self.game_state.stage == Stage::Ended || self.game_state.determine_winner().is_some()
    }

    /// Obtenir le gagnant de la partie
    fn get_winner(&self) -> Option<PlayerId> {
        self.game_state.determine_winner()
    }

    /// Obtenir le score du joueur actif (nombre de trous)
    fn get_score(&self, player_id: PlayerId) -> i32 {
        if let Some(player) = self.game_state.players.get(&player_id) {
            player.holes as i32
        } else {
            -1
        }
    }

    /// Obtenir l'ID du joueur actif
    fn get_active_player_id(&self) -> PlayerId {
        self.game_state.active_player_id
    }

    /// Définir une séquence de dés à utiliser (pour la reproductibilité)
    fn set_dice_sequence(&mut self, sequence: Vec<(u8, u8)>) {
        self.dice_roll_sequence = sequence;
        self.current_dice_index = 0;
    }

    /// Afficher l'état du jeu (pour le débogage)
    fn __str__(&self) -> String {
        format!("{}", self.game_state)
    }
}

/// A Python module implemented in Rust. The name of this function must match
/// the `lib.name` setting in the `Cargo.toml`, else Python will not be able to
/// import the module.
#[pymodule]
fn trictrac(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<TricTrac>()?;

    Ok(())
}
```

Et pour `store/python/trictracEnv.py` :

```python
import gym
import numpy as np
from gym import spaces
import trictrac  # module Rust exposé via PyO3
from typing import Dict, List, Tuple, Optional, Any, Union

class TricTracEnv(gym.Env):
    """Environnement OpenAI Gym pour le jeu de Trictrac"""

    metadata = {"render.modes": ["human"]}

    def __init__(self, opponent_strategy="random"):
        super(TricTracEnv, self).__init__()

        # Instancier le jeu
        self.game = trictrac.TricTrac()

        # Stratégie de l'adversaire
        self.opponent_strategy = opponent_strategy

        # Constantes
        self.MAX_FIELD = 24  # Nombre de cases sur le plateau
        self.MAX_CHECKERS = 15  # Nombre maximum de pièces par joueur

        # Définition de l'espace d'observation
        # Format:
        # - Position des pièces blanches (24)
        # - Position des pièces noires (24)
        # - Joueur actif (1: blanc, 2: noir) (1)
        # - Valeurs des dés (2)
        # - Points de chaque joueur (2)
        # - Trous de chaque joueur (2)
        # - Phase du jeu (1)
        self.observation_space = spaces.Dict({
            'board': spaces.Box(low=-self.MAX_CHECKERS, high=self.MAX_CHECKERS, shape=(self.MAX_FIELD,), dtype=np.int8),
            'active_player': spaces.Discrete(3),  # 0: pas de joueur, 1: blanc, 2: noir
            'dice': spaces.MultiDiscrete([7, 7]),  # Valeurs des dés (1-6)
            'white_points': spaces.Discrete(13),  # Points du joueur blanc (0-12)
            'white_holes': spaces.Discrete(13),   # Trous du joueur blanc (0-12)
            'black_points': spaces.Discrete(13),  # Points du joueur noir (0-12)
            'black_holes': spaces.Discrete(13),   # Trous du joueur noir (0-12)
            'turn_stage': spaces.Discrete(6),     # Étape du tour
        })

        # Définition de l'espace d'action
        # Format:
        # - Action type: 0=move, 1=mark, 2=go
        # - Move: (from1, to1, from2, to2) ou zeros
        self.action_space = spaces.Dict({
            'action_type': spaces.Discrete(3),
            'move': spaces.MultiDiscrete([self.MAX_FIELD + 1, self.MAX_FIELD + 1,
                                         self.MAX_FIELD + 1, self.MAX_FIELD + 1])
        })

        # État courant
        self.state = self._get_observation()

        # Historique des états pour éviter les situations sans issue
        self.state_history = []

        # Pour le débogage et l'entraînement
        self.steps_taken = 0
        self.max_steps = 1000  # Limite pour éviter les parties infinies

    def reset(self):
        """Réinitialise l'environnement et renvoie l'état initial"""
        self.game.reset()
        self.state = self._get_observation()
        self.state_history = []
        self.steps_taken = 0
        return self.state

    def step(self, action):
        """
        Exécute une action et retourne (state, reward, done, info)

        Action format:
        {
            'action_type': 0/1/2,  # 0=move, 1=mark, 2=go
            'move': [from1, to1, from2, to2]  # Utilisé seulement si action_type=0
        }
        """
        action_type = action['action_type']
        reward = 0
        done = False
        info = {}

        # Vérifie que l'action est valide pour le joueur humain (id=1)
        player_id = self.game.get_active_player_id()
        is_agent_turn = player_id == 1  # L'agent joue toujours le joueur 1

        if is_agent_turn:
            # Exécute l'action selon son type
            if action_type == 0:  # Move
                from1, to1, from2, to2 = action['move']
                move_made = self.game.play_move(((from1, to1), (from2, to2)))
                if not move_made:
                    # Pénaliser les mouvements invalides
                    reward -= 2.0
                    info['invalid_move'] = True
                else:
                    # Petit bonus pour un mouvement valide
                    reward += 0.1
            elif action_type == 1:  # Mark
                points = self.game.calculate_points()
                marked = self.game.mark_points(points)
                if not marked:
                    # Pénaliser les actions invalides
                    reward -= 2.0
                    info['invalid_mark'] = True
                else:
                    # Bonus pour avoir marqué des points
                    reward += 0.1 * points
            elif action_type == 2:  # Go
                go_made = self.game.choose_go()
                if not go_made:
                    # Pénaliser les actions invalides
                    reward -= 2.0
                    info['invalid_go'] = True
                else:
                    # Petit bonus pour l'action valide
                    reward += 0.1
        else:
            # Tour de l'adversaire
            self._play_opponent_turn()

        # Vérifier si la partie est terminée
        if self.game.is_done():
            done = True
            winner = self.game.get_winner()
            if winner == 1:
                # Bonus si l'agent gagne
                reward += 10.0
                info['winner'] = 'agent'
            else:
                # Pénalité si l'adversaire gagne
                reward -= 5.0
                info['winner'] = 'opponent'

        # Récompense basée sur la progression des trous
        agent_holes = self.game.get_score(1)
        opponent_holes = self.game.get_score(2)
        reward += 0.5 * (agent_holes - opponent_holes)

        # Mettre à jour l'état
        new_state = self._get_observation()

        # Vérifier les états répétés
        if self._is_state_repeating(new_state):
            reward -= 0.2  # Pénalité légère pour éviter les boucles
            info['repeating_state'] = True

        # Ajouter l'état à l'historique
        self.state_history.append(self._get_state_id())

        # Limiter la durée des parties
        self.steps_taken += 1
        if self.steps_taken >= self.max_steps:
            done = True
            info['timeout'] = True

            # Comparer les scores en cas de timeout
            if agent_holes > opponent_holes:
                reward += 5.0
                info['winner'] = 'agent'
            elif opponent_holes > agent_holes:
                reward -= 2.0
                info['winner'] = 'opponent'

        self.state = new_state
        return self.state, reward, done, info

    def _play_opponent_turn(self):
        """Simule le tour de l'adversaire avec la stratégie choisie"""
        player_id = self.game.get_active_player_id()

        # Boucle tant qu'il est au tour de l'adversaire
        while player_id == 2 and not self.game.is_done():
            # Action selon l'étape du tour
            state_dict = self._get_state_dict()
            turn_stage = state_dict.get('turn_stage')

            if turn_stage == 'RollDice' or turn_stage == 'RollWaiting':
                self.game.roll_dice()
            elif turn_stage == 'MarkPoints' or turn_stage == 'MarkAdvPoints':
                points = self.game.calculate_points()
                self.game.mark_points(points)
            elif turn_stage == 'HoldOrGoChoice':
                # Stratégie simple: toujours continuer (Go)
                self.game.choose_go()
            elif turn_stage == 'Move':
                available_moves = self.game.get_available_moves()
                if available_moves:
                    if self.opponent_strategy == "random":
                        # Choisir un mouvement au hasard
                        move = available_moves[np.random.randint(0, len(available_moves))]
                    else:
                        # Par défaut, prendre le premier mouvement valide
                        move = available_moves[0]
                    self.game.play_move(move)

            # Mise à jour de l'ID du joueur actif
            player_id = self.game.get_active_player_id()

    def _get_observation(self):
        """Convertit l'état du jeu en un format utilisable par l'apprentissage par renforcement"""
        state_dict = self._get_state_dict()

        # Créer un tableau représentant le plateau
        board = np.zeros(self.MAX_FIELD, dtype=np.int8)

        # Remplir les positions des pièces blanches (valeurs positives)
        white_positions = state_dict.get('white_positions', [])
        for pos, count in white_positions:
            if 1 <= pos <= self.MAX_FIELD:
                board[pos-1] = count

        # Remplir les positions des pièces noires (valeurs négatives)
        black_positions = state_dict.get('black_positions', [])
        for pos, count in black_positions:
            if 1 <= pos <= self.MAX_FIELD:
                board[pos-1] = -count

        # Créer l'observation complète
        observation = {
            'board': board,
            'active_player': state_dict.get('active_player', 0),
            'dice': np.array([
                state_dict.get('dice', (1, 1))[0],
                state_dict.get('dice', (1, 1))[1]
            ]),
            'white_points': state_dict.get('white_points', 0),
            'white_holes': state_dict.get('white_holes', 0),
            'black_points': state_dict.get('black_points', 0),
            'black_holes': state_dict.get('black_holes', 0),
            'turn_stage': self._turn_stage_to_int(state_dict.get('turn_stage', 'RollDice')),
        }

        return observation

    def _get_state_dict(self) -> Dict:
        """Récupère l'état du jeu sous forme de dictionnaire depuis le module Rust"""
        return self.game.get_state_dict()

    def _get_state_id(self) -> str:
        """Récupère l'identifiant unique de l'état actuel"""
        return self.game.get_state_id()

    def _is_state_repeating(self, new_state) -> bool:
        """Vérifie si l'état se répète trop souvent"""
        state_id = self.game.get_state_id()
        # Compter les occurrences de l'état dans l'historique récent
        count = sum(1 for s in self.state_history[-10:] if s == state_id)
        return count >= 3  # Considéré comme répétitif si l'état apparaît 3 fois ou plus

    def _turn_stage_to_int(self, turn_stage: str) -> int:
        """Convertit l'étape du tour en entier pour l'observation"""
        stages = {
            'RollDice': 0,
            'RollWaiting': 1,
            'MarkPoints': 2,
            'HoldOrGoChoice': 3,
            'Move': 4,
            'MarkAdvPoints': 5
        }
        return stages.get(turn_stage, 0)

    def render(self, mode="human"):
        """Affiche l'état actuel du jeu"""
        if mode == "human":
            print(str(self.game))
            print(f"État actuel: {self._get_state_id()}")

            # Afficher les actions possibles
            if self.game.get_active_player_id() == 1:
                turn_stage = self._get_state_dict().get('turn_stage')
                print(f"Étape: {turn_stage}")

                if turn_stage == 'Move' or turn_stage == 'HoldOrGoChoice':
                    print("Mouvements possibles:")
                    moves = self.game.get_available_moves()
                    for i, move in enumerate(moves):
                        print(f"  {i}: {move}")

                if turn_stage == 'HoldOrGoChoice':
                    print("Option: Go (continuer)")

    def get_action_mask(self):
        """Retourne un masque des actions valides dans l'état actuel"""
        state_dict = self._get_state_dict()
        turn_stage = state_dict.get('turn_stage')

        # Masque par défaut (toutes les actions sont invalides)
        mask = {
            'action_type': np.zeros(3, dtype=bool),
            'move': np.zeros((self.MAX_FIELD + 1, self.MAX_FIELD + 1,
                             self.MAX_FIELD + 1, self.MAX_FIELD + 1), dtype=bool)
        }

        if self.game.get_active_player_id() != 1:
            return mask  # Pas au tour de l'agent

        # Activer les types d'actions valides selon l'étape du tour
        if turn_stage == 'Move' or turn_stage == 'HoldOrGoChoice':
            mask['action_type'][0] = True  # Activer l'action de mouvement

            # Activer les mouvements valides
            valid_moves = self.game.get_available_moves()
            for ((from1, to1), (from2, to2)) in valid_moves:
                mask['move'][from1, to1, from2, to2] = True

        if turn_stage == 'MarkPoints' or turn_stage == 'MarkAdvPoints':
            mask['action_type'][1] = True  # Activer l'action de marquer des points

        if turn_stage == 'HoldOrGoChoice':
            mask['action_type'][2] = True  # Activer l'action de continuer (Go)

        return mask

    def sample_valid_action(self):
        """Échantillonne une action valide selon le masque d'actions"""
        mask = self.get_action_mask()

        # Trouver les types d'actions valides
        valid_action_types = np.where(mask['action_type'])[0]

        if len(valid_action_types) == 0:
            # Aucune action valide (pas le tour de l'agent)
            return {
                'action_type': 0,
                'move': np.zeros(4, dtype=np.int32)
            }

        # Choisir un type d'action
        action_type = np.random.choice(valid_action_types)

        action = {
            'action_type': action_type,
            'move': np.zeros(4, dtype=np.int32)
        }

        # Si c'est un mouvement, sélectionner un mouvement valide
        if action_type == 0:
            valid_moves = np.where(mask['move'])
            if len(valid_moves[0]) > 0:
                # Sélectionner un mouvement valide aléatoirement
                idx = np.random.randint(0, len(valid_moves[0]))
                from1 = valid_moves[0][idx]
                to1 = valid_moves[1][idx]
                from2 = valid_moves[2][idx]
                to2 = valid_moves[3][idx]
                action['move'] = np.array([from1, to1, from2, to2], dtype=np.int32)

        return action

    def close(self):
        """Nettoie les ressources à la fermeture de l'environnement"""
        pass

# Exemple d'utilisation avec Stable-Baselines3
def example_usage():
    from stable_baselines3 import PPO
    from stable_baselines3.common.vec_env import DummyVecEnv

    # Fonction d'enveloppement pour créer l'environnement
    def make_env():
        return TricTracEnv()

    # Créer un environnement vectorisé (peut être parallélisé)
    env = DummyVecEnv([make_env])

    # Créer le modèle
    model = PPO("MultiInputPolicy", env, verbose=1)

    # Entraîner le modèle
    model.learn(total_timesteps=10000)

    # Sauvegarder le modèle
    model.save("trictrac_ppo")

    print("Entraînement terminé et modèle sauvegardé")

if __name__ == "__main__":
    # Tester l'environnement
    env = TricTracEnv()
    obs = env.reset()

    print("Environnement initialisé")
    env.render()

    # Jouer quelques coups aléatoires
    for _ in range(10):
        action = env.sample_valid_action()
        obs, reward, done, info = env.step(action)

        print(f"\nAction: {action}")
        print(f"Reward: {reward}")
        print(f"Info: {info}")
        env.render()

        if done:
            print("Game over!")
            break

    env.close()
```

Ces implémentations vous permettront de créer un environnement Gym pour votre jeu Trictrac qui:

1. Expose l'état du jeu dans un format approprié pour l'apprentissage par renforcement
2. Gère le déroulement des parties et le tour de l'adversaire
3. Définit un système de récompenses pour l'entraînement de l'agent
4. Fournit des masques d'actions pour garantir que seules les actions valides sont sélectionnées
5. Inclut des exemples pour l'intégration avec Stable-Baselines3

Ce code nécessitera probablement quelques ajustements en fonction de la façon dont votre code Rust est organisé, mais il devrait vous donner une base solide pour commencer.

Pour utiliser cette implémentation :

1. Mettez à jour `engine.rs` pour exposer l'API nécessaire à Python
2. Compilez la bibliothèque Rust avec maturin : `maturin build -m store/Cargo.toml --release`
3. Implémentez `trictracEnv.py` avec l'environnement Gym
4. Créez un script d'entraînement qui utilise Stable-Baselines3 ou une autre bibliothèque d'apprentissage par renforcement

Voulez-vous que j'explique plus en détail certaines parties de l'implémentation ou que je vous aide à résoudre des problèmes spécifiques ?
