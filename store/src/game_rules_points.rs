use std::collections::HashMap;

use crate::board::{Board, EMPTY_MOVE};
use crate::dice::Dice;
use crate::game_rules_moves::MoveRules;
use crate::player::Color;
use crate::CheckerMove;
use crate::Error;

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
enum Jan {
    FilledQuarter,
    TrueHitSmallJan,
    TrueHitBigJan,
    // jans de récompense :
    //  - battre une dame seule (par autant de façons de le faire, y compris
    // utilisant une dame du coin de repos)
    //  - battre le coin adverse : si deux dames (hormis les deux dernière de son propre coin de
    // repos) peuvent battre le coin vide adverse
    // jans qui ne peut (pts pour l'adversaire) :
    //  - battre à faux :  si on passe par une case pleine pour atteindre la
    // case que l'on peut battre
    //  - si on ne peut pas jouer ses deux dés
}

impl Jan {
    pub fn get_points(&self, is_double: bool) -> i8 {
        match self {
            Self::TrueHitBigJan => {
                if is_double {
                    4
                } else {
                    2
                }
            }
            _ => {
                if is_double {
                    6
                } else {
                    4
                }
            }
        }
    }

    // « JAN DE RÉCOMPENSE »
    // Battre à vrai une dame située dans la table des grands jans 	2 | 4 	1, 2 ou 3 (sauf doublet) 	Joueur
    // Battre à vrai une dame située dans la table des petits jans 	4 | 6 	1, 2 ou 3 	Joueur
    // Battre le coin adverse 	4 	6 	1 	Joueur
}

type PossibleJans = HashMap<Jan, Vec<(CheckerMove, CheckerMove)>>;

trait PossibleJansMethods {
    fn push(&mut self, jan: Jan, cmoves: (CheckerMove, CheckerMove));
    fn merge(&mut self, other: Self);
    // fn get_points(&self) -> u8;
}

impl PossibleJansMethods for PossibleJans {
    fn push(&mut self, jan: Jan, cmoves: (CheckerMove, CheckerMove)) {
        if let Some(ways) = self.get_mut(&jan) {
            if !ways.contains(&cmoves) {
                ways.push(cmoves);
            }
        } else {
            self.insert(jan, [cmoves].into());
        }
    }

    fn merge(&mut self, other: Self) {
        for (jan, cmoves_list) in other {
            for cmoves in cmoves_list {
                self.push(jan.clone(), cmoves);
            }
        }
    }
}

/// PointsRules always consider that the current player is White
/// You must use 'mirror' function on board if player is Black
#[derive(Default)]
pub struct PointsRules {
    pub board: Board,
    pub dice: Dice,
    pub move_rules: MoveRules,
}

impl PointsRules {
    /// Revert board if color is black
    pub fn new(color: &Color, board: &Board, dice: Dice) -> Self {
        let board = if *color == Color::Black {
            board.mirror()
        } else {
            board.clone()
        };
        let move_rules = MoveRules::new(color, &board, dice);

        // let move_rules = MoveRules::new(color, &self.board, dice, moves);
        Self {
            board,
            dice,
            move_rules,
        }
    }

    fn get_jans(&self, board_ini: &Board, dices: &Vec<u8>) -> PossibleJans {
        let mut dices_reversed = dices.clone();
        dices_reversed.reverse();

        let mut jans = self.get_jans_by_dice_order(board_ini, dices);
        let jans_revert_dices = self.get_jans_by_dice_order(board_ini, &dices_reversed);
        jans.merge(jans_revert_dices);
        jans
    }

