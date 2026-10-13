//! Órdenes de construcción como datos (`data/build_orders/*.toml`).
//!
//! Una orden dice qué hace cada aldeano al nacer y qué hitos se disparan
//! cuando se cumplen ciertas condiciones. El simulador la ejecuta tal cual;
//! el optimizador (Fase 3) generará órdenes con este mismo formato.

use aoe2_data::{Age, LoadError};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildOrder {
    pub name: String,
    /// Mapa para el que está pensada (informativo).
    pub map: String,
    /// Pool de los aldeanos que no tienen asignación explícita.
    pub default_pool: String,
    /// El CU deja de crear aldeanos al llegar a este número.
    pub max_villagers: Option<u32>,
    /// Asignación de los aldeanos iniciales, en orden.
    #[serde(rename = "start", default)]
    pub start: Vec<StartAssign>,
    /// Asignación de los aldeanos nuevos por número (4 = el primero creado).
    #[serde(rename = "villager", default)]
    pub villagers: Vec<VillagerAssign>,
    /// Hitos: cada uno se dispara una sola vez cuando se cumple `when`.
    #[serde(rename = "step", default)]
    pub steps: Vec<Step>,
}

/// Qué hace un aldeano: ir a un pool, o construir y después ir a `then`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assign {
    pub pool: Option<String>,
    pub build: Option<String>,
    pub then: Option<String>,
    /// Pool junto al que se construye el punto de entrega (por defecto, la
    /// primera fuente del mapa que usa ese edificio).
    pub at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartAssign {
    #[serde(default = "one")]
    pub villagers: u32,
    #[serde(flatten)]
    pub assign: Assign,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VillagerAssign {
    /// Número de aldeano (o primero del rango si hay `to`).
    pub n: u32,
    pub to: Option<u32>,
    #[serde(flatten)]
    pub assign: Assign,
}

/// Condiciones; todas las presentes deben cumplirse.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Condition {
    pub time: Option<u32>,
    pub pop: Option<u32>,
    pub villagers: Option<u32>,
    /// Edad alcanzada (o superior).
    pub age: Option<Age>,
    /// Tecnología terminada (nombre corto de rules.toml).
    pub researched: Option<String>,
    /// Edificio terminado (nombre corto).
    pub built: Option<String>,
    /// Todas las fuentes del pool agotadas.
    pub pool_exhausted: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    #[serde(default)]
    pub when: Condition,
    /// Construir un edificio.
    pub build: Option<String>,
    #[serde(default = "one")]
    pub builders: u32,
    /// Pool del que salen los constructores o los aldeanos a mover.
    pub from: Option<String>,
    /// Pool al que van al terminar; por defecto vuelven a `from`.
    pub then: Option<String>,
    /// Pool junto al que se construye el punto de entrega.
    pub at: Option<String>,
    /// Investigar una tecnología.
    pub research: Option<String>,
    /// Atraer una fuente (id del mapa, p. ej. "boar_2") con un aldeano de `from`.
    pub lure: Option<String>,
    /// Mover aldeanos de `from` a `to` (`count` ausente = todos).
    pub to: Option<String>,
    pub count: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepAction<'a> {
    Build { building: &'a str, builders: u32, from: &'a str, then: &'a str, at: Option<&'a str> },
    Research(&'a str),
    Move { from: &'a str, to: &'a str, count: Option<u32> },
    Lure { source: &'a str, from: &'a str },
}

fn one() -> u32 {
    1
}

impl Step {
    /// Interpreta el paso; error si no tiene exactamente una acción válida.
    pub fn action(&self) -> Result<StepAction<'_>, String> {
        if let Some(source) = &self.lure {
            if self.build.is_some() || self.research.is_some() || self.to.is_some() {
                return Err("cada paso necesita exactamente una acción".into());
            }
            let from = self.from.as_deref().ok_or(format!("lure {source}: falta `from`"))?;
            return Ok(StepAction::Lure { source, from });
        }
        match (&self.build, &self.research, &self.to) {
            (Some(b), None, None) => {
                let from = self.from.as_deref().ok_or(format!("build {b}: falta `from`"))?;
                Ok(StepAction::Build {
                    building: b,
                    builders: self.builders,
                    from,
                    then: self.then.as_deref().unwrap_or(from),
                    at: self.at.as_deref(),
                })
            }
            (None, Some(r), None) => Ok(StepAction::Research(r)),
            (None, None, Some(to)) => Ok(StepAction::Move {
                from: self.from.as_deref().ok_or(format!("move a {to}: falta `from`"))?,
                to,
                count: self.count,
            }),
            _ => Err("cada paso necesita exactamente uno de build, research, to o lure".into()),
        }
    }
}

impl Assign {
    pub fn validate(&self) -> Result<(), String> {
        match (&self.pool, &self.build) {
            (Some(_), None) => Ok(()),
            (None, Some(_)) if self.then.is_some() => Ok(()),
            (None, Some(b)) => Err(format!("build {b}: falta `then`")),
            _ => Err("cada asignación necesita `pool` o `build` + `then`".into()),
        }
    }
}

impl BuildOrder {
    pub fn load(path: &Path) -> Result<BuildOrder, LoadError> {
        let text = std::fs::read_to_string(path).map_err(|e| LoadError::Io(path.to_owned(), e))?;
        toml::from_str(&text).map_err(|e| LoadError::Toml(path.to_owned(), e))
    }

    pub fn parse(text: &str) -> Result<BuildOrder, toml::de::Error> {
        toml::from_str(text)
    }

    /// Asignación del aldeano número `n` (los creados empiezan después de los iniciales).
    pub fn assignment_for(&self, n: u32) -> Option<&Assign> {
        self.villagers
            .iter()
            .find(|v| n >= v.n && n <= v.to.unwrap_or(v.n))
            .map(|v| &v.assign)
    }
}
