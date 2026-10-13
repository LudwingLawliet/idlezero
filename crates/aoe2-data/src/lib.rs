//! Tipos y carga de los datos del juego que viven en `data/`.
//!
//! El motor nunca lee el JSON de terceros (`vendor/`): `tools/import` lo
//! convierte a `data/game_data.json` con el esquema definido aquí.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

pub mod civs;
pub mod i18n;
pub mod maps;
pub mod modifiers;
pub mod rates;
pub mod rules;

pub use civs::Civ;
pub use i18n::Locale;
pub use maps::MapDef;
pub use modifiers::Modifiers;
pub use rates::GatherRates;
pub use rules::Rules;

/// Versión del esquema de `game_data.json`. Subirla al cambiar los tipos.
pub const SCHEMA_VERSION: u32 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Age {
    Dark,
    Feudal,
    Castle,
    Imperial,
}

impl Age {
    /// Convierte el `age_id` de aoe2techtree (1 = Oscura … 4 = Imperial).
    pub fn from_techtree_id(id: u32) -> Option<Age> {
        match id {
            1 => Some(Age::Dark),
            2 => Some(Age::Feudal),
            3 => Some(Age::Castle),
            4 => Some(Age::Imperial),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cost {
    #[serde(default)]
    pub food: u32,
    #[serde(default)]
    pub wood: u32,
    #[serde(default)]
    pub gold: u32,
    #[serde(default)]
    pub stone: u32,
}

impl Cost {
    pub const fn new(food: u32, wood: u32, gold: u32, stone: u32) -> Cost {
        Cost { food, wood, gold, stone }
    }
}

impl fmt::Display for Cost {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}F {}W {}G {}S", self.food, self.wood, self.gold, self.stone)
    }
}

/// Nombre en los idiomas que usa la interfaz.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Names {
    pub en: String,
    pub es: String,
}

impl Names {
    /// Nombre en `lang`; inglés si el idioma no está.
    pub fn get(&self, lang: &str) -> &str {
        match lang {
            "es" => &self.es,
            _ => &self.en,
        }
    }
}

/// Dónde y cuándo está disponible una entidad en el árbol tecnológico.
/// Se toma el valor más común entre civilizaciones (escenario neutro).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TreeInfo {
    /// Edad mínima en la que está disponible (Edad Feudal se investiga en Oscura).
    pub age: Age,
    /// Edificio que la produce o investiga (para edificios, `None`).
    pub produced_at: Option<u32>,
    /// Edificio que debe existir antes (p. ej. Cuartel para Galería de tiro).
    pub requires_building: Option<u32>,
    /// Cuántas civilizaciones tienen acceso.
    pub civ_count: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Unit {
    pub id: u32,
    pub internal_name: String,
    pub names: Names,
    pub cost: Cost,
    /// Tiempo de entrenamiento en segundos.
    pub train_time: u32,
    pub hp: i32,
    /// Casillas por segundo.
    pub speed: f64,
    pub tree: Option<TreeInfo>,
    /// Datos de combate (simulador de batalla y carta de la unidad).
    #[serde(default)]
    pub combat: Combat,
}

