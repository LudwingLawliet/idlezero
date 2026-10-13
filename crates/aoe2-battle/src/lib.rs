//! Simulador de combate (Fase 6): dos ejércitos frente a frente, sin terreno.
//!
//! Versión 1 del spec: pasos de 0,1 s; cada unidad elige el enemigo vivo más
//! cercano y lo persigue hasta tenerlo a su alcance; el daño por golpe es el
//! del juego: suma, por cada clase de ataque que el objetivo tenga en su
//! armadura, de max(0, ataque − armadura), con un mínimo de 1. La precisión
//! se aplica como daño esperado (determinista: sin aleatoriedad).
//! Sin micro, formaciones, elevación, proyectiles en vuelo ni colisiones.

use aoe2_data::{Age, ClassAmount, Cost, GameData, Unit};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// Segundos por paso.
pub const DT: f64 = 0.1;
/// Una batalla se corta aquí (empate si quedan unidades en los dos bandos).
pub const MAX_SECONDS: f64 = 600.0;
/// Cada cuánto se guarda la vida total de cada ejército (gráfica).
pub const SAMPLE_SECONDS: f64 = 0.5;
/// Distancia inicial entre los dos frentes (casillas).
const START_GAP: f64 = 12.0;
/// Radio aproximado de una unidad: el cuerpo a cuerpo golpea al tocarse.
const RADIUS: f64 = 0.25;
const ROW_SIZE: usize = 8;
const SPACING: f64 = 0.7;

const PIERCE: u32 = 3;
const MELEE: u32 = 4;

// ---------- reglas (data/combat_upgrades.toml, combat_civ_bonuses.toml) ----------

fn one() -> f64 {
    1.0
}

/// Efecto de una mejora o bono de civilización sobre una unidad.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct Effect {
    #[serde(default)]
    pub melee_attack: i32,
    #[serde(default)]
    pub pierce_attack: i32,
    #[serde(default)]
    pub melee_armor: i32,
    #[serde(default)]
    pub pierce_armor: i32,
    #[serde(default)]
    pub range: f64,
    #[serde(default)]
    pub hp: i32,
    #[serde(default = "one")]
    pub hp_mult: f64,
    /// > 1: ataca más rápido (la recarga se divide por este valor).
    #[serde(default = "one")]
    pub reload_mult: f64,
    #[serde(default = "one")]
    pub speed_mult: f64,
    pub accuracy: Option<u32>,
}

