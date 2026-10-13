mod bundle;

use aoe2_data::Age;
use aoe2_opt::{Choice, Goal, OptConfig, OptResult, Optimizer};
use aoe2_sim::calc::{CalcInput, CalcResult};
use aoe2_sim::report::describe;
use aoe2_sim::{BuildOrder, Event, Scenario, Sim, Timeline};
use bundle::{bundle, locale};
use serde::Serialize;
use std::collections::BTreeMap;

/// Civilizaciones mínimas para que una entidad aparezca en el selector.
const MIN_CIVS: u32 = 20;

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
    /// Cantidad total en el mapa (`None` = infinito).
    amount: Option<u32>,
}

#[derive(Serialize)]
struct MapInfo {
    id: String,
    name: String,
    /// Mapa de prueba con recursos infinitos.
    test: bool,
    pools: Vec<PoolInfo>,
}

/// Una entidad elegible como objetivo.
#[derive(Serialize)]
struct Entity {
    /// Lo que entiende el motor: nombre corto de rules.toml o el id.
    key: String,
    id: u32,
    name: String,
    age: Age,
}

#[derive(Serialize)]
struct Catalog {
    units: Vec<Entity>,
    techs: Vec<Entity>,
    buildings: Vec<Entity>,
}

#[derive(Serialize)]
struct Preset {
    id: String,
    name: String,
    goal: Goal,
}

#[derive(Serialize)]
struct AppInfo {
    version: &'static str,
    source_commit: String,
    maps: Vec<MapInfo>,
    presets: Vec<Preset>,
    langs: Vec<Named>,
    /// Todos los textos del idioma ("seccion.clave" -> texto).
    labels: BTreeMap<String, String>,
    catalog: Catalog,
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
                let amount = s.amount.map(|a| a * s.count);
                if let Some(p) = pools.iter_mut().find(|p| p.id == s.pool()) {
                    p.amount = p.amount.zip(amount).map(|(a, b)| a + b);
                    continue;
                }
                let resource = b.rates.task(&s.task).map_or("food".into(), |t| format!("{:?}", t.resource).to_lowercase());
                // Las granjas se siembran: no tienen tope en el mapa.
                let amount = if s.task == "farmer" { None } else { amount };
                pools.push(PoolInfo { id: s.pool().to_owned(), resource, amount });
            }
            MapInfo { id: id.clone(), name: m.name.clone(), test: m.level == 0, pools }
        })
        .collect();
    let presets = b
        .goals
        .iter()
        .map(|g| Preset { id: g.id.clone(), name: l.t(&format!("goals.{}", g.id)), goal: g.clone() })
        .collect();
    let langs = bundle::LANGS
        .iter()
        .map(|(code, _)| Named { id: code.to_string(), name: locale(code).t("meta.name") })
        .collect();

    let key_of = |table: &BTreeMap<String, u32>, id: u32| {
        table.iter().find(|(_, v)| **v == id).map_or(id.to_string(), |(k, _)| k.clone())
    };
    let common = |t: Option<&aoe2_data::TreeInfo>| t.is_some_and(|t| t.civ_count >= MIN_CIVS);
    let units = b
        .data
        .units
        .values()
        .filter(|u| common(u.tree.as_ref()) && u.tree.as_ref().is_some_and(|t| t.produced_at.is_some()))
        .map(|u| Entity {
            key: key_of(&b.rules.names.units, u.id),
            id: u.id,
            name: u.names.get(&lang).to_owned(),
            age: u.tree.as_ref().unwrap().age,
        })
        .collect();
    let techs = b
        .data
        .techs
        .values()
        .filter(|t| common(t.tree.as_ref()) && b.rules.age_of_tech(t.id).is_none())
        .map(|t| Entity {
            key: key_of(&b.rules.names.techs, t.id),
            id: t.id,
            name: t.names.get(&lang).to_owned(),
            age: t.tree.as_ref().unwrap().age,
        })
        .collect();
    let buildings = b
        .data
        .buildings
        .values()
        .filter(|x| common(x.tree.as_ref()))
        .map(|x| {
            let key = key_of(&b.rules.names.buildings, x.id);
            let sc = bundle::scenario(&b.maps[0].0).ok();
            let age = sc.map_or(x.tree.as_ref().unwrap().age, |sc| sc.building_age(x.id));
            Entity { key, id: x.id, name: x.names.get(&lang).to_owned(), age }
        })
        .collect();
    AppInfo {
        version: env!("CARGO_PKG_VERSION"),
        source_commit: b.data.source.commit.clone(),
        maps,
        presets,
        langs,
        labels: l.all(),
        catalog: Catalog { units, techs, buildings },
    }
}

