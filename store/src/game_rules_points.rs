use crate::board::Board;
use crate::dice::Dice;

#[derive(std::cmp::PartialEq, Debug)]
pub enum PointsRule {
    FilledQuarter,
}

pub trait PointsRules {
    fn board(&self) -> &Board;
    fn dice(&self) -> &Dice;

    fn get_points(&self) -> Vec<(u8, PointsRule)> {
        Vec::new()
    }
}
