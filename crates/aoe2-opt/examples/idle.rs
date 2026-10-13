//! Segundos de aldeanos sin tarea, por causa, para cada objetivo de data/goals.toml.
//!
//! cargo run --release -p aoe2-opt --example idle -- [mapa]

use aoe2_data::repo_data_dir;
use aoe2_opt::goal::load_presets;
use aoe2_opt::{OptConfig, optimize};
use aoe2_sim::timeline::{Event, Task};
use aoe2_sim::{BuildOrder, Scenario};
use std::collections::BTreeMap;

fn main() {
    let map = std::env::args().nth(1).unwrap_or_else(|| "arabia_neutra".into());
    let dir = repo_data_dir();
    let empty = BuildOrder::parse(&format!("name = \"-\"\nmap = \"{map}\"\ndefault_pool = \"forest\"")).unwrap();
    let base = Scenario::with_order(&dir, &map, empty).unwrap();
    let only = std::env::args().nth(2);
    for goal in load_presets(&dir).unwrap() {
        if only.as_ref().is_some_and(|o| *o != goal.id) {
            continue;
        }
        let r = optimize(&base, &goal, OptConfig::default()).unwrap();
        if let Ok(path) = std::env::var("DUMP_ORDER") {
            std::fs::write(path, &r.order_toml).unwrap();
        }
        let end = r.timeline.summary.end_time;
        let mut cur: BTreeMap<u32, (u32, Task)> = BTreeMap::new();
        let mut by: BTreeMap<String, u32> = BTreeMap::new();
        let mut add = |t: &Task, from: u32, to: u32| {
            if let Task::Idle { reason } = t {
                *by.entry(format!("{reason:?}")).or_default() += to - from;
            }
        };
        for e in &r.timeline.events {
            if let Event::TaskChanged { villager, task } = &e.event {
                if let Some((since, old)) = cur.insert(*villager, (e.time, task.clone())) {
                    add(&old, since, e.time);
                }
            }
        }
        for (since, t) in cur.values() {
            add(t, *since, end);
        }
        println!("{:<22} {:>6}  {:?}", goal.id, r.best.time.map_or("-".into(), aoe2_sim::clock), by);
    }
}
