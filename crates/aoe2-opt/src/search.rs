//! Búsqueda: haz por decisión de aldeano, refinamiento local y árbol de
//! alternativas. Todo determinista (sin aleatoriedad, órdenes de recorrido fijos).

use crate::goal::Goal;
use aoe2_data::Age;
use aoe2_sim::orders::{Assign, Auto, BuildOrder, Condition, Production, Step, VillagerAssign};
use aoe2_sim::state::GameState;
use aoe2_sim::{Scenario, Sim, Snapshot, Timeline};
use serde::Serialize;
use std::collections::BTreeSet;

/// Madera que conviene tener de más para casas y granjas (no son parte del
/// objetivo pero frenan todo si faltan).
const WOOD_RESERVE: f64 = 120.0;

#[derive(Debug, Clone)]
pub struct OptConfig {
    pub beam_width: usize,
    pub max_seconds: u32,
    /// Simulaciones completas permitidas en el refinamiento.
    pub refine_budget: usize,
    /// Planes completos distintos a devolver.
    pub plans: usize,
}

impl Default for OptConfig {
    fn default() -> Self {
        OptConfig { beam_width: 64, max_seconds: 60 * 60, refine_budget: 600, plans: 5 }
    }
}

/// Lo que se decide al nacer cada aldeano.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, serde::Deserialize)]
pub struct Choice {
    /// Pool al que va el aldeano.
    pub pool: String,
    /// Investigación de edad que se pide en este momento.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub click: Option<String>,
    /// Dejar de crear aldeanos desde aquí.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub stop: bool,
    /// Clic "duro": el CU para y ahorra para la edad (si el objetivo lo permite).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub hard: bool,
    /// Antes de ir a `pool`, construir este edificio…
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub build: Option<String>,
    /// …con este número de constructores en total.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub builders: u32,
}

fn is_zero(n: &u32) -> bool {
    *n == 0
}

impl Choice {
    pub fn gather(pool: &str) -> Choice {
        Choice { pool: pool.to_owned(), click: None, stop: false, hard: false, build: None, builders: 0 }
    }
}

/// Una decisión ya tomada, con el momento y el aldeano.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Decision {
    pub villager: u32,
    pub time: u32,
    #[serde(flatten)]
    pub choice: Choice,
    /// Estado en ese segundo (para el panel lateral de la interfaz).
    pub state: Snapshot,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Plan {
    /// Segundo en que se cumple el objetivo (`None` = no se cumplió a tiempo).
    pub time: Option<u32>,
    pub choices: Vec<Choice>,
}

/// Alternativa en un nodo del árbol: cambiar solo esa decisión.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Alternative {
    #[serde(flatten)]
    pub choice: Choice,
    /// Segundo en que se cumple el objetivo con esta alternativa (aunque el
    /// CU haya parado; ver `tc_idle`). `None` = no se cumple.
    pub time: Option<u32>,
    /// Segundos de diferencia con la mejor ruta (positivo = peor).
    pub delta: Option<i64>,
    /// Segundos que el CU queda parado con esta alternativa.
    pub tc_idle: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TreeNode {
    pub index: usize,
    #[serde(flatten)]
    pub decision: Decision,
    pub alternatives: Vec<Alternative>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct OptResult {
    pub goal: Goal,
    pub map: String,
    pub best: Plan,
    /// La mejor ruta como orden de construcción, en TOML (editable, re-simulable).
    pub order_toml: String,
    pub timeline: Timeline,
    pub tree: Vec<TreeNode>,
    /// Los mejores planes distintos, el primero es `best`.
    pub plans: Vec<Plan>,
    /// Simulaciones hechas (parciales en el haz, completas en el resto).
    pub simulations: usize,
    /// No hubo forma de cumplir el objetivo sin dejar el CU inactivo: el plan
    /// es el más rápido admitiendo esperas (ver timeline.summary.tc_idle_seconds).
    pub relaxed: bool,
    /// Con civilización: tiempo del mismo objetivo en el escenario neutro.
    pub neutral_time: Option<u32>,
}

pub struct Optimizer<'a> {
    sc: Scenario,
    /// Tasa efectiva de un aldeano nuevo por recurso (comida, madera, oro,
    /// piedra), con el mejor punto de entrega del mapa.
    per_villager: [f64; 4],
    goal: &'a Goal,
    cfg: OptConfig,
    pools: Vec<String>,
    target: Age,
    /// El objetivo necesita piedra (si no, no se propone minar piedra).
    needs_stone: bool,
    /// Sin plan con el CU siempre activo: se acepta cualquier CU inactivo.
    relaxed: bool,
    /// Planes conocidos (p. ej. el mejor neutro) que entran como candidatos.
    seeds: Vec<Vec<Choice>>,
    /// Cota de tiempo externa para podar el haz cuando no hay plan propio.
    hint_bound: Option<u32>,
    simulations: std::sync::atomic::AtomicUsize,
}

struct Run<'s> {
    time: Option<u32>,
    /// Segundo en que se cumplió el objetivo aunque el CU haya parado.
    reached: Option<u32>,
    /// Segundos de CU parado (sin contar "sin más aldeanos en la orden").
    tc_idle: u32,
    /// Segundo en que empezó cada investigación de edad.
    clicks: std::collections::BTreeMap<Age, u32>,
    decisions: Vec<Decision>,
    /// Simulador justo antes de cada decisión.
    checkpoints: Vec<Sim<'s>>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Heuristic {
    Simple,
    Macro,
}

/// Parámetros de la política voraz.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Policy {
    /// Aldeanos al hacer clic en Feudal, Castillos e Imperial.
    pub click_at: [u32; 3],
    /// Comida de reserva (en aldeanos de 50) para no parar el CU.
    pub food_reserve: f64,
    /// Dejar de crear aldeanos al llegar a este número (objetivos de unidades).
    pub stop_at: Option<u32>,
    /// Clics duros (parar el CU para ahorrar).
    pub hard_clicks: bool,
    /// CU adicionales a construir en cuanto se pueda.
    pub extra_tcs: u32,
}

