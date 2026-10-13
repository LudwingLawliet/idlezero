//! Batalla rápida: cargo run -p aoe2-battle --example duel -- <unidad> <n> <unidad> <n> [edad]
use aoe2_battle::{Army, CombatRules, Squad, fight};
use aoe2_data::{Age, GameData, repo_data_dir};

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let dir = repo_data_dir();
    let data = GameData::load(&dir.join("game_data.json")).unwrap();
    let rules = CombatRules::load(&dir).unwrap();
    let age = match a.get(4).map(String::as_str) {
        Some("feudal") => Age::Feudal,
        Some("imperial") => Age::Imperial,
        _ => Age::Castle,
    };
    let army = |u: &str, n: &str| Army { civ: None, age, techs: vec![], squads: vec![Squad { unit: u.parse().unwrap(), count: n.parse().unwrap() }] };
    let r = fight(&data, &rules, &army(&a[0], &a[1]), &army(&a[2], &a[3])).unwrap();
    println!("ganador {:?} en {:.1} s · supervivientes {:?} · destruido {:?}", r.winner, r.seconds, r.survivors.iter().map(|s| s.iter().map(|x| x.count).sum::<u32>()).collect::<Vec<_>>(), r.destroyed);
}
