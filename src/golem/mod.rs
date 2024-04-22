pub mod cast;
pub mod character;
pub mod modifier;
pub mod skill;

use crate::parse::AgentId;
use crate::parse::Event;
use crate::parse::EventKind::*;
use crate::parse::Time;
use character::*;
use modifier::Modifier;
use skill::Skill;

use std::collections::HashMap;

const SIC_EM_CD: u64 = 22400;
const SIC_EM_ID: i32 = 33902;

#[derive(Debug, Copy, Clone)]
struct Stats {
    mainhand: Weapon,
    power: u32,
    ferocity: u32,
}

#[derive(Debug, Copy, Clone)]
enum Sim {
    Min,
    Avg,
    Max,
}

fn damage(kind: Sim, skill: Skill, stats: Stats, extra_mods: &[Modifier]) -> i64 {
    let armor = 2597;

    let mut mods = vec![modifier::vuln(25)];

    if !skill.uncrittable() {
        mods.push(modifier::crit_dmg(stats.ferocity));
    }

    let weapon = match skill.weapon() {
        skill::WeaponKind::Fixed(weapon) => weapon,
        skill::WeaponKind::Current => stats.mainhand,
    };

    mods.extend(extra_mods);
    let multiplier = modifier::sum(&mods);

    let strength = match kind {
        Sim::Min => weapon.strn().min(),
        Sim::Avg => weapon.strn().avg(),
        Sim::Max => weapon.strn().max(),
    };

    (skill.coefficient() * strength * stats.power as f64 * multiplier / armor as f64) as i64
}

fn sorted_events(mut events: Vec<Event>) -> Vec<Event> {
    use std::cmp::Ordering;

    events.sort_by(|a, b| match a.time.cmp(&b.time) {
        Ordering::Equal => match (&a.kind, &b.kind) {
            (WeaponSwap(_), WeaponSwap(_)) => Ordering::Equal,
            (WeaponSwap(_), PhysDamage(_)) => Ordering::Less,
            (PhysDamage(_), WeaponSwap(_)) => Ordering::Greater,
            (WeaponSwap(_), _) => Ordering::Less,
            (_, WeaponSwap(_)) => Ordering::Greater,
            (PhysDamage(_), PhysDamage(_)) => Ordering::Equal,
            (PhysDamage(_), _) => Ordering::Less,
            (_, PhysDamage(_)) => Ordering::Greater,
            _ => Ordering::Equal,
        },
        otherwise => otherwise,
    });
    events
}

fn fake_event(events: &[Event], my_char: &Character, skill: Skill) -> Option<Event> {
    events
        .iter()
        .find(|e| matches!( &e.kind, PhysDamage(inner) if inner.skill == skill.id()))
        .cloned()
        .map(|mut e| {
            e.time = 0;
            if let PhysDamage(ref mut inner) = e.kind {
                let (_, avg, _) = damage_for(my_char, skill);
                inner.dmg = avg as i32;
            }
            e
        })
}

