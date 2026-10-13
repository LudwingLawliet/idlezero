//! El paso de 1 segundo. Siempre en el mismo orden, sin aleatoriedad:
//! 1. hitos de la orden, 2. CU, 3. otras investigaciones, 4. aldeanos,
//! 5. construcciones, 6. fuentes agotadas, 7. foto por tick.

use crate::orders::{Condition, StepAction};
use crate::scenario::Scenario;
use crate::state::*;
use crate::timeline::*;
use aoe2_data::modifiers::ModifierKind;
use aoe2_data::{Age, Cost, ids};
use std::collections::BTreeMap;

/// Cuándo parar la simulación.
#[derive(Debug, Clone, Copy)]
pub struct Stop {
    pub max_seconds: u32,
    /// Parar al alcanzar esta edad.
    pub age: Option<Age>,
}

#[derive(Clone)]
pub struct Sim<'a> {
    sc: &'a Scenario,
    /// Si es true, los aldeanos sin asignación en la orden esperan una
    /// decisión externa (el optimizador) en lugar de ir al pool por defecto.
    external: bool,
    /// Aldeanos que esperan decisión externa, en orden.
    decisions: std::collections::VecDeque<u32>,
    born_before_tick: u32,
    max_villagers: Option<u32>,
    builders_pool: usize,
    /// Puntos de entrega encargados por `auto.drop_sites` sin colocar: (edificio, pool).
    pending_drop_sites: Vec<(u32, usize)>,
    fallback: Vec<(usize, usize)>,
    pub state: GameState,
    pools: Vec<String>,
    farm_pool: Option<usize>,
    steps: Vec<StepState>,
    villager_speed: f64,
    events: Vec<TimedEvent>,
    ticks: Vec<Snapshot>,
    summary: Summary,
    /// Racha de inactividad de cada CU: (causa, desde).
    idle_runs: Vec<Option<(IdleCause, u32)>>,
    exhausted: Vec<bool>,
    /// Constructores forzados por nº de edificio colocado (hipótesis "¿y si…?").
    builder_overrides: BTreeMap<usize, u32>,
    /// Edificios colocados hasta ahora (numera cada obra).
    placements: usize,
    series: Series,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum StepState {
    Waiting,
    /// Condición cumplida; la acción espera recursos o requisitos.
    Triggered,
    Done,
}

