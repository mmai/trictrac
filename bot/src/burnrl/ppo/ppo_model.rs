use crate::burnrl::environment::TrictracEnvironment;
use burn::module::Module;
use burn::nn::{Initializer, Linear, LinearConfig};
use burn::optim::AdamWConfig;
use burn::tensor::activation::{relu, softmax};
use burn::tensor::backend::{AutodiffBackend, Backend};
use burn::tensor::Tensor;
use burn_rl::agent::{PPOModel, PPOOutput, PPOTrainingConfig, PPO};
use burn_rl::base::{Action, Agent, ElemType, Environment, Memory, Model, State};
use std::fmt;
use std::time::SystemTime;

#[derive(Module, Debug)]
pub struct Net<B: Backend> {
    linear: Linear<B>,
    linear_actor: Linear<B>,
    linear_critic: Linear<B>,
}

impl<B: Backend> Net<B> {
    #[allow(unused)]
    pub fn new(input_size: usize, dense_size: usize, output_size: usize) -> Self {
        let initializer = Initializer::XavierUniform { gain: 1.0 };
        Self {
            linear: LinearConfig::new(input_size, dense_size)
                .with_initializer(initializer.clone())
                .init(&Default::default()),
            linear_actor: LinearConfig::new(dense_size, output_size)
                .with_initializer(initializer.clone())
                .init(&Default::default()),
            linear_critic: LinearConfig::new(dense_size, 1)
                .with_initializer(initializer)
                .init(&Default::default()),
        }
    }
}

impl<B: Backend> Model<B, Tensor<B, 2>, PPOOutput<B>, Tensor<B, 2>> for Net<B> {
    fn forward(&self, input: Tensor<B, 2>) -> PPOOutput<B> {
        let layer_0_output = relu(self.linear.forward(input));
        let policies = softmax(self.linear_actor.forward(layer_0_output.clone()), 1);
        let values = self.linear_critic.forward(layer_0_output);

        PPOOutput::<B>::new(policies, values)
    }

    fn infer(&self, input: Tensor<B, 2>) -> Tensor<B, 2> {
        let layer_0_output = relu(self.linear.forward(input));
        softmax(self.linear_actor.forward(layer_0_output.clone()), 1)
    }
}

impl<B: Backend> PPOModel<B> for Net<B> {}
#[allow(unused)]
const MEMORY_SIZE: usize = 512;

pub struct PpoConfig {
    pub max_steps: usize,
    pub num_episodes: usize,
    pub dense_size: usize,

    pub gamma: f32,
    pub lambda: f32,
    pub epsilon_clip: f32,
    pub critic_weight: f32,
    pub entropy_weight: f32,
    pub learning_rate: f32,
    pub epochs: usize,
    pub batch_size: usize,
    pub clip_grad: f32,
}

impl fmt::Display for PpoConfig {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut s = String::new();
        s.push_str(&format!("max_steps={:?}\n", self.max_steps));
        s.push_str(&format!("num_episodes={:?}\n", self.num_episodes));
        s.push_str(&format!("dense_size={:?}\n", self.dense_size));
        s.push_str(&format!("gamma={:?}\n", self.gamma));
        s.push_str(&format!("lambda={:?}\n", self.lambda));
        s.push_str(&format!("epsilon_clip={:?}\n", self.epsilon_clip));
        s.push_str(&format!("critic_weight={:?}\n", self.critic_weight));
        s.push_str(&format!("entropy_weight={:?}\n", self.entropy_weight));
        s.push_str(&format!("learning_rate={:?}\n", self.learning_rate));
        s.push_str(&format!("epochs={:?}\n", self.epochs));
        s.push_str(&format!("batch_size={:?}\n", self.batch_size));
        write!(f, "{s}")
    }
}

impl Default for PpoConfig {
    fn default() -> Self {
        Self {
            max_steps: 2000,
            num_episodes: 1000,
            dense_size: 256,

            gamma: 0.99,
            lambda: 0.95,
            epsilon_clip: 0.2,
            critic_weight: 0.5,
            entropy_weight: 0.01,
            learning_rate: 0.001,
            epochs: 8,
            batch_size: 8,
            clip_grad: 100.0,
        }
    }
}
type MyAgent<E, B> = PPO<E, B, Net<B>>;

#[allow(unused)]
pub fn run<E: Environment + AsMut<TrictracEnvironment>, B: AutodiffBackend>(
    conf: &PpoConfig,
    visualized: bool,
    // ) -> PPO<E, B, Net<B>> {
) -> impl Agent<E> {
    let mut env = E::new(visualized);
    env.as_mut().max_steps = conf.max_steps;

    let mut model = Net::<B>::new(
        <<E as Environment>::StateType as State>::size(),
        conf.dense_size,
        <<E as Environment>::ActionType as Action>::size(),
    );
    let agent = MyAgent::default();
    let config = PPOTrainingConfig {
        gamma: conf.gamma,
        lambda: conf.lambda,
        epsilon_clip: conf.epsilon_clip,
        critic_weight: conf.critic_weight,
        entropy_weight: conf.entropy_weight,
        learning_rate: conf.learning_rate,
        epochs: conf.epochs,
        batch_size: conf.batch_size,
        clip_grad: Some(burn::grad_clipping::GradientClippingConfig::Value(
            conf.clip_grad,
        )),
    };

    let mut optimizer = AdamWConfig::new()
        .with_grad_clipping(config.clip_grad.clone())
        .init();
    let mut memory = Memory::<E, B, MEMORY_SIZE>::default();
    for episode in 0..conf.num_episodes {
        let mut episode_done = false;
        let mut episode_reward = 0.0;
        let mut episode_duration = 0_usize;
        let mut now = SystemTime::now();

        env.reset();
        while !episode_done {
            let state = env.state();
            if let Some(action) = MyAgent::<E, _>::react_with_model(&state, &model) {
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

                episode_duration += 1;
                episode_done = snapshot.done() || episode_duration >= conf.max_steps;
            }
        }
        println!(
            "{{\"episode\": {episode}, \"reward\": {episode_reward:.4}, \"steps count\": {episode_duration}, \"duration\": {}}}",
                    now.elapsed().unwrap().as_secs(),
        );

        now = SystemTime::now();
        model = MyAgent::train::<MEMORY_SIZE>(model, &memory, &mut optimizer, &config);
        memory.clear();
    }

    agent.valid(model)
    // agent
}