pub fn sic_em_times(log: crate::log::Log) {
    let (_, data) = crate::parse::parse(&log).unwrap();
    let mut my_char = my_char();

    let me = data.id_for(".4623").unwrap();
    let cutoff = last_relevant_time(&data.events);

    let mut events: Vec<_> = sorted_events(data.events)
        .into_iter()
        .take_while(|e| e.time <= cutoff)
        .collect();

    events.insert(0, fake_event(&events, &my_char, Skill::Barrage).unwrap());
    if let Some(event) = fake_event(&events, &my_char, Skill::LightningStrike) {
        events.insert(1, event);
    }

    let mut highest_damag = (0, 0);
    let mut sic_ems = HashMap::new();
    my_char.remove_buff(Buff::SicEm);
    for (i, event) in events.iter().enumerate() {
        //if !matches!(&event.kind, PhysDamage(_)) { continue; }

        my_char = apply_event(event, my_char, me);
        let mut forward_sim_char = my_char.clone();

        // TODO: what about TaV being added in this forward sim but not removed?
        let mut sic_em_damage = 0;
        for event in events[i..]
            .iter()
            .take_while(|e| e.time <= event.time + 9800)
        {
            forward_sim_char = apply_event(event, forward_sim_char, me);
            if let PhysDamage(inner) = &event.kind {
                let skill = Skill::from_id(inner.skill).unwrap();

                forward_sim_char.add_buff(Buff::SicEm);
                let (_, avg, _) = damage_for(&forward_sim_char, skill);
                forward_sim_char.remove_buff(Buff::SicEm);
                let (_, avg_without, _) = damage_for(&forward_sim_char, skill);
                sic_em_damage += avg - avg_without;
            }
        }

        // only want this to update if damage is higher
        if let Some(damage) = sic_ems.get(&event.time) {
            if *damage < sic_em_damage {
                sic_ems.insert(event.time, sic_em_damage);
            }
        } else {
            sic_ems.insert(event.time, sic_em_damage);
        }

        if sic_em_damage > highest_damag.1 {
            highest_damag = (event.time, sic_em_damage);
        }
    }

    let mut sic_ems_vec: Vec<_> = sic_ems.iter().map(|(t, d)| (*t, *d)).collect();
    sic_ems_vec.sort_by_key(|v| v.0);

    let weaponswap_times = events
        .iter()
        .filter_map(|e| match &e.kind {
            WeaponSwap(_) => Some(e.time),
            _ => None,
        })
        .enumerate()
        .filter(|(i, _)| i % 2 == 1)
        .map(|(_, x)| {
            sic_ems_vec
                .iter()
                .enumerate()
                .find(|(_, (t, _))| *t > x)
                .map(|x| x.0)
                .unwrap()
        });
    let loops: Vec<_> = std::iter::once(0 as usize)
        .chain(weaponswap_times.clone())
        .zip(weaponswap_times.chain(std::iter::once(sic_ems_vec.len() - 1)))
        .collect();

    let currents: Vec<_> = std::iter::repeat((0, 0)).take(loops.len()).collect();
    let optimals = find_optimal_sic_em(&sic_ems_vec[..], loops, currents);

    for (t, dmg) in optimals {
        let mut printed = false;
        println!();

        for (time, e) in events
            .iter()
            .filter_map(|e| match &e.kind {
                PhysDamage(inner) => Some((e.time, inner)),
                _ => None,
            })
            .skip_while(|(time, _)| *time < t.saturating_sub(2000))
            .take_while(|(time, _)| *time < t + 9800)
            .filter(|(time, _)| (*time <= t + 2000) || (*time > t + 8000))
        {
            if time >= t && !printed {
                println!("{} -- \"Sic 'Em!\" {} --", t, dmg);
                printed = true;
            }
            let skill = Skill::from_id(e.skill).unwrap();
            println!("{} {}", time, skill.name());
        }
    }
}

fn find_optimal_sic_em(
    sic_ems: &[(Time, i64)],
    loops: Vec<(usize, usize)>,
    mut currents: Vec<(Time, i64)>,
) -> Vec<(Time, i64)> {
    if loops.is_empty() {
        unreachable!();
    }

    if loops.len() == 1 {
        return vec![*sic_ems[loops[0].0..loops[0].1]
            .iter()
            .max_by_key(|(_, d)| d)
            .unwrap()];
    }

    let mut highests: Vec<(Time, i64)> = Vec::new();
    highests.resize(loops.len(), (0, 0));

    for i in loops[0].0..loops[0].1 {
        currents[0] = sic_ems[i];
        let sic_em_time = currents[0].0 + SIC_EM_CD;
        if sic_em_time > currents[1].0 {
            let mut other_loops: Vec<_> = loops.iter().skip(1).cloned().collect();
            let other_currents: Vec<_> = currents.iter().skip(1).cloned().collect();
            if let Some(se) = sic_ems[other_loops[0].0..other_loops[0].1]
                .iter()
                .enumerate()
                .find(|(_, (t, _))| *t >= sic_em_time)
            {
                other_loops[0].0 += se.0;
                let mut others = find_optimal_sic_em(sic_ems, other_loops, other_currents);
                others.insert(0, currents[0]);
                currents = others;
            } else {
                continue;
            }
        }

        if currents.iter().map(|(_, d)| d).sum::<i64>()
            > highests.iter().map(|(_, d)| d).sum::<i64>()
        {
            highests = currents.clone();
        }
    }

    highests
}

