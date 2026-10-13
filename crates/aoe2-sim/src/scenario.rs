//! Todo lo que el simulador necesita para correr: datos del juego, tasas,
//! reglas, modificadores, mapa y orden de construcción.

use crate::orders::BuildOrder;
use aoe2_data::{GameData, GatherRates, LoadError, MapDef, Modifiers, Rules};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Scenario {
    pub data: GameData,
    pub rates: GatherRates,
    pub rules: Rules,
    pub modifiers: Modifiers,
    pub map: MapDef,
    pub order: BuildOrder,
    /// Civilización (árbol + bonos). `None` = escenario neutro.
    pub civ: Option<aoe2_data::Civ>,
}

impl Scenario {
    /// Con la civilización `id` (árbol y bonos de data/civs.json y civ_bonuses.toml).
    pub fn with_civ(mut self, data_dir: &Path, id: &str) -> Result<Scenario, LoadError> {
        self.civ = aoe2_data::Civ::load(data_dir, id)?;
        Ok(self)
    }

    /// Bonos de la civilización (vacío en el escenario neutro).
    pub fn bonuses(&self) -> impl Iterator<Item = &aoe2_data::civs::BonusKind> {
        self.civ.iter().flat_map(|c| c.bonuses.iter().map(|b| &b.kind))
    }

    /// ¿La civilización tiene esta unidad / tecnología / edificio? (neutro: todo)
    pub fn civ_has_unit(&self, id: u32) -> bool {
        self.civ.as_ref().is_none_or(|c| c.has_unit(id))
    }

    pub fn civ_has_tech(&self, id: u32) -> bool {
        self.civ.as_ref().is_none_or(|c| c.has_tech(id) || self.data.unit_upgrades.values().any(|t| *t == id))
    }

    pub fn civ_has_building(&self, id: u32) -> bool {
        self.civ.as_ref().is_none_or(|c| c.has_building(id))
    }

    /// Unidad por nombre corto de rules.toml o por id ("archer" o "4").
    pub fn unit_ref(&self, s: &str) -> Option<u32> {
        self.rules.unit_id(s).or_else(|| s.parse().ok().filter(|id| self.data.unit(*id).is_some()))
    }

    pub fn tech_ref(&self, s: &str) -> Option<u32> {
        self.rules.tech_id(s).or_else(|| s.parse().ok().filter(|id| self.data.tech(*id).is_some()))
    }

    pub fn building_ref(&self, s: &str) -> Option<u32> {
        self.rules.building_id(s).or_else(|| s.parse().ok().filter(|id| self.data.building(*id).is_some()))
    }

    /// Edad mínima para construir (árbol, corregido por rules.building_age).
    pub fn building_age(&self, b: u32) -> aoe2_data::Age {
        let tree = self.data.building(b).and_then(|x| x.tree.as_ref()).map_or(aoe2_data::Age::Dark, |t| t.age);
        self.rules.building_age_override(b).map_or(tree, |o| o.max(tree))
    }

    /// Tecnología que exige `tech` antes: la del árbol de aoe2techtree o, si
    /// no la trae, la de rules.toml.
    pub fn tech_prereq(&self, tech: u32) -> Option<u32> {
        self.data.tech_previous.get(&tech).copied().or_else(|| self.rules.tech_prerequisite(tech))
    }

    /// Tecnologías previas que faltan para `tech`, de la más antigua a la más nueva.
    pub fn tech_chain(&self, tech: u32) -> Vec<u32> {
        let mut chain = vec![tech];
        let mut t = tech;
        while let Some(prev) = self.tech_prereq(t) {
            if chain.contains(&prev) {
                break;
            }
            chain.push(prev);
            t = prev;
        }
        chain.reverse();
        chain
    }

    /// Carga `data_dir/maps/<map>.toml` y `data_dir/build_orders/<order>.toml`.
    pub fn load(data_dir: &Path, map: &str, order: &str) -> Result<Scenario, LoadError> {
        let order = BuildOrder::load(&data_dir.join("build_orders").join(format!("{order}.toml")))?;
        Scenario::with_order(data_dir, map, order)
    }

    /// Igual que `load`, con una orden ya construida (tests, optimizador).
    pub fn with_order(data_dir: &Path, map: &str, order: BuildOrder) -> Result<Scenario, LoadError> {
        Ok(Scenario {
            data: GameData::load(&data_dir.join("game_data.json"))?,
            rates: GatherRates::load(&data_dir.join("gather_rates.toml"))?,
            rules: Rules::load(&data_dir.join("rules.toml"))?,
            modifiers: Modifiers::load(&data_dir.join("modifiers.toml"))?,
            map: MapDef::load(&data_dir.join("maps").join(format!("{map}.toml")))?,
            order,
            civ: None,
        })
    }
}
