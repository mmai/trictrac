//! [`BotStrategy`] implementations backed by `spiel_bot` models.
//!
//! | Strategy struct | Network | CLI token |
//! |-----------------|---------|-----------|
//! | [`AzBotStrategy`] (mlp) | MlpNet (AlphaZero) | `az` / `az:PATH` |
//! | [`AzBotStrategy`] (resnet) | ResNet (AlphaZero) | `az-resnet` / `az-resnet:PATH` |
//! | [`DqnSpielBotStrategy`] | QNet (DQN) | `az-dqn` / `az-dqn:PATH` |
//!
//! All strategies operate from **White's perspective** (player_id = 1) internally;
//! the [`Bot`](trictrac_bot::Bot) wrapper handles board mirroring for Black.

use std::cell::RefCell;
use std::path::Path;

use burn::{
    backend::NdArray,
    tensor::{Tensor, TensorData},
};
use rand::{SeedableRng, rngs::SmallRng};
use trictrac_bot::BotStrategy;
use trictrac_store::{
    training_common::{get_valid_action_indices, TrictracAction},
    CheckerMove, Color, GameEvent, GameState, MoveRules, PlayerId,
};

use crate::{
    alphazero::BurnEvaluator,
    env::{GameEnv, TrictracEnv},
    mcts::{self, Evaluator, MctsConfig},
    network::{MlpConfig, MlpNet, QNet, QNetConfig, QValueNet, ResNet, ResNetConfig},
};

type B = NdArray<f32>;

/// Default MCTS simulations per move used by [`AzBotStrategy`].
pub const AZ_BOT_N_SIM: usize = 50;

// ── Shared helpers ─────────────────────────────────────────────────────────────

/// Decode an action index → `(CheckerMove, CheckerMove)` using the game state.
fn action_to_moves(action: usize, game: &GameState) -> Option<(CheckerMove, CheckerMove)> {
    match TrictracAction::from_action_index(action)?.to_event(game)? {
        GameEvent::Move { moves, .. } => Some(moves),
        _ => None,
    }
}

/// Fallback: return the first legal move from `MoveRules` (always succeeds).
fn fallback_move(game: &GameState) -> (CheckerMove, CheckerMove) {
    let rules = MoveRules::new(&Color::White, &game.board, game.dice);
    let moves = rules.get_possible_moves_sequences(true, vec![]);
    *moves.first().unwrap_or(&(CheckerMove::default(), CheckerMove::default()))
}

// ── AzBotStrategy ─────────────────────────────────────────────────────────────

/// AlphaZero bot usable as a [`BotStrategy`].
///
/// Supports both MlpNet and ResNet checkpoints through separate constructors.
/// Uses greedy (temperature = 0) MCTS for action selection.
///
/// # Construction
///
/// ```rust,ignore
/// // MlpNet with random weights
/// AzBotStrategy::new_mlp(None);
///
/// // MlpNet from a checkpoint
/// AzBotStrategy::new_mlp(Some("checkpoints/iter_0100.mpk"));
///
/// // ResNet from a checkpoint
/// AzBotStrategy::new_resnet(Some("checkpoints/resnet_0200.mpk"));
/// ```
pub struct AzBotStrategy {
    game: GameState,
    evaluator: Box<dyn Evaluator>,
    mcts_config: MctsConfig,
    /// Interior-mutable RNG so `choose_move(&self)` can drive MCTS.
    rng: RefCell<SmallRng>,
}

impl std::fmt::Debug for AzBotStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AzBotStrategy")
            .field("n_sim", &self.mcts_config.n_simulations)
            .finish()
    }
}

impl AzBotStrategy {
    fn from_evaluator(evaluator: Box<dyn Evaluator>) -> Self {
        Self {
            game: GameState::default(),
            evaluator,
            mcts_config: MctsConfig {
                n_simulations: AZ_BOT_N_SIM,
                dirichlet_alpha: 0.0, // no noise during play
                dirichlet_eps: 0.0,
                temperature: 0.0, // greedy selection
                ..MctsConfig::default()
            },
            rng: RefCell::new(SmallRng::seed_from_u64(42)),
        }
    }

    /// MlpNet-backed bot.  `path = None` → random weights.
    pub fn new_mlp(path: Option<&str>) -> Self {
        let device: <B as burn::tensor::backend::Backend>::Device = Default::default();
        let cfg = MlpConfig { obs_size: 217, action_size: 514, hidden_size: 256 };
        let model = match path {
            Some(p) => MlpNet::<B>::load(&cfg, Path::new(p), &device).unwrap_or_else(|e| {
                eprintln!("az: load failed ({e}), using random weights");
                MlpNet::<B>::new(&cfg, &device)
            }),
            None => MlpNet::<B>::new(&cfg, &device),
        };
        Self::from_evaluator(Box::new(BurnEvaluator::<B, MlpNet<B>>::new(model, device)))
    }