fn apply_event(
    event: &crate::parse::Event,
    mut character: Character,
    char_id: AgentId,
) -> Character {
    use crate::parse::EventKind::*;

    match &event.kind {
        BuffApply(inner) if inner.target == char_id => {
            if let Some(buff) = Buff::from_id(inner.id) {
                character.add_buff(buff);
            }
        }
        BuffRemove(inner) if inner.target == char_id => {
            if let Some(buff) = Buff::from_id(inner.id) {
                character.remove_buff(buff);
            }
        }
        WeaponSwap(inner) if inner.target == char_id => {
            character.swap_weapons();
        }

        _ => {}
    }

    character
}

fn impact_difference() -> f64 {
    use modifier::Fixed::*;

    let mut additive_mods = vec![
        FrostSpirit.val(),
        FuriousStrength.val(),
        TwiceAsVicious.val(),
        Force.val(),
    ];
    let sum = modifier::sum(&additive_mods);

    additive_mods.push(Impact.val());
    let sum_with_impact = modifier::sum(&additive_mods);

    sum_with_impact / sum
}

fn buff_expires_first(events: &[Event], buff: Buff) -> bool {
    events
        .iter()
        .find(|e| match &e.kind {
            BuffRemove(inner) if inner.id == buff.id().unwrap() => true,
            BuffApply(inner) if inner.id == buff.id().unwrap() => true,
            _ => false,
        })
        .filter(|e| matches!(&e.kind, BuffRemove(inner) if inner.id == buff.id().unwrap()))
        .is_some()
}

fn last_relevant_time(events: &[Event]) -> Time {
    events
        .iter()
        .rev()
        .find(
            |e| matches!(&e.kind, PhysDamage(inner) if inner.skill == Skill::WhirlingDefense.id()),
        )
        .map(|e| e.time)
        .unwrap_or_else(u64::max_value)
}

fn damage_for(character: &Character, skill: Skill) -> (i64, i64, i64) {
    let stats = character.get_stats();
    let stats = Stats {
        mainhand: character.current_weapons().mainhand(),
        power: stats.power(),
        ferocity: stats.ferocity(),
    };
    let min = damage(Sim::Min, skill, stats, &character.modifiers());
    let avg = damage(Sim::Avg, skill, stats, &character.modifiers());
    let max = damage(Sim::Max, skill, stats, &character.modifiers());
    (min, avg, max)
}

fn debug_events(data: &crate::parse::Data) {
    // 40642
    for event in data
        .events
        .iter()
        .filter(|e| !matches!(&e.kind, CondDamage(_) | BuffApply(_) | BuffRemove(_)))
    // .filter(|e| matches!(&e.kind, CastStart(_) | PhysDamage(_)))
    // .filter(|e| match &e.kind {
    // PhysDamage(inner) if inner.skill == Skill::Barrage.id() => true,
    // CastStart(inner) if inner.skill == Skill::Barrage.id() => true,
    // _ => false,
    // })
    //.filter(|e| match &e.kind {
    //BuffApply(inner) if inner.id == 40642 => true,
    //BuffRemove(inner) if inner.id == 40642 => true,
    //PhysDamage(_) => true,
    //PhysDamage(inner) if inner.skill == Skill::OneWolfPack.id() => true,
    //_ => false,
    //})
    //.take(1000)
    {
        event.pretty_print(&data.agents, &data.skills);
    }
}

