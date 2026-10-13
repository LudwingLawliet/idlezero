// Evita la consola extra en Windows en release. NO QUITAR.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Autoprueba sin ventana (CI y diagnóstico): `idlezero --selftest`.
    if std::env::args().any(|a| a == "--selftest") {
        std::process::exit(idlezero_lib::selftest());
    }
    idlezero_lib::run()
}
