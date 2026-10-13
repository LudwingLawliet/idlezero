//! Busca la mejor orden para un objetivo e imprime el plan.
//!
//! cargo run -p aoe2-opt --example optimize -- <mapa> <objetivo> [--lang en] [--beam N]
//! Ej.: cargo run -p aoe2-opt --example optimize -- arabia_neutra feudal
//! Objetivos: los de data/goals.toml (feudal, castle, archers_20…).

use aoe2_data::{Locale, repo_data_dir};
use aoe2_opt::goal::load_presets;
use aoe2_opt::{OptConfig, optimize};
use aoe2_sim::report::describe;
use aoe2_sim::{BuildOrder, Scenario, clock};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let flag = |name: &str| args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned();
    let lang = flag("--lang").unwrap_or_else(|| "es".into());
    let beam = flag("--beam").and_then(|b| b.parse().ok());
    let mut pos = Vec::new();
    let mut skip = false;
    for a in &args {
        if skip {
            skip = false;
        } else if a.starts_with("--") {
            skip = true;
        } else {
            pos.push(a.as_str());
        }
    }
    let (Some(map), Some(goal_id)) = (pos.first(), pos.get(1)) else {
        eprintln!("uso: optimize <mapa> <objetivo> [--lang en] [--beam N]");
        std::process::exit(2);
    };
    let dir = repo_data_dir();
    let fail = |e: String| -> ! {
        eprintln!("{e}");
        std::process::exit(1)
    };
    let goals = load_presets(&dir).unwrap_or_else(|e| fail(e.to_string()));
    let goal = goals.iter().find(|g| g.id == *goal_id).unwrap_or_else(|| {
        let ids: Vec<&str> = goals.iter().map(|g| g.id.as_str()).collect();
        fail(format!("objetivo desconocido {goal_id}; disponibles: {}", ids.join(", ")))
    });
    let l = Locale::load(&dir, &lang).unwrap_or_else(|e| fail(e.to_string()));
    let empty = BuildOrder::parse(&format!("name = \"-\"\nmap = \"{map}\"\ndefault_pool = \"forest\"")).unwrap();
    let base = Scenario::with_order(&dir, map, empty).unwrap_or_else(|e| fail(e.to_string()));
    let mut cfg = OptConfig::default();
    if let Some(b) = beam {
        cfg.beam_width = b;
    }
    let started = std::time::Instant::now();
    let r = optimize(&base, goal, cfg).unwrap_or_else(|e| fail(e));
    let elapsed = started.elapsed();

    println!("== {} · {} ==", l.t(&format!("goals.{}", goal.id)), r.map);
    match r.best.time {
        Some(t) => println!("tiempo: {}  ({} simulaciones, {:.1} s)", clock(t), r.simulations, elapsed.as_secs_f64()),
        None => println!("no se cumple en el tiempo máximo"),
    }
    println!();
    for node in &r.tree {
        let d = &node.decision;
        let mut line = format!(
            "{:>6}  aldeano {:>2} → {}",
            clock(d.time),
            d.villager,
            l.pool(&d.choice.pool)
        );
        if let Some(t) = &d.choice.click {
            line += &format!("  + clic {}", aoe2_sim::report::tech(&base, &l, t));
        }
        if d.choice.stop {
            line += "  (deja de crear aldeanos)";
        }
        let alts: Vec<String> = node
            .alternatives
            .iter()
            .take(3)
            .map(|a| match a.delta {
                Some(d) => format!("{} {:+} s", l.pool(&a.choice.pool), d),
                None => format!("{} ✗", l.pool(&a.choice.pool)),
            })
            .collect();
        println!("{line:<52} | alternativas: {}", alts.join(", "));
    }
    println!("\nEventos clave:");
    for e in &r.timeline.events {
        if matches!(e.event, aoe2_sim::Event::VillagerCreated { .. } | aoe2_sim::Event::Decision { .. }) {
            continue;
        }
        if let Some(text) = describe(&base, &l, &e.event) {
            println!("{:>6}  {text}", clock(e.time));
        }
    }
    println!("\nOtros planes:");
    for (i, p) in r.plans.iter().enumerate().skip(1) {
        println!("  {}. {}", i + 1, p.time.map_or("—".into(), clock));
    }
    let out = repo_data_dir().join("../target/opt");
    std::fs::create_dir_all(&out).ok();
    let path = out.join(format!("{}_{}.toml", map, goal.id));
    std::fs::write(&path, &r.order_toml).ok();
    println!("\nOrden guardada en {}", path.display());
}
