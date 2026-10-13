//! Calculadora: cuánto junta de verdad un grupo de aldeanos en una tarea,
//! con el mismo motor que la simulación completa (carga, caminata, punto de
//! entrega, mejoras).

use crate::orders::BuildOrder;
use crate::scenario::Scenario;
use crate::step::Sim;
use aoe2_data::{Cost, ids};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct CalcInput {
    /// Pool del mapa (sheep, forest, gold…).
    pub pool: String,
    pub villagers: u32,
    pub seconds: u32,
    /// Con el punto de entrega (campamento, molino) ya construido.
    #[serde(default)]
    pub drop_site: bool,
    /// Tecnologías ya investigadas (nombres cortos o ids).
    #[serde(default)]
    pub techs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CalcResult {
    /// Lo depositado en `seconds` (lo que ya está en el almacén).
    pub gathered: f64,
    /// Lo que llevaban encima al final (aún sin depositar).
    pub carrying: f64,
    /// Recurso por segundo por aldeano, contando la caminata.
    pub rate_per_villager: f64,
    /// Tasa de recolección sin caminata (con mejoras).
    pub gather_rate: f64,
    pub carry_capacity: f64,
    /// Casillas hasta el punto de entrega.
    pub distance_tiles: f64,
    /// Segundos de un viaje completo: recolectar la carga + ir y volver.
    pub trip_seconds: f64,
    pub walk_seconds_per_trip: f64,
    /// Viajes completos (depósitos) en total.
    pub trips: u32,
    /// Recurso: food, wood, gold o stone.
    pub resource: String,
    /// La fuente se agotó antes de terminar.
    pub depleted: bool,
}

pub fn calculate(base: &Scenario, input: &CalcInput) -> Result<CalcResult, String> {
    if input.villagers == 0 || input.seconds == 0 {
        return Err("hacen falta al menos 1 aldeano y 1 segundo".into());
    }
    let mut map = base.map.clone();
    map.start.villagers = input.villagers;
    map.start.scouts = 0;
    map.start.resources = Cost::default();
    let src = map
        .sources
        .iter()
        .find(|s| s.pool() == input.pool)
        .ok_or(format!("el mapa no tiene {}", input.pool))?
        .clone();
    // Fuentes que llegan más tarde (ovejas del explorador, ciervos): ya están.
    for s in &mut map.sources {
        s.arrives_at_s = 0;
    }
    let order = BuildOrder::parse(&format!(
        "name = \"calc\"\nmap = \"-\"\ndefault_pool = \"{0}\"\nmax_villagers = {1}\n[[start]]\nvillagers = {1}\npool = \"{0}\"",
        input.pool, input.villagers
    ))
    .map_err(|e| e.to_string())?;
    let sc = Scenario { map, order, ..base.clone() };
    let mut sim = Sim::new(&sc)?;
    for t in &input.techs {
        let id = sc.tech_ref(t).ok_or(format!("tecnología desconocida: {t}"))?;
        sim.grant_tech(id);
    }
    if input.drop_site {
        sim.grant_drop_site(&input.pool);
    }
    let speed = sc.data.unit(ids::VILLAGER).map_or(0.8, |u| u.speed) * sim.state.effects.speed_mult;
    let task_i = sc.rates.tasks.iter().position(|t| t.id == src.task).ok_or("tarea desconocida")?;
    let task = &sc.rates.tasks[task_i];
    let carry_capacity = (sc.rates.carry_for(task) as f64 + sim.state.effects.carry_add[task_i]) * sim.state.effects.carry_mult;
    let gather_rate = task.rate * sim.state.effects.rate_mult[task_i];
    let distance = if input.drop_site { src.distance_with_drop_site.unwrap_or(src.distance_tiles) } else { src.distance_tiles };
    let walk = 2.0 * distance / speed;
    // La tasa de granjero del spec ya incluye la caminata.
    let walk = if task.id == "farmer" { 0.0 } else { walk };

    sim.advance(input.seconds, &|_| false);
    let carrying: f64 = sim.state.villagers.iter().map(|v| v.carry).sum();
    let depleted = sim.state.sources.iter().any(|s| s.pool == sim_pool(&sim, &input.pool) && s.remaining <= 1e-9);
    let tl = sim.into_timeline();
    let r = task.resource;
    let gathered = match r {
        aoe2_data::rates::Resource::Food => tl.summary.gathered.food,
        aoe2_data::rates::Resource::Wood => tl.summary.gathered.wood,
        aoe2_data::rates::Resource::Gold => tl.summary.gathered.gold,
        aoe2_data::rates::Resource::Stone => tl.summary.gathered.stone,
    };
    let trip_seconds = carry_capacity / gather_rate + walk;
    Ok(CalcResult {
        gathered,
        carrying: (carrying * 10.0).round() / 10.0,
        rate_per_villager: carry_capacity / trip_seconds,
        gather_rate,
        carry_capacity,
        distance_tiles: distance,
        trip_seconds,
        walk_seconds_per_trip: walk,
        trips: (gathered / carry_capacity).round() as u32,
        resource: format!("{r:?}").to_lowercase(),
        depleted,
    })
}

fn sim_pool(sim: &Sim, pool: &str) -> usize {
    sim.pool_names().iter().position(|p| p == pool).unwrap_or(usize::MAX)
}
