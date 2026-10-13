//! La calculadora coincide con la fórmula del spec (carga ÷ (carga ÷ tasa + caminata)).

mod common;

use aoe2_sim::calc::{CalcInput, calculate};

fn input(pool: &str, villagers: u32, seconds: u32, drop_site: bool) -> CalcInput {
    CalcInput { pool: pool.into(), villagers, seconds, drop_site, techs: vec![] }
}

#[test]
fn one_lumberjack_one_minute_with_and_without_camp() {
    let sc = common::scenario("arabia_neutra", "fast_castle_27_2");
    let far = calculate(&sc, &input("forest", 1, 60, false)).unwrap();
    let near = calculate(&sc, &input("forest", 1, 60, true)).unwrap();
    // Sin campamento: 10 casillas a 0.8 = 25 s de ida y vuelta por viaje.
    assert!((far.walk_seconds_per_trip - 25.0).abs() < 1e-9);
    // Con campamento (2 casillas) junta más.
    assert!(near.gathered > far.gathered, "{} vs {}", near.gathered, far.gathered);
    // Lo depositado + lo que lleva encima ≈ tasa efectiva × tiempo (± una carga).
    for r in [&far, &near] {
        let total = r.gathered + r.carrying;
        let expected = r.rate_per_villager * 60.0;
        assert!((total - expected).abs() <= r.carry_capacity, "{total} vs {expected}");
    }
}

#[test]
fn more_villagers_gather_proportionally_and_techs_help() {
    let sc = common::scenario("arabia_neutra", "fast_castle_27_2");
    let one = calculate(&sc, &input("forest", 1, 600, true)).unwrap();
    let five = calculate(&sc, &input("forest", 5, 600, true)).unwrap();
    assert!((five.gathered / one.gathered - 5.0).abs() < 0.2);
    let mut with_axe = input("forest", 1, 600, true);
    with_axe.techs = vec!["double_bit_axe".into()];
    let axe = calculate(&sc, &with_axe).unwrap();
    assert!(axe.gathered > one.gathered * 1.1);
}

#[test]
fn finite_sources_run_out() {
    let sc = common::scenario("arabia_neutra", "fast_castle_27_2");
    // 8 ovejas × 100 = 800 de comida: 10 aldeanos en 10 min las acaban.
    let r = calculate(&sc, &input("sheep", 10, 600, false)).unwrap();
    assert!(r.depleted);
    assert!(r.gathered <= 800.0 + 1e-6);
}
