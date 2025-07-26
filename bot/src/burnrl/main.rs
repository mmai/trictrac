use bot::burnrl::{dqn_model, environment, utils::demo_model};
use burn::backend::{ndarray::NdArrayDevice, Autodiff, NdArray};
use burn::module::Module;
use burn::record::{CompactRecorder, Recorder};
use burn_rl::agent::DQN;
use burn_rl::base::{Action, Agent, ElemType, Environment, State};

type Backend = Autodiff<NdArray<ElemType>>;
type Env = environment::TrictracEnvironment;

fn main() {
    println!("> Entraînement");
    let num_episodes = 50;
    let agent = dqn_model::run::<Env, Backend>(num_episodes, false); //true);

    let valid_agent = agent.valid();

    println!("> Sauvegarde du modèle de validation");

    let path = "models/burn_dqn_50".to_string();
    save_model(valid_agent.model().as_ref().unwrap(), &path);

    // println!("> Test avec le modèle entraîné");
    // demo_model::<Env>(valid_agent);

    println!("> Chargement du modèle pour test");
    let loaded_model = load_model(&path);
    let loaded_agent = DQN::new(loaded_model);

    println!("> Test avec le modèle chargé");
    demo_model(loaded_agent);
}

fn save_model(model: &dqn_model::Net<NdArray<ElemType>>, path: &String) {
    let recorder = CompactRecorder::new();
    let model_path = format!("{}_model.mpk", path);
    println!("Modèle de validation sauvegardé : {}", model_path);
    recorder
        .record(model.clone().into_record(), model_path.into())
        .unwrap();
}

fn load_model(path: &String) -> dqn_model::Net<NdArray<ElemType>> {
    // TODO : reprendre le DENSE_SIZE de dqn_model.rs
    const DENSE_SIZE: usize = 128;

    let model_path = format!("{}_model.mpk", path);
    println!("Chargement du modèle depuis : {}", model_path);

    let device = NdArrayDevice::default();
    let recorder = CompactRecorder::new();

    let record = recorder
        .load(model_path.into(), &device)
        .expect("Impossible de charger le modèle");

    dqn_model::Net::new(
        <environment::TrictracEnvironment as Environment>::StateType::size(),
        DENSE_SIZE,
        <environment::TrictracEnvironment as Environment>::ActionType::size(),
    )
    .load_record(record)
}
