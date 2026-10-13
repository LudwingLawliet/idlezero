mod bundle;

use aoe2_opt::{OptConfig, OptResult};
use aoe2_sim::report::describe;
use bundle::{bundle, locale};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Serialize)]
struct Named {
    id: String,
    name: String,
}

#[derive(Serialize)]
struct PoolInfo {
    id: String,
    /// food, wood, gold o stone (para el ícono).
    resource: String,
}

#[derive(Serialize)]
struct MapInfo {
    id: String,
    name: String,
    pools: Vec<PoolInfo>,
}

#[derive(Serialize)]
struct AppInfo {
    version: &'static str,
    source_commit: String,
    maps: Vec<MapInfo>,
    goals: Vec<Named>,
    langs: Vec<Named>,
    /// Todos los textos del idioma ("seccion.clave" -> texto).
    labels: BTreeMap<String, String>,
    /// Nombres del juego traducidos, por nombre corto de rules.toml.
    buildings: BTreeMap<String, String>,
    techs: BTreeMap<String, String>,
    units: BTreeMap<String, String>,
}

#[tauri::command]
fn app_info(lang: String) -> AppInfo {
    let b = bundle();
    let l = locale(&lang);
    let maps = b
        .maps
        .iter()
        .map(|(id, m)| {
            let mut pools: Vec<PoolInfo> = Vec::new();
            for s in &m.sources {
                if pools.iter().any(|p| p.id == s.pool()) {
                    continue;
                }
                let resource = b.rates.task(&s.task).map_or("food".into(), |t| format!("{:?}", t.resource).to_lowercase());
                pools.push(PoolInfo { id: s.pool().to_owned(), resource });
            }
            MapInfo { id: id.clone(), name: m.name.clone(), pools }
        })
        .collect();
    let goals = b.goals.iter().map(|g| Named { id: g.id.clone(), name: l.t(&format!("goals.{}", g.id)) }).collect();
    let langs = bundle::LANGS
        .iter()
        .map(|(code, _)| Named { id: code.to_string(), name: locale(code).t("meta.name") })
        .collect();
    let names = |table: &BTreeMap<String, u32>, f: &dyn Fn(u32) -> String| -> BTreeMap<String, String> {
        table.iter().map(|(k, id)| (k.clone(), f(*id))).collect()
    };
    AppInfo {
        version: env!("CARGO_PKG_VERSION"),
        source_commit: b.data.source.commit.clone(),
        maps,
        goals,
        langs,
        labels: l.all(),
        buildings: names(&b.rules.names.buildings, &|id| l.building(&b.data, id)),
        techs: names(&b.rules.names.techs, &|id| l.tech(&b.data, id)),
        units: names(&b.rules.names.units, &|id| l.unit(&b.data, id)),
    }
}

#[derive(Serialize)]
struct TextEvent {
    time: u32,
    text: String,
    /// Tipo de evento (para íconos y filtros en la interfaz).
    kind: String,
}

#[derive(Serialize)]
struct OptimizeView {
    result: OptResult,
    events: Vec<TextEvent>,
    millis: u128,
}

#[tauri::command]
async fn optimize(map: String, goal: String, lang: String) -> Result<OptimizeView, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let b = bundle();
        let sc = bundle::scenario(&map)?;
        let g = b.goals.iter().find(|g| g.id == goal).ok_or(format!("objetivo desconocido: {goal}"))?;
        let started = std::time::Instant::now();
        let result = aoe2_opt::optimize(&sc, g, OptConfig::default())?;
        let millis = started.elapsed().as_millis();
        let l = locale(&lang);
        let events = result
            .timeline
            .events
            .iter()
            .filter(|e| !matches!(e.event, aoe2_sim::Event::Decision { .. }))
            .filter_map(|e| {
                let kind = serde_json::to_value(&e.event).ok()?.get("type")?.as_str()?.to_owned();
                describe(&sc, &l, &e.event).map(|text| TextEvent { time: e.time, text, kind })
            })
            .collect();
        Ok(OptimizeView { result, events, millis })
    })
    .await
    .map_err(|e| e.to_string())?
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
        .invoke_handler(tauri::generate_handler![app_info, optimize])
        .run(tauri::generate_context!())
        .expect("error al iniciar la aplicación Tauri");
}

#[cfg(test)]
mod tests {
    #[test]
    fn embedded_data_loads_and_lists_goals() {
        let info = super::app_info("es".into());
        assert!(!info.maps.is_empty() && !info.goals.is_empty());
        assert_eq!(info.labels.get("pools.forest").map(String::as_str), Some("madera"));
        assert_eq!(info.buildings.get("house").map(String::as_str), Some("Casa"));
    }
}
