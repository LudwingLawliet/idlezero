//! El simulador no inventa recursos: todo lo que hay salió de algún sitio y
//! todo lo gastado se pagó con lo que había.

mod common;

use aoe2_data::Age;
use aoe2_sim::{Event, Sim, Stop, Task, Timeline, simulate};
use std::collections::BTreeMap;

fn fc() -> (aoe2_sim::Scenario, Timeline) {
    let sc = common::scenario("arabia_neutra", "fast_castle_27_2");
    let tl = simulate(&sc, Stop { max_seconds: 1800, age: Some(Age::Castle) }).unwrap();
    (sc, tl)
}

#[test]
fn stock_never_goes_negative() {
    let (_, tl) = fc();
    let s = &tl.series;
    for (i, ((f, w), (g, st))) in s.food.iter().zip(&s.wood).zip(s.gold.iter().zip(&s.stone)).enumerate() {
        assert!(*f >= -1e-3 && *w >= -1e-3 && *g >= -1e-3 && *st >= -1e-3, "segundo {i}: {f} {w} {g} {st}");
    }
    for e in &tl.events {
        if let Event::Spent { what, stock, .. } = &e.event {
            assert!(stock.iter().all(|x| *x >= -1e-6), "{what} dejó reservas negativas: {stock:?}");
        }
    }
}

#[test]
fn stock_equals_start_plus_gathered_minus_spent() {
    let (sc, tl) = fc();
    let start = sc.map.start.resources;
    let g = &tl.summary.gathered;
    let sp = &tl.summary.spent;
    let last = tl.series.food.len() - 1;
    let expected = [
        start.food as f64 + g.food - sp.food,
        start.wood as f64 + g.wood - sp.wood,
        start.gold as f64 + g.gold - sp.gold,
        start.stone as f64 + g.stone - sp.stone,
    ];
    let got = [tl.series.food[last], tl.series.wood[last], tl.series.gold[last], tl.series.stone[last]];
    for i in 0..4 {
        assert!((expected[i] - got[i] as f64).abs() < 0.5, "recurso {i}: esperado {}, hay {}", expected[i], got[i]);
    }
    // Lo gastado según el libro coincide con el resumen.
    let mut spent = [0u32; 4];
    for e in &tl.events {
        if let Event::Spent { cost, .. } = &e.event {
            spent[0] += cost.food;
            spent[1] += cost.wood;
            spent[2] += cost.gold;
            spent[3] += cost.stone;
        }
    }
    assert_eq!(spent.map(|x| x as f64), [sp.food, sp.wood, sp.gold, sp.stone]);
}

#[test]
fn villagers_gathered_add_up_to_total() {
    let (_, tl) = fc();
    let mut sum = [0.0; 4];
    for v in &tl.villagers {
        for i in 0..4 {
            sum[i] += v.gathered[i];
        }
    }
    let g = &tl.summary.gathered;
    for (i, total) in [g.food, g.wood, g.gold, g.stone].into_iter().enumerate() {
        assert!((sum[i] - total).abs() < 1.0, "recurso {i}: aldeanos {} vs total {total}", sum[i]);
    }
}

#[test]
fn finite_sources_are_never_overdrawn() {
    let sc = common::scenario("arabia_neutra", "fast_castle_27_2");
    let mut sim = Sim::new(&sc).unwrap();
    sim.advance(1800, &|s| s.age >= Age::Castle);
    for s in &sim.state.sources {
        assert!(s.remaining >= -1e-6, "{} sobregirado: {}", s.id, s.remaining);
    }
}

#[test]
fn every_villager_has_a_task_history() {
    let (_, tl) = fc();
    let mut tasks: BTreeMap<u32, Vec<&Task>> = BTreeMap::new();
    for e in &tl.events {
        if let Event::TaskChanged { villager, task } = &e.event {
            tasks.entry(*villager).or_default().push(task);
        }
    }
    for v in &tl.villagers {
        let t = tasks.get(&v.id).unwrap_or_else(|| panic!("aldeano {} sin historial", v.id));
        assert!(t.iter().any(|t| matches!(t, Task::Gather { .. })), "aldeano {} nunca recolecta", v.id);
    }
    // Los iniciales empiezan construyendo casas.
    assert!(matches!(tasks[&1][0], Task::Build { building, .. } if building == "house"));
}

const SECOND_TC: &str = r#"
name = "2.º CU"
map = "arabia_neutra"
default_pool = "forest"
[auto]
builders = "forest"
houses = 3
drop_sites = true
fallback = [["sheep", "boar"], ["boar", "berries"], ["berries", "deer"], ["deer", "farms"]]
[[start]]
villagers = 3
pool = "sheep"
[[villager]]
n = 4
to = 7
pool = "sheep"
[[villager]]
n = 8
to = 14
pool = "forest"
[[villager]]
n = 15
to = 16
pool = "stone"
[[step]]
when = { villagers = 16 }
build = "town_center"
builders = 3
from = "forest"
"#;

#[test]
fn a_second_town_center_produces_villagers_and_builders_speed_it_up() {
    let order = aoe2_sim::BuildOrder::parse(SECOND_TC).unwrap();
    let sc = aoe2_sim::Scenario::with_order(&aoe2_data::repo_data_dir(), "arabia_neutra", order).unwrap();
    let run = |overrides: BTreeMap<usize, u32>| {
        let mut sim = Sim::new(&sc).unwrap();
        sim.set_builder_overrides(overrides);
        sim.advance(1500, &|_| false);
        sim.into_timeline()
    };
    let tl = run(BTreeMap::new());
    let (placement, tc_done) = tl
        .events
        .iter()
        .find_map(|e| match &e.event {
            Event::BuildStarted { building, placement, .. } if building == "town_center" => Some(*placement),
            _ => None,
        })
        .zip(tl.events.iter().find_map(|e| match &e.event {
            Event::BuildFinished { building } if building == "town_center" => Some(e.time),
            _ => None,
        }))
        .expect("se construye el 2.º CU");
    assert!(tl.villagers.iter().any(|v| v.born_tc == Some(1)), "nadie nació del 2.º CU");
    assert!(tl.villagers.iter().filter(|v| v.born_tc == Some(1)).all(|v| v.born > tc_done));
    assert_eq!(*tl.series.tcs.last().unwrap(), 2);

    // Con más constructores el CU termina antes.
    let faster = run(BTreeMap::from([(placement, 6)]));
    let tc_done_6 = faster
        .events
        .iter()
        .find_map(|e| match &e.event {
            Event::BuildFinished { building } if building == "town_center" => Some(e.time),
            _ => None,
        })
        .unwrap();
    assert!(tc_done_6 < tc_done, "6 constructores: {tc_done_6} vs 3: {tc_done}");
}
