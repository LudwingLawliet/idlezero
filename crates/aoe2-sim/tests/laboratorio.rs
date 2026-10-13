//! Tests analíticos en el mapa Laboratorio: la simulación debe coincidir con
//! las fórmulas del spec, sin ruido de mapa.

mod common;

use aoe2_data::rates::effective_rate;
use aoe2_data::{Age, ids, repo_data_dir, GameData, GatherRates, MapDef};
use aoe2_sim::{Event, IdleCause, Sim, Stop, simulate};
use common::lab;

const NO_NEW_VILLAGERS: &str = r#"
name = "3 leñadores"
map = "laboratorio"
default_pool = "forest"
max_villagers = 3
"#;

fn stop(seconds: u32) -> Stop {
    Stop { max_seconds: seconds, age: None }
}

/// Lo que recoge un leñador en `t` segundos según la fórmula del spec.
fn expected_wood(t: f64, rate: f64) -> f64 {
    let rates = GatherRates::load(&repo_data_dir().join("gather_rates.toml")).unwrap();
    let map = MapDef::load(&repo_data_dir().join("maps/laboratorio.toml")).unwrap();
    let data = GameData::load(&repo_data_dir().join("game_data.json")).unwrap();
    let speed = data.unit(ids::VILLAGER).unwrap().speed;
    let forest = map.sources.iter().find(|s| s.id == "forest").unwrap();
    effective_rate(rates.carry.default, rate, forest.distance_tiles / speed) * t
}

#[test]
fn lumberjacks_match_effective_rate_formula() {
    let sc = lab(NO_NEW_VILLAGERS);
    let tl = simulate(&sc, stop(600)).unwrap();
    let rate = sc.rates.task("lumberjack").unwrap().rate;
    let expected = 3.0 * expected_wood(600.0, rate);
    let got = tl.summary.gathered.wood;
    // Margen: una carga por aldeano (lo que llevan encima al cortar).
    let carry = 3.0 * sc.rates.carry.default as f64;
    assert!((got - expected).abs() <= carry, "madera {got}, esperado {expected:.1} ± {carry}");
    assert_eq!(tl.summary.villagers, 3);
    assert_eq!(tl.summary.tc_idle_seconds.get(&IdleCause::NoOrder), Some(&600));
}

#[test]
fn double_bit_axe_raises_wood_rate_by_20_percent() {
    let sc = lab(NO_NEW_VILLAGERS);
    let base = simulate(&sc, stop(1200)).unwrap().summary.gathered.wood;
    let mut sim = Sim::new(&sc).unwrap();
    sim.grant_tech(sc.rules.tech_id("double_bit_axe").unwrap());
    let boosted = sim.run(stop(1200)).summary.gathered.wood;

    let rate = sc.rates.task("lumberjack").unwrap().rate;
    // Con caminata, +20 % de tasa bruta da algo menos de +20 % neto.
    let expected_ratio = expected_wood(1.0, rate * 1.2) / expected_wood(1.0, rate);
    let ratio = boosted / base;
    assert!((ratio - expected_ratio).abs() < 0.02, "ratio {ratio:.3}, esperado {expected_ratio:.3}");
    assert!(ratio > 1.1);
}

fn finish_time(tl: &aoe2_sim::Timeline, building: &str, nth: usize) -> u32 {
    tl.events
        .iter()
        .filter(|e| matches!(&e.event, Event::BuildFinished { building: b } if b == building))
        .nth(nth)
        .map(|e| e.time)
        .expect("edificio terminado")
}

#[test]
fn house_build_time_follows_builder_formula() {
    let sc = lab(r#"
        name = "casas"
        map = "laboratorio"
        default_pool = "forest"
        max_villagers = 3
        [[start]]
        villagers = 2
        build = "house"
        then = "forest"
        [[start]]
        villagers = 1
        build = "house"
        then = "forest"
    "#);
    let tl = simulate(&sc, stop(120)).unwrap();
    let walk = sc.map.build_walk("house") / sc.data.unit(ids::VILLAGER).unwrap().speed;
    let base = sc.data.building(ids::HOUSE).unwrap().build_time;
    for (nth, builders) in [(0, 2), (1, 1)] {
        let expected = walk + sc.rates.build_time(base, builders);
        let got = finish_time(&tl, "house", nth) as f64;
        // Resolución de 1 s: la llegada se redondea al segundo.
        assert!((got - expected).abs() <= 1.0, "{builders} constructores: {got} s, esperado {expected:.2} s");
    }
}

#[test]
fn town_center_idles_when_housed() {
    let tl = simulate(&lab(r#"
        name = "sin casas"
        map = "laboratorio"
        default_pool = "forest"
    "#), stop(300)).unwrap();
    // 5 de población: 4 aldeanos + explorador.
    assert_eq!(tl.summary.villagers, 4);
    let housed = tl.summary.tc_idle_seconds.get(&IdleCause::Housed).copied().unwrap_or(0);
    assert_eq!(housed, 300 - 25, "el CU solo trabaja los primeros 25 s");
}

#[test]
fn feudal_needs_two_dark_age_buildings() {
    let tl = simulate(&lab(r#"
        name = "Feudal sin edificios"
        map = "laboratorio"
        default_pool = "forest"
        [[step]]
        when = { villagers = 3 }
        research = "feudal_age"
    "#), Stop { max_seconds: 400, age: Some(Age::Feudal) }).unwrap();
    assert!(tl.summary.age_reached.is_empty());
    assert_eq!(tl.summary.tc_idle_seconds.get(&IdleCause::WaitingResearch), Some(&400));
}

#[test]
fn laboratorio_build_order_reaches_feudal() {
    let sc = common::scenario("laboratorio", "laboratorio_feudal");
    let tl = simulate(&sc, Stop { max_seconds: 1800, age: Some(Age::Feudal) }).unwrap();
    let click = tl.summary.age_clicks[&Age::Feudal];
    let reached = tl.summary.age_reached[&Age::Feudal];
    let research = sc.data.tech(ids::FEUDAL_AGE).unwrap().research_time;
    assert_eq!(reached - click, research);
    // Al hacer clic ya hay molino y campamento maderero.
    let built: Vec<&str> = tl.summary.buildings.iter().filter(|(_, t)| *t <= click).map(|(b, _)| b.as_str()).collect();
    assert!(built.contains(&"mill") && built.contains(&"lumber_camp"), "{built:?}");
}
