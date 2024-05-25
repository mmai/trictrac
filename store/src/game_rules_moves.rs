//! # Play a TricTrac Game
use crate::board::{Board, CheckerMove, Field, EMPTY_MOVE};
use crate::dice::Dice;
use crate::game::GameState;
use crate::player::Color;
use std::cmp;

#[derive(std::cmp::PartialEq, Debug)]
pub enum MoveError {
    // 2 checkers must go at the same time on an empty corner
    // & the last 2 checkers of a corner must leave at the same time
    CornerNeedsTwoCheckers,
    // Prise de coin de repos par puissance alors qu'il est possible
    // de le prendre directement (par "effet")
    CornerByEffectPossible,
    // toutes les dames doivent être dans le jan de retour
    ExitNeedsAllCheckersOnLastQuarter,
    // mouvement avec nombre en exédant alors qu'une séquence de mouvements
    // sans nombre en excédant est possible
    ExitByEffectPossible,
    // Sortie avec nombre en excédant d'une dame qui n'est pas la plus éloignée
    ExitNotFasthest,
    // Jeu dans un cadran que l'adversaire peut encore remplir
    OpponentCanFillQuarter,
    // remplir cadran si possible & conserver cadran rempli si possible ----
    MustFillQuarter,
    // On n'a pas le droit de jouer d'une manière qui empêche de jouer les deux dés si on a la possibilité de les jouer.
    MustPlayAllDice,
    // Si on ne peut jouer qu'un seul dé, on doit jouer le plus fort si possible.
    MustPlayStrongerDie,
}

/// MoveRules always consider that the current player is White
/// You must use 'mirror' functions on board & CheckerMoves if player is Black
#[derive(Default)]
pub struct MoveRules {
    pub board: Board,
    pub dice: Dice,
}

impl MoveRules {
    /// Revert board if color is black
    pub fn new(color: &Color, board: &Board, dice: Dice) -> Self {
        let board = if *color == Color::Black {
            board.mirror()
        } else {
            board.clone()
        };
        Self { board, dice }
    }

    pub fn moves_follow_rules(&self, moves: &(CheckerMove, CheckerMove)) -> bool {
        // Check moves possibles on the board
        // Check moves conforms to the dice
        // Check move is allowed by the rules (to desactivate when playing with schools)
        self.moves_possible(moves)
            && self.moves_follows_dices(moves)
            && self.moves_allowed(moves).is_ok()
    }

    /// ---- moves_possibles : First of three checks for moves
    fn moves_possible(&self, moves: &(CheckerMove, CheckerMove)) -> bool {
        let color = &Color::White;
        // Check move is physically possible
        if !self.board.move_possible(color, &moves.0) {
            return false;
        }

        // Chained_move : "Tout d'une"
        if let Ok(chained_move) = moves.0.chain(moves.1) {
            if !self.board.move_possible(color, &chained_move) {
                return false;
            }
        } else if !self.board.move_possible(color, &moves.1) {
            return false;
        }
        true
    }

    /// ----- moves_follows_dices : Second of three checks for moves
    fn moves_follows_dices(&self, moves: &(CheckerMove, CheckerMove)) -> bool {
        // Prise de coin par puissance
        if self.is_move_by_puissance(moves) {
            return true;
        }

        let (dice1, dice2) = self.dice.values;
        let (move1, move2): &(CheckerMove, CheckerMove) = &moves;

        let move1_dices = self.get_move_compatible_dices(move1);
        if move1_dices.is_empty() {
            return false;
        }
        let move2_dices = self.get_move_compatible_dices(move2);
        if move2_dices.is_empty() {
            return false;
        }
        if move1_dices.len() == 1
            && move2_dices.len() == 1
            && move1_dices[0] == move2_dices[0]
            && dice1 != dice2
        {
            return false;
        }

        // no rule was broken
        true
    }

