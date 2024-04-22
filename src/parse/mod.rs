mod event;
mod evtc;

use std::collections::HashMap;

use crate::{log, target::Target};

pub use event::Event;
pub use event::EventKind;
pub use evtc::target_id;
pub use evtc::AgentId;
pub use evtc::Data;
pub use evtc::Time;

#[derive(Debug, Clone)]
pub struct Encounter {
    pub target: Target,
    pub success: bool,
    pub phases: Vec<Phase>,
}

pub fn parse(log: &log::Log) -> Option<(Vec<Encounter>, evtc::Data)> {
    use Target::*;
    match log.target() {
        Mama | Siax | Enso | Skor | Arts | Arkk | Ai | Golem => {}
        _ => return None,
    }

    let data = evtc::parse(log.path());

    // make sure skorvald is cm via health check
    if log.target() == Skor && data.agents.get(&data.boss).unwrap().health < 5526980 {
        return None;
    }

    let ctx = gather_context(&data, log.target());

    if log.target() == Target::Ai {
        return Some((parse_ai(&data, ctx), data));
    }

    let phases = parse_phases(&ctx, log.target());

    let encounter = Encounter {
        target: log.target(),
        success: ctx.success.is_some(),
        phases,
    };

    Some((vec![encounter], data))
}

fn parse_ai(data: &evtc::Data, ctx: LogContext) -> Vec<Encounter> {
    let dark_form_phase_event_time = ctx.casts.get(&53569);
    let has_dark_form = ctx.casts.get(&61356).is_some();
    let has_elemental_form = !has_dark_form || dark_form_phase_event_time.is_some();

    let mut offset = 0;

    // find when dark form started if log has both parts
    if has_elemental_form && has_dark_form {
        offset = *ctx
            .casts
            .get(&61277)
            .map(|v| {
                v.iter()
                    .find(|t| *t >= dark_form_phase_event_time.unwrap().get(0).unwrap())
            })
            .flatten()
            .unwrap()
            + 1;

        // look for 895 invuln removal past dark form start time
        if let Some(invuln_loss) = ctx
            .other_invuln_changes
            .iter()
            .find(|(t, r)| *t <= offset && *r)
        {
            offset = invuln_loss.0 + 1;
        }
    }

    let mut encounters: Vec<Encounter> = Vec::new();
    let mut ctx = ctx;

    if has_elemental_form {
        ctx.start = data.agents.get(&data.boss).unwrap().first_aware;
        let elemental_phases = parse_phases_ai_elemental(&ctx);
        let elemental_encounter = Encounter {
            target: Target::AiElemental,
            success: ctx.success.is_some(),
            phases: elemental_phases,
        };

        encounters.push(elemental_encounter);
    }

    if has_dark_form {
        offset = offset.max(data.agents.get(&data.boss).unwrap().first_aware);
        ctx.success = if has_elemental_form {
            check_success_ai(&ctx, offset)
        } else {
            ctx.success
        };

        let dark_phases = parse_phases_ai_dark(&ctx, offset);

        let dark_encounter = Encounter {
            target: Target::AiDark,
            success: ctx.success.is_some(),
            phases: dark_phases,
        };
        encounters.push(dark_encounter);
    }

    encounters
}

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Phase {
    start: Time,
    end: Time,
    pub name: Option<String>,
}

fn parse_phases(ctx: &LogContext, target: Target) -> Vec<Phase> {
    match target {
        Target::Mama
        | Target::Siax
        | Target::Enso
        | Target::Skor
        | Target::Arts
        | Target::Arkk
        | Target::Golem => parse_phases_by_invulns(ctx, target),
        _ => panic!(),
    }
}

fn parse_phases_ai_dark(ctx: &LogContext, offset: Time) -> Vec<Phase> {
    let dark_form_start = offset;
    let dark_form_end = ctx.success.unwrap_or(ctx.last_event);

    let mut phases = Vec::new();
    phases.push((dark_form_start - offset, dark_form_end - offset).into());

    if let Some(fear_to_sorrow) = ctx
        .casts
        .get(&61606)
        .map(|v| v.iter().find(|t| **t >= offset))
        .flatten()
    {
        phases.push((dark_form_start + 1 - offset, fear_to_sorrow - offset).into());

        if let Some(sorrow_to_guilt) = ctx
            .casts
            .get(&61602)
            .map(|v| v.iter().find(|t| **t >= offset))
            .flatten()
        {
            phases.push((fear_to_sorrow + 1 - offset, sorrow_to_guilt - offset).into());
            phases.push((sorrow_to_guilt + 1 - offset, dark_form_end - offset).into());
        } else {
            phases.push((fear_to_sorrow + 1 - offset, dark_form_end - offset).into());
        }
    }

    phases
}

