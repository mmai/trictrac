use crate::dqn::burnrl_big::environment::TrictracEnvironment;
use crate::dqn::burnrl_big::utils::soft_update_linear;
use burn::module::Module;
use burn::nn::{Linear, LinearConfig};
use burn::optim::AdamWConfig;
use burn::tensor::activation::relu;
use burn::tensor::backend::{AutodiffBackend, Backend};
use burn::tensor::Tensor;
use burn_rl::agent::DQN;
use burn_rl::agent::{DQNModel, DQNTrainingConfig};
use burn_rl::base::{Action, ElemType, Environment, Memory, Model, State};
use std::fmt;
use std::time::SystemTime;

#[derive(Module, Debug)]
pub struct Net<B: Backend> {
    linear_0: Linear<B>,
    linear_1: Linear<B>,
    linear_2: Linear<B>,
}

impl<B: Backend> Net<B> {
    #[allow(unused)]
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

impl<B: Backend> Model<B, Tensor<B, 2>, Tensor<B, 2>> for Net<B> {
    fn forward(&self, input: Tensor<B, 2>) -> Tensor<B, 2> {
        let layer_0_output = relu(self.linear_0.forward(input));
        let layer_1_output = relu(self.linear_1.forward(layer_0_output));

        relu(self.linear_2.forward(layer_1_output))
    }

    fn infer(&self, input: Tensor<B, 2>) -> Tensor<B, 2> {
        self.forward(input)
    }
}

impl<B: Backend> DQNModel<B> for Net<B> {
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
const MEMORY_SIZE: usize = 8192;

pub struct DqnConfig {
    pub min_steps: f32,
    pub max_steps: usize,
    pub num_episodes: usize,
    pub dense_size: usize,
    pub eps_start: f64,
    pub eps_end: f64,
    pub eps_decay: f64,

    pub gamma: f32,
    pub tau: f32,
    pub learning_rate: f32,
    pub batch_size: usize,
    pub clip_grad: f32,
}

impl fmt::Display for DqnConfig {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut s = String::new();
        s.push_str(&format!("min_steps={:?}\n", self.min_steps));
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
        write!(f, "{s}")
    }
}

impl Default for DqnConfig {
    fn default() -> Self {
        Self {
            min_steps: 250.0,
            max_steps: 2000,
            num_episodes: 1000,
            dense_size: 256,
            eps_start: 0.9,
            eps_end: 0.05,
            eps_decay: 1000.0,

            gamma: 0.999,
            tau: 0.005,
            learning_rate: 0.001,
            batch_size: 32,
            clip_grad: 100.0,
        }
    }
}

type MyAgent<E, B> = DQN<E, B, Net<B>>;

#[allow(unused)]
pub fn run<E: Environment + AsMut<TrictracEnvironment>, B: AutodiffBackend>(
    conf: &DqnConfig,
    visualized: bool,
) -> DQN<E, B, Net<B>> {
    // ) -> impl Agent<E> {
    let mut env = E::new(visualized);
    env.as_mut().min_steps = conf.min_steps;
    env.as_mut().max_steps = conf.max_steps;

    let model = Net::<B>::new(
        <<E as Environment>::StateType as State>::size(),
        conf.dense_size,
        <<E as Environment>::ActionType as Action>::size(),
    );

    let mut agent = MyAgent::new(model);

    // let config = DQNTrainingConfig::default();
    let config = DQNTrainingConfig {
        gamma: conf.gamma,
        tau: conf.tau,
        learning_rate: conf.learning_rate,
        batch_size: conf.batch_size,
        clip_grad: Some(burn::grad_clipping::GradientClippingConfig::Value(
            conf.clip_grad,
        )),
    };

    let mut memory = Memory::<E, B, MEMORY_SIZE>::default();

    let mut optimizer = AdamWConfig::new()
        .with_grad_clipping(config.clip_grad.clone())
        .init();

    let mut policy_net = agent.model().as_ref().unwrap().clone();

    let mut step = 0_usize;

    for episode in 0..conf.num_episodes {
        let mut episode_done = false;
        let mut episode_reward: ElemType = 0.0;
        let mut episode_duration = 0_usize;
        let mut state = env.state();
        let mut now = SystemTime::now();

        while !episode_done {
            let eps_threshold = conf.eps_end
                + (conf.eps_start - conf.eps_end) * f64::exp(-(step as f64) / conf.eps_decay);
            let action =
                DQN::<E, B, Net<B>>::react_with_exploration(&policy_net, state, eps_threshold);
            let snapshot = env.step(action);

            episode_reward +=
                <<E as Environment>::RewardType as Into<ElemType>>::into(snapshot.reward().clone());

            memory.push(
                state,
                *snapshot.state(),
                action,
                snapshot.reward().clone(),
                snapshot.done(),
            );

            if config.batch_size < memory.len() {
                policy_net =
                    agent.train::<MEMORY_SIZE>(policy_net, &memory, &mut optimizer, &config);
            }

            step += 1;
            episode_duration += 1;

            if snapshot.done() || episode_duration >= conf.max_steps {
                let envmut = env.as_mut();
                println!(
                    "{{\"episode\": {episode}, \"reward\": {episode_reward:.4}, \"steps count\": {episode_duration}, \"epsilon\": {eps_threshold:.3}, \"goodmoves\": {}, \"rollpoints\":{}, \"duration\": {}}}",
                    envmut.goodmoves_count,
                    envmut.pointrolls_count,
                    now.elapsed().unwrap().as_secs(),
                );
                env.reset();
                episode_done = true;
                now = SystemTime::now();
            } else {
                state = *snapshot.state();
            }
        }
    }
    agent
}