    fn get_move_compatible_dices(&self, cmove: &CheckerMove) -> Vec<u8> {
        let (dice1, dice2) = self.dice.values;

        let mut move_dices = Vec::new();
        if cmove.get_to() == 0 {
            // handle empty move (0, 0) only one checker left, exiting with the first die.
            if cmove.get_from() == 0 {
                move_dices.push(dice1);
                move_dices.push(dice2);
                return move_dices;
            }

            // Exits
            let min_dist = 25 - cmove.get_from();
            if dice1 as usize >= min_dist {
                move_dices.push(dice1);
            }
            if dice2 as usize >= min_dist {
                move_dices.push(dice2);
            }
        } else {
            let dist = (cmove.get_to() as i8 - cmove.get_from() as i8).unsigned_abs();
            if dice1 == dist {
                move_dices.push(dice1);
            }
            if dice2 == dist {
                move_dices.push(dice2);
            }
        }
        move_dices
    }

    /// ---- moves_allowed : Third of three checks for moves
    fn moves_allowed(&self, moves: &(CheckerMove, CheckerMove)) -> Result<(), MoveError> {
        self.check_corner_rules(&moves)?;

        if self.is_move_by_puissance(moves) {
            if self.can_take_corner_by_effect() {
                return Err(MoveError::CornerByEffectPossible);
            } else {
                // subsequent rules cannot be broken whith a move by puissance
                return Ok(());
            }
        }

        // Si possible, les deux dés doivent être joués
        if moves.0.get_from() == 0 || moves.1.get_from() == 0 {
            let mut possible_moves_sequences = self.get_possible_moves_sequences(true);
            println!("{:?}", possible_moves_sequences);
            possible_moves_sequences.retain(|moves| self.check_exit_rules(moves).is_ok());
            // possible_moves_sequences.retain(|moves| self.check_corner_rules(moves).is_ok());
            if !possible_moves_sequences.contains(&moves) && !possible_moves_sequences.is_empty() {
                if *moves == (EMPTY_MOVE, EMPTY_MOVE) {
                    return Err(MoveError::MustPlayAllDice);
                }
                let empty_removed = possible_moves_sequences
                    .iter()
                    .filter(|(c1, c2)| *c1 != EMPTY_MOVE && *c2 != EMPTY_MOVE);
                if empty_removed.count() > 0 {
                    return Err(MoveError::MustPlayAllDice);
                }
                return Err(MoveError::MustPlayStrongerDie);
            }
        }

        // check exit rules
        self.check_exit_rules(moves)?;

        // --- interdit de jouer dans cadran que l'adversaire peut encore remplir ----
        let farthest = cmp::max(moves.0.get_to(), moves.1.get_to());
        let in_opponent_side = farthest > 12;
        if in_opponent_side && self.board.is_quarter_fillable(Color::Black, farthest) {
            return Err(MoveError::OpponentCanFillQuarter);
        }

        // --- remplir cadran si possible & conserver cadran rempli si possible ----
        let filling_moves_sequences = self.get_quarter_filling_moves_sequences();
        if !filling_moves_sequences.contains(moves) && !filling_moves_sequences.is_empty() {
            return Err(MoveError::MustFillQuarter);
        }
        // no rule was broken
        Ok(())
    }

    fn check_corner_rules(&self, moves: &(CheckerMove, CheckerMove)) -> Result<(), MoveError> {
        let corner_field: Field = self.board.get_color_corner(&Color::White);
        let (corner_count, _color) = self.board.get_field_checkers(corner_field).unwrap();
        let (from0, to0, from1, to1) = (
            moves.0.get_from(),
            moves.0.get_to(),
            moves.1.get_from(),
            moves.1.get_to(),
        );
        // 2 checkers must go at the same time on an empty corner
        if (to0 == corner_field || to1 == corner_field) && (to0 != to1) && corner_count == 0 {
            return Err(MoveError::CornerNeedsTwoCheckers);
        }

        // the last 2 checkers of a corner must leave at the same time
        if (from0 == corner_field || from1 == corner_field) && (from0 != from1) && corner_count == 2
        {
            return Err(MoveError::CornerNeedsTwoCheckers);
        }
        Ok(())
    }

