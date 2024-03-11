use crate::Error;
use rand::distributions::{Distribution, Uniform};
use rand::{rngs::StdRng, SeedableRng};
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct DiceRoller {
    rng: StdRng,
}

impl Default for DiceRoller {
    fn default() -> Self {
        Self::new(None)
    }
}

impl DiceRoller {
    pub fn new(opt_seed: Option<u64>) -> Self {
        Self {
            rng: match opt_seed {
                None => StdRng::from_rng(rand::thread_rng()).unwrap(),
                Some(seed) => SeedableRng::seed_from_u64(seed),
            },
        }
    }

    /// Roll the dices which generates two random numbers between 1 and 6, replicating a perfect
    /// dice. We use the operating system's random number generator.
    pub fn roll(&mut self) -> Dice {
        let between = Uniform::new_inclusive(1, 6);

        let v = (between.sample(&mut self.rng), between.sample(&mut self.rng));

        Dice { values: (v.0, v.1) }
    }

    // Heads or tails
    // pub fn coin(self) -> bool {
    //     let between = Uniform::new_inclusive(1, 2);
    //     let mut rng = rand::thread_rng();
    //     between.sample(&mut rng) == 1
    // }
}

/// Represents the two dice
///
/// Trictrac is always played with two dice.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Deserialize, Default)]
pub struct Dice {
    /// The two dice values
    pub values: (u8, u8),
}

impl Dice {
    pub fn to_bits_string(self) -> String {
        format!("{:0>3b}{:0>3b}", self.values.0, self.values.1)
    }

    pub fn to_display_string(self) -> String {
        format!("{} & {}", self.values.0, self.values.1)
    }

    // pub fn to_bits(self) -> [bool;6] {
    //     self.to_bits_string().into_bytes().iter().map(|strbit| *strbit == '1' as u8).collect()
    // }

    // pub from_bits_string(String bits) -> Self {
    //
    //     Dices {
    //         values: ()
    //     }
    // }
}

/// Trait to roll the dices
pub trait Roll {
    /// Roll the dices
    fn roll(&mut self) -> &mut Self;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roll() {
        let dice = DiceRoller::default().roll();
        assert!(dice.values.0 >= 1 && dice.values.0 <= 6);
        assert!(dice.values.1 >= 1 && dice.values.1 <= 6);
    }

    #[test]
    fn test_seed() {
        let dice = DiceRoller::new(Some(123)).roll();
        assert!(dice.values.0 == 3);
        assert!(dice.values.1 == 2);
    }

    #[test]
    fn test_to_bits_string() {
        let dice = Dice { values: (4, 2) };
        assert!(dice.to_bits_string() == "100010");
    }
}
