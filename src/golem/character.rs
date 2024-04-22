use crate::golem::modifier;
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct Character {
    head: Head,
    shoulders: Shoulders,
    chest: Chest,
    hands: Hands,
    leggings: Leggings,
    feet: Feet,

    accessories: (Accessory, Accessory),
    amulet: Amulet,
    rings: (Ring, Ring),
    back: Back,

    current_set: Weapons,
    weapons: (Weaponset, Weaponset),

    traits: Vec<Trait>,
    buffs: HashSet<Buff>,
    boons: Vec<Boon>,

    food: Food,
    utility: Utility,
}

pub enum QueriedWeapons {
    Single(Weapon),
    Double(Weapon, Weapon),
}

impl QueriedWeapons {
    pub const fn mainhand(&self) -> Weapon {
        match self {
            Self::Single(weapon) => *weapon,
            Self::Double(weapon, _) => *weapon,
        }
    }
}

impl Character {
    pub fn get_stats(&self) -> Stats {
        self.stats()
    }

    pub fn has_buff(&self, buff: Buff) -> bool {
        self.buffs.contains(&buff)
    }

    pub fn add_buff(&mut self, buff: Buff) {
        self.buffs.insert(buff);
    }

    pub fn remove_buff(&mut self, buff: Buff) {
        self.buffs.remove(&buff);
    }

    pub fn set_mainhand(&mut self, set: Weapons, weapon: Weapon) {
        match set {
            Weapons::First => match self.weapons.0 {
                Weaponset::Dual(ref mut current, _) => {
                    current.kind = MainhandKind::specific(weapon).unwrap()
                }
                _ => panic!(),
            },
            Weapons::Second => match self.weapons.1 {
                Weaponset::Dual(ref mut current, _) => {
                    current.kind = MainhandKind::specific(weapon).unwrap()
                }
                _ => panic!(),
            },
        }
    }

    pub fn swap_weapons(&mut self) {
        match self.current_set {
            Weapons::First => self.current_set = Weapons::Second,
            Weapons::Second => self.current_set = Weapons::First,
        }
    }

    pub fn current_weapons(&self) -> QueriedWeapons {
        let current_set = match self.current_set {
            Weapons::First => self.weapons.0,
            Weapons::Second => self.weapons.1,
        };

        match current_set {
            Weaponset::Dual(main, off) => {
                QueriedWeapons::Double(main.kind.weapon(), off.kind.weapon())
            }
            Weaponset::Twohand(two) => QueriedWeapons::Single(two.kind.weapon()),
        }
    }

    pub fn current_mainhand(&self) -> Weapon {
        let current_set = match self.current_set {
            Weapons::First => self.weapons.0,
            Weapons::Second => self.weapons.1,
        };

        match current_set {
            Weaponset::Dual(main, _) => main.kind.weapon(),
            Weaponset::Twohand(two) => two.kind.weapon(),
        }
    }

    pub fn alternate_mainhand(&self) -> Weapon {
        let current_set = match self.current_set {
            Weapons::First => self.weapons.1,
            Weapons::Second => self.weapons.0,
        };

        match current_set {
            Weaponset::Dual(main, _) => main.kind.weapon(),
            Weaponset::Twohand(two) => two.kind.weapon(),
        }
    }

    pub const fn sigils(&self, weapons: Weapons) -> (Sigil, Sigil) {
        match weapons {
            Weapons::First => self.weapons.0.sigils(),
            Weapons::Second => self.weapons.1.sigils(),
        }
    }

    pub fn runes(&self) -> Vec<Rune> {
        vec![
            self.head.0.rune,
            self.shoulders.0.rune,
            self.chest.0.rune,
            self.hands.0.rune,
            self.leggings.0.rune,
            self.feet.0.rune,
        ]
    }

