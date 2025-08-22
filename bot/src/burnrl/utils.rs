use burn::module::{Param, ParamId};
use burn::nn::Linear;
use burn::tensor::backend::Backend;
use burn::tensor::Tensor;
use burn_rl::base::{Agent, ElemType, Environment};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub save_path: Option<String>,
    pub max_steps: usize, // max steps by episode
    pub num_episodes: usize,
    pub dense_size: usize, // neural network complexity

    // discount factor. Plus élevé = encourage stratégies à long terme
    pub gamma: f32,
    // soft update rate. Taux de mise à jour du réseau cible. Plus bas = adaptation plus lente moins sensible aux coups de chance
    pub tau: f32,
    // taille du pas. Bas : plus lent, haut : risque de ne jamais
    pub learning_rate: f32,
    // nombre d'expériences passées sur lesquelles pour calcul de l'erreur moy.
    pub batch_size: usize,
    // limite max de correction à apporter au gradient (default 100)
    pub clip_grad: f32,

    // ---- for SAC
    pub min_probability: f32,

    // ---- for DQN
    // epsilon initial value (0.9 => more exploration)
    pub eps_start: f64,
    pub eps_end: f64,
    // eps_decay higher = epsilon decrease slower
    // used in : epsilon = eps_end + (eps_start - eps_end) * e^(-step / eps_decay);
    // epsilon is updated at the start of each episode
    pub eps_decay: f64,

    // ---- for PPO
    pub lambda: f32,
    pub epsilon_clip: f32,
    pub critic_weight: f32,
    pub entropy_weight: f32,
    pub epochs: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            save_path: None,
            max_steps: 2000,
            num_episodes: 1000,
            dense_size: 256,
            gamma: 0.999,
            tau: 0.005,
            learning_rate: 0.001,
            batch_size: 32,
            clip_grad: 100.0,
            min_probability: 1e-9,
            eps_start: 0.9,
            eps_end: 0.05,
            eps_decay: 1000.0,
            lambda: 0.95,
            epsilon_clip: 0.2,
            critic_weight: 0.5,
            entropy_weight: 0.01,
            epochs: 8,
        }
    }
}

impl std::fmt::Display for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut s = String::new();
        s.push_str(&format!("max_steps={:?}\n", self.max_steps));
        s.push_str(&format!("num_episodes={:?}\n", self.num_episodes));
        s.push_str(&format!("dense_size={:?}\n", self.dense_size));
        s.push_str(&format!("eps_start={:?}\n", self.eps_start));
        s.push_str(&format!("eps_end={:?}\n", self.eps_end));
        s.push_str(&format!("eps_decay={:?}\n", self.eps_decay));
        s.push_str(&format!("gamma={:?}\n", self.gamma));
        s.push_str(&format!("tau={:?}\n", self.tau));
        s.push_str(&format!("learning_rate={:?}\n", self.learning_rate));
        s.push_str(&format!("batch_size={:?}\n", self.batch_size));
        s.push_str(&format!("clip_grad={:?}\n", self.clip_grad));
        s.push_str(&format!("min_probability={:?}\n", self.min_probability));
        s.push_str(&format!("lambda={:?}\n", self.lambda));
        s.push_str(&format!("epsilon_clip={:?}\n", self.epsilon_clip));
        s.push_str(&format!("critic_weight={:?}\n", self.critic_weight));
        s.push_str(&format!("entropy_weight={:?}\n", self.entropy_weight));
        s.push_str(&format!("epochs={:?}\n", self.epochs));
        write!(f, "{s}")
    }
}

pub fn demo_model<E: Environment>(agent: impl Agent<E>) {
    let mut env = E::new(true);
    let mut state = env.state();
    let mut done = false;
    while !done {
        if let Some(action) = agent.react(&state) {
            let snapshot = env.step(action);
            state = *snapshot.state();
            done = snapshot.done();
        }
    }
}

fn soft_update_tensor<const N: usize, B: Backend>(
    this: &Param<Tensor<B, N>>,
    that: &Param<Tensor<B, N>>,
    tau: ElemType,
) -> Param<Tensor<B, N>> {
    let that_weight = that.val();
    let this_weight = this.val();
    let new_weight = this_weight * (1.0 - tau) + that_weight * tau;

    Param::initialized(ParamId::new(), new_weight)
}

pub fn soft_update_linear<B: Backend>(
    this: Linear<B>,
    that: &Linear<B>,
    tau: ElemType,
) -> Linear<B> {
    let weight = soft_update_tensor(&this.weight, &that.weight, tau);
    let bias = match (&this.bias, &that.bias) {
        (Some(this_bias), Some(that_bias)) => Some(soft_update_tensor(this_bias, that_bias, tau)),
        _ => None,
    };

    Linear::<B> { weight, bias }
}
