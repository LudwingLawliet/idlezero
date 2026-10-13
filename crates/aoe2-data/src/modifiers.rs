//! Efectos económicos de las tecnologías (`data/modifiers.toml`).

use crate::{load_toml, LoadError};
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct Modifiers {
    pub schema_version: u32,
    #[serde(rename = "modifier")]
    pub modifiers: Vec<Modifier>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModifierKind {
    RateMult,
    CarryAdd,
    CarryMult,
    SpeedMult,
    FarmFoodAdd,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Modifier {
    pub tech: u32,
    pub kind: ModifierKind,
    /// Tareas afectadas (ids de `gather_rates.toml`); vacío = todas.
    #[serde(default)]
    pub tasks: Vec<String>,
    pub value: f64,
    pub source: String,
    pub approximate: bool,
}

impl Modifier {
    pub fn applies_to(&self, task: &str) -> bool {
        self.tasks.is_empty() || self.tasks.iter().any(|t| t == task)
    }
}

impl Modifiers {
    pub fn load(path: &Path) -> Result<Modifiers, LoadError> {
        load_toml(path)
    }

    pub fn for_tech(&self, tech: u32) -> impl Iterator<Item = &Modifier> {
        self.modifiers.iter().filter(move |m| m.tech == tech)
    }
}
