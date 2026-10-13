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

pub struct Sim<'a> {
    sc: &'a Scenario,
    pub state: GameState,
    pools: Vec<String>,
    farm_pool: Option<usize>,
    steps: Vec<StepState>,
    villager_speed: f64,
    events: Vec<TimedEvent>,
    ticks: Vec<Snapshot>,
    summary: Summary,
    idle_run: Option<(IdleCause, u32)>,
    exhausted: Vec<bool>,
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
            sc.rules.building_id(name).ok_or(format!("edificio desconocido en la orden: {name}"))
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
                    sc.rules.tech_id(t).ok_or(format!("tecnología desconocida en la orden: {t}"))?;
                }
                StepAction::Move { from, to, .. } => {
                    check_pool(from)?;
                    check_pool(to)?;
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
                sc.rules.tech_id(t).ok_or(format!("tecnología desconocida en la orden: {t}"))?;
            }
        }

        let villager = sc.data.unit(ids::VILLAGER).ok_or("falta el aldeano en game_data")?;
        let n_tasks = sc.rates.tasks.len();
        let n_pools = pools.len();
        let start = &sc.map.start;
        let mut sim = Sim {
            sc,
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
                tc: None,
                research: Vec::new(),
                pending_research: Vec::new(),
                next_villager: 1,
            },
            pools,
            farm_pool,
            steps: vec![StepState::Waiting; order.steps.len()],
            villager_speed: villager.speed,
            events: Vec::new(),
            ticks: Vec::new(),
            summary: Summary::default(),
            idle_run: None,
            exhausted: vec![false; n_pools],
        };

        // Aldeanos iniciales: cada entrada [[start]] con `build` comparte un edificio.
        let assigned: u32 = order.start.iter().map(|s| s.villagers).sum();
        if assigned > start.villagers {
            return Err(format!("la orden asigna {assigned} aldeanos iniciales y el mapa da {}", start.villagers));
        }
        for entry in &order.start {
            let ids: Vec<u32> = (0..entry.villagers).map(|_| sim.spawn_villager()).collect();
            let a = &entry.assign;
            let Some(name) = a.build.as_deref() else {
                let p = sim.pool(a.pool.as_deref().unwrap());
                for id in ids {
                    let vi = sim.vidx(id);
                    sim.state.villagers[vi].pool = p;
                }
                continue;
            };
            let b = sc.rules.building_id(name).unwrap();
            let then = sim.pool(a.then.as_deref().unwrap());
            let at = a.at.as_deref().map(|p| sim.pool(p));
            let cost = sim.building_cost(b);
            let site = if sim.state.res.can_afford(&cost) {
                sim.state.res.pay(&cost);
                Some(sim.new_site(b, None, at))
            } else {
                None
            };
            for &id in &ids {
                let vi = sim.vidx(id);
                sim.state.villagers[vi].pool = then;
                match site {
                    Some(site) => sim.send_to_site(vi, site, sc.map.build_walk(name)),
                    None => sim.state.villagers[vi].activity = Activity::WaitBuild { building: b, at },
                }
            }
            if site.is_some() {
                sim.event(Event::BuildStarted { building: name.to_owned(), builders: ids });
            }
        }
        while sim.state.villagers.len() < start.villagers as usize {
            sim.spawn_villager();
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

    fn spawn_villager(&mut self) -> u32 {
        let id = self.state.next_villager;
        self.state.next_villager += 1;
        let pool = self.pool(&self.sc.order.default_pool);
        self.state.villagers.push(Villager {
            id,
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
        self.state.sites.push(Site { building, progress: 0.0, done: false, farm_for, at });
        self.state.sites.len() - 1
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
        self.run_steps();
        self.run_tc();
        self.run_research();
        for vi in 0..self.state.villagers.len() {
            self.act(vi);
        }
        self.run_construction();
        self.check_exhausted();
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
            && c.researched.as_deref().is_none_or(|t| s.has_tech(self.sc.rules.tech_id(t).unwrap()))
            && c.built.as_deref().is_none_or(|b| s.count_built(self.sc.rules.building_id(b).unwrap()) > 0)
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
                    let tech = self.sc.rules.tech_id(t).unwrap();
                    self.state.pending_research.push(tech);
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
                    let b = self.sc.rules.building_id(building).unwrap();
                    if !self.building_available(b) || !self.state.res.can_afford(&self.building_cost(b)) {
                        continue;
                    }
                    let (from, then) = (self.pool(from), self.pool(then));
                    let chosen: Vec<usize> = (0..self.state.villagers.len())
                        .rev()
                        .filter(|&vi| {
                            let v = &self.state.villagers[vi];
                            v.pool == from && !v.activity.is_building()
                        })
                        .take(builders as usize)
                        .collect();
                    if chosen.is_empty() {
                        continue;
                    }
                    let cost = self.building_cost(b);
                    self.state.res.pay(&cost);
                    let at = at.map(|p| self.pool(p));
                    let site = self.new_site(b, None, at);
                    let walk = self.sc.map.build_walk(building);
                    let mut ids = Vec::new();
                    for &vi in chosen.iter().rev() {
                        self.state.villagers[vi].pool = then;
                        self.send_to_site(vi, site, walk);
                        ids.push(self.state.villagers[vi].id);
                    }
                    self.event(Event::BuildStarted { building: building.to_owned(), builders: ids });
                    self.steps[i] = StepState::Done;
                }
            }
        }
    }

    fn building_available(&self, b: u32) -> bool {
        self.sc
            .data
            .building(b)
            .and_then(|b| b.tree.as_ref())
            .is_none_or(|t| t.age <= self.state.age)
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
        if self.state.tc.is_none() {
            self.start_tc_job();
        }
        if let Some(mut r) = self.state.tc {
            r.left -= 1;
            if r.left == 0 {
                self.state.tc = None;
                self.finish_job(r);
            } else {
                self.state.tc = Some(r);
            }
        }
    }

    fn start_tc_job(&mut self) {
        let tc_research = self
            .state
            .pending_research
            .iter()
            .position(|&t| self.research_building(t) == ids::TOWN_CENTER);
        let cause = if let Some(pos) = tc_research {
            let tech = self.state.pending_research[pos];
            let cost = self.sc.data.tech(tech).unwrap().cost;
            if self.research_ready(tech) && self.state.res.can_afford(&cost) {
                self.state.res.pay(&cost);
                self.state.pending_research.remove(pos);
                let time = self.sc.data.tech(tech).unwrap().research_time;
                self.state.tc = Some(Running { building: ids::TOWN_CENTER, job: Job::Research(tech), left: time });
                if let Some(age) = self.sc.rules.age_of_tech(tech) {
                    self.summary.age_clicks.insert(age, self.state.time);
                }
                let tech = self.tech_name(tech);
                self.event(Event::ResearchStarted { tech });
                None
            } else {
                Some(IdleCause::WaitingResearch)
            }
        } else {
            let v = self.sc.data.unit(ids::VILLAGER).unwrap();
            let enough = self.sc.order.max_villagers.is_some_and(|m| self.state.villagers.len() as u32 >= m);
            if enough {
                Some(IdleCause::NoOrder)
            } else if self.state.pop() >= self.pop_cap() {
                Some(IdleCause::Housed)
            } else if !self.state.res.can_afford(&v.cost) {
                Some(IdleCause::NoFood)
            } else {
                self.state.res.pay(&v.cost);
                self.state.tc = Some(Running { building: ids::TOWN_CENTER, job: Job::Villager, left: v.train_time });
                None
            }
        };
        self.track_idle(cause);
    }

    fn track_idle(&mut self, cause: Option<IdleCause>) {
        if let Some(c) = cause {
            *self.summary.tc_idle_seconds.entry(c).or_default() += 1;
        }
        let current = self.idle_run.map(|(c, _)| c);
        if current != cause {
            self.flush_idle();
            self.idle_run = cause.map(|c| (c, self.state.time));
        }
    }

    fn flush_idle(&mut self) {
        if let Some((cause, since)) = self.idle_run.take() {
            let seconds = self.state.time - since;
            self.events.push(TimedEvent { time: since, event: Event::TcIdle { cause, seconds } });
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
                self.state.res.pay(&cost);
                self.state.pending_research.remove(i);
                let time = self.sc.data.tech(tech).unwrap().research_time;
                self.state.research.push(Running { building: b, job: Job::Research(tech), left: time });
                let tech = self.tech_name(tech);
                self.event(Event::ResearchStarted { tech });
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
            self.finish_job(r);
        }
    }

    /// Efectos de un trabajo terminado; ocurre al final del segundo actual.
    fn finish_job(&mut self, r: Running) {
        let now = self.state.time + 1;
        match r.job {
            Job::Villager => {
                let id = self.spawn_villager();
                let vi = self.vidx(id);
                let assign = self.sc.order.assignment_for(id).cloned();
                let label = match &assign {
                    Some(a) if a.build.is_some() => {
                        let b = self.sc.rules.building_id(a.build.as_deref().unwrap()).unwrap();
                        let at = a.at.as_deref().map(|p| self.pool(p));
                        self.state.villagers[vi].pool = self.pool(a.then.as_deref().unwrap());
                        self.state.villagers[vi].activity = Activity::WaitBuild { building: b, at };
                        format!("{} → {}", a.build.as_deref().unwrap(), a.then.as_deref().unwrap())
                    }
                    Some(a) => {
                        self.state.villagers[vi].pool = self.pool(a.pool.as_deref().unwrap());
                        a.pool.clone().unwrap()
                    }
                    None => self.sc.order.default_pool.clone(),
                };
                self.events.push(TimedEvent { time: now, event: Event::VillagerCreated { villager: id, assignment: label } });
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
        let farm = self.sc.rules.building_id("farm").unwrap_or(ids::FARM);
        let cost = self.building_cost(farm);
        if !self.state.res.can_afford(&cost) || (had_farm && !self.sc.rules.farm.auto_reseed) {
            return None;
        }
        self.state.res.pay(&cost);
        let id = self.state.villagers[vi].id;
        let site = self.new_site(farm, Some(id), None);
        // Resembrar es en el mismo sitio: sin caminata.
        let walk = if had_farm { 0.0 } else { self.sc.map.build_walk("farm") / self.speed() };
        self.event(Event::BuildStarted { building: "farm".into(), builders: vec![id] });
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
                Activity::WaitBuild { building, at } => {
                    if self.building_available(building) && self.state.res.can_afford(&self.building_cost(building)) {
                        let cost = self.building_cost(building);
                        self.state.res.pay(&cost);
                        let site = self.new_site(building, None, at);
                        let name = self.building_name(building);
                        let id = self.state.villagers[vi].id;
                        self.event(Event::BuildStarted { building: name.clone(), builders: vec![id] });
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
        let b = self.state.sites[site].building;
        self.state.built.push((b, now));
        let name = self.building_name(b);
        self.summary.buildings.push((name.clone(), now));
        self.events.push(TimedEvent { time: now, event: Event::BuildFinished { building: name } });

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
        self.ticks.push(Snapshot {
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
        });
    }

    fn finish(mut self) -> Timeline {
        self.flush_idle();
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
        Timeline { events: self.events, ticks: self.ticks, summary }
    }
}

fn round1(x: f64) -> f64 {
    (x * 10.0).round() / 10.0
}
