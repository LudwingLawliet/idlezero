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

/// Optimiza con la regla del CU siempre activo; si no hay ningún plan así,
/// repite admitiendo CU inactivo y lo marca en `OptResult::relaxed`.
pub fn optimize(base: &Scenario, goal: &Goal, cfg: OptConfig) -> Result<OptResult, String> {
    let strict = Optimizer::new(base, goal, cfg.clone())?.run();
    if strict.best.time.is_some() {
        return Ok(strict);
    }
    Ok(Optimizer::new(base, goal, cfg)?.relax().run())
}
