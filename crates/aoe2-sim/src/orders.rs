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
    /// Reglas automáticas (casas, puntos de entrega, edificios de edad…).
    #[serde(default)]
    pub auto: Auto,
}

/// Reglas que el simulador aplica solo, sin pasos explícitos. Las usa el
/// optimizador para decidir únicamente lo importante (qué hace cada aldeano y
/// cuándo subir de edad), y simplifican las órdenes escritas a mano.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Auto {
    /// Pool del que salen los constructores automáticos.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub builders: Option<String>,
    /// Empezar una casa cuando falten estas plazas o menos.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub houses: Option<u32>,
    /// El primer aldeano de un pool lejano construye antes su punto de entrega.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub drop_sites: bool,
    /// [agotado, siguiente]: al agotarse un pool sus aldeanos pasan al siguiente.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fallback: Vec<[String; 2]>,
    /// Construir los edificios que piden las edades hasta esta.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub age_buildings: Option<Age>,
    /// Tecnologías a investigar en cuanto se pueda (construye su edificio).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub research: Vec<String>,
    /// Atraer la siguiente fuente de un pool (el 2.º jabalí) cuando a la
    /// actual le quede esta comida o menos.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lure_ahead: Option<u32>,
    /// Producción continua de una unidad.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub production: Option<Production>,
    /// Edificios a construir en cuanto la edad y los recursos lo permitan
    /// (los que pide el objetivo, p. ej. un castillo).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub buildings: Vec<BuildingTarget>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BuildingTarget {
    pub building: String,
    pub count: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Production {
    pub unit: String,
    pub count: u32,
    /// Edificios productores a mantener.
    pub buildings: u32,
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
    /// Constructores en total (este aldeano + ayudantes del pool de constructores).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub builders: Option<u32>,
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
    /// Con `research` en el CU: seguir creando aldeanos hasta poder
    /// investigar (en vez de parar el CU y esperar).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub soft: bool,
    /// Entrenar `count` unidades (nombre corto de rules.toml).
    pub train: Option<String>,
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
    Train { unit: &'a str, count: u32 },
}

fn one() -> u32 {
    1
}

impl Step {
    /// Interpreta el paso; error si no tiene exactamente una acción válida.
    pub fn action(&self) -> Result<StepAction<'_>, String> {
        if let Some(unit) = &self.train {
            if self.build.is_some() || self.research.is_some() || self.to.is_some() || self.lure.is_some() {
                return Err("cada paso necesita exactamente una acción".into());
            }
            return Ok(StepAction::Train { unit, count: self.count.unwrap_or(1) });
        }
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