    pub fn modifiers(&self) -> Vec<modifier::Modifier> {
        use modifier::Fixed as Mods;

        let mut mods = Vec::new();

        for r#trait in &self.traits {
            let modifier = match r#trait {
                Trait::HuntersTactics => Mods::HuntersTactics.val(),
                Trait::LoudWhistle => Mods::LoudWhistle.val(),
                Trait::OppressiveSuperiority => Mods::OppressiveSuperiority.val(),
                Trait::FuriousStrength => Mods::FuriousStrength.val(),
                _ => continue,
            };
            mods.push(modifier);
        }

        for buff in &self.buffs {
            let modifier = match buff {
                Buff::TwiceAsVicious => Mods::TwiceAsVicious.val(),
                Buff::SicEm => Mods::SicEm.val(),
                Buff::FrostSpirit => Mods::FrostSpirit.val(),
                _ => continue,
            };
            mods.push(modifier);
        }

        let sigils = self.sigils(self.current_set);
        for sigil in &[sigils.0, sigils.1] {
            let modifier = match sigil {
                Sigil::Force => Mods::Force.val(),
                Sigil::Impact => Mods::Impact.val(),
                _ => continue,
            };
            mods.push(modifier);
        }

        let (_, rune_mods) = rune_bonuses(&self.runes());
        for m in rune_mods {
            mods.push(m);
        }

        mods
    }
}

impl HasStats for Character {
    fn stats(&self) -> Stats {
        let base_stats = Stats::new(1000, 1000, 0);

        let (rune_stats, _) = rune_bonuses(&self.runes());
        let items: Vec<&dyn HasStats> = vec![
            &self.head,
            &self.shoulders,
            &self.chest,
            &self.hands,
            &self.leggings,
            &self.feet,
            &self.accessories,
            &self.amulet,
            &self.rings,
            &self.back,
            match self.current_set {
                Weapons::First => &self.weapons.0,
                Weapons::Second => &self.weapons.1,
            },
            &self.food,
            &rune_stats,
        ];

        let mut stats = items.iter().fold(base_stats, |mut acc, x| {
            acc = acc + x.stats();
            acc
        });

        if self.traits.iter().any(|x| *x == Trait::HonedAxes) && self.current_set == Weapons::Second
        {
            stats = stats + Stats::new(0, 0, 120);
        }

        stats = stats + self.utility.effect(&stats);

        for r#trait in &self.traits {
            let increase = match r#trait {
                Trait::ViciousQuarry => Stats::new(0, 0, 250),
                Trait::PackAlpha => Stats::new(150, 150, 0),
                Trait::PetsProwess => Stats::new(0, 0, 300),
                Trait::HonedAxes => Stats::new(0, 0, 240),
                _ => continue,
            };

            stats = stats + increase;
        }

        for boon in &self.boons {
            let increase = match boon {
                Boon::Might => Stats::new(25 * 30, 0, 0),
                _ => continue,
            };

            stats = stats + increase;
        }

        for buff in &self.buffs {
            let increase = match buff {
                Buff::Spotter => Stats::new(0, 100, 0),
                Buff::EmpowerAllies => Stats::new(100, 0, 0),
                Buff::BannerOfStrength => Stats::new(100, 0, 0),
                Buff::BannerOfDiscipline => Stats::new(0, 100, 100),
                Buff::Ferocious => Stats::new(150, 0, 100),
                Buff::SignetOfTheWild => Stats::new(0, 0, 180),
                _ => continue,
            };

            stats = stats + increase;
        }

        stats
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Weapons {
    First,
    Second,
}

pub fn rune_bonuses(runes: &[Rune]) -> (Stats, Vec<modifier::Modifier>) {
    use std::collections::HashMap;
    use Bonus::*;

    let mut buckets = HashMap::new();
    for rune in runes {
        let counter = buckets.entry(rune).or_insert(0);
        *counter += 1;
    }

    let mut total_bonuses = Vec::new();

    for (rune, count) in buckets {
        let mut bonuses = rune.bonuses(count);
        total_bonuses.append(&mut bonuses);
    }

    let mut total_stats = Stats::new(0, 0, 0);
    let mut total_mods = Vec::new();

    for bonus in total_bonuses {
        match bonus {
            Power(n) => total_stats = total_stats + Stats::new(n, 0, 0),
            Precision(n) => total_stats = total_stats + Stats::new(0, n, 0),
            Ferocity(n) => total_stats = total_stats + Stats::new(0, 0, n),
            Multiplier(m) => total_mods.push(m),
            CritChance(_) => {}
        }
    }

    (total_stats, total_mods)
}

