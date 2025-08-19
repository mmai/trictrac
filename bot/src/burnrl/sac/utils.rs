use crate::burnrl::environment::{TrictracAction, TrictracEnvironment};
use crate::burnrl::sac::sac_model;
use crate::training_common::get_valid_action_indices;
use burn::backend::{ndarray::NdArrayDevice, NdArray};
use burn::module::{Module, Param, ParamId};
use burn::nn::Linear;
use burn::record::{CompactRecorder, Recorder};
use burn::tensor::backend::Backend;
use burn::tensor::cast::ToElement;
use burn::tensor::Tensor;
// use burn_rl::agent::{SACModel, SAC};
use burn_rl::base::{Agent, ElemType, Environment};

// pub fn save_model(model: &sac_model::Net<NdArray<ElemType>>, path: &String) {
//     let recorder = CompactRecorder::new();
//     let model_path = format!("{path}.mpk");
//     println!("Modèle de validation sauvegardé : {model_path}");
//     recorder
//         .record(model.clone().into_record(), model_path.into())
//         .unwrap();
// }
//
// pub fn load_model(dense_size: usize, path: &String) -> Option<sac_model::Net<NdArray<ElemType>>> {
//     let model_path = format!("{path}.mpk");
//     // println!("Chargement du modèle depuis : {model_path}");
//
//     CompactRecorder::new()
//         .load(model_path.into(), &NdArrayDevice::default())
//         .map(|record| {
//             dqn_model::Net::new(
//                 <TrictracEnvironment as Environment>::StateType::size(),
//                 dense_size,
//                 <TrictracEnvironment as Environment>::ActionType::size(),
//             )
//             .load_record(record)
//         })
//         .ok()
// }
//

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
