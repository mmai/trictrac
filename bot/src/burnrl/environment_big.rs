use crate::training_common_big;
use burn::{prelude::Backend, tensor::Tensor};
use burn_rl::base::{Action, Environment, Snapshot, State};
use rand::{thread_rng, Rng};
use store::{GameEvent, GameState, PlayerId, PointsRules, Stage, TurnStage};

const ERROR_REWARD: f32 = -2.12121;
const REWARD_VALID_MOVE: f32 = 2.12121;
const REWARD_RATIO: f32 = 0.01;
const WIN_POINTS: f32 = 0.1;

/// État du jeu Trictrac pour burn-rl
#[derive(Debug, Clone, Copy)]
pub struct TrictracState {
    pub data: [i8; 36], // Représentation vectorielle de l'état du jeu
}

impl State for TrictracState {
    type Data = [i8; 36];

    fn to_tensor<B: Backend>(&self) -> Tensor<B, 1> {
        Tensor::from_floats(self.data, &B::Device::default())
    }

    fn size() -> usize {
        36
    }
}

impl TrictracState {
    /// Convertit un GameState en TrictracState
    pub fn from_game_state(game_state: &GameState) -> Self {
        let state_vec = game_state.to_vec();
        let mut data = [0; 36];

        // Copier les données en s'assurant qu'on ne dépasse pas la taille
        let copy_len = state_vec.len().min(36);
        data[..copy_len].copy_from_slice(&state_vec[..copy_len]);

        TrictracState { data }
    }
}

/// Actions possibles dans Trictrac pour burn-rl
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrictracAction {
    // u32 as required by burn_rl::base::Action type
    pub index: u32,
}

impl Action for TrictracAction {
    fn random() -> Self {
        use rand::{thread_rng, Rng};
        let mut rng = thread_rng();
        TrictracAction {
            index: rng.gen_range(0..Self::size() as u32),
        }
    }

    fn enumerate() -> Vec<Self> {
        (0..Self::size() as u32)
            .map(|index| TrictracAction { index })
            .collect()
    }

    fn size() -> usize {
        1252
    }
}

impl From<u32> for TrictracAction {
    fn from(index: u32) -> Self {
        TrictracAction { index }
    }
}

impl From<TrictracAction> for u32 {
    fn from(action: TrictracAction) -> u32 {
        action.index
    }
}

/// Environnement Trictrac pour burn-rl
#[derive(Debug)]
pub struct TrictracEnvironment {
    pub game: GameState,
    active_player_id: PlayerId,
    opponent_id: PlayerId,
    current_state: TrictracState,
    episode_reward: f32,
    pub step_count: usize,
    pub min_steps: f32,
    pub max_steps: usize,
    pub pointrolls_count: usize,
    pub goodmoves_count: usize,
    pub goodmoves_ratio: f32,
    pub visualized: bool,
}

impl Environment for TrictracEnvironment {
    type StateType = TrictracState;
    type ActionType = TrictracAction;
    type RewardType = f32;

    fn new(visualized: bool) -> Self {
        let mut game = GameState::new(false);

        // Ajouter deux joueurs
        game.init_player("DQN Agent");
        game.init_player("Opponent");
        let player1_id = 1;
        let player2_id = 2;

        // Commencer la partie
        game.consume(&GameEvent::BeginGame { goes_first: 1 });

        let current_state = TrictracState::from_game_state(&game);
        TrictracEnvironment {
            game,
            active_player_id: player1_id,
            opponent_id: player2_id,
            current_state,
            episode_reward: 0.0,
            step_count: 0,
            min_steps: 250.0,
            max_steps: 2000,
            pointrolls_count: 0,
            goodmoves_count: 0,
            goodmoves_ratio: 0.0,
            visualized,
        }
    }

    fn state(&self) -> Self::StateType {
        self.current_state
    }

    fn reset(&mut self) -> Snapshot<Self> {
        // Réinitialiser le jeu
        self.game = GameState::new(false);
        self.game.init_player("DQN Agent");
        self.game.init_player("Opponent");

        // Commencer la partie
        self.game.consume(&GameEvent::BeginGame { goes_first: 1 });

        self.current_state = TrictracState::from_game_state(&self.game);
        self.episode_reward = 0.0;
        self.goodmoves_ratio = if self.step_count == 0 {
            0.0
        } else {
            self.goodmoves_count as f32 / self.step_count as f32
        };
        println!(
            "info: correct moves: {} ({}%)",
            self.goodmoves_count,
            (100.0 * self.goodmoves_ratio).round() as u32
        );
        self.step_count = 0;
        self.pointrolls_count = 0;
        self.goodmoves_count = 0;

        Snapshot::new(self.current_state, 0.0, false)
    }

