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

pub struct Bundle {
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