impl Default for Policy {
    fn default() -> Self {
        Policy { click_at: [21, 24, 32], food_reserve: 2.0, stop_at: None, hard_clicks: false, extra_tcs: 0 }
    }
}

#[derive(Clone)]
struct Node<'s> {
    sim: Sim<'s>,
    choices: Vec<Choice>,
    est: f64,
}

/// Orden base: solo reglas automáticas, sin asignaciones. El optimizador
/// decide qué hace cada aldeano y cuándo subir de edad.
pub fn policy_order(base: &Scenario, goal: &Goal) -> BuildOrder {
    let food_pools: Vec<String> = {
        let mut v: Vec<String> = Vec::new();
        for s in &base.map.sources {
            let task = base.rates.task(&s.task);
            let food = task.is_some_and(|t| t.resource == aoe2_data::rates::Resource::Food);
            if food && s.task != "farmer" && !v.iter().any(|p| p == s.pool()) {
                v.push(s.pool().to_owned());
            }
        }
        v
    };
    let farms = base.map.sources.iter().find(|s| s.task == "farmer").map(|s| s.pool().to_owned());
    let mut fallback = Vec::new();
    for (i, p) in food_pools.iter().enumerate() {
        match (food_pools.get(i + 1), &farms) {
            (Some(next), _) => fallback.push([p.clone(), next.clone()]),
            (None, Some(f)) => fallback.push([p.clone(), f.clone()]),
            _ => {}
        }
    }
    let target = goal.target_age(base);
    BuildOrder {
        name: format!("IdleZero · {}", goal.id),
        map: base.map.name.clone(),
        default_pool: "forest".to_owned(),
        max_villagers: None,
        // Los iniciales también los decide el optimizador (en el segundo 0).
        start: Vec::new(),
        villagers: Vec::new(),
        steps: Vec::new(),
        auto: Auto {
            builders: Some("forest".to_owned()),
            houses: Some(3),
            drop_sites: true,
            fallback,
            age_buildings: (target > Age::Dark).then_some(target),
            research: goal.techs.clone(),
            lure_ahead: Some(100),
            production: goal.units.first().map(|u| Production {
                unit: u.unit.clone(),
                count: u.count,
                buildings: crate::goal::PRODUCTION_BUILDINGS,
            }),
        },
    }
}

fn next_age(a: Age) -> Option<Age> {
    match a {
        Age::Dark => Some(Age::Feudal),
        Age::Feudal => Some(Age::Castle),
        Age::Castle => Some(Age::Imperial),
        Age::Imperial => None,
    }
}

impl<'a> Optimizer<'a> {
    /// `base` aporta datos y mapa; su orden se reemplaza por la orden base.
    pub fn new(base: &Scenario, goal: &'a Goal, cfg: OptConfig) -> Result<Optimizer<'a>, String> {
        goal.validate(base)?;
        let order = policy_order(base, goal);
        let sc = Scenario { order, ..base.clone() };
        let pools = Sim::new(&sc)?.pool_names().to_vec();
        let target = goal.target_age(&sc);
        let speed = sc.data.unit(aoe2_data::ids::VILLAGER).map_or(0.8, |u| u.speed);
        let mut per_villager = [0.0f64; 4];
        for src in &sc.map.sources {
            let Some(task) = sc.rates.task(&src.task) else { continue };
            let dist = src.distance_with_drop_site.unwrap_or(src.distance_tiles);
            let r = aoe2_data::rates::effective_rate(sc.rates.carry_for(task), task.rate, dist / speed);
            use aoe2_data::rates::Resource::*;
            let i = match task.resource {
                Food => 0,
                Wood => 1,
                Gold => 2,
                Stone => 3,
            };
            per_villager[i] = per_villager[i].max(r);
        }
        let needs_stone = goal.remaining_cost(&sc, &Sim::new(&sc)?.state).stone > 0;
        Ok(Optimizer {
            sc,
            per_villager,
            goal,
            cfg,
            pools,
            target,
            needs_stone,
            relaxed: false,
            seeds: Vec::new(),
            hint_bound: None,
            simulations: Default::default(),
        })
    }

    /// Acepta planes con el CU inactivo (si con la regla estricta no hay ninguno).
    pub fn relax(mut self) -> Self {
        self.relaxed = true;
        self
    }

    /// Planes ya conocidos que se prueban como candidatos (se re-simulan aquí).
    pub fn with_seeds(mut self, seeds: Vec<Vec<Choice>>) -> Self {
        self.seeds = seeds;
        self
    }

    /// Cota de tiempo para podar el haz si las políticas no encuentran plan.
    pub fn with_bound(mut self, bound: Option<u32>) -> Self {
        self.hint_bound = bound;
        self
    }

    /// Solo la búsqueda por políticas (rápida): mejor tiempo encontrado.
    pub fn quick_time(&self) -> Option<u32> {
        self.policy_search().iter().filter_map(|p| p.time).min()
    }

    pub fn scenario(&self) -> &Scenario {
        &self.sc
    }

    fn done_fn(&self) -> impl Fn(&GameState) -> bool + '_ {
        move |s: &GameState| self.goal.is_done(&self.sc, s)
    }

