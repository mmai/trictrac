import gym
import numpy as np
from gym import spaces
import trictrac  # module Rust exposé via PyO3

class TricTracEnv(gym.Env):
    """Environnement OpenAI Gym pour le jeu de Trictrac"""
    
    def __init__(self):
        super(TricTracEnv, self).__init__()

        # Définition des espaces d'observation et d'action
        self.observation_space = spaces.Box(low=0, high=1, shape=(N,), dtype=np.int32)  # Exemple
        self.action_space = spaces.Discrete(ACTION_COUNT)  # Exemple
        
        self.game = trictrac.TricTrac()  # Instance du jeu en Rust
        self.state = self.game.get_state()  # État initial

    def step(self, action):
        """Exécute une action et retourne (next_state, reward, done, info)"""
        self.game.play(action)  
        self.state = self.game.get_state()
        
        reward = self.compute_reward()
        done = self.game.is_done()
        
        return self.state, reward, done, {}

    def reset(self):
        """Réinitialise la partie"""
        self.game.reset()
        self.state = self.game.get_state()
        return self.state

    def render(self, mode="human"):
        """Affiche l'état du jeu"""
        print(self.game)

    def compute_reward(self):
        """Calcule la récompense (à définir)"""
        return 0  # À affiner selon la stratégie d'entraînement

