use bot::burnrl::{dqn_model, environment, utils::demo_model};
use burn::backend::{Autodiff, NdArray};
use burn::module::Module;
use burn::record::{CompactRecorder, Recorder};
use burn_rl::agent::DQN;
use burn_rl::base::ElemType;

type Backend = Autodiff<NdArray<ElemType>>;
type Env = environment::TrictracEnvironment;

fn main() {
    println!("> Entraînement");
    let num_episodes = 3;
    let agent = dqn_model::run::<Env, Backend>(num_episodes, false); //true);
    println!("> Sauvegarde");
    save(&agent);

    // cette ligne sert à extraire le "cerveau" de l'agent entraîné,
    // sans les données nécessaires à l'entraînement
    let valid_agent = agent.valid();

    println!("> Test");
    demo_model::<Env>(valid_agent);
}

fn save(agent: &DQN<Env, Backend, dqn_model::Net<Backend>>) {
    let path = "models/burn_dqn".to_string();
    let inference_network = agent.model().clone().into_record();
    let recorder = CompactRecorder::new();
    let model_path = format!("{}_model.burn", path);
    println!("Modèle sauvegardé : {}", model_path);
    recorder
        .record(inference_network, model_path.into())
        .unwrap();
}