    /// Opciones razonables en una decisión.
    fn options(&self, sim: &Sim) -> Vec<Choice> {
        let s = &sim.state;
        let need = self.goal.remaining_cost(&self.sc, s);
        let mut pools: Vec<&String> = Vec::new();
        for p in &self.pools {
            if !sim.pool_has_resources(p) {
                continue;
            }
            let src = self.sc.map.sources.iter().find(|x| x.pool() == p.as_str());
            let resource = src.and_then(|x| self.sc.rates.task(&x.task)).map(|t| t.resource);
            use aoe2_data::rates::Resource::*;
            let needed = match resource {
                Some(Gold) => need.gold > 0,
                Some(Stone) => need.stone > 0,
                _ => true,
            };
            // Fuentes que llegan más tarde (ciervos): solo si llegan pronto.
            let soon = src.is_none_or(|x| x.arrives_at_s <= s.time + 60);
            if needed && soon {
                pools.push(p);
            }
        }
        // Clic de edad: si el objetivo pide una edad mayor y no está pedida.
        let click = next_age(s.age).filter(|a| *a <= self.target).and_then(|a| {
            let tech = self.sc.rules.age_tech(a)?;
            let pending = s.pending_research.contains(&tech) || s.tc_researching(tech);
            (!pending).then(|| self.sc.rules.tech_name(tech).unwrap().to_owned())
        });
        // Dejar de crear aldeanos: objetivos de unidades/edificios, ya en la edad.
        let can_stop = (!self.goal.units.is_empty() || !self.goal.buildings.is_empty())
            && s.age >= self.target
            && sim.max_villagers().is_none();

        let hards: &[bool] = if self.goal.allow_tc_idle { &[false, true] } else { &[false] };
        let mut out = Vec::new();
        for p in pools {
            for c in [None, click.clone()].into_iter().collect::<BTreeSet<_>>() {
                for stop in if can_stop { vec![false, true] } else { vec![false] } {
                    for &hard in hards {
                        if hard && c.is_none() {
                            continue;
                        }
                        out.push(Choice { click: c.clone(), stop, hard, ..Choice::gather(p) });
                    }
                }
            }
        }
        // Construir antes de ir a trabajar (la "izquierda" del árbol).
        for (b, n) in self.build_options(sim) {
            out.push(Choice { build: Some(b.to_owned()), builders: n, ..Choice::gather("forest") });
        }
        out
    }

    /// Construcciones que vale la pena proponer en esta decisión.
    fn build_options(&self, sim: &Sim) -> Vec<(&'static str, u32)> {
        let s = &sim.state;
        let mut out = Vec::new();
        let houses_building = s.sites.iter().filter(|x| x.building == aoe2_data::ids::HOUSE && !x.done).count() as u32;
        let room = sim.pop_capacity() + houses_building * self.sc.rules.population.per_house;
        let pop = s.pop() + s.tcs.len() as u32;
        if room < self.sc.rules.population.max && room.saturating_sub(pop) <= 3 {
            out.push(("house", 1));
            out.push(("house", 2));
        }
        let boom = self.goal.villagers.is_some() || !self.goal.units.is_empty();
        if boom && sim.can_build("town_center") && sim.planned("town_center") < 3 {
            out.push(("town_center", 3));
            out.push(("town_center", 5));
        }
        out
    }

    fn apply(&self, sim: &mut Sim, c: &Choice) {
        if c.stop {
            let n = sim.state.villagers.len() as u32;
            sim.set_max_villagers(Some(n));
        }
        if let Some(t) = &c.click {
            sim.queue_research(t, !c.hard);
        }
        match &c.build {
            Some(b) => sim.decide_build(b, c.builders.max(1), &c.pool),
            None => sim.decide(&c.pool),
        }
    }

    /// Tiempo estimado para cumplir el objetivo desde este estado.
    ///
    /// Proyección macro en saltos de 25 s: los recursos crecen con el ingreso
    /// actual; mientras el CU crea aldeanos paga 50 de comida por cada uno y
    /// el nuevo aldeano se suma al recurso que más frena el objetivo. Si en un
    /// salto no hay comida para el aldeano, se penaliza (CU parado).
    fn estimate_macro(&self, sim: &Sim) -> f64 {
        const STEP: f64 = 25.0;
        let s = &sim.state;
        let need = self.goal.remaining_cost(&self.sc, s);
        let need = [need.food as f64, need.wood as f64 + WOOD_RESERVE, need.gold as f64, need.stone as f64];
        let inc0 = sim.income();
        let mut inc = [inc0.food, inc0.wood, inc0.gold, inc0.stone];
        let mut stock = [s.res.food, s.res.wood, s.res.gold, s.res.stone];
        let v = self.sc.data.unit(aoe2_data::ids::VILLAGER).unwrap();
        let vcost = v.cost.food as f64;
        let mut villagers = s.villagers.len() as u32;
        let max_v = sim.max_villagers().or(self.goal.villagers.filter(|_| self.goal.units.is_empty()).map(|n| n.max(villagers)));
        let mut tc_free_in = match s.tc_age_research() {
            Some((tech, left)) if self.sc.rules.age_of_tech(tech).is_some() => left as f64,
            _ => 0.0,
        };
        // Sin clic pedido, el CU hará al menos un aldeano más antes de poder
        // subir (la próxima decisión llega con el siguiente aldeano).
        let next_tech = next_age(s.age).filter(|a| *a <= self.target).and_then(|a| self.sc.rules.age_tech(a));
        let clicked = next_tech.is_none_or(|t| s.pending_research.contains(&t) || s.has_tech(t));
        let mut t = 0.0;
        let mut penalty = 0.0;
        for step in 0..200 {
            let enough = (0..4).all(|i| stock[i] + 1e-6 >= need[i]);
            if enough && tc_free_in <= 0.0 && (clicked || step > 0) {
                break;
            }
            // El CU (si no investiga) crea un aldeano en este salto.
            let making = tc_free_in <= 0.0 && max_v.is_none_or(|m| villagers < m);
            if making {
                if stock[0] >= vcost {
                    stock[0] -= vcost;
                    villagers += 1;
                    // El nuevo aldeano va al recurso con más déficit relativo
                    // (contando la comida de los próximos aldeanos).
                    let mut best = 0;
                    let mut worst = f64::MIN;
                    for i in 0..4 {
                        if self.per_villager[i] <= 0.0 {
                            continue;
                        }
                        let reserve = if i == 0 { 2.0 * vcost } else { 0.0 };
                        let deficit = need[i] + reserve - stock[i];
                        let score = deficit / (inc[i] + self.per_villager[i]);
                        if score > worst {
                            worst = score;
                            best = i;
                        }
                    }
                    inc[best] += self.per_villager[best];
                } else {
                    penalty += STEP * 4.0;
                }
            }
            for i in 0..4 {
                stock[i] += inc[i] * STEP;
            }
            tc_free_in -= STEP;
            t += STEP;
        }
        // Investigaciones de edad que faltan después de tener los recursos.
        let mut ages_after = 0.0;
        let mut age = s.age;
        while let Some(a) = next_age(age).filter(|a| *a <= self.target) {
            let tech = self.sc.rules.age_tech(a).unwrap();
            let started = s.tc_researching(tech);
            if !started {
                ages_after += self.sc.data.tech(tech).unwrap().research_time as f64;
            }
            age = a;
        }
        let mut est = s.time as f64 + t + ages_after + penalty;
        for u in &self.goal.units {
            let id = self.sc.unit_ref(&u.unit).unwrap();
            let left = u.count.saturating_sub(s.units_of(id));
            let train = self.sc.data.unit(id).unwrap().train_time as f64;
            est += (left as f64 / 2.0).ceil() * train * 0.5;
        }
        if let Some(n) = self.goal.villagers {
            let left = n.saturating_sub(s.villagers.len() as u32) as f64;
            est = est.max(s.time as f64 + left * 25.0);
        }
        est
    }

