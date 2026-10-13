//! Arabia neutra: determinismo, reglas de edad y validación de la FC 27+2.

mod common;

use aoe2_data::Age;
use aoe2_sim::{Event, Stop, simulate};
use serde::Deserialize;
use std::path::Path;

const FC: (&str, &str) = ("arabia_neutra", "fast_castle_27_2");

fn run_fc() -> aoe2_sim::Timeline {
    let sc = common::scenario(FC.0, FC.1);
    simulate(&sc, Stop { max_seconds: 30 * 60, age: Some(Age::Castle) }).unwrap()
}

#[test]
fn simulation_is_deterministic() {
    let a = serde_json::to_string(&run_fc()).unwrap();
    let b = serde_json::to_string(&run_fc()).unwrap();
    assert_eq!(a, b);
}

#[test]
fn age_research_blocks_villager_production() {
    let tl = run_fc();
    let s = &tl.summary;
    for age in [Age::Feudal, Age::Castle] {
        let (click, reached) = (s.age_clicks[&age], s.age_reached[&age]);
        let during = tl.events.iter().filter(|e| {
            e.time > click && e.time < reached && matches!(e.event, Event::VillagerCreated { .. })
        });
        assert_eq!(during.count(), 0, "se crearon aldeanos durante la investigación de {age:?}");
    }
}

#[test]
fn castle_click_needs_two_feudal_buildings() {
    let tl = run_fc();
    let click = tl.summary.age_clicks[&Age::Castle];
    let feudal_buildings = tl
        .summary
        .buildings
        .iter()
        .filter(|(b, t)| ["market", "blacksmith", "archery_range", "stable"].contains(&b.as_str()) && *t <= click)
        .count();
    assert!(feudal_buildings >= 2);
}

#[derive(Deserialize)]
struct Validation {
    reference: Reference,
    tolerance: Tolerance,
}

#[derive(Deserialize)]
struct Reference {
    feudal_click: u32,
    castle_click: u32,
    castle_reached: u32,
}

#[derive(Deserialize)]
struct Tolerance {
    castle_reached: u32,
    milestones: u32,
}

#[test]
fn fast_castle_matches_community_reference() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/validation/fast_castle_27_2.toml");
    let v: Validation = toml::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let tl = run_fc();
    let s = &tl.summary;
    let check = |name: &str, got: u32, want: u32, tol: u32| {
        assert!(got.abs_diff(want) <= tol, "{name}: {} vs referencia {} (±{tol} s)", aoe2_sim::clock(got), aoe2_sim::clock(want));
    };
    check("clic Feudal", s.age_clicks[&Age::Feudal], v.reference.feudal_click, v.tolerance.milestones);
    check("clic Castillos", s.age_clicks[&Age::Castle], v.reference.castle_click, v.tolerance.milestones);
    check("llegada a Castillos", s.age_reached[&Age::Castle], v.reference.castle_reached, v.tolerance.castle_reached);
    assert_eq!(s.tc_idle_total(), 0, "CU inactivo: {:?}", s.tc_idle_seconds);
}