fn parse_phases_ai_elemental(ctx: &LogContext) -> Vec<Phase> {
    let mut phases = Vec::new();
    let end_time = ctx.success.unwrap_or(ctx.last_event);
    phases.push((ctx.start, end_time).into());

    let invuln_loss_times = ctx
        .invuln_changes
        .iter()
        .filter(|(_, is_gain)| *is_gain)
        .map(|(time, _)| *time)
        .collect::<Vec<_>>();
    let invuln_gain_times = ctx
        .invuln_changes
        .iter()
        .filter(|(_, is_gain)| (!*is_gain))
        .map(|(time, _)| *time)
        .collect::<Vec<_>>();

    let mut start = ctx.start;

    for (i, gain_time) in invuln_gain_times.iter().enumerate() {
        let end = *gain_time;
        if i < invuln_loss_times.len() {
            phases.push((start, end).into());
            let loss_time = *invuln_loss_times.get(i).unwrap();
            if let Some(casts) = ctx.casts.get(&61385) {
                if let Some(cast_time) = casts.iter().find(|time| **time >= loss_time) {
                    start = *cast_time;
                } else {
                    break;
                }
            } else {
                break;
            }
        } else {
            phases.push((start, end).into());
        }
    }

    phases
}

fn parse_phases_by_invulns(ctx: &LogContext, target: Target) -> Vec<Phase> {
    let mut phases: Vec<Phase> = Vec::new();
    let mut extra_phases: Vec<Phase> = Vec::new();

    let mut invuln_times = ctx.invuln_changes.iter().map(|(time, _)| *time);

    let start_time = ctx.start;
    let end_time = ctx.success.unwrap_or(ctx.last_event);

    // first phase is always fight start time to end time
    phases.push((start_time, end_time).into());

    // first boss phase is either time from start to first invuln or
    // if it's the only phase then start to end
    if let Some(first_invuln) = invuln_times.next() {
        phases.push((start_time, first_invuln).into());
    } else {
        phases.push((start_time, end_time).into());
    }

    // for all pairs of invuln removals and gains: add a phase
    while let (Some(phase_start), Some(phase_end)) = (invuln_times.next(), invuln_times.next()) {
        if target == Target::Siax {
            let last_phase = phases.last().unwrap();
            extra_phases.push((last_phase.end, phase_start).into());
        }
        phases.push((phase_start, phase_end).into());
    }

    // if there's an invuln change left over, log ends either with defeat during
    // a phase or success with boss defeated. add phase to reflect this, later to
    // be modified with exact boss defeat time if log was success
    if let Some((time, was_removal)) = ctx.invuln_changes.last() {
        if target == Target::Siax && *was_removal {
            let last_phase = phases.last().unwrap();
            extra_phases.push((last_phase.end, *time).into());
        }
        phases.push((*time, end_time).into());
    }

    if target == Target::Siax {
        /*
        if phases.len() >= 3 {
            let first_phase = &phases[1];
            let second_phase = &phases[2];
            let new_phase = (first_phase.end, second_phase.start).into();
            phases.push(new_phase);
        }
        if phases.len() >= 5 {
            let second_phase = &phases[2];
            let third_phase = &phases[3];
            let new_phase = (second_phase.end, third_phase.start).into();
            phases.push(new_phase);
        }
        */
        phases.append(&mut extra_phases);
    }

    phases
}

impl Phase {
    // as millis?
    pub const fn duration(&self) -> u64 {
        self.end - self.start
    }

    pub const fn end(&self) -> u64 {
        self.end
    }
}

impl From<(u64, u64)> for Phase {
    fn from((start, end): (Time, Time)) -> Self {
        Self {
            start,
            end,
            name: None,
        }
    }
}

#[derive(Debug, Default)]
struct LogContext {
    target: AgentId,
    success: Option<Time>,
    players: Vec<AgentId>,
    start: Time,
    end: Time,
    last_event: Time,
    last_aware: Time,
    first_reward: Option<Time>,
    last_dmg: Option<Time>,
    invuln_changes: Vec<(Time, bool)>,
    other_invuln_changes: Vec<(Time, bool)>,
    combat_enters: HashMap<AgentId, Time>,
    combat_exits: HashMap<AgentId, Time>,
    spawns: HashMap<AgentId, Time>,
    deaths: HashMap<AgentId, Time>,
    casts: HashMap<i32, Vec<Time>>,
}

