use bot::burnrl::environment;
use bot::burnrl::sac::{sac_model, utils::demo_model};
use burn::backend::{Autodiff, NdArray};
use burn_rl::agent::SAC;
use burn_rl::base::ElemType;

type Backend = Autodiff<NdArray<ElemType>>;
type Env = environment::TrictracEnvironment;

fn main() {
    // println!("> Entraînement");

    // See also MEMORY_SIZE in dqn_model.rs : 8192
    let conf = sac_model::SacConfig {
        //                   defaults
        num_episodes: 50, // 40
        max_steps: 1000,  // 1000 max steps by episode
        dense_size: 256,  // 128  neural network complexity (default 128)

        gamma: 0.999, // 0.999 discount factor. Plus élevé = encourage stratégies à long terme
        tau: 0.005, // 0.005 soft update rate. Taux de mise à jour du réseau cible. Plus bas = adaptation
        // plus lente moins sensible aux coups de chance
        learning_rate: 0.001, // 0.001 taille du pas. Bas : plus lent, haut : risque de ne jamais
        // converger
        batch_size: 32, // 32 nombre d'expériences passées sur lesquelles pour calcul de l'erreur moy.
        clip_grad: 1.0, // 1.0 limite max de correction à apporter au gradient
        min_probability: 1e-9,
    };
    println!("{conf}----------");
    let valid_agent = sac_model::run::<Env, Backend>(&conf, false); //true);

    // let valid_agent = agent.valid();

    // println!("> Sauvegarde du modèle de validation");
    //
    // let path = "bot/models/burnrl_dqn".to_string();
    // save_model(valid_agent.model().as_ref().unwrap(), &path);
    //
    // println!("> Chargement du modèle pour test");
    // let loaded_model = load_model(conf.dense_size, &path);
    // let loaded_agent = DQN::new(loaded_model.unwrap());
    //
    // println!("> Test avec le modèle chargé");
    // demo_model(loaded_agent);
}
