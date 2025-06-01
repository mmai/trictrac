use bot::strategy::dqn_common::{DqnConfig, TrictracAction};
use bot::strategy::dqn_trainer::DqnTrainer;
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let args: Vec<String> = env::args().collect();

    // Paramètres par défaut
    let mut episodes = 1000;
    let mut model_path = "models/dqn_model".to_string();
    let mut save_every = 100;

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

    println!("Configuration d'entraînement DQN :");
    println!("  Épisodes : {}", episodes);
    println!("  Chemin du modèle : {}", model_path);
    println!("  Sauvegarde tous les {} épisodes", save_every);
    println!();

    // Configuration DQN
    let config = DqnConfig {
        state_size: 36, // state.to_vec size
        hidden_size: 256,
        num_actions: TrictracAction::action_space_size(),
        learning_rate: 0.001,
        gamma: 0.99,
        epsilon: 0.9, // Commencer avec plus d'exploration
        epsilon_decay: 0.995,
        epsilon_min: 0.01,
        replay_buffer_size: 10000,
        batch_size: 32,
    };

    // Créer et lancer l'entraîneur
    let mut trainer = DqnTrainer::new(config);
    trainer.train(episodes, save_every, &model_path)?;

    println!("Entraînement terminé avec succès !");
    println!("Pour utiliser le modèle entraîné :");
    println!(
        "  cargo run --bin=client_cli -- --bot dqn:{}_final.json,dummy",
        model_path
    );

    Ok(())
}

fn print_help() {
    println!("Entraîneur DQN pour Trictrac");
    println!();
    println!("USAGE:");
    println!("  cargo run --bin=train_dqn [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("  --episodes <NUM>      Nombre d'épisodes d'entraînement (défaut: 1000)");
    println!("  --model-path <PATH>   Chemin de base pour sauvegarder les modèles (défaut: models/dqn_model)");
    println!("  --save-every <NUM>    Sauvegarder le modèle tous les N épisodes (défaut: 100)");
    println!("  -h, --help            Afficher cette aide");
    println!();
    println!("EXEMPLES:");
    println!("  cargo run --bin=train_dqn");
    println!("  cargo run --bin=train_dqn -- --episodes 5000 --save-every 500");
    println!("  cargo run --bin=train_dqn -- --model-path models/my_model --episodes 2000");
}
