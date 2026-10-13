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
    copy_icons(&vendor, &root, &votes)?;
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

    let data = GameData {
        schema_version: SCHEMA_VERSION,
        source: SourceInfo { repository: REPOSITORY.to_owned(), commit: git_commit(vendor) },
        units,
        buildings,
        techs,
    };
    Ok((data, votes))
}

#[derive(Deserialize)]
struct RulesNames {
    names: NamesTables,
}

#[derive(Deserialize)]
struct NamesTables {
    buildings: BTreeMap<String, u32>,
    techs: BTreeMap<String, u32>,
    units: BTreeMap<String, u32>,
}

/// Copia a app/public/img los íconos que usa la app, con su nombre corto de
/// data/rules.toml (`building/house.png`, `tech/feudal_age.png`…), más los de
/// recursos y edades. Solo íconos ya publicados por aoe2techtree (reglas de
/// contenido de Microsoft: nada extraído de los archivos del juego).
fn copy_icons(vendor: &Path, root: &Path, votes: &HashMap<(Kind, u32), NodeVotes>) -> Res<()> {
    let rules: RulesNames = toml::from_str(&std::fs::read_to_string(root.join("data/rules.toml"))?)?;
    let img = vendor.join("img");
    let out = root.join("app/public/img");
    let mut copied = 0;
    let groups = [
        (Kind::Building, "Building", "building", &rules.names.buildings),
        (Kind::Tech, "Tech", "tech", &rules.names.techs),
        (Kind::Unit, "Unit", "unit", &rules.names.units),
    ];
    for (kind, src_dir, dst_dir, names) in groups {
        std::fs::create_dir_all(out.join(dst_dir))?;
        for (name, id) in names {
            let Some(pic) = votes.get(&(kind, *id)).and_then(|v| v.picture_index) else {
                eprintln!("sin ícono: {dst_dir}/{name}");
                continue;
            };
            std::fs::copy(img.join(src_dir).join(format!("{pic}.png")), out.join(dst_dir).join(format!("{name}.png")))?;
            copied += 1;
        }
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