#[derive(Serialize)]
struct TextEvent {
    time: u32,
    text: String,
    /// Tipo de evento (para íconos y filtros en la interfaz).
    kind: String,
}

fn text_events(sc: &Scenario, lang: &str, tl: &Timeline) -> Vec<TextEvent> {
    let l = locale(lang);
    tl.events
        .iter()
        .filter(|e| !matches!(e.event, Event::Decision { .. } | Event::TaskChanged { .. } | Event::Spent { .. }))
        .filter_map(|e| {
            let kind = serde_json::to_value(&e.event).ok()?.get("type")?.as_str()?.to_owned();
            describe(sc, &l, &e.event).map(|text| TextEvent { time: e.time, text, kind })
        })
        .collect()
}

#[derive(Serialize)]
struct OptimizeView {
    result: OptResult,
    events: Vec<TextEvent>,
    millis: u128,
}

/// Corre trabajo pesado fuera del hilo de la interfaz.
async fn blocking<T: Send + 'static>(f: impl FnOnce() -> Result<T, String> + Send + 'static) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn optimize(map: String, goal: Goal, lang: String) -> Result<OptimizeView, String> {
    blocking(move || {
        let sc = bundle::scenario(&map)?;
        let started = std::time::Instant::now();
        let result = aoe2_opt::optimize(&sc, &goal, OptConfig::default())?;
        let millis = started.elapsed().as_millis();
        let events = text_events(&sc, &lang, &result.timeline);
        Ok(OptimizeView { result, events, millis })
    })
    .await
}

#[derive(Serialize)]
struct PlanView {
    time: Option<u32>,
    timeline: Timeline,
    events: Vec<TextEvent>,
}

/// Simula otro camino completo (para compararlo con el mejor).
#[tauri::command]
async fn simulate_plan(map: String, goal: Goal, choices: Vec<Choice>, lang: String) -> Result<PlanView, String> {
    blocking(move || {
        let sc = bundle::scenario(&map)?;
        let opt = Optimizer::new(&sc, &goal, OptConfig::default())?;
        let (time, timeline) = opt.plan_timeline(&choices);
        let events = text_events(&sc, &lang, &timeline);
        Ok(PlanView { time, timeline, events })
    })
    .await
}

#[derive(Serialize)]
struct WhatIf {
    builders: u32,
    /// Segundos de obra (desde que se coloca hasta que se termina).
    build_seconds: Option<u32>,
    /// Segundo en que se cumple el objetivo con esos constructores.
    goal_time: Option<u32>,
}

/// "¿Y si construyo este edificio con N aldeanos?": re-simula la orden con
/// 1 a 5 constructores en el edificio nº `placement`.
#[tauri::command]
async fn builders_what_if(map: String, goal: Goal, order_toml: String, placement: usize) -> Result<Vec<WhatIf>, String> {
    blocking(move || {
        let base = bundle::scenario(&map)?;
        let order = BuildOrder::parse(&order_toml).map_err(|e| e.to_string())?;
        let sc = Scenario { order, ..base };
        let mut out = Vec::new();
        for n in 1..=5 {
            let mut sim = Sim::new(&sc)?;
            sim.set_builder_overrides(BTreeMap::from([(placement, n)]));
            let mut started = None;
            sim.advance(3600, &|s| goal.is_done(&sc, s));
            if let Some(site) = sim.state.sites.get(placement) {
                started = site.finished;
            }
            let tl_done = goal.is_done(&sc, &sim.state).then_some(sim.state.time);
            let tl = sim.into_timeline();
            let placed = tl.events.iter().find_map(|e| match e.event {
                Event::BuildStarted { placement: p, .. } if p == placement => Some(e.time),
                _ => None,
            });
            let build_seconds = placed.zip(started).map(|(a, b)| b - a);
            out.push(WhatIf { builders: n, build_seconds, goal_time: tl_done });
        }
        Ok(out)
    })
    .await
}

