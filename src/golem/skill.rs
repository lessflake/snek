pub struct Info {
    pub id: i32,
    pub coefficient: f64,
    pub uncrittable: bool,
    pub weapon: WeaponKind,
}

impl Info {
    const fn new(id: i32, coefficient: f64, uncrittable: bool, weapon: WeaponKind) -> Self {
        Self {
            id,
            coefficient,
            uncrittable,
            weapon,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Skill {
    WorldlyImpact,
    Barrage,
    WhirlingDefense,
    HuntersShot,
    FrostBurst,
    PathOfScars,
    OneWolfPack,
    FrenziedAttack,
    LightningStrike,
    RapidFire,
    DoubleArc,
    GroundworkGouge,
    LeadingSwipe,
    SerpentStab,
    DeadlyDelivery,
    PointBlankShot,
    FrostTrap,
    LongRangeShot,
    WintersBite,
    Ricochet,
    Slash,
    CripplingThrust,
    PrecisionSwipe,
}

impl Skill {
    pub const fn id(&self) -> i32 {
        self.data().id
    }

    pub const fn coefficient(&self) -> f64 {
        self.data().coefficient
    }

    pub const fn uncrittable(&self) -> bool {
        self.data().uncrittable
    }

    pub const fn weapon(&self) -> WeaponKind {
        self.data().weapon
    }

    const fn data(&self) -> Info {
        use super::character::Weapon::*;
        use Skill::*;
        use WeaponKind::*;

        match self {
            WorldlyImpact => Info::new(40729, 1.89, false, Current),
            Barrage => Info::new(12469, 0.5, false, Fixed(Longbow)),
            WhirlingDefense => Info::new(12639, 0.66, false, Fixed(Axe)),
            HuntersShot => Info::new(12573, 0.4, false, Fixed(Longbow)),
            FrostBurst => Info::new(9428, 1.0, false, Fixed(Unequipped)),
            PathOfScars => Info::new(12638, 1.2, false, Fixed(Axe)),
            OneWolfPack => Info::new(42145, 0.63, false, Fixed(Unequipped)),
            FrenziedAttack => Info::new(43548, 0.4, false, Current),
            LightningStrike => Info::new(9292, 1.1, true, Fixed(Unequipped)),
            RapidFire => Info::new(12509, 0.375, false, Fixed(Longbow)),
            DoubleArc => Info::new(43536, 0.5, false, Fixed(Dagger)),
            GroundworkGouge => Info::new(45426, 0.4, false, Fixed(Dagger)),
            LeadingSwipe => Info::new(40301, 0.42, false, Fixed(Dagger)),
            SerpentStab => Info::new(41800, 0.44, false, Fixed(Dagger)),
            DeadlyDelivery => Info::new(44278, 0.88, false, Fixed(Dagger)),
            PointBlankShot => Info::new(12511, 0.8, false, Fixed(Longbow)),
            FrostTrap => Info::new(12492, 1.0, false, Fixed(Unequipped)),
            LongRangeShot => Info::new(12510, 0.7, false, Fixed(Longbow)),
            WintersBite => Info::new(12490, 1.25, false, Fixed(Axe)),
            Ricochet => Info::new(12466, 0.8, false, Fixed(Axe)),
            Slash => Info::new(12471, 0.7, false, Fixed(Sword)),
            CripplingThrust => Info::new(12472, 0.7, false, Fixed(Sword)),
            PrecisionSwipe => Info::new(12473, 0.96, false, Fixed(Sword)),
        }
    }

    pub const fn name(&self) -> &'static str {
        use Skill::*;

        match self {
            WorldlyImpact => "Worldly Impact",
            Barrage => "Barrage",
            WhirlingDefense => "Whirling Defense",
            HuntersShot => "Hunter's Shot",
            FrostBurst => "Frost Burst",
            PathOfScars => "Path of Scars",
            OneWolfPack => "One Wolf Pack",
            FrenziedAttack => "Frenzied Attack",
            LightningStrike => "Lightning Strike",
            RapidFire => "Rapid Fire",
            DoubleArc => "Double Arc",
            GroundworkGouge => "Groundwork Gouge",
            LeadingSwipe => "Leading Swipe",
            SerpentStab => "Serpent Stab",
            DeadlyDelivery => "Deadly Delivery",
            PointBlankShot => "Point-Blank Shot",
            FrostTrap => "Frost Trap",
            LongRangeShot => "Long Range Shot",
            WintersBite => "Winter's Bite",
            Ricochet => "Ricochet",
            Slash => "Slash",
            CripplingThrust => "Crippling Thrust",
            PrecisionSwipe => "Precision Swipe",
        }
    }

    pub const fn from_id(id: i32) -> Option<Self> {
        use Skill::*;

        Some(match id {
            40729 => WorldlyImpact,
            12469 => Barrage,
            12639 => WhirlingDefense,
            12573 => HuntersShot,
            9428 => FrostBurst,
            12638 => PathOfScars,
            42145 => OneWolfPack,
            43548 => FrenziedAttack,
            9292 => LightningStrike,
            12509 => RapidFire,
            43536 => DoubleArc,
            45426 => GroundworkGouge,
            40301 => LeadingSwipe,
            41800 => SerpentStab,
            44278 => DeadlyDelivery,
            12511 => PointBlankShot,
            12492 => FrostTrap,
            12510 => LongRangeShot,
            12490 => WintersBite,
            12466 => Ricochet,
            12471 => Slash,
            12472 => CripplingThrust,
            12473 => PrecisionSwipe,
            _ => return None,
        })
    }
}

pub enum WeaponKind {
    Current,
    Fixed(super::character::Weapon),
}