/// A quién afecta un efecto (basta con cumplir una de las condiciones).
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct Applies {
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub units: Vec<u32>,
    #[serde(default)]
    pub trained_at: Vec<u32>,
    #[serde(default)]
    pub classes: Vec<u32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TechEffect {
    pub id: u32,
    pub applies: Applies,
    #[serde(flatten)]
    pub effect: Effect,
    #[serde(default)]
    pub approximate: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CivEffect {
    pub id: String,
    pub applies: Applies,
    pub by_age: BTreeMap<Age, Effect>,
    /// Línea del texto oficial que lo respalda.
    pub text: String,
}

#[derive(Debug, Clone, Deserialize)]
struct UpgradesFile {
    tech: Vec<TechEffect>,
}

#[derive(Debug, Clone, Deserialize)]
struct CivFile {
    civ: Vec<CivEffect>,
}

#[derive(Debug, Clone, Deserialize)]
struct Category {
    id: String,
    class: u32,
}

#[derive(Debug, Clone, Deserialize)]
struct ClassesFile {
    category: Vec<Category>,
}

/// Reglas de combate: mejoras, bonos militares y categorías de unidad.
#[derive(Debug, Clone)]
pub struct CombatRules {
    pub upgrades: Vec<TechEffect>,
    pub civs: Vec<CivEffect>,
    categories: Vec<Category>,
}

impl CombatRules {
    /// A partir del texto de data/combat_upgrades.toml,
    /// data/combat_civ_bonuses.toml y data/armor_classes.toml.
    pub fn parse(upgrades: &str, civs: &str, classes: &str) -> Result<CombatRules, String> {
        let u: UpgradesFile = toml::from_str(upgrades).map_err(|e| format!("combat_upgrades.toml: {e}"))?;
        let c: CivFile = toml::from_str(civs).map_err(|e| format!("combat_civ_bonuses.toml: {e}"))?;
        let k: ClassesFile = toml::from_str(classes).map_err(|e| format!("armor_classes.toml: {e}"))?;
        Ok(CombatRules { upgrades: u.tech, civs: c.civ, categories: k.category })
    }

    pub fn load(data_dir: &Path) -> Result<CombatRules, String> {
        let read = |f: &str| std::fs::read_to_string(data_dir.join(f)).map_err(|e| format!("{f}: {e}"));
        CombatRules::parse(&read("combat_upgrades.toml")?, &read("combat_civ_bonuses.toml")?, &read("armor_classes.toml")?)
    }

    /// Categoría de una unidad (infantry, archers…) por sus clases de armadura.
    pub fn category(&self, unit: &Unit) -> &str {
        self.categories
            .iter()
            .find(|c| unit.combat.armours.iter().any(|a| a.class == c.class))
            .map_or("other", |c| c.id.as_str())
    }

    fn applies(&self, a: &Applies, unit: &Unit) -> bool {
        a.units.contains(&unit.id)
            || a.categories.iter().any(|c| c == self.category(unit))
            || unit.tree.as_ref().and_then(|t| t.produced_at).is_some_and(|b| a.trained_at.contains(&b))
            || unit.combat.armours.iter().any(|x| a.classes.contains(&x.class))
    }

    /// Efectos que recibe una unidad de un ejército (mejoras investigadas y
    /// bonos de su civilización en su edad).
    pub fn effects(&self, unit: &Unit, army: &Army) -> Vec<Effect> {
        let mut out = Vec::new();
        for t in &self.upgrades {
            if army.techs.contains(&t.id) && self.applies(&t.applies, unit) {
                out.push(t.effect.clone());
            }
        }
        if let Some(civ) = &army.civ {
            for c in self.civs.iter().filter(|c| c.id == *civ) {
                if !self.applies(&c.applies, unit) {
                    continue;
                }
                if let Some((_, e)) = c.by_age.iter().filter(|(a, _)| **a <= army.age).last() {
                    out.push(e.clone());
                }
            }
        }
        out
    }
}

// ---------- ejércitos ----------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Squad {
    pub unit: u32,
    pub count: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Army {
    pub civ: Option<String>,
    pub age: Age,
    /// Mejoras investigadas (ids de tecnologías).
    #[serde(default)]
    pub techs: Vec<u32>,
    pub squads: Vec<Squad>,
}

/// Estadísticas de combate de una unidad con sus mejoras y bonos.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Stats {
    pub unit: u32,
    pub hp: f64,
    pub attacks: Vec<ClassAmount>,
    pub armours: Vec<ClassAmount>,
    pub range: f64,
    pub min_range: f64,
    pub reload: f64,
    pub speed: f64,
    pub accuracy: f64,
}

impl Stats {
    pub fn of(unit: &Unit, effects: &[Effect]) -> Stats {
        let c = &unit.combat;
        let mut attacks = c.attacks.clone();
        let mut armours = c.armours.clone();
        let (mut hp, mut range, mut reload, mut speed, mut accuracy) =
            (unit.hp as f64, c.range, c.reload.max(0.1), unit.speed, c.accuracy.max(1) as f64);
        let mut hp_mult = 1.0;
        let bump = |list: &mut Vec<ClassAmount>, class: u32, d: i32| {
            if d != 0 {
                if let Some(x) = list.iter_mut().find(|x| x.class == class) {
                    x.amount += d;
                }
            }
        };
        for e in effects {
            bump(&mut attacks, MELEE, e.melee_attack);
            bump(&mut attacks, PIERCE, e.pierce_attack);
            bump(&mut armours, MELEE, e.melee_armor);
            bump(&mut armours, PIERCE, e.pierce_armor);
            if c.range > 0.0 {
                range += e.range;
            }
            hp += e.hp as f64;
            hp_mult *= e.hp_mult;
            reload /= e.reload_mult;
            speed *= e.speed_mult;
            if let Some(a) = e.accuracy {
                accuracy = accuracy.max(a as f64);
            }
        }
        Stats { unit: unit.id, hp: hp * hp_mult, attacks, armours, range, min_range: c.min_range, reload, speed, accuracy: accuracy.min(100.0) }
    }
}

/// Daño de un golpe de `a` a `d` (fórmula del juego, sin la precisión).
pub fn hit_damage(attacks: &[ClassAmount], armours: &[ClassAmount]) -> f64 {
    let mut total = 0;
    for at in attacks {
        if let Some(ar) = armours.iter().find(|x| x.class == at.class) {
            total += (at.amount - ar.amount).max(0);
        }
    }
    total.max(1) as f64
}

// ---------- simulación ----------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    A,
    B,
}

