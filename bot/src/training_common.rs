use std::cmp::{max, min};
use std::fmt::{Debug, Display, Formatter};

use serde::{Deserialize, Serialize};
use store::{CheckerMove, GameEvent, GameState};

// 1 (Roll) + 1 (Go) + mouvements possibles
// Pour les mouvements : 2*16*16 = 514 (choix du dé + choix de la dame 0-15 pour chaque from)
pub const ACTION_SPACE_SIZE: usize = 514;

/// Types d'actions possibles dans le jeu
#[derive(Debug, Copy, Clone, Eq, Serialize, Deserialize, PartialEq)]
pub enum TrictracAction {
    /// Lancer les dés
    Roll,
    /// Continuer après avoir gagné un trou
    Go,
    /// Effectuer un mouvement de pions
    Move {
        dice_order: bool, // true = utiliser dice[0] en premier, false = dice[1] en premier
        checker1: usize, // premier pion à déplacer en numérotant depuis la colonne de départ (0-15) 0 : aucun pion
        checker2: usize, // deuxième pion (0-15)
    },
    // Marquer les points : à activer si support des écoles
    // Mark,
}

impl Display for TrictracAction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = format!("{self:?}");
        writeln!(f, "{}", s.chars().rev().collect::<String>())?;
        Ok(())
    }
}

impl TrictracAction {
    /// Encode une action en index pour le réseau de neurones
    pub fn to_action_index(&self) -> usize {
        match self {
            TrictracAction::Roll => 0,
            TrictracAction::Go => 1,
            TrictracAction::Move {
                dice_order,
                checker1,
                checker2,
            } => {
                // Encoder les mouvements dans l'espace d'actions
                // Indices 2+ pour les mouvements
                // de 2 à 513 (2 à  257 pour dé 1 en premier, 258 à 513 pour dé 2 en premier)
                let mut start = 2;
                if !dice_order {
                    // 16 * 16 = 256
                    start += 256;
                }
                start + checker1 * 16 + checker2
            } // TrictracAction::Mark => 514,
        }
    }

    pub fn to_event(&self, state: &GameState) -> Option<GameEvent> {
        match self {
            TrictracAction::Roll => {
                // Lancer les dés
                Some(GameEvent::Roll {
                    player_id: state.active_player_id,
                })
            }
            // TrictracAction::Mark => {
            //     // Marquer des points
            //     let points = self.game.
            //     Some(GameEvent::Mark {
            //         player_id: self.active_player_id,
            //         points,
            //     })
            // }
            TrictracAction::Go => {
                // Continuer après avoir gagné un trou
                Some(GameEvent::Go {
                    player_id: state.active_player_id,
                })
            }
            TrictracAction::Move {
                dice_order,
                checker1,
                checker2,
            } => {
                // Effectuer un mouvement
                let (dice1, dice2) = if *dice_order {
                    (state.dice.values.0, state.dice.values.1)
                } else {
                    (state.dice.values.1, state.dice.values.0)
                };

                let color = &store::Color::White;
                let from1 = state
                    .board
                    .get_checker_field(color, *checker1 as u8)
                    .unwrap_or(0);
                let mut to1 = from1 + dice1 as usize;
                let checker_move1 = store::CheckerMove::new(from1, to1).unwrap_or_default();

                let mut tmp_board = state.board.clone();
                let move_result = tmp_board.move_checker(color, checker_move1);
                if move_result.is_err() {
                    None
                    // panic!("Error while moving checker {move_result:?}")
                } else {
                    let from2 = tmp_board
                        .get_checker_field(color, *checker2 as u8)
                        .unwrap_or(0);
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
                        player_id: state.active_player_id,
                        moves: (checker_move1, checker_move2),
                    })
                }
            }
        }
    }

    /// Décode un index d'action en TrictracAction
    pub fn from_action_index(index: usize) -> Option<TrictracAction> {
        match index {
            0 => Some(TrictracAction::Roll),
            1 => Some(TrictracAction::Go),
            // 514 => Some(TrictracAction::Mark),
            i if i >= 2 => {
                let move_code = i - 2;
                let (dice_order, checker1, checker2) = Self::decode_move(move_code);
                Some(TrictracAction::Move {
                    dice_order,
                    checker1,
                    checker2,
                })
            }
            _ => None,
        }
    }

    /// Décode un entier en paire de mouvements
    fn decode_move(code: usize) -> (bool, usize, usize) {
        let mut encoded = code;
        let dice_order = code < 256;
        if !dice_order {
            encoded -= 256
        }
        let checker1 = encoded / 16;
        let checker2 = encoded % 16;
        (dice_order, checker1, checker2)
    }

    /// Retourne la taille de l'espace d'actions total
    pub fn action_space_size() -> usize {
        ACTION_SPACE_SIZE
    }

    // pub fn to_game_event(&self, player_id: PlayerId, dice: Dice) -> GameEvent {
    //     match action {
    //         TrictracAction::Roll => Some(GameEvent::Roll { player_id }),
    //         TrictracAction::Mark => Some(GameEvent::Mark { player_id, points }),
    //         TrictracAction::Go => Some(GameEvent::Go { player_id }),
    //         TrictracAction::Move {
    //             dice_order,
    //             from1,
    //             from2,
    //         } => {
    //             // Effectuer un mouvement
    //             let checker_move1 = store::CheckerMove::new(move1.0, move1.1).unwrap_or_default();
    //             let checker_move2 = store::CheckerMove::new(move2.0, move2.1).unwrap_or_default();
    //
    //             Some(GameEvent::Move {
    //                 player_id: self.agent_player_id,
    //                 moves: (checker_move1, checker_move2),
    //             })
    //         }
    //     };
    // }
}

