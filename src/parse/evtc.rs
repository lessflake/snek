use std::{
    collections::{hash_map::Entry, HashMap},
    io::Read,
    mem,
    path::Path,
    slice,
};

use super::event;

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub struct AgentId(u16);
pub type AgentMap = HashMap<AgentId, Agent>;
pub type SkillMap = HashMap<i32, String>;
pub type Time = u64;

impl AgentId {
    pub const fn to_inner(self) -> u16 {
        self.0
    }
}

#[derive(Debug, Clone)]
pub struct Agent {
    pub name: String,
    pub health: u64,
    pub first_aware: Time,
    pub last_aware: Time,
}

pub struct Data {
    pub boss: AgentId,
    pub agents: AgentMap,
    pub players: AgentMap,
    pub skills: SkillMap,
    pub events: Vec<event::Event>,
}

impl Data {
    pub fn id_for(&self, name: &str) -> Option<AgentId> {
        self.players.iter().find_map(|(id, a)| {
            if a.name.as_str() == name {
                Some(*id)
            } else {
                None
            }
        })
    }
}

#[derive(Debug)]
struct Header {
    evtc: [u8; 4],
    timestamp: [u8; 8],
    revision: u8,
    boss_id: [u8; 2],
    pad: u8,
}

#[allow(dead_code)]
struct EvtcAgent {
    addr: u64,
    prof: u32,
    is_elite: u32,
    toughness: u16,
    concentration: u16,
    healing: u16,
    hitbox_width: u16,
    condition: u16,
    hitbox_height: u16,
    name: [u8; 64],
}

struct Skill {
    id: i32,
    name: [u8; 64],
}

#[derive(Debug)]
pub struct RawEvent {
    pub time: Time,
    pub src_agent: u64,
    pub dst_agent: u64,
    pub value: i32,
    pub buff_dmg: i32,
    pub overstack_value: u32,
    pub skill_id: u32,
    pub src_instid: AgentId,
    pub dst_instid: AgentId,
    pub src_master_instid: AgentId,
    pub dst_master_instid: AgentId,

    pub iff: u8,
    pub buff: u8,
    pub result: u8,
    pub is_activation: u8,
    pub is_buffremove: u8,
    pub is_ninety: u8,
    pub is_fifty: u8,
    pub is_moving: u8,
    pub is_statechange: u8,
    pub is_shields: u8,
    pub is_offcycle: u8,

    pad61: u8,
    pad62: u8,
    pad63: u8,
    pad64: u8,
}

pub fn parse(path: impl AsRef<Path>) -> Data {
    let path = path.as_ref();
    let bytes = load_evtc(path);

    // assume that correct extension means correct format ...
    // TODO: don't do this (logs come straight from arcdps though, unlikely to be malformed)
    unsafe { parse_unchecked(&bytes[..]) }
}

fn load_evtc(path: &Path) -> Vec<u8> {
    let extension = path.extension().and_then(|s| s.to_str()).unwrap();
    let mut file = std::fs::File::open(&path).unwrap();

    let mut bytes = vec![];
    match extension {
        "zevtc" => {
            let mut archive = zip::ZipArchive::new(file).unwrap();
            let mut unzipped = archive.by_index(0).unwrap();
            unzipped.read_to_end(&mut bytes).unwrap();
        }
        "evtc" => {
            file.read_to_end(&mut bytes).unwrap();
        }
        _ => panic!("trying to parse file with invalid extension"),
    };

    bytes
}

