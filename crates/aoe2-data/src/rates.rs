//! Tasas de recolección y reglas de aldeano (`data/gather_rates.toml`).
//!
//! No vienen en los datos del juego: las mide la comunidad. Cada valor lleva
//! su fuente y una marca de *aproximado* hasta validarlo en partida.

use crate::{load_toml, LoadError};
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Resource {
    Food,
    Wood,
    Gold,
    Stone,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GatherRates {
    pub schema_version: u32,
    pub carry: Carry,
    pub build: BuildRule,
    #[serde(rename = "task")]
    pub tasks: Vec<GatherTask>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Carry {
    pub default: u32,
    pub hunting: u32,
    pub source: String,
    pub approximate: bool,
}

/// Tiempo con n constructores = `numerator * t / (n + offset)`.
#[derive(Debug, Clone, Deserialize)]
pub struct BuildRule {
    pub numerator: u32,
    pub offset: u32,
    pub source: String,
    pub approximate: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GatherTask {
    pub id: String,
    pub resource: Resource,
    /// Recursos por segundo sin mejoras, sin contar caminata.
    pub rate: f64,
    /// Usa la carga de caza en lugar de la normal.
    #[serde(default)]
    pub hunting: bool,
    pub source: String,
    pub approximate: bool,
}

impl GatherRates {
    pub fn load(path: &Path) -> Result<GatherRates, LoadError> {
        load_toml(path)
    }

    pub fn task(&self, id: &str) -> Option<&GatherTask> {
        self.tasks.iter().find(|t| t.id == id)
    }

    pub fn carry_for(&self, task: &GatherTask) -> u32 {
        if task.hunting { self.carry.hunting } else { self.carry.default }
    }

    /// Segundos que tarda un edificio de `base_time` s con `builders` aldeanos.
    pub fn build_time(&self, base_time: u32, builders: u32) -> f64 {
        assert!(builders > 0, "se necesita al menos un constructor");
        (self.build.numerator * base_time) as f64 / (builders + self.build.offset) as f64
    }
}

/// Tasa efectiva con caminata: carga ÷ (carga ÷ tasa + 2 × caminata_s).
pub fn effective_rate(carry: u32, rate: f64, walk_seconds_one_way: f64) -> f64 {
    let carry = carry as f64;
    carry / (carry / rate + 2.0 * walk_seconds_one_way)
}