#[derive(Debug, Clone)]
struct SkillUse {
    time: Time,
    skill: Skill,
    dmg: i32,
    sim: i64,
}

// Damage done on this weapon, by skill
type SkillData = HashMap<Skill, Vec<SkillUse>>;
type ParsedData = HashMap<character::Weapon, Vec<SkillData>>;

fn analyse_skill(duration: Time, skill: Skill, data: &[SkillData]) {
    let mut total_procs = 0;
    let mut total_skill = 0;
    let mut total_impact = 0;

    for (n, phase) in data.iter().enumerate() {
        let total: i32 = phase.iter().map(|(_, v)| v).flatten().map(|u| u.dmg).sum();
        let uses = phase.get(&skill).unwrap();
        let procs = uses.len();
        total_procs += procs;
        let dmg: i32 = uses.iter().map(|u| u.dmg).sum();
        total_skill += dmg;
        if procs == 0 {
            println!("No instances of {:?}", skill);
            return;
        }

        let without_skill = total - dmg;
        let if_impact: i32 = (without_skill as f64 * impact_difference()) as i32;
        let impact_gain = if_impact - without_skill;
        total_impact += impact_gain;
        let diff = total - if_impact;
        println!(
            "p{}: {} in {} proc(s) vs {} impact dmg: diff of {}",
            n, dmg, procs, impact_gain, diff
        );
    }

    let diff = total_skill - total_impact;
    let dps = (diff as f64) / (duration as f64 / 1000.);
    println!(
        "{} dmg in {} proc(s) vs {} impact dmg: {:.3} dps diff",
        total_skill, total_procs, total_impact, dps
    );
}

fn fmt_skills(skills: impl Iterator<Item = Skill> + Clone) -> String {
    let mut out = String::new();
    let len = skills.clone().count();
    for (i, s) in skills.enumerate() {
        out = format!("{}{}", out, s.name());
        if i != len - 1 {
            out = format!("{} - ", out);
        }
    }
    out
}

#[derive(Default)]
struct SicEm {
    time: Time,
    expiry: Time,
    skills: Vec<SkillUse>,
}

fn analyse_sic_em(casts: &[SicEm]) {
    const SIC_EM_MOD: f64 = 1.4;

    for (n, sic_em) in casts.iter().enumerate() {
        let total_damage: i32 = sic_em.skills.iter().map(|s| s.dmg).sum();
        let est_damage: i64 = sic_em.skills.iter().map(|s| s.sim).sum();
        let nwp_damage: i32 = sic_em
            .skills
            .iter()
            .filter(|s| s.skill != Skill::OneWolfPack)
            .map(|s| s.dmg)
            .sum();

        let diff = (total_damage as f64 - total_damage as f64 / SIC_EM_MOD) as i64;
        let est_diff = (est_damage as f64 - est_damage as f64 / SIC_EM_MOD) as i64;
        let nwp_diff = (nwp_damage as f64 - nwp_damage as f64 / SIC_EM_MOD) as i64;

        println!(
            "\n\"Sic 'Em!\" {}: act {} ({}); est {} ({}); no owp {}; rng {}",
            n + 1,
            total_damage,
            diff,
            est_damage,
            est_diff,
            nwp_diff,
            est_diff - diff,
        );

        let ms_early = sic_em.skills.first().unwrap().time - sic_em.time;
        let ms_after = sic_em.expiry - sic_em.skills.last().unwrap().time;

        let first_skills = sic_em.skills.iter().take(5).map(|s| s.skill);
        let last_skills = sic_em.skills.iter().rev().take(5).rev().map(|s| s.skill);
        print!("{}ms -> ", ms_early);
        print!("{}", fmt_skills(first_skills));
        print!(" .. ");
        print!("{}", fmt_skills(last_skills));
        println!(" -> {}ms", ms_after);
    }
}

