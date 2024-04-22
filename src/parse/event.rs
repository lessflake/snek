use super::evtc::RawEvent;
use crate::parse::evtc::{Agent, AgentId, Time};

#[derive(Debug, Clone)]
pub struct Event {
    pub time: Time,
    pub kind: EventKind,
}

macro_rules! events {
    () => {};
    (@s) => {};

    (@r $($name:ident)* $(,)?) => {
        #[derive(Debug, Clone, Eq, PartialEq)]
        pub enum EventKind {
            $($name($name)),*
        }
    };

    (@r $($name:ident)*, $thing:ident$(($($item:ident: $type:ty),*))? $(, $($tail:tt)*)? ) => {
        events!(@r $($name)* $thing $(, $($tail)*)?);
    };

    (@s $name:ident$(($($item:ident: $type:ty),*))? $(, $($tail:tt)*)? ) => {
        #[derive(Debug, Clone, Eq, PartialEq)]
        pub struct $name {
            pub target: AgentId,
            $( $(pub $item: $type),* )?
        }

        $(events!(@s $($tail)*);)?
    };

    ( $name:ident$(($($item:ident: $type:ty),*))?, $($tail:tt)* ) => {
        events!(@r $name, $($tail)*);
        events!(@s $name$(($($item: $type),*))?, $($tail)* );
    };
}

events! {
    WeaponSwap(set: u16), // 0/1 water, 4/5 land
    PhysDamage(src: AgentId, dmg: i32, skill: i32),
    BuffApply(src: AgentId, id: i32, duration: i32), // src applied it to target
    BuffRemove(dst: AgentId, id: i32, stacks: u8), // "src had buff removed, dst removed it"
    CastStart(skill: i32, effect: Time, duration: Time),
    CastCFire(skill: i32, animation: Time, scaled: Time),
    CastCancel(skill: i32, animation: Time, scaled: Time),
    CastEnd(skill: i32),
    CombatEnter,
    CombatExit,
    CondDamage(src: AgentId, dmg: i32, skill: i32),
    Death,
    Reward(kind: i32, id: u16),
    Spawn,
    Despawn,
}

// TODO: having the macro generate something along these lines would be good
impl EventKind {
    pub const fn buff_apply(event: RawEvent) -> Self {
        Self::BuffApply(BuffApply {
            target: event.dst_instid,
            src: event.src_instid,
            id: event.skill_id as i32,
            duration: event.value,
        })
    }

    pub const fn buff_remove(event: RawEvent) -> Self {
        Self::BuffRemove(BuffRemove {
            target: event.src_instid,
            dst: event.dst_instid,
            id: event.skill_id as i32,
            stacks: event.result,
        })
    }

    pub const fn cast_start(event: RawEvent) -> Self {
        Self::CastStart(CastStart {
            target: event.src_instid,
            skill: event.skill_id as i32,
            effect: event.value as Time,
            duration: event.buff_dmg as Time,
        })
    }

    pub const fn cast_cfire(event: RawEvent) -> Self {
        Self::CastCFire(CastCFire {
            target: event.src_instid,
            skill: event.skill_id as i32,
            animation: event.value as Time,
            scaled: event.buff_dmg as Time,
        })
    }

    pub const fn cast_cancel(event: RawEvent) -> Self {
        Self::CastCancel(CastCancel {
            target: event.src_instid,
            skill: event.skill_id as i32,
            animation: event.value as Time,
            scaled: event.buff_dmg as Time,
        })
    }

    pub const fn cast_end(event: RawEvent) -> Self {
        Self::CastEnd(CastEnd {
            target: event.src_instid,
            skill: event.skill_id as i32,
        })
    }

    pub const fn combat_enter(event: RawEvent) -> Self {
        Self::CombatEnter(CombatEnter {
            target: event.src_instid,
        })
    }

    pub const fn combat_exit(event: RawEvent) -> Self {
        Self::CombatExit(CombatExit {
            target: event.src_instid,
        })
    }

    pub const fn cond_damage(event: RawEvent) -> Self {
        Self::CondDamage(CondDamage {
            target: event.dst_instid,
            src: event.src_instid,
            dmg: event.buff_dmg,
            skill: event.skill_id as i32,
        })
    }

    pub const fn phys_damage(event: RawEvent) -> Self {
        Self::PhysDamage(PhysDamage {
            target: event.dst_instid,
            src: event.src_instid,
            dmg: event.value,
            skill: event.skill_id as i32,
        })
    }

    pub const fn weapon_swap(event: RawEvent) -> Self {
        Self::WeaponSwap(WeaponSwap {
            target: event.src_instid,
            set: event.dst_agent as _,
        })
    }

    pub const fn death(event: RawEvent) -> Self {
        Self::Death(Death {
            target: event.src_instid,
        })
    }

    pub const fn reward(event: RawEvent) -> Self {
        Self::Reward(Reward {
            target: event.src_instid,
            kind: event.value,
            id: event.dst_instid.to_inner(),
        })
    }

