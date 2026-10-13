//! Civilizaciones: árbol tecnológico (`data/civs.json`, del importador) y
//! bonos que simula el motor (`data/civ_bonuses.toml`).

use crate::{Age, LoadError, Names, load_toml};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// Una civilización tal como la exporta el importador.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CivInfo {
    pub id: String,
    pub names: Names,
    /// Texto oficial del juego con sus bonos.
    pub description: Names,
    pub icon: String,
    pub units: Vec<u32>,
    pub techs: Vec<u32>,
    pub buildings: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ResourceDelta {
    #[serde(default)]
    pub food: i32,
    #[serde(default)]
    pub wood: i32,
    #[serde(default)]
    pub gold: i32,
    #[serde(default)]
    pub stone: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BonusKind {
    NoHouses,
    StartResourcesAdd { resources: ResourceDelta },
    StartVillagersAdd { value: u32 },
    RateMult { tasks: Vec<String>, value: f64 },
    SourceAmountMult { value: f64 },
    BuildingCostMult { building: u32, resource: String, value: f64, from_age: Age },
    TechFree { techs: Vec<u32> },
    UnitCostMult { units: Vec<u32>, by_age: BTreeMap<Age, f64> },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bonus {
    #[serde(flatten)]
    pub kind: BonusKind,
    /// Línea del texto oficial que respalda el bono.
    pub text: String,
    #[serde(default)]
    pub approximate: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct BonusFile {
    civ: Vec<CivBonuses>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct CivBonuses {
    pub id: String,
    #[serde(rename = "bonus", default)]
    pub bonuses: Vec<Bonus>,
}

/// Civilización lista para el simulador: árbol + bonos simulados.
#[derive(Debug, Clone, PartialEq)]
pub struct Civ {
    pub info: CivInfo,
    pub bonuses: Vec<Bonus>,
}

pub fn parse_civs(json: &str) -> Result<Vec<CivInfo>, serde_json::Error> {
    serde_json::from_str(json)
}

pub fn parse_bonuses(toml_text: &str) -> Result<Vec<CivBonuses>, toml::de::Error> {
    Ok(toml::from_str::<BonusFile>(toml_text)?.civ)
}

impl Civ {
    /// Une el árbol de `civs` con los bonos de `bonuses` para la civilización `id`.
    pub fn build(id: &str, civs: &[CivInfo], bonuses: &[CivBonuses]) -> Option<Civ> {
        let info = civs.iter().find(|c| c.id == id)?.clone();
        let bonuses = bonuses.iter().find(|b| b.id == id).map(|b| b.bonuses.clone()).unwrap_or_default();
        Some(Civ { info, bonuses })
    }

    pub fn load(data_dir: &Path, id: &str) -> Result<Option<Civ>, LoadError> {
        let path = data_dir.join("civs.json");
        let text = std::fs::read_to_string(&path).map_err(|e| LoadError::Io(path.clone(), e))?;
        let civs = parse_civs(&text).map_err(|e| LoadError::Json(path, e))?;
        let bonus_path = data_dir.join("civ_bonuses.toml");
        let file: BonusFile = load_toml(&bonus_path)?;
        Ok(Civ::build(id, &civs, &file.civ))
    }

    pub fn has(&self, f: impl Fn(&BonusKind) -> bool) -> bool {
        self.bonuses.iter().any(|b| f(&b.kind))
    }

    pub fn no_houses(&self) -> bool {
        self.has(|k| matches!(k, BonusKind::NoHouses))
    }

    pub fn has_unit(&self, id: u32) -> bool {
        self.info.units.contains(&id)
    }

    pub fn has_tech(&self, id: u32) -> bool {
        self.info.techs.contains(&id)
    }

    pub fn has_building(&self, id: u32) -> bool {
        self.info.buildings.contains(&id)
    }
}
