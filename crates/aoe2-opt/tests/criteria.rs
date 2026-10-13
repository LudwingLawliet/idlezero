//! Criterio de la Fase 3 (docs/SPEC.md): "Feudal lo antes posible" encuentra
//! una orden igual o más rápida que la mejor orden manual, sin CU inactivo.

use aoe2_data::{Age, repo_data_dir};
use aoe2_opt::goal::load_presets;
use aoe2_opt::{Goal, OptConfig, OptResult, optimize};
use aoe2_sim::{BuildOrder, IdleCause, Scenario, Stop, simulate};

fn base() -> Scenario {
    let empty = BuildOrder::parse("name = \"-\"\nmap = \"arabia_neutra\"\ndefault_pool = \"forest\"").unwrap();
    Scenario::with_order(&repo_data_dir(), "arabia_neutra", empty).unwrap()
}

fn preset(id: &str) -> Goal {
    load_presets(&repo_data_dir()).unwrap().into_iter().find(|g| g.id == id).unwrap()
}

fn manual_reached(order: &str, age: Age) -> u32 {
    let sc = Scenario::load(&repo_data_dir(), "arabia_neutra", order).unwrap();
    let tl = simulate(&sc, Stop { max_seconds: 3600, age: Some(age) }).unwrap();
    tl.summary.age_reached[&age]
}

/// La orden generada, simulada como una orden normal, reproduce el resultado.
fn check_order_roundtrip(r: &OptResult, age: Age) -> aoe2_sim::Timeline {
    let order = BuildOrder::parse(&r.order_toml).expect("orden TOML válida");
    let sc = Scenario::with_order(&repo_data_dir(), "arabia_neutra", order).unwrap();
    let tl = simulate(&sc, Stop { max_seconds: 3600, age: Some(age) }).unwrap();
    assert_eq!(Some(tl.summary.age_reached[&age]), r.best.time, "la orden exportada no reproduce el plan");
    tl
}

#[test]
fn feudal_asap_beats_manual_orders_without_idle_tc() {
    let r = optimize(&base(), &preset("feudal"), OptConfig::default()).unwrap();
    let best = r.best.time.expect("el optimizador encuentra un plan");
    for manual in ["scouts_21", "fast_castle_27_2"] {
        let m = manual_reached(manual, Age::Feudal);
        assert!(best <= m, "optimizador {best} s vs manual {manual} {m} s");
    }
    let tl = check_order_roundtrip(&r, Age::Feudal);
    let idle: u32 = tl.summary.tc_idle_seconds.iter().filter(|(c, _)| **c != IdleCause::NoOrder).map(|(_, s)| s).sum();
    assert_eq!(idle, 0, "CU inactivo: {:?}", tl.summary.tc_idle_seconds);
}

#[test]
fn castle_asap_beats_manual_fast_castle() {
    let r = optimize(&base(), &preset("castle"), OptConfig::default()).unwrap();
    let best = r.best.time.expect("plan");
    let manual = manual_reached("fast_castle_27_2", Age::Castle);
    assert!(best <= manual, "optimizador {best} s vs FC manual {manual} s");
    check_order_roundtrip(&r, Age::Castle);
}

#[test]
fn optimizer_is_deterministic() {
    let goal = preset("feudal");
    let a = optimize(&base(), &goal, OptConfig::default()).unwrap();
    let b = optimize(&base(), &goal, OptConfig::default()).unwrap();
    let key = |r: &OptResult| serde_json::to_string(&(&r.best, &r.tree, &r.plans, &r.order_toml)).unwrap();
    assert_eq!(key(&a), key(&b));
}

#[test]
fn tree_follows_best_route_and_alternatives_are_not_better() {
    let r = optimize(&base(), &preset("feudal"), OptConfig::default()).unwrap();
    assert_eq!(r.tree.len(), r.best.choices.len());
    for node in &r.tree {
        assert_eq!(node.decision.choice, r.best.choices[node.index]);
        assert!(!node.alternatives.is_empty());
        // Tras el refinamiento, cambiar una sola decisión no mejora la ruta
        // (sin dejar el CU parado: con CU parado puede llegar antes, pero no vale).
        for alt in &node.alternatives {
            if let (Some(d), 0) = (alt.delta, alt.tc_idle) {
                assert!(d >= 0, "alternativa mejor que la ruta óptima: {alt:?}");
            }
        }
    }
    assert!(!r.plans.is_empty() && r.plans[0] == r.best);
}

#[test]
fn unit_goal_trains_the_units() {
    let goal = preset("archers_20");
    let sc = base();
    let r = optimize(&sc, &goal, OptConfig::default()).unwrap();
    assert!(r.best.time.is_some());
    let trained = r
        .timeline
        .events
        .iter()
        .filter(|e| matches!(e.event, aoe2_sim::Event::UnitTrained { .. }))
        .count();
    assert_eq!(trained, 20);
}

#[test]
fn presets_are_valid() {
    let sc = base();
    for g in load_presets(&repo_data_dir()).unwrap() {
        g.validate(&sc).unwrap_or_else(|e| panic!("{}: {e}", g.id));
    }
}