// axe 3: 500ms
// ricochet: 600ms

pub fn rotation(log: &crate::log::Log) {
    let (_, data) = crate::parse::parse(log).unwrap();

    let mut casts: Vec<cast::Cast> = Vec::new();
    let mut current_cast: Option<(Time, cast::Kind)> = None;

    for event in data.events.iter().filter(|e| match &e.kind {
        CastStart(_) | CastEnd(_) | CastCFire(_) | CastCancel(_) => true,
        _ => false,
    }) {
        event.pretty_print(&data.agents, &data.skills);
        match &event.kind {
            CastStart(inner) => {
                println!("{}", inner.skill);
                let skill = cast::Kind::from_id(inner.skill).unwrap();
                if let cast::Kind::Dodge = skill {
                    continue;
                }

                match current_cast {
                    Some(_) => panic!("current cast should be empty"),
                    None => {
                        current_cast = Some((event.time, skill));
                    }
                }
            }

            CastEnd(inner) => {
                println!("{}", inner.skill);
                let skill = cast::Kind::from_id(inner.skill).unwrap();
                if let cast::Kind::Dodge = skill {
                    continue;
                }
                match current_cast {
                    None => panic!("cast end: current cast empty"),
                    Some((time, cast)) => {
                        if cast == skill {
                            casts.push(cast::Cast::new(
                                skill,
                                cast::Status::Normal,
                                event.time - time,
                            ));
                            current_cast = None;
                        } else {
                            panic!("ended wrong skill?");
                        }
                    }
                }
            }

            CastCFire(inner) => {
                println!("{}", inner.skill);
                let skill = cast::Kind::from_id(inner.skill).unwrap();
                match current_cast {
                    None => {
                        println!("cast cfire without cast start");
                        casts.push(cast::Cast::new(skill, cast::Status::Normal, event.time));
                    }
                    Some((time, cast)) => {
                        if cast == skill {
                            casts.push(cast::Cast::new(
                                skill,
                                cast::Status::Normal,
                                event.time - time,
                            ));
                            current_cast = None;
                        } else {
                            panic!("ended wrong skill?");
                        }
                    }
                }
            }

            CastCancel(inner) => {
                println!("{}", inner.skill);
                let skill = cast::Kind::from_id(inner.skill).unwrap();
                match current_cast {
                    None => panic!("cast cancel: current cast empty"),
                    Some((time, cast)) => {
                        if cast == skill {
                            casts.push(cast::Cast::new(
                                skill,
                                cast::Status::Normal,
                                event.time - time,
                            ));
                            current_cast = None;
                        } else {
                            panic!("ended wrong skill?");
                        }
                    }
                }
            }
            _ => unreachable!(),
        }
    }
    println!("{:#?}", casts);

    use cast::Kind::*;
    use cast::Rotation;
    use skill::Skill::*;

    let mut sword_char = my_char();
    sword_char.set_mainhand(character::Weapons::Second, character::Weapon::Sword);
    let mut axe_char = my_char();
    axe_char.set_mainhand(character::Weapons::Second, character::Weapon::Axe);

    let sword_test = vec![
        Skill(Slash),
        Skill(CripplingThrust),
        Skill(PrecisionSwipe),
        Skill(Slash),
        Skill(CripplingThrust),
        Skill(PrecisionSwipe),
        Skill(Slash),
        Skill(CripplingThrust),
        Skill(PrecisionSwipe),
    ];

    let axe_test = vec![
        Skill(WintersBite),
        Skill(Ricochet),
        Skill(Ricochet),
        Skill(Ricochet),
        Skill(Ricochet),
        Skill(Ricochet),
        Skill(Ricochet),
        Skill(Ricochet),
        Skill(Ricochet),
    ];

    let mut things: Vec<(String, (Time, i64))> = Vec::new();
    for i in 1..sword_test.len() + 1 {
        let sword_rota = Rotation {
            order: sword_test[..i].to_vec(),
        };
        let axe_rota = Rotation {
            order: axe_test[..i].to_vec(),
        };

        let sword_name = format!("{} sword auto(s)", i);
        let axe_name = format!("winter's bite & {} axe auto(s)", i - 1);

        things.push((sword_name, damage_for_rota(sword_rota, &sword_char)));
        things.push((axe_name, damage_for_rota(axe_rota, &axe_char)));
    }
    things.sort_by_key(|(_, (length, _))| *length);

    for (name, (length, damage)) in things {
        println!(
            "{:32}{}ms for {} dmg ({:.0} dps)",
            name,
            length,
            damage,
            damage as f64 / (length as f64 / 1000.0)
        );
    }
}