    /// Estimación simple: déficit de cada recurso ÷ ingreso neto actual
    /// (descontando la comida de los aldeanos) + investigaciones de edad.
    fn estimate_simple(&self, sim: &Sim) -> f64 {
        let s = &sim.state;
        let need = self.goal.remaining_cost(&self.sc, s);
        let mut inc = sim.income();
        if s.villagers_in_production() > 0 && sim.max_villagers().is_none() {
            let v = self.sc.data.unit(aoe2_data::ids::VILLAGER).unwrap();
            inc.food -= v.cost.food as f64 / v.train_time as f64;
        }
        let mut t_res: f64 = 0.0;
        for (need, have, rate) in [
            (need.food, s.res.food, inc.food),
            (need.wood, s.res.wood, inc.wood),
            (need.gold, s.res.gold, inc.gold),
            (need.stone, s.res.stone, inc.stone),
        ] {
            let deficit = need as f64 - have;
            if deficit > 0.0 {
                t_res = t_res.max(if rate > 1e-6 { deficit / rate } else { 1e5 + deficit });
            }
        }
        let mut tc_left = 0.0;
        let mut ages_after = 0.0;
        let mut age = s.age;
        while let Some(a) = next_age(age).filter(|a| *a <= self.target) {
            let tech = self.sc.rules.age_tech(a).unwrap();
            match s.tc_age_research() {
                Some((t, left)) if t == tech => tc_left = left as f64,
                _ => ages_after += self.sc.data.tech(tech).unwrap().research_time as f64,
            }
            age = a;
        }
        s.time as f64 + t_res.max(tc_left) + ages_after
    }

    fn estimate(&self, sim: &Sim, h: Heuristic) -> f64 {
        match h {
            Heuristic::Simple => self.estimate_simple(sim),
            Heuristic::Macro => self.estimate_macro(sim),
        }
    }

    /// ¿El plan respeta la regla del CU? Nunca sin casa ni sin comida; esperar
    /// para investigar una edad solo si el objetivo lo permite.
    fn tc_ok(&self, sim: &Sim) -> bool {
        use aoe2_sim::IdleCause::*;
        if self.relaxed {
            return true;
        }
        sim.summary().tc_idle_seconds.iter().all(|(cause, secs)| match cause {
            NoOrder => true,
            WaitingResearch => self.goal.allow_tc_idle || *secs == 0,
            Housed | NoFood => *secs == 0,
        })
    }

    fn signature(sim: &Sim, choices: &[Choice]) -> String {
        let mut counts = vec![0u32; sim.pool_names().len()];
        for v in &sim.state.villagers {
            counts[v.pool] += 1;
        }
        let clicks: Vec<usize> = choices.iter().enumerate().filter(|(_, c)| c.click.is_some()).map(|(i, _)| i).collect();
        let stop = choices.iter().position(|c| c.stop);
        format!("{}|{:?}|{:?}|{:?}|{:?}", sim.state.time, counts, clicks, stop, sim.state.age)
    }

    fn count_sim(&self) {
        self.simulations.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Simula una secuencia de decisiones desde el principio, o desde un punto
    /// de control (`from` = índice de la decisión y estado justo antes de ella).
    /// Si se acaban las decisiones, repite el último pool (sin clic). Con
    /// `record`, guarda una copia del simulador antes de cada decisión.
    fn simulate_choices<'s>(&'s self, choices: &[Choice], from: Option<(usize, &Sim<'s>)>, record: bool) -> Run<'s> {
        self.count_sim();
        let sc: &Scenario = &self.sc;
        let (mut sim, mut i) = match from {
            Some((i, cp)) => (cp.clone(), i),
            None => (Sim::new_external(sc).expect("orden base válida"), 0),
        };
        let done = |s: &GameState| self.goal.is_done(sc, s);
        let mut decisions = Vec::new();
        let mut checkpoints = Vec::new();
        let advance = |sim: &mut Sim<'s>| {
            if sim.pending_decision().is_some() { sim.pending_decision() } else { sim.advance(self.cfg.max_seconds, &done) }
        };
        while let Some(v) = advance(&mut sim) {
            let c = choices
                .get(i)
                .cloned()
                .unwrap_or_else(|| Choice::gather(&choices.last().map_or("forest".to_owned(), |c| c.pool.clone())));
            if record {
                checkpoints.push(sim.clone());
                decisions.push(Decision { villager: v, time: sim.state.time, choice: c.clone(), state: sim.current_snapshot() });
            }
            self.apply(&mut sim, &c);
            i += 1;
        }
        let reached = done(&sim.state).then_some(sim.state.time);
        let time = reached.filter(|_| self.tc_ok(&sim));
        let tc_idle = sim
            .summary()
            .tc_idle_seconds
            .iter()
            .filter(|(c, _)| **c != aoe2_sim::IdleCause::NoOrder)
            .map(|(_, s)| *s)
            .sum();
        let clicks = sim.summary().age_clicks.clone();
        Run { time, reached, tc_idle, clicks, decisions, checkpoints }
    }

    /// Simula un plan completo y devuelve su línea de tiempo (para comparar
    /// caminos en la interfaz).
    pub fn plan_timeline(&self, choices: &[Choice]) -> (Option<u32>, Timeline) {
        let sc: &Scenario = &self.sc;
        let mut sim = Sim::new_external(sc).expect("orden base válida");
        let done = |s: &GameState| self.goal.is_done(sc, s);
        let mut i = 0;
        while sim.advance(self.cfg.max_seconds, &done).is_some() {
            let c = choices
                .get(i)
                .cloned()
                .unwrap_or_else(|| Choice::gather(&choices.last().map_or("forest".to_owned(), |c| c.pool.clone())));
            self.apply(&mut sim, &c);
            i += 1;
        }
        let time = (done(&sim.state) && self.tc_ok(&sim)).then_some(sim.state.time);
        (time, sim.into_timeline())
    }

    /// Tiempo de `choices`, reutilizando el punto de control anterior a la
    /// primera decisión que cambia (`k`).
    fn time_from<'s>(&'s self, base: &Run<'s>, choices: &[Choice], k: usize) -> Option<u32> {
        match base.checkpoints.get(k) {
            Some(cp) => self.simulate_choices(choices, Some((k, cp)), false).time,
            None => self.simulate_choices(choices, None, false).time,
        }
    }

