use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Target {
    Mama,
    Siax,
    Enso,
    Skor,
    Arts,
    Arkk,
    Ai,

    AiElemental,
    AiDark,

    Golem,
}

impl Target {
    pub fn dir_name(self) -> &'static str {
        use Target::*;
        match self {
            Mama => "MAMA",
            Siax => "Nightmare Oratuss",
            Enso => "Ensolyss of the Endless Torment",
            Skor => "Skorvald the Shattered",
            Arts => "Artsariiv",
            Arkk => "Arkk",
            Ai => "Sorrowful Spellcaster",

            Golem => "Standard Kitty Golem",
            _ => unreachable!(),
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        use Target::*;
        match name {
            "MAMA" => Some(Mama),
            "Nightmare Oratuss" => Some(Siax),
            "Ensolyss of the Endless Torment" => Some(Enso),
            "Skorvald the Shattered" => Some(Skor),
            "Artsariiv" => Some(Arts),
            "Arkk" => Some(Arkk),
            "Sorrowful Spellcaster" => Some(Ai),
            "Standard Kitty Golem" => Some(Golem),
            _ => None,
        }
    }

    pub const fn from_id(id: u16) -> Option<Self> {
        use Target::*;
        let target = match id {
            17021 => Mama,
            17028 => Siax,
            16948 => Enso,
            17632 => Skor,
            17949 => Arts,
            17759 => Arkk,
            23254 => Ai,
            16199 => Golem,
            _ => return None,
        };
        Some(target)
    }
}

impl std::fmt::Display for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::Siax => "Siax the Corrupted",
            Self::Ai => "Ai, Keeper of the Peak",
            Self::AiElemental => "Ai, Keeper of the Peak (Elemental)",
            Self::AiDark => "Ai, Keeper of the Peak (Dark)",
            t => t.dir_name(),
        };

        write!(f, "{}", name)
    }
}