fn damage_for_rota(rotation: cast::Rotation, character: &character::Character) -> (Time, i64) {
    let damag: i64 = rotation
        .order
        .iter()
        .filter_map(|s| {
            if let cast::Kind::Skill(skill) = s {
                Some(skill)
            } else {
                None
            }
        })
        .map(|s| damage_for(character, *s))
        .map(|(_, avg, _)| avg)
        .sum();
    println!("{}ms for {} dmg", rotation.total_time(), damag);
    (rotation.total_time(), damag)
}

pub fn owp_testing(log: &crate::log::Log) {
    use crate::parse::EventKind::*;
    let (_, data) = crate::parse::parse(log).unwrap();

    let me = data.id_for(".4623").unwrap();

    let mut events = sorted_events(data.events);
    let mut my_char = my_char();

    events.insert(0, fake_event(&events, &my_char, Skill::Barrage).unwrap());
    if let Some(event) = fake_event(&events, &my_char, Skill::LightningStrike) {
        events.insert(1, event);
    }

    let cutoff = last_relevant_time(&events);

    if buff_expires_first(&events[..], Buff::OneWolfPack) {
        my_char.add_buff(Buff::OneWolfPack);
    }

    #[derive(Debug, Default, Clone)]
    struct OwpUse {
        hits: u8,
        duration: u64,
        downtime: u64,
    }

    let mut owp_uses = Vec::new();
    let mut owp_use = OwpUse::default();

    let mut last_application = 0;
    let mut last_removal = 0;
    let mut last_was_apply = false;
    for event in events.iter().take_while(|e| e.time <= cutoff) {
        my_char = apply_event(&event, my_char, me);

        match &event.kind {
            BuffApply(inner) if inner.id == Buff::OneWolfPackIcd.id().unwrap() => {
                event.pretty_print(&data.agents, &data.skills);
                if !last_was_apply {
                    println!("Adding {} downtime\n", event.time - last_removal);
                    owp_use.downtime += event.time - last_removal;
                } else {
                    // two apply events in a row, means no downtime?
                    println!("Two apply events in a row\n");
                    println!("Time since last: {}", event.time - last_application);
                }
                last_was_apply = true;
                last_application = event.time;
            }

            BuffRemove(inner) if inner.id == Buff::OneWolfPackIcd.id().unwrap() => {
                println!("Time since last: {}", event.time - last_application);
                event.pretty_print(&data.agents, &data.skills);
                owp_use.hits += 1;
                last_removal = event.time;
                last_was_apply = false;
            }

            BuffApply(inner) if inner.id == Buff::OneWolfPack.id().unwrap() => {
                event.pretty_print(&data.agents, &data.skills);

                owp_uses.push(owp_use);
                owp_use = OwpUse::default();
                owp_use.duration = event.time;

                last_removal = event.time;
            }
            BuffRemove(inner) if inner.id == Buff::OneWolfPack.id().unwrap() => {
                event.pretty_print(&data.agents, &data.skills);
                owp_use.duration = event.time - owp_use.duration;
            }

            PhysDamage(_) => {
                if my_char.has_buff(Buff::OneWolfPack) {
                    event.pretty_print(&data.agents, &data.skills);
                }
            }
            _ => {}
        }
    }

    if my_char.has_buff(Buff::OneWolfPack) {
        owp_use.duration = cutoff - owp_use.duration;
    }
    owp_uses.push(owp_use);

    for (i, owp) in owp_uses.iter().enumerate() {
        let utilisation = (owp.duration - owp.downtime) as f64 / owp.duration as f64;
        println!("\nOWP #{}", i);
        println!("OWP duration: {}ms", owp.duration);
        println!("hits: {}", owp.hits);
        println!(
            "theoretical max hits: {}",
            (owp.duration as f64 / 250.).floor()
        );
        println!("downtime: {}ms", owp.downtime);
        println!("OWP \"utilisation\": {:.2}%", utilisation * 100.);
    }
}

