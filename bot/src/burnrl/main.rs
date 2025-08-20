use bot::burnrl::sac_model as burn_model;
// use bot::burnrl::dqn_big_model as burn_model;
// use bot::burnrl::dqn_model as burn_model;
// use bot::burnrl::environment_big::TrictracEnvironment;
use bot::burnrl::environment::TrictracEnvironment;
use bot::burnrl::utils::{demo_model, Config};
use burn::backend::{Autodiff, NdArray};
use burn_rl::agent::SAC as MyAgent;
// use burn_rl::agent::DQN as MyAgent;
use burn_rl::base::ElemType;

type Backend = Autodiff<NdArray<ElemType>>;
type Env = TrictracEnvironment;

fn main() {
    let path = "bot/models/burnrl_dqn".to_string();
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

        min_probability: 1e-9,

        eps_start: 0.9, // 0.9  epsilon initial value (0.9 => more exploration)
        eps_end: 0.05,  // 0.05
        // eps_decay higher = epsilon decrease slower
        // used in : epsilon = eps_end + (eps_start - eps_end) * e^(-step / eps_decay);
        // epsilon is updated at the start of each episode
        eps_decay: 2000.0, // 1000 ?

        lambda: 0.95,
        epsilon_clip: 0.2,
        critic_weight: 0.5,
        entropy_weight: 0.01,
        epochs: 8,
    };
    println!("{conf}----------");

    let agent = burn_model::run::<Env, Backend>(&conf, false); //true);

    // println!("> Chargement du modèle pour test");
    // let loaded_model = burn_model::load_model(conf.dense_size, &path);
    // let loaded_agent: MyAgent<Env, _, _> = MyAgent::new(loaded_model.unwrap());
    //
    // println!("> Test avec le modèle chargé");
    // demo_model(loaded_agent);

    // demo_model::<Env>(agent);
}
