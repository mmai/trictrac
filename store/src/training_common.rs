/// training_common.rs : environnement avec espace d'actions optimisé
/// (514 au lieu de 1252 pour training_common_big.rs de la branche 'big_and_full' )
use std::cmp::{max, min};
use std::fmt::{Debug, Display, Formatter};

use crate::board::Board;
use crate::{CheckerMove, Dice, GameEvent, GameState};
use serde::{Deserialize, Serialize};

// 1 (Roll) + 1 (Go) + 512 (mouvements possibles)
// avec 512 = 2 (choix du dé) * 16 * 16 (choix de la dame 0-15 pour chaque from)
pub const ACTION_SPACE_SIZE: usize = 514;

/// Types d'actions possibles dans le jeu
#[derive(Debug, Copy, Clone, Eq, Serialize, Deserialize, PartialEq)]
pub enum TrictracAction {
    /// Lancer les dés
    Roll,
    /// Faire un nouveau 'relevé' (repositionnement des dames à l'état de départ) après avoir gagné un trou,
    /// au lieu de continuer dans la position courante
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

    pub fn mirror(&self) -> TrictracAction {
        match self {
            TrictracAction::Roll => TrictracAction::Roll,
            TrictracAction::Go => TrictracAction::Go,
            TrictracAction::Move {
                dice_order,
                checker1,
                checker2,
            } => TrictracAction::Move {
                dice_order: *dice_order,
                checker1: if *checker1 == 0 { 0 } else { 25 - checker1 },
                checker2: if *checker2 == 0 { 0 } else { 25 - checker2 },
            },
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

                let color = &crate::Color::White;
                let from1 = state
                    .board
                    .get_checker_field(color, *checker1 as u8)
                    .unwrap_or(0);
                let mut to1 = from1 + dice1 as usize;
                if 24 < to1 {
                    // exit board
                    to1 = 0;
                }
                let checker_move1 = CheckerMove::new(from1, to1).unwrap_or_default();

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
                    if 24 < to2 {
                        // exit board
                        to2 = 0;
                    }

                    // Gestion prise de coin par puissance
                    let opp_rest_field = 13;
                    if to1 == opp_rest_field && to2 == opp_rest_field {
                        to1 -= 1;
                        to2 -= 1;
                    }

                    let checker_move1 = CheckerMove::new(from1, to1).unwrap_or_default();
                    let checker_move2 = CheckerMove::new(from2, to2).unwrap_or_default();

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
}

/// Obtient les actions valides pour l'état de jeu actuel
pub fn get_valid_actions(game_state: &GameState) -> anyhow::Result<Vec<TrictracAction>> {
    use crate::TurnStage;

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
                anyhow::bail!(
                    "get_valid_actions not implemented for turn stage {:?}",
                    game_state.turn_stage
                );
            }
            TurnStage::HoldOrGoChoice => {
                valid_actions.push(TrictracAction::Go);

                // Ajoute aussi les mouvements possibles
                let rules = crate::MoveRules::new(&color, &game_state.board, game_state.dice);
                let possible_moves = rules.get_possible_moves_sequences(true, vec![]);

                for (move1, move2) in possible_moves {
                    valid_actions.push(checker_moves_to_trictrac_action(
                        &move1, &move2, &color, game_state,
                    )?);
                }
            }
            TurnStage::Move => {
                let rules = crate::MoveRules::new(&color, &game_state.board, game_state.dice);
                let mut possible_moves = rules.get_possible_moves_sequences(true, vec![]);
                if possible_moves.is_empty() {
                    // Empty move
                    possible_moves.push((CheckerMove::default(), CheckerMove::default()));
                }

                for (move1, move2) in possible_moves {
                    valid_actions.push(checker_moves_to_trictrac_action(
                        &move1, &move2, &color, game_state,
                    )?);
                }
            }
        }
    }

    if valid_actions.is_empty() {
        anyhow::bail!("empty valid_actions for state {game_state}");
    }
    Ok(valid_actions)
}