unsafe fn parse_unchecked(mut rdr: impl ReadStruct) -> Data {
    let header = rdr.read_struct::<Header>().unwrap();
    assert_eq!(header.revision, 1);

    let agent_count = rdr.read_struct::<u32>().unwrap();
    let mut agents_by_addr = HashMap::<u64, Agent>::new();
    let mut players_by_addr = HashMap::<u64, Agent>::new();
    agents_by_addr.reserve(agent_count as _);

    let mut boss_addr = u64::MAX;

    for _ in 0..agent_count {
        let evtc_agent = rdr.read_struct::<EvtcAgent>().unwrap();
        let name = str_from_u8_nul_utf8_unchecked(&evtc_agent.name).to_string();
        let len = name.len();
        if evtc_agent.name[len + 1] != 0 {
            let acc_name = str_from_u8_nul_utf8_unchecked(&evtc_agent.name[len + 2..]).to_string();
            let agent = Agent {
                name: acc_name,
                health: 0,
                first_aware: 0,
                last_aware: 0,
            };
            players_by_addr.insert(evtc_agent.addr, agent);
        } else if boss_addr == u64::MAX {
            // seems to be consistent that the first non-player agent we encounter is the boss
            boss_addr = evtc_agent.addr;
        }
        let agent = Agent {
            name,
            health: 0,
            first_aware: u64::MAX,
            last_aware: 0,
        };
        agents_by_addr.insert(evtc_agent.addr, agent);
    }

    let skill_count = rdr.read_struct::<u32>().unwrap();
    let mut skills = SkillMap::new();
    skills.reserve(skill_count as _);
    for _ in 0..skill_count {
        let skill = rdr.read_struct::<Skill>().unwrap();
        let skill_name = String::from_utf8_unchecked(skill.name.to_vec());
        skills.insert(skill.id, skill_name);
    }

    let mut events = Vec::new();
    let first_event = rdr.read_struct::<RawEvent>().unwrap();
    println!("start time: {}", first_event.time);

    let mut boss: AgentId = AgentId(0);
    let mut agents = AgentMap::new();
    let mut players = AgentMap::new();

    while let Ok(mut raw_event) = rdr.read_struct::<RawEvent>() {
        raw_event.time = raw_event.time.saturating_sub(first_event.time);

        if let Some((id, instid)) = event_agent(&raw_event) {
            // Set instance IDs
            if let Entry::Vacant(entry) = agents.entry(instid) {
                if id == boss_addr {
                    boss = instid;
                }

                if let Some(agent) = agents_by_addr.get(&id).cloned() {
                    entry.insert(agent.clone());
                }

                if let Some(player) = players_by_addr.get(&id).cloned() {
                    players.insert(instid, player.clone());
                }
            }

            // Set first_aware, last_aware, health
            if let Some(agent) = agents.get_mut(&instid) {
                if raw_event.time < agent.first_aware {
                    agent.first_aware = raw_event.time;
                } else {
                    agent.last_aware = raw_event.time;
                }

                if raw_event.is_statechange == 12 {
                    // max health update
                    agent.health = agent.health.max(raw_event.dst_agent);
                }
            }
        }

        if let Some(event) = raw_event.into_event() {
            events.push(event);
        }
    }

    Data {
        boss,
        agents,
        players,
        skills,
        events,
    }
}

pub fn target_id(path: impl AsRef<Path>) -> Option<u16> {
    use byteorder::{ByteOrder, LittleEndian};

    let path = path.as_ref();
    let extension = path.extension().and_then(|s| s.to_str())?;
    let mut f = std::fs::File::open(path).ok()?;

    let mut buffer = [0; 16];
    match extension {
        "zevtc" => {
            let mut archive = zip::ZipArchive::new(f).ok()?;
            let mut unzipped = archive.by_index(0).ok()?;
            unzipped.read(&mut buffer[..]).ok()?;
        }
        "evtc" => {
            f.read(&mut buffer[..]).ok()?;
        }
        _ => panic!("trying to parse file with invalid extension: {:?}", path),
    };

    Some(LittleEndian::read_u16(&buffer[13..15]))
}

impl RawEvent {
    fn into_event(self) -> Option<event::Event> {
        use event::*;

        let time = self.time;

        let kind = match self.is_statechange {
            0 => {
                if self.buff != 0 {
                    match self.is_buffremove {
                        3 => EventKind::buff_remove(self),
                        0 => {
                            if self.value != 0 {
                                EventKind::buff_apply(self)
                            } else if self.result == 0 {
                                EventKind::cond_damage(self)
                            } else {
                                return None;
                            }
                        }
                        _ => return None,
                    }
                } else {
                    match self.is_activation {
                        0 => EventKind::phys_damage(self),
                        1 => EventKind::cast_start(self),
                        3 => EventKind::cast_cfire(self),
                        4 => EventKind::cast_cancel(self),
                        5 => EventKind::cast_end(self),
                        _ => return None,
                    }
                }
            }
            1 => EventKind::combat_enter(self),
            2 => EventKind::combat_exit(self),
            4 => EventKind::death(self),
            6 => EventKind::spawn(self),
            7 => EventKind::despawn(self),
            11 => EventKind::weapon_swap(self),
            17 => EventKind::reward(self),
            _ => return None,
        };

        Some(Event { time, kind })
    }
}

