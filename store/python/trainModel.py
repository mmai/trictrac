from stable_baselines3 import PPO
from stable_baselines3.common.vec_env import DummyVecEnv
from trictracEnv import TricTracEnv
import os
import torch
import sys

# Vérifier si le GPU est disponible
try:
    if torch.cuda.is_available():
        device = torch.device("cuda")
        print(f"GPU disponible: {torch.cuda.get_device_name(0)}")
        print(f"CUDA version: {torch.version.cuda}")
        print(f"Using device: {device}")
    else:
        device = torch.device("cpu")
        print("GPU non disponible, utilisation du CPU")
        print(f"Using device: {device}")
except Exception as e:
    print(f"Erreur lors de la vérification de la disponibilité du GPU: {e}")
    device = torch.device("cpu")
    print(f"Using device: {device}")

# Créer l'environnement vectorisé
env = DummyVecEnv([lambda: TricTracEnv()])

try:
    # Créer et entraîner le modèle avec support GPU si disponible
    model = PPO("MultiInputPolicy", env, verbose=1, device=device)
    
    print("Démarrage de l'entraînement...")
    # Petit entraînement pour tester
    # model.learn(total_timesteps=50)
    # Entraînement complet
    model.learn(total_timesteps=50000)
    print("Entraînement terminé")
    
except Exception as e:
    print(f"Erreur lors de l'entraînement: {e}")
    sys.exit(1)

# Sauvegarder le modèle
os.makedirs("models", exist_ok=True)
model.save("models/trictrac_ppo")

# Test du modèle entraîné
obs = env.reset()
for _ in range(100):
    action, _ = model.predict(obs)
    # L'interface de DummyVecEnv ne retourne que 4 valeurs
    obs, _, done, _ = env.step(action)
    if done.any():
        break