    fn get_jans_by_dice_order(&self, board_ini: &Board, dices: &Vec<u8>) -> PossibleJans {
        let mut jans = PossibleJans::default();
        let mut dices = dices.clone();
        if let Some(dice) = dices.pop() {
            let color = Color::White;
            let mut board = board_ini.clone();
            let corner_field = board.get_color_corner(&color);
            let adv_corner_field = board.get_color_corner(&Color::Black);
            for (from, _) in board.get_color_fields(color) {
                let to = if from + dice as usize > 24 {
                    0
                } else {
                    from + dice as usize
                };
                if let Ok(cmove) = CheckerMove::new(from, to) {
                    // let res = state.moves_allowed(&moves);
                    // if res.is_ok() {
                    //     println!("dice : {:?}, res : {:?}", dice, res);
                    // On vérifie que le mouvement n'est pas interdit par les règles des coins de
                    // repos :
                    // - on ne va pas sur le coin de l'adversaire
                    // - ni sur son propre coin de repos avec une seule dame
                    // - règle non prise en compte pour le battage des dames : on ne sort pas de son coin de repos s'il ni reste que deux dames
                    let (corner_count, _color) = board.get_field_checkers(corner_field).unwrap();
                    if to != adv_corner_field && (to != corner_field || corner_count > 1)
                    // && (from != corner_field || corner_count > 2)
                    {
                        // println!(
                        //     "dice : {}, adv_corn_field : {:?}, from : {}, to : {}, corner_count : {}",
                        //     dice, adv_corner_field, from, to, corner_count
                        // );
                        let mut can_try_toutdune = true;
                        match board.move_checker(&color, cmove) {
                            Err(Error::FieldBlockedByOne) => {
                                let jan = if Board::is_field_in_small_jan(to) {
                                    Jan::TrueHitSmallJan
                                } else {
                                    Jan::TrueHitBigJan
                                };
                                jans.push(jan, (cmove, EMPTY_MOVE));
                            }
                            Err(_) => {
                                can_try_toutdune = false;
                                // let next_dice_jan = self.get_jans(&board, &dices);
                                // jans possibles en tout d'une après un battage à vrai :
                                // truehit
                            }
                            Ok(()) => {}
                        }
                        if can_try_toutdune {
                            // Try tout d'une :
                            // - use original board before first die move
                            // - use a virtual dice by adding current dice to remaining dice
                            let next_dice_jan = self.get_jans_by_dice_order(
                                &board_ini,
                                &dices.iter().map(|d| d + dice).collect(),
                            );
                            jans.merge(next_dice_jan);
                        }
                    }
                    // Second die
                    let next_dice_jan = self.get_jans_by_dice_order(&board_ini, &dices);
                    jans.merge(next_dice_jan);
                }
            }
        }

        // TODO : mouvements en tout d'une asdf
        // - faire un dé d1+d2 et regarder si hit
        // - si hit : regarder s'il existe le truehit intermédiaire
        // - regarder les TrueHit qui nécessitent deux mouvemments non nuls
        // TODO : tout d'une (sans doublons avec 1 + 1) ?
        jans
    }

