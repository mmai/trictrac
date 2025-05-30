use crate::{BotStrategy, CheckerMove, Color, GameState, PlayerId, PointsRules};

#[derive(Debug)]
pub struct ErroneousStrategy {
    pub game: GameState,
    pub player_id: PlayerId,
    pub color: Color,
}

impl Default for ErroneousStrategy {
    fn default() -> Self {
        let game = GameState::default();
        Self {
            game,
            player_id: 2,
            color: Color::Black,
        }
    }
}

impl BotStrategy for ErroneousStrategy {
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
        let dice_roll_count = self
            .get_game()
            .players
            .get(&self.player_id)
            .unwrap()
            .dice_roll_count;
        let points_rules = PointsRules::new(&Color::White, &self.game.board, self.game.dice);
        points_rules.get_points(dice_roll_count).0
    }

    fn calculate_adv_points(&self) -> u8 {
        self.calculate_points()
    }

    fn choose_go(&self) -> bool {
        true
    }

    fn choose_move(&self) -> (CheckerMove, CheckerMove) {
        (
            CheckerMove::new(1, 10).unwrap(),
            CheckerMove::new(2, 7).unwrap(),
        )
    }
}
