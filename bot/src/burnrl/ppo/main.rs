use bot::burnrl::environment;
use bot::burnrl::ppo::{
    ppo_model,
    utils::{demo_model, load_model, save_model},
};
use burn::backend::{Autodiff, NdArray};
use burn_rl::agent::PPO;
use burn_rl::base::ElemType;

type Backend = Autodiff<NdArray<ElemType>>;
type Env = environment::TrictracEnvironment;

fn main() {
    // println!("> Entraînement");

    // See also MEMORY_SIZE in ppo_model.rs : 8192
    let conf = ppo_model::PpoConfig {
        //                   defaults
        num_episodes: 50, // 40
        max_steps: 1000,  // 1000 max steps by episode
        dense_size: 128,  // 128  neural network complexity (default 128)
        gamma: 0.999,     // 0.999 discount factor. Plus élevé = encourage stratégies à long terme
        // plus lente moins sensible aux coups de chance
        learning_rate: 0.001, // 0.001 taille du pas. Bas : plus lent, haut : risque de ne jamais
        // converger
        batch_size: 128, // 32 nombre d'expériences passées sur lesquelles pour calcul de l'erreur moy.
        clip_grad: 100.0, // 100 limite max de correction à apporter au gradient (default 100)

        lambda: 0.95,
        epsilon_clip: 0.2,
        critic_weight: 0.5,
        entropy_weight: 0.01,
        epochs: 8,
    };
    println!("{conf}----------");
    let valid_agent = ppo_model::run::<Env, Backend>(&conf, false); //true);

    // let valid_agent = agent.valid(model);

    println!("> Sauvegarde du modèle de validation");

    let path = "bot/models/burnrl_ppo".to_string();
    panic!("how to do that  : save model");
    // save_model(valid_agent.model().as_ref().unwrap(), &path);

    // println!("> Chargement du modèle pour test");
    // let loaded_model = load_model(conf.dense_size, &path);
    // let loaded_agent = PPO::new(loaded_model.unwrap());
    //
    // println!("> Test avec le modèle chargé");
    // demo_model(loaded_agent);
}