    pub fn get_points(&self) -> i8 {
        let mut points: i8 = 0;

        let jans = self.get_jans(&self.board, &vec![self.dice.values.0, self.dice.values.1]);
        points += jans.into_iter().fold(0, |acc: i8, (jan, moves)| {
            acc + jan.get_points(self.dice.is_double()) * (moves.len() as i8)
        });

        // Jans de remplissage
        let filling_moves_sequences = self.move_rules.get_quarter_filling_moves_sequences();
        points += 4 * filling_moves_sequences.len() as i8;
        // cf. https://fr.wikipedia.org/wiki/Trictrac
        //  	Points par simple par moyen | Points par doublet par moyen 	Nombre de moyens possibles 	Bénéficiaire
        // « JAN RARE »
        // Jan de six tables 	4 	n/a 	1 	Joueur
        // Jan de deux tables 	4 	6 	1 	Joueur
        // Jan de mézéas 	4 	6 	1 	Joueur
        // Contre jan de deux tables 	4 	6 	1 	Adversaire
        // Contre jan de mézéas 	4 	6 	1 	Adversaire
        // « JAN DE RÉCOMPENSE »
        // Battre à vrai une dame située dans la table des grands jans 	2 | 4 	1, 2 ou 3 (sauf doublet) 	Joueur
        // Battre à vrai une dame située dans la table des petits jans 	4 | 6 	1, 2 ou 3 	Joueur
        // Battre le coin adverse 	4 	6 	1 	Joueur
        // « JAN QUI NE PEUT »
        // Battre à faux une dame
        // située dans la table des grands jans 	2 	4 	1 	Adversaire
        // Battre à faux une dame
        // située dans la table des petits jans 	4 	6 	1 	Adversaire
        // Pour chaque dé non jouable (dame impuissante) 	2 	2 	n/a 	Adversaire
        // « JAN DE REMPLISSAGE »
        // Faire un petit jan, un grand jan ou un jan de retour 	4 		1, 2, ou 3 	Joueur
        // 	6 	1 ou 2 	Joueur
        // Conserver un petit jan, un grand jan ou un jan de retour 	4 	6 	1 	Joueur
        // « AUTRE »
        // Sortir le premier toutes ses dames 	4 	6 	n/a 	Joueur

        points
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn get_jans_by_dice_order() {
        let mut rules = PointsRules::default();
        rules.board.set_positions([
            2, 0, -1, -1, 0, -1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ]);

        let jans = rules.get_jans_by_dice_order(&rules.board, &vec![2, 3]);
        assert_eq!(1, jans.len());
        assert_eq!(3, jans.get(&Jan::TrueHitSmallJan).unwrap().len());

        let jans = rules.get_jans_by_dice_order(&rules.board, &vec![2, 2]);
        assert_eq!(1, jans.len());
        assert_eq!(1, jans.get(&Jan::TrueHitSmallJan).unwrap().len());

        // On peut passer par une dame battue pour battre une autre dame
        // mais pas par une case remplie par l'adversaire
        rules.board.set_positions([
            2, 0, -1, -2, 0, -1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ]);

        let mut jans = rules.get_jans_by_dice_order(&rules.board, &vec![2, 3]);
        let jans_revert_dices = rules.get_jans_by_dice_order(&rules.board, &vec![3, 2]);
        assert_eq!(1, jans.len());
        assert_eq!(1, jans_revert_dices.len());
        jans.merge(jans_revert_dices);
        assert_eq!(2, jans.get(&Jan::TrueHitSmallJan).unwrap().len());

        rules.board.set_positions([
            2, 0, -1, 0, 0, -1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ]);

        let jans = rules.get_jans_by_dice_order(&rules.board, &vec![2, 3]);
        assert_eq!(1, jans.len());
        assert_eq!(2, jans.get(&Jan::TrueHitSmallJan).unwrap().len());

        rules.board.set_positions([
            2, 0, 0, 0, 0, -1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ]);

        let jans = rules.get_jans_by_dice_order(&rules.board, &vec![2, 3]);
        assert_eq!(1, jans.len());
        assert_eq!(1, jans.get(&Jan::TrueHitSmallJan).unwrap().len());

        rules.board.set_positions([
            2, 0, 1, 1, 0, -1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ]);

        let jans = rules.get_jans_by_dice_order(&rules.board, &vec![2, 3]);
        assert_eq!(1, jans.len());
        assert_eq!(3, jans.get(&Jan::TrueHitSmallJan).unwrap().len());

        // corners handling

        // deux dés bloqués (coin de repos et coin de l'adversaire)
        rules.board.set_positions([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, -1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ]);
        // le premier dé traité est le dernier du vecteur : 1
        let jans = rules.get_jans_by_dice_order(&rules.board, &vec![2, 1]);
        // println!("jans (dés bloqués) : {:?}", jans.get(&Jan::TrueHit));
        assert_eq!(0, jans.len());

        // dé dans son coin de repos : peut tout de même battre à vrai
        rules.board.set_positions([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, -1, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ]);
        let mut jans = rules.get_jans_by_dice_order(&rules.board, &vec![3, 3]);
        assert_eq!(1, jans.len());

        // premier dé bloqué, mais tout d'une possible en commençant par le second
        rules.board.set_positions([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, -1, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ]);
        let mut jans = rules.get_jans_by_dice_order(&rules.board, &vec![3, 1]);
        let jans_revert_dices = rules.get_jans_by_dice_order(&rules.board, &vec![1, 3]);
        assert_eq!(1, jans_revert_dices.len());

        jans.merge(jans_revert_dices);
        assert_eq!(1, jans.len());
        // print!("jans (2) : {:?}", jans.get(&Jan::TrueHit));

        // battage à faux : ne pas prendre en compte si en inversant l'ordre des dés il y a battage
        // à vrai
    }

    #[test]
    fn get_points() {
        let mut rules = PointsRules::default();
        rules.board.set_positions([
            2, 0, -1, -1, 0, -1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ]);
        rules.dice = Dice { values: (2, 3) };
        assert_eq!(12, rules.get_points());
    }
}
