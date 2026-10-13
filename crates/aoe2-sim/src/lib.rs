//! Simulador económico determinista.
//!
//! Dado un escenario (datos, mapa, orden de construcción) produce siempre la
//! misma línea de tiempo, segundo a segundo. Ver docs/SPEC.md, "Modelo del
//! simulador".
//!
//! Límites de la Fase 2: no se modela la descomposición de la carne cazada,
//! ni el micro del explorador (las ovejas llegan en segundos fijos del mapa),
//! ni bonos de civilización.

pub mod orders;
pub mod scenario;
pub mod state;
pub mod step;
pub mod timeline;

pub use orders::BuildOrder;
pub use scenario::Scenario;
pub use step::{Sim, Stop};
pub use timeline::{Event, IdleCause, Snapshot, Summary, Timeline, clock};

/// Simula el escenario hasta `stop`.
pub fn simulate(scenario: &Scenario, stop: Stop) -> Result<Timeline, String> {
    Ok(Sim::new(scenario)?.run(stop))
}
