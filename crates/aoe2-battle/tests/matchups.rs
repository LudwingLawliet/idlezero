//! Criterios de la Fase 6: ventajas clásicas, fórmula de daño y determinismo.

use aoe2_battle::{Army, CombatRules, Side, Squad, Stats, fight, hit_damage};
use aoe2_data::{Age, ClassAmount, GameData, repo_data_dir};

const SPEARMAN: u32 = 93;
const PIKEMAN: u32 = 358;
const KNIGHT: u32 = 38;
const ARCHER: u32 = 4;
const SKIRMISHER: u32 = 7;
const MILITIA: u32 = 74;

fn setup() -> (GameData, CombatRules) {
    let dir = repo_data_dir();
    (GameData::load(&dir.join("game_data.json")).unwrap(), CombatRules::load(&dir).unwrap())
}

fn army(unit: u32, count: u32, age: Age) -> Army {
    Army { civ: None, age, techs: vec![], squads: vec![Squad { unit, count }] }
}

fn ca(class: u32, amount: i32) -> ClassAmount {
    ClassAmount { class, amount }
}

#[test]
fn damage_formula_matches_the_game() {
    // Lancero contra caballero: (3 − 2) cuerpo a cuerpo + 15 contra montados.
    let (data, _) = setup();
    let spear = &data.unit(SPEARMAN).unwrap().combat;
    let knight = &data.unit(KNIGHT).unwrap().combat;
    assert_eq!(hit_damage(&spear.attacks, &knight.armours), 16.0);
    // Caballero contra lancero: 10 − 0.
    assert_eq!(hit_damage(&knight.attacks, &spear.armours), 10.0);
    // Mínimo 1 aunque la armadura supere al ataque.
    assert_eq!(hit_damage(&[ca(3, 1)], &[ca(3, 5)]), 1.0);
    // Las clases que el objetivo no tiene no cuentan.
    assert_eq!(hit_damage(&[ca(4, 5), ca(15, 10)], &[ca(4, 0)]), 5.0);
}

#[test]
fn spearmen_beat_knights_for_less_resources() {
    // 12 lanceros (720 recursos) contra 6 caballeros (810). Con 10 la pelea
    // es pareja y la ganan los caballeros: 1 contra 1 el caballero mata en 5
    // golpes y el lancero necesita 7.
    let (data, rules) = setup();
    let r = fight(&data, &rules, &army(SPEARMAN, 12, Age::Castle), &army(KNIGHT, 6, Age::Castle)).unwrap();
    assert_eq!(r.winner, Some(Side::A), "{r:?}");
    assert!(r.cost[0].food + r.cost[0].wood < r.cost[1].food + r.cost[1].gold);
    let r = fight(&data, &rules, &army(PIKEMAN, 10, Age::Castle), &army(KNIGHT, 6, Age::Castle)).unwrap();
    assert_eq!(r.winner, Some(Side::A), "{r:?}");
}

#[test]
fn ten_skirmishers_beat_ten_archers() {
    let (data, rules) = setup();
    let r = fight(&data, &rules, &army(SKIRMISHER, 10, Age::Feudal), &army(ARCHER, 10, Age::Feudal)).unwrap();
    assert_eq!(r.winner, Some(Side::A), "{r:?}");
}

#[test]
fn ten_knights_beat_ten_archers() {
    let (data, rules) = setup();
    let r = fight(&data, &rules, &army(KNIGHT, 10, Age::Castle), &army(ARCHER, 10, Age::Castle)).unwrap();
    assert_eq!(r.winner, Some(Side::A), "{r:?}");
    assert!(r.survivors[0].iter().map(|s| s.count).sum::<u32>() >= 7, "{:?}", r.survivors);
}

#[test]
fn equal_armies_draw_or_split_evenly_and_battles_are_deterministic() {
    let (data, rules) = setup();
    let a = army(MILITIA, 10, Age::Dark);
    let r1 = fight(&data, &rules, &a, &a).unwrap();
    let r2 = fight(&data, &rules, &a, &a).unwrap();
    assert_eq!(format!("{r1:?}"), format!("{r2:?}"));
    // Mismo ejército: ningún bando sale con más de 2 supervivientes.
    for s in &r1.survivors {
        assert!(s.iter().map(|x| x.count).sum::<u32>() <= 2, "{:?}", r1.survivors);
    }
}

#[test]
fn blacksmith_upgrades_change_the_result() {
    let (data, rules) = setup();
    let archer = data.unit(ARCHER).unwrap();
    let base = Stats::of(archer, &[]);
    let mut up = army(ARCHER, 1, Age::Castle);
    up.techs = vec![199, 200, 211];
    let s = Stats::of(archer, &rules.effects(archer, &up));
    assert_eq!(s.range, base.range + 2.0);
    let pierce = |l: &[ClassAmount]| l.iter().find(|c| c.class == 3).unwrap().amount;
    assert_eq!(pierce(&s.attacks), pierce(&base.attacks) + 2);
    assert_eq!(pierce(&s.armours), pierce(&base.armours) + 1);
    // 12 arqueros con Emplumado + Punzón y armadura vencen a 12 sin mejoras.
    let mut a = army(ARCHER, 12, Age::Castle);
    a.techs = vec![199, 200, 211, 212];
    let r = fight(&data, &rules, &a, &army(ARCHER, 12, Age::Castle)).unwrap();
    assert_eq!(r.winner, Some(Side::A));
}

#[test]
fn civ_military_bonuses_apply_by_age_and_cite_the_official_text() {
    let (data, rules) = setup();
    let knight = data.unit(KNIGHT).unwrap();
    let mut franks = army(KNIGHT, 1, Age::Castle);
    franks.civ = Some("Franks".into());
    assert!((Stats::of(knight, &rules.effects(knight, &franks)).hp - 120.0).abs() < 1e-9);
    franks.age = Age::Dark;
    assert!((Stats::of(knight, &rules.effects(knight, &franks)).hp - 100.0).abs() < 1e-9);

    let civs = aoe2_data::civs::parse_civs(&std::fs::read_to_string(repo_data_dir().join("civs.json")).unwrap()).unwrap();
    let norm = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
    for c in &rules.civs {
        let civ = civs.iter().find(|x| x.id == c.id).unwrap();
        assert!(norm(&civ.description.es).contains(&norm(&c.text)), "{}: {}", c.id, c.text);
    }
}

#[test]
fn hp_series_falls_and_costs_add_up() {
    let (data, rules) = setup();
    let r = fight(&data, &rules, &army(KNIGHT, 5, Age::Castle), &army(SPEARMAN, 3, Age::Castle)).unwrap();
    assert_eq!(r.cost[0].food, 5 * 60);
    assert_eq!(r.cost[0].gold, 5 * 75);
    for s in &r.hp {
        assert!(s.windows(2).all(|w| w[1] <= w[0] + 1e-9), "la vida nunca sube");
    }
    assert!(r.destroyed.iter().sum::<u32>() > 0);
}
