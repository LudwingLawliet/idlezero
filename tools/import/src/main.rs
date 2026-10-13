//! Importa vendor/aoe2techtree a data/game_data.json.
//!
//! Uso: `cargo run -p import [-- <ruta aoe2techtree> <salida>]`
//! Por defecto lee `vendor/aoe2techtree` y escribe `data/game_data.json`.
//! La salida es determinista: mismos datos de entrada = mismo archivo.

use aoe2_data::{
    Age, Building, Cost, GameData, Names, SCHEMA_VERSION, SourceInfo, Tech, TreeInfo, Unit,
};
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};
use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::Command;

type Res<T> = Result<T, Box<dyn Error>>;

const REPOSITORY: &str = "https://github.com/SiegeEngineers/aoe2techtree";

#[derive(Deserialize)]
struct RawData {
    data: RawSections,
}

#[derive(Deserialize)]
struct RawSections {
    #[serde(rename = "Unit")]
    units: HashMap<String, RawEntity>,
    #[serde(rename = "Building")]
    buildings: HashMap<String, RawEntity>,
    #[serde(rename = "Tech")]
    techs: HashMap<String, RawEntity>,
    unit_upgrades: HashMap<String, RawEntity>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RawEntity {
    #[serde(rename = "ID")]
    id: u32,
    #[serde(rename = "internal_name")]
    internal_name: String,
    #[serde(default)]
    cost: HashMap<String, u32>,
    train_time: Option<u32>,
    research_time: Option<u32>,
    #[serde(rename = "HP")]
    hp: Option<i32>,
    speed: Option<f64>,
    repeatable: Option<bool>,
}

#[derive(Deserialize)]
struct RawTree {
    buildings: Vec<RawNode>,
    units_techs: Vec<RawNode>,
}

#[derive(Deserialize)]
struct RawNode {
    picture_index: u32,
    age_id: u32,
    building_id: u32,
    link_id: Option<u32>,
    link_node_type: Option<String>,
    name_string_id: u32,
    node_id: u32,
    node_status: String,
    use_type: String,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum Kind {
    Unit,
    Building,
    Tech,
}

/// Lo que cada civilización dice de un nodo; se resuelve por moda.
#[derive(Default)]
struct NodeVotes {
    ages: BTreeMap<u32, u32>,
    produced_at: BTreeMap<u32, u32>,
    requires: BTreeMap<Option<u32>, u32>,
    name_string_id: Option<u32>,
    picture_index: Option<u32>,
    civs: u32,
}

/// El valor más votado; en empate gana el menor, para que sea determinista.
fn mode<K: Ord + Clone>(votes: &BTreeMap<K, u32>) -> Option<K> {
    let max = *votes.values().max()?;
    votes.iter().find(|(_, n)| **n == max).map(|(k, _)| k.clone())
}

fn main() -> Res<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut args = std::env::args().skip(1);
    let vendor = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("vendor/aoe2techtree"));
    let out = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("data/game_data.json"));

    let (data, votes) = import(&vendor)?;
    copy_icons(&vendor, &root, &data, &votes)?;
    import_civs(&vendor, &root)?;
    let mut json = serde_json::to_string_pretty(&data)?;
    json.push('\n');
    std::fs::write(&out, json)?;
    eprintln!(
        "{} unidades, {} edificios, {} tecnologías -> {}",
        data.units.len(),
        data.buildings.len(),
        data.techs.len(),
        out.display()
    );
    Ok(())
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Res<T> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?)
}

