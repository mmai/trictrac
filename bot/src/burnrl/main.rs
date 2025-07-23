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

    let valid_agent = agent.valid();

    println!("> Sauvegarde du modèle de validation");
    save_model(valid_agent.model().as_ref().unwrap());

    println!("> Test");
    demo_model::<Env>(valid_agent);
}

fn save_model(model: &dqn_model::Net<NdArray<ElemType>>) {
    let path = "models/burn_dqn".to_string();
    let recorder = CompactRecorder::new();
    let model_path = format!("{}_model.burn", path);
    println!("Modèle de validation sauvegardé : {}", model_path);
    recorder
        .record(model.clone().into_record(), model_path.into())
        .unwrap();
}