    fn resource_index(&self, pool: &str) -> Option<usize> {
        let src = self.sc.map.sources.iter().find(|x| x.pool() == pool)?;
        use aoe2_data::rates::Resource::*;
        Some(match self.sc.rates.task(&src.task)?.resource {
            Food => 0,
            Wood => 1,
            Gold => 2,
            Stone => 3,
        })
    }

    /// Decisión de la política voraz: el aldeano va al recurso que más frena
    /// el objetivo (déficit ÷ ingreso), a la primera fuente disponible de ese
    /// recurso; clic suave al llegar a `click_at` aldeanos.
    fn greedy(&self, sim: &Sim, p: &Policy) -> Choice {
        let s = &sim.state;
        let need = self.goal.stage_cost(&self.sc, s);
        let v = self.sc.data.unit(aoe2_data::ids::VILLAGER).unwrap();
        let making = sim.max_villagers().is_none_or(|m| (s.villagers.len() as u32) < m);
        let reserve = if making { p.food_reserve * v.cost.food as f64 } else { 0.0 };
        // Madera de reserva para casas y granjas, que no cuentan en el objetivo.
        let need = [need.food as f64 + reserve, need.wood as f64 + WOOD_RESERVE, need.gold as f64, need.stone as f64];
        let inc = sim.income();
        let inc = [inc.food, inc.wood, inc.gold, inc.stone];
        let stock = [s.res.food, s.res.wood, s.res.gold, s.res.stone];
        let mut best = 0;
        let mut worst = f64::MIN;
        for i in 0..4 {
            if self.per_villager[i] <= 0.0 {
                continue;
            }
            let score = (need[i] - stock[i]) / (inc[i] + self.per_villager[i]);
            if score > worst {
                worst = score;
                best = i;
            }
        }
        // Primera fuente disponible de ese recurso (en el orden del mapa);
        // las granjas solo si no queda otra comida.
        let farm_pool = self.sc.map.sources.iter().find(|x| x.task == "farmer").map(|x| x.pool().to_owned());
        let mut pool = None;
        for src in &self.sc.map.sources {
            let p = src.pool();
            if self.resource_index(p) != Some(best) || Some(p.to_owned()) == farm_pool {
                continue;
            }
            if src.arrives_at_s <= s.time + 10 && sim.pool_has_resources(p) {
                pool = Some(p.to_owned());
                break;
            }
        }
        let pool = pool.or_else(|| if best == 0 { farm_pool.clone() } else { None }).unwrap_or_else(|| "forest".to_owned());

        let villagers = s.villagers.len() as u32;
        let click = next_age(s.age).filter(|a| *a <= self.target).and_then(|a| {
            let tech = self.sc.rules.age_tech(a)?;
            let i = match a {
                Age::Feudal => 0,
                Age::Castle => 1,
                _ => 2,
            };
            let pending = s.pending_research.contains(&tech) || s.tc_researching(tech);
            (!pending && villagers >= p.click_at[i]).then(|| self.sc.rules.tech_name(tech).unwrap().to_owned())
        });
        let stop = p.stop_at.is_some_and(|n| villagers >= n) && sim.max_villagers().is_none() && s.age >= self.target;
        let hard = click.is_some() && p.hard_clicks;
        // CU adicionales (objetivos de muchos aldeanos o unidades), ya en Castillos.
        let tc_cost = self.sc.data.building(aoe2_data::ids::TOWN_CENTER).map(|b| b.cost).unwrap_or_default();
        if click.is_none()
            && (sim.planned("town_center") as u32) < 1 + p.extra_tcs
            && sim.can_build("town_center")
            && s.res.can_afford(&tc_cost)
        {
            return Choice { build: Some("town_center".into()), builders: 3, ..Choice::gather(&pool) };
        }
        Choice { click, stop, hard, ..Choice::gather(&pool) }
    }

    /// Ejecuta la política completa; devuelve el plan (decisiones tomadas).
    fn run_policy(&self, p: &Policy) -> Plan {
        self.count_sim();
        let sc: &Scenario = &self.sc;
        let mut sim = Sim::new_external(sc).expect("orden base válida");
        let done = |s: &GameState| self.goal.is_done(sc, s);
        let mut choices = Vec::new();
        while sim.advance(self.cfg.max_seconds, &done).is_some() {
            let c = self.greedy(&sim, p);
            self.apply(&mut sim, &c);
            choices.push(c);
        }
        let time = (done(&sim.state) && self.tc_ok(&sim)).then_some(sim.state.time);
        Plan { time, choices }
    }

