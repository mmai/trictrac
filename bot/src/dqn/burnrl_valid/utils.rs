use crate::dqn::burnrl_valid::{
    dqn_model,
    environment::{TrictracAction, TrictracEnvironment},
};
use crate::dqn::dqn_common::get_valid_action_indices;
use burn::backend::{ndarray::NdArrayDevice, Autodiff, NdArray};
use burn::module::{Module, Param, ParamId};
use burn::nn::Linear;
use burn::record::{CompactRecorder, Recorder};
use burn::tensor::backend::Backend;
use burn::tensor::cast::ToElement;
use burn::tensor::Tensor;
use burn_rl::agent::{DQNModel, DQN};
use burn_rl::base::{Action, ElemType, Environment, State};

pub fn save_model(model: &dqn_model::Net<NdArray<ElemType>>, path: &String) {
    let recorder = CompactRecorder::new();
    let model_path = format!("{path}_model.mpk");
    println!("Modèle de validation sauvegardé : {model_path}");
    recorder
        .record(model.clone().into_record(), model_path.into())
        .unwrap();
}

pub fn load_model(dense_size: usize, path: &String) -> Option<dqn_model::Net<NdArray<ElemType>>> {
    let model_path = format!("{path}_model.mpk");
    // println!("Chargement du modèle depuis : {model_path}");

    CompactRecorder::new()
        .load(model_path.into(), &NdArrayDevice::default())
        .map(|record| {
            dqn_model::Net::new(
                <TrictracEnvironment as Environment>::StateType::size(),
                dense_size,
                <TrictracEnvironment as Environment>::ActionType::size(),
            )
            .load_record(record)
        })
        .ok()
}

pub fn demo_model<B: Backend, M: DQNModel<B>>(agent: DQN<TrictracEnvironment, B, M>) {
    let mut env = TrictracEnvironment::new(true);
    let mut done = false;
    while !done {
        // let action = match infer_action(&agent, &env, state) {
        let action = match infer_action(&agent, &env) {
            Some(value) => value,
            None => break,
        };
        // Execute action
        let snapshot = env.step(action);
        done = snapshot.done();
    }
}

fn infer_action<B: Backend, M: DQNModel<B>>(
    agent: &DQN<TrictracEnvironment, B, M>,
    env: &TrictracEnvironment,
) -> Option<TrictracAction> {
    let state = env.state();
    // Get q-values
    let q_values = agent
        .model()
        .as_ref()
        .unwrap()
        .infer(state.to_tensor().unsqueeze());
    // Get valid actions
    let valid_actions_indices = get_valid_action_indices(&env.game);
    if valid_actions_indices.is_empty() {
        return None; // No valid actions, end of episode
    }
    // Set non valid actions q-values to lowest
    let mut masked_q_values = q_values.clone();
    let q_values_vec: Vec<f32> = q_values.into_data().into_vec().unwrap();
    for (index, q_value) in q_values_vec.iter().enumerate() {
        if !valid_actions_indices.contains(&index) {
            masked_q_values = masked_q_values.clone().mask_fill(
                masked_q_values.clone().equal_elem(*q_value),
                f32::NEG_INFINITY,
            );
        }
    }
    // Get best action (highest q-value)
    let action_index = masked_q_values.argmax(1).into_scalar().to_u32();
    let action = TrictracAction::from(action_index);
    Some(action)
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