    /// ResNet-backed bot.  `path = None` → random weights.
    pub fn new_resnet(path: Option<&str>) -> Self {
        let device: <B as burn::tensor::backend::Backend>::Device = Default::default();
        let cfg = ResNetConfig { obs_size: 217, action_size: 514, hidden_size: 512 };
        let model = match path {
            Some(p) => ResNet::<B>::load(&cfg, Path::new(p), &device).unwrap_or_else(|e| {
                eprintln!("az-resnet: load failed ({e}), using random weights");
                ResNet::<B>::new(&cfg, &device)
            }),
            None => ResNet::<B>::new(&cfg, &device),
        };
        Self::from_evaluator(Box::new(BurnEvaluator::<B, ResNet<B>>::new(model, device)))
    }

    /// Run MCTS and return the greedy best action index, or `None` if no legal moves.
    fn best_action(&self) -> Option<usize> {
        let env = TrictracEnv;
        if env.legal_actions(&self.game).is_empty() {
            return None;
        }
        let mut rng = self.rng.borrow_mut();
        let root = mcts::run_mcts(
            &env,
            &self.game,
            self.evaluator.as_ref(),
            &self.mcts_config,
            &mut *rng,
        );
        Some(mcts::select_action(&root, 0.0, &mut *rng))
    }
}

impl BotStrategy for AzBotStrategy {
    fn get_game(&self) -> &GameState { &self.game }
    fn get_mut_game(&mut self) -> &mut GameState { &mut self.game }
    fn calculate_points(&self) -> u8 { self.game.dice_points.0 }
    fn calculate_adv_points(&self) -> u8 { self.game.dice_points.1 }
    fn set_player_id(&mut self, _player_id: PlayerId) {}
    fn set_color(&mut self, _color: Color) {}

    fn choose_go(&self) -> bool {
        // Action index 1 == TrictracAction::Go
        self.best_action().map(|a| a == 1).unwrap_or(false)
    }

    fn choose_move(&self) -> (CheckerMove, CheckerMove) {
        self.best_action()
            .and_then(|a| action_to_moves(a, &self.game))
            .unwrap_or_else(|| fallback_move(&self.game))
    }
}

// ── DqnSpielBotStrategy ───────────────────────────────────────────────────────

/// DQN bot (QNet from `spiel_bot`) usable as a [`BotStrategy`].
///
/// Selects actions by greedy argmax over Q-values, masked to legal moves.
/// When no checkpoint is provided the model falls back to the first legal move.
///
/// # Construction
///
/// ```rust,ignore
/// // No model — always picks first legal move
/// DqnSpielBotStrategy::new(None);
///
/// // Trained checkpoint
/// DqnSpielBotStrategy::new(Some("checkpoints/dqn_iter_0500.mpk"));
/// ```
#[derive(Debug)]
pub struct DqnSpielBotStrategy {
    game: GameState,
    model: Option<QNet<B>>,
}

impl DqnSpielBotStrategy {
    /// Create a DQN bot.  `path = None` → falls back to first legal move.
    pub fn new(path: Option<&str>) -> Self {
        let model = path.map(|p| {
            let device: <B as burn::tensor::backend::Backend>::Device = Default::default();
            let cfg = QNetConfig::default();
            QNet::<B>::load(&cfg, Path::new(p), &device).unwrap_or_else(|e| {
                eprintln!("az-dqn: load failed ({e}), using random weights");
                QNet::<B>::new(&cfg, &device)
            })
        });
        Self { game: GameState::default(), model }
    }

    /// Greedy Q-value selection masked to legal actions, or `None` if no model / no legal moves.
    fn best_action(&self) -> Option<usize> {
        let model = self.model.as_ref()?;
        let legal = get_valid_action_indices(&self.game).unwrap_or_default();
        if legal.is_empty() {
            return None;
        }
        let device: <B as burn::tensor::backend::Backend>::Device = Default::default();
        let obs = self.game.to_tensor();
        let obs_t = Tensor::<B, 2>::from_data(TensorData::new(obs, [1, 217]), &device);
        let q_vals: Vec<f32> = model.forward(obs_t).into_data().to_vec().unwrap();
        legal.into_iter().max_by(|&a, &b| {
            q_vals[a].partial_cmp(&q_vals[b]).unwrap_or(std::cmp::Ordering::Equal)
        })
    }
}

impl BotStrategy for DqnSpielBotStrategy {
    fn get_game(&self) -> &GameState { &self.game }
    fn get_mut_game(&mut self) -> &mut GameState { &mut self.game }
    fn calculate_points(&self) -> u8 { self.game.dice_points.0 }
    fn calculate_adv_points(&self) -> u8 { self.game.dice_points.1 }
    fn set_player_id(&mut self, _player_id: PlayerId) {}
    fn set_color(&mut self, _color: Color) {}

    fn choose_go(&self) -> bool {
        self.best_action().map(|a| a == 1).unwrap_or(false)
    }

    fn choose_move(&self) -> (CheckerMove, CheckerMove) {
        self.best_action()
            .and_then(|a| action_to_moves(a, &self.game))
            .unwrap_or_else(|| fallback_move(&self.game))
    }
}