    /// Descenso coordenado sobre los parámetros de la política.
    fn policy_search(&self) -> Vec<Plan> {
        let mut best_p = Policy::default();
        if !self.goal.units.is_empty() {
            best_p.stop_at = Some(30);
        }
        let mut best = self.run_policy(&best_p);
        let mut all = vec![best.clone()];
        let key = |pl: &Plan| pl.time.unwrap_or(u32::MAX);
        for _round in 0..2 {
            let mut variants: Vec<Policy> = Vec::new();
            for n in 12..=32 {
                variants.push(Policy { click_at: [n, best_p.click_at[1].max(n), best_p.click_at[2].max(n)], ..best_p });
            }
            if self.target >= Age::Castle {
                for k in 0..=8 {
                    let c = best_p.click_at[0] + k;
                    variants.push(Policy { click_at: [best_p.click_at[0], c, best_p.click_at[2].max(c)], ..best_p });
                }
            }
            if self.target >= Age::Imperial {
                for k in (0..=16).step_by(2) {
                    variants.push(Policy { click_at: [best_p.click_at[0], best_p.click_at[1], best_p.click_at[1] + k], ..best_p });
                }
            }
            for r in [0.5, 1.0, 2.0, 3.0, 4.0] {
                variants.push(Policy { food_reserve: r, ..best_p });
            }
            if self.goal.allow_tc_idle {
                variants.push(Policy { hard_clicks: !best_p.hard_clicks, ..best_p });
            }
            if self.goal.villagers.is_some() || !self.goal.units.is_empty() {
                for n in 0..=2 {
                    variants.push(Policy { extra_tcs: n, ..best_p });
                }
            }
            if best_p.stop_at.is_some() {
                for n in (15..=50).step_by(5) {
                    variants.push(Policy { stop_at: Some(n), ..best_p });
                }
            }
            for v in variants {
                let plan = self.run_policy(&v);
                if key(&plan) < key(&best) {
                    best = plan.clone();
                    best_p = v;
                }
                if plan.time.is_some() {
                    all.push(plan);
                }
            }
        }
        all.sort_by(|a, b| key(a).cmp(&key(b)).then_with(|| a.choices.cmp(&b.choices)));
        all
    }

    /// Devuelve los planes terminados (mejor primero) y cuántas simulaciones hizo.
    /// `bound`: tiempo de un plan ya conocido; se descartan nodos que no pueden mejorarlo.
    fn beam(&self, h: Heuristic, bound: Option<u32>, beam_width: usize) -> (Vec<Plan>, usize) {
        let sc: &Scenario = &self.sc;
        let mut root = Sim::new_external(sc).expect("orden base válida");
        let done = self.done_fn();
        root.advance(self.cfg.max_seconds, &done);
        let est = self.estimate(&root, h);
        let mut beam = vec![Node { sim: root, choices: Vec::new(), est }];
        let mut finished: Vec<Plan> = Vec::new();
        let mut sims = 0;
        // Objetivos largos (muchas decisiones): haz más estrecho para acotar el tiempo.
        let levels = bound.map_or(30, |b| (b / 25).max(1) as usize);
        let width = beam_width.min((beam_width * 30 / levels).max(16));
        while !beam.is_empty() {
            // Expandir todos los hijos en paralelo; los resultados se juntan en
            // el mismo orden que las tareas, así que el resultado es determinista.
            let tasks: Vec<(usize, Choice)> = beam
                .iter()
                .enumerate()
                .filter(|(_, n)| n.sim.pending_decision().is_some())
                .flat_map(|(i, n)| self.options(&n.sim).into_iter().map(move |c| (i, c)))
                .collect();
            sims += tasks.len();
            let threads = std::thread::available_parallelism().map_or(1, |n| n.get()).min(tasks.len().max(1));
            let chunk = tasks.len().div_ceil(threads).max(1);
            let beam_ref = &beam;
            let done_ref = &done;
            let outcomes: Vec<Result<Node, Plan>> = std::thread::scope(|scope| {
                let handles: Vec<_> = tasks
                    .chunks(chunk)
                    .map(|part| {
                        scope.spawn(move || {
                            part.iter()
                                .filter_map(|(i, c)| {
                                    let node = &beam_ref[*i];
                                    let mut sim = node.sim.clone();
                                    self.apply(&mut sim, c);
                                    sim.advance(self.cfg.max_seconds, done_ref);
                                    let mut choices = node.choices.clone();
                                    choices.push(c.clone());
                                    if sim.pending_decision().is_none() {
                                        // Terminó: objetivo cumplido o sin tiempo.
                                        let ok = done_ref(&sim.state) && self.tc_ok(&sim);
                                        return Some(Err(Plan { time: ok.then_some(sim.state.time), choices }));
                                    }
                                    if bound.is_some_and(|b| sim.state.time >= b) {
                                        return None;
                                    }
                                    let est = self.estimate(&sim, h);
                                    Some(Ok(Node { sim, choices, est }))
                                })
                                .collect::<Vec<_>>()
                        })
                    })
                    .collect();
                handles.into_iter().flat_map(|h| h.join().expect("hilo del haz")).collect()
            });
            let mut children: Vec<Node> = Vec::new();
            for o in outcomes {
                match o {
                    Ok(n) => children.push(n),
                    Err(p) => finished.push(p),
                }
            }
            // Mejores primero; en empate, la secuencia (determinista).
            // Primero los que no dejaron el CU parado; si no hay ninguno, los que
            // menos (para no vaciar el haz). Esos planes no valen como resultado.
            let idle = |n: &Node| if self.tc_ok(&n.sim) { 0 } else { n.sim.summary().tc_idle_total().max(1) };
            let min_idle = children.iter().map(idle).min().unwrap_or(0);
            children.retain(|n| idle(n) == min_idle);
            // Desempate por el orden de las fuentes en el mapa (ovejas, jabalí,
            // bayas…: el orden natural de una partida), no alfabético.
            let rank = |n: &Node| -> Vec<usize> {
                n.choices
                    .iter()
                    .map(|c| self.pools.iter().position(|p| *p == c.pool).unwrap_or(usize::MAX) * 4 + c.build.is_some() as usize * 2 + c.click.is_some() as usize)
                    .collect()
            };
            children.sort_by(|a, b| a.est.total_cmp(&b.est).then_with(|| rank(a).cmp(&rank(b))));
            let mut seen = BTreeSet::new();
            beam = children
                .into_iter()
                .filter(|n| seen.insert(Self::signature(&n.sim, &n.choices)))
                .take(width)
                .collect();
            if std::env::var_os("IDLEZERO_DEBUG").is_some() {
                let b = beam.first();
                eprintln!(
                    "nivel: {} nodos, terminados {}, mejor est {:?} t={:?} edad={:?} elecciones={}",
                    beam.len(),
                    finished.len(),
                    b.map(|n| n.est as u32),
                    b.map(|n| n.sim.state.time),
                    b.map(|n| n.sim.state.age),
                    b.map_or(0, |n| n.choices.len())
                );
            }
            // (Sin corte por estimación: la heurística no es exacta y podaría
            // caminos buenos. La poda por tiempo frente a la cota ya vacía el haz.)
        }
        finished.sort_by(|a, b| {
            a.time.unwrap_or(u32::MAX).cmp(&b.time.unwrap_or(u32::MAX)).then_with(|| a.choices.cmp(&b.choices))
        });
        (finished, sims)
    }

