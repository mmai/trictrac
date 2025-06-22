use bot::strategy::burn_environment::{TrictracAction, TrictracEnvironment};
use bot::strategy::dqn_common::get_valid_actions;
use burn_rl::base::Environment;
use rand::Rng;
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let args: Vec<String> = env::args().collect();

    // Paramètres par défaut
    let mut episodes = 1000;
    let mut save_every = 100;
    let mut max_steps_per_episode = 500;

    // Parser les arguments de ligne de commande
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--episodes" => {
                if i + 1 < args.len() {
                    episodes = args[i + 1].parse().unwrap_or(1000);
                    i += 2;
                } else {
                    eprintln!("Erreur : --episodes nécessite une valeur");
                    std::process::exit(1);
                }
            }
            "--save-every" => {
                if i + 1 < args.len() {
                    save_every = args[i + 1].parse().unwrap_or(100);
                    i += 2;
                } else {
                    eprintln!("Erreur : --save-every nécessite une valeur");
                    std::process::exit(1);
                }
            }
            "--max-steps" => {
                if i + 1 < args.len() {
                    max_steps_per_episode = args[i + 1].parse().unwrap_or(500);
                    i += 2;
                } else {
                    eprintln!("Erreur : --max-steps nécessite une valeur");
                    std::process::exit(1);
                }
            }
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            _ => {
                eprintln!("Argument inconnu : {}", args[i]);
                print_help();
                std::process::exit(1);
            }
        }
    }

    println!("=== Entraînement DQN avec Burn-RL ===");
    println!("Épisodes : {}", episodes);
    println!("Sauvegarde tous les {} épisodes", save_every);
    println!("Max steps par épisode : {}", max_steps_per_episode);
    println!();

    // Créer l'environnement
    let mut env = TrictracEnvironment::new(true);
    let mut rng = rand::thread_rng();

    // Variables pour les statistiques
    let mut total_rewards = Vec::new();
    let mut episode_lengths = Vec::new();
    let mut epsilon = 1.0; // Exploration rate
    let epsilon_decay = 0.995;
    let epsilon_min = 0.01;

    println!("Début de l'entraînement...");
    println!();

    for episode in 1..=episodes {
        // Reset de l'environnement
        let mut snapshot = env.reset();
        let mut episode_reward = 0.0;
        let mut step = 0;

        loop {
            step += 1;
            let current_state = snapshot.state();

            // Obtenir les actions valides selon le contexte du jeu
            let valid_actions = get_valid_actions(&env.game);

            if valid_actions.is_empty() {
                if env.visualized && episode % 50 == 0 {
                    println!("  Pas d'actions valides disponibles à l'étape {}", step);
                }
                break;
            }

            // Sélection d'action epsilon-greedy simple
            let action = if rng.gen::<f32>() < epsilon {
                // Exploration : action aléatoire parmi les valides
                let random_valid_index = rng.gen_range(0..valid_actions.len());
                TrictracAction {
                    index: random_valid_index as u32,
                }
            } else {
                // Exploitation : action simple (première action valide pour l'instant)
                TrictracAction { index: 0 }
            };

            // Exécuter l'action
            snapshot = env.step(action);
            episode_reward += snapshot.reward();

            if env.visualized && episode % 50 == 0 && step % 10 == 0 {
                println!(
                    "  Episode {}, Step {}, Reward: {:.3}, Action: {}",
                    episode,
                    step,
                    snapshot.reward(),
                    action.index
                );
            }

            // Vérifier les conditions de fin
            if snapshot.done() || step >= max_steps_per_episode {
                break;
            }
        }

        // Décroissance epsilon
        if epsilon > epsilon_min {
            epsilon *= epsilon_decay;
        }

        // Sauvegarder les statistiques
        total_rewards.push(episode_reward);
        episode_lengths.push(step);

        // Affichage des statistiques
        if episode % save_every == 0 {
            let avg_reward =
                total_rewards.iter().rev().take(save_every).sum::<f32>() / save_every as f32;
            let avg_length =
                episode_lengths.iter().rev().take(save_every).sum::<usize>() / save_every;

            println!(
                "Episode {} | Avg Reward: {:.3} | Avg Length: {} | Epsilon: {:.3}",
                episode, avg_reward, avg_length, epsilon
            );

            // Ici on pourrait sauvegarder un modèle si on en avait un
            println!("  → Checkpoint atteint (pas de modèle à sauvegarder pour l'instant)");
        } else if episode % 10 == 0 {
            println!(
                "Episode {} | Reward: {:.3} | Length: {} | Epsilon: {:.3}",
                episode, episode_reward, step, epsilon
            );
        }
    }

    // Statistiques finales
    println!();
    println!("=== Résultats de l'entraînement ===");
    let final_avg_reward = total_rewards
        .iter()
        .rev()
        .take(100.min(episodes))
        .sum::<f32>()
        / 100.min(episodes) as f32;
    let final_avg_length = episode_lengths
        .iter()
        .rev()
        .take(100.min(episodes))
        .sum::<usize>()
        / 100.min(episodes);

    println!(
        "Récompense moyenne (100 derniers épisodes) : {:.3}",
        final_avg_reward
    );
    println!(
        "Longueur moyenne (100 derniers épisodes) : {}",
        final_avg_length
    );
    println!("Epsilon final : {:.3}", epsilon);

    // Statistiques globales
    let max_reward = total_rewards
        .iter()
        .cloned()
        .fold(f32::NEG_INFINITY, f32::max);
    let min_reward = total_rewards.iter().cloned().fold(f32::INFINITY, f32::min);
    println!("Récompense max : {:.3}", max_reward);
    println!("Récompense min : {:.3}", min_reward);

    println!();
    println!("Entraînement terminé avec succès !");
    println!("L'environnement Burn-RL fonctionne correctement.");

    Ok(())
}

fn print_help() {
    println!("Entraîneur DQN avec Burn-RL pour Trictrac");
    println!();
    println!("USAGE:");
    println!("  cargo run --bin=train_burn_rl [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("  --episodes <NUM>      Nombre d'épisodes d'entraînement (défaut: 1000)");
    println!("  --save-every <NUM>    Afficher stats tous les N épisodes (défaut: 100)");
    println!("  --max-steps <NUM>     Nombre max de steps par épisode (défaut: 500)");
    println!("  -h, --help            Afficher cette aide");
    println!();
    println!("EXEMPLES:");
    println!("  cargo run --bin=train_burn_rl");
    println!("  cargo run --bin=train_burn_rl -- --episodes 2000 --save-every 200");
    println!("  cargo run --bin=train_burn_rl -- --max-steps 1000 --episodes 500");
    println!();
    println!("NOTES:");
    println!("  - Utilise l'environnement Burn-RL avec l'espace d'actions compactes");
    println!("  - Pour l'instant, implémente seulement une politique epsilon-greedy simple");
    println!("  - L'intégration avec un vrai agent DQN peut être ajoutée plus tard");
}