    fn step(&mut self, action: Self::ActionType) -> Snapshot<Self> {
        self.step_count += 1;

        // Convertir l'action burn-rl vers une action Trictrac
        let trictrac_action = Self::convert_action(action);

        let mut reward = 0.0;
        let is_rollpoint;

        // Exécuter l'action si c'est le tour de l'agent DQN
        if self.game.active_player_id == self.active_player_id {
            if let Some(action) = trictrac_action {
                (reward, is_rollpoint) = self.execute_action(action);
                if is_rollpoint {
                    self.pointrolls_count += 1;
                }
                if reward != ERROR_REWARD {
                    self.goodmoves_count += 1;
                    // println!("{str_action}");
                }
            } else {
                // Action non convertible, pénalité
                reward = -0.5;
            }
        }

        // Faire jouer l'adversaire (stratégie simple)
        while self.game.active_player_id == self.opponent_id && self.game.stage != Stage::Ended {
            // print!(":");
            reward += self.play_opponent_if_needed();
        }

        // Vérifier si la partie est terminée
        let max_steps = self.min_steps
            + (self.max_steps as f32 - self.min_steps)
                * f32::exp((self.goodmoves_ratio - 1.0) / 0.25);
        let done = self.game.stage == Stage::Ended || self.game.determine_winner().is_some();

        if done {
            // Récompense finale basée sur le résultat
            if let Some(winner_id) = self.game.determine_winner() {
                if winner_id == self.active_player_id {
                    reward += WIN_POINTS; // Victoire
                } else {
                    reward -= WIN_POINTS; // Défaite
                }
            }
        }
        let terminated = done || self.step_count >= max_steps.round() as usize;

        // Mettre à jour l'état
        self.current_state = TrictracState::from_game_state(&self.game);
        self.episode_reward += reward;
        if self.visualized && terminated {
            println!(
                "Episode terminé. Récompense totale: {:.2}, Étapes: {}",
                self.episode_reward, self.step_count
            );
        }

        Snapshot::new(self.current_state, reward, terminated)
    }
}

impl TrictracEnvironment {
    /// Convertit une action burn-rl vers une action Trictrac
    pub fn convert_action(action: TrictracAction) -> Option<training_common_big::TrictracAction> {
        training_common_big::TrictracAction::from_action_index(action.index.try_into().unwrap())
    }

    /// Convertit l'index d'une action au sein des actions valides vers une action Trictrac
    #[allow(dead_code)]
    fn convert_valid_action_index(
        &self,
        action: TrictracAction,
        game_state: &GameState,
    ) -> Option<training_common_big::TrictracAction> {
        use training_common_big::get_valid_actions;

        // Obtenir les actions valides dans le contexte actuel
        let valid_actions = get_valid_actions(game_state);

        if valid_actions.is_empty() {
            return None;
        }

        // Mapper l'index d'action sur une action valide
        let action_index = (action.index as usize) % valid_actions.len();
        Some(valid_actions[action_index].clone())
    }

    /// Exécute une action Trictrac dans le jeu
    // fn execute_action(
    //     &mut self,
    //     action:training_common_big::TrictracAction,
    // ) -> Result<f32, Box<dyn std::error::Error>> {
    fn execute_action(&mut self, action: training_common_big::TrictracAction) -> (f32, bool) {
        use training_common_big::TrictracAction;

        let mut reward = 0.0;
        let mut is_rollpoint = false;
        let mut need_roll = false;

        let event = match action {
            TrictracAction::Roll => {
                // Lancer les dés
                need_roll = true;
                Some(GameEvent::Roll {
                    player_id: self.active_player_id,
                })
            }
            // TrictracAction::Mark => {
            //     // Marquer des points
            //     let points = self.game.
            //     reward += 0.1 * points as f32;
            //     Some(GameEvent::Mark {
            //         player_id: self.active_player_id,
            //         points,
            //     })
            // }
            TrictracAction::Go => {
                // Continuer après avoir gagné un trou
                Some(GameEvent::Go {
                    player_id: self.active_player_id,
                })
            }
            TrictracAction::Move {
                dice_order,
                from1,
                from2,
            } => {
                // Effectuer un mouvement
                let (dice1, dice2) = if dice_order {
                    (self.game.dice.values.0, self.game.dice.values.1)
                } else {
                    (self.game.dice.values.1, self.game.dice.values.0)
                };
                let mut to1 = from1 + dice1 as usize;
                let mut to2 = from2 + dice2 as usize;

                // Gestion prise de coin par puissance
                let opp_rest_field = 13;
                if to1 == opp_rest_field && to2 == opp_rest_field {
                    to1 -= 1;
                    to2 -= 1;
                }

                let checker_move1 = store::CheckerMove::new(from1, to1).unwrap_or_default();
                let checker_move2 = store::CheckerMove::new(from2, to2).unwrap_or_default();

                Some(GameEvent::Move {
                    player_id: self.active_player_id,
                    moves: (checker_move1, checker_move2),
                })
            }
        };

        // Appliquer l'événement si valide
        if let Some(event) = event {
            if self.game.validate(&event) {
                self.game.consume(&event);
                reward += REWARD_VALID_MOVE;
                // Simuler le résultat des dés après un Roll
                // if matches!(action, TrictracAction::Roll) {
                if need_roll {
                    let mut rng = thread_rng();
                    let dice_values = (rng.gen_range(1..=6), rng.gen_range(1..=6));
                    let dice_event = GameEvent::RollResult {
                        player_id: self.active_player_id,
                        dice: store::Dice {
                            values: dice_values,
                        },
                    };
                    // print!("o");
                    if self.game.validate(&dice_event) {
                        self.game.consume(&dice_event);
                        let (points, adv_points) = self.game.dice_points;
                        reward += REWARD_RATIO * (points - adv_points) as f32;
                        if points > 0 {
                            is_rollpoint = true;
                            // println!("info: rolled for {reward}");
                        }
                        // Récompense proportionnelle aux points
                    }
                }
            } else {
                // Pénalité pour action invalide
                // on annule les précédents reward
                // et on indique une valeur reconnaissable pour statistiques
                reward = ERROR_REWARD;
            }
        }

        (reward, is_rollpoint)
    }