    pub const fn spawn(event: RawEvent) -> Self {
        Self::Spawn(Spawn {
            target: event.src_instid,
        })
    }

    pub const fn despawn(event: RawEvent) -> Self {
        Self::Despawn(Despawn {
            target: event.src_instid,
        })
    }
}

// pretty printing for debugging
#[cfg(debug_assertions)]
impl Event {
    #[allow(dead_code)]
    pub fn pretty_print(
        &self,
        agents: &crate::parse::evtc::AgentMap,
        skills: &crate::parse::evtc::SkillMap,
    ) {
        match &self.kind {
            EventKind::BuffApply(e) /* if e.id == 762 */ => println!(
                //"{:6} {:>12} - {} -> {} for {}ms ({})",
                "{:6} {:>12} - {} applied {} ({}) to {} for {}ms",
                self.time,
                "Buff Apply",

                agents.pretty(&e.src),
                skills.pretty(&e.id),
                e.id,
                agents.pretty(&e.target),

                e.duration,
            ),
            EventKind::BuffRemove(e) /* if e.id == 762 */ => println!(
                "{:6} {:>12} - {} removed {} ({}) on {} x{}",
                self.time,
                "Buff Expire",

                agents.pretty(&e.dst),
                skills.pretty(&e.id),
                e.id,
                agents.pretty(&e.target),
                e.stacks,
            ),
            EventKind::CastStart(e) => println!(
                "{:6} {:>12} - {}: {} ({}ms - {}ms)", 
                self.time,
                "Cast Start", 
                agents.pretty(&e.target),
                skills.pretty(&e.skill),
                e.effect,
                e.duration,
            ),
            EventKind::CastCFire(e) => println!(
                "{:6} {:>12} - {}: {} ({}ms - {}ms)",
                self.time,
                "Cast CFire",
                agents.pretty(&e.target),
                skills.pretty(&e.skill),
                e.animation,
                e.scaled,
            ),
            EventKind::CastCancel(e) => println!(
                "{:6} {:>12} - {}: {} ({}ms - {}ms)",
                self.time,
                "Cast Cancel",
                agents.pretty(&e.target),
                skills.pretty(&e.skill),
                e.animation,
                e.scaled,
            ),
            EventKind::CastEnd(e) => println!(
                "{:6} {:>12} - {}: {}",
                self.time,
                "Cast End",
                agents.pretty(&e.target),
                skills.pretty(&e.skill),
            ),
            EventKind::CombatEnter(e) => println!(
                "{:6} {:>12} - {}",
                self.time,
                "Combat Enter",
                agents.pretty(&e.target)
            ),
            EventKind::CombatExit(e) => {
                println!("{:6} {:>12} - {}", self.time, "Combat Exit",  agents.pretty(&e.target))
            }
            EventKind::CondDamage(e) => println!(
                "{:6} {:>12} - {} -> {}, Skill: {}, Dmg: {}",
                self.time,
                "Condition",
                agents.pretty(&e.src),
                agents.pretty(&e.target),
                skills.pretty(&e.skill),
                e.dmg
            ),
            EventKind::PhysDamage(e) => println!(
                "{:6} {:>12} - {} -> {}, Skill: {} ({}), Dmg: {}",
                self.time,
                "Physical",
                agents.pretty(&e.src),
                agents.pretty(&e.target),
                skills.pretty(&e.skill),
                e.skill,
                e.dmg
            ),
            EventKind::WeaponSwap(e) => println!(
                "{:6} {:>12} - {} to set {}",
                self.time,
                "Swap",
                agents.pretty(&e.target),
                e.set
            ),
            EventKind::Death(e) => println!("{:6} {:>12} - {}", self.time, "Death", agents.pretty(&e.target)),
            EventKind::Reward(e) => println!("{:6} {:>12} - {} ({}), kind: {}, id: {}", self.time, "Reward", agents.pretty(&e.target), e.target, e.kind, e.id),
            EventKind::Spawn(e) => println!("{:6} {:>12} - {}", self.time, "Spawn", agents.pretty(&e.target)),
            EventKind::Despawn(e) => println!("{:6} {:>12} - {}", self.time, "Despawn", agents.pretty(&e.target)),
        }
    }
}

#[cfg(debug_assertions)]
pub trait Pretty<K> {
    fn pretty(&self, k: &K) -> String;
}

#[cfg(debug_assertions)]
impl<K> Pretty<K> for std::collections::HashMap<K, Agent>
where
    K: std::cmp::Eq + std::hash::Hash + std::fmt::Display,
{
    fn pretty(&self, k: &K) -> String {
        self.get(k)
            .map(|a| a.name.clone())
            .unwrap_or_else(|| format!("Not found ({})", k))
    }
}

#[cfg(debug_assertions)]
impl<K> Pretty<K> for std::collections::HashMap<K, String>
where
    K: std::cmp::Eq + std::hash::Hash + std::fmt::Display,
{
    fn pretty(&self, k: &K) -> String {
        self.get(k)
            .cloned()
            .unwrap_or_else(|| format!("Not found ({})", k))
    }
}
