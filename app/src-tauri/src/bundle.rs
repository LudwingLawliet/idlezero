//! Datos del juego embebidos en el binario (data/ al compilar).

use aoe2_data::{GameData, GatherRates, Locale, MapDef, Modifiers, Rules};
use aoe2_opt::Goal;
use aoe2_sim::{BuildOrder, Scenario};
use std::sync::OnceLock;

macro_rules! data {
    ($path:literal) => {
        include_str!(concat!("../../../data/", $path))
    };
}

/// (id, contenido) de cada mapa.
const MAPS: &[(&str, &str)] = &[
    ("arabia_neutra", data!("maps/arabia_neutra.toml")),
    ("laboratorio", data!("maps/laboratorio.toml")),
];

/// (código, contenido) de cada idioma; el primero es el de respaldo.
pub const LANGS: &[(&str, &str)] = &[("en", data!("i18n/en.toml")), ("es", data!("i18n/es.toml"))];

#[derive(serde::Deserialize, Clone)]
pub struct Tip {
    pub id: String,
    pub context: Vec<String>,
    #[serde(default)]
    pub keys: Vec<String>,
    #[serde(default)]
    pub keys_en: Vec<String>,
    pub es: String,
    pub en: String,
}

#[derive(serde::Deserialize)]
struct TipsFile {
    tip: Vec<Tip>,
}

#[derive(serde::Deserialize, Clone)]
pub struct ClassName {
    pub en: String,
    pub es: String,
    #[serde(default = "yes")]
    pub shown: bool,
}

fn yes() -> bool {
    true
}

#[derive(serde::Deserialize, Clone)]
pub struct Category {
    pub id: String,
    pub class: u32,
}

#[derive(serde::Deserialize)]
pub struct ArmorClasses {
    pub class: std::collections::BTreeMap<String, ClassName>,
    pub category: Vec<Category>,
}

#[derive(serde::Deserialize, Clone)]
pub struct UpgradeGroup {
    pub category: String,
    pub techs: Vec<u32>,
    pub recommended: Vec<u32>,
}

#[derive(serde::Deserialize)]
struct UpgradeFile {
    group: Vec<UpgradeGroup>,
}

pub struct Bundle {
    pub classes: ArmorClasses,
    pub upgrade_groups: Vec<UpgradeGroup>,
    pub tips: Vec<Tip>,
    pub civs: Vec<aoe2_data::civs::CivInfo>,
    pub civ_bonuses: Vec<aoe2_data::civs::CivBonuses>,
    pub data: GameData,
    pub rates: GatherRates,
    pub rules: Rules,
    pub modifiers: Modifiers,
    pub maps: Vec<(String, MapDef)>,
    pub goals: Vec<Goal>,
}

fn parse<T: serde::de::DeserializeOwned>(what: &str, text: &str) -> T {
    toml::from_str(text).unwrap_or_else(|e| panic!("datos embebidos inválidos ({what}): {e}"))
}

pub fn bundle() -> &'static Bundle {
    static B: OnceLock<Bundle> = OnceLock::new();
    B.get_or_init(|| Bundle {
        tips: parse::<TipsFile>("tips", data!("tips.toml")).tip,
        classes: parse("armor_classes", data!("armor_classes.toml")),
        upgrade_groups: parse::<UpgradeFile>("upgrade_groups", data!("upgrade_groups.toml")).group,
        civs: aoe2_data::civs::parse_civs(data!("civs.json")).expect("civs.json embebido"),
        civ_bonuses: aoe2_data::civs::parse_bonuses(data!("civ_bonuses.toml")).expect("civ_bonuses.toml embebido"),
        data: serde_json::from_str(data!("game_data.json")).expect("game_data.json embebido"),
        rates: parse("gather_rates", data!("gather_rates.toml")),
        rules: parse("rules", data!("rules.toml")),
        modifiers: parse("modifiers", data!("modifiers.toml")),
        maps: MAPS.iter().map(|(id, t)| (id.to_string(), parse(id, t))).collect(),
        goals: aoe2_opt::goal::parse_presets(data!("goals.toml")).expect("goals.toml embebido"),
    })
}

pub fn locale(lang: &str) -> Locale {
    let fallback = LANGS[0].1;
    let text = LANGS.iter().find(|(l, _)| *l == lang).map_or(fallback, |(_, t)| t);
    let lang = if LANGS.iter().any(|(l, _)| *l == lang) { lang } else { LANGS[0].0 };
    Locale::from_toml(lang, text, (lang != LANGS[0].0).then_some(fallback)).expect("i18n embebido")
}

/// Escenario con una orden vacía (el optimizador pone la suya) y, si se da,
/// la civilización (árbol + bonos).
pub fn scenario(map: &str, civ: Option<&str>) -> Result<Scenario, String> {
    let b = bundle();
    let (_, m) = b.maps.iter().find(|(id, _)| id == map).ok_or(format!("mapa desconocido: {map}"))?;
    let order = BuildOrder::parse(&format!("name = \"-\"\nmap = \"{map}\"\ndefault_pool = \"forest\""))
        .map_err(|e| e.to_string())?;
    Ok(Scenario {
        data: b.data.clone(),
        rates: b.rates.clone(),
        rules: b.rules.clone(),
        modifiers: b.modifiers.clone(),
        map: m.clone(),
        order,
        civ: match civ {
            Some(id) => Some(aoe2_data::Civ::build(id, &b.civs, &b.civ_bonuses).ok_or(format!("civilización desconocida: {id}"))?),
            None => None,
        },
    })
}

/// Categoría de una unidad (infantry, archers, cavalry…) según sus clases de armadura.
pub fn unit_category(unit: &aoe2_data::Unit) -> String {
    let b = bundle();
    for c in &b.classes.category {
        if unit.combat.armours.iter().any(|a| a.class == c.class) {
            return c.id.clone();
        }
    }
    "other".into()
}

/// Primera unidad de la línea (Milicia para Campeón).
pub fn line_root(id: u32) -> u32 {
    let mut cur = id;
    for _ in 0..10 {
        match bundle().data.unit_previous.get(&cur) {
            Some(p) => cur = *p,
            None => break,
        }
    }
    cur
}
