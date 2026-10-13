//! Reglas automáticas (`[auto]`), decisiones externas y entrenamiento.

mod common;

use aoe2_data::Age;
use aoe2_sim::{Event, IdleCause, Sim, Stop, simulate};

/// FC 27+2 escrita solo con reglas automáticas: el optimizador genera órdenes así.
const AUTO_FC: &str = r#"
name = "FC automática"
map = "arabia_neutra"
default_pool = "forest"

[auto]
builders = "forest"
houses = 3
drop_sites = true
fallback = [["sheep", "boar"], ["boar", "deer"], ["deer", "farms"], ["berries", "farms"]]
age_buildings = "Castle"

[[start]]
villagers = 3
pool = "sheep"

[[villager]]
n = 4
to = 6
pool = "sheep"
[[villager]]
n = 7
to = 10
pool = "forest"
[[villager]]
n = 11
pool = "boar"
[[villager]]
n = 12
to = 15
pool = "berries"
[[villager]]
n = 16
to = 17
pool = "boar"
[[villager]]
n = 18
to = 23
pool = "forest"
[[villager]]
n = 24
to = 26
pool = "gold"

[[step]]
when = { villagers = 15 }
lure = "boar_2"
from = "boar"

[[step]]
when = { pop = 27 }
research = "loom"
[[step]]
when = { pop = 27 }
research = "feudal_age"
[[step]]
when = { pop = 29, age = "Feudal" }
research = "castle_age"
"#;

fn auto_fc() -> aoe2_sim::Scenario {
    let order = aoe2_sim::BuildOrder::parse(AUTO_FC).unwrap();
    aoe2_sim::Scenario::with_order(&aoe2_data::repo_data_dir(), "arabia_neutra", order).unwrap()
}

#[test]
fn auto_rules_never_house_and_build_drop_sites_and_age_buildings() {
    let sc = auto_fc();
    let tl = simulate(&sc, Stop { max_seconds: 1800, age: Some(Age::Castle) }).unwrap();
    let s = &tl.summary;
    assert_eq!(s.tc_idle_seconds.get(&IdleCause::Housed), None, "{:?}", s.tc_idle_seconds);
    let built: Vec<&str> = s.buildings.iter().map(|(b, _)| b.as_str()).collect();
    for b in ["lumber_camp", "mill", "mining_camp", "blacksmith", "market"] {
        assert!(built.contains(&b), "falta {b}: {built:?}");
    }
    // Las casas automáticas alcanzan para 29 de población.
    assert!(built.iter().filter(|b| **b == "house").count() >= 5);
    assert!(s.age_reached.contains_key(&Age::Castle));
    // Ninguna orden manual movió aldeanos: las granjas vienen del fallback.
    assert!(!tl.events.iter().any(|e| matches!(e.event, Event::Moved { .. })));
    assert!(built.contains(&"farm"));
}

#[test]
fn external_decisions_reproduce_an_order() {
    // Las mismas asignaciones, tomadas desde fuera, dan el mismo resultado.
    let sc = auto_fc();
    let by_order = simulate(&sc, Stop { max_seconds: 1800, age: Some(Age::Castle) }).unwrap();

    let mut order = sc.order.clone();
    let assignments: Vec<(u32, String)> = (4..=28)
        .filter_map(|n| order.assignment_for(n).and_then(|a| a.pool.clone()).map(|p| (n, p)))
        .collect();
    order.villagers.clear();
    let sc2 = aoe2_sim::Scenario { order, ..sc.clone() };
    let mut sim = Sim::new_external(&sc2).unwrap();
    let done = |s: &aoe2_sim::state::GameState| s.age >= Age::Castle;
    while let Some(v) = sim.advance(1800, &done) {
        let pool = assignments.iter().find(|(n, _)| *n == v).map_or("forest", |(_, p)| p.as_str());
        sim.decide(pool);
    }
    let external = sim.into_timeline();
    assert_eq!(external.summary.age_reached, by_order.summary.age_reached);
    assert_eq!(external.summary.gathered, by_order.summary.gathered);
}

#[test]
fn production_trains_units_in_parallel_buildings() {
    let sc = common::lab(r#"
        name = "arqueros"
        map = "laboratorio"
        default_pool = "forest"
        max_villagers = 20
        [auto]
        builders = "forest"
        houses = 3
        age_buildings = "Feudal"
        production = { unit = "archer", count = 10, buildings = 2 }
        [[villager]]
        n = 4
        to = 10
        pool = "farms"
        [[villager]]
        n = 11
        to = 16
        pool = "gold"
        [[step]]
        when = { villagers = 20 }
        research = "feudal_age"
    "#);
    let mut sim = Sim::new(&sc).unwrap();
    let archer = sc.rules.unit_id("archer").unwrap();
    sim.advance(3600, &|s| s.units_of(archer) >= 10);
    let state = sim.state.clone();
    let tl = sim.into_timeline();
    assert_eq!(state.units_of(archer), 10);
    let built: Vec<&str> = tl.summary.buildings.iter().map(|(b, _)| b.as_str()).collect();
    assert!(built.contains(&"barracks"), "la galería necesita cuartel: {built:?}");
    assert_eq!(built.iter().filter(|b| **b == "archery_range").count(), 2);
    // Con 2 galerías, algunos arqueros terminan con menos de un tiempo de
    // entrenamiento de diferencia: se entrenaron en paralelo.
    let train = sc.data.unit(archer).unwrap().train_time;
    let times: Vec<u32> = tl
        .events
        .iter()
        .filter(|e| matches!(e.event, Event::UnitTrained { .. }))
        .map(|e| e.time)
        .collect();
    assert_eq!(times.len(), 10);
    assert!(times.windows(2).any(|w| w[1] - w[0] < train), "{times:?}");
}
