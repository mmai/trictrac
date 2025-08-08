use bot::dqn::burnrl::{
    dqn_model, environment,
    utils::{demo_model, load_model, save_model},
};
use burn::backend::{Autodiff, NdArray};
use burn_rl::agent::DQN;
use burn_rl::base::ElemType;

type Backend = Autodiff<NdArray<ElemType>>;
type Env = environment::TrictracEnvironment;

fn main() {
    // println!("> Entraînement");
    let conf = dqn_model::DqnConfig {
        num_episodes: 40,
        // memory_size: 8192, // must be set in  dqn_model.rs with the MEMORY_SIZE constant
        // max_steps: 700, // must be set in  environment.rs with the MAX_STEPS constant
        dense_size: 256, // neural network complexity
        eps_start: 0.9,  // epsilon initial value (0.9 => more exploration)
        eps_end: 0.05,
        eps_decay: 3000.0,
    };
    let agent = dqn_model::run::<Env, Backend>(&conf, false); //true);

    let valid_agent = agent.valid();

    println!("> Sauvegarde du modèle de validation");

    let path = "models/burn_dqn_40".to_string();
    save_model(valid_agent.model().as_ref().unwrap(), &path);

    println!("> Chargement du modèle pour test");
    let loaded_model = load_model(conf.dense_size, &path);
    let loaded_agent = DQN::new(loaded_model);

    println!("> Test avec le modèle chargé");
    demo_model(loaded_agent);
}
