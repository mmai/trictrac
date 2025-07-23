use bot::burnrl::{dqn_model, environment, utils::demo_model};
use burn::backend::{Autodiff, NdArray};
use burn_rl::base::ElemType;

type Backend = Autodiff<NdArray<ElemType>>;
type Env = environment::TrictracEnvironment;

fn main() {
    let num_episodes = 3;
    let agent = dqn_model::run::<Env, Backend>(num_episodes, false); //true);

    demo_model::<Env>(agent);
}
