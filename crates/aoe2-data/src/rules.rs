//! Reglas del juego que no vienen en aoe2techtree (`data/rules.toml`).

use crate::{load_toml, Age, LoadError};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct Rules {
    pub schema_version: u32,
    pub population: PopulationRules,
    pub farm: FarmRules,
    #[serde(rename = "age_requirement")]
    pub age_requirements: Vec<AgeRequirement>,
    /// Tecnología de edad (id como texto) -> edad a la que lleva.
    pub ages: BTreeMap<String, Age>,
    pub names: Names,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PopulationRules {
    pub per_town_center: u32,
    pub per_house: u32,
    pub max: u32,
    pub source: String,
    pub approximate: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FarmRules {
    pub food: u32,
    pub auto_reseed: bool,
    pub source: String,
    pub approximate: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AgeRequirement {
    pub tech: u32,
    pub count: u32,
    pub buildings: Vec<u32>,
    pub source: String,
    pub approximate: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Names {
    pub buildings: BTreeMap<String, u32>,
    pub techs: BTreeMap<String, u32>,
    pub units: BTreeMap<String, u32>,
}

impl Rules {
    pub fn load(path: &Path) -> Result<Rules, LoadError> {
        load_toml(path)
    }

    /// Edad a la que lleva la tecnología `tech`, si es una tecnología de edad.
    pub fn age_of_tech(&self, tech: u32) -> Option<Age> {
        self.ages.get(&tech.to_string()).copied()
    }

    pub fn age_requirement(&self, tech: u32) -> Option<&AgeRequirement> {
        self.age_requirements.iter().find(|r| r.tech == tech)
    }

    pub fn building_id(&self, name: &str) -> Option<u32> {
        self.names.buildings.get(name).copied()
    }

    pub fn tech_id(&self, name: &str) -> Option<u32> {
        self.names.techs.get(name).copied()
    }

    pub fn unit_id(&self, name: &str) -> Option<u32> {
        self.names.units.get(name).copied()
    }

    /// Nombre corto de una tecnología (inverso de `tech_id`).
    pub fn tech_name(&self, id: u32) -> Option<&str> {
        self.names.techs.iter().find(|(_, v)| **v == id).map(|(k, _)| k.as_str())
    }

    /// Edad a la que lleva cada tecnología de edad, al revés: tecnología de `age`.
    pub fn age_tech(&self, age: Age) -> Option<u32> {
        self.ages.iter().find(|(_, a)| **a == age).and_then(|(t, _)| t.parse().ok())
    }

    /// Nombre corto de un edificio (inverso de `building_id`).
    pub fn building_name(&self, id: u32) -> Option<&str> {
        self.names.buildings.iter().find(|(_, v)| **v == id).map(|(k, _)| k.as_str())
    }
}