fn checker_moves_to_trictrac_action(
    move1: &CheckerMove,
    move2: &CheckerMove,
    color: &crate::Color,
    state: &GameState,
) -> anyhow::Result<TrictracAction> {
    let dice = &state.dice;
    let board = &state.board;

    if color == &crate::Color::Black {
        // Moves are already 'white', so we don't mirror them
        white_checker_moves_to_trictrac_action(
            move1,
            move2,
            // &move1.clone().mirror(),
            // &move2.clone().mirror(),
            dice,
            &board.clone().mirror(),
        )
        // .map(|a| a.mirror())
    } else {
        white_checker_moves_to_trictrac_action(move1, move2, dice, board)
    }
}

fn white_checker_moves_to_trictrac_action(
    move1: &CheckerMove,
    move2: &CheckerMove,
    dice: &Dice,
    board: &Board,
) -> anyhow::Result<TrictracAction> {
    let to1 = move1.get_to();
    let to2 = move2.get_to();
    let from1 = move1.get_from();
    let from2 = move2.get_from();

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

    let checker1 = board.get_field_checker(&crate::Color::White, from1) as usize;
    let mut tmp_board = board.clone();
    // should not raise an error for a valid action
    tmp_board.move_checker(&crate::Color::White, *move1)?;
    let checker2 = tmp_board.get_field_checker(&crate::Color::White, from2) as usize;
    Ok(TrictracAction::Move {
        dice_order,
        checker1,
        checker2,
    })
}

/// Retourne les indices des actions valides
pub fn get_valid_action_indices(game_state: &GameState) -> anyhow::Result<Vec<usize>> {
    let actions = get_valid_actions(game_state)?;
    Ok(actions
        .into_iter()
        .map(|action| action.to_action_index())
        .collect())
}

/// Sélectionne une action valide aléatoire
pub fn sample_valid_action(game_state: &GameState) -> Option<TrictracAction> {
    use rand::{prelude::IndexedRandom, rng};

    let valid_actions = get_valid_actions(game_state);
    let mut rng = rng();
    valid_actions
        .map(|va| va.choose(&mut rng).cloned())
        .unwrap_or_default()
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

    #[test]
    fn get_valid_actions() {
        let mut state = GameState::new_with_players("white", "black");
        state.active_player_id = 2;
        state.dice = Dice { values: (5, 3) };
        state.turn_stage = crate::TurnStage::Move;
        state.board.set_positions(
            &crate::Color::White,
            [
                -3, -3, -2, -2, -2, -2, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 1, 1, 3, 8,
            ],
        );

        let actions = vec![TrictracAction::Move {
            dice_order: true,
            checker1: 11,
            checker2: 13,
        }];
        assert_eq!(Some(actions), super::get_valid_actions(&state).ok());
    }

    #[test]
    fn checker_moves_to_trictrac_action() {
        let mut state = GameState::new_with_players("white", "black");
        state.turn_stage = crate::TurnStage::Move;
        state.dice = Dice { values: (5, 3) };

        // White player
        state.active_player_id = 1;
        state.board.set_positions(
            &crate::Color::White,
            [
                -8, -3, -1, -1, 0, -1, 0, 0, 0, 0, -1, 0, 0, 0, 0, 0, 0, 0, 2, 2, 2, 2, 3, 3,
            ],
        );

        let ttaction = super::checker_moves_to_trictrac_action(
            &CheckerMove::new(23, 0).unwrap(),
            &CheckerMove::new(24, 0).unwrap(),
            &crate::Color::White,
            &state,
        );

        assert_eq!(
            Some(TrictracAction::Move {
                dice_order: true,
                checker1: 11,
                checker2: 13, // because the 11th has left
            }),
            ttaction.ok()
        );

        // Black player
        state.active_player_id = 2;
        state.board.set_positions(
            &crate::Color::White,
            [
                -3, -3, -2, -2, -2, -2, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 1, 1, 3, 8,
            ],
        );
        let ttaction = super::checker_moves_to_trictrac_action(
            // &CheckerMove::new(2, 0).unwrap(),
            // &CheckerMove::new(1, 0).unwrap(),
            &CheckerMove::new(23, 0).unwrap(),
            &CheckerMove::new(24, 0).unwrap(),
            &crate::Color::Black,
            &state,
        );

        assert_eq!(
            Some(TrictracAction::Move {
                dice_order: true,
                checker1: 11,
                checker2: 13, // because the 11th has left
            }),
            ttaction.ok()
        );
    }
}
