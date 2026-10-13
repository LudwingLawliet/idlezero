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
///
/// Con civilización, primero se optimiza el escenario neutro: su mejor plan
/// entra como candidato (una civilización con bonos a favor no puede salir
/// peor por azar de la búsqueda) y su tiempo se informa en `neutral_time`.
pub fn optimize(base: &Scenario, goal: &Goal, cfg: OptConfig) -> Result<OptResult, String> {
    let mut seeds = Vec::new();
    let mut neutral_time = None;
    if base.civ.is_some() {
        let neutral = Scenario { civ: None, ..base.clone() };
        if goal.validate(&neutral).is_ok() {
            if let Ok(n) = optimize(&neutral, goal, cfg.clone()) {
                neutral_time = n.best.time;
                seeds.push(n.best.choices);
            }
        }
    }
    // Cota práctica: si no hay plan con el CU siempre activo, no vale la pena
    // explorar planes mucho más lentos que uno rápido admitiendo esperas.
    let relaxed_hint = Optimizer::new(base, goal, cfg.clone())?.relax().quick_time().map(|t| t + t / 4);
    let strict = Optimizer::new(base, goal, cfg.clone())?.with_seeds(seeds.clone()).with_bound(relaxed_hint).run();
    let mut result = if strict.best.time.is_some() {
        strict
    } else {
        Optimizer::new(base, goal, cfg)?.relax().with_seeds(seeds).run()
    };
    result.neutral_time = neutral_time;
    Ok(result)
}
