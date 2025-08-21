use bot::burnrl::algos::{
    dqn, dqn_big, dqn_valid, ppo, ppo_big, ppo_valid, sac, sac_big, sac_valid,
};
use bot::burnrl::environment::TrictracEnvironment;
use bot::burnrl::environment_big::TrictracEnvironment as TrictracEnvironmentBig;
use bot::burnrl::environment_valid::TrictracEnvironment as TrictracEnvironmentValid;
use bot::burnrl::utils::{demo_model, Config};
use burn::backend::{Autodiff, NdArray};
use burn_rl::base::ElemType;
use std::env;

type Backend = Autodiff<NdArray<ElemType>>;

fn main() {
    let args: Vec<String> = env::args().collect();
    let algo = &args[1];
    // let dir_path = &args[2];

    let path = format!("bot/models/burnrl_{algo}");
    let conf = Config {
        save_path: Some(path.clone()),
        num_episodes: 30, // 40
        max_steps: 1000,  // 1000 max steps by episode
        dense_size: 256,  // 128  neural network complexity (default 128)

        gamma: 0.9999, // 0.999 discount factor. Plus élevé = encourage stratégies à long terme
        tau: 0.0005, // 0.005 soft update rate. Taux de mise à jour du réseau cible. Plus bas = adaptation
        // plus lente moins sensible aux coups de chance
        learning_rate: 0.001, // 0.001 taille du pas. Bas : plus lent, haut : risque de ne jamais
        // converger
        batch_size: 128, // 32 nombre d'expériences passées sur lesquelles pour calcul de l'erreur moy.
        clip_grad: 70.0, // 100 limite max de correction à apporter au gradient (default 100)

        // SAC
        min_probability: 1e-9,

        // DQN
        eps_start: 0.9, // 0.9  epsilon initial value (0.9 => more exploration)
        eps_end: 0.05,  // 0.05
        // eps_decay higher = epsilon decrease slower
        // used in : epsilon = eps_end + (eps_start - eps_end) * e^(-step / eps_decay);
        // epsilon is updated at the start of each episode
        eps_decay: 2000.0, // 1000 ?

        // PPO
        lambda: 0.95,
        epsilon_clip: 0.2,
        critic_weight: 0.5,
        entropy_weight: 0.01,
        epochs: 8,
    };
    println!("{conf}----------");

    match algo.as_str() {
        "dqn" => {
            let _agent = dqn::run::<TrictracEnvironment, Backend>(&conf, false);
            println!("> Chargement du modèle pour test");
            let loaded_model = dqn::load_model(conf.dense_size, &path);
            let loaded_agent: burn_rl::agent::DQN<TrictracEnvironment, _, _> =
                burn_rl::agent::DQN::new(loaded_model.unwrap());

            println!("> Test avec le modèle chargé");
            demo_model(loaded_agent);
        }
        "dqn_big" => {
            let _agent = dqn_big::run::<TrictracEnvironmentBig, Backend>(&conf, false);
            println!("> Chargement du modèle pour test");
            let loaded_model = dqn_big::load_model(conf.dense_size, &path);
            let loaded_agent: burn_rl::agent::DQN<TrictracEnvironmentBig, _, _> =
                burn_rl::agent::DQN::new(loaded_model.unwrap());

            println!("> Test avec le modèle chargé");
            demo_model(loaded_agent);
        }
        "dqn_valid" => {
            let _agent = dqn_valid::run::<TrictracEnvironmentValid, Backend>(&conf, false);
            println!("> Chargement du modèle pour test");
            let loaded_model = dqn_valid::load_model(conf.dense_size, &path);
            let loaded_agent: burn_rl::agent::DQN<TrictracEnvironmentValid, _, _> =
                burn_rl::agent::DQN::new(loaded_model.unwrap());

            println!("> Test avec le modèle chargé");
            demo_model(loaded_agent);
        }
        "sac" => {
            let _agent = sac::run::<TrictracEnvironment, Backend>(&conf, false);
            println!("> Chargement du modèle pour test");
            let loaded_model = sac::load_model(conf.dense_size, &path);
            let loaded_agent: burn_rl::agent::SAC<TrictracEnvironment, _, _> =
                burn_rl::agent::SAC::new(loaded_model.unwrap());

            println!("> Test avec le modèle chargé");
            demo_model(loaded_agent);
        }
        "sac_big" => {
            let _agent = sac_big::run::<TrictracEnvironmentBig, Backend>(&conf, false);
            println!("> Chargement du modèle pour test");
            let loaded_model = sac_big::load_model(conf.dense_size, &path);
            let loaded_agent: burn_rl::agent::SAC<TrictracEnvironmentBig, _, _> =
                burn_rl::agent::SAC::new(loaded_model.unwrap());

            println!("> Test avec le modèle chargé");
            demo_model(loaded_agent);
        }
        "sac_valid" => {
            let _agent = sac_valid::run::<TrictracEnvironmentValid, Backend>(&conf, false);
            println!("> Chargement du modèle pour test");
            let loaded_model = sac_valid::load_model(conf.dense_size, &path);
            let loaded_agent: burn_rl::agent::SAC<TrictracEnvironmentValid, _, _> =
                burn_rl::agent::SAC::new(loaded_model.unwrap());

            println!("> Test avec le modèle chargé");
            demo_model(loaded_agent);
        }
        "ppo" => {
            let _agent = ppo::run::<TrictracEnvironment, Backend>(&conf, false);
            println!("> Chargement du modèle pour test");
            let loaded_model = ppo::load_model(conf.dense_size, &path);
            let loaded_agent: burn_rl::agent::PPO<TrictracEnvironment, _, _> =
                burn_rl::agent::PPO::new(loaded_model.unwrap());

            println!("> Test avec le modèle chargé");
            demo_model(loaded_agent);
        }
        "ppo_big" => {
            let _agent = ppo_big::run::<TrictracEnvironmentBig, Backend>(&conf, false);
            println!("> Chargement du modèle pour test");
            let loaded_model = ppo_big::load_model(conf.dense_size, &path);
            let loaded_agent: burn_rl::agent::PPO<TrictracEnvironmentBig, _, _> =
                burn_rl::agent::PPO::new(loaded_model.unwrap());

            println!("> Test avec le modèle chargé");
            demo_model(loaded_agent);
        }
        "ppo_valid" => {
            let _agent = ppo_valid::run::<TrictracEnvironmentValid, Backend>(&conf, false);
            println!("> Chargement du modèle pour test");
            let loaded_model = ppo_valid::load_model(conf.dense_size, &path);
            let loaded_agent: burn_rl::agent::PPO<TrictracEnvironmentValid, _, _> =
                burn_rl::agent::PPO::new(loaded_model.unwrap());

            println!("> Test avec le modèle chargé");
            demo_model(loaded_agent);
        }
        &_ => {
            println!("unknown algo {algo}");
        }
    }
}
