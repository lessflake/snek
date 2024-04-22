use crate::golem::skill;
use crate::parse::Time;

// optimal cast durations
// axe 5: 2600s
// lb2: 1800ms

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Cast {
    kind: Kind,
    status: Status,
    duration: Time,
}

impl Cast {
    pub const fn new(kind: Kind, status: Status, duration: Time) -> Self {
        Self {
            kind,
            status,
            duration,
        }
    }

    pub const fn duration(&self) -> Time {
        self.duration
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Status {
    Normal,
    Cancel,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Kind {
    Skill(skill::Skill),
    OneWolfPack,

    Dodge,
    WeaponSwap,
}

impl Kind {
    pub const fn from_id(id: i32) -> Option<Self> {
        use Kind::*;

        Some(match id {
            45717 => OneWolfPack,
            65001 => Dodge,
            _ => {
                if let Some(s) = skill::Skill::from_id(id) {
                    Skill(s)
                } else {
                    return None;
                }
            }
        })
    }
}

#[derive(Debug, Clone)]
pub struct Rotation {
    pub order: Vec<Kind>, // can't just be skills. need weaponswaps too
}

impl Rotation {
    pub fn total_time(&self) -> Time {
        use skill::Skill::*;
        use Kind::*;

        let mut time = 0;
        for (i, skill) in self.order.iter().enumerate() {
            // Use this to generate events?
            time += self.time_for(i).unwrap();
        }

        time
    }

    pub fn time_for(&self, i: usize) -> Option<Time> {
        use skill::Skill::*;
        use Kind::*;

        match self.order.get(i) {
            Some(Skill(WorldlyImpact)) => Some(680),
            Some(Skill(Barrage)) => Some(1800),
            Some(Skill(WhirlingDefense)) => Some(2600),
            Some(Skill(HuntersShot)) => Some(350),
            Some(Skill(PathOfScars)) => Some(430),
            Some(Skill(FrenziedAttack)) => Some(750),
            Some(Skill(RapidFire)) => Some(1800),
            Some(Skill(DoubleArc)) => Some(570),
            Some(Skill(PointBlankShot)) => Some(325),
            Some(Skill(FrostTrap)) => match self.order.get(i + 1) {
                Some(Skill(WorldlyImpact))
                | Some(Skill(PointBlankShot))
                | Some(Skill(HuntersShot))
                | Some(WeaponSwap) => Some(400),
                _ => Some(600),
            },
            Some(Skill(LongRangeShot)) => Some(680),
            Some(Skill(Ricochet)) => Some(600),
            Some(Skill(WintersBite)) => Some(500),
            Some(Skill(GroundworkGouge)) => Some(275),
            Some(Skill(LeadingSwipe)) => Some(325),
            Some(Skill(SerpentStab)) => Some(275),
            Some(Skill(DeadlyDelivery)) => Some(650),
            Some(Skill(Slash)) => Some(480),
            Some(Skill(CripplingThrust)) => Some(333),
            Some(Skill(PrecisionSwipe)) => Some(600),
            Some(Kind::OneWolfPack) => Some(400),

            Some(WeaponSwap) => Some(0),
            Some(_) => unimplemented!(),
            None => None,
        }
    }
}