    fn has_checkers_outside_last_quarter(&self) -> bool {
        !self
            .board
            .get_color_fields(Color::White)
            .iter()
            .filter(|(field, _count)| *field < 19)
            .collect::<Vec<&(usize, i8)>>()
            .is_empty()
    }

    fn check_exit_rules(&self, moves: &(CheckerMove, CheckerMove)) -> Result<(), MoveError> {
        if !moves.0.is_exit() && !moves.1.is_exit() {
            return Ok(());
        }
        // toutes les dames doivent être dans le jan de retour
        if self.has_checkers_outside_last_quarter() {
            return Err(MoveError::ExitNeedsAllCheckersOnLastQuarter);
        }

        // toutes les sorties directes sont autorisées, ainsi que les nombres défaillants
        let possible_moves_sequences = self.get_possible_moves_sequences(false);
        if !possible_moves_sequences.contains(moves) {
            // À ce stade au moins un des déplacements concerne un nombre en excédant
            // - si d'autres séquences de mouvements sans nombre en excédant étaient possibles, on
            // refuse cette séquence
            if !possible_moves_sequences.is_empty() {
                return Err(MoveError::ExitByEffectPossible);
            }

            // - la dame choisie doit être la plus éloignée de la sortie
            let mut checkers = self.board.get_color_fields(Color::White);
            checkers.sort_by(|a, b| b.0.cmp(&a.0));
            let mut farthest = 24;
            let mut next_farthest = 24;
            let mut has_two_checkers = false;
            if let Some((field, count)) = checkers.first() {
                farthest = *field;
                if *count > 1 {
                    next_farthest = *field;
                    has_two_checkers = true;
                } else if let Some((field, _count)) = checkers.get(1) {
                    next_farthest = *field;
                    has_two_checkers = true;
                }
            }

            // s'il reste au moins deux dames, on vérifie que les plus éloignées soint choisies
            if has_two_checkers {
                if moves.0.get_to() == 0 && moves.1.get_to() == 0 {
                    // Deux coups sortants en excédant
                    if cmp::max(moves.0.get_from(), moves.1.get_from()) > next_farthest {
                        return Err(MoveError::ExitNotFasthest);
                    }
                } else {
                    // Un seul coup sortant en excédant le coup sortant doit concerner la plus éloignée du bord
                    let exit_move_field = if moves.0.get_to() == 0 {
                        moves.0.get_from()
                    } else {
                        moves.1.get_from()
                    };
                    if exit_move_field != farthest {
                        return Err(MoveError::ExitNotFasthest);
                    }
                }
            }
        }
        Ok(())
    }

    pub fn get_possible_moves_sequences(
        &self,
        with_excedents: bool,
    ) -> Vec<(CheckerMove, CheckerMove)> {
        let (dice1, dice2) = self.dice.values;
        let (dice_max, dice_min) = if dice1 > dice2 {
            (dice1, dice2)
        } else {
            (dice2, dice1)
        };
        let mut moves_seqs =
            self.get_possible_moves_sequences_by_dices(dice_max, dice_min, with_excedents, false);
        // if we got valid sequences whith the highest die, we don't accept sequences using only the
        // lowest die
        let ignore_empty = !moves_seqs.is_empty();
        let mut moves_seqs_order2 = self.get_possible_moves_sequences_by_dices(
            dice_min,
            dice_max,
            with_excedents,
            ignore_empty,
        );
        moves_seqs.append(&mut moves_seqs_order2);
        let empty_removed = moves_seqs
            .iter()
            .filter(|(c1, c2)| *c1 != EMPTY_MOVE && *c2 != EMPTY_MOVE);
        if empty_removed.count() > 0 {
            moves_seqs.retain(|(c1, c2)| *c1 != EMPTY_MOVE && *c2 != EMPTY_MOVE);
        }
        moves_seqs
    }

