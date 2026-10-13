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
}

impl Scenario {
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
        })
    }
}
