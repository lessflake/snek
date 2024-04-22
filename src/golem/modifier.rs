#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Modifier {
    Multiplicative(f64),
    Additive(f64),
}

pub use Modifier::*;

pub enum Fixed {
    HuntersTactics,
    LoudWhistle,
    OppressiveSuperiority,
    FuriousStrength,
    TwiceAsVicious,
    SicEm,
    FrostSpirit,
    Force,
    Impact,
    Scholar,
}

impl Fixed {
    pub const fn val(&self) -> Modifier {
        use Fixed::*;

        match self {
            HuntersTactics => Multiplicative(10.),
            LoudWhistle => Multiplicative(10.),
            OppressiveSuperiority => Multiplicative(10.),
            FuriousStrength => Additive(7.),
            TwiceAsVicious => Additive(10.),
            SicEm => Multiplicative(40.),
            FrostSpirit => Additive(5.),
            Force => Additive(5.),
            Impact => Additive(3.),
            Scholar => Multiplicative(5.),
        }
    }
}

pub const fn vuln(stacks: u32) -> Modifier {
    Multiplicative(stacks as f64)
}

pub fn crit_dmg(fero: u32) -> Modifier {
    Multiplicative(50. + fero as f64 / 15.)
}

pub fn sum(mods: &[Modifier]) -> f64 {
    use Modifier::*;
    let mut add_sum = 1.;
    let mut mult_sum = 1.;
    for m in mods {
        match *m {
            Additive(x) => add_sum += x / 100.,
            Multiplicative(x) => mult_sum *= 1. + x / 100.,
        }
    }
    add_sum * mult_sum
}
