use crate::burnrl::environment::TrictracEnvironment;
use crate::burnrl::utils::Config;
use burn::backend::{ndarray::NdArrayDevice, NdArray};
use burn::module::Module;
use burn::nn::{Initializer, Linear, LinearConfig};
use burn::optim::AdamWConfig;
use burn::record::{CompactRecorder, Recorder};
use burn::tensor::activation::{relu, softmax};
use burn::tensor::backend::{AutodiffBackend, Backend};
use burn::tensor::Tensor;
use burn_rl::agent::{PPOModel, PPOOutput, PPOTrainingConfig, PPO};
use burn_rl::base::{Action, Agent, ElemType, Environment, Memory, Model, State};
use std::env;
use std::fs;
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

type MyAgent<E, B> = PPO<E, B, Net<B>>;

#[allow(unused)]
pub fn run<
    E: Environment + AsMut<TrictracEnvironment>,
    B: AutodiffBackend<InnerBackend = NdArray>,
>(
    conf: &Config,
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

    if let Some(path) = &conf.save_path {
        let device = NdArrayDevice::default();
        let recorder = CompactRecorder::new();
        let tmp_path = env::temp_dir().join("tmp_model.mpk");

        // Save the trained model (backend B) to a temporary file
        recorder
            .record(model.clone().into_record(), tmp_path.clone())
            .expect("Failed to save temporary model");

        // Create a new model instance with the target backend (NdArray)
        let model_to_save: Net<NdArray<ElemType>> = Net::new(
            <<E as Environment>::StateType as State>::size(),
            conf.dense_size,
            <<E as Environment>::ActionType as Action>::size(),
        );

        // Load the record from the temporary file into the new model
        let record = recorder
            .load(tmp_path.clone(), &device)
            .expect("Failed to load temporary model");
        let model_with_loaded_weights = model_to_save.load_record(record);

        // Clean up the temporary file
        fs::remove_file(tmp_path).expect("Failed to remove temporary model file");

        save_model(&model_with_loaded_weights, path);
    }
    let valid_agent = agent.valid(model);
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

