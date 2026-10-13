//! Imprime una simulación tick a tick.
//!
//! cargo run -p aoe2-sim --example run -- <mapa> <orden> [feudal|castle|imperial|none] [--lang en] [--json]
//! Ej.: cargo run -p aoe2-sim --example run -- arabia_neutra fast_castle_27_2

use aoe2_data::{Age, Locale, repo_data_dir};
use aoe2_sim::report::{describe, idle_cause};
use aoe2_sim::{Scenario, Stop, clock, simulate};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let json = args.iter().any(|a| a == "--json");
    let lang = args.iter().position(|a| a == "--lang").and_then(|i| args.get(i + 1)).map_or("es", |s| s.as_str());
    let mut pos = Vec::new();
    let mut skip = false;
    for a in &args {
        if skip {
            skip = false;
        } else if a == "--lang" {
            skip = true;
        } else if !a.starts_with("--") {
            pos.push(a.as_str());
        }
    }
    let (Some(map), Some(order)) = (pos.first(), pos.get(1)) else {
        eprintln!("uso: run <mapa> <orden> [feudal|castle|imperial|none] [--lang en] [--json]");
        std::process::exit(2);
    };
    let age = match pos.get(2).copied() {
        Some("feudal") => Some(Age::Feudal),
        Some("imperial") => Some(Age::Imperial),
        Some("none") => None,
        _ => Some(Age::Castle),
    };
    let fail = |e: String| -> ! {
        eprintln!("{e}");
        std::process::exit(1)
    };
    let dir = repo_data_dir();
    let sc = Scenario::load(&dir, map, order).unwrap_or_else(|e| fail(e.to_string()));
    let l = Locale::load(&dir, lang).unwrap_or_else(|e| fail(e.to_string()));
    let tl = simulate(&sc, Stop { max_seconds: 40 * 60, age }).unwrap_or_else(|e| fail(e));
    if json {
        println!("{}", serde_json::to_string_pretty(&tl).unwrap());
        return;
    }

    println!("== {} · {} ==", sc.order.name, sc.map.name);
    let mut events = tl.events.iter().peekable();
    for snap in &tl.ticks {
        while let Some(e) = events.next_if(|e| e.time <= snap.time) {
            if let Some(text) = describe(&sc, &l, &e.event) {
                println!("    {:>6}  {text}", clock(e.time));
            }
        }
        let tasks: Vec<String> =
            snap.villagers_by_task.iter().map(|(k, v)| format!("{} {v}", l.pool(k))).collect();
        println!(
            "{} {:>3} ({:>5}) · {} {:>2}/{:<3} · {} {:>5.0} {} {:>5.0} {} {:>4.0} {} {:>4.0} · {}",
            l.t("report.tick"),
            snap.tick,
            clock(snap.time),
            l.t("report.pop"),
            snap.pop,
            snap.pop_cap,
            l.t("resources_short.food"),
            snap.food,
            l.t("resources_short.wood"),
            snap.wood,
            l.t("resources_short.gold"),
            snap.gold,
            l.t("resources_short.stone"),
            snap.stone,
            tasks.join(", ")
        );
    }
    let s = &tl.summary;
    println!("\n== {} ==", l.t("report.summary"));
    for (age, t) in &s.age_clicks {
        println!("{}: {}", l.fmt("report.click", &[("age", &l.age(*age))]), clock(*t));
    }
    for (age, t) in &s.age_reached {
        println!("{}: {}", l.fmt("report.reached", &[("age", &l.age(*age))]), clock(*t));
    }
    let causes: Vec<String> =
        s.tc_idle_seconds.iter().map(|(c, n)| format!("{} {n} s", idle_cause(&l, *c))).collect();
    println!("{}: {} s {}", l.t("report.tc_idle"), s.tc_idle_total(), causes.join(", "));
    println!("{}: {}", l.t("report.villager_idle"), s.villager_idle_seconds);
    println!(
        "{}: {} {:.0} · {} {:.0} · {} {:.0} · {} {:.0}",
        l.t("report.gathered"),
        l.t("resources.food"),
        s.gathered.food,
        l.t("resources.wood"),
        s.gathered.wood,
        l.t("resources.gold"),
        s.gathered.gold,
        l.t("resources.stone"),
        s.gathered.stone
    );
}
