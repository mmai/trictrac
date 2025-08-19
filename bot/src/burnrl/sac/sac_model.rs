use crate::burnrl::environment::TrictracEnvironment;
use crate::burnrl::sac::utils::soft_update_linear;
use burn::module::Module;
use burn::nn::{Linear, LinearConfig};
use burn::optim::AdamWConfig;
use burn::tensor::activation::{relu, softmax};
use burn::tensor::backend::{AutodiffBackend, Backend};
use burn::tensor::Tensor;
use burn_rl::agent::{SACActor, SACCritic, SACNets, SACOptimizer, SACTrainingConfig, SAC};
use burn_rl::base::{Action, Agent, ElemType, Environment, Memory, Model, State};
use std::fmt;
use std::time::SystemTime;

#[derive(Module, Debug)]
pub struct Actor<B: Backend> {
    linear_0: Linear<B>,
    linear_1: Linear<B>,
    linear_2: Linear<B>,
}

impl<B: Backend> Actor<B> {
    pub fn new(input_size: usize, dense_size: usize, output_size: usize) -> Self {
        Self {
            linear_0: LinearConfig::new(input_size, dense_size).init(&Default::default()),
            linear_1: LinearConfig::new(dense_size, dense_size).init(&Default::default()),
            linear_2: LinearConfig::new(dense_size, output_size).init(&Default::default()),
        }
    }
}

impl<B: Backend> Model<B, Tensor<B, 2>, Tensor<B, 2>> for Actor<B> {
    fn forward(&self, input: Tensor<B, 2>) -> Tensor<B, 2> {
        let layer_0_output = relu(self.linear_0.forward(input));
        let layer_1_output = relu(self.linear_1.forward(layer_0_output));

        softmax(self.linear_2.forward(layer_1_output), 1)
    }

    fn infer(&self, input: Tensor<B, 2>) -> Tensor<B, 2> {
        self.forward(input)
    }
}

impl<B: Backend> SACActor<B> for Actor<B> {}

#[derive(Module, Debug)]
pub struct Critic<B: Backend> {
    linear_0: Linear<B>,
    linear_1: Linear<B>,
    linear_2: Linear<B>,
}

impl<B: Backend> Critic<B> {
    pub fn new(input_size: usize, dense_size: usize, output_size: usize) -> Self {
        Self {
            linear_0: LinearConfig::new(input_size, dense_size).init(&Default::default()),
            linear_1: LinearConfig::new(dense_size, dense_size).init(&Default::default()),
            linear_2: LinearConfig::new(dense_size, output_size).init(&Default::default()),
        }
    }

    fn consume(self) -> (Linear<B>, Linear<B>, Linear<B>) {
        (self.linear_0, self.linear_1, self.linear_2)
    }
}

impl<B: Backend> Model<B, Tensor<B, 2>, Tensor<B, 2>> for Critic<B> {
    fn forward(&self, input: Tensor<B, 2>) -> Tensor<B, 2> {
        let layer_0_output = relu(self.linear_0.forward(input));
        let layer_1_output = relu(self.linear_1.forward(layer_0_output));

        self.linear_2.forward(layer_1_output)
    }

    fn infer(&self, input: Tensor<B, 2>) -> Tensor<B, 2> {
        self.forward(input)
    }
}

impl<B: Backend> SACCritic<B> for Critic<B> {
    fn soft_update(this: Self, that: &Self, tau: ElemType) -> Self {
        let (linear_0, linear_1, linear_2) = this.consume();

        Self {
            linear_0: soft_update_linear(linear_0, &that.linear_0, tau),
            linear_1: soft_update_linear(linear_1, &that.linear_1, tau),
            linear_2: soft_update_linear(linear_2, &that.linear_2, tau),
        }
    }
}

#[allow(unused)]
const MEMORY_SIZE: usize = 4096;

pub struct SacConfig {
    pub max_steps: usize,
    pub num_episodes: usize,
    pub dense_size: usize,

    pub gamma: f32,
    pub tau: f32,
    pub learning_rate: f32,
    pub batch_size: usize,
    pub clip_grad: f32,
    pub min_probability: f32,
}

