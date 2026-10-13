//! Mapas neutros de prueba (`data/maps/*.toml`).

use crate::{load_toml, Cost, LoadError};
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct MapDef {
    pub name: String,
    /// 0 = Laboratorio, 1 = Arabia neutra.
    pub level: u32,
    pub start: StartConditions,
    #[serde(rename = "source")]
    pub sources: Vec<ResourceSource>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StartConditions {
    pub resources: Cost,
    pub villagers: u32,
    pub scouts: u32,
    pub town_centers: u32,
    pub source: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ResourceSource {
    pub id: String,
    /// Id de tarea en `gather_rates.toml`.
    pub task: String,
    /// Cuántas fuentes iguales hay (ovejas, montones de oro…).
    #[serde(default = "one")]
    pub count: u32,
    /// Recurso por fuente; `None` = infinito.
    pub amount: Option<u32>,
    /// Casillas hasta el punto de entrega más cercano al inicio.
    pub distance_tiles: f64,
    /// Casillas si se construye su edificio de entrega al lado.
    pub distance_with_drop_site: Option<f64>,
    /// Id del edificio de entrega que acorta la caminata (molino, campamento…).
    pub drop_site: Option<u32>,
    pub approximate: bool,
    pub note: Option<String>,
}

fn one() -> u32 {
    1
}

impl MapDef {
    pub fn load(path: &Path) -> Result<MapDef, LoadError> {
        load_toml(path)
    }
}
