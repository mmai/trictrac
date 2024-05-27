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
    TrueHit,
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

// #[derive(Debug)]
// struct PossibleJan {
//     pub jan: Jan,
//     pub ways: Vec<(CheckerMove, CheckerMove)>,
// }

#[derive(Default)]
struct PossibleJans(HashMap<Jan, Vec<(CheckerMove, CheckerMove)>>);

impl PossibleJans {
    pub fn push(&mut self, jan: Jan, cmoves: (CheckerMove, CheckerMove)) {
        if let Some(ways) = self.0.get_mut(&jan) {
            if !ways.contains(&cmoves) {
                ways.push(cmoves);
            }
        } else {
            self.0.insert(jan, [cmoves].into());
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

    fn get_jans(&self, board: &Board, dices: &Vec<u8>) -> PossibleJans {
        let mut jans = PossibleJans::default();
        if dices.is_empty() {
            return jans;
        }
        let color = Color::White;
        let mut dices = dices.clone();
        let mut board = board.clone();
        let fields = board.get_color_fields(color);
        if let Some(dice) = dices.pop() {
            for (from, _) in fields {
                let to = if from + dice as usize > 24 {
                    0
                } else {
                    from + dice as usize
                };
                if let Ok(cmove) = CheckerMove::new(from, to) {
                    match board.move_checker(&color, cmove) {
                        Err(Error::FieldBlockedByOne) => {
                            jans.push(Jan::TrueHit, (cmove, EMPTY_MOVE));
                        }
                        Err(_) => {
                            // let next_dice_jan = self.get_jans(&board, &dices);
                            // jans possibles en tout d'une après un battage à vrai :
                            // truehit
                        }
                        Ok(()) => {
                            // TODO : check if it's a jan
                            let next_dice_jan = self.get_jans(&board, &dices);
                            // TODO : merge jans du dé courant et du prochain dé
                        }
                    }
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

    pub fn get_points(&self) -> usize {
        let mut points = 0;

        let jans = self.get_jans(&self.board, &vec![self.dice.values.0, self.dice.values.1]);

        // Jans de remplissage
        let filling_moves_sequences = self.move_rules.get_quarter_filling_moves_sequences();
        points += 4 * filling_moves_sequences.len();
        //  	Points par simple par moyen 	Points par doublet par moyen 	Nombre de moyens possibles 	Bénéficiaire
        // « JAN RARE »
        // Jan de six tables 	4 	n/a 	1 	Joueur
        // Jan de deux tables 	4 	6 	1 	Joueur
        // Jan de mézéas 	4 	6 	1 	Joueur
        // Contre jan de deux tables 	4 	6 	1 	Adversaire
        // Contre jan de mézéas 	4 	6 	1 	Adversaire
        // « JAN DE RÉCOMPENSE »
        // Battre à vrai une dame
        // située dans la table des grands jans 	2 		1, 2 ou 3 	Joueur
        // 	4 	1 ou 2 	Joueur
        // Battre à vrai une dame
        // située dans la table des petits jans 	4 		1, 2 ou 3 	Joueur
        // 	6 	1 ou 2 	Joueur
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