impl Default for SacConfig {
    fn default() -> Self {
        Self {
            max_steps: 2000,
            num_episodes: 1000,
            dense_size: 32,

            gamma: 0.999,
            tau: 0.005,
            learning_rate: 0.001,
            batch_size: 32,
            clip_grad: 1.0,
            min_probability: 1e-9,
        }
    }
}

impl fmt::Display for SacConfig {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut s = String::new();
        s.push_str(&format!("max_steps={:?}\n", self.max_steps));
        s.push_str(&format!("num_episodes={:?}\n", self.num_episodes));
        s.push_str(&format!("dense_size={:?}\n", self.dense_size));
        s.push_str(&format!("gamma={:?}\n", self.gamma));
        s.push_str(&format!("tau={:?}\n", self.tau));
        s.push_str(&format!("learning_rate={:?}\n", self.learning_rate));
        s.push_str(&format!("batch_size={:?}\n", self.batch_size));
        s.push_str(&format!("clip_grad={:?}\n", self.clip_grad));
        s.push_str(&format!("min_probability={:?}\n", self.min_probability));
        write!(f, "{s}")
    }
}

type MyAgent<E, B> = SAC<E, B, Actor<B>>;

#[allow(unused)]
pub fn run<E: Environment + AsMut<TrictracEnvironment>, B: AutodiffBackend>(
    conf: &SacConfig,
    visualized: bool,
) -> impl Agent<E> {
    let mut env = E::new(visualized);
    env.as_mut().max_steps = conf.max_steps;
    let state_dim = <<E as Environment>::StateType as State>::size();
    let action_dim = <<E as Environment>::ActionType as Action>::size();

    let mut actor = Actor::<B>::new(state_dim, conf.dense_size, action_dim);
    let mut critic_1 = Critic::<B>::new(state_dim, conf.dense_size, action_dim);
    let mut critic_2 = Critic::<B>::new(state_dim, conf.dense_size, action_dim);
    let mut nets = SACNets::<B, Actor<B>, Critic<B>>::new(actor, critic_1, critic_2);

    let mut agent = MyAgent::default();

    let config = SACTrainingConfig {
        gamma: conf.gamma,
        tau: conf.tau,
        learning_rate: conf.learning_rate,
        min_probability: conf.min_probability,
        batch_size: conf.batch_size,
        clip_grad: Some(burn::grad_clipping::GradientClippingConfig::Value(
            conf.clip_grad,
        )),
    };

    let mut memory = Memory::<E, B, MEMORY_SIZE>::default();

    let optimizer_config = AdamWConfig::new().with_grad_clipping(config.clip_grad.clone());

    let mut optimizer = SACOptimizer::new(
        optimizer_config.clone().init(),
        optimizer_config.clone().init(),
        optimizer_config.clone().init(),
        optimizer_config.init(),
    );

    let mut policy_net = agent.model().clone();

    let mut step = 0_usize;

    for episode in 0..conf.num_episodes {
        let mut episode_done = false;
        let mut episode_reward = 0.0;
        let mut episode_duration = 0_usize;
        let mut state = env.state();
        let mut now = SystemTime::now();

        while !episode_done {
            if let Some(action) = MyAgent::<E, _>::react_with_model(&state, &nets.actor) {
                let snapshot = env.step(action);

                episode_reward += <<E as Environment>::RewardType as Into<ElemType>>::into(
                    snapshot.reward().clone(),
                );

                memory.push(
                    state,
                    *snapshot.state(),
                    action,
                    snapshot.reward().clone(),
                    snapshot.done(),
                );

                if config.batch_size < memory.len() {
                    nets = agent.train::<MEMORY_SIZE, _>(nets, &memory, &mut optimizer, &config);
                }

                step += 1;
                episode_duration += 1;

                if snapshot.done() || episode_duration >= conf.max_steps {
                    env.reset();
                    episode_done = true;

                    println!(
                        "{{\"episode\": {episode}, \"reward\": {episode_reward:.4}, \"steps count\": {episode_duration}, \"duration\": {}}}",
                    now.elapsed().unwrap().as_secs()
                    );
                    now = SystemTime::now();
                } else {
                    state = *snapshot.state();
                }
            }
        }
    }

    agent.valid(nets.actor)
}