pub fn my_char() -> Character {
    let infusion = Infusion::NineAgonyFivePower;

    let rune = Rune::Scholar;
    let head = Head(Armor::new(infusion, rune, Stats::new(63, 45, 45)));
    let shoulders = Shoulders(Armor::new(infusion, rune, Stats::new(47, 34, 34)));
    let chest = Chest(Armor::new(infusion, rune, Stats::new(141, 101, 101)));
    let hands = Hands(Armor::new(infusion, rune, Stats::new(47, 34, 34)));
    let leggings = Leggings(Armor::new(infusion, rune, Stats::new(94, 67, 67)));
    let feet = Feet(Armor::new(infusion, rune, Stats::new(47, 34, 34)));

    let accessories = (
        Accessory {
            infusion,
            stats: Stats::new(110, 74, 74),
        },
        Accessory {
            infusion,
            stats: Stats::new(110, 74, 74),
        },
    );
    let amulet = Amulet {
        stats: Stats::new(157, 108, 108),
    };
    let rings = (
        Ring {
            infusions: (infusion, infusion, infusion),
            stats: Stats::new(126, 85, 85),
        },
        Ring {
            infusions: (infusion, infusion, infusion),
            stats: Stats::new(126, 85, 85),
        },
    );

    let back = Back {
        infusions: (infusion, infusion),
        stats: Stats::new(63, 40, 40),
    };

    let current_set = Weapons::First;

    let weapons = (
        Weaponset::Twohand(Twohand {
            kind: TwohandKind::Longbow,
            infusions: (infusion, infusion),
            sigils: (Sigil::Force, Sigil::Air),
        }),
        Weaponset::Dual(
            Mainhand {
                kind: MainhandKind::Dagger,
                infusion,
                sigil: Sigil::Force,
            },
            Offhand {
                kind: OffhandKind::Axe,
                infusion,
                sigil: Sigil::Hydro,
            },
        ),
    );

    let traits = base_traits();
    let buffs = base_buffs();
    let boons = base_boons();

    let food = Food::SweetAndSpicyButternutSquashSoup;
    let utility = Utility::SuperiorSharpeningStone;

    Character {
        head,
        shoulders,
        chest,
        hands,
        leggings,
        feet,
        accessories,
        amulet,
        rings,
        back,
        current_set,
        weapons,

        traits,
        buffs,
        boons,

        food,
        utility,
    }
}

fn base_traits() -> Vec<Trait> {
    vec![
        Trait::HuntersTactics,
        Trait::ViciousQuarry,
        Trait::PackAlpha,
        Trait::PetsProwess,
        Trait::HonedAxes,
        Trait::LoudWhistle,
        Trait::OppressiveSuperiority,
        Trait::FuriousStrength,
    ]
}

fn base_buffs() -> HashSet<Buff> {
    [
        Buff::Spotter,
        Buff::FrostSpirit,
        Buff::EmpowerAllies,
        Buff::BannerOfStrength,
        Buff::BannerOfDiscipline,
        Buff::Ferocious,
        Buff::SignetOfTheWild,
    ]
    .iter()
    .cloned()
    .collect::<HashSet<_>>()
}

