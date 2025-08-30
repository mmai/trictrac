use crate::{BotStrategy, CheckerMove, Color, GameState, PlayerId};
use store::MoveRules;

#[derive(Debug)]
pub struct DefaultStrategy {
    pub game: GameState,
    pub player_id: PlayerId,
    pub color: Color,
}

impl Default for DefaultStrategy {
    fn default() -> Self {
        let game = GameState::default();
        Self {
            game,
            player_id: 1,
            color: Color::White,
        }
    }
}

impl BotStrategy for DefaultStrategy {
    fn get_game(&self) -> &GameState {
        &self.game
    }
    fn get_mut_game(&mut self) -> &mut GameState {
        &mut self.game
    }

    fn set_color(&mut self, color: Color) {
        self.color = color;
    }

    fn set_player_id(&mut self, player_id: PlayerId) {
        self.player_id = player_id;
    }

    fn calculate_points(&self) -> u8 {
        // let dice_roll_count = self
        //     .get_game()
        //     .players
        //     .get(&self.player_id)
        //     .unwrap()
        //     .dice_roll_count;
        // let points_rules = PointsRules::new(&Color::White, &self.game.board, self.game.dice);
        // points_rules.get_points(dice_roll_count).0
        self.game.dice_points.0
    }

    fn calculate_adv_points(&self) -> u8 {
        // self.calculate_points()
        self.game.dice_points.1
    }

    fn choose_go(&self) -> bool {
        true
    }

    fn choose_move(&self) -> (CheckerMove, CheckerMove) {
        let rules = MoveRules::new(&self.color, &self.game.board, self.game.dice);
        let possible_moves = rules.get_possible_moves_sequences(true, vec![]);
        let choosen_move = *possible_moves
            .first()
            .unwrap_or(&(CheckerMove::default(), CheckerMove::default()));
        if self.color == Color::White {
            choosen_move
        } else {
            (choosen_move.0.mirror(), choosen_move.1.mirror())
        }

        // let (dice1, dice2) = match self.color {
        //     Color::White => (self.game.dice.values.0 as i8, self.game.dice.values.1 as i8),
        //     Color::Black => (
        //         0 - self.game.dice.values.0 as i8,
        //         0 - self.game.dice.values.1 as i8,
        //     ),
        // };
        //
        // let fields = self.game.board.get_color_fields(self.color);
        // let first_field = fields.first().unwrap();
        // (
        //     CheckerMove::new(first_field.0, (first_field.0 as i8 + dice1) as usize).unwrap(),
        //     CheckerMove::new(first_field.0, (first_field.0 as i8 + dice2) as usize).unwrap(),
        // )
    }
}