fn import(vendor: &Path) -> Res<(GameData, HashMap<(Kind, u32), NodeVotes>)> {
    let raw: RawData = read_json(&vendor.join("data/data.json"))?;
    let en: HashMap<String, String> = read_json(&vendor.join("data/locales/en/strings.json"))?;
    let es: HashMap<String, String> = read_json(&vendor.join("data/locales/es/strings.json"))?;
    let votes = collect_tree_votes(&vendor.join("data/trees"))?;

    let names = |kind: Kind, id: u32, internal: &str| -> Names {
        let lookup = |table: &HashMap<String, String>| {
            votes
                .get(&(kind, id))
                .and_then(|v| v.name_string_id)
                .and_then(|sid| table.get(&sid.to_string()))
                .map(|s| clean_name(s))
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| internal.to_owned())
        };
        Names { en: lookup(&en), es: lookup(&es) }
    };
    let tree = |kind: Kind, id: u32| -> Res<Option<TreeInfo>> {
        let Some(v) = votes.get(&(kind, id)) else { return Ok(None) };
        let age_id = mode(&v.ages).ok_or("nodo sin edad")?;
        let age = Age::from_techtree_id(age_id).ok_or(format!("age_id desconocido {age_id}"))?;
        let produced_at = match kind {
            Kind::Building => None,
            _ => mode(&v.produced_at),
        };
        Ok(Some(TreeInfo {
            age,
            produced_at,
            requires_building: mode(&v.requires).flatten(),
            civ_count: v.civs,
        }))
    };

    let mut units = BTreeMap::new();
    for e in raw.data.units.values() {
        units.insert(
            e.id,
            Unit {
                id: e.id,
                internal_name: e.internal_name.clone(),
                names: names(Kind::Unit, e.id, &e.internal_name),
                cost: cost(&e.cost)?,
                train_time: e.train_time.ok_or(format!("unidad {} sin TrainTime", e.id))?,
                hp: e.hp.unwrap_or(0),
                speed: e.speed.unwrap_or(0.0),
                tree: tree(Kind::Unit, e.id)?,
            },
        );
    }
    let mut buildings = BTreeMap::new();
    for e in raw.data.buildings.values() {
        buildings.insert(
            e.id,
            Building {
                id: e.id,
                internal_name: e.internal_name.clone(),
                names: names(Kind::Building, e.id, &e.internal_name),
                cost: cost(&e.cost)?,
                build_time: e.train_time.ok_or(format!("edificio {} sin TrainTime", e.id))?,
                hp: e.hp.unwrap_or(0),
                tree: tree(Kind::Building, e.id)?,
            },
        );
    }
    let mut techs = BTreeMap::new();
    for e in raw.data.techs.values() {
        techs.insert(
            e.id,
            Tech {
                id: e.id,
                internal_name: e.internal_name.clone(),
                names: names(Kind::Tech, e.id, &e.internal_name),
                cost: cost(&e.cost)?,
                research_time: e
                    .research_time
                    .ok_or(format!("tecnología {} sin ResearchTime", e.id))?,
                repeatable: e.repeatable.unwrap_or(false),
                tree: tree(Kind::Tech, e.id)?,
            },
        );
    }

    // Mejoras de unidad (Ballestero, Hostigador de élite…): se guardan como
    // tecnologías que se investigan en el edificio de la unidad.
    let mut unit_upgrades = BTreeMap::new();
    // Orden fijo (por id de unidad) y, si dos unidades comparten tecnología de
    // mejora, gana la más común: el resultado no depende del orden del JSON.
    let mut upgrades: Vec<(u32, &RawEntity)> = raw
        .data
        .unit_upgrades
        .iter()
        .map(|(k, e)| Ok((k.parse::<u32>()?, e)))
        .collect::<Res<_>>()?;
    upgrades.sort_by_key(|(u, _)| *u);
    let civs = |u: u32| units.get(&u).and_then(|x: &Unit| x.tree.as_ref()).map_or(0, |t| t.civ_count);
    let mut owner: BTreeMap<u32, u32> = BTreeMap::new();
    for (unit, e) in &upgrades {
        if !units.contains_key(unit) || raw.data.techs.contains_key(&e.id.to_string()) {
            continue;
        }
        match owner.get(&e.id) {
            Some(prev) if civs(*prev) >= civs(*unit) => {}
            _ => {
                owner.insert(e.id, *unit);
            }
        }
    }
    for (unit, e) in &upgrades {
        if owner.get(&e.id) != Some(unit) {
            continue;
        }
        let u = &units[unit];
        let names = Names {
            en: format!("{} (upgrade)", u.names.en),
            es: format!("{} (mejora)", u.names.es),
        };
        techs.insert(
            e.id,
            Tech {
                id: e.id,
                internal_name: e.internal_name.clone(),
                names,
                cost: cost(&e.cost)?,
                research_time: e.research_time.ok_or(format!("mejora {} sin ResearchTime", e.id))?,
                repeatable: false,
                tree: u.tree.clone(),
            },
        );
        unit_upgrades.insert(*unit, e.id);
    }

    let data = GameData {
        schema_version: SCHEMA_VERSION,
        source: SourceInfo { repository: REPOSITORY.to_owned(), commit: git_commit(vendor) },
        units,
        buildings,
        techs,
        unit_upgrades,
    };
    Ok((data, votes))
}

