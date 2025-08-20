use crate::burnrl::environment_big::TrictracEnvironment;
use crate::burnrl::utils::{soft_update_linear, Config};
use burn::backend::{ndarray::NdArrayDevice, NdArray};
use burn::module::Module;
use burn::nn::{Linear, LinearConfig};
use burn::optim::AdamWConfig;
use burn::record::{CompactRecorder, Recorder};
use burn::tensor::activation::relu;
use burn::tensor::backend::{AutodiffBackend, Backend};
use burn::tensor::Tensor;
use burn_rl::agent::DQN;
use burn_rl::agent::{DQNModel, DQNTrainingConfig};
use burn_rl::base::{Action, Agent, ElemType, Environment, Memory, Model, State};
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

type MyAgent<E, B> = DQN<E, B, Net<B>>;

#[allow(unused)]
// pub fn run<E: Environment + AsMut<TrictracEnvironment>, B: AutodiffBackend>(
pub fn run<
    E: Environment + AsMut<TrictracEnvironment>,
    B: AutodiffBackend<InnerBackend = NdArray>,
>(
    conf: &Config,
    visualized: bool,
    // ) -> DQN<E, B, Net<B>> {
) -> impl Agent<E> {
    let mut env = E::new(visualized);
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
                let goodmoves_ratio = ((envmut.goodmoves_count as f32 / episode_duration as f32)
                    * 100.0)
                    .round() as u32;
                println!(
                    "{{\"episode\": {episode}, \"reward\": {episode_reward:.4}, \"steps count\": {episode_duration}, \"epsilon\": {eps_threshold:.3}, \"goodmoves\": {}, \"ratio\": {}%, \"rollpoints\":{}, \"duration\": {}}}",
                    envmut.goodmoves_count,
                    goodmoves_ratio,
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
    let valid_agent = agent.valid();
    if let Some(path) = &conf.save_path {
        save_model(valid_agent.model().as_ref().unwrap(), path);
    }
    valid_agent
}

pub fn save_model(model: &Net<NdArray<ElemType>>, path: &String) {
    let recorder = CompactRecorder::new();
    let model_path = format!("{path}.mpk");
    println!("info: Modèle de validation sauvegardé : {model_path}");
    recorder
        .record(model.clone().into_record(), model_path.into())
        .unwrap();
}

pub fn load_model(dense_size: usize, path: &String) -> Option<Net<NdArray<ElemType>>> {
    let model_path = format!("{path}.mpk");
    // println!("Chargement du modèle depuis : {model_path}");

    CompactRecorder::new()
        .load(model_path.into(), &NdArrayDevice::default())
        .map(|record| {
            Net::new(
                <TrictracEnvironment as Environment>::StateType::size(),
                dense_size,
                <TrictracEnvironment as Environment>::ActionType::size(),
            )
            .load_record(record)
        })
        .ok()
}
