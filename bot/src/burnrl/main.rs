use burn::backend::{Autodiff, NdArray};
use burn_rl::base::ElemType;
use bot::burnrl::{
    dqn_model,
    environment,
    utils::demo_model,
};

type Backend = Autodiff<NdArray<ElemType>>;
type Env = environment::TrictracEnvironment;

fn main() {
    let agent = dqn_model::run::<Env, Backend>(512, false); //true);

    demo_model::<Env>(agent);
}
