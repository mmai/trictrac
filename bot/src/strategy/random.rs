use crate::{BotStrategy, CheckerMove, Color, GameState, PlayerId};
use store::MoveRules;

#[derive(Debug)]
pub struct RandomStrategy {
    pub game: GameState,
    pub player_id: PlayerId,
    pub color: Color,
}

impl Default for RandomStrategy {
    fn default() -> Self {
        let game = GameState::default();
        Self {
            game,
            player_id: 1,
            color: Color::White,
        }
    }
}

impl BotStrategy for RandomStrategy {
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
        self.game.dice_points.0
    }

    fn calculate_adv_points(&self) -> u8 {
        self.game.dice_points.1
    }

    fn choose_go(&self) -> bool {
        true
    }

    fn choose_move(&self) -> (CheckerMove, CheckerMove) {
        let rules = MoveRules::new(&self.color, &self.game.board, self.game.dice);
        let possible_moves = rules.get_possible_moves_sequences(true, vec![]);

        use rand::{seq::SliceRandom, thread_rng};
        let mut rng = thread_rng();
        let choosen_move = possible_moves
            .choose(&mut rng)
            .cloned()
            .unwrap_or((CheckerMove::default(), CheckerMove::default()));

        if self.color == Color::White {
            choosen_move
        } else {
            (choosen_move.0.mirror(), choosen_move.1.mirror())
        }
    }
}