#[allow(dead_code)]
enum CbtStateChange {
    NotStateChange = 0,
    EnterCombat = 1,
    ExitCombat = 2,
    ChangeUp = 3,
    ChangeDead = 4,
    ChangeDown = 5,
    Spawn = 6,
    Despawn = 7,
    HealthUpdate = 8,
    LogStart = 9,
    LogEnd = 10,
    WeapSwap = 11,
    MaxHealthUpdate = 12,
    PointOfView = 13,
    Language = 14,
    GwBuild = 15,
    ShardId = 16,
    Reward = 17,
    BuffInitial = 18,
    Position = 19,
    Velocity = 20,
    Facing = 21,
    TeamChange = 22,
    AttackTarget = 23,
    Targetable = 24,
    MapId = 25,
    ReplInfo = 26,
    StackActive = 27,
    StackReset = 28,
    Guild = 29,
    BuffInfo = 30,
    BuffFormula = 31,
    SkillInfo = 32,
    SkillTiming = 33,
    BreakbarState = 34,
    BreakbarPercent = 35,
    Error = 36,
    Tag = 37,
}

fn src_is_agent(is_statechange: u8) -> bool {
    use CbtStateChange::*;
    match is_statechange {
        x if x == NotStateChange as u8 => true,
        x if x == EnterCombat as u8 => true,
        x if x == ExitCombat as u8 => true,
        x if x == ChangeUp as u8 => true,
        x if x == ChangeDead as u8 => true,
        x if x == ChangeDown as u8 => true,
        x if x == Spawn as u8 => true,
        x if x == Despawn as u8 => true,
        x if x == HealthUpdate as u8 => true,
        x if x == WeapSwap as u8 => true,
        x if x == MaxHealthUpdate as u8 => true,
        x if x == PointOfView as u8 => true,
        x if x == BuffInitial as u8 => true,
        x if x == Position as u8 => true,
        x if x == Velocity as u8 => true,
        x if x == Facing as u8 => true,
        x if x == TeamChange as u8 => true,
        x if x == AttackTarget as u8 => true,
        x if x == Targetable as u8 => true,
        x if x == StackActive as u8 => true,
        x if x == StackReset as u8 => true,
        x if x == BreakbarState as u8 => true,
        x if x == BreakbarPercent as u8 => true,
        _ => false,
    }
}

fn dst_is_agent(is_statechange: u8) -> bool {
    use CbtStateChange::*;
    match is_statechange {
        x if x == NotStateChange as u8 => true,
        x if x == AttackTarget as u8 => true,
        _ => false,
    }
}

fn event_agent(event: &RawEvent) -> Option<(u64, AgentId)> {
    if src_is_agent(event.is_statechange) {
        Some((event.src_agent, event.src_instid))
    } else if dst_is_agent(event.is_statechange) {
        Some((event.dst_agent, event.dst_instid))
    } else {
        None
    }
}

// unsafe transmute: logs have reliable format
trait ReadStruct: std::io::Read {
    unsafe fn read_struct<T>(&mut self) -> std::io::Result<T> {
        let mut s: T = mem::zeroed();
        let size = mem::size_of::<T>();
        let s_slice = slice::from_raw_parts_mut(&mut s as *mut _ as *mut u8, size);
        self.read_exact(s_slice)?;
        Ok(s)
    }
}

impl ReadStruct for &[u8] {}

impl std::fmt::Display for AgentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

unsafe fn str_from_u8_nul_utf8_unchecked(src: &[u8]) -> &str {
    let nul_range_end = src
        .iter()
        .position(|&c| c == b'\0')
        .unwrap_or_else(|| src.len());
    std::str::from_utf8_unchecked(&src[0..nul_range_end])
}
