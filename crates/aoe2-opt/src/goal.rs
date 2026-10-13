//! Objetivos: condiciones sobre el estado del juego, combinables.

use aoe2_data::{Age, Cost, LoadError};
use aoe2_sim::Scenario;
use aoe2_sim::state::GameState;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Edificios productores que mantiene el optimizador para objetivos de unidades.
pub const PRODUCTION_BUILDINGS: u32 = 2;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Goal {
    #[serde(default)]
    pub id: String,
    pub age: Option<Age>,
    pub villagers: Option<u32>,
    #[serde(default)]
    pub units: Vec<UnitGoal>,
    /// Tecnologías (nombres cortos de rules.toml).
    #[serde(default)]
    pub techs: Vec<String>,
    #[serde(default)]
    pub buildings: Vec<BuildingGoal>,
    /// Permitir que el CU quede inactivo (p. ej. clic de edad esperando
    /// recursos). Por defecto no: el CU nunca para, salvo al dejar de crear
    /// aldeanos a propósito.
    #[serde(default)]
    pub allow_tc_idle: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnitGoal {
    pub unit: String,
    pub count: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BuildingGoal {
    pub building: String,
    pub count: u32,
}

#[derive(Deserialize)]
struct GoalsFile {
    goal: Vec<Goal>,
}

pub fn load_presets(data_dir: &Path) -> Result<Vec<Goal>, LoadError> {
    let path = data_dir.join("goals.toml");
    let text = std::fs::read_to_string(&path).map_err(|e| LoadError::Io(path.clone(), e))?;
    parse_presets(&text).map_err(|e| LoadError::Toml(path, e))
}

pub fn parse_presets(text: &str) -> Result<Vec<Goal>, toml::de::Error> {
    Ok(toml::from_str::<GoalsFile>(text)?.goal)
}

impl Goal {
    pub fn age(age: Age) -> Goal {
        Goal { id: format!("{age:?}").to_lowercase(), age: Some(age), ..Goal::default() }
    }

    /// Valida los nombres contra las reglas.
    pub fn validate(&self, sc: &Scenario) -> Result<(), String> {
        let civ = sc.civ.as_ref().map_or(String::new(), |c| c.info.names.es.clone());
        let missing = |what: &str, name: &str| format!("{civ} no tiene {what} {name}");
        for u in &self.units {
            let id = sc.unit_ref(&u.unit).ok_or(format!("unidad desconocida: {}", u.unit))?;
            if !sc.civ_has_unit(id) {
                return Err(missing("la unidad", &sc.data.unit(id).unwrap().names.es));
            }
        }
        for t in &self.techs {
            let id = sc.tech_ref(t).ok_or(format!("tecnología desconocida: {t}"))?;
            if !sc.civ_has_tech(id) {
                return Err(missing("la tecnología", &sc.data.tech(id).unwrap().names.es));
            }
        }
        for b in &self.buildings {
            let id = sc.building_ref(&b.building).ok_or(format!("edificio desconocido: {}", b.building))?;
            if !sc.civ_has_building(id) {
                return Err(missing("el edificio", &sc.data.building(id).unwrap().names.es));
            }
        }
        Ok(())
    }

    pub fn is_done(&self, sc: &Scenario, s: &GameState) -> bool {
        // (los requisitos implícitos, como la mejora de Ballestero, ya los exige
        // el entrenamiento; aquí solo cuenta lo pedido)
        self.age.is_none_or(|a| s.age >= a)
            && self.villagers.is_none_or(|n| s.villagers.len() as u32 >= n)
            && self.units.iter().all(|u| s.units_of(sc.unit_ref(&u.unit).unwrap()) >= u.count)
            && self.techs.iter().all(|t| s.has_tech(sc.tech_ref(t).unwrap()))
            && self
                .buildings
                .iter()
                .all(|b| s.count_built(sc.building_ref(&b.building).unwrap()) as u32 >= b.count)
    }

    /// Edad mínima que exige el objetivo (la pedida o la de sus unidades,
    /// tecnologías y edificios).
    pub fn target_age(&self, sc: &Scenario) -> Age {
        let mut age = self.age.unwrap_or(Age::Dark);
        let tree_age = |t: Option<&aoe2_data::TreeInfo>| t.map_or(Age::Dark, |t| t.age);
        for u in &self.units {
            let id = sc.unit_ref(&u.unit).unwrap();
            age = age.max(tree_age(sc.data.unit(id).and_then(|x| x.tree.as_ref())));
        }
        for t in &self.techs {
            let id = sc.tech_ref(t).unwrap();
            age = age.max(tree_age(sc.data.tech(id).and_then(|x| x.tree.as_ref())));
        }
        for b in &self.buildings {
            let id = sc.building_ref(&b.building).unwrap();
            age = age.max(sc.building_age(id));
        }
        for u in &self.units {
            let id = sc.unit_ref(&u.unit).unwrap();
            if let Some(&up) = sc.data.unit_upgrades.get(&id) {
                age = age.max(tree_age(sc.data.tech(up).and_then(|x| x.tree.as_ref())));
            }
        }
        age
    }

    /// Lo que hace falta para la próxima etapa, como lo piensa un jugador:
    /// si falta subir de edad, solo la siguiente edad (y sus edificios); ya en
    /// la edad del objetivo, todo lo que falta. Así no se manda gente al oro
    /// en el minuto 0 para unidades que no se pueden hacer hasta Feudal.
    pub fn stage_cost(&self, sc: &Scenario, s: &GameState) -> Cost {
        let target = self.target_age(sc);
        if s.age >= target {
            return self.remaining_cost(sc, s);
        }
        let next = match s.age {
            Age::Dark => Age::Feudal,
            Age::Feudal => Age::Castle,
            _ => Age::Imperial,
        };
        let stage = Goal { age: Some(next), ..Goal::default() };
        let mut c = stage.remaining_cost(sc, s);
        // Más lo que el objetivo pide en la edad actual (p. ej. Telar).
        let now = Goal {
            techs: self
                .techs
                .iter()
                .filter(|t| {
                    sc.tech_ref(t).and_then(|id| sc.data.tech(id)).and_then(|x| x.tree.as_ref()).is_some_and(|tr| tr.age <= s.age)
                })
                .cloned()
                .collect(),
            ..Goal::default()
        };
        let extra = now.remaining_cost(sc, s);
        c.food += extra.food;
        c.wood += extra.wood;
        c.gold += extra.gold;
        c.stone += extra.stone;
        c
    }

    /// Costo de lo que falta para cumplir el objetivo desde `s` (sin contar
    /// lo ya pagado: investigaciones en curso, unidades entrenándose).
    pub fn remaining_cost(&self, sc: &Scenario, s: &GameState) -> Cost {
        let mut c = Cost::default();
        let mut add = |x: &Cost, n: u32| {
            c.food += x.food * n;
            c.wood += x.wood * n;
            c.gold += x.gold * n;
            c.stone += x.stone * n;
        };
        let target = self.target_age(sc);
        for age in [Age::Feudal, Age::Castle, Age::Imperial] {
            if age > target || s.age >= age {
                continue;
            }
            let tech = sc.rules.age_tech(age).unwrap();
            let started = s.tc_researching(tech);
            if !started {
                add(&sc.data.tech(tech).unwrap().cost, 1);
            }
            // Edificios que pide esa edad y aún no están.
            if let Some(req) = sc.rules.age_requirement(tech) {
                let have = req.buildings.iter().filter(|b| s.count_planned(**b) > 0).count() as u32;
                let mut missing: Vec<&Cost> = req
                    .buildings
                    .iter()
                    .filter(|b| s.count_planned(**b) == 0)
                    .filter_map(|b| sc.data.building(*b).map(|x| &x.cost))
                    .collect();
                missing.sort_by_key(|c| c.food + c.wood + c.gold + c.stone);
                for m in missing.into_iter().take(req.count.saturating_sub(have) as usize) {
                    add(m, 1);
                }
            }
        }
        // Edificios productores (los que mantiene auto.production) y sus requisitos.
        for u in self.units.iter().take(1) {
            let id = sc.unit_ref(&u.unit).unwrap();
            let mut b = sc.data.unit(id).and_then(|x| x.tree.as_ref()).and_then(|t| t.produced_at);
            let mut wanted = PRODUCTION_BUILDINGS;
            while let Some(bid) = b {
                let missing = wanted.saturating_sub(s.count_planned(bid) as u32);
                add(&sc.data.building(bid).unwrap().cost, missing);
                b = sc.data.building(bid).and_then(|x| x.tree.as_ref()).and_then(|t| t.requires_building);
                wanted = 1;
            }
        }
        for u in &self.units {
            let id = sc.unit_ref(&u.unit).unwrap();
            let in_training = s.training.iter().filter(|r| r.job == aoe2_sim::state::Job::Unit(id)).count() as u32;
            let left = u.count.saturating_sub(s.units_of(id) + in_training);
            add(&sc.data.unit(id).unwrap().cost, left);
        }
        // Tecnologías pedidas, sus previas y las mejoras de las unidades.
        let mut techs: Vec<u32> = Vec::new();
        for t in &self.techs {
            techs.extend(sc.tech_chain(sc.tech_ref(t).unwrap()));
        }
        for u in &self.units {
            if let Some(&up) = sc.data.unit_upgrades.get(&sc.unit_ref(&u.unit).unwrap()) {
                techs.extend(sc.tech_chain(up));
            }
        }
        techs.sort();
        techs.dedup();
        for id in techs {
            if !s.has_tech(id) && !s.research.iter().any(|r| r.job == aoe2_sim::state::Job::Research(id)) {
                add(&sc.data.tech(id).unwrap().cost, 1);
            }
        }
        for b in &self.buildings {
            let id = sc.building_ref(&b.building).unwrap();
            let left = b.count.saturating_sub(s.count_planned(id) as u32);
            add(&sc.data.building(id).unwrap().cost, left);
        }
        c
    }
}
