//! Optimizador: busca la secuencia de decisiones que cumple un objetivo en el
//! menor tiempo simulado. Ver docs/SPEC.md, "Optimizador y árbol de decisiones".
//!
//! Decide qué hace cada aldeano al nacer, cuándo hacer clic de edad y, para
//! objetivos de unidades, cuándo dejar de crear aldeanos. Lo demás (casas,
//! puntos de entrega, edificios de edad, granjas) lo hacen las reglas
//! automáticas del simulador. No promete el óptimo matemático: promete una
//! solución reproducible y comparable.

pub mod goal;
pub mod search;

pub use goal::Goal;
pub use search::{Choice, OptConfig, OptResult, Optimizer, Plan, TreeNode};

use aoe2_sim::Scenario;

pub fn optimize(base: &Scenario, goal: &Goal, cfg: OptConfig) -> Result<OptResult, String> {
    Ok(Optimizer::new(base, goal, cfg)?.run())
}