fn gather_context(data: &evtc::Data, target: Target) -> LogContext {
    use event::*;

    let mut ctx = LogContext::default();
    ctx.target = data.boss;

    for (id, _) in data.players.iter() {
        ctx.players.push(*id);
    }

    ctx.last_event = data.events.iter().last().unwrap().time;
    ctx.last_aware = data.agents.get(&data.boss).unwrap().last_aware;

    for event in data.events.iter() {
        //event.pretty_print(&data.agents, &data.skills);

        match event.kind {
            EventKind::PhysDamage(PhysDamage {
                target, src, dmg, ..
            })
            | EventKind::CondDamage(CondDamage {
                target, src, dmg, ..
            }) => {
                if target != src && target == ctx.target && dmg > 0 {
                    println!("{} - {} dmg - {:?}", event.time, dmg, event.kind);
                    ctx.last_dmg = Some(event.time)
                }
            }

            EventKind::BuffApply(BuffApply { target, id, .. }) => {
                if target == ctx.target {
                    // if this is the second application in a row, ignore it
                    match id {
                        762 => {
                            if let Some((_, kind)) = ctx.invuln_changes.last() {
                                if !*kind {
                                    continue;
                                }
                            }
                            ctx.invuln_changes.push((event.time, false));
                        }
                        895 => {
                            if let Some((_, kind)) = ctx.other_invuln_changes.last() {
                                if !*kind {
                                    continue;
                                }
                            }
                            ctx.other_invuln_changes.push((event.time, false));
                        }
                        _ => {}
                    }
                }
            }

            EventKind::BuffRemove(BuffRemove { target, id, .. }) => {
                if target == ctx.target {
                    // if this is the second removal in a row, ignore it
                    match id {
                        762 => {
                            if let Some((_, kind)) = ctx.invuln_changes.last() {
                                if *kind {
                                    continue;
                                }
                            }
                            ctx.invuln_changes.push((event.time, true));
                        }
                        895 => {
                            if let Some((_, kind)) = ctx.other_invuln_changes.last() {
                                if *kind {
                                    continue;
                                }
                            }
                            ctx.other_invuln_changes.push((event.time, true));
                        }
                        _ => {}
                    }
                }
            }

            EventKind::CombatEnter(CombatEnter { target, .. }) => {
                ctx.combat_enters.insert(target, event.time);
            }

            EventKind::CombatExit(CombatExit { target, .. }) => {
                ctx.combat_exits.insert(target, event.time);
            }

            EventKind::CastStart(CastStart { skill, .. }) => {
                if skill > 50000 {
                    ctx.casts
                        .entry(skill)
                        .or_insert_with(Vec::new)
                        .push(event.time);
                }
            }

            EventKind::Death(Death { target, .. }) => {
                ctx.deaths.insert(target, event.time);
            }

            EventKind::Reward(_) if ctx.first_reward.is_none() => {
                ctx.first_reward = Some(event.time)
            }

            EventKind::Spawn(Spawn { target, .. }) => {
                ctx.spawns.insert(target, event.time);
            }

            _ => {}
        }
    }

    // adjust start time based on weird invuln application discrepancies in enso/mama logs
    // removing from beginning here on a vec is inefficient, queue would be better
    // but this array is tiny anyway so does it really matter?
    if matches!(target, Target::Enso | Target::Mama) {
        match ctx.invuln_changes.first() {
            Some((_, r)) if *r => ctx.start = ctx.invuln_changes.remove(0).0 + 1,
            Some((t, _)) if *t < 1500 => {
                ctx.invuln_changes.remove(0); // remove invuln gain
                ctx.invuln_changes.remove(0); // and subsequent removal
                ctx.start = ctx.combat_enters[&ctx.target] + 1
            }
            _ => {}
        }
    };

    ctx.success = match target {
        Target::Arts => check_success_by_invuln_count(&ctx, 4),
        Target::Arkk => check_success_by_invuln_count(&ctx, 10),
        Target::Ai => check_success_ai(&ctx, 0),
        _ => check_success(&ctx),
    };

    ctx
}

fn check_success_ai(ctx: &LogContext, start: Time) -> Option<Time> {
    if let Some((time, _)) = ctx
        .other_invuln_changes
        .iter()
        .find(|(t, r)| *t >= start && !*r)
    {
        Some(*time)
    } else {
        None
    }
}

fn check_success(ctx: &LogContext) -> Option<Time> {
    let dmg = ctx.last_dmg?;

    if let Some(reward) = ctx.first_reward {
        //println!("dmg: {}, reward: {}", dmg, reward);
        if (dmg as i64).saturating_sub(reward as i64).abs() < 100 {
            return Some(std::cmp::min(dmg, reward));
        }
    }

    if let Some(death) = ctx.deaths.get(&ctx.target) {
        //println!("death: {}, dmg: {}", *death, dmg);
        return Some(std::cmp::min(*death, dmg));
    }

    None
}

fn check_success_by_invuln_count(ctx: &LogContext, count: usize) -> Option<Time> {
    let last_is_removal = ctx.invuln_changes.last().map(|(_, is)| *is)?;

    if ctx.invuln_changes.len() == count && last_is_removal {
        check_success_by_combat_exit(ctx)
    } else {
        None
    }
}

