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
    println!(
        "info: loading configuration from file {:?}",
        confy::get_configuration_file_path("trictrac_bot", None).unwrap()
    );
    let mut conf: Config = confy::load("trictrac_bot", None).expect("Could not load config");
    conf.save_path = Some(path.clone());
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