#[derive(Debug, Clone)]
struct Fighter {
    side: usize,
    stats: usize,
    hp: f64,
    x: f64,
    y: f64,
    target: Option<usize>,
    cooldown: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Survivor {
    pub unit: u32,
    pub count: u32,
    /// Vida total que les queda.
    pub hp: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Outcome {
    /// `None`: empate (los dos caen o se acaba el tiempo).
    pub winner: Option<Side>,
    pub seconds: f64,
    pub survivors: [Vec<Survivor>; 2],
    /// Vida total de cada ejército cada `SAMPLE_SECONDS`.
    pub hp: [Vec<f64>; 2],
    pub cost: [Cost; 2],
    /// Recursos que cada bando hizo perder al otro (costo de lo que mató).
    pub destroyed: [u32; 2],
    /// Estadísticas con mejoras de cada tipo de unidad de cada bando.
    pub stats: [Vec<Stats>; 2],
}

fn cost_total(c: &Cost) -> u32 {
    c.food + c.wood + c.gold + c.stone
}

/// Simula la batalla entre dos ejércitos. Determinista.
pub fn fight(data: &GameData, rules: &CombatRules, a: &Army, b: &Army) -> Result<Outcome, String> {
    let armies = [a, b];
    let mut stats: [Vec<Stats>; 2] = [Vec::new(), Vec::new()];
    let mut stat_index: Vec<BTreeMap<u32, usize>> = vec![BTreeMap::new(), BTreeMap::new()];
    let mut cost = [Cost::default(), Cost::default()];
    let mut fighters: Vec<Fighter> = Vec::new();
    for (side, army) in armies.iter().enumerate() {
        // El cuerpo a cuerpo delante y los de distancia detrás, como el juego al
        // agrupar un ejército.
        let mut squads: Vec<(&Squad, &Unit)> = Vec::new();
        for s in &army.squads {
            let u = data.unit(s.unit).ok_or(format!("unidad desconocida: {}", s.unit))?;
            squads.push((s, u));
        }
        squads.sort_by(|x, y| x.1.combat.range.partial_cmp(&y.1.combat.range).unwrap().then(x.0.unit.cmp(&y.0.unit)));
        let total: usize = squads.iter().map(|(s, _)| s.count as usize).sum();
        let mut slot = 0usize;
        for (s, u) in squads {
            let si = *stat_index[side].entry(u.id).or_insert_with(|| {
                stats[side].push(Stats::of(u, &rules.effects(u, army)));
                stats[side].len() - 1
            });
            for _ in 0..s.count {
                let row = slot / ROW_SIZE;
                let col = slot % ROW_SIZE;
                let width = total.min(ROW_SIZE) as f64;
                let y = (col as f64 - (width - 1.0) / 2.0) * SPACING;
                let depth = row as f64 * SPACING;
                let x = if side == 0 { -depth } else { START_GAP + depth };
                fighters.push(Fighter { side, stats: si, hp: stats[side][si].hp, x, y, target: None, cooldown: 0.0 });
                slot += 1;
            }
            let c = &mut cost[side];
            c.food += u.cost.food * s.count;
            c.wood += u.cost.wood * s.count;
            c.gold += u.cost.gold * s.count;
            c.stone += u.cost.stone * s.count;
        }
    }

    let mut hp_series: [Vec<f64>; 2] = [Vec::new(), Vec::new()];
    let sample = |f: &[Fighter], hp: &mut [Vec<f64>; 2]| {
        for (side, series) in hp.iter_mut().enumerate() {
            series.push(f.iter().filter(|x| x.side == side && x.hp > 0.0).map(|x| x.hp).sum());
        }
    };
    sample(&fighters, &mut hp_series);
    let steps_per_sample = (SAMPLE_SECONDS / DT).round() as usize;
    let mut step = 0usize;
    let alive = |f: &[Fighter], side: usize| f.iter().any(|x| x.side == side && x.hp > 0.0);
    while alive(&fighters, 0) && alive(&fighters, 1) && (step as f64) * DT < MAX_SECONDS {
        // 1. Objetivo: se mantiene mientras esté vivo y al alcance; si no, el
        //    enemigo vivo más cercano (empate: el de menor índice). Así cada
        //    unidad pelea con quien tiene al lado en vez de perseguir en círculo.
        for i in 0..fighters.len() {
            if fighters[i].hp <= 0.0 {
                continue;
            }
            let reach = {
                let st = &stats[fighters[i].side][fighters[i].stats];
                st.range + 2.0 * RADIUS
            };
            if fighters[i].target.is_some_and(|t| {
                fighters[t].hp > 0.0 && (fighters[t].x - fighters[i].x).hypot(fighters[t].y - fighters[i].y) <= reach + 1e-9
            }) {
                continue;
            }
            let (x, y, side) = (fighters[i].x, fighters[i].y, fighters[i].side);
            let mut best: Option<(f64, usize)> = None;
            for (j, e) in fighters.iter().enumerate() {
                if e.side == side || e.hp <= 0.0 {
                    continue;
                }
                let d = (e.x - x).hypot(e.y - y);
                if best.is_none_or(|(bd, _)| d < bd - 1e-9) {
                    best = Some((d, j));
                }
            }
            fighters[i].target = best.map(|(_, j)| j);
        }
        // 2. Moverse hacia el objetivo o golpearlo (los golpes se aplican juntos).
        let mut damage: Vec<(usize, f64)> = Vec::new();
        for i in 0..fighters.len() {
            if fighters[i].hp <= 0.0 {
                continue;
            }
            let Some(t) = fighters[i].target else { continue };
            let st = &stats[fighters[i].side][fighters[i].stats];
            let (dx, dy) = (fighters[t].x - fighters[i].x, fighters[t].y - fighters[i].y);
            let dist = dx.hypot(dy);
            let reach = st.range + 2.0 * RADIUS;
            fighters[i].cooldown = (fighters[i].cooldown - DT).max(0.0);
            if dist > reach {
                let mv = (st.speed * DT).min(dist - reach);
                fighters[i].x += dx / dist * mv;
                fighters[i].y += dy / dist * mv;
            } else if dist + 1e-9 >= st.min_range && fighters[i].cooldown <= 0.0 {
                let target_stats = &stats[fighters[t].side][fighters[t].stats];
                let dmg = hit_damage(&st.attacks, &target_stats.armours) * st.accuracy / 100.0;
                damage.push((t, dmg));
                fighters[i].cooldown = st.reload;
            }
        }
        for (t, d) in damage {
            fighters[t].hp -= d;
        }
        step += 1;
        if step % steps_per_sample == 0 {
            sample(&fighters, &mut hp_series);
        }
    }
    if step % steps_per_sample != 0 {
        sample(&fighters, &mut hp_series);
    }

    let (a_alive, b_alive) = (alive(&fighters, 0), alive(&fighters, 1));
    let winner = match (a_alive, b_alive) {
        (true, false) => Some(Side::A),
        (false, true) => Some(Side::B),
        _ => None,
    };
    let mut survivors: [Vec<Survivor>; 2] = [Vec::new(), Vec::new()];
    let mut destroyed = [0u32; 2];
    for f in &fighters {
        let unit = stats[f.side][f.stats].unit;
        if f.hp > 0.0 {
            let list = &mut survivors[f.side];
            match list.iter_mut().find(|s| s.unit == unit) {
                Some(s) => {
                    s.count += 1;
                    s.hp += f.hp;
                }
                None => list.push(Survivor { unit, count: 1, hp: f.hp }),
            }
        } else {
            destroyed[1 - f.side] += data.unit(unit).map_or(0, |u| cost_total(&u.cost));
        }
    }
    Ok(Outcome { winner, seconds: step as f64 * DT, survivors, hp: hp_series, cost, destroyed, stats })
}
