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
    /// Multiplica la tasa de las tareas `tasks` ("*" = todas).
    RateMult { tasks: Vec<String>, value: f64 },
    /// Suma capacidad de carga a las tareas `tasks` ("*" = todas).
    CarryAdd { tasks: Vec<String>, value: f64 },
    /// Los recursos duran más; `pools` vacío = todos (y las granjas).
    SourceAmountMult {
        #[serde(default)]
        pools: Vec<String>,
        value: f64,
    },
    /// Velocidad de los aldeanos según la edad.
    VillagerSpeedMult { by_age: BTreeMap<Age, f64> },
    /// Multiplica el costo de edificios (`buildings` vacío = todos; `resources`
    /// vacío = todos los recursos) desde una edad.
    BuildingCostMult {
        #[serde(default)]
        buildings: Vec<u32>,
        #[serde(default)]
        resources: Vec<String>,
        value: f64,
        #[serde(default = "dark")]
        from_age: Age,
    },
    /// Suma (o resta) al costo de unos edificios.
    BuildingCostAdd { buildings: Vec<u32>, resources: ResourceDelta },
    /// Tecnologías gratis: las de `techs` y las que se investigan en `researched_at`.
    TechFree {
        #[serde(default)]
        techs: Vec<u32>,
        #[serde(default)]
        researched_at: Vec<u32>,
    },
    /// Multiplica el costo de tecnologías según la edad: las de `techs` y las
    /// de `researched_at` (todo vacío = todas menos las de edad; `resources`
    /// vacío = todos los recursos).
    TechCostMult {
        #[serde(default)]
        techs: Vec<u32>,
        #[serde(default)]
        researched_at: Vec<u32>,
        #[serde(default)]
        resources: Vec<String>,
        by_age: BTreeMap<Age, f64>,
    },
    /// Multiplica el costo de unidades según la edad. Se aplica a las de
    /// `units`, a las entrenadas en `trained_at` y a las de las clases de
    /// armadura `classes` (todo vacío = todas); `resources` vacío = todos.
    UnitCostMult {
        #[serde(default)]
        units: Vec<u32>,
        #[serde(default)]
        trained_at: Vec<u32>,
        #[serde(default)]
        classes: Vec<u32>,
        #[serde(default)]
        resources: Vec<String>,
        by_age: BTreeMap<Age, f64>,
    },
    /// Edificios que entrenan e investigan más rápido, según la edad.
    ProductionSpeedMult { buildings: Vec<u32>, by_age: BTreeMap<Age, f64> },
    /// Tecnologías que se investigan más rápido (`techs` vacío = todas).
    ResearchSpeedMult {
        #[serde(default)]
        techs: Vec<u32>,
        value: f64,
    },
    /// Recursos por cada tecnología investigada.
    ResearchReward { resources: ResourceDelta },
    /// Aldeanos gratis al terminar estas tecnologías.
    TechVillagers { techs: Vec<u32>, value: u32 },
    /// Los aldeanos construyen más rápido (`buildings` vacío = todos).
    BuildSpeedMult {
        #[serde(default)]
        buildings: Vec<u32>,
        value: f64,
    },
    /// Recursos al llegar a cada edad.
    AgeReward { resources: ResourceDelta },
    /// Aldeanos gratis por cada CU al llegar a cada edad.
    AgeVillagers { value: u32 },
    /// Recursos al terminar uno de estos edificios.
    BuildingReward { buildings: Vec<u32>, resources: ResourceDelta },
    /// Población extra que da un edificio.
    PopulationAdd { building: u32, value: u32 },
    /// Comida de cada granja.
    FarmFoodMult { value: f64 },
    /// Efecto de las mejoras de granja (Collera, Arado…).
    FarmTechMult { value: f64 },
    /// No necesita edificios para subir de edad.
    NoAgeBuildings,
}

fn dark() -> Age {
    Age::Dark
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

impl CivInfo {
    /// Líneas de bonos del texto oficial en un idioma: las de la lista y la
    /// de equipo al final (si la hay). Mismo orden en todos los idiomas.
    pub fn bonus_lines(&self, lang: &str) -> Vec<String> {
        let text = self.description.get(lang).replace("\r", "");
        let paras: Vec<&str> = text.split("\n\n").map(str::trim).filter(|p| !p.is_empty()).collect();
        let join = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
        let mut out: Vec<String> = paras.get(1).map_or(Vec::new(), |p| p.split('•').skip(1).map(join).filter(|l| !l.is_empty()).collect());
        if paras.len() > 2 {
            if let Some((_, team)) = paras[paras.len() - 1].split_once('\n') {
                out.push(join(team));
            }
        }
        out
    }
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
