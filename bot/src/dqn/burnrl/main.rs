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

    // See also MEMORY_SIZE in dqn_model.rs : 8192
    let conf = dqn_model::DqnConfig {
        num_episodes: 40, // default : 40
        min_steps: 250.0, // min of max steps by episode (mise à jour par la fonction)(default 1000 ?)
        max_steps: 3000,  // max steps by episode (default 1000 ?)
        dense_size: 256,  // neural network complexity (default 128)
        eps_start: 0.9,   // epsilon initial value (0.9 => more exploration) (default 0.9)
        eps_end: 0.05,    // (default 0.05)
        // eps_decay higher = epsilon decrease slower
        // used in : epsilon = eps_end + (eps_start - eps_end) * e^(-step / eps_decay);
        // epsilon is updated at the start of each episode
        eps_decay: 5000.0, // default 1000 ?

        gamma: 0.999, // discount factor. Plus élevé = encourage stratégies à long terme
        tau: 0.005, // soft update rate. Taux de mise à jour du réseau cible. Plus bas = adaptation
        // plus lente moins sensible aux coups de chance
        learning_rate: 0.001, // taille du pas. Bas : plus lent, haut : risque de ne jamais
        // converger
        batch_size: 32, // nombre d'expériences passées sur lesquelles pour calcul de l'erreur moy.
        clip_grad: 50.0, // limite max de correction à apporter au gradient (default 100)
    };
    println!("{conf}----------");
    let agent = dqn_model::run::<Env, Backend>(&conf, false); //true);

    let valid_agent = agent.valid();

    println!("> Sauvegarde du modèle de validation");

    let path = "models/burn_dqn_40".to_string();
    save_model(valid_agent.model().as_ref().unwrap(), &path);

    println!("> Chargement du modèle pour test");
    let loaded_model = load_model(conf.dense_size, &path);
    let loaded_agent = DQN::new(loaded_model.unwrap());

    println!("> Test avec le modèle chargé");
    demo_model(loaded_agent);
}
