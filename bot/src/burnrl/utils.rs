use burn::module::{Param, ParamId};
use burn::nn::Linear;
use burn::tensor::backend::Backend;
use burn::tensor::Tensor;
use burn_rl::base::{Agent, ElemType, Environment};

pub fn demo_model<E: Environment>(agent: impl Agent<E>) {
    let mut env = E::new(true);
    let mut state = env.state();
    let mut done = false;
    while !done {
        // // Get q values for current state
        // let model = agent.model().as_ref().unwrap();
        // let state_tensor = E::StateType::to_tensor(&state).unsqueeze();
        // let q_values = model.infer(state_tensor);
        //
        // // Get valid actions
        // let valid_actions = get_valid_actions(&state);
        // if valid_actions.is_empty() {
        //     break; // No valid actions, end of episode
        // }
        //
        // // Set q values of non valid actions to the lowest
        // let mut masked_q_values = q_values.clone();
        // let q_values_vec: Vec<f32> = q_values.into_data().into_vec().unwrap();
        // for (index, q_value) in q_values_vec.iter().enumerate() {
        //     if !valid_actions.contains(&E::ActionType::from(index as u32)) {
        //         masked_q_values = masked_q_values.clone().mask_fill(
        //             masked_q_values.clone().equal_elem(*q_value),
        //             f32::NEG_INFINITY,
        //         );
        //     }
        // }
        //
        // // Get action with the highest q-value
        // let action_index = masked_q_values.argmax(1).into_scalar().to_u32();
        // let action = E::ActionType::from(action_index);
        //
        // // Execute action
        // let snapshot = env.step(action);
        // state = *snapshot.state();
        // // println!("{:?}", state);
        // done = snapshot.done();

        if let Some(action) = agent.react(&state) {
            // println!("before : {:?}", state);
            // println!("action : {:?}", action);
            let snapshot = env.step(action);
            state = *snapshot.state();
            // println!("after : {:?}", state);
            // done = true;
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
