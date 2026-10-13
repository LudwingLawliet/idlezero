//! Fase 5: bonos de civilización aplicados por el simulador.

mod common;

use aoe2_data::{Age, repo_data_dir};
use aoe2_sim::calc::{CalcInput, calculate};
use aoe2_sim::{Event, Scenario, Sim, Stop, simulate};

fn fc(civ: Option<&str>) -> Scenario {
    let sc = common::scenario("arabia_neutra", "fast_castle_27_2");
    match civ {
        Some(c) => sc.with_civ(&repo_data_dir(), c).unwrap(),
        None => sc,
    }
}

fn houses(tl: &aoe2_sim::Timeline) -> usize {
    tl.events
        .iter()
        .filter(|e| matches!(&e.event, Event::BuildStarted { building, .. } if building == "house"))
        .count()
}

#[test]
fn all_test_civs_exist_with_bonuses() {
    for id in ["Huns", "Britons", "Franks", "Mayans"] {
        let sc = fc(Some(id));
        let civ = sc.civ.as_ref().unwrap_or_else(|| panic!("falta {id}"));
        assert!(!civ.bonuses.is_empty(), "{id} sin bonos");
        assert!(!civ.info.description.es.is_empty());
    }
}

#[test]
fn huns_need_no_houses_and_start_with_less_wood() {
    // Con la orden automática (las casas de la orden manual se colocarían igual).
    let order = aoe2_sim::BuildOrder::parse(
        "name = \"x\"\nmap = \"arabia_neutra\"\ndefault_pool = \"forest\"\n[auto]\nbuilders = \"forest\"\nhouses = 3\n[[start]]\nvillagers = 3\npool = \"sheep\"",
    )
    .unwrap();
    let base = Scenario::with_order(&repo_data_dir(), "arabia_neutra", order).unwrap();
    let huns = base.clone().with_civ(&repo_data_dir(), "Huns").unwrap();
    let stop = Stop { max_seconds: 600, age: None };
    let neutral = simulate(&base, stop).unwrap();
    let hun = simulate(&huns, stop).unwrap();
    assert!(houses(&neutral) > 0);
    assert_eq!(houses(&hun), 0, "los hunos no construyen casas");
    assert_eq!(hun.series.wood[0], 100.0);
    assert_eq!(hun.series.pop_cap[0], 200);
    assert_eq!(hun.summary.tc_idle_seconds.get(&aoe2_sim::IdleCause::Housed), None);
}

#[test]
fn britons_shepherds_are_25_percent_faster() {
    let input = CalcInput { pool: "sheep".into(), villagers: 1, seconds: 60, drop_site: false, techs: vec![] };
    let neutral = calculate(&fc(None), &input).unwrap();
    let britons = calculate(&fc(Some("Britons")), &input).unwrap();
    assert!((britons.gather_rate / neutral.gather_rate - 1.25).abs() < 1e-9);
}

#[test]
fn franks_get_mill_techs_for_free() {
    let order = aoe2_sim::BuildOrder::parse(
        "name = \"x\"\nmap = \"arabia_neutra\"\ndefault_pool = \"forest\"\n[[step]]\nwhen = { time = 0 }\nresearch = \"horse_collar\"",
    )
    .unwrap();
    let mut sc = Scenario::with_order(&repo_data_dir(), "arabia_neutra", order).unwrap().with_civ(&repo_data_dir(), "Franks").unwrap();
    // Molino ya hecho y en Feudal para que se pueda investigar en el segundo 0.
    sc.order.auto.research = vec![];
    let mut sim = Sim::new(&sc).unwrap();
    sim.state.age = Age::Feudal;
    sim.state.built.push((aoe2_data::ids::MILL, 0));
    sim.advance(60, &|_| false);
    let tl = sim.into_timeline();
    let spent = tl.events.iter().find_map(|e| match &e.event {
        Event::Spent { what, cost, .. } if what == "horse_collar" => Some(*cost),
        _ => None,
    });
    assert_eq!(spent, Some(aoe2_data::Cost::default()), "la Collera debe ser gratis para los francos");
}

#[test]
fn mayans_start_with_extra_villager_and_resources_last_longer() {
    let sc = fc(Some("Mayans"));
    let mut sim = Sim::new(&sc).unwrap();
    assert_eq!(sim.state.villagers.len(), 4);
    assert_eq!(sim.state.res.food, 150.0);
    let sheep: f64 = sim.state.sources.iter().filter(|s| s.id.starts_with("sheep")).map(|s| s.remaining).sum();
    assert!((sheep - 920.0).abs() < 1e-6, "ovejas {sheep}");
    sim.advance(10, &|_| false);
}

#[test]
fn civ_simulation_is_deterministic() {
    let a = serde_json::to_string(&simulate(&fc(Some("Mayans")), Stop { max_seconds: 1200, age: Some(Age::Castle) }).unwrap()).unwrap();
    let b = serde_json::to_string(&simulate(&fc(Some("Mayans")), Stop { max_seconds: 1200, age: Some(Age::Castle) }).unwrap()).unwrap();
    assert_eq!(a, b);
}

/// Espacios normalizados (el texto oficial parte algunas líneas).
fn norm(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ").replace('\u{a0}', " ")
}

#[test]
fn every_bonus_cites_its_official_line() {
    let dir = repo_data_dir();
    let civs = aoe2_data::civs::parse_civs(&std::fs::read_to_string(dir.join("civs.json")).unwrap()).unwrap();
    let bonuses = aoe2_data::civs::parse_bonuses(&std::fs::read_to_string(dir.join("civ_bonuses.toml")).unwrap()).unwrap();
    assert!(bonuses.len() >= 40, "solo {} civilizaciones con bonos", bonuses.len());
    for cb in &bonuses {
        let civ = civs.iter().find(|c| c.id == cb.id).unwrap_or_else(|| panic!("civilización desconocida {}", cb.id));
        let desc = norm(&civ.description.es);
        for b in &cb.bonuses {
            assert!(desc.contains(&norm(&b.text)), "{}: \"{}\" no está en el texto oficial", cb.id, b.text);
        }
    }
}

#[test]
fn all_civilizations_simulate_the_fast_castle_order() {
    let dir = repo_data_dir();
    let civs = aoe2_data::civs::parse_civs(&std::fs::read_to_string(dir.join("civs.json")).unwrap()).unwrap();
    assert_eq!(civs.len(), 56);
    let neutral = simulate(&fc(None), Stop { max_seconds: 3600, age: Some(Age::Castle) }).unwrap();
    let n = neutral.summary.age_reached[&Age::Castle];
    for c in &civs {
        let tl = simulate(&fc(Some(&c.id)), Stop { max_seconds: 3600, age: Some(Age::Castle) }).unwrap_or_else(|e| panic!("{}: {e}", c.id));
        let t = *tl.summary.age_reached.get(&Age::Castle).unwrap_or_else(|| panic!("{} no llega a Castillos", c.id));
        // La orden manual no aprovecha los bonos, pero ninguna civilización
        // debería quedar muy lejos del escenario neutro.
        eprintln!("{:<12} {:+}", c.id, t as i64 - n as i64);
        // Los chinos empiezan sin comida: la orden manual no está hecha para ellos.
        if c.id != "Chinese" {
            assert!(t <= n + 120, "{}: {t} s frente a {n} s neutro", c.id);
        }
    }
}
