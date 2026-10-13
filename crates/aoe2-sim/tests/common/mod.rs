// Utilidades compartidas por los tests (incluido con `mod common;`).
#![allow(dead_code)]

use aoe2_data::repo_data_dir;
use aoe2_sim::{BuildOrder, Scenario};

pub fn lab(order: &str) -> Scenario {
    let order = BuildOrder::parse(order).expect("orden de prueba válida");
    Scenario::with_order(&repo_data_dir(), "laboratorio", order).expect("cargar escenario")
}

pub fn scenario(map: &str, order: &str) -> Scenario {
    Scenario::load(&repo_data_dir(), map, order).expect("cargar escenario")
}