fn check_success_by_combat_exit(ctx: &LogContext) -> Option<Time> {
    let last_dmg = ctx.last_dmg?;
    let target_enter = *ctx.combat_enters.get(&ctx.target)?;
    let target_exit = *ctx.combat_exits.get(&ctx.target)?;
    let player_exit = ctx
        .players
        .iter()
        .filter_map(|id| ctx.combat_exits.get(id).cloned())
        .filter(|t| *t > target_exit) // ignore cases where someone dies early
        .max();

    println!(
        "last_dmg: {}, target_enter: {}, target_exit: {}, player_exit: {}",
        last_dmg,
        target_enter,
        target_exit,
        player_exit.unwrap()
    );
    if let Some(player_exit) = player_exit {
        if player_exit > (target_exit + 1000) && target_exit > target_enter {
            return Some(last_dmg);
        }
    } else if ctx.last_event > ctx.last_aware + 2000 {
        return Some(last_dmg);
    }
    None
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_parses_logs_correctly() {
        use std::collections::HashMap;
        use std::fs;

        let test_logs: HashMap<_, _> = [
            ("20201029-171322", vec![(false, vec![18798, 7101, 11697])]), // Siax
            (
                "20200831-011339",
                vec![(true, vec![74338, 4211, 15807, 16358, 10962])],
            ), // MAMA
            (
                "20200907-020134",
                vec![(true, vec![72067, 10670, 21207, 17689])],
            ), // Arts
            (
                "20200904-013857",
                vec![(true, vec![47959, 3758, 10209, 6284, 9799])],
            ), // MAMA
            (
                "20200901-035740",
                vec![(true, vec![120597, 7225, 5633, 11272, 8247, 9684, 17769])],
            ), // Arkk
            (
                "20200722-145459",
                vec![(true, vec![39472, 6713, 8960, 10670])],
            ), // Siax
            (
                "20200722-145912",
                vec![(true, vec![144288, 19370, 21663, 27298])],
            ), // Enso
            (
                "20200429-192541",
                vec![(true, vec![103097, 6559, 4962, 10079, 5905, 10914, 13285])],
            ), // Arkk
            (
                "20200411-225507",
                vec![(false, vec![315557, 161820, 71097])],
            ), // MAMA
            (
                "20200628-191224",
                vec![(true, vec![48832, 3155, 9922, 7876, 11801])],
            ), // MAMA
            (
                "20201030-200434",
                vec![(true, vec![128904, 8125, 5855, 22401, 5200, 11592, 19349])],
            ), // Arkk
            (
                "20201103-191913",
                vec![(true, vec![147053, 11975, 29520, 16597])],
            ), // Skorvald
            (
                "20200918-163859",
                vec![
                    (true, vec![377403, 89833, 106499, 103849]),
                    (true, vec![270329, 73282, 72319, 124725]),
                ],
            ), // Ai (both forms)
            (
                "20201030-183345",
                vec![(false, vec![192546, 61050, 54764, 76729])],
            ), // Ai (dark form)
            ("20201018-221051", vec![(false, vec![5744])]),               // Ai (elemental form)
            (
                "20201030-182828",
                vec![(true, vec![242048, 41976, 73273, 52366])],
            ), // Ai (elemental form)
            (
                "20201030-183850",
                vec![(true, vec![188975, 58057, 72930, 57985])],
            ), // Ai (dark form)
            (
                "20201024-180052",
                vec![(true, vec![184963, 63038, 55310, 66612])],
            ), // Ai (dark form)
            ("20201103-193702", vec![(false, vec![64580, 15898, 38654])]), // Siax
        ]
        .iter()
        .cloned()
        .collect();

        for f in fs::read_dir("tests/evil_logs").unwrap() {
            let path = f.unwrap().path();
            if path.extension().unwrap() != "zevtc" && path.extension().unwrap() != "evtc" {
                continue;
            }
            let log = crate::log::Log::from_file_checked(path).unwrap();

            println!("log: {:?}", &log);
            let (encounters, _) = super::parse(&log).unwrap();
            for (actual, (expected_success, expected_phases)) in
                encounters.iter().zip(test_logs[log.id().as_str()].iter())
            {
                println!("Success: {} vs {}", actual.success, *expected_success);
                assert_eq!(actual.success, *expected_success);
                assert_eq!(actual.phases.len(), expected_phases.len());
                for (actual_phase, expected_phase) in actual
                    .phases
                    .iter()
                    .map(|p| p.duration())
                    .zip(expected_phases.iter())
                {
                    println!("Phase: {} vs {}", actual_phase, *expected_phase);
                    assert_eq!(actual_phase, *expected_phase);
                }
            }
        }
    }
}