    pub fn get_quarter_filling_moves_sequences(&self) -> Vec<(CheckerMove, CheckerMove)> {
        let mut moves_seqs = Vec::new();
        let color = &Color::White;
        for moves in self.get_possible_moves_sequences(true) {
            let mut board = self.board.clone();
            board.move_checker(color, moves.0).unwrap();
            board.move_checker(color, moves.1).unwrap();
            if board.any_quarter_filled(*color) {
                moves_seqs.push(moves);
            }
        }
        moves_seqs
    }

    fn get_possible_moves_sequences_by_dices(
        &self,
        dice1: u8,
        dice2: u8,
        with_excedents: bool,
        ignore_empty: bool,
    ) -> Vec<(CheckerMove, CheckerMove)> {
        let mut moves_seqs = Vec::new();
        let color = &Color::White;
        let forbid_exits = self.has_checkers_outside_last_quarter();
        for first_move in
            self.board
                .get_possible_moves(*color, dice1, with_excedents, false, forbid_exits)
        {
            let mut board2 = self.board.clone();
            if board2.move_checker(color, first_move).is_err() {
                println!("err move");
                continue;
            }

            let mut has_second_dice_move = false;
            for second_move in
                board2.get_possible_moves(*color, dice2, with_excedents, true, forbid_exits)
            {
                if self.check_corner_rules(&(first_move, second_move)).is_ok() {
                    moves_seqs.push((first_move, second_move));
                    has_second_dice_move = true;
                }
            }
            if !has_second_dice_move
                && with_excedents
                && !ignore_empty
                && self.check_corner_rules(&(first_move, EMPTY_MOVE)).is_ok()
            {
                // empty move
                moves_seqs.push((first_move, EMPTY_MOVE));
            }
            //if board2.get_color_fields(*color).is_empty() {
        }
        moves_seqs
    }

    fn get_direct_exit_moves(&self, state: &GameState) -> Vec<CheckerMove> {
        let mut moves = Vec::new();
        let (dice1, dice2) = state.dice.values;

        // sorties directes simples
        let (field1_candidate, field2_candidate) = (25 - dice1 as usize, 25 - dice2 as usize);
        let (count1, col1) = state.board.get_field_checkers(field1_candidate).unwrap();
        let (count2, col2) = state.board.get_field_checkers(field2_candidate).unwrap();
        if count1 > 0 {
            moves.push(CheckerMove::new(field1_candidate, 0).unwrap());
        }
        if dice2 != dice1 {
            if count2 > 0 {
                moves.push(CheckerMove::new(field2_candidate, 0).unwrap());
            }
        } else if count1 > 1 {
            // doublet et deux dames disponibles
            moves.push(CheckerMove::new(field1_candidate, 0).unwrap());
        }

        // sortie directe tout d'une
        let fieldall_candidate = (25 - dice1 - dice2) as usize;
        let (countall, _col) = state.board.get_field_checkers(fieldall_candidate).unwrap();
        let color = &Color::White;
        if countall > 0 {
            if col1.is_none() || col1 == Some(color) {
                moves.push(CheckerMove::new(fieldall_candidate, field1_candidate).unwrap());
                moves.push(CheckerMove::new(field1_candidate, 0).unwrap());
            }
            if col2.is_none() || col2 == Some(color) {
                moves.push(CheckerMove::new(fieldall_candidate, field2_candidate).unwrap());
                moves.push(CheckerMove::new(field2_candidate, 0).unwrap());
            }
        }
        moves
    }

    fn is_move_by_puissance(&self, moves: &(CheckerMove, CheckerMove)) -> bool {
        let (dice1, dice2) = self.dice.values;
        let dist1 = (moves.0.get_to() as i8 - moves.0.get_from() as i8).unsigned_abs();
        let dist2 = (moves.1.get_to() as i8 - moves.1.get_from() as i8).unsigned_abs();

        // Both corners must be empty
        let (count1, _color) = self.board.get_field_checkers(12).unwrap();
        let (count2, _color2) = self.board.get_field_checkers(13).unwrap();
        if count1 > 0 || count2 > 0 {
            return false;
        }

        let color = &Color::White;
        moves.0.get_to() == moves.1.get_to()
            && moves.0.get_to() == self.board.get_color_corner(color)
            && (cmp::min(dist1, dist2) == cmp::min(dice1, dice2) - 1
                && cmp::max(dist1, dist2) == cmp::max(dice1, dice2) - 1)
    }

