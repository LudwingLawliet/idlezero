//! Imprime una simulación tick a tick.
//!
//! cargo run -p aoe2-sim --example run -- <mapa> <orden> [edad final] [--json]
//! Ej.: cargo run -p aoe2-sim --example run -- arabia_neutra fast_castle_27_2 castle

use aoe2_data::{Age, repo_data_dir};
use aoe2_sim::{Event, Scenario, Stop, clock, simulate};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let json = args.iter().any(|a| a == "--json");
    let pos: Vec<&String> = args.iter().filter(|a| !a.starts_with("--")).collect();
    let (Some(map), Some(order)) = (pos.first(), pos.get(1)) else {
        eprintln!("uso: run <mapa> <orden> [feudal|castle|imperial] [--json]");
        std::process::exit(2);
    };
    let age = match pos.get(2).map(|s| s.as_str()) {
        Some("feudal") => Some(Age::Feudal),
        Some("imperial") => Some(Age::Imperial),
        Some("none") => None,
        _ => Some(Age::Castle),
    };
    let sc = Scenario::load(&repo_data_dir(), map, order).unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(1)
    });
    let tl = simulate(&sc, Stop { max_seconds: 40 * 60, age }).unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(1)
    });
    if json {
        println!("{}", serde_json::to_string_pretty(&tl).unwrap());
        return;
    }

    println!("== {} · {} ==", sc.order.name, sc.map.name);
    let mut events = tl.events.iter().peekable();
    for snap in &tl.ticks {
        while let Some(e) = events.next_if(|e| e.time <= snap.time) {
            let text = match &e.event {
                Event::VillagerCreated { villager, assignment } => format!("aldeano {villager} → {assignment}"),
                Event::BuildStarted { building, builders } if building != "farm" => {
                    format!("construye {building} con {builders:?}")
                }
                Event::BuildFinished { building } if building != "farm" => format!("{building} terminado"),
                Event::ResearchStarted { tech } => format!("investiga {tech}"),
                Event::ResearchFinished { tech } => format!("{tech} listo"),
                Event::AgeReached { age } => format!("*** {age:?} ***"),
                Event::Moved { villagers, from, to } => format!("{} aldeanos {from} → {to}", villagers.len()),
                Event::PoolExhausted { pool } => format!("{pool} agotado"),
                Event::Lured { source, villager } => format!("aldeano {villager} atrae {source}"),
                Event::TcIdle { cause, seconds } => format!("CU inactivo {seconds} s ({cause:?})"),
                _ => continue,
            };
            println!("    {:>6}  {text}", clock(e.time));
        }
        let tasks: Vec<String> = snap.villagers_by_task.iter().map(|(k, v)| format!("{k} {v}")).collect();
        println!(
            "Tick {:>3} ({:>5}) · pop {:>2}/{:<3} · F {:>5.0} W {:>5.0} G {:>4.0} S {:>4.0} · {}",
            snap.tick,
            clock(snap.time),
            snap.pop,
            snap.pop_cap,
            snap.food,
            snap.wood,
            snap.gold,
            snap.stone,
            tasks.join(", ")
        );
    }
    let s = &tl.summary;
    println!("\n== Resumen ==");
    for (age, t) in &s.age_clicks {
        println!("clic {age:?}: {}", clock(*t));
    }
    for (age, t) in &s.age_reached {
        println!("llega a {age:?}: {}", clock(*t));
    }
    println!("CU inactivo: {} s {:?}", s.tc_idle_total(), s.tc_idle_seconds);
    println!("aldeanos ociosos (aldeano·s): {}", s.villager_idle_seconds);
    println!(
        "recolectado: F {:.0} W {:.0} G {:.0} S {:.0}",
        s.gathered.food, s.gathered.wood, s.gathered.gold, s.gathered.stone
    );
}
