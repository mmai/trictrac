use burn::module::{Module, Param, ParamId};
use burn::nn::Linear;
use burn::tensor::backend::Backend;
use burn::tensor::cast::ToElement;
use burn::tensor::Tensor;
use burn_rl::agent::DQN;
use burn_rl::base::{Action, ElemType, Environment, State};

pub fn demo_model<E, M, B, F>(agent: DQN<E, B, M>, mut get_valid_actions: F)
where
    E: Environment,
    M: Module<B> + burn_rl::agent::DQNModel<B>,
    B: Backend,
    F: FnMut(&E) -> Vec<E::ActionType>,
    <E as Environment>::ActionType: PartialEq,
{
    let mut env = E::new(true);
    let mut state = env.state();
    let mut done = false;
    let mut total_reward = 0.0;
    let mut steps = 0;

    while !done {
        let model = agent.model().as_ref().unwrap();
        let state_tensor = E::StateType::to_tensor(&state).unsqueeze();
        let q_values = model.infer(state_tensor);

        let valid_actions = get_valid_actions(&env);
        if valid_actions.is_empty() {
            break; // No valid actions, end of episode
        }

        let mut masked_q_values = q_values.clone();
        let q_values_vec: Vec<f32> = q_values.into_data().into_vec().unwrap();

        for (index, q_value) in q_values_vec.iter().enumerate() {
            if !valid_actions.contains(&E::ActionType::from(index as u32)) {
                masked_q_values = masked_q_values.clone().mask_fill(
                    masked_q_values.clone().equal_elem(*q_value),
                    f32::NEG_INFINITY,
                );
            }
        }

        let action_index = masked_q_values.argmax(1).into_scalar().to_u32();
        let action = E::ActionType::from(action_index);

        let snapshot = env.step(action);
        state = *snapshot.state();
        total_reward +=
            <<E as Environment>::RewardType as Into<ElemType>>::into(snapshot.reward().clone());
        steps += 1;
        done = snapshot.done() || steps >= E::MAX_STEPS;
    }
    println!(
        "Episode terminé. Récompense totale: {:.2}, Étapes: {}",
        total_reward, steps
    );
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