    fn can_take_corner_by_effect(&self) -> bool {
        // return false if corner already taken
        let color = &Color::White;
        let corner_field: Field = self.board.get_color_corner(color);
        let (count, _col) = self.board.get_field_checkers(corner_field).unwrap();
        if count > 0 {
            return false;
        }

        let (dice1, dice2) = self.dice.values;
        let (field1, field2) = (corner_field - dice1 as usize, corner_field - dice2 as usize);
        let res1 = self.board.get_field_checkers(field1);
        let res2 = self.board.get_field_checkers(field2);
        if res1.is_err() || res2.is_err() {
            return false;
        }
        let (count1, opt_color1) = res1.unwrap();
        let (count2, opt_color2) = res2.unwrap();
        count1 > 0 && count2 > 0 && opt_color1 == Some(color) && opt_color2 == Some(color)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_take_corner_by_effect() {
        let mut rules = MoveRules::default();
        rules.board.set_positions([
            10, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, -15,
        ]);
        rules.dice.values = (4, 4);
        assert!(rules.can_take_corner_by_effect());

        rules.dice.values = (5, 5);
        assert!(!rules.can_take_corner_by_effect());

        rules.board.set_positions([
            10, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, -15,
        ]);
        rules.dice.values = (4, 4);
        assert!(!rules.can_take_corner_by_effect());
    }

    #[test]
    fn prise_en_puissance() {
        let mut state = MoveRules::default();
        // prise par puissance ok
        state.board.set_positions([
            10, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, -15,
        ]);
        state.dice.values = (5, 5);
        let moves = (
            CheckerMove::new(8, 12).unwrap(),
            CheckerMove::new(8, 12).unwrap(),
        );
        assert!(state.is_move_by_puissance(&moves));
        assert!(state.moves_follows_dices(&moves));
        assert!(state.moves_allowed(&moves).is_ok());

        // opponent corner must be empty
        state.board.set_positions([
            10, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, -2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, -13,
        ]);
        assert!(!state.is_move_by_puissance(&moves));
        assert!(!state.moves_follows_dices(&moves));

        // Si on a la possibilité de prendre son coin à la fois par effet, c'est à dire naturellement, et aussi par puissance, on doit le prendre par effet
        state.board.set_positions([
            5, 0, 0, 0, 0, 0, 5, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, -15,
        ]);
        assert_eq!(
            Err(MoveError::CornerByEffectPossible),
            state.moves_allowed(&moves)
        );

        // on a déjà pris son coin : on ne peux plus y deplacer des dames par puissance
        state.board.set_positions([
            8, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, -15,
        ]);
        assert!(!state.is_move_by_puissance(&moves));
        assert!(!state.moves_follows_dices(&moves));
    }

    #[test]
    fn exit() {
        let mut state = MoveRules::default();
        // exit ok
        state.board.set_positions([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0,
        ]);
        state.dice.values = (5, 5);
        let moves = (
            CheckerMove::new(20, 0).unwrap(),
            CheckerMove::new(20, 0).unwrap(),
        );
        assert!(state.moves_follows_dices(&moves));
        assert!(state.moves_allowed(&moves).is_ok());

        // toutes les dames doivent être dans le jan de retour
        state.board.set_positions([
            0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0,
        ]);
        state.dice.values = (5, 5);
        let moves = (
            CheckerMove::new(20, 0).unwrap(),
            CheckerMove::new(20, 0).unwrap(),
        );
        assert_eq!(
            Err(MoveError::ExitNeedsAllCheckersOnLastQuarter),
            state.moves_allowed(&moves)
        );

        // on ne peut pas sortir une dame avec un nombre excédant si on peut en jouer une avec un nombre défaillant
        state.board.set_positions([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 3, 0, 0, 2, 0,
        ]);
        state.dice.values = (5, 5);
        let moves = (
            CheckerMove::new(20, 0).unwrap(),
            CheckerMove::new(23, 0).unwrap(),
        );
        assert_eq!(
            Err(MoveError::ExitByEffectPossible),
            state.moves_allowed(&moves)
        );

        // on doit jouer le nombre excédant le plus éloigné
        state.board.set_positions([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 1, 0,
        ]);
        state.dice.values = (5, 5);
        let moves = (
            CheckerMove::new(20, 0).unwrap(),
            CheckerMove::new(23, 0).unwrap(),
        );
        assert_eq!(Err(MoveError::ExitNotFasthest), state.moves_allowed(&moves));
        let moves = (
            CheckerMove::new(20, 0).unwrap(),
            CheckerMove::new(21, 0).unwrap(),
        );
        assert!(state.moves_allowed(&moves).is_ok());

        // Cas de la dernière dame
        state.board.set_positions([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0,
        ]);
        state.dice.values = (5, 5);
        let moves = (
            CheckerMove::new(23, 0).unwrap(),
            CheckerMove::new(0, 0).unwrap(),
        );
        assert!(state.moves_follows_dices(&moves));
        assert!(state.moves_allowed(&moves).is_ok());
    }

    #[test]
    fn move_check_opponent_fillable_quarter() {
        let mut state = MoveRules::default();
        state.board.set_positions([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 1, 0,
        ]);
        state.dice.values = (5, 5);
        let moves = (
            CheckerMove::new(11, 16).unwrap(),
            CheckerMove::new(11, 16).unwrap(),
        );
        assert!(state.moves_allowed(&moves).is_ok());

        state.board.set_positions([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, -12, 0, 0, 0, 0, 1, 0,
        ]);
        state.dice.values = (5, 5);
        let moves = (
            CheckerMove::new(11, 16).unwrap(),
            CheckerMove::new(11, 16).unwrap(),
        );
        assert_eq!(
            Err(MoveError::OpponentCanFillQuarter),
            state.moves_allowed(&moves)
        );
    }

    #[test]
    fn move_check_fillable_quarter() {
        let mut state = MoveRules::default();
        state.board.set_positions([
            3, 3, 2, 2, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 1, 0,
        ]);
        state.dice.values = (5, 4);
        let moves = (
            CheckerMove::new(1, 6).unwrap(),
            CheckerMove::new(2, 6).unwrap(),
        );
        assert!(state.moves_allowed(&moves).is_ok());
        let moves = (
            CheckerMove::new(1, 5).unwrap(),
            CheckerMove::new(2, 7).unwrap(),
        );
        assert_eq!(Err(MoveError::MustFillQuarter), state.moves_allowed(&moves));

        state.board.set_positions([
            2, 3, 2, 2, 3, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ]);
        state.dice.values = (2, 3);
        let moves = (
            CheckerMove::new(6, 8).unwrap(),
            CheckerMove::new(6, 9).unwrap(),
        );
        assert_eq!(Err(MoveError::MustFillQuarter), state.moves_allowed(&moves));
        let moves = (
            CheckerMove::new(2, 4).unwrap(),
            CheckerMove::new(5, 8).unwrap(),
        );
        assert!(state.moves_allowed(&moves).is_ok());
    }

    #[test]
    fn move_play_all_dice() {
        let mut state = MoveRules::default();
        state.board.set_positions([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0,
        ]);
        state.dice.values = (1, 3);
        let moves = (
            CheckerMove::new(22, 0).unwrap(),
            CheckerMove::new(0, 0).unwrap(),
        );

        assert_eq!(Err(MoveError::MustPlayAllDice), state.moves_allowed(&moves));
        let moves = (
            CheckerMove::new(22, 23).unwrap(),
            CheckerMove::new(23, 0).unwrap(),
        );
        assert!(state.moves_allowed(&moves).is_ok());
    }

    #[test]
    fn move_rest_corner_enter() {
        // direct
        let mut state = MoveRules::default();
        state.board.set_positions([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ]);
        state.dice.values = (2, 1);
        let moves = (
            CheckerMove::new(10, 12).unwrap(),
            CheckerMove::new(11, 12).unwrap(),
        );
        assert!(state.moves_follows_dices(&moves));
        assert!(state.moves_allowed(&moves).is_ok());

        // par puissance
        state.dice.values = (3, 2);
        let moves = (
            CheckerMove::new(10, 12).unwrap(),
            CheckerMove::new(11, 12).unwrap(),
        );
        assert!(state.moves_follows_dices(&moves));
        assert!(state.moves_allowed(&moves).is_ok());
    }

    #[test]
    fn move_rest_corner_blocked() {
        let mut state = MoveRules::default();
        state.board.set_positions([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, -1, -1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ]);
        state.dice.values = (2, 1);
        let moves = (
            CheckerMove::new(0, 0).unwrap(),
            CheckerMove::new(0, 0).unwrap(),
        );
        assert!(state.moves_follows_dices(&moves));
        assert!(state.moves_allowed(&moves).is_ok());

        state.board.set_positions([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, -1, -1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0,
        ]);
        state.dice.values = (2, 1);
        let moves = (
            CheckerMove::new(23, 24).unwrap(),
            CheckerMove::new(0, 0).unwrap(),
        );
        assert!(state.moves_follows_dices(&moves));
        // let res = state.moves_allowed(&moves);
        // println!("{:?}", res);
        assert!(state.moves_allowed(&moves).is_ok());

        let moves = (
            CheckerMove::new(0, 0).unwrap(),
            CheckerMove::new(0, 0).unwrap(),
        );
        assert_eq!(Err(MoveError::MustPlayAllDice), state.moves_allowed(&moves));
    }

    #[test]
    fn move_rest_corner_exit() {
        let mut state = MoveRules::default();
        state.board.set_positions([
            1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, -1, -1, 0, 0, 0, 0, 0, 0,
        ]);
        state.dice.values = (2, 3);
        let moves = (
            CheckerMove::new(12, 14).unwrap(),
            CheckerMove::new(1, 4).unwrap(),
        );
        assert_eq!(
            Err(MoveError::CornerNeedsTwoCheckers),
            state.moves_allowed(&moves)
        );
    }

    #[test]
    fn move_play_stronger_dice() {
        let mut state = MoveRules::default();
        state.board.set_positions([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, -1, -1, -1, 0, 0, 0, 0, 0, 0,
        ]);
        state.dice.values = (2, 3);
        let moves = (
            CheckerMove::new(12, 14).unwrap(),
            CheckerMove::new(0, 0).unwrap(),
        );
        // let poss = state.get_possible_moves_sequences(&Color::White, true);
        // println!("{:?}", poss);
        assert_eq!(
            Err(MoveError::MustPlayStrongerDie),
            state.moves_allowed(&moves)
        );
        let moves = (
            CheckerMove::new(12, 15).unwrap(),
            CheckerMove::new(0, 0).unwrap(),
        );
        assert!(state.moves_allowed(&moves).is_ok());
    }

    #[test]
    fn moves_possible() {
        let mut state = MoveRules::default();

        // Chained moves
        let moves = (
            CheckerMove::new(1, 5).unwrap(),
            CheckerMove::new(5, 9).unwrap(),
        );
        assert!(state.moves_possible(&moves));

        // not chained moves
        let moves = (
            CheckerMove::new(1, 5).unwrap(),
            CheckerMove::new(6, 9).unwrap(),
        );
        assert!(!state.moves_possible(&moves));

        // black moves
        let state = MoveRules::new(&Color::Black, &Board::default(), Dice::default());
        let moves = (
            CheckerMove::new(24, 20).unwrap().mirror(),
            CheckerMove::new(20, 19).unwrap().mirror(),
        );
        assert!(state.moves_possible(&moves));
    }
}
