// Evita la consola extra en Windows en release. NO QUITAR.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    aoe2_optimizer_lib::run()
}