    /// Fait jouer l'adversaire avec une stratégie simple
    fn play_opponent_if_needed(&mut self) -> f32 {
        // print!("z?");
        let mut reward = 0.0;

        // Si c'est le tour de l'adversaire, jouer automatiquement
        if self.game.active_player_id == self.opponent_id && self.game.stage != Stage::Ended {
            // Utiliser la stratégie default pour l'adversaire
            use crate::BotStrategy;

            let mut strategy = crate::strategy::random::RandomStrategy::default();
            strategy.set_player_id(self.opponent_id);
            if let Some(color) = self.game.player_color_by_id(&self.opponent_id) {
                strategy.set_color(color);
            }
            *strategy.get_mut_game() = self.game.clone();

            // Exécuter l'action selon le turn_stage
            let mut calculate_points = false;
            let opponent_color = store::Color::Black;
            let event = match self.game.turn_stage {
                TurnStage::RollDice => GameEvent::Roll {
                    player_id: self.opponent_id,
                },
                TurnStage::RollWaiting => {
                    let mut rng = thread_rng();
                    let dice_values = (rng.gen_range(1..=6), rng.gen_range(1..=6));
                    calculate_points = true; // comment to replicate burnrl_before
                    GameEvent::RollResult {
                        player_id: self.opponent_id,
                        dice: store::Dice {
                            values: dice_values,
                        },
                    }
                }
                TurnStage::MarkPoints => {
                    panic!("in play_opponent_if_needed > TurnStage::MarkPoints");
                    // let dice_roll_count = self
                    //     .game
                    //     .players
                    //     .get(&self.opponent_id)
                    //     .unwrap()
                    //     .dice_roll_count;
                    // let points_rules =
                    //     PointsRules::new(&opponent_color, &self.game.board, self.game.dice);
                    // GameEvent::Mark {
                    //     player_id: self.opponent_id,
                    //     points: points_rules.get_points(dice_roll_count).0,
                    // }
                }
                TurnStage::MarkAdvPoints => {
                    let dice_roll_count = self
                        .game
                        .players
                        .get(&self.opponent_id)
                        .unwrap()
                        .dice_roll_count;
                    let points_rules =
                        PointsRules::new(&opponent_color, &self.game.board, self.game.dice);
                    // pas de reward : déjà comptabilisé lors du tour de blanc
                    GameEvent::Mark {
                        player_id: self.opponent_id,
                        points: points_rules.get_points(dice_roll_count).1,
                    }
                }
                TurnStage::HoldOrGoChoice => {
                    // Stratégie simple : toujours continuer
                    GameEvent::Go {
                        player_id: self.opponent_id,
                    }
                }
                TurnStage::Move => GameEvent::Move {
                    player_id: self.opponent_id,
                    moves: strategy.choose_move(),
                },
            };

            if self.game.validate(&event) {
                self.game.consume(&event);
                // print!(".");
                if calculate_points {
                    // print!("x");
                    let dice_roll_count = self
                        .game
                        .players
                        .get(&self.opponent_id)
                        .unwrap()
                        .dice_roll_count;
                    let points_rules =
                        PointsRules::new(&opponent_color, &self.game.board, self.game.dice);
                    let (points, adv_points) = points_rules.get_points(dice_roll_count);
                    // Récompense proportionnelle aux points
                    let adv_reward = REWARD_RATIO * (points - adv_points) as f32;
                    reward -= adv_reward;
                    // if adv_reward != 0.0 {
                    //     println!("info: opponent : {adv_reward} -> {reward}");
                    // }
                }
            }
        }
        reward
    }
}

impl AsMut<TrictracEnvironment> for TrictracEnvironment {
    fn as_mut(&mut self) -> &mut Self {
        self
    }
}