/// Copia a app/public/img los íconos del catálogo de la app, por id
/// (`unit/4.png`, `building/68.png`, `tech/101.png`), más recursos y edades.
/// Solo entidades comunes (al menos `MIN_CIVS` civilizaciones). Solo íconos ya
/// publicados por aoe2techtree (reglas de contenido de Microsoft: nada
/// extraído de los archivos del juego).
fn copy_icons(vendor: &Path, root: &Path, data: &GameData, votes: &HashMap<(Kind, u32), NodeVotes>) -> Res<()> {
    let img = vendor.join("img");
    let out = root.join("app/public/img");
    for dir in ["unit", "building", "tech"] {
        let _ = std::fs::remove_dir_all(out.join(dir));
        std::fs::create_dir_all(out.join(dir))?;
    }
    let mut copied = 0;
    let mut copy = |src_dir: &str, dst_dir: &str, id: u32, pic_of: (Kind, u32)| -> Res<()> {
        let Some(v) = votes.get(&pic_of) else { return Ok(()) };
        let (Some(pic), true) = (v.picture_index, v.civs >= MIN_CIVS) else { return Ok(()) };
        let src = img.join(src_dir).join(format!("{pic}.png"));
        if src.exists() {
            std::fs::copy(src, out.join(dst_dir).join(format!("{id}.png")))?;
            copied += 1;
        }
        Ok(())
    };
    for id in data.units.keys() {
        copy("Unit", "unit", *id, (Kind::Unit, *id))?;
    }
    for id in data.buildings.keys() {
        copy("Building", "building", *id, (Kind::Building, *id))?;
    }
    for id in data.techs.keys() {
        copy("Tech", "tech", *id, (Kind::Tech, *id))?;
    }
    // Mejoras de unidad: ícono de la unidad que se obtiene.
    for (unit, tech) in &data.unit_upgrades {
        copy("Unit", "tech", *tech, (Kind::Unit, *unit))?;
    }
    std::fs::create_dir_all(out.join("resource"))?;
    for r in ["food", "wood", "gold", "stone"] {
        std::fs::copy(img.join(format!("{r}.png")), out.join("resource").join(format!("{r}.png")))?;
        copied += 1;
    }
    std::fs::create_dir_all(out.join("age"))?;
    for (age, file) in [("Dark", "dark"), ("Feudal", "feudal"), ("Castle", "castle"), ("Imperial", "imperial")] {
        std::fs::copy(img.join("Ages").join(format!("base_{file}_age.png")), out.join("age").join(format!("{age}.png")))?;
        copied += 1;
    }
    std::fs::write(
        out.join("CREDITS.txt"),
        "Game Icons © Hidden Path Entertainment, Forgotten Empires, SkyBox Labs, Ensemble Studios.\n\
         Copiados de https://github.com/SiegeEngineers/aoe2techtree (img/), publicados bajo las\n\
         Game Content Usage Rules de Microsoft: https://www.xbox.com/en-US/developers/rules\n",
    )?;
    eprintln!("{copied} íconos -> {}", out.display());
    Ok(())
}

/// Una civilización del juego (modo base, no crónicas).
#[derive(serde::Serialize)]
struct CivOut {
    /// Nombre interno en aoe2techtree ("Huns").
    id: String,
    names: Names,
    /// Texto oficial del juego con sus bonos (sin etiquetas HTML).
    description: Names,
    /// Archivo del escudo en app/public/img/civ/.
    icon: String,
    units: Vec<u32>,
    techs: Vec<u32>,
    buildings: Vec<u32>,
}

/// Limpia el texto de ayuda del juego: <br> -> salto de línea, sin <b>.
fn clean_help(s: &str) -> String {
    let s = s.replace("<br>\n", "\n").replace("<br>", "\n").replace("<b>", "").replace("</b>", "");
    let lines: Vec<&str> = s.lines().map(str::trim_end).collect();
    let mut out = lines.join("\n");
    while out.contains("\n\n\n") {
        out = out.replace("\n\n\n", "\n\n");
    }
    out.trim().to_owned()
}

