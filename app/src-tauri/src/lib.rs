use aoe2_data::GameData;
use serde::Serialize;

/// Datos del juego embebidos en el binario al compilar.
const GAME_DATA_JSON: &str = include_str!("../../../data/game_data.json");

#[derive(Serialize)]
struct EngineInfo {
    version: &'static str,
    source_commit: String,
    units: usize,
    buildings: usize,
    techs: usize,
}

#[tauri::command]
fn engine_info() -> Result<EngineInfo, String> {
    let data: GameData = serde_json::from_str(GAME_DATA_JSON).map_err(|e| e.to_string())?;
    Ok(EngineInfo {
        version: env!("CARGO_PKG_VERSION"),
        source_commit: data.source.commit,
        units: data.units.len(),
        buildings: data.buildings.len(),
        techs: data.techs.len(),
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // WebKitGTK con GPU NVIDIA en Wayland falla al iniciar ("Error 71 (Protocol
    // error) dispatching to Wayland display"). Desactivar el renderer DMABUF lo
    // evita; el usuario puede anularlo definiendo la variable.
    #[cfg(target_os = "linux")]
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        // SAFETY: se llama al inicio, antes de crear otros hilos.
        unsafe { std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1") };
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![engine_info])
        .run(tauri::generate_context!())
        .expect("error al iniciar la aplicación Tauri");
}

#[cfg(test)]
mod tests {
    #[test]
    fn embedded_game_data_loads() {
        let info = super::engine_info().unwrap();
        assert!(info.units > 0 && info.buildings > 0 && info.techs > 0);
    }
}
