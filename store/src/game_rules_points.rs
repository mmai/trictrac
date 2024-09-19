use std::cmp;
use std::collections::HashMap;

use crate::board::{Board, Field, EMPTY_MOVE};
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
    TrueHitOpponentCorner,
    FirstPlayerToExit,
    SixTables,
    TwoTables,
    Mezeas,

    FalseHitSmallJan,
    FalseHitBigJan,
    ContreTwoTables,
    ContreMezeas,
    HelplessMan,
}

impl Jan {
    pub fn get_points(&self, is_double: bool) -> i8 {
        match self {
            Self::FalseHitSmallJan | Self::ContreTwoTables | Self::ContreMezeas => {
                if is_double {
                    -6
                } else {
                    -4
                }
            }
            Self::FalseHitBigJan | Self::HelplessMan => {
                if is_double {
                    -4
                } else {
                    -2
                }
            }
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
                // pour un même mouvement, le battage à vrai est prioritaire sur le battage à faux.
                match jan {
                    Jan::FalseHitBigJan => {
                        let mut has_true_hit = false;
                        if let Some(true_moves) = self.get(&Jan::TrueHitBigJan) {
                            has_true_hit = true_moves.contains(&cmoves);
                        }
                        if !has_true_hit {
                            self.push(jan.clone(), cmoves);
                        }
                    }
                    Jan::FalseHitSmallJan => {
                        let mut has_true_hit = false;
                        if let Some(true_moves) = self.get(&Jan::TrueHitSmallJan) {
                            has_true_hit = true_moves.contains(&cmoves);
                        }
                        if !has_true_hit {
                            self.push(jan.clone(), cmoves);
                        }
                    }
                    Jan::TrueHitBigJan => {
                        if let Some(false_moves) = self.get_mut(&Jan::FalseHitBigJan) {
                            false_moves.retain(|fmoves| *fmoves != cmoves);
                        }
                        self.push(jan.clone(), cmoves);
                    }
                    Jan::TrueHitSmallJan => {
                        if let Some(false_moves) = self.get_mut(&Jan::FalseHitSmallJan) {
                            false_moves.retain(|fmoves| *fmoves != cmoves);
                        }
                        self.push(jan.clone(), cmoves);
                    }
                    _ => {
                        self.push(jan.clone(), cmoves);
                    }
                }
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

    pub fn set_dice(&mut self, dice: Dice) {
        self.dice = dice;
        self.move_rules.dice = dice;
    }

    pub fn update_positions(&mut self, positions: [i8; 24]) {
        self.board.set_positions(positions);
        self.move_rules.board.set_positions(positions);
    }

    fn get_jans(&self, board_ini: &Board) -> PossibleJans {
        let dices = &vec![self.dice.values.0, self.dice.values.1];
        let dices_reversed = &vec![self.dice.values.1, self.dice.values.0];
        let dice1 = self.dice.values.0 as usize;
        let dice2 = self.dice.values.1 as usize;

        // « JAN DE RÉCOMPENSE »
        // Battre à vrai une dame située dans la table des grands jans
        // Battre à vrai une dame située dans la table des petits jans
        let mut jans = self.get_jans_by_ordered_dice(board_ini, dices, None, false);
        let jans_revert_dices =
            self.get_jans_by_ordered_dice(board_ini, dices_reversed, None, false);
        jans.merge(jans_revert_dices);

        // Battre à vrai le coin de repos de l'adversaire
        let corner_field = board_ini.get_color_corner(&Color::White);
        let adv_corner_field = board_ini.get_color_corner(&Color::Black);
        let (adv_corner_count, _color) = board_ini.get_field_checkers(adv_corner_field).unwrap();
        if adv_corner_count == 0 {
            let from0 = adv_corner_field - self.dice.values.0 as usize;
            let from1 = adv_corner_field - self.dice.values.1 as usize;

            let (from0_count, from0_color) = board_ini.get_field_checkers(from0).unwrap();
            let (from1_count, from1_color) = board_ini.get_field_checkers(from1).unwrap();
            let hit_moves = vec![(
                CheckerMove::new(from0, adv_corner_field).unwrap(),
                CheckerMove::new(from1, adv_corner_field).unwrap(),
            )];

            if from0 == from1 {
                // doublet
                if from0_count > if from0 == corner_field { 3 } else { 1 } {
                    jans.insert(Jan::TrueHitOpponentCorner, hit_moves);
                }
            } else {
                // simple
                if from0_count > if from0 == corner_field { 2 } else { 0 }
                    && from1_count > if from1 == corner_field { 2 } else { 0 }
                {
                    jans.insert(Jan::TrueHitOpponentCorner, hit_moves);
                }
            }
        }

        // « JAN DE REMPLISSAGE »
        // Faire un petit jan, un grand jan ou un jan de retour
        let filling_moves_sequences = self
            .move_rules
            .get_scoring_quarter_filling_moves_sequences();
        if !filling_moves_sequences.is_empty() {
            jans.insert(Jan::FilledQuarter, filling_moves_sequences);
        }

        // « AUTRE »
        // sortir le premier toutes ses dames
        let mut checkers = board_ini.get_color_fields(Color::White);
        checkers.sort_by(|a, b| b.0.cmp(&a.0));
        let checkers_count = checkers.iter().fold(0, |acc, (_f, count)| acc + count);
        if checkers_count < 3 {
            let mut farthest = 24;
            let mut next_farthest = 24;
            if let Some((field, count)) = checkers.first() {
                farthest = *field;
                if *count > 1 {
                    next_farthest = *field;
                } else if let Some((field, _count)) = checkers.get(1) {
                    next_farthest = *field;
                }
            }

            if farthest + cmp::max(self.dice.values.0, self.dice.values.1) as usize > 23
                && next_farthest + cmp::min(self.dice.values.0, self.dice.values.1) as usize > 23
            {
                let exit_moves = vec![(
                    CheckerMove::new(farthest, 0).unwrap(),
                    if checkers_count > 1 {
                        CheckerMove::new(next_farthest, 0).unwrap()
                    } else {
                        CheckerMove::new(0, 0).unwrap()
                    },
                )];

                jans.insert(Jan::FirstPlayerToExit, exit_moves);
            }
        }

        // « JANS RARES »
        // Jan de 6 tables
        //   on devrait avoir 4 cases occupées par une dame chacune
        let fields_with_single: Vec<&(usize, i8)> =
            checkers.iter().filter(|(f, c)| c == &1).collect();
        if fields_with_single.len() == 4 {
            let checkers_fields: Vec<usize> = checkers.iter().map(|(f, c)| *f).collect();
            let mut missing_for_6tables: Vec<usize> = Vec::from([2, 3, 4, 5, 6, 7])
                .into_iter()
                .filter(|f| !checkers_fields.contains(f))
                .collect();
            if missing_for_6tables.len() == 2 {
                // Les dés doivent permettre le mouvement de deux dames du talon vers les 2 cases
                // vides
                let mut dice_to: Vec<usize> = vec![
                    1 + self.dice.values.0 as usize,
                    1 + self.dice.values.1 as usize,
                ];
                missing_for_6tables.sort();
                dice_to.sort();
                if dice_to == missing_for_6tables {
                    let moves = vec![(
                        CheckerMove::new(1, missing_for_6tables[0]).unwrap(),
                        CheckerMove::new(1, missing_for_6tables[1]).unwrap(),
                    )];
                    jans.insert(Jan::SixTables, moves);
                }
            }
        }

        // Jans nécessitant que deux dames uniquement soient sorties du talon
        let (talon, candidates): (Vec<(usize, i8)>, Vec<(usize, i8)>) =
            checkers.iter().partition(|(field, count)| field == &1);
        let candidates_fields = candidates.iter().fold(vec![], |mut acc, (f, c)| {
            acc.extend_from_slice(&vec![*f; *c as usize]);
            acc
        });
        if !talon.is_empty() && talon[0].1 == 13 && candidates_fields.len() == 2 {
            let field1 = candidates_fields[0];
            let field2 = candidates_fields[1];

            // Jan de 2 tables et contre jan de 2 tables
            let jan = if adv_corner_count == 0 {
                Jan::TwoTables
            } else {
                Jan::ContreTwoTables
            };
            if (field1 + dice1 == 12 && field2 + dice2 == 13)
                || (field1 + dice2 == 12 && field2 + dice1 == 13)
            {
                let moves = vec![(
                    CheckerMove::new(field1, 12).unwrap(),
                    CheckerMove::new(field2, 13).unwrap(),
                )];
                jans.insert(jan, moves);
            } else if (field1 + dice1 == 13 && field2 + dice2 == 12)
                || (field1 + dice2 == 13 && field2 + dice1 == 12)
            {
                let moves = vec![(
                    CheckerMove::new(field1, 13).unwrap(),
                    CheckerMove::new(field2, 12).unwrap(),
                )];
                jans.insert(jan, moves);
            }

            // Jan de Mezeas et contre jan de Mezeas
            let jan = if adv_corner_count == 0 {
                Jan::Mezeas
            } else {
                Jan::ContreMezeas
            };
            if field1 == 12 && field2 == 12 && (dice1 == 1 || dice2 == 1) {
                let moves = vec![(
                    CheckerMove::new(field1, field1 + dice1).unwrap(),
                    CheckerMove::new(field2, field2 + dice2).unwrap(),
                )];
                jans.insert(jan, moves);
            }
        }

        // Jan qui ne peut : dés non jouables
        let poss = self.move_rules.get_possible_moves_sequences(true);
        let moves = poss.iter().fold(vec![], |mut acc, (m1, m2)| {
            acc.push(*m1);
            acc.push(*m2);
            acc
        });
        let moves_dice1: Vec<&CheckerMove> =
            moves.iter().filter(|m| m.doable_with_dice(dice1)).collect();
        let moves_dice2: Vec<&CheckerMove> =
            moves.iter().filter(|m| m.doable_with_dice(dice2)).collect();
        if poss.is_empty() {
            jans.insert(
                Jan::HelplessMan,
                vec![(CheckerMove::default(), CheckerMove::default())],
            );
        } else if moves_dice1.is_empty() || moves_dice2.is_empty() {
            jans.insert(Jan::HelplessMan, vec![poss[0]]);
        }

        jans
    }

    fn get_jans_by_ordered_dice(
        &self,
        board_ini: &Board,
        dices: &Vec<u8>,
        only_from: Option<usize>,
        only_false_hit: bool,
    ) -> PossibleJans {
        let mut jans = PossibleJans::default();
        let mut dices = dices.clone();
        if let Some(dice) = dices.pop() {
            let color = Color::White;
            let mut board = board_ini.clone();
            let corner_field = board.get_color_corner(&color);
            let adv_corner_field = board.get_color_corner(&Color::Black);
            let froms = if let Some(from) = only_from {
                vec![from]
            } else {
                board
                    .get_color_fields(color)
                    .iter()
                    .map(|cf| cf.0)
                    .collect()
            };
            for from in froms {
                // for (from, _) in board.get_color_fields(color) {
                let to = if from + dice as usize > 24 {
                    0
                } else {
                    from + dice as usize
                };
                if let Ok(cmove) = CheckerMove::new(from, to) {
                    // print!(
                    //     " <dice_move dice='{:?}' moves='{:?} -> {:?}'> ",
                    //     dice, from, to
                    // );
                    // On vérifie que le mouvement n'est pas interdit par les règles des coins de
                    // repos :
                    // - on ne va pas sur le coin de l'adversaire
                    // - ni sur son propre coin de repos avec une seule dame
                    // - règle non prise en compte pour le battage des dames : on ne sort pas de son coin de repos s'il n'y reste que deux dames
                    let (corner_count, _color) = board.get_field_checkers(corner_field).unwrap();
                    if to != adv_corner_field && (to != corner_field || corner_count > 1) {
                        // si only_false_hit est vrai, on est déja dans une tentative tout d'une
                        let mut can_try_toutdune = !only_false_hit;
                        let mut only_falsehit = false;
                        match board.move_checker(&color, cmove) {
                            Err(Error::FieldBlockedByOne) => {
                                let jan = match (Board::is_field_in_small_jan(to), only_false_hit) {
                                    (true, false) => Jan::TrueHitSmallJan,
                                    (true, true) => Jan::FalseHitSmallJan,
                                    (false, false) => Jan::TrueHitBigJan,
                                    (false, true) => Jan::FalseHitBigJan,
                                };
                                jans.push(jan, (cmove, EMPTY_MOVE));
                            }
                            Err(Error::FieldBlocked) => {
                                only_falsehit = true;
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
                            // - limit the checker to the current one
                            let next_dice_jan = self.get_jans_by_ordered_dice(
                                &board_ini,
                                &dices.iter().map(|d| d + dice).collect(),
                                Some(from),
                                only_falsehit,
                            );
                            jans.merge(next_dice_jan);
                        }
                    }
                    // Second die
                    let next_dice_jan =
                        self.get_jans_by_ordered_dice(&board_ini, &dices, None, false);
                    jans.merge(next_dice_jan);
                }
            }
        }

        jans
    }

    pub fn get_points(&self) -> (i8, i8) {
        let jans = self.get_jans(&self.board);
        let (points, adv_points) = jans
            .into_iter()
            .fold((0, 0), |acc: (i8, i8), (jan, moves)| {
                println!("get_points : {:?}", jan);
                let is_double = if jan == Jan::HelplessMan {
                    moves[0] == (CheckerMove::default(), CheckerMove::default())
                } else {
                    self.dice.is_double()
                };
                let points = jan.get_points(is_double) * (moves.len() as i8);
                if points < 0 {
                    (acc.0, acc.1 - points)
                } else {
                    (acc.0 + points, acc.1)
                }
            });

        (points, adv_points)
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

        let jans = rules.get_jans_by_ordered_dice(&rules.board, &vec![2, 3], None, false);
        assert_eq!(1, jans.len());
        assert_eq!(3, jans.get(&Jan::TrueHitSmallJan).unwrap().len());

        let jans = rules.get_jans_by_ordered_dice(&rules.board, &vec![2, 2], None, false);
        assert_eq!(1, jans.len());
        assert_eq!(1, jans.get(&Jan::TrueHitSmallJan).unwrap().len());

        // On peut passer par une dame battue pour battre une autre dame
        // mais pas par une case remplie par l'adversaire
        rules.board.set_positions([
            2, 0, -1, -2, 0, -1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ]);

        let mut jans = rules.get_jans_by_ordered_dice(&rules.board, &vec![2, 3], None, false);
        let jans_revert_dices =
            rules.get_jans_by_ordered_dice(&rules.board, &vec![3, 2], None, false);
        assert_eq!(2, jans.len());
        assert_eq!(1, jans_revert_dices.len());
        jans.merge(jans_revert_dices);
        assert_eq!(2, jans.get(&Jan::TrueHitSmallJan).unwrap().len());

        rules.board.set_positions([
            2, 0, -1, 0, 0, -1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ]);

        let jans = rules.get_jans_by_ordered_dice(&rules.board, &vec![2, 3], None, false);
        assert_eq!(1, jans.len());
        assert_eq!(2, jans.get(&Jan::TrueHitSmallJan).unwrap().len());

        rules.board.set_positions([
            2, 0, 0, 0, 0, -1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ]);

        let jans = rules.get_jans_by_ordered_dice(&rules.board, &vec![2, 3], None, false);
        assert_eq!(1, jans.len());
        assert_eq!(1, jans.get(&Jan::TrueHitSmallJan).unwrap().len());

        rules.board.set_positions([
            2, 0, 1, 1, 0, -1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ]);

        let jans = rules.get_jans_by_ordered_dice(&rules.board, &vec![2, 3], None, false);
        assert_eq!(1, jans.len());
        assert_eq!(3, jans.get(&Jan::TrueHitSmallJan).unwrap().len());

        // corners handling

        // deux dés bloqués (coin de repos et coin de l'adversaire)
        rules.board.set_positions([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, -1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ]);
        // le premier dé traité est le dernier du vecteur : 1
        let jans = rules.get_jans_by_ordered_dice(&rules.board, &vec![2, 1], None, false);
        // println!("jans (dés bloqués) : {:?}", jans.get(&Jan::TrueHit));
        assert_eq!(0, jans.len());

        // dé dans son coin de repos : peut tout de même battre à vrai
        rules.board.set_positions([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, -1, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ]);
        let jans = rules.get_jans_by_ordered_dice(&rules.board, &vec![3, 3], None, false);
        assert_eq!(1, jans.len());

        // premier dé bloqué, mais tout d'une possible en commençant par le second
        rules.board.set_positions([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, -1, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ]);
        let mut jans = rules.get_jans_by_ordered_dice(&rules.board, &vec![3, 1], None, false);
        let jans_revert_dices =
            rules.get_jans_by_ordered_dice(&rules.board, &vec![1, 3], None, false);
        assert_eq!(1, jans_revert_dices.len());

        jans.merge(jans_revert_dices);
        assert_eq!(1, jans.len());
        // print!("jans (2) : {:?}", jans.get(&Jan::TrueHit));

        // battage à faux : ne pas prendre en compte si en inversant l'ordre des dés il y a battage
        // à vrai
    }

    #[test]
    fn get_points() {
        // ----- Jan de récompense
        //  Battre à vrai une dame située dans la table des petits jans : 4 + 4 + 4 = 12
        let mut rules = PointsRules::default();
        rules.update_positions([
            2, 0, -1, -1, 0, -1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ]);
        rules.set_dice(Dice { values: (2, 3) });
        assert_eq!(12, rules.get_points().0);

        //  Battre à vrai une dame située dans la table des grands jans : 2 + 2 = 4
        let mut rules = PointsRules::default();
        rules.update_positions([
            2, 0, 0, -1, 2, 0, -1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ]);
        rules.set_dice(Dice { values: (2, 4) });
        assert_eq!(4, rules.get_points().0);
        //  Battre à vrai une dame située dans la table des grands jans : 2
        let mut rules = PointsRules::default();
        rules.update_positions([
            2, 0, -2, -1, 0, 0, -1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ]);
        rules.set_dice(Dice { values: (2, 4) });
        assert_eq!((2, 2), rules.get_points());

        //  Battre à vrai le coin adverse par doublet : 6
        rules.update_positions([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ]);
        rules.set_dice(Dice { values: (2, 2) });
        assert_eq!(6, rules.get_points().0);

        //  Cas de battage du coin de repos adverse impossible
        rules.update_positions([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ]);
        rules.set_dice(Dice { values: (1, 1) });
        assert_eq!(0, rules.get_points().0);

        // ---- Jan de remplissage
        // Faire un petit jan : 4
        rules.update_positions([
            3, 1, 2, 2, 3, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ]);
        rules.set_dice(Dice { values: (2, 1) });
        assert_eq!(1, rules.get_jans(&rules.board).len());
        assert_eq!(4, rules.get_points().0);

        // Faire un petit jan avec un doublet : 6
        rules.update_positions([
            2, 3, 1, 2, 2, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ]);
        rules.set_dice(Dice { values: (1, 1) });
        assert_eq!(6, rules.get_points().0);

        // Faire un petit jan avec 2 moyens : 6 + 6 = 12
        rules.update_positions([
            3, 3, 1, 2, 2, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ]);
        rules.set_dice(Dice { values: (1, 1) });
        assert_eq!(12, rules.get_points().0);

        // Conserver un jan avec un doublet : 6
        rules.update_positions([
            3, 3, 2, 2, 2, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ]);
        rules.set_dice(Dice { values: (1, 1) });
        assert_eq!(6, rules.get_points().0);

        // ----  Sorties
        // Sortir toutes ses dames avant l'adversaire (simple)
        rules.update_positions([
            0, 0, -2, 0, 0, -1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1,
        ]);
        rules.set_dice(Dice { values: (3, 1) });
        assert_eq!(4, rules.get_points().0);

        // Sortir toutes ses dames avant l'adversaire (doublet)
        rules.update_positions([
            0, 0, -2, 0, 0, -1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0,
        ]);
        rules.set_dice(Dice { values: (2, 2) });
        assert_eq!(6, rules.get_points().0);

        // ---- JANS  RARES
        // Jan de six tables
        rules.update_positions([
            10, 1, 0, 0, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, -1, 0,
        ]);
        rules.set_dice(Dice { values: (2, 3) });
        assert_eq!(4, rules.get_points().0);
        rules.update_positions([
            10, 1, 0, 0, 1, 2, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, -1, 0,
        ]);
        rules.set_dice(Dice { values: (2, 3) });
        assert_eq!(0, rules.get_points().0);
        rules.update_positions([
            10, 1, 0, 0, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, -1, 0,
        ]);
        rules.set_dice(Dice { values: (2, 3) });
        assert_eq!(0, rules.get_points().0);

        // Jan de deux tables
        rules.update_positions([
            13, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, -1, 0,
        ]);
        rules.set_dice(Dice { values: (2, 2) });
        assert_eq!(6, rules.get_points().0);
        rules.update_positions([
            12, 0, 0, 1, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, -1, 0,
        ]);
        rules.set_dice(Dice { values: (2, 2) });
        assert_eq!(0, rules.get_points().0);

        // Contre jan de deux tables
        rules.update_positions([
            13, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, -2, 0, 0, 0, 0, 0, 0, 0, 0, 0, -1, 0,
        ]);
        rules.set_dice(Dice { values: (2, 2) });
        assert_eq!((0, 6), rules.get_points());

        // Jan de mézéas
        rules.update_positions([
            13, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, -1, 0,
        ]);
        rules.set_dice(Dice { values: (1, 1) });
        assert_eq!(6, rules.get_points().0);
        rules.update_positions([
            13, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, -1, 0,
        ]);
        rules.set_dice(Dice { values: (1, 2) });
        assert_eq!(4, rules.get_points().0);

        // Contre jan de mézéas
        rules.update_positions([
            13, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, -2, 0, 0, 0, 0, 0, 0, 0, 0, 0, -1, 0,
        ]);
        rules.set_dice(Dice { values: (1, 1) });
        assert_eq!((0, 6), rules.get_points());

        // ---- JANS QUI NE PEUT
        //  Battre à faux une dame située dans la table des petits jans
        let mut rules = PointsRules::default();
        rules.update_positions([
            2, 0, -2, -2, 0, -1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ]);
        rules.set_dice(Dice { values: (2, 3) });
        assert_eq!((0, 4), rules.get_points());

        //  Battre à faux une dame située dans la table des grands jans
        let mut rules = PointsRules::default();
        rules.update_positions([
            2, 0, -2, -1, -2, 0, -1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ]);
        rules.set_dice(Dice { values: (2, 4) });
        assert_eq!((0, 2), rules.get_points());

        // Pour chaque dé non jouable (dame impuissante)
        let mut rules = PointsRules::default();
        rules.update_positions([
            2, 0, -2, -2, -2, 0, -2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ]);
        rules.set_dice(Dice { values: (2, 4) });
        assert_eq!((0, 4), rules.get_points());
    }
}