fn base_boons() -> Vec<Boon> {
    vec![Boon::Might, Boon::Fury]
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Stats {
    power: u32,
    precision: u32,
    ferocity: u32,
}

trait HasStats {
    fn stats(&self) -> Stats;
}

impl<T: HasStats> HasStats for (T, T) {
    fn stats(&self) -> Stats {
        self.0.stats() + self.1.stats()
    }
}

impl<T: HasStats> HasStats for (T, T, T) {
    fn stats(&self) -> Stats {
        self.0.stats() + self.1.stats() + self.2.stats()
    }
}

impl Stats {
    // TODO: only pub for now
    pub const fn new(power: u32, precision: u32, ferocity: u32) -> Self {
        Self {
            power,
            precision,
            ferocity,
        }
    }

    pub const fn power(&self) -> u32 {
        self.power
    }
    pub const fn precision(&self) -> u32 {
        self.precision
    }
    pub const fn ferocity(&self) -> u32 {
        self.ferocity
    }
}

impl std::ops::Add for Stats {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            power: self.power + other.power,
            precision: self.precision + other.precision,
            ferocity: self.ferocity + other.ferocity,
        }
    }
}

impl HasStats for Stats {
    fn stats(&self) -> Stats {
        *self
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Sigil {
    Force,
    Impact,
    Air,
    Hydro,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Rune {
    Scholar,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Bonus {
    Power(u32),
    Precision(u32),
    Ferocity(u32),
    Multiplier(modifier::Modifier),
    CritChance(u32),
}

impl Rune {
    fn bonuses(&self, count: usize) -> Vec<Bonus> {
        use modifier::*;
        use Bonus::*;

        match self {
            Self::Scholar => {
                let scholar_bonuses = [
                    vec![Power(25)],
                    vec![Ferocity(35)],
                    vec![Power(50)],
                    vec![Ferocity(65)],
                    vec![Power(100)],
                    vec![Ferocity(125), Multiplier(Fixed::Scholar.val())],
                ];

                scholar_bonuses[..count]
                    .iter()
                    .flatten()
                    .copied()
                    .collect::<Vec<_>>()
            }
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Boon {
    Might,
    Fury,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Buff {
    Spotter,
    FrostSpirit,
    EmpowerAllies,
    BannerOfStrength,
    BannerOfDiscipline,

    Ferocious,
    SignetOfTheWild,
    TwiceAsVicious,
    SicEm,
    OneWolfPack,

    OneWolfPackIcd,
}

impl Buff {
    pub const fn from_id(id: i32) -> Option<Self> {
        use Buff::*;
        Some(match id {
            33902 => SicEm,
            45600 => TwiceAsVicious,
            44139 => OneWolfPack,
            40642 => OneWolfPackIcd,
            _ => return None,
        })
    }

    pub const fn id(&self) -> Option<i32> {
        use Buff::*;
        Some(match self {
            SicEm => 33902,
            TwiceAsVicious => 45600,
            OneWolfPack => 44139,
            OneWolfPackIcd => 40642,
            _ => return None,
        })
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Trait {
    HuntersTactics,
    ViciousQuarry,
    PackAlpha,
    PetsProwess,
    HonedAxes,
    LoudWhistle,
    OppressiveSuperiority,
    FuriousStrength,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Infusion {
    Empty,
    NineAgonyFivePower,
}

impl HasStats for Infusion {
    fn stats(&self) -> Stats {
        use Infusion::*;

        match self {
            Empty => Stats::new(0, 0, 0),
            NineAgonyFivePower => Stats::new(5, 0, 0),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum Food {
    SweetAndSpicyButternutSquashSoup,
}

impl HasStats for Food {
    fn stats(&self) -> Stats {
        use Food::*;

        match self {
            SweetAndSpicyButternutSquashSoup => Stats::new(100, 0, 70),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum Utility {
    SuperiorSharpeningStone,
}

impl Utility {
    fn effect(&self, stats: &Stats) -> Stats {
        match self {
            Self::SuperiorSharpeningStone => {
                let power_from_prec = (0.03 * stats.precision as f64).round();
                let power_from_fero = (0.06 * stats.ferocity as f64).round();
                let increase = power_from_prec + power_from_fero;
                Stats::new(increase as u32, 0, 0)
            }
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct Head(Armor);
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct Shoulders(Armor);
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct Chest(Armor);
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct Hands(Armor);
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct Leggings(Armor);
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct Feet(Armor);

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct Armor {
    infusion: Infusion,
    rune: Rune,
    stats: Stats,
}

impl Armor {
    const fn new(infusion: Infusion, rune: Rune, stats: Stats) -> Self {
        Self {
            infusion,
            rune,
            stats,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct Accessory {
    infusion: Infusion,
    stats: Stats,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct Ring {
    infusions: (Infusion, Infusion, Infusion),
    stats: Stats,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct Amulet {
    stats: Stats,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct Back {
    infusions: (Infusion, Infusion),
    stats: Stats,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum Weaponset {
    Dual(Mainhand, Offhand),
    Twohand(Twohand),
}

impl Weaponset {
    pub const fn sigils(&self) -> (Sigil, Sigil) {
        match self {
            Self::Dual(one, two) => (one.sigil, two.sigil),
            Self::Twohand(one) => one.sigils,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct Mainhand {
    kind: MainhandKind,
    infusion: Infusion,
    sigil: Sigil,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct Offhand {
    kind: OffhandKind,
    infusion: Infusion,
    sigil: Sigil,
}

/*
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum WeaponKind {
Mainhand(MainhandKind),
Offhand(OffhandKind),
Twohand(TwohandKind),
}
*/

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum MainhandKind {
    Dagger,
    Sword,
    Axe,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum OffhandKind {
    Axe,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum TwohandKind {
    Longbow,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct Twohand {
    kind: TwohandKind,
    infusions: (Infusion, Infusion),
    sigils: (Sigil, Sigil),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct WeaponStrength {
    min: u32,
    max: u32,
}

impl WeaponStrength {
    pub const fn min(&self) -> f64 {
        self.min as f64
    }

    pub const fn max(&self) -> f64 {
        self.max as f64
    }

    pub fn avg(&self) -> f64 {
        (self.min as f64 + self.max as f64) / 2.
    }
}

trait Strength {
    fn strength(&self) -> WeaponStrength;
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Weapon {
    Unequipped,
    Longbow,
    Dagger,
    Sword,
    Axe,
}

impl Weapon {
    pub fn strn(&self) -> WeaponStrength {
        self.strength()
    }
}

impl Strength for Weapon {
    fn strength(&self) -> WeaponStrength {
        match self {
            Self::Unequipped => WeaponStrength { min: 656, max: 725 },
            Self::Longbow => WeaponStrength {
                min: 966,
                max: 1134,
            },
            Self::Dagger => WeaponStrength {
                min: 970,
                max: 1030,
            },
            Self::Sword => WeaponStrength {
                min: 950,
                max: 1050,
            },
            Self::Axe => WeaponStrength {
                min: 900,
                max: 1100,
            },
        }
    }
}

trait IsWeapon {
    fn weapon(&self) -> Weapon;
    fn specific(weapon: Weapon) -> Option<Self>
    where
        Self: Sized;
}

impl IsWeapon for TwohandKind {
    fn weapon(&self) -> Weapon {
        match self {
            Self::Longbow => Weapon::Longbow,
        }
    }

    fn specific(weapon: Weapon) -> Option<Self> {
        match weapon {
            Weapon::Longbow => Some(Self::Longbow),
            _ => None,
        }
    }
}

impl IsWeapon for MainhandKind {
    fn weapon(&self) -> Weapon {
        match self {
            Self::Dagger => Weapon::Dagger,
            Self::Sword => Weapon::Sword,
            Self::Axe => Weapon::Axe,
        }
    }

    fn specific(weapon: Weapon) -> Option<Self> {
        match weapon {
            Weapon::Dagger => Some(Self::Dagger),
            Weapon::Sword => Some(Self::Sword),
            Weapon::Axe => Some(Self::Axe),
            _ => None,
        }
    }
}

impl IsWeapon for OffhandKind {
    fn weapon(&self) -> Weapon {
        match self {
            Self::Axe => Weapon::Axe,
        }
    }

    fn specific(weapon: Weapon) -> Option<Self> {
        match weapon {
            Weapon::Axe => Some(Self::Axe),
            _ => None,
        }
    }
}

impl HasStats for Weapon {
    fn stats(&self) -> Stats {
        match self {
            Self::Unequipped => Stats::new(0, 0, 0),
            Self::Longbow => Stats::new(251, 179, 179),
            Self::Dagger => Stats::new(125, 90, 90),
            Self::Sword => Stats::new(125, 90, 90),
            Self::Axe => Stats::new(125, 90, 90),
        }
    }
}

impl Strength for Mainhand {
    fn strength(&self) -> WeaponStrength {
        self.kind.strength()
    }
}

impl Strength for Offhand {
    fn strength(&self) -> WeaponStrength {
        self.kind.strength()
    }
}

impl Strength for MainhandKind {
    fn strength(&self) -> WeaponStrength {
        match self {
            Self::Dagger => Weapon::Dagger.strength(),
            Self::Sword => Weapon::Sword.strength(),
            Self::Axe => Weapon::Axe.strength(),
        }
    }
}

impl Strength for OffhandKind {
    fn strength(&self) -> WeaponStrength {
        match self {
            Self::Axe => Weapon::Axe.strength(),
        }
    }
}

impl Strength for TwohandKind {
    fn strength(&self) -> WeaponStrength {
        match self {
            Self::Longbow => Weapon::Longbow.strength(),
        }
    }
}

impl Strength for Twohand {
    fn strength(&self) -> WeaponStrength {
        self.kind.strength()
    }
}

impl HasStats for Head {
    fn stats(&self) -> Stats {
        self.0.stats()
    }
}

impl HasStats for Shoulders {
    fn stats(&self) -> Stats {
        self.0.stats()
    }
}

impl HasStats for Chest {
    fn stats(&self) -> Stats {
        self.0.stats()
    }
}

impl HasStats for Hands {
    fn stats(&self) -> Stats {
        self.0.stats()
    }
}

impl HasStats for Leggings {
    fn stats(&self) -> Stats {
        self.0.stats()
    }
}

impl HasStats for Feet {
    fn stats(&self) -> Stats {
        self.0.stats()
    }
}

impl HasStats for Armor {
    fn stats(&self) -> Stats {
        self.stats + self.infusion.stats()
    }
}

impl HasStats for Accessory {
    fn stats(&self) -> Stats {
        self.stats + self.infusion.stats()
    }
}

impl HasStats for Ring {
    fn stats(&self) -> Stats {
        self.stats + self.infusions.stats()
    }
}

impl HasStats for Amulet {
    fn stats(&self) -> Stats {
        self.stats
    }
}

impl HasStats for Back {
    fn stats(&self) -> Stats {
        self.stats + self.infusions.stats()
    }
}

impl HasStats for Weaponset {
    fn stats(&self) -> Stats {
        match self {
            Self::Dual(one, two) => one.stats() + two.stats(),
            Self::Twohand(one) => one.stats(),
        }
    }
}

impl HasStats for Mainhand {
    fn stats(&self) -> Stats {
        self.kind.stats() + self.infusion.stats()
    }
}

impl HasStats for Offhand {
    fn stats(&self) -> Stats {
        self.kind.stats() + self.infusion.stats()
    }
}

impl HasStats for MainhandKind {
    fn stats(&self) -> Stats {
        match self {
            Self::Dagger => Weapon::Dagger.stats(),
            Self::Sword => Weapon::Sword.stats(),
            Self::Axe => Weapon::Axe.stats(),
        }
    }
}

impl HasStats for OffhandKind {
    fn stats(&self) -> Stats {
        match self {
            Self::Axe => Weapon::Axe.stats(),
        }
    }
}

impl HasStats for TwohandKind {
    fn stats(&self) -> Stats {
        match self {
            Self::Longbow => Weapon::Longbow.stats(),
        }
    }
}

impl HasStats for Twohand {
    fn stats(&self) -> Stats {
        self.kind.stats() + self.infusions.stats()
    }
}