/// Ataque o armadura contra una clase (ver data/armor_classes.toml).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClassAmount {
    pub class: u32,
    pub amount: i32,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Combat {
    pub attack: i32,
    pub melee_armor: i32,
    pub pierce_armor: i32,
    /// Casillas.
    pub range: f64,
    pub min_range: f64,
    /// Segundos entre ataques.
    pub reload: f64,
    pub line_of_sight: f64,
    pub accuracy: u32,
    /// Ataque por clase (incluye los bonos contra clases concretas).
    pub attacks: Vec<ClassAmount>,
    /// Armadura por clase.
    pub armours: Vec<ClassAmount>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Building {
    pub id: u32,
    pub internal_name: String,
    pub names: Names,
    pub cost: Cost,
    /// Tiempo de construcción en segundos con un solo aldeano.
    pub build_time: u32,
    pub hp: i32,
    pub tree: Option<TreeInfo>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tech {
    pub id: u32,
    pub internal_name: String,
    pub names: Names,
    pub cost: Cost,
    /// Tiempo de investigación en segundos.
    pub research_time: u32,
    pub repeatable: bool,
    pub tree: Option<TreeInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceInfo {
    pub repository: String,
    pub commit: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameData {
    pub schema_version: u32,
    pub source: SourceInfo,
    pub units: BTreeMap<u32, Unit>,
    pub buildings: BTreeMap<u32, Building>,
    pub techs: BTreeMap<u32, Tech>,
    /// Unidades que se obtienen con una mejora (Ballestero = investigar
    /// "Crossbow"): unidad -> tecnología de mejora (incluida en `techs`).
    #[serde(default)]
    pub unit_upgrades: BTreeMap<u32, u32>,
    /// Línea de cada unidad: unidad -> la anterior (Ballestero -> Arquero).
    #[serde(default)]
    pub unit_previous: BTreeMap<u32, u32>,
    /// Tecnología -> la que exige antes (Alto horno -> Fundición), según el árbol.
    #[serde(default)]
    pub tech_previous: BTreeMap<u32, u32>,
}

/// IDs de aoe2techtree que el motor necesita con nombre propio.
pub mod ids {
    pub const VILLAGER: u32 = 83;
    pub const ARCHER: u32 = 4;
    pub const TOWN_CENTER: u32 = 109;
    pub const HOUSE: u32 = 70;
    pub const MILL: u32 = 68;
    pub const LUMBER_CAMP: u32 = 562;
    pub const MINING_CAMP: u32 = 584;
    pub const FARM: u32 = 50;
    pub const FEUDAL_AGE: u32 = 101;
    pub const CASTLE_AGE: u32 = 102;
    pub const IMPERIAL_AGE: u32 = 103;
}

#[derive(Debug)]
pub enum LoadError {
    Io(PathBuf, std::io::Error),
    Json(PathBuf, serde_json::Error),
    Toml(PathBuf, toml::de::Error),
    SchemaVersion { found: u32, expected: u32 },
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadError::Io(p, e) => write!(f, "{}: {e}", p.display()),
            LoadError::Json(p, e) => write!(f, "{}: {e}", p.display()),
            LoadError::Toml(p, e) => write!(f, "{}: {e}", p.display()),
            LoadError::SchemaVersion { found, expected } => write!(
                f,
                "game_data.json tiene schema_version {found}, se esperaba {expected}; re-ejecuta tools/import"
            ),
        }
    }
}

impl std::error::Error for LoadError {}

fn read(path: &Path) -> Result<String, LoadError> {
    std::fs::read_to_string(path).map_err(|e| LoadError::Io(path.to_owned(), e))
}

pub(crate) fn load_toml<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, LoadError> {
    toml::from_str(&read(path)?).map_err(|e| LoadError::Toml(path.to_owned(), e))
}

/// Carpeta `data/` del repositorio (útil para tests y herramientas).
pub fn repo_data_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data")
}

impl GameData {
    pub fn load(path: &Path) -> Result<GameData, LoadError> {
        let data: GameData =
            serde_json::from_str(&read(path)?).map_err(|e| LoadError::Json(path.to_owned(), e))?;
        if data.schema_version != SCHEMA_VERSION {
            return Err(LoadError::SchemaVersion {
                found: data.schema_version,
                expected: SCHEMA_VERSION,
            });
        }
        Ok(data)
    }

    pub fn unit(&self, id: u32) -> Option<&Unit> {
        self.units.get(&id)
    }

    pub fn building(&self, id: u32) -> Option<&Building> {
        self.buildings.get(&id)
    }

    pub fn tech(&self, id: u32) -> Option<&Tech> {
        self.techs.get(&id)
    }
}