/// Exporta data/civs.json (nombre, descripción oficial y árbol de cada
/// civilización) y copia sus escudos a app/public/img/civ/.
fn import_civs(vendor: &Path, root: &Path) -> Res<()> {
    let raw: serde_json::Value = read_json(&vendor.join("data/data.json"))?;
    let en: HashMap<String, String> = read_json(&vendor.join("data/locales/en/strings.json"))?;
    let es: HashMap<String, String> = read_json(&vendor.join("data/locales/es/strings.json"))?;
    let text = |table: &HashMap<String, String>, id: &serde_json::Value| -> String {
        id.as_u64().and_then(|i| table.get(&i.to_string())).cloned().unwrap_or_default()
    };
    let ids = |v: &serde_json::Value| -> Vec<u32> {
        let mut out: Vec<u32> = v.as_array().map(|a| a.iter().filter_map(|x| x.as_u64().map(|n| n as u32)).collect()).unwrap_or_default();
        out.sort();
        out
    };
    let out_img = root.join("app/public/img/civ");
    let _ = std::fs::remove_dir_all(&out_img);
    std::fs::create_dir_all(&out_img)?;
    let civs = raw["civs"].as_object().ok_or("data.json sin civs")?;
    let mut list: Vec<CivOut> = Vec::new();
    for (id, c) in civs {
        if c["era"].as_str() != Some("base") {
            continue;
        }
        let icon = format!("{}.png", id.to_lowercase());
        let src = vendor.join("img/Civs").join(&icon);
        if src.exists() {
            std::fs::copy(&src, out_img.join(&icon))?;
        }
        list.push(CivOut {
            id: id.clone(),
            names: Names { en: text(&en, &c["name_string_id"]), es: text(&es, &c["name_string_id"]) },
            description: Names { en: clean_help(&text(&en, &c["help_string_id"])), es: clean_help(&text(&es, &c["help_string_id"])) },
            icon,
            units: ids(&c["Unit"]),
            techs: ids(&c["Tech"]),
            buildings: ids(&c["Building"]),
        });
    }
    list.sort_by(|a, b| a.id.cmp(&b.id));
    let mut json = serde_json::to_string_pretty(&list)?;
    json.push('\n');
    std::fs::write(root.join("data/civs.json"), json)?;
    eprintln!("{} civilizaciones -> data/civs.json", list.len());
    Ok(())
}

/// Civilizaciones mínimas para entrar en el catálogo (excluye lo único de una civ).
const MIN_CIVS: u32 = 20;

/// Los textos del árbol traen saltos de línea HTML ("Lumber<br>\nCamp").
fn clean_name(s: &str) -> String {
    s.replace("<br>", " ").split_whitespace().collect::<Vec<_>>().join(" ")
}

fn cost(raw:&HashMap<String, u32>) -> Res<Cost> {
    let mut c = Cost::default();
    for (k, v) in raw {
        match k.as_str() {
            "Food" => c.food = *v,
            "Wood" => c.wood = *v,
            "Gold" => c.gold = *v,
            "Stone" => c.stone = *v,
            other => return Err(format!("recurso desconocido en costo: {other}").into()),
        }
    }
    Ok(c)
}

/// Recorre el árbol de cada civilización y cuenta edad, edificio productor
/// y edificio requerido de cada nodo disponible.
fn collect_tree_votes(dir: &Path) -> Res<HashMap<(Kind, u32), NodeVotes>> {
    let mut files: Vec<_> = std::fs::read_dir(dir)?
        .map(|e| e.map(|e| e.path()))
        .collect::<Result<_, _>>()?;
    files.sort();

    let mut votes: HashMap<(Kind, u32), NodeVotes> = HashMap::new();
    for file in files.iter().filter(|p| p.extension().is_some_and(|e| e == "json")) {
        let tree: RawTree = read_json(file)?;
        for node in tree.buildings.iter().chain(&tree.units_techs) {
            if node.node_status == "NotAvailable" {
                continue;
            }
            let kind = match node.use_type.as_str() {
                "Unit" => Kind::Unit,
                "Building" => Kind::Building,
                "Tech" => Kind::Tech,
                _ => continue,
            };
            let v = votes.entry((kind, node.node_id)).or_default();
            v.civs += 1;
            *v.ages.entry(node.age_id).or_default() += 1;
            *v.produced_at.entry(node.building_id).or_default() += 1;
            let requires = node
                .link_id
                .filter(|_| kind == Kind::Building)
                .filter(|_| node.link_node_type.as_deref() == Some("BuildingTech"));
            *v.requires.entry(requires).or_default() += 1;
            v.name_string_id.get_or_insert(node.name_string_id);
            v.picture_index.get_or_insert(node.picture_index);
        }
    }
    Ok(votes)
}

fn git_commit(repo: &Path) -> String {
    Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
        .unwrap_or_else(|| "desconocido".to_owned())
}
