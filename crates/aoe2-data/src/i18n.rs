//! Textos traducidos (`data/i18n/<idioma>.toml`).
//!
//! El motor trabaja con ids neutros ("forest", "housed"); la traducción se
//! aplica solo al presentar. Si falta una clave se usa el inglés y, en último
//! caso, la propia clave.

use crate::{load_toml, Age, GameData, LoadError};
use std::collections::BTreeMap;
use std::path::Path;

pub const DEFAULT_LANG: &str = "es";
pub const FALLBACK_LANG: &str = "en";

type Table = BTreeMap<String, BTreeMap<String, String>>;

#[derive(Debug, Clone)]
pub struct Locale {
    pub lang: String,
    texts: BTreeMap<String, String>,
    fallback: BTreeMap<String, String>,
}

fn flatten(table: Table) -> BTreeMap<String, String> {
    table
        .into_iter()
        .flat_map(|(section, keys)| keys.into_iter().map(move |(k, v)| (format!("{section}.{k}"), v)))
        .collect()
}

impl Locale {
    pub fn load(data_dir: &Path, lang: &str) -> Result<Locale, LoadError> {
        let dir = data_dir.join("i18n");
        let texts = flatten(load_toml(&dir.join(format!("{lang}.toml")))?);
        let fallback = if lang == FALLBACK_LANG {
            BTreeMap::new()
        } else {
            flatten(load_toml(&dir.join(format!("{FALLBACK_LANG}.toml")))?)
        };
        Ok(Locale { lang: lang.to_owned(), texts, fallback })
    }

    /// Crea un Locale desde textos TOML ya leídos (datos embebidos en la app).
    pub fn from_toml(lang: &str, text: &str, fallback: Option<&str>) -> Result<Locale, toml::de::Error> {
        Ok(Locale {
            lang: lang.to_owned(),
            texts: flatten(toml::from_str(text)?),
            fallback: match fallback {
                Some(f) => flatten(toml::from_str(f)?),
                None => BTreeMap::new(),
            },
        })
    }

    /// Todas las claves de este idioma ("seccion.clave").
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.texts.keys().map(String::as_str)
    }

    /// Todos los textos, con el respaldo debajo (para enviarlos a la interfaz).
    pub fn all(&self) -> BTreeMap<String, String> {
        let mut all = self.fallback.clone();
        all.extend(self.texts.clone());
        all
    }

    pub fn t(&self, key: &str) -> String {
        self.texts
            .get(key)
            .or_else(|| self.fallback.get(key))
            .cloned()
            .unwrap_or_else(|| key.to_owned())
    }

    /// Texto con parámetros `{nombre}`.
    pub fn fmt(&self, key: &str, args: &[(&str, &str)]) -> String {
        let mut s = self.t(key);
        for (k, v) in args {
            s = s.replace(&format!("{{{k}}}"), v);
        }
        s
    }

    pub fn pool(&self, pool: &str) -> String {
        self.t(&format!("pools.{pool}"))
    }

    pub fn age(&self, age: Age) -> String {
        self.t(&format!("ages.{age:?}"))
    }

    pub fn building(&self, data: &GameData, id: u32) -> String {
        data.building(id).map(|b| b.names.get(&self.lang).to_owned()).unwrap_or_else(|| id.to_string())
    }

    pub fn tech(&self, data: &GameData, id: u32) -> String {
        data.tech(id).map(|t| t.names.get(&self.lang).to_owned()).unwrap_or_else(|| id.to_string())
    }

    pub fn unit(&self, data: &GameData, id: u32) -> String {
        data.unit(id).map(|u| u.names.get(&self.lang).to_owned()).unwrap_or_else(|| id.to_string())
    }
}