    /// Búsqueda local: cambios de una decisión, intercambios y mover el clic.
    /// Cada variante se simula desde el punto de control de la primera
    /// decisión que cambia.
    fn refine(&self, mut best: Plan) -> Plan {
        let mut budget = self.cfg.refine_budget;
        let mut run = self.simulate_choices(&best.choices, None, true);
        'outer: loop {
            let n = best.choices.len();
            // (primera decisión que cambia, nueva secuencia)
            let mut candidates: Vec<(usize, Vec<Choice>)> = Vec::new();
            for i in 0..n {
                if best.choices[i].click.is_some() {
                    for j in [i.wrapping_sub(1), i + 1] {
                        if j < n && best.choices[j].click.is_none() {
                            let mut c = best.choices.clone();
                            let click = c[i].click.take();
                            c[j].click = click;
                            c[j].hard = c[i].hard;
                            candidates.push((i.min(j), c));
                        }
                    }
                }
                if i + 1 < n && best.choices[i].pool != best.choices[i + 1].pool {
                    let mut c = best.choices.clone();
                    let p = c[i].pool.clone();
                    c[i].pool = c[i + 1].pool.clone();
                    c[i + 1].pool = p;
                    candidates.push((i, c));
                }
                for p in &self.pools {
                    if *p != best.choices[i].pool && self.pool_relevant(p) {
                        let mut c = best.choices.clone();
                        c[i].pool = p.clone();
                        candidates.push((i, c));
                    }
                }
            }
            for (k, c) in candidates {
                if budget == 0 {
                    break 'outer;
                }
                budget -= 1;
                let time = self.time_from(&run, &c, k);
                if time.unwrap_or(u32::MAX) < best.time.unwrap_or(u32::MAX) {
                    best = Plan { time, choices: c };
                    run = self.simulate_choices(&best.choices, None, true);
                    continue 'outer;
                }
            }
            break;
        }
        best
    }

    /// Un clic suave da igual en cualquier decisión anterior a la subida real.
    /// Para que la ruta se lea bien, se mueve a la última decisión antes de
    /// que empiece la investigación (si el resultado no cambia).
    fn normalize_clicks(&self, mut best: Plan) -> Plan {
        // La simulación completa decisiones que falten repitiendo la última:
        // dejar el plan con la lista completa para que los índices coincidan.
        best.choices = self.simulate_choices(&best.choices, None, true).decisions.into_iter().map(|d| d.choice).collect();
        for _ in 0..3 {
            let run = self.simulate_choices(&best.choices, None, true);
            let mut changed = false;
            for i in 0..best.choices.len() {
                let Some(tech) = best.choices[i].click.clone() else { continue };
                if best.choices[i].hard {
                    continue;
                }
                let Some(age) = self.sc.tech_ref(&tech).and_then(|t| self.sc.rules.age_of_tech(t)) else { continue };
                let Some(&started) = run.clicks.get(&age) else { continue };
                let Some(j) = run.decisions.iter().rposition(|d| d.time <= started) else { continue };
                if j <= i || j >= best.choices.len() || best.choices[j].click.is_some() {
                    continue;
                }
                let mut c = best.choices.clone();
                c[j].click = c[i].click.take();
                if self.simulate_choices(&c, None, false).time == best.time {
                    best.choices = c;
                    changed = true;
                    break;
                }
            }
            if !changed {
                break;
            }
        }
        best
    }

    /// Pools que tiene sentido proponer (existen en el mapa y el objetivo
    /// necesita su recurso; las granjas siempre).
    fn pool_relevant(&self, pool: &str) -> bool {
        if !self.sc.map.sources.iter().any(|s| s.pool() == pool) {
            return false;
        }
        self.resource_index(pool) != Some(3) || self.needs_stone
    }

    /// Árbol: la mejor ruta y, en cada decisión, el costo de cambiarla.
    fn tree(&self, best: &Plan, run: &Run) -> Vec<TreeNode> {
        let best_time = best.time.map(|t| t as i64);
        let mut nodes = Vec::new();
        for (i, d) in run.decisions.iter().enumerate() {
            let mut alternatives = Vec::new();
            let mut eval = |choices: Vec<Choice>, alt: Choice| {
                let r = match run.checkpoints.get(i) {
                    Some(cp) => self.simulate_choices(&choices, Some((i, cp)), false),
                    None => self.simulate_choices(&choices, None, false),
                };
                let time = r.reached;
                let delta = time.zip(best_time).map(|(t, b)| t as i64 - b);
                alternatives.push(Alternative { choice: alt, time, delta, tc_idle: r.tc_idle });
            };
            for p in &self.pools {
                if *p != d.choice.pool && self.pool_relevant(p) {
                    let alt = Choice { pool: p.clone(), ..d.choice.clone() };
                    let mut c = best.choices.clone();
                    if i < c.len() {
                        c[i] = alt.clone();
                    }
                    eval(c, alt);
                }
            }
            // Clic duro: adelantarlo a esta decisión (uno suave da igual dónde se pida).
            if let Some(j) = best.choices.iter().position(|c| c.click.is_some() && c.hard).filter(|j| *j > i) {
                let mut c = best.choices.clone();
                let click = c[j].click.take();
                c[i].click = click;
                c[i].hard = c[j].hard;
                let alt = c[i].clone();
                eval(c, alt);
            }
            alternatives.sort_by_key(|a| (a.delta.unwrap_or(i64::MAX), a.tc_idle, a.choice.clone()));
            nodes.push(TreeNode { index: i, decision: d.clone(), alternatives });
        }
        nodes
    }

    pub fn run(self) -> OptResult {
        let dbg = std::env::var_os("IDLEZERO_DEBUG").is_some();
        let t0 = std::time::Instant::now();
        let lap = |what: &str| {
            if dbg {
                eprintln!("[{:>6.2} s] {what}", t0.elapsed().as_secs_f64());
            }
        };
        let mut finished = self.policy_search();
        // Semillas (p. ej. el mejor plan neutro re-simulado con esta civilización).
        let no_houses = self.sc.civ.as_ref().is_some_and(|c| c.no_houses());
        for seed in self.seeds.clone() {
            // Sin casas que construir para quien no las necesita.
            let seed: Vec<Choice> = seed
                .into_iter()
                .map(|c| if no_houses && c.build.as_deref() == Some("house") { Choice { build: None, builders: 0, ..c } } else { c })
                .collect();
            let time = self.simulate_choices(&seed, None, false).time;
            finished.push(Plan { time, choices: seed });
        }
        lap("políticas");
        // Dos heurísticas y dos anchos de haz: la búsqueda en haz no es
        // monótona (más ancho no siempre es mejor), así que se combinan.
        let short = finished.iter().filter_map(|p| p.time).min().or(self.hint_bound).is_some_and(|t| t / 25 <= 26);
        let mut passes = vec![
            (Heuristic::Simple, self.cfg.beam_width),
            (Heuristic::Macro, self.cfg.beam_width),
            (Heuristic::Simple, self.cfg.beam_width * 2),
            (Heuristic::Macro, self.cfg.beam_width * 2),
        ];
        // Objetivos cortos (pocas decisiones): una pasada más ancha sale barata.
        if short {
            passes.push((Heuristic::Simple, self.cfg.beam_width * 4));
        }
        for (h, w) in passes {
            let bound = finished.iter().filter_map(|p| p.time).min().or(self.hint_bound);
            let (plans, sims) = self.beam(h, bound, w);
            self.simulations.fetch_add(sims, std::sync::atomic::Ordering::Relaxed);
            finished.extend(plans);
            lap(&format!(
                "haz {h:?} ancho {w}: mejor {:?}",
                finished.iter().filter_map(|p| p.time).min()
            ));
        }
        let key = |pl: &Plan| pl.time.unwrap_or(u32::MAX);
        finished.sort_by(|a, b| key(a).cmp(&key(b)).then_with(|| a.choices.cmp(&b.choices)));
        let first = finished.first().cloned().unwrap_or(Plan { time: None, choices: Vec::new() });
        let best = self.normalize_clicks(self.refine(first));
        lap("refinamiento");
        let run = self.simulate_choices(&best.choices, None, true);
        let best = Plan { time: run.time, ..best };
        let tree = self.tree(&best, &run);
        drop(run);
        lap("árbol");

        // Caminos distintos de verdad: otro reparto de aldeanos o clic en otro momento.
        let shape = |p: &Plan| {
            let mut counts: std::collections::BTreeMap<&str, u32> = Default::default();
            for c in &p.choices {
                *counts.entry(c.pool.as_str()).or_default() += 1;
            }
            let clicks: Vec<usize> = p.choices.iter().enumerate().filter(|(_, c)| c.click.is_some()).map(|(i, _)| i).collect();
            format!("{counts:?}{clicks:?}{}", p.choices.len())
        };
        let mut plans = vec![best.clone()];
        let mut shapes = BTreeSet::from([shape(&best)]);
        // Primero hasta 2 con el mismo tiempo que el mejor, luego otros tiempos.
        let same_time = |p: &Plan| p.time == best.time;
        for pass in 0..2 {
            for p in &finished {
                let quota = if pass == 0 { 3 } else { self.cfg.plans };
                if plans.len() >= quota.min(self.cfg.plans) {
                    break;
                }
                if p.time.is_some() && (pass == 1 || same_time(p)) && (pass == 0 || !same_time(p)) && shapes.insert(shape(p)) {
                    plans.push(p.clone());
                }
            }
        }
        plans[1..].sort_by_key(|p| p.time.unwrap_or(u32::MAX));

        let order = self.to_order(&best);
        let order_toml = toml::to_string(&order).unwrap_or_default();
        let sc = Scenario { order, ..self.sc.clone() };
        let stop_goal = self.goal.clone();
        let timeline = {
            let mut sim = Sim::new(&sc).expect("orden generada válida");
            let s2 = sc.clone();
            sim.advance(self.cfg.max_seconds, &|s| stop_goal.is_done(&s2, s));
            sim.into_timeline()
        };
        OptResult {
            goal: self.goal.clone(),
            map: self.sc.map.name.clone(),
            best,
            order_toml,
            timeline,
            tree,
            plans,
            simulations: self.simulations.load(std::sync::atomic::Ordering::Relaxed),
            relaxed: self.relaxed,
            neutral_time: None,
        }
    }

    /// Convierte un plan en una orden de construcción normal.
    pub fn to_order(&self, plan: &Plan) -> BuildOrder {
        let decisions = self.simulate_choices(&plan.choices, None, true).decisions;
        let mut order = self.sc.order.clone();
        let initial = self.sc.map.start.villagers;
        for d in &decisions {
            let assign = match &d.choice.build {
                Some(b) => Assign {
                    pool: None,
                    build: Some(b.clone()),
                    then: Some(d.choice.pool.clone()),
                    at: None,
                    builders: Some(d.choice.builders.max(1)),
                },
                None => Assign { pool: Some(d.choice.pool.clone()), build: None, then: None, at: None, builders: None },
            };
            if d.villager <= initial {
                order.start.push(aoe2_sim::orders::StartAssign { villagers: 1, assign });
            } else {
                order.villagers.push(VillagerAssign { n: d.villager, to: None, assign });
            }
            if let Some(t) = &d.choice.click {
                order.steps.push(Step {
                    when: Condition { villagers: Some(d.villager), ..Condition::default() },
                    build: None,
                    builders: 1,
                    from: None,
                    then: None,
                    at: None,
                    research: Some(t.clone()),
                    soft: !d.choice.hard,
                    to: None,
                    count: None,
                    lure: None,
                    train: None,
                });
            }
            if d.choice.stop {
                order.max_villagers = Some(d.villager);
            }
        }
        order
    }
}
