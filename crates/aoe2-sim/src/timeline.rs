//! Resultado de una simulación: eventos segundo a segundo y una foto por tick.

use aoe2_data::Age;
use serde::Serialize;
use std::collections::BTreeMap;

/// Segundos por tick aldeano (lo que tarda el CU en crear un aldeano).
pub const TICK_SECONDS: u32 = 25;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Timeline {
    pub events: Vec<TimedEvent>,
    pub ticks: Vec<Snapshot>,
    /// Estado de cada segundo (para inspeccionar cualquier momento).
    pub series: Series,
    /// Cada aldeano: cuándo y de dónde nació y lo que juntó.
    pub villagers: Vec<VillagerInfo>,
    pub summary: Summary,
}

/// Qué hace un aldeano (cada cambio es un evento `TaskChanged`).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Task {
    /// Recolectar en un pool (incluye ir y volver del punto de entrega).
    Gather { pool: String },
    /// Atraer un jabalí al CU.
    Lure { pool: String },
    /// Construir (incluye caminar hasta la obra). `placement` = nº de edificio colocado.
    Build { building: String, placement: usize },
    /// Esperando recursos para colocar un edificio.
    Wait { building: String },
    Idle,
}

/// Estado segundo a segundo, en columnas (compacto para la interfaz).
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Series {
    pub food: Vec<f32>,
    pub wood: Vec<f32>,
    pub gold: Vec<f32>,
    pub stone: Vec<f32>,
    pub pop: Vec<u16>,
    pub pop_cap: Vec<u16>,
    /// CU trabajando en ese segundo (de cuántos hay).
    pub tc_busy: Vec<u8>,
    pub tcs: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct VillagerInfo {
    pub id: u32,
    pub born: u32,
    /// Nº de CU del que nació (0 = el inicial); `None` = aldeano inicial.
    pub born_tc: Option<usize>,
    /// Comida, madera, oro y piedra depositadas.
    pub gathered: [f64; 4],
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TimedEvent {
    pub time: u32,
    pub event: Event,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    VillagerCreated { villager: u32, assignment: String },
    BuildStarted { building: String, builders: Vec<u32>, placement: usize },
    BuildFinished { building: String },
    ResearchStarted { tech: String },
    ResearchFinished { tech: String },
    AgeReached { age: Age },
    Moved { villagers: Vec<u32>, from: String, to: String },
    PoolExhausted { pool: String },
    Lured { source: String, villager: u32 },
    TcIdle { cause: IdleCause, seconds: u32, tc: usize },
    UnitTrained { unit: String, count: u32 },
    TaskChanged { villager: u32, task: Task },
    /// Pago de algo: qué, cuánto y cómo quedaron las reservas.
    Spent { what: String, cost: aoe2_data::Cost, stock: [f64; 4] },
    /// Momento en que el optimizador (u otra fuente externa) tomó una decisión.
    Decision { villager: u32, choice: String },
    StepBlocked { step: usize, reason: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IdleCause {
    /// Población al tope: falta una casa.
    Housed,
    /// No hay 50 de comida para el siguiente aldeano.
    NoFood,
    /// Esperando recursos o requisitos para una investigación del CU.
    WaitingResearch,
    /// La orden no pide más aldeanos.
    NoOrder,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Snapshot {
    pub time: u32,
    pub tick: u32,
    pub age: Age,
    pub food: f64,
    pub wood: f64,
    pub gold: f64,
    pub stone: f64,
    pub pop: u32,
    pub pop_cap: u32,
    /// Aldeanos por actividad: nombre de pool, "build" o "idle".
    pub villagers_by_task: BTreeMap<String, u32>,
    pub tc_idle_seconds: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Summary {
    pub end_time: u32,
    pub villagers: u32,
    /// Segundo en que se hizo clic en cada edad.
    pub age_clicks: BTreeMap<Age, u32>,
    /// Segundo en que se alcanzó cada edad.
    pub age_reached: BTreeMap<Age, u32>,
    pub tc_idle_seconds: BTreeMap<IdleCause, u32>,
    /// Suma de segundos de aldeanos sin nada que hacer.
    pub villager_idle_seconds: u32,
    pub gathered: Gathered,
    /// Todo lo pagado (aldeanos, edificios, investigaciones, unidades).
    pub spent: Gathered,
    /// (edificio, segundo en que se terminó).
    pub buildings: Vec<(String, u32)>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Gathered {
    pub food: f64,
    pub wood: f64,
    pub gold: f64,
    pub stone: f64,
}

impl Summary {
    pub fn tc_idle_total(&self) -> u32 {
        self.tc_idle_seconds.values().sum()
    }
}

/// "m:ss" para mostrar segundos de juego.
pub fn clock(seconds: u32) -> String {
    format!("{}:{:02}", seconds / 60, seconds % 60)
}
