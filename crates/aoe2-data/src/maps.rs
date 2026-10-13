//! Mapas neutros de prueba (`data/maps/*.toml`).

use crate::{load_toml, Cost, LoadError};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct MapDef {
    pub name: String,
    /// 0 = Laboratorio, 1 = Arabia neutra.
    pub level: u32,
    pub start: StartConditions,
    /// Casillas que camina un constructor hasta cada tipo de edificio, por su
    /// nombre corto (`house`, `mill`…); `default` para el resto.
    pub build_walk_tiles: BTreeMap<String, f64>,
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
    /// Grupo al que las órdenes asignan aldeanos; por defecto, el `id`.
    pool: Option<String>,
    /// Id de tarea en `gather_rates.toml`.
    pub task: String,
    /// Cuántas fuentes iguales hay (ovejas, montones de oro…).
    #[serde(default = "one")]
    pub count: u32,
    /// Recurso por fuente; `None` = infinito.
    pub amount: Option<u32>,
    /// Segundo en que la fuente queda disponible (ovejas que trae el explorador).
    #[serde(default)]
    pub arrives_at_s: u32,
    /// Segundos que pierde el primer aldeano en atraerla al punto de entrega.
    #[serde(default)]
    pub lure_seconds: u32,
    /// Casillas hasta el punto de entrega más cercano al inicio.
    pub distance_tiles: f64,
    /// Casillas si se construye su edificio de entrega al lado.
    pub distance_with_drop_site: Option<f64>,
    /// Id del edificio de entrega que acorta la caminata (molino, campamento…).
    pub drop_site: Option<u32>,
    pub approximate: bool,
    pub note: Option<String>,
}

impl ResourceSource {
    pub fn pool(&self) -> &str {
        self.pool.as_deref().unwrap_or(&self.id)
    }
}

impl MapDef {
    /// Casillas hasta el edificio `name`, o el valor `default` del mapa.
    pub fn build_walk(&self, name: &str) -> f64 {
        self.build_walk_tiles
            .get(name)
            .or_else(|| self.build_walk_tiles.get("default"))
            .copied()
            .unwrap_or(0.0)
    }
}

fn one() -> u32 {
    1
}

impl MapDef {
    pub fn load(path: &Path) -> Result<MapDef, LoadError> {
        load_toml(path)
    }
}