pub fn golem(log: &crate::log::Log) {
    use crate::parse::EventKind::*;
    let (encounters, data) = crate::parse::parse(log).unwrap();
    let encounter = encounters.get(0).unwrap();
    debug_events(&data);
    panic!();

    let me = data.id_for(".4623").unwrap();
    let mut events = sorted_events(data.events);
    let mut my_char = my_char();
    let mut parsed_data = ParsedData::new();
    let mut current_data = SkillData::new();

    let mut sic_em_casts = Vec::new();
    let mut sic_em = SicEm::default();
    let mut total_condi_dmg = 0;

    if buff_expires_first(&events[..], Buff::SicEm) {
        my_char.add_buff(Buff::SicEm);
    }

    events.insert(0, fake_event(&events, &my_char, Skill::Barrage).unwrap());
    if let Some(event) = fake_event(&events, &my_char, Skill::LightningStrike) {
        events.insert(1, event);
    }

    let cutoff = last_relevant_time(&events);
    for i in 0..events.iter().take_while(|e| e.time <= cutoff).count() {
        my_char = apply_event(&events[i], my_char, me);

        match &events[i].kind {
            BuffApply(inner) => {
                if inner.id == SIC_EM_ID {
                    sic_em.time = events[i].time;
                }
            }

            BuffRemove(inner) => {
                if inner.id == SIC_EM_ID {
                    sic_em.expiry = events[i].time;
                    sic_em_casts.push(sic_em);
                    sic_em = SicEm::default();
                }
            }

            WeaponSwap(_) => {
                parsed_data
                    .entry(my_char.alternate_mainhand())
                    .or_insert_with(Vec::new)
                    .push(current_data);
                current_data = SkillData::new();
            }

            CondDamage(inner) => {
                if inner.src == me {
                    total_condi_dmg += inner.dmg;
                }
            }

            PhysDamage(inner) => {
                if inner.dmg == 0 {
                    continue;
                }

                let skill = Skill::from_id(inner.skill).unwrap();

                let (min, avg, max) = damage_for(&my_char, skill);
                if i64::from(inner.dmg) > max || i64::from(inner.dmg) < min {
                    // hacky fallback, just come back to this event later?
                    let current_event = events.remove(i);
                    let mut j = i;
                    while events[j].time == current_event.time {
                        j += 1;
                    }
                    if j == i + 1 {
                        panic!();
                    }
                    events.insert(j, current_event);
                    println!("FALLBACK: moved this event at {} to {}", i, j);
                    continue;
                }

                assert!(i64::from(inner.dmg) >= min);
                assert!(i64::from(inner.dmg) <= max);

                let skill_use = SkillUse {
                    time: events[i].time,
                    skill,
                    dmg: inner.dmg,
                    sim: avg,
                };

                current_data
                    .entry(skill)
                    .or_insert_with(Vec::new)
                    .push(skill_use.clone());

                if my_char.has_buff(Buff::SicEm) {
                    sic_em.skills.push(skill_use);
                }
            }
            _ => {}
        }
    }

    if !current_data.is_empty() {
        parsed_data
            .entry(my_char.current_mainhand())
            .or_insert_with(Vec::new)
            .push(current_data);
    }

    if my_char.has_buff(Buff::SicEm) {
        sic_em.expiry = encounter.phases[0].end();
        sic_em_casts.push(sic_em);
    }

    analyse_skill(
        encounter.phases[0].duration(),
        Skill::LightningStrike,
        parsed_data.get(&character::Weapon::Longbow).unwrap(),
    );
    analyse_skill(
        encounter.phases[0].duration(),
        Skill::FrostBurst,
        parsed_data.get(&character::Weapon::Dagger).unwrap(),
    );
    analyse_sic_em(&sic_em_casts);

    let (total_dmg, total_est): (i32, i64) = parsed_data
        .iter()
        .flat_map(|(_, v)| v)
        .flatten()
        .flat_map(|(_, v)| v)
        .map(|u| (u.dmg, u.sim))
        .fold((0, 0), |(a, b), (c, d)| (a + c, b + d));

    println!("phys damage: {}", total_dmg);
    println!("cond damag: {}", total_condi_dmg);
    println!("simulated phys damage: {}", total_est);
    println!("phys damage from rng: {}", total_dmg as i64 - total_est);
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn damage() {
        let mut my_char = my_char();
        my_char.add_buff(Buff::SicEm);

        let stats = my_char.get_stats();
        let lb_stats = super::Stats {
            mainhand: my_char.current_weapons().mainhand(),
            power: stats.power(),
            ferocity: stats.ferocity(),
        };

        my_char.swap_weapons();
        let stats = my_char.get_stats();
        let axe_stats = super::Stats {
            mainhand: my_char.current_weapons().mainhand(),
            power: stats.power(),
            ferocity: stats.ferocity(),
        };

        println!("{:#?}", my_char);
        println!("{:?}", stats);

        use Skill::*;
        let stuff = vec![
            ("lb5 (on lb)", Barrage, lb_stats),
            ("lb5 (on axe)", Barrage, axe_stats),
            ("axe5", WhirlingDefense, axe_stats),
            ("f3 (on lb)", WorldlyImpact, lb_stats),
            ("f3 (on dagger)", WorldlyImpact, axe_stats),
            ("lb2", RapidFire, lb_stats),
            //("lb1", 0.7, longbow.min, lb_stats),
            ("lb1", LongRangeShot, lb_stats),
            ("hydro", FrostBurst, axe_stats),
            ("air", LightningStrike, lb_stats),
            ("owp (on lb)", OneWolfPack, lb_stats),
            ("owp (on axe)", OneWolfPack, axe_stats),
            ("trap (on lb)", FrostTrap, lb_stats),
            ("trap (on axe)", FrostTrap, axe_stats),
            ("f2 (on lb)", FrenziedAttack, lb_stats),
            //("lb1", 0.7, longbow.max, lb_stats),
        ];

        let mods = my_char.modifiers();

        println!("{:?}", mods);
        for (name, skill, stats) in stuff {
            println!("{}: {}", name, damage(Sim::Avg, skill, stats, &mods));
        }
    }

    #[test]
    fn rotation_test() {
        let path = "tests/example_logs/Standard Kitty Golem/20201019-114937.zevtc";

        let log = crate::log::Log::from_file_checked(path).unwrap();
        rotation(&log);
    }

    #[test]
    fn owp() {
        let path = "tests/example_logs/Standard Kitty Golem/20201019-114937.zevtc";

        let log = crate::log::Log::from_file_checked(path).unwrap();
        owp_testing(&log);
    }

    #[test]
    fn single_golem() {
        let path = "tests/example_logs/Standard Kitty Golem/20201029-133341.zevtc";
        let path = "tests/example_logs/MAMA/20191015-100520.zevtc";

        let log = crate::log::Log::from_file_checked(path).unwrap();
        golem(&log);
        //sic_em_times(log);
    }
}