impl<'a> Sim<'a> {
    pub fn new(sc: &'a Scenario) -> Result<Sim<'a>, String> {
        let mut pools: Vec<String> = Vec::new();
        let pool_id = |name: &str, pools: &mut Vec<String>| match pools.iter().position(|p| p == name) {
            Some(i) => i,
            None => {
                pools.push(name.to_owned());
                pools.len() - 1
            }
        };

        let mut sources = Vec::new();
        let mut farm_pool = None;
        for s in &sc.map.sources {
            let task = sc
                .rates
                .tasks
                .iter()
                .position(|t| t.id == s.task)
                .ok_or(format!("fuente {}: tarea desconocida {}", s.id, s.task))?;
            let pool = pool_id(s.pool(), &mut pools);
            if s.task == "farmer" {
                farm_pool = Some(pool);
                continue;
            }
            sources.push(SourceState {
                id: s.id.clone(),
                pool,
                task,
                remaining: s.amount.map_or(f64::INFINITY, |a| (a * s.count) as f64),
                distance: s.distance_tiles,
                distance_with_drop_site: s.distance_with_drop_site,
                drop_site: s.drop_site,
                has_drop_site: false,
                drop_site_planned: false,
                arrives_at: s.arrives_at_s,
                lure_seconds: s.lure_seconds,
                lured: false,
                luring: false,
            });
        }

        // Validar la orden: todos los pools y nombres deben existir.
        let order = &sc.order;
        let known_pool = |name: &str| pools.iter().any(|p| p == name);
        let check_pool = |name: &str| -> Result<(), String> {
            if known_pool(name) { Ok(()) } else { Err(format!("pool desconocido en la orden: {name}")) }
        };
        let check_building = |name: &str| -> Result<u32, String> {
            sc.building_ref(name).ok_or(format!("edificio desconocido en la orden: {name}"))
        };
        check_pool(&order.default_pool)?;
        for a in order.start.iter().map(|s| &s.assign).chain(order.villagers.iter().map(|v| &v.assign)) {
            a.validate()?;
            for p in [&a.pool, &a.then, &a.at].into_iter().flatten() {
                check_pool(p)?;
            }
            if let Some(b) = &a.build {
                check_building(b)?;
            }
        }
        for (i, step) in order.steps.iter().enumerate() {
            match step.action().map_err(|e| format!("paso {i}: {e}"))? {
                StepAction::Build { building, from, then, at, .. } => {
                    check_building(building)?;
                    check_pool(from)?;
                    check_pool(then)?;
                    if let Some(at) = at {
                        check_pool(at)?;
                    }
                }
                StepAction::Research(t) => {
                    sc.tech_ref(t).ok_or(format!("tecnología desconocida en la orden: {t}"))?;
                }
                StepAction::Move { from, to, .. } => {
                    check_pool(from)?;
                    check_pool(to)?;
                }
                StepAction::Train { unit, .. } => {
                    sc.unit_ref(unit).ok_or(format!("unidad desconocida en la orden: {unit}"))?;
                }
                StepAction::Lure { source, from } => {
                    check_pool(from)?;
                    let s = sc.map.sources.iter().find(|s| s.id == source);
                    if !s.is_some_and(|s| s.lure_seconds > 0) {
                        return Err(format!("lure: {source} no existe o no se atrae"));
                    }
                }
            }
            let c = &step.when;
            for p in [&c.pool_exhausted].into_iter().flatten() {
                check_pool(p)?;
            }
            if let Some(b) = &c.built {
                check_building(b)?;
            }
            if let Some(t) = &c.researched {
                sc.tech_ref(t).ok_or(format!("tecnología desconocida en la orden: {t}"))?;
            }
        }

        let auto = &order.auto;
        let builders_name = auto.builders.as_deref().unwrap_or(&order.default_pool);
        check_pool(builders_name)?;
        for [a, b] in &auto.fallback {
            check_pool(a)?;
            check_pool(b)?;
        }
        for t in &auto.research {
            sc.tech_ref(t).ok_or(format!("tecnología desconocida en auto.research: {t}"))?;
        }
        if let Some(p) = &auto.production {
            sc.unit_ref(&p.unit).ok_or(format!("unidad desconocida en auto.production: {}", p.unit))?;
        }
        let builders_pool = pools.iter().position(|p| p == builders_name).unwrap();
        let fallback = auto
            .fallback
            .iter()
            .map(|[a, b]| (pools.iter().position(|p| p == a).unwrap(), pools.iter().position(|p| p == b).unwrap()))
            .collect();

        let villager = sc.data.unit(ids::VILLAGER).ok_or("falta el aldeano en game_data")?;
        let n_tasks = sc.rates.tasks.len();
        let n_pools = pools.len();
        let start = &sc.map.start;
        let train_orders = auto
            .production
            .iter()
            .map(|p| (sc.unit_ref(&p.unit).unwrap(), p.count))
            .collect();
        let mut sim = Sim {
            sc,
            external: false,
            decisions: Default::default(),
            born_before_tick: 0,
            max_villagers: order.max_villagers,
            builders_pool,
            pending_drop_sites: Vec::new(),
            fallback,
            state: GameState {
                time: 0,
                age: Age::Dark,
                res: Resources::from_cost(&start.resources),
                scouts: start.scouts,
                town_centers: start.town_centers,
                villagers: Vec::new(),
                sources,
                farms: Vec::new(),
                sites: Vec::new(),
                built: (0..start.town_centers).map(|_| (ids::TOWN_CENTER, 0)).collect(),
                techs: Vec::new(),
                effects: Effects {
                    rate_mult: vec![1.0; n_tasks],
                    carry_add: vec![0.0; n_tasks],
                    carry_mult: 1.0,
                    speed_mult: 1.0,
                    farm_food_add: 0.0,
                },
                tcs: (0..start.town_centers).map(|_| TownCenter { site: None, job: None }).collect(),
                research: Vec::new(),
                pending_research: Vec::new(),
                soft_research: Vec::new(),
                training: Vec::new(),
                train_orders,
                units: BTreeMap::new(),
                next_villager: 1,
            },
            pools,
            farm_pool,
            steps: vec![StepState::Waiting; order.steps.len()],
            villager_speed: villager.speed,
            events: Vec::new(),
            ticks: Vec::new(),
            summary: Summary::default(),
            idle_runs: vec![None; start.town_centers as usize],
            exhausted: vec![false; n_pools],
            builder_overrides: BTreeMap::new(),
            placements: 0,
            series: Series::default(),
        };

        // Aldeanos iniciales: cada entrada [[start]] con `build` comparte un edificio.
        let assigned: u32 = order.start.iter().map(|s| s.villagers).sum();
        if assigned > start.villagers {
            return Err(format!("la orden asigna {assigned} aldeanos iniciales y el mapa da {}", start.villagers));
        }
        for entry in &order.start {
            let ids: Vec<u32> = (0..entry.villagers).map(|_| sim.spawn_villager(None)).collect();
            let a = &entry.assign;
            let Some(name) = a.build.as_deref() else {
                let p = sim.pool(a.pool.as_deref().unwrap());
                for id in ids {
                    let vi = sim.vidx(id);
                    sim.assign_pool(vi, p);
                }
                continue;
            };
            if let Some(n) = a.builders {
                // Con nº de constructores: se coloca en el primer segundo, como
                // una decisión (este aldeano + ayudantes).
                let b = sc.building_ref(name).unwrap();
                let at = a.at.as_deref().map(|p| sim.pool(p));
                let then = sim.pool(a.then.as_deref().unwrap());
                for &id in &ids {
                    let vi = sim.vidx(id);
                    sim.state.villagers[vi].pool = then;
                    sim.state.villagers[vi].activity =
                        Activity::WaitBuild { building: b, at, helpers: n.saturating_sub(1).min(255) as u8 };
                }
                continue;
            }
            let b = sc.building_ref(name).unwrap();
            let then = sim.pool(a.then.as_deref().unwrap());
            let at = a.at.as_deref().map(|p| sim.pool(p));
            let cost = sim.building_cost(b);
            let site = if sim.state.res.can_afford(&cost) {
                sim.spend(name.to_owned(), &cost);
                Some(sim.new_site(b, None, at))
            } else {
                None
            };
            for &id in &ids {
                let vi = sim.vidx(id);
                sim.state.villagers[vi].pool = then;
                match site {
                    Some(site) => sim.send_to_site(vi, site, sc.map.build_walk(name)),
                    None => sim.state.villagers[vi].activity = Activity::WaitBuild { building: b, at, helpers: 0 },
                }
            }
            if let Some(site) = site {
                let placement = sim.state.sites[site].placement;
                sim.event(Event::BuildStarted { building: name.to_owned(), builders: ids, placement });
            }
        }
        while sim.state.villagers.len() < start.villagers as usize {
            sim.spawn_villager(None);
        }
        Ok(sim)
    }

    pub fn run(mut self, stop: Stop) -> Timeline {
        self.snapshot();
        while self.state.time < stop.max_seconds {
            self.tick();
            if let Some(age) = stop.age {
                if self.state.age >= age {
                    break;
                }
            }
        }
        self.finish()
    }

    /// Como `new`, pero los aldeanos sin asignación esperan `decide`.
    pub fn new_external(sc: &'a Scenario) -> Result<Sim<'a>, String> {
        let mut sim = Sim::new(sc)?;
        sim.external = true;
        // Si la orden no dice qué hacen los aldeanos iniciales, también se
        // deciden desde fuera (en el segundo 0).
        let assigned: u32 = sc.order.start.iter().map(|s| s.villagers).sum();
        for v in sim.state.villagers.iter().skip(assigned as usize) {
            sim.decisions.push_back(v.id);
        }
        Ok(sim)
    }

    /// Avanza hasta que nace un aldeano que necesita decisión, o hasta
    /// `max_seconds`, o hasta que `done` diga que se cumplió el objetivo.
    /// Devuelve el aldeano que espera decisión, si lo hay.
    pub fn advance(&mut self, max_seconds: u32, done: &dyn Fn(&GameState) -> bool) -> Option<u32> {
        if self.ticks.is_empty() {
            self.snapshot();
        }
        while self.decisions.is_empty() && self.state.time < max_seconds && !done(&self.state) {
            self.tick();
        }
        self.decisions.front().copied()
    }

    /// Asigna el aldeano que esperaba decisión a un pool (aplica las reglas
    /// automáticas, como construir antes su punto de entrega).
    pub fn decide(&mut self, pool: &str) {
        let id = self.decisions.pop_front().expect("no hay decisión pendiente");
        let vi = self.vidx(id);
        let p = self.pool(pool);
        self.assign_pool(vi, p);
        self.events.push(TimedEvent { time: self.state.time, event: Event::Decision { villager: id, choice: pool.to_owned() } });
    }

    /// Como `decide`, pero el aldeano primero construye `building` con
    /// `builders` constructores en total y después va a `pool`.
    pub fn decide_build(&mut self, building: &str, builders: u32, pool: &str) {
        let id = self.decisions.pop_front().expect("no hay decisión pendiente");
        let vi = self.vidx(id);
        let b = self.sc.building_ref(building).expect("edificio conocido");
        self.state.villagers[vi].pool = self.pool(pool);
        self.state.villagers[vi].activity =
            Activity::WaitBuild { building: b, at: None, helpers: builders.saturating_sub(1).min(255) as u8 };
        let choice = format!("{building} → {pool}");
        self.events.push(TimedEvent { time: self.state.time, event: Event::Decision { villager: id, choice } });
    }

    /// ¿Se puede construir ya `building` por edad? (no mira recursos)
    pub fn can_build(&self, building: &str) -> bool {
        self.sc.building_ref(building).is_some_and(|b| self.building_available(b))
    }

    /// Edificios de este tipo terminados o en obra (incluye los iniciales).
    pub fn planned(&self, building: &str) -> usize {
        self.sc.building_ref(building).map_or(0, |b| self.state.count_planned(b))
    }

    pub fn pop_capacity(&self) -> u32 {
        self.pop_cap()
    }

    /// Pide una investigación (p. ej. el clic de edad) desde fuera. Con
    /// `soft`, el CU sigue creando aldeanos hasta que se pueda investigar.
    pub fn queue_research(&mut self, tech: &str, soft: bool) {
        let id = self.sc.tech_ref(tech).expect("tecnología conocida");
        if !self.state.pending_research.contains(&id) && !self.state.has_tech(id) {
            self.state.pending_research.push(id);
            if soft {
                self.state.soft_research.push(id);
            }
        }
    }

    /// Fuerza el nº de constructores de ciertos edificios, por su nº de
    /// colocación (el `placement` de `BuildStarted`).
    pub fn set_builder_overrides(&mut self, overrides: BTreeMap<usize, u32>) {
        self.builder_overrides = overrides;
    }

    pub fn set_max_villagers(&mut self, n: Option<u32>) {
        self.max_villagers = n;
    }

    pub fn max_villagers(&self) -> Option<u32> {
        self.max_villagers
    }

    pub fn pending_decision(&self) -> Option<u32> {
        self.decisions.front().copied()
    }

    pub fn pool_names(&self) -> &[String] {
        &self.pools
    }

    pub fn scenario(&self) -> &'a Scenario {
        self.sc
    }

    pub fn summary(&self) -> &Summary {
        &self.summary
    }

    /// ¿El pool tiene fuentes con recurso ahora (o en el futuro) o es de granjas?
    pub fn pool_has_resources(&self, pool: &str) -> bool {
        let Some(p) = self.pools.iter().position(|x| x == pool) else { return false };
        Some(p) == self.farm_pool || self.state.sources.iter().any(|s| s.pool == p && s.remaining > EPS)
    }

    /// Ingreso estimado por segundo de cada recurso con la asignación actual
    /// (tasa efectiva con caminata; cuenta solo aldeanos que recolectan).
    pub fn income(&self) -> Resources {
        let mut inc = Resources::default();
        for v in &self.state.villagers {
            if v.activity.is_building() || v.idle {
                continue;
            }
            let task = if Some(v.pool) == self.farm_pool {
                self.farm_task()
            } else if let Some(s) = self.state.sources.iter().find(|s| s.pool == v.pool && s.remaining > EPS) {
                s.task
            } else {
                continue;
            };
            let dist = self
                .state
                .sources
                .iter()
                .find(|s| s.pool == v.pool && s.remaining > EPS)
                .map_or(0.0, |s| s.distance);
            let r = aoe2_data::rates::effective_rate(
                self.capacity(task).round() as u32,
                self.rate(task),
                dist / self.speed(),
            );
            *inc.get_mut(self.sc.rates.tasks[task].resource) += r;
        }
        inc
    }

    /// Termina y devuelve la línea de tiempo.
    pub fn into_timeline(self) -> Timeline {
        self.finish()
    }

    // ---------- utilidades ----------

    fn pool(&self, name: &str) -> usize {
        self.pools.iter().position(|p| p == name).expect("pool validado en Sim::new")
    }

    fn vidx(&self, id: u32) -> usize {
        self.state.villagers.iter().position(|v| v.id == id).unwrap()
    }

    fn event(&mut self, event: Event) {
        self.events.push(TimedEvent { time: self.state.time, event });
    }

    fn building_cost(&self, b: u32) -> Cost {
        self.sc.data.building(b).map(|b| b.cost).unwrap_or_default()
    }

    fn building_name(&self, b: u32) -> String {
        self.sc.rules.building_name(b).map(str::to_owned).unwrap_or_else(|| b.to_string())
    }

    fn tech_name(&self, t: u32) -> String {
        self.sc
            .rules
            .names
            .techs
            .iter()
            .find(|(_, v)| **v == t)
            .map(|(k, _)| k.clone())
            .unwrap_or_else(|| t.to_string())
    }

    fn spawn_villager(&mut self, born_tc: Option<usize>) -> u32 {
        let id = self.state.next_villager;
        self.state.next_villager += 1;
        let pool = self.pool(&self.sc.order.default_pool);
        let born = if born_tc.is_some() { self.state.time + 1 } else { 0 };
        self.state.villagers.push(Villager {
            id,
            born,
            born_tc,
            gathered: [0.0; 4],
            task: None,
            pool,
            activity: Activity::Idle,
            carry: 0.0,
            carry_resource: None,
            farm: None,
            idle: false,
        });
        id
    }

    fn new_site(&mut self, building: u32, farm_for: Option<u32>, at: Option<usize>) -> usize {
        let placement = self.placements;
        self.placements += 1;
        self.state.sites.push(Site { building, progress: 0.0, done: false, farm_for, at, placement, finished: None });
        self.state.sites.len() - 1
    }

    /// Paga algo y lo anota en el libro de recursos.
    fn spend(&mut self, what: String, cost: &Cost) {
        self.state.res.pay(cost);
        let sp = &mut self.summary.spent;
        sp.food += cost.food as f64;
        sp.wood += cost.wood as f64;
        sp.gold += cost.gold as f64;
        sp.stone += cost.stone as f64;
        let r = self.state.res;
        self.event(Event::Spent { what, cost: *cost, stock: [r.food, r.wood, r.gold, r.stone] });
    }

    fn speed(&self) -> f64 {
        self.villager_speed * self.state.effects.speed_mult
    }

    fn send_to_site(&mut self, vi: usize, site: usize, walk_tiles: f64) {
        let left = walk_tiles / self.speed();
        let v = &mut self.state.villagers[vi];
        v.activity = if left > EPS { Activity::ToSite { site, left } } else { Activity::Building { site } };
    }

    fn capacity(&self, task: usize) -> f64 {
        let t = &self.sc.rates.tasks[task];
        let e = &self.state.effects;
        (self.sc.rates.carry_for(t) as f64 + e.carry_add[task]) * e.carry_mult
    }

    fn rate(&self, task: usize) -> f64 {
        self.sc.rates.tasks[task].rate * self.state.effects.rate_mult[task]
    }

    fn farm_task(&self) -> usize {
        self.sc.rates.tasks.iter().position(|t| t.id == "farmer").expect("tarea farmer")
    }

    fn task_of(&self, target: Target) -> usize {
        match target {
            Target::Source(s) => self.state.sources[s].task,
            Target::Farm(_) => self.farm_task(),
        }
    }

    fn walk_seconds(&self, target: Target) -> f64 {
        match target {
            Target::Source(s) => self.state.sources[s].distance / self.speed(),
            Target::Farm(_) => 0.0,
        }
    }

    fn pop_cap(&self) -> u32 {
        let p = &self.sc.rules.population;
        let houses = self.state.count_built(ids::HOUSE) as u32;
        let tcs = self.state.count_built(ids::TOWN_CENTER) as u32;
        (tcs * p.per_town_center + houses * p.per_house).min(p.max)
    }

    fn pool_exhausted(&self, pool: usize) -> bool {
        let mut any = false;
        for s in self.state.sources.iter().filter(|s| s.pool == pool) {
            any = true;
            if s.remaining > EPS || s.arrives_at > self.state.time {
                return false;
            }
        }
        any
    }

    // ---------- el paso ----------

    pub fn tick(&mut self) {
        self.record_second();
        self.born_before_tick = self.state.next_villager;
        self.run_steps();
        self.run_auto();
        self.run_tc();
        self.run_research();
        self.run_training();
        // Los aldeanos que nacen en este segundo empiezan a trabajar en el siguiente.
        let born = self.state.next_villager;
        let n_before = self.born_before_tick;
        for vi in 0..self.state.villagers.len() {
            let id = self.state.villagers[vi].id;
            if (id >= n_before && id < born) || self.decisions.contains(&id) {
                continue;
            }
            self.act(vi);
        }
        self.run_construction();
        self.check_exhausted();
        self.track_tasks();
        self.state.time += 1;
        if self.state.time % TICK_SECONDS == 0 {
            self.snapshot();
        }
    }

    fn condition_met(&self, c: &Condition) -> bool {
        let s = &self.state;
        c.time.is_none_or(|t| s.time >= t)
            && c.pop.is_none_or(|p| s.pop() >= p)
            && c.villagers.is_none_or(|n| s.villagers.len() as u32 >= n)
            && c.age.is_none_or(|a| s.age >= a)
            && c.researched.as_deref().is_none_or(|t| s.has_tech(self.sc.tech_ref(t).unwrap()))
            && c.built.as_deref().is_none_or(|b| s.count_built(self.sc.building_ref(b).unwrap()) > 0)
            && c.pool_exhausted.as_deref().is_none_or(|p| self.pool_exhausted(self.pool(p)))
    }

    fn run_steps(&mut self) {
        for i in 0..self.steps.len() {
            if self.steps[i] == StepState::Waiting && self.condition_met(&self.sc.order.steps[i].when) {
                self.steps[i] = StepState::Triggered;
            }
            if self.steps[i] != StepState::Triggered {
                continue;
            }
            let step = &self.sc.order.steps[i];
            match step.action().expect("validado") {
                StepAction::Research(t) => {
                    let tech = self.sc.tech_ref(t).unwrap();
                    self.state.pending_research.push(tech);
                    if step.soft {
                        self.state.soft_research.push(tech);
                    }
                    self.steps[i] = StepState::Done;
                }
                StepAction::Move { from, to, count } => {
                    let (from, to) = (self.pool(from), self.pool(to));
                    let mut moved = Vec::new();
                    for v in self.state.villagers.iter_mut().rev() {
                        if count.is_some_and(|c| moved.len() as u32 >= c) {
                            break;
                        }
                        if v.pool == from && !v.activity.is_building() {
                            v.pool = to;
                            if !matches!(v.activity, Activity::ToDrop { .. }) {
                                v.activity = Activity::Idle;
                            }
                            moved.push(v.id);
                        }
                    }
                    moved.reverse();
                    let (from, to) = (self.pools[from].clone(), self.pools[to].clone());
                    self.event(Event::Moved { villagers: moved, from, to });
                    self.steps[i] = StepState::Done;
                }
                StepAction::Lure { source, from } => {
                    let si = self.state.sources.iter().position(|s| s.id == source).unwrap();
                    let s = &self.state.sources[si];
                    if s.lured || s.luring {
                        self.steps[i] = StepState::Done;
                        continue;
                    }
                    if s.arrives_at > self.state.time {
                        continue;
                    }
                    let from = self.pool(from);
                    let Some(vi) = (0..self.state.villagers.len()).rev().find(|&vi| {
                        let v = &self.state.villagers[vi];
                        v.pool == from && !v.activity.is_building() && !matches!(v.activity, Activity::Luring { .. })
                    }) else {
                        continue;
                    };
                    let pool = self.state.sources[si].pool;
                    let left = self.state.sources[si].lure_seconds as f64;
                    self.state.sources[si].luring = true;
                    let v = &mut self.state.villagers[vi];
                    v.pool = pool;
                    v.activity = Activity::Luring { source: si, left };
                    let villager = v.id;
                    self.event(Event::Lured { source: source.to_owned(), villager });
                    self.steps[i] = StepState::Done;
                }
                StepAction::Build { building, builders, from, then, at } => {
                    let b = self.sc.building_ref(building).unwrap();
                    let (from, then) = (self.pool(from), self.pool(then));
                    let at = at.map(|p| self.pool(p));
                    if self.place_building(b, builders, from, Some(then), at, false) {
                        self.steps[i] = StepState::Done;
                    }
                }
                StepAction::Train { unit, count } => {
                    let u = self.sc.unit_ref(unit).unwrap();
                    self.state.train_orders.push((u, count));
                    self.steps[i] = StepState::Done;
                }
            }
        }
    }

    /// Coloca un edificio con hasta `builders` aldeanos de `from` (los más
    /// nuevos). Paga al colocarlo. Al terminar van a `then` (o vuelven a `from`).
    /// Devuelve false si todavía no se puede (edad, recursos o constructores).
    fn place_building(
        &mut self,
        b: u32,
        builders: u32,
        from: usize,
        then: Option<usize>,
        at: Option<usize>,
        any_pool: bool,
    ) -> bool {
        if !self.building_available(b) || !self.state.res.can_afford(&self.building_cost(b)) {
            return false;
        }
        let free = |v: &Villager| {
            !v.activity.is_building() && !matches!(v.activity, Activity::Luring { .. }) && !self.decisions.contains(&v.id)
        };
        let forced = self.builder_overrides.get(&self.placements).copied();
        let wanted = forced.unwrap_or(builders).max(1) as usize;
        let mut chosen: Vec<usize> = (0..self.state.villagers.len())
            .rev()
            .filter(|&vi| self.state.villagers[vi].pool == from && free(&self.state.villagers[vi]))
            .take(wanted)
            .collect();
        if (chosen.is_empty() && any_pool) || (forced.is_some() && chosen.len() < wanted) {
            // Faltan constructores: sirve cualquier aldeano libre (de recolectar).
            for vi in (0..self.state.villagers.len()).rev() {
                if chosen.len() >= wanted {
                    break;
                }
                if !chosen.contains(&vi) && free(&self.state.villagers[vi]) {
                    chosen.push(vi);
                }
            }
        }
        if chosen.is_empty() {
            return false;
        }
        let cost = self.building_cost(b);
        let name = self.building_name(b);
        self.spend(name.clone(), &cost);
        let site = self.new_site(b, None, at);
        let walk = self.sc.map.build_walk(&name);
        let mut ids = Vec::new();
        chosen.sort();
        for &vi in &chosen {
            if let Some(then) = then.filter(|_| self.state.villagers[vi].pool == from) {
                self.state.villagers[vi].pool = then;
            }
            self.send_to_site(vi, site, walk);
            ids.push(self.state.villagers[vi].id);
        }
        let placement = self.state.sites[site].placement;
        self.event(Event::BuildStarted { building: name, builders: ids, placement });
        true
    }

    /// Manda `n` ayudantes del pool de constructores (o de cualquier pool si no
    /// hay) a una obra; vuelven a su tarea al terminar. Devuelve sus ids.
    fn add_helpers(&mut self, site: usize, n: usize) -> Vec<u32> {
        let name = self.building_name(self.state.sites[site].building);
        let walk = self.sc.map.build_walk(&name);
        let from = self.builders_pool;
        let free = |v: &Villager| {
            !v.activity.is_building() && !matches!(v.activity, Activity::Luring { .. }) && !self.decisions.contains(&v.id)
        };
        let mut chosen: Vec<usize> = (0..self.state.villagers.len())
            .rev()
            .filter(|&vi| self.state.villagers[vi].pool == from && free(&self.state.villagers[vi]))
            .take(n)
            .collect();
        for vi in (0..self.state.villagers.len()).rev() {
            if chosen.len() >= n {
                break;
            }
            if !chosen.contains(&vi) && free(&self.state.villagers[vi]) {
                chosen.push(vi);
            }
        }
        let mut ids = Vec::new();
        for vi in chosen {
            self.send_to_site(vi, site, walk);
            ids.push(self.state.villagers[vi].id);
        }
        ids
    }

    /// Con hipótesis de constructores, suma ayudantes a una obra recién colocada.
    fn add_forced_helpers(&mut self, site: usize, already: usize) {
        let placement = self.state.sites[site].placement;
        let Some(&wanted) = self.builder_overrides.get(&placement) else { return };
        let name = self.building_name(self.state.sites[site].building);
        let walk = self.sc.map.build_walk(&name);
        let mut added = already;
        for vi in (0..self.state.villagers.len()).rev() {
            if added >= wanted as usize {
                break;
            }
            let v = &self.state.villagers[vi];
            if !v.activity.is_building() && !matches!(v.activity, Activity::Luring { .. }) && !self.decisions.contains(&v.id) {
                self.send_to_site(vi, site, walk);
                added += 1;
            }
        }
    }

    /// Asigna un aldeano a un pool. Con `auto.drop_sites`, si la fuente del
    /// pool aún no tiene punto de entrega, primero lo construye.
    fn assign_pool(&mut self, vi: usize, pool: usize) {
        self.state.villagers[vi].pool = pool;
        self.state.villagers[vi].activity = Activity::Idle;
        if !self.sc.order.auto.drop_sites {
            return;
        }
        let Some(si) = self.state.sources.iter().position(|s| s.pool == pool && s.remaining > EPS) else {
            return;
        };
        let s = &self.state.sources[si];
        if let (Some(b), false, false) = (s.drop_site, s.has_drop_site, s.drop_site_planned) {
            // El aldeano empieza a trabajar ya; el punto de entrega se coloca
            // en cuanto alcancen los recursos (ver run_auto).
            self.state.sources[si].drop_site_planned = true;
            self.pending_drop_sites.push((b, pool));
        }
    }

    /// Reglas automáticas de la orden (`[auto]`).
    fn run_auto(&mut self) {
        let auto = &self.sc.order.auto;
        let from = self.builders_pool;
        // Casas: empezar una cuando falten `margin` plazas o menos.
        if let Some(margin) = auto.houses {
            let p = &self.sc.rules.population;
            let in_progress = self.state.sites.iter().filter(|s| s.building == ids::HOUSE && !s.done).count() as u32;
            let room = self.pop_cap() + in_progress * p.per_house;
            // + el aldeano que cada CU tiene (o está por tener) en cola.
            let pop = self.state.pop() + self.state.tcs.len() as u32 + self.state.training.len() as u32;
            let free = room.saturating_sub(pop);
            if room < p.max && free <= margin {
                // Con prisa (sin plazas libres), dos constructores.
                let builders = if free == 0 { 2 } else { 1 };
                self.place_building(ids::HOUSE, builders, from, None, None, true);
            }
        }
        // Puntos de entrega pendientes: colocarlos cuando alcance, sin gastar
        // la madera de una casa que hace falta pronto (el CU no debe quedarse
        // sin población por un campamento).
        let house_soon = auto.houses.is_some() && {
            let p = &self.sc.rules.population;
            let in_progress = self.state.sites.iter().filter(|s| s.building == ids::HOUSE && !s.done).count() as u32;
            let room = self.pop_cap() + in_progress * p.per_house;
            let pop = self.state.pop() + self.state.tcs.len() as u32 + self.state.training.len() as u32;
            room < p.max && room.saturating_sub(pop) <= auto.houses.unwrap_or(0)
        };
        let house_wood = if house_soon { self.building_cost(ids::HOUSE).wood as f64 } else { 0.0 };
        let mut i = 0;
        while i < self.pending_drop_sites.len() {
            let (b, pool) = self.pending_drop_sites[i];
            let after = self.state.res.wood - self.building_cost(b).wood as f64;
            if after + EPS >= house_wood && self.place_building(b, 1, pool, None, Some(pool), false) {
                self.pending_drop_sites.remove(i);
            } else {
                i += 1;
            }
        }
        // Atraer el siguiente jabalí antes de que se acabe el actual.
        if let Some(threshold) = auto.lure_ahead {
            self.lure_ahead(threshold as f64);
        }
        // Edificios que pide la siguiente edad.
        if let Some(target) = auto.age_buildings {
            let next = match self.state.age {
                Age::Dark => Some(Age::Feudal),
                Age::Feudal => Some(Age::Castle),
                Age::Castle => Some(Age::Imperial),
                Age::Imperial => None,
            };
            if let Some(next) = next.filter(|n| *n <= target) {
                let tech = self.sc.rules.age_tech(next);
                let wanted = self.state.age > Age::Dark || tech.is_some_and(|t| self.state.pending_research.contains(&t));
                if let (true, Some(req)) = (wanted, tech.and_then(|t| self.sc.rules.age_requirement(t))) {
                    let have = req.buildings.iter().filter(|b| self.state.count_planned(**b) > 0).count() as u32;
                    if have < req.count {
                        let mut options: Vec<u32> = req
                            .buildings
                            .iter()
                            .copied()
                            .filter(|b| self.state.count_planned(*b) == 0 && self.building_available(*b))
                            .collect();
                        let total = |b: &u32| {
                            let c = self.building_cost(*b);
                            c.food + c.wood + c.gold + c.stone
                        };
                        options.sort_by_key(total); // estable: en empate, el orden de rules.toml
                        if let Some(&b) = options.first() {
                            self.place_building(b, 2, from, None, None, true);
                        }
                    }
                }
            }
        }
        // Tecnologías automáticas (con sus previas y la mejora de la unidad que
        // se produce): pedirlas en cuanto la edad lo permita.
        let mut wanted: Vec<u32> = Vec::new();
        for name in &auto.research {
            wanted.extend(self.sc.tech_chain(self.sc.tech_ref(name).unwrap()));
        }
        if let Some(prod) = &auto.production {
            let unit = self.sc.unit_ref(&prod.unit).unwrap();
            if let Some(&up) = self.sc.data.unit_upgrades.get(&unit) {
                wanted.extend(self.sc.tech_chain(up));
            }
        }
        for tech in wanted {
            if self.state.has_tech(tech)
                || self.state.pending_research.contains(&tech)
                || self.state.research.iter().any(|r| r.job == Job::Research(tech))
                || self.state.tc_researching(tech)
            {
                continue;
            }
            let age_ok = self.sc.data.tech(tech).and_then(|t| t.tree.as_ref()).is_none_or(|t| t.age <= self.state.age);
            let prev_ok = self
                .sc
                .rules
                .tech_prerequisite(tech)
                .is_none_or(|p| self.state.has_tech(p) || self.state.pending_research.contains(&p) || self.state.research.iter().any(|r| r.job == Job::Research(p)));
            if age_ok && prev_ok {
                let b = self.research_building(tech);
                if b == ids::TOWN_CENTER || self.ensure_building(b) {
                    self.state.pending_research.push(tech);
                }
            }
        }
        // Producción: mantener los edificios productores.
        if let Some(prod) = auto.production.clone() {
            let unit = self.sc.unit_ref(&prod.unit).unwrap();
            if let Some(b) = self.sc.data.unit(unit).and_then(|u| u.tree.as_ref()).and_then(|t| t.produced_at) {
                if self.building_available(b) && self.ensure_prerequisites(b) {
                    if (self.state.count_planned(b) as u32) < prod.buildings {
                        self.place_building(b, 2, from, None, None, true);
                    }
                }
            }
        }
    }

    fn lure_ahead(&mut self, threshold: f64) {
        let now = self.state.time;
        for si in 0..self.state.sources.len() {
            let s = &self.state.sources[si];
            if s.lure_seconds == 0 || !s.lured || s.remaining > threshold {
                continue;
            }
            let pool = s.pool;
            let next = (si + 1..self.state.sources.len()).find(|&n| {
                let x = &self.state.sources[n];
                x.pool == pool && x.lure_seconds > 0 && !x.lured && !x.luring && x.arrives_at <= now && x.remaining > EPS
            });
            let Some(next) = next else { continue };
            let Some(vi) = (0..self.state.villagers.len()).rev().find(|&vi| {
                let v = &self.state.villagers[vi];
                v.pool == pool && !v.activity.is_building() && !matches!(v.activity, Activity::Luring { .. })
            }) else {
                continue;
            };
            let left = self.state.sources[next].lure_seconds as f64;
            self.state.sources[next].luring = true;
            self.state.villagers[vi].activity = Activity::Luring { source: next, left };
            let (source, villager) = (self.state.sources[next].id.clone(), self.state.villagers[vi].id);
            self.event(Event::Lured { source, villager });
        }
    }

    /// Edificios requeridos por `b` (p. ej. Cuartel para la Galería de tiro):
    /// true si ya están terminados; si no, los encarga.
    fn ensure_prerequisites(&mut self, b: u32) -> bool {
        let req = self.sc.data.building(b).and_then(|x| x.tree.as_ref()).and_then(|t| t.requires_building);
        match req {
            Some(r) if self.state.count_built(r) == 0 => {
                self.ensure_building(r);
                false
            }
            _ => true,
        }
    }

    /// Que exista un edificio de tipo `b`: true si ya está terminado; si no
    /// está ni en obra, lo encarga (y sus requisitos).
    fn ensure_building(&mut self, b: u32) -> bool {
        if self.state.count_built(b) > 0 {
            return true;
        }
        if self.state.count_planned(b) == 0 && self.building_available(b) && self.ensure_prerequisites(b) {
            let from = self.builders_pool;
            self.place_building(b, 2, from, None, None, true);
        }
        false
    }

    /// Entrenamiento: cada edificio productor terminado hace una unidad a la vez.
    fn run_training(&mut self) {
        for i in 0..self.state.train_orders.len() {
            let (unit, left) = self.state.train_orders[i];
            if left == 0 {
                continue;
            }
            let Some(u) = self.sc.data.unit(unit) else { continue };
            let Some(b) = u.tree.as_ref().and_then(|t| t.produced_at) else { continue };
            if u.tree.as_ref().is_some_and(|t| t.age > self.state.age) {
                continue;
            }
            if self.sc.data.unit_upgrades.get(&unit).is_some_and(|t| !self.state.has_tech(*t)) {
                continue; // falta investigar la mejora (p. ej. Ballestero)
            }
            let busy = self.state.training.iter().filter(|r| r.building == b).count();
            let mut free = self.state.count_built(b).saturating_sub(busy);
            let mut left = left;
            let in_training = self.state.training.len() as u32;
            let mut started = 0;
            let in_production = self.state.villagers_in_production();
            while free > 0
                && left > 0
                && self.state.res.can_afford(&u.cost)
                && self.state.pop() + in_training + in_production + started < self.pop_cap()
            {
                let what = self.sc.rules.names.units.iter().find(|(_, v)| **v == unit).map_or(unit.to_string(), |(k, _)| k.clone());
                self.spend(what, &u.cost);
                self.state.training.push(Running { building: b, job: Job::Unit(unit), left: u.train_time });
                free -= 1;
                left -= 1;
                started += 1;
            }
            self.state.train_orders[i].1 = left;
        }
        let mut finished = Vec::new();
        for r in &mut self.state.training {
            r.left -= 1;
            if r.left == 0 {
                finished.push(*r);
            }
        }
        self.state.training.retain(|r| r.left > 0);
        for r in finished {
            self.finish_job(r, None);
        }
    }

    fn building_available(&self, b: u32) -> bool {
        self.sc.building_age(b) <= self.state.age
    }

    /// ¿Se puede empezar a investigar `tech` ya? (sin mirar recursos)
    fn research_ready(&self, tech: u32) -> bool {
        let Some(t) = self.sc.data.tech(tech) else { return false };
        if self.state.has_tech(tech) {
            return false;
        }
        if t.tree.as_ref().is_some_and(|tr| tr.age > self.state.age) {
            return false;
        }
        if self.sc.rules.tech_prerequisite(tech).is_some_and(|p| !self.state.has_tech(p)) {
            return false;
        }
        if let Some(req) = self.sc.rules.age_requirement(tech) {
            let have = req.buildings.iter().filter(|b| self.state.count_built(**b) > 0).count() as u32;
            if have < req.count {
                return false;
            }
        }
        true
    }

    fn research_building(&self, tech: u32) -> u32 {
        self.sc
            .data
            .tech(tech)
            .and_then(|t| t.tree.as_ref())
            .and_then(|t| t.produced_at)
            .unwrap_or(ids::TOWN_CENTER)
    }

    fn run_tc(&mut self) {
        for i in 0..self.state.tcs.len() {
            if i >= self.idle_runs.len() {
                self.idle_runs.push(None);
            }
            if self.state.tcs[i].job.is_none() {
                self.start_tc_job(i);
            }
            if let Some(mut r) = self.state.tcs[i].job {
                r.left -= 1;
                if r.left == 0 {
                    self.state.tcs[i].job = None;
                    self.finish_job(r, Some(i));
                } else {
                    self.state.tcs[i].job = Some(r);
                }
            }
        }
    }

    fn start_tc_job(&mut self, i: usize) {
        // Las investigaciones del CU (edades, Telar…) las hace un solo CU a la vez.
        let busy_researching = self.state.tcs.iter().any(|t| t.job.is_some_and(|r| matches!(r.job, Job::Research(_))));
        let tc_research = if busy_researching {
            None
        } else {
            self.state.pending_research.iter().position(|&t| self.research_building(t) == ids::TOWN_CENTER)
        };
        let mut cause = None;
        let mut make_villager = tc_research.is_none();
        if let Some(pos) = tc_research {
            let tech = self.state.pending_research[pos];
            let cost = self.sc.data.tech(tech).unwrap().cost;
            if self.research_ready(tech) && self.state.res.can_afford(&cost) {
                let name = self.tech_name(tech);
                self.spend(name.clone(), &cost);
                self.state.pending_research.remove(pos);
                self.state.soft_research.retain(|t| *t != tech);
                let time = self.sc.data.tech(tech).unwrap().research_time;
                self.state.tcs[i].job = Some(Running { building: ids::TOWN_CENTER, job: Job::Research(tech), left: time });
                if let Some(age) = self.sc.rules.age_of_tech(tech) {
                    self.summary.age_clicks.insert(age, self.state.time);
                }
                self.event(Event::ResearchStarted { tech: name });
            } else if self.state.soft_research.contains(&tech) || i > 0 {
                // Clic "suave" (o un CU que no es el que sube de edad): mientras
                // no se pueda investigar, el CU sigue con aldeanos.
                make_villager = true;
            } else {
                cause = Some(IdleCause::WaitingResearch);
            }
        }
        if make_villager {
            let v = self.sc.data.unit(ids::VILLAGER).unwrap();
            let in_production = self.state.villagers_in_production();
            let enough = self.max_villagers.is_some_and(|m| self.state.villagers.len() as u32 + in_production >= m);
            cause = if enough {
                Some(IdleCause::NoOrder)
            } else if self.state.pop() + in_production + self.state.training.len() as u32 >= self.pop_cap() {
                Some(IdleCause::Housed)
            } else if !self.state.res.can_afford(&v.cost) {
                Some(IdleCause::NoFood)
            } else {
                self.spend("villager".to_owned(), &v.cost);
                self.state.tcs[i].job = Some(Running { building: ids::TOWN_CENTER, job: Job::Villager, left: v.train_time });
                None
            };
        }
        self.track_idle(i, cause);
    }

    fn track_idle(&mut self, tc: usize, cause: Option<IdleCause>) {
        if let Some(c) = cause {
            *self.summary.tc_idle_seconds.entry(c).or_default() += 1;
        }
        let current = self.idle_runs[tc].map(|(c, _)| c);
        if current != cause {
            self.flush_idle(tc);
            self.idle_runs[tc] = cause.map(|c| (c, self.state.time));
        }
    }

    fn flush_idle(&mut self, tc: usize) {
        if let Some((cause, since)) = self.idle_runs[tc].take() {
            let seconds = self.state.time - since;
            self.events.push(TimedEvent { time: since, event: Event::TcIdle { cause, seconds, tc } });
        }
    }

    fn run_research(&mut self) {
        // Empezar investigaciones pendientes fuera del CU.
        let mut i = 0;
        while i < self.state.pending_research.len() {
            let tech = self.state.pending_research[i];
            let b = self.research_building(tech);
            let busy = self.state.research.iter().any(|r| r.building == b);
            let cost = self.sc.data.tech(tech).unwrap().cost;
            if b != ids::TOWN_CENTER
                && !busy
                && self.state.count_built(b) > 0
                && self.research_ready(tech)
                && self.state.res.can_afford(&cost)
            {
                let name = self.tech_name(tech);
                self.spend(name.clone(), &cost);
                self.state.pending_research.remove(i);
                let time = self.sc.data.tech(tech).unwrap().research_time;
                self.state.research.push(Running { building: b, job: Job::Research(tech), left: time });
                self.event(Event::ResearchStarted { tech: name });
            } else {
                i += 1;
            }
        }
        let mut finished = Vec::new();
        for r in &mut self.state.research {
            r.left -= 1;
            if r.left == 0 {
                finished.push(*r);
            }
        }
        self.state.research.retain(|r| r.left > 0);
        for r in finished {
            self.finish_job(r, None);
        }
    }

    /// Efectos de un trabajo terminado; ocurre al final del segundo actual.
    fn finish_job(&mut self, r: Running, tc: Option<usize>) {
        let now = self.state.time + 1;
        match r.job {
            Job::Villager => {
                let id = self.spawn_villager(Some(tc.unwrap_or(0)));
                let vi = self.vidx(id);
                let assign = self.sc.order.assignment_for(id).cloned();
                let label = match &assign {
                    Some(a) if a.build.is_some() => {
                        let b = self.sc.building_ref(a.build.as_deref().unwrap()).unwrap();
                        let at = a.at.as_deref().map(|p| self.pool(p));
                        self.state.villagers[vi].pool = self.pool(a.then.as_deref().unwrap());
                        let helpers = a.builders.unwrap_or(1).saturating_sub(1).min(255) as u8;
                        self.state.villagers[vi].activity = Activity::WaitBuild { building: b, at, helpers };
                        format!("{} → {}", a.build.as_deref().unwrap(), a.then.as_deref().unwrap())
                    }
                    Some(a) => {
                        let p = self.pool(a.pool.as_deref().unwrap());
                        self.assign_pool(vi, p);
                        a.pool.clone().unwrap()
                    }
                    None if self.external => {
                        self.decisions.push_back(id);
                        "?".to_owned()
                    }
                    None => {
                        let p = self.pool(&self.sc.order.default_pool);
                        self.assign_pool(vi, p);
                        self.sc.order.default_pool.clone()
                    }
                };
                self.events.push(TimedEvent { time: now, event: Event::VillagerCreated { villager: id, assignment: label } });
            }
            Job::Unit(unit) => {
                *self.state.units.entry(unit).or_default() += 1;
                let count = self.state.units_of(unit);
                let name = self.sc.rules.names.units.iter().find(|(_, v)| **v == unit).map_or(unit.to_string(), |(k, _)| k.clone());
                self.events.push(TimedEvent { time: now, event: Event::UnitTrained { unit: name, count } });
            }
            Job::Research(tech) => {
                self.state.techs.push(tech);
                self.apply_modifiers(tech);
                let name = self.tech_name(tech);
                self.events.push(TimedEvent { time: now, event: Event::ResearchFinished { tech: name } });
                if let Some(age) = self.sc.rules.age_of_tech(tech) {
                    self.state.age = age;
                    self.summary.age_reached.insert(age, now);
                    self.events.push(TimedEvent { time: now, event: Event::AgeReached { age } });
                }
            }
        }
    }

    /// Da por construido el punto de entrega de la primera fuente del pool
    /// (calculadora: "con campamento").
    pub fn grant_drop_site(&mut self, pool: &str) {
        let Some(p) = self.pools.iter().position(|x| x == pool) else { return };
        if let Some(s) = self.state.sources.iter_mut().find(|s| s.pool == p && s.drop_site.is_some()) {
            s.has_drop_site = true;
            s.drop_site_planned = true;
            if let Some(d) = s.distance_with_drop_site {
                s.distance = d;
            }
        }
    }

    /// Da una tecnología ya investigada (escenarios de prueba, bonos de inicio).
    pub fn grant_tech(&mut self, tech: u32) {
        if !self.state.has_tech(tech) {
            self.state.techs.push(tech);
            self.apply_modifiers(tech);
        }
    }

    fn apply_modifiers(&mut self, tech: u32) {
        for m in self.sc.modifiers.for_tech(tech) {
            let e = &mut self.state.effects;
            match m.kind {
                ModifierKind::RateMult | ModifierKind::CarryAdd => {
                    for (i, t) in self.sc.rates.tasks.iter().enumerate() {
                        if m.applies_to(&t.id) {
                            if m.kind == ModifierKind::RateMult {
                                e.rate_mult[i] *= m.value;
                            } else {
                                e.carry_add[i] += m.value;
                            }
                        }
                    }
                }
                ModifierKind::CarryMult => e.carry_mult *= m.value,
                ModifierKind::SpeedMult => e.speed_mult *= m.value,
                ModifierKind::FarmFoodAdd => e.farm_food_add += m.value,
            }
        }
    }

    // ---------- aldeanos ----------

    /// Busca trabajo en el pool del aldeano. Devuelve la nueva actividad o
    /// `None` si no hay nada que hacer este segundo.
    fn find_work(&mut self, vi: usize) -> Option<Activity> {
        for _ in 0..self.pools.len() {
            let pool = self.state.villagers[vi].pool;
            if let Some(a) = self.find_work_in(vi) {
                return Some(a);
            }
            // Pool agotado: pasar al siguiente según `auto.fallback`.
            match self.fallback.iter().find(|(a, _)| *a == pool) {
                Some(&(_, next)) if self.pool_exhausted(pool) => self.state.villagers[vi].pool = next,
                _ => return None,
            }
        }
        None
    }

    fn find_work_in(&mut self, vi: usize) -> Option<Activity> {
        let pool = self.state.villagers[vi].pool;
        if Some(pool) == self.farm_pool {
            return self.find_farm(vi);
        }
        let now = self.state.time;
        for si in 0..self.state.sources.len() {
            let s = &self.state.sources[si];
            if s.pool != pool || s.arrives_at > now || s.remaining <= EPS {
                continue;
            }
            if s.lure_seconds > 0 && !s.lured {
                if s.luring {
                    return None; // otro aldeano lo está atrayendo: esperar
                }
                self.state.sources[si].luring = true;
                let id = self.state.villagers[vi].id;
                let source = self.state.sources[si].id.clone();
                self.event(Event::Lured { source, villager: id });
                return Some(Activity::Luring { source: si, left: self.state.sources[si].lure_seconds as f64 });
            }
            return Some(Activity::Gathering(Target::Source(si)));
        }
        None
    }

    fn find_farm(&mut self, vi: usize) -> Option<Activity> {
        let v = &self.state.villagers[vi];
        if let Some(f) = v.farm {
            if self.state.farms[f].remaining > EPS {
                return Some(Activity::Gathering(Target::Farm(f)));
            }
        }
        let had_farm = v.farm.is_some();
        let farm = self.sc.building_ref("farm").unwrap_or(ids::FARM);
        let cost = self.building_cost(farm);
        if !self.state.res.can_afford(&cost) || (had_farm && !self.sc.rules.farm.auto_reseed) {
            return None;
        }
        self.spend(if had_farm { "reseed".into() } else { "farm".into() }, &cost);
        let id = self.state.villagers[vi].id;
        let site = self.new_site(farm, Some(id), None);
        // Resembrar es en el mismo sitio: sin caminata.
        let walk = if had_farm { 0.0 } else { self.sc.map.build_walk("farm") / self.speed() };
        let placement = self.state.sites[site].placement;
        self.event(Event::BuildStarted { building: "farm".into(), builders: vec![id], placement });
        Some(if walk > EPS { Activity::ToSite { site, left: walk } } else { Activity::Building { site } })
    }

    /// Un segundo de vida de un aldeano, con presupuesto de tiempo continuo.
    fn act(&mut self, vi: usize) {
        let mut budget = 1.0_f64;
        let mut idle = false;
        let mut guard = 0;
        while budget > EPS {
            guard += 1;
            debug_assert!(guard < 100, "bucle sin progreso");
            if guard >= 100 {
                break;
            }
            let activity = self.state.villagers[vi].activity;
            let next = match activity {
                Activity::Idle => match self.find_work(vi) {
                    Some(a) => a,
                    None => {
                        idle = true;
                        break;
                    }
                },
                Activity::WaitBuild { building, at, helpers } => {
                    if self.building_available(building) && self.state.res.can_afford(&self.building_cost(building)) {
                        let cost = self.building_cost(building);
                        let name = self.building_name(building);
                        self.spend(name.clone(), &cost);
                        let site = self.new_site(building, None, at);
                        let id = self.state.villagers[vi].id;
                        let placement = self.state.sites[site].placement;
                        let extra = self.add_helpers(site, helpers as usize);
                        let mut builders = vec![id];
                        builders.extend(extra);
                        self.event(Event::BuildStarted { building: name.clone(), builders, placement });
                        self.add_forced_helpers(site, 1 + helpers as usize);
                        let left = self.sc.map.build_walk(&name) / self.speed();
                        Activity::ToSite { site, left }
                    } else {
                        idle = true;
                        break;
                    }
                }
                Activity::ToSite { site, left } => {
                    let dt = left.min(budget);
                    budget -= dt;
                    if left - dt > EPS { Activity::ToSite { site, left: left - dt } } else { Activity::Building { site } }
                }
                Activity::Building { site } => {
                    if self.state.sites[site].done {
                        Activity::Idle
                    } else {
                        // Construye el resto del segundo; el avance se calcula en run_construction.
                        break;
                    }
                }
                Activity::Luring { source, left } => {
                    let dt = left.min(budget);
                    budget -= dt;
                    if left - dt > EPS {
                        Activity::Luring { source, left: left - dt }
                    } else {
                        let s = &mut self.state.sources[source];
                        s.lured = true;
                        s.luring = false;
                        Activity::Gathering(Target::Source(source))
                    }
                }
                Activity::Gathering(target) => self.gather(vi, target, &mut budget),
                Activity::ToDrop { target, left } => {
                    let dt = left.min(budget);
                    budget -= dt;
                    if left - dt > EPS {
                        Activity::ToDrop { target, left: left - dt }
                    } else {
                        self.deposit(vi);
                        self.after_deposit(vi, target)
                    }
                }
                Activity::Back { target, left } => {
                    let dt = left.min(budget);
                    budget -= dt;
                    if left - dt > EPS { Activity::Back { target, left: left - dt } } else { Activity::Gathering(target) }
                }
            };
            self.state.villagers[vi].activity = next;
        }
        self.state.villagers[vi].idle = idle;
        if idle {
            self.summary.villager_idle_seconds += 1;
        }
    }

    fn remaining(&self, target: Target) -> f64 {
        match target {
            Target::Source(s) => self.state.sources[s].remaining,
            Target::Farm(f) => self.state.farms[f].remaining,
        }
    }

    fn gather(&mut self, vi: usize, target: Target, budget: &mut f64) -> Activity {
        let task = self.task_of(target);
        let resource = self.sc.rates.tasks[task].resource;
        let cap = self.capacity(task);
        let rate = self.rate(task);
        {
            let v = &mut self.state.villagers[vi];
            if v.carry_resource != Some(resource) {
                // Cambiar de recurso tira lo que llevaba, como en el juego.
                v.carry = 0.0;
                v.carry_resource = Some(resource);
            }
        }
        let carry = self.state.villagers[vi].carry;
        let remaining = self.remaining(target);
        if remaining <= EPS {
            return if carry > EPS { self.leave_to_drop(target) } else { Activity::Idle };
        }
        let amount = (*budget * rate).min(cap - carry).min(remaining);
        let dt = amount / rate;
        *budget -= dt;
        self.state.villagers[vi].carry += amount;
        match target {
            Target::Source(s) => self.state.sources[s].remaining -= amount,
            Target::Farm(f) => self.state.farms[f].remaining -= amount,
        }
        let full = self.state.villagers[vi].carry >= cap - EPS;
        if full || self.remaining(target) <= EPS {
            self.leave_to_drop(target)
        } else {
            Activity::Gathering(target)
        }
    }

    fn leave_to_drop(&self, target: Target) -> Activity {
        let left = self.walk_seconds(target);
        Activity::ToDrop { target, left }
    }

    fn deposit(&mut self, vi: usize) {
        let v = &mut self.state.villagers[vi];
        let (amount, resource) = (v.carry, v.carry_resource);
        v.carry = 0.0;
        if let Some(r) = resource {
            v.gathered[res_index(r)] += amount;
            *self.state.res.get_mut(r) += amount;
            let g = &mut self.summary.gathered;
            *match r {
                aoe2_data::rates::Resource::Food => &mut g.food,
                aoe2_data::rates::Resource::Wood => &mut g.wood,
                aoe2_data::rates::Resource::Gold => &mut g.gold,
                aoe2_data::rates::Resource::Stone => &mut g.stone,
            } += amount;
        }
    }

    fn after_deposit(&self, vi: usize, target: Target) -> Activity {
        let v = &self.state.villagers[vi];
        let same_pool = match target {
            Target::Source(s) => self.state.sources[s].pool == v.pool,
            Target::Farm(_) => Some(v.pool) == self.farm_pool,
        };
        if !same_pool || self.remaining(target) <= EPS {
            return Activity::Idle;
        }
        let left = self.walk_seconds(target);
        if left > EPS { Activity::Back { target, left } } else { Activity::Gathering(target) }
    }

    // ---------- construcción ----------

    fn run_construction(&mut self) {
        for site in 0..self.state.sites.len() {
            if self.state.sites[site].done {
                continue;
            }
            let builders = self
                .state
                .villagers
                .iter()
                .filter(|v| v.activity == Activity::Building { site })
                .count() as u32;
            if builders == 0 {
                continue;
            }
            let b = self.state.sites[site].building;
            let base = self.sc.data.building(b).map_or(1, |b| b.build_time);
            // Tiempo con n constructores = 3t/(n+2): avance por segundo = 1 / ese tiempo.
            self.state.sites[site].progress += 1.0 / self.sc.rates.build_time(base, builders);
            if self.state.sites[site].progress >= 1.0 - EPS {
                self.complete_site(site);
            }
        }
    }

    fn complete_site(&mut self, site: usize) {
        let now = self.state.time + 1;
        self.state.sites[site].done = true;
        self.state.sites[site].finished = Some(now);
        let b = self.state.sites[site].building;
        self.state.built.push((b, now));
        let name = self.building_name(b);
        self.summary.buildings.push((name.clone(), now));
        self.events.push(TimedEvent { time: now, event: Event::BuildFinished { building: name } });

        if b == ids::TOWN_CENTER {
            self.state.tcs.push(TownCenter { site: Some(site), job: None });
        }
        if let Some(owner) = self.state.sites[site].farm_for {
            let food = self.sc.rules.farm.food as f64 + self.state.effects.farm_food_add;
            self.state.farms.push(Farm { remaining: food });
            let f = self.state.farms.len() - 1;
            let vi = self.vidx(owner);
            self.state.villagers[vi].farm = Some(f);
        }
        // Punto de entrega: acorta la caminata de la fuente que sirve.
        let at = self.state.sites[site].at;
        if let Some(si) = self.state.sources.iter().position(|s| {
            s.drop_site == Some(b) && !s.has_drop_site && at.is_none_or(|p| s.pool == p)
        }) {
            let s = &mut self.state.sources[si];
            s.has_drop_site = true;
            if let Some(d) = s.distance_with_drop_site {
                s.distance = d;
            }
        }
        for v in &mut self.state.villagers {
            if v.activity == (Activity::Building { site }) {
                v.activity = Activity::Idle;
            }
        }
    }

    fn check_exhausted(&mut self) {
        for p in 0..self.pools.len() {
            if !self.exhausted[p] && self.pool_exhausted(p) {
                self.exhausted[p] = true;
                let pool = self.pools[p].clone();
                self.events.push(TimedEvent { time: self.state.time + 1, event: Event::PoolExhausted { pool } });
            }
        }
    }

    // ---------- salida ----------

    fn snapshot(&mut self) {
        let snap = self.current_snapshot();
        self.ticks.push(snap);
    }

    /// Foto del estado en este segundo (recursos, población, aldeanos por tarea).
    pub fn current_snapshot(&self) -> Snapshot {
        let mut by_task: BTreeMap<String, u32> = BTreeMap::new();
        for v in &self.state.villagers {
            let key = if v.activity.is_building() {
                "build".to_owned()
            } else if v.idle {
                "idle".to_owned()
            } else {
                self.pools[v.pool].clone()
            };
            *by_task.entry(key).or_default() += 1;
        }
        let r = self.state.res;
        Snapshot {
            time: self.state.time,
            tick: self.state.time / TICK_SECONDS,
            age: self.state.age,
            food: round1(r.food),
            wood: round1(r.wood),
            gold: round1(r.gold),
            stone: round1(r.stone),
            pop: self.state.pop(),
            pop_cap: self.pop_cap(),
            villagers_by_task: by_task,
            tc_idle_seconds: self.summary.tc_idle_total(),
        }
    }

    fn finish(mut self) -> Timeline {
        self.record_second();
        for tc in 0..self.idle_runs.len() {
            self.flush_idle(tc);
        }
        if self.state.time % TICK_SECONDS != 0 {
            self.snapshot();
        }
        self.events.sort_by_key(|e| e.time); // estable: conserva el orden dentro del mismo segundo
        let mut summary = self.summary;
        summary.end_time = self.state.time;
        summary.villagers = self.state.villagers.len() as u32;
        let g = &mut summary.gathered;
        g.food = round1(g.food);
        g.wood = round1(g.wood);
        g.gold = round1(g.gold);
        g.stone = round1(g.stone);
        let villagers = self
            .state
            .villagers
            .iter()
            .map(|v| VillagerInfo {
                id: v.id,
                born: v.born,
                born_tc: v.born_tc,
                gathered: v.gathered.map(round1),
            })
            .collect();
        Timeline { events: self.events, ticks: self.ticks, series: self.series, villagers, summary }
    }

    /// Guarda el estado de este segundo en la serie (una vez por segundo).
    fn record_second(&mut self) {
        if self.series.food.len() as u32 > self.state.time {
            return;
        }
        let r = self.state.res;
        let cap = self.pop_cap();
        let s = &mut self.series;
        s.food.push(r.food as f32);
        s.wood.push(r.wood as f32);
        s.gold.push(r.gold as f32);
        s.stone.push(r.stone as f32);
        s.pop.push(self.state.pop() as u16);
        s.pop_cap.push(cap as u16);
        s.tc_busy.push(self.state.tcs.iter().filter(|t| t.job.is_some()).count() as u8);
        s.tcs.push(self.state.tcs.len() as u8);
    }

    /// Qué hace cada aldeano ahora; publica un `TaskChanged` cuando cambia.
    fn track_tasks(&mut self) {
        for vi in 0..self.state.villagers.len() {
            let v = &self.state.villagers[vi];
            if self.decisions.contains(&v.id) {
                continue; // espera la decisión del optimizador
            }
            let task = match v.activity {
                Activity::Idle => Task::Idle,
                Activity::Gathering(t) | Activity::ToDrop { target: t, .. } | Activity::Back { target: t, .. } => {
                    let pool = match t {
                        Target::Source(s) => self.state.sources[s].pool,
                        Target::Farm(_) => self.farm_pool.unwrap_or(v.pool),
                    };
                    Task::Gather { pool: self.pools[pool].clone() }
                }
                Activity::Luring { source, .. } => Task::Lure { pool: self.pools[self.state.sources[source].pool].clone() },
                Activity::WaitBuild { building, .. } => Task::Wait { building: self.building_name(building) },
                Activity::ToSite { site, .. } | Activity::Building { site } => {
                    let s = &self.state.sites[site];
                    Task::Build { building: self.building_name(s.building), placement: s.placement }
                }
            };
            if v.task.as_ref() != Some(&task) {
                let id = v.id;
                self.state.villagers[vi].task = Some(task.clone());
                self.events.push(TimedEvent { time: self.state.time, event: Event::TaskChanged { villager: id, task } });
            }
        }
    }
}

fn res_index(r: aoe2_data::rates::Resource) -> usize {
    use aoe2_data::rates::Resource::*;
    match r {
        Food => 0,
        Wood => 1,
        Gold => 2,
        Stone => 3,
    }
}

fn round1(x: f64) -> f64 {
    (x * 10.0).round() / 10.0
}