#[tauri::command]
fn calculate(map: String, input: CalcInput) -> Result<CalcResult, String> {
    let sc = bundle::scenario(&map)?;
    aoe2_sim::calc::calculate(&sc, &input)
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
        .invoke_handler(tauri::generate_handler![app_info, optimize, simulate_plan, builders_what_if, calculate])
        .run(tauri::generate_context!())
        .expect("error al iniciar la aplicación Tauri");
}

#[cfg(test)]
mod tests {
    #[test]
    fn embedded_data_loads_and_builds_catalog() {
        let info = super::app_info("es".into());
        assert!(!info.maps.is_empty() && !info.presets.is_empty());
        assert_eq!(info.labels.get("pools.forest").map(String::as_str), Some("madera"));
        let archer = info.catalog.units.iter().find(|u| u.id == 4).expect("arquero en el catálogo");
        assert_eq!(archer.key, "archer");
        assert_eq!(archer.name, "Arquero");
        let tc = info.catalog.buildings.iter().find(|b| b.id == 109).unwrap();
        assert_eq!(tc.age, aoe2_data::Age::Castle, "CU adicional desde Castillos");
        assert!(info.maps.iter().any(|m| m.test), "el laboratorio se marca como prueba");
    }

    #[test]
    fn builders_what_if_more_builders_faster() {
        let order = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/build_orders/fast_castle_27_2.toml")).unwrap();
        let goal = aoe2_opt::Goal::age(aoe2_data::Age::Feudal);
        // Edificio nº 2: el campamento maderero del aldeano 7.
        let r = tauri::async_runtime::block_on(super::builders_what_if("arabia_neutra".into(), goal, order, 2)).unwrap();
        let secs: Vec<u32> = r.iter().map(|w| w.build_seconds.unwrap()).collect();
        assert!(secs.windows(2).all(|w| w[1] <= w[0]), "{secs:?}");
        assert!(secs[0] > secs[4]);
    }
}

#[cfg(test)]
mod custom_goal {
    #[test]
    fn free_goal_by_id_with_unit_upgrade() {
        // 10 ballesteros (id 24): no está en las plantillas y exige investigar "Crossbow".
        let goal: aoe2_opt::Goal =
            serde_json::from_str(r#"{"id":"custom","units":[{"unit":"24","count":10}],"techs":[],"buildings":[],"allow_tc_idle":false}"#).unwrap();
        let v = tauri::async_runtime::block_on(super::optimize("arabia_neutra".into(), goal, "es".into())).unwrap();
        assert!(v.result.best.time.is_some(), "sin plan para 10 ballesteros");
        let tl = &v.result.timeline;
        let trained = tl.events.iter().filter(|e| matches!(e.event, aoe2_sim::Event::UnitTrained { .. })).count();
        assert_eq!(trained, 10);
        let upgrade_done = tl.events.iter().find_map(|e| match &e.event {
            aoe2_sim::Event::ResearchFinished { tech } if tech == "100" => Some(e.time),
            _ => None,
        });
        let first_unit = tl.events.iter().find_map(|e| matches!(e.event, aoe2_sim::Event::UnitTrained { .. }).then_some(e.time));
        assert!(upgrade_done.is_some(), "falta la mejora Ballestero");
        assert!(first_unit.unwrap() > upgrade_done.unwrap());
        println!("10 ballesteros: {:?}", v.result.best.time);
    }
}
