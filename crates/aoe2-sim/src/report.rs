//! Textos legibles de una simulación en el idioma elegido.

use crate::scenario::Scenario;
use crate::timeline::{Event, IdleCause};
use aoe2_data::Locale;

pub fn idle_cause_key(c: IdleCause) -> &'static str {
    match c {
        IdleCause::Housed => "housed",
        IdleCause::NoFood => "no_food",
        IdleCause::WaitingResearch => "waiting_research",
        IdleCause::NoOrder => "no_order",
    }
}

pub fn idle_cause(l: &Locale, c: IdleCause) -> String {
    l.t(&format!("idle.{}", idle_cause_key(c)))
}

/// Nombre traducido de un edificio por su nombre corto de rules.toml.
pub fn building(sc: &Scenario, l: &Locale, name: &str) -> String {
    sc.building_ref(name).map(|id| l.building(&sc.data, id)).unwrap_or_else(|| name.to_owned())
}

pub fn tech(sc: &Scenario, l: &Locale, name: &str) -> String {
    sc.tech_ref(name).map(|id| l.tech(&sc.data, id)).unwrap_or_else(|| name.to_owned())
}

/// Etiqueta de un pool o de "construir X → pool" (formato de VillagerCreated).
pub fn assignment(sc: &Scenario, l: &Locale, a: &str) -> String {
    match a.split_once(" → ") {
        Some((b, then)) => format!("{} → {}", building(sc, l, b), l.pool(then)),
        None => l.pool(a),
    }
}

/// Descripción de un evento; `None` para los que no vale la pena mostrar
/// (granjas, que se construyen y resiembran sin parar).
pub fn describe(sc: &Scenario, l: &Locale, e: &Event) -> Option<String> {
    Some(match e {
        Event::VillagerCreated { villager, assignment: a } => {
            let n = villager.to_string();
            match a.split_once(" → ") {
                Some((b, then)) => l.fmt(
                    "events.villager_builds",
                    &[("n", &n), ("building", &building(sc, l, b)), ("task", &l.pool(then))],
                ),
                None => l.fmt("events.villager_created", &[("n", &n), ("task", &l.pool(a))]),
            }
        }
        Event::BuildStarted { building: b, builders, .. } if b != "farm" => l.fmt(
            "events.build_started",
            &[("building", &building(sc, l, b)), ("builders", &builders.len().to_string())],
        ),
        Event::BuildFinished { building: b } if b != "farm" => {
            l.fmt("events.build_finished", &[("building", &building(sc, l, b))])
        }
        Event::ResearchStarted { tech: t } => l.fmt("events.research_started", &[("tech", &tech(sc, l, t))]),
        Event::ResearchFinished { tech: t } => l.fmt("events.research_finished", &[("tech", &tech(sc, l, t))]),
        Event::AgeReached { age } => l.fmt("events.age_reached", &[("age", &l.age(*age))]),
        Event::Moved { villagers, from, to } => l.fmt(
            "events.moved",
            &[("count", &villagers.len().to_string()), ("from", &l.pool(from)), ("to", &l.pool(to))],
        ),
        Event::PoolExhausted { pool } => l.fmt("events.pool_exhausted", &[("pool", &l.pool(pool))]),
        Event::Lured { source, villager } => {
            let pool = sc.map.sources.iter().find(|s| &s.id == source).map(|s| s.pool()).unwrap_or(source);
            l.fmt("events.lured", &[("n", &villager.to_string()), ("source", &l.pool(pool))])
        }
        Event::TcIdle { cause, seconds, .. } => l.fmt(
            "events.tc_idle",
            &[("seconds", &seconds.to_string()), ("cause", &idle_cause(l, *cause))],
        ),
        _ => return None,
    })
}
