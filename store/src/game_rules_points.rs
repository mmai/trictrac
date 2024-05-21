use crate::board::Board;
use crate::dice::Dice;

#[derive(std::cmp::PartialEq, Debug)]
pub enum PointsRule {
    FilledQuarter,
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

pub trait PointsRules {
    fn board(&self) -> &Board;
    fn dice(&self) -> &Dice;

    fn get_points(&self) -> Vec<(u8, PointsRule)> {
        Vec::new()
    }
}
