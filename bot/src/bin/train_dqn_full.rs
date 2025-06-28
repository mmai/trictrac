use bot::strategy::burn_dqn_agent::{BurnDqnAgent, DqnConfig, Experience};
use bot::strategy::burn_environment::{TrictracAction, TrictracEnvironment};
use bot::strategy::dqn_common::get_valid_actions;
use burn_rl::base::Environment;
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let args: Vec<String> = env::args().collect();

    // Paramètres par défaut
    let mut episodes = 1000;
    let mut model_path = "models/burn_dqn_model".to_string();
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
            "--model-path" => {
                if i + 1 < args.len() {
                    model_path = args[i + 1].clone();
                    i += 2;
                } else {
                    eprintln!("Erreur : --model-path nécessite une valeur");
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

    // Créer le dossier models s'il n'existe pas
    std::fs::create_dir_all("models")?;

    println!("=== Entraînement DQN complet avec Burn ===");
    println!("Épisodes : {}", episodes);
    println!("Modèle : {}", model_path);
    println!("Sauvegarde tous les {} épisodes", save_every);
    println!("Max steps par épisode : {}", max_steps_per_episode);
    println!();

    // Configuration DQN
    let config = DqnConfig {
        state_size: 36,
        action_size: 1252, // Espace d'actions réduit via contexte
        hidden_size: 256,
        learning_rate: 0.001,
        gamma: 0.99,
        epsilon: 1.0,
        epsilon_decay: 0.995,
        epsilon_min: 0.01,
        replay_buffer_size: 10000,
        batch_size: 32,
        target_update_freq: 100,
    };

    // Créer l'agent et l'environnement
    let mut agent = BurnDqnAgent::new(config);
    let mut optimizer = AdamConfig::new().init();

    let mut env = TrictracEnvironment::new(true);

    // Variables pour les statistiques
    let mut total_rewards = Vec::new();
    let mut episode_lengths = Vec::new();
    let mut losses = Vec::new();

    println!("Début de l'entraînement avec agent DQN complet...");
    println!();

    for episode in 1..=episodes {
        // Reset de l'environnement
        let mut snapshot = env.reset();
        let mut episode_reward = 0.0;
        let mut step = 0;
        let mut episode_loss = 0.0;
        let mut loss_count = 0;

        loop {
            step += 1;
            let current_state = snapshot.state();

            // Obtenir les actions valides selon le contexte du jeu
            let valid_actions = get_valid_actions(&env.game);

            if valid_actions.is_empty() {
                break;
            }

            // Convertir les actions Trictrac en indices pour l'agent
            let valid_indices: Vec<usize> = (0..valid_actions.len()).collect();

            // Sélectionner une action avec l'agent DQN
            let action_index = agent.select_action(
                &current_state
                    .data
                    .iter()
                    .map(|&x| x as f32)
                    .collect::<Vec<_>>(),
                &valid_indices,
            );
            let action = TrictracAction {
                index: action_index as u32,
            };

            // Exécuter l'action
            snapshot = env.step(action);
            episode_reward += snapshot.reward();

            // Préparer l'expérience pour l'agent
            let experience = Experience {
                state: current_state.data.iter().map(|&x| x as f32).collect(),
                action: action_index,
                reward: snapshot.reward(),
                next_state: if snapshot.terminated {
                    None
                } else {
                    Some(snapshot.state().data.iter().map(|&x| x as f32).collect())
                },
                done: snapshot.terminated,
            };

            // Ajouter l'expérience au replay buffer
            agent.add_experience(experience);

            // Entraîner l'agent
            if let Some(loss) = agent.train_step(optimizer) {
                episode_loss += loss;
                loss_count += 1;
            }

            // Vérifier les conditions de fin
            if snapshot.terminated || step >= max_steps_per_episode {
                break;
            }
        }

        // Calculer la loss moyenne de l'épisode
        let avg_loss = if loss_count > 0 {
            episode_loss / loss_count as f32
        } else {
            0.0
        };

        // Sauvegarder les statistiques
        total_rewards.push(episode_reward);
        episode_lengths.push(step);
        losses.push(avg_loss);

        // Affichage des statistiques
        if episode % save_every == 0 {
            let avg_reward =
                total_rewards.iter().rev().take(save_every).sum::<f32>() / save_every as f32;
            let avg_length =
                episode_lengths.iter().rev().take(save_every).sum::<usize>() / save_every;
            let avg_episode_loss =
                losses.iter().rev().take(save_every).sum::<f32>() / save_every as f32;

            println!("Episode {} | Avg Reward: {:.3} | Avg Length: {} | Avg Loss: {:.6} | Epsilon: {:.3} | Buffer: {}", 
                     episode, avg_reward, avg_length, avg_episode_loss, agent.get_epsilon(), agent.get_buffer_size());

            // Sauvegarder le modèle
            let checkpoint_path = format!("{}_{}", model_path, episode);
            if let Err(e) = agent.save_model(&checkpoint_path) {
                eprintln!("Erreur lors de la sauvegarde : {}", e);
            } else {
                println!("  → Modèle sauvegardé : {}", checkpoint_path);
            }
        } else if episode % 10 == 0 {
            println!(
                "Episode {} | Reward: {:.3} | Length: {} | Loss: {:.6} | Epsilon: {:.3}",
                episode,
                episode_reward,
                step,
                avg_loss,
                agent.get_epsilon()
            );
        }
    }

    // Sauvegarder le modèle final
    let final_path = format!("{}_final", model_path);
    agent.save_model(&final_path)?;

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
    let final_avg_loss =
        losses.iter().rev().take(100.min(episodes)).sum::<f32>() / 100.min(episodes) as f32;

    println!(
        "Récompense moyenne (100 derniers épisodes) : {:.3}",
        final_avg_reward
    );
    println!(
        "Longueur moyenne (100 derniers épisodes) : {}",
        final_avg_length
    );
    println!(
        "Loss moyenne (100 derniers épisodes) : {:.6}",
        final_avg_loss
    );
    println!("Epsilon final : {:.3}", agent.get_epsilon());
    println!("Taille du buffer final : {}", agent.get_buffer_size());

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
    println!("Modèle final sauvegardé : {}", final_path);
    println!();
    println!("Pour utiliser le modèle entraîné :");
    println!(
        "  cargo run --bin=client_cli -- --bot burn_dqn:{}_final,dummy",
        model_path
    );

    Ok(())
}

fn print_help() {
    println!("Entraîneur DQN complet avec Burn pour Trictrac");
    println!();
    println!("USAGE:");
    println!("  cargo run --bin=train_dqn_full [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("  --episodes <NUM>       Nombre d'épisodes d'entraînement (défaut: 1000)");
    println!("  --model-path <PATH>    Chemin de base pour sauvegarder les modèles (défaut: models/burn_dqn_model)");
    println!("  --save-every <NUM>     Sauvegarder le modèle tous les N épisodes (défaut: 100)");
    println!("  --max-steps <NUM>      Nombre max de steps par épisode (défaut: 500)");
    println!("  -h, --help             Afficher cette aide");
    println!();
    println!("EXEMPLES:");
    println!("  cargo run --bin=train_dqn_full");
    println!("  cargo run --bin=train_dqn_full -- --episodes 2000 --save-every 200");
    println!("  cargo run --bin=train_dqn_full -- --model-path models/my_model --episodes 500");
    println!();
    println!("FONCTIONNALITÉS:");
    println!("  - Agent DQN complet avec réseau de neurones Burn");
    println!("  - Experience replay buffer avec échantillonnage aléatoire");
    println!("  - Epsilon-greedy avec décroissance automatique");
    println!("  - Target network avec mise à jour périodique");
    println!("  - Sauvegarde automatique des modèles");
    println!("  - Statistiques d'entraînement détaillées");
}
