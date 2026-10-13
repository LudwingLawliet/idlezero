//! Cancelar un cálculo largo termina pronto con un error claro. Va en su propio
//! archivo: la señal de cancelación es global al proceso.

use aoe2_data::repo_data_dir;
use aoe2_opt::goal::load_presets;
use aoe2_opt::{CANCELLED, OptConfig, optimize, request_cancel, reset_cancel};
use aoe2_sim::{BuildOrder, Scenario};

#[test]
fn a_long_optimization_can_be_cancelled() {
    let empty = BuildOrder::parse("name = \"-\"\nmap = \"arabia_neutra\"\ndefault_pool = \"forest\"").unwrap();
    let base = Scenario::with_order(&repo_data_dir(), "arabia_neutra", empty).unwrap();
    let goal = load_presets(&repo_data_dir()).unwrap().into_iter().find(|g| g.id == "imperial").unwrap();
    reset_cancel();
    let started = std::time::Instant::now();
    let r = std::thread::scope(|s| {
        s.spawn(|| {
            std::thread::sleep(std::time::Duration::from_millis(500));
            request_cancel();
        });
        optimize(&base, &goal, OptConfig::default())
    });
    reset_cancel();
    assert_eq!(r.err().as_deref(), Some(CANCELLED));
    assert!(started.elapsed().as_secs() < 10, "tardó {:?} en cancelar", started.elapsed());
}
