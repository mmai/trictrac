import gymnasium as gym
import numpy as np
from gymnasium import spaces
# import trictrac  # module Rust exposé via PyO3
import store  # module Rust exposé via PyO3
from typing import Dict, List, Tuple, Optional, Any, Union

class TricTracEnv(gym.Env):
    """Environnement OpenAI Gym pour le jeu de Trictrac"""

    metadata = {"render.modes": ["human"]}

    def __init__(self, opponent_strategy="random"):
        super(TricTracEnv, self).__init__()

        # Instancier le jeu
        self.game = store.TricTrac()

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
        # Format: espace multidiscret avec 5 dimensions
        # - Action type: 0=move, 1=mark, 2=go (première dimension)
        # - Move: (from1, to1, from2, to2) (4 dernières dimensions)
        # Pour un total de 5 dimensions
        self.action_space = spaces.MultiDiscrete([
            3,  # Action type: 0=move, 1=mark, 2=go
            self.MAX_FIELD + 1,  # from1 (0 signifie non utilisé)
            self.MAX_FIELD + 1,  # to1
            self.MAX_FIELD + 1,  # from2
            self.MAX_FIELD + 1,  # to2
        ])

        # État courant
        self.state = self._get_observation()

        # Historique des états pour éviter les situations sans issue
        self.state_history = []

        # Pour le débogage et l'entraînement
        self.steps_taken = 0
        self.max_steps = 1000  # Limite pour éviter les parties infinies

    def reset(self, seed=None, options=None):
        """Réinitialise l'environnement et renvoie l'état initial"""
        super().reset(seed=seed)
        
        self.game.reset()
        self.state = self._get_observation()
        self.state_history = []
        self.steps_taken = 0
        
        return self.state, {}

    def step(self, action):
        """
        Exécute une action et retourne (state, reward, terminated, truncated, info)

        Action format: array de 5 entiers
        [action_type, from1, to1, from2, to2]
        - action_type: 0=move, 1=mark, 2=go
        - from1, to1, from2, to2: utilisés seulement si action_type=0
        """
        action_type = action[0]
        reward = 0
        terminated = False
        truncated = False
        info = {}

        # Vérifie que l'action est valide pour le joueur humain (id=1)
        player_id = self.game.get_active_player_id()
        is_agent_turn = player_id == 1  # L'agent joue toujours le joueur 1

        if is_agent_turn:
            # Exécute l'action selon son type
            if action_type == 0:  # Move
                from1, to1, from2, to2 = action[1], action[2], action[3], action[4]
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
            terminated = True
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
            truncated = True
            info['timeout'] = True

            # Comparer les scores en cas de timeout
            if agent_holes > opponent_holes:
                reward += 5.0
                info['winner'] = 'agent'
            elif opponent_holes > agent_holes:
                reward -= 2.0
                info['winner'] = 'opponent'

        self.state = new_state
        return self.state, reward, terminated, truncated, info

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
        # Pour le nouveau format d'action: [action_type, from1, to1, from2, to2]
        action_type_mask = np.zeros(3, dtype=bool)
        move_mask = np.zeros((self.MAX_FIELD + 1, self.MAX_FIELD + 1,
                             self.MAX_FIELD + 1, self.MAX_FIELD + 1), dtype=bool)

        if self.game.get_active_player_id() != 1:
            return action_type_mask, move_mask  # Pas au tour de l'agent

        # Activer les types d'actions valides selon l'étape du tour
        if turn_stage == 'Move' or turn_stage == 'HoldOrGoChoice':
            action_type_mask[0] = True  # Activer l'action de mouvement

            # Activer les mouvements valides
            valid_moves = self.game.get_available_moves()
            for ((from1, to1), (from2, to2)) in valid_moves:
                move_mask[from1, to1, from2, to2] = True

        if turn_stage == 'MarkPoints' or turn_stage == 'MarkAdvPoints':
            action_type_mask[1] = True  # Activer l'action de marquer des points

        if turn_stage == 'HoldOrGoChoice':
            action_type_mask[2] = True  # Activer l'action de continuer (Go)

        return action_type_mask, move_mask

    def sample_valid_action(self):
        """Échantillonne une action valide selon le masque d'actions"""
        action_type_mask, move_mask = self.get_action_mask()

        # Trouver les types d'actions valides
        valid_action_types = np.where(action_type_mask)[0]

        if len(valid_action_types) == 0:
            # Aucune action valide (pas le tour de l'agent)
            return np.array([0, 0, 0, 0, 0], dtype=np.int32)

        # Choisir un type d'action
        action_type = np.random.choice(valid_action_types)

        # Initialiser l'action
        action = np.array([action_type, 0, 0, 0, 0], dtype=np.int32)

        # Si c'est un mouvement, sélectionner un mouvement valide
        if action_type == 0:
            valid_moves = np.where(move_mask)
            if len(valid_moves[0]) > 0:
                # Sélectionner un mouvement valide aléatoirement
                idx = np.random.randint(0, len(valid_moves[0]))
                from1 = valid_moves[0][idx]
                to1 = valid_moves[1][idx]
                from2 = valid_moves[2][idx]
                to2 = valid_moves[3][idx]
                action[1:] = [from1, to1, from2, to2]

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
    obs, _ = env.reset()

    print("Environnement initialisé")
    env.render()

    # Jouer quelques coups aléatoires
    for _ in range(10):
        action = env.sample_valid_action()
        obs, reward, terminated, truncated, info = env.step(action)

        print(f"\nAction: {action}")
        print(f"Reward: {reward}")
        print(f"Terminated: {terminated}")
        print(f"Truncated: {truncated}")
        print(f"Info: {info}")
        env.render()

        if terminated or truncated:
            print("Game over!")
            break

    env.close()