/// Obtient les actions valides pour l'état de jeu actuel
pub fn get_valid_actions(game_state: &crate::GameState) -> Vec<TrictracAction> {
    use store::TurnStage;

    let mut valid_actions = Vec::new();

    let active_player_id = game_state.active_player_id;
    let player_color = game_state.player_color_by_id(&active_player_id);

    if let Some(color) = player_color {
        match game_state.turn_stage {
            TurnStage::RollDice => {
                valid_actions.push(TrictracAction::Roll);
            }
            TurnStage::MarkPoints | TurnStage::MarkAdvPoints | TurnStage::RollWaiting => {
                // valid_actions.push(TrictracAction::Mark);
                panic!(
                    "get_valid_actions not implemented for turn stage {:?}",
                    game_state.turn_stage
                );
            }
            TurnStage::HoldOrGoChoice => {
                valid_actions.push(TrictracAction::Go);

                // Ajoute aussi les mouvements possibles
                let rules = store::MoveRules::new(&color, &game_state.board, game_state.dice);
                let possible_moves = rules.get_possible_moves_sequences(true, vec![]);

                // Modififier checker_moves_to_trictrac_action si on doit gérer Black
                assert_eq!(color, store::Color::White);
                for (move1, move2) in possible_moves {
                    valid_actions.push(checker_moves_to_trictrac_action(
                        &move1, &move2, &color, game_state,
                    ));
                }
            }
            TurnStage::Move => {
                let rules = store::MoveRules::new(&color, &game_state.board, game_state.dice);
                let mut possible_moves = rules.get_possible_moves_sequences(true, vec![]);
                if possible_moves.is_empty() {
                    // Empty move
                    possible_moves.push((CheckerMove::default(), CheckerMove::default()));
                }

                // Modififier checker_moves_to_trictrac_action si on doit gérer Black
                assert_eq!(color, store::Color::White);
                for (move1, move2) in possible_moves {
                    valid_actions.push(checker_moves_to_trictrac_action(
                        &move1, &move2, &color, game_state,
                    ));
                }
            }
        }
    }

    if valid_actions.is_empty() {
        panic!("empty valid_actions for state {game_state}");
    }
    valid_actions
}

// Valid only for White player
fn checker_moves_to_trictrac_action(
    move1: &CheckerMove,
    move2: &CheckerMove,
    color: &store::Color,
    state: &crate::GameState,
) -> TrictracAction {
    let to1 = move1.get_to();
    let to2 = move2.get_to();
    let from1 = move1.get_from();
    let from2 = move2.get_from();
    let dice = state.dice;

    let mut diff_move1 = if to1 > 0 {
        // Mouvement sans sortie
        to1 - from1
    } else {
        // sortie, on utilise la valeur du dé
        if to2 > 0 {
            // sortie pour le mouvement 1 uniquement
            let dice2 = to2 - from2;
            if dice2 == dice.values.0 as usize {
                dice.values.1 as usize
            } else {
                dice.values.0 as usize
            }
        } else {
            // double sortie
            if from1 < from2 {
                max(dice.values.0, dice.values.1) as usize
            } else {
                min(dice.values.0, dice.values.1) as usize
            }
        }
    };

    // modification de diff_move1 si on est dans le cas d'un mouvement par puissance
    let rest_field = 12;
    if to1 == rest_field
        && to2 == rest_field
        && max(dice.values.0 as usize, dice.values.1 as usize) + min(from1, from2) != rest_field
    {
        // prise par puissance
        diff_move1 += 1;
    }
    let dice_order = diff_move1 == dice.values.0 as usize;

    let checker1 = state.board.get_field_checker(color, from1) as usize;
    let mut tmp_board = state.board.clone();
    // should not raise an error for a valid action
    let move_res = tmp_board.move_checker(color, *move1);
    if move_res.is_err() {
        panic!("error while moving checker {move_res:?}");
    }
    let checker2 = tmp_board.get_field_checker(color, from2) as usize;
    TrictracAction::Move {
        dice_order,
        checker1,
        checker2,
    }
}

/// Retourne les indices des actions valides
pub fn get_valid_action_indices(game_state: &crate::GameState) -> Vec<usize> {
    get_valid_actions(game_state)
        .into_iter()
        .map(|action| action.to_action_index())
        .collect()
}

/// Sélectionne une action valide aléatoire
pub fn sample_valid_action(game_state: &crate::GameState) -> Option<TrictracAction> {
    use rand::{seq::SliceRandom, thread_rng};

    let valid_actions = get_valid_actions(game_state);
    let mut rng = thread_rng();
    valid_actions.choose(&mut rng).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_action_index() {
        let action = TrictracAction::Move {
            dice_order: true,
            checker1: 3,
            checker2: 4,
        };
        let index = action.to_action_index();
        assert_eq!(Some(action), TrictracAction::from_action_index(index));
        assert_eq!(54, index);
    }

    #[test]
    fn from_action_index() {
        let action = TrictracAction::Move {
            dice_order: true,
            checker1: 3,
            checker2: 4,
        };
        assert_eq!(Some(action), TrictracAction::from_action_index(54));
    }
}
