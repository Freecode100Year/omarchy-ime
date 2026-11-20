//! Local runtime shared by the command-line diagnostics and Fcitx5 adapter.
use lexicon::{read_pinyin_tsv, FstPinyinLexicon};
use pinyin::{PinyinConfig, PinyinEngine};
use serde::{Deserialize, Serialize};
use std::{
    fs, io,
    path::Path,
    sync::{Arc, OnceLock},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub mode: String,
    pub page_size: usize,
    pub dictionary_path: String,
    pub user_data_path: String,
    pub fuzzy_typing: bool,
}
impl Default for AppConfig {
    fn default() -> Self {
        Self {
            mode: "pinyin".into(),
            page_size: 9,
            dictionary_path: "~/.local/share/omarchy-ime/pinyin.tsv".into(),
            user_data_path: "~/.local/share/omarchy-ime/user.sqlite3".into(),
            fuzzy_typing: true,
        }
    }
}
impl AppConfig {
    pub fn parse(text: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(text)
    }
    pub fn load(path: impl AsRef<Path>) -> io::Result<Self> {
        let text = fs::read_to_string(path)?;
        Self::parse(&text).map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
    }
    pub fn resolve_dictionary_path(&self, home: impl AsRef<Path>) -> std::path::PathBuf {
        let value = self.dictionary_path.strip_prefix("~/");
        match value {
            Some(rest) => home.as_ref().join(rest),
            None => {
                let path = Path::new(&self.dictionary_path);
                if path.is_absolute() {
                    path.to_path_buf()
                } else {
                    home.as_ref().join(path)
                }
            }
        }
    }
}

fn parse_lexicon(text: &str) -> io::Result<Arc<dyn pinyin::PinyinLexicon>> {
    let entries = read_pinyin_tsv(text)?;
    if entries.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "dictionary has no entries",
        ));
    }
    let lexicon = FstPinyinLexicon::from_sorted(entries)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid dictionary rows"))?;
    Ok(Arc::new(lexicon))
}
pub fn load_pinyin(path: impl AsRef<Path>) -> io::Result<PinyinEngine> {
    let text = fs::read_to_string(path)?;
    Ok(PinyinEngine::new(
        parse_lexicon(&text)?,
        PinyinConfig::default(),
    ))
}
fn app_config_path() -> std::path::PathBuf {
    std::env::var_os("OMARCHY_IME_CONFIG")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::env::var_os("XDG_CONFIG_HOME")
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| {
                    std::path::PathBuf::from(std::env::var_os("HOME").unwrap_or_default())
                        .join(".config")
                })
                .join("omarchy-ime/config.toml")
        })
}
fn load_app_config() -> AppConfig {
    AppConfig::load(app_config_path())
        .or_else(|_| AppConfig::load("/usr/share/omarchy-ime/config.toml"))
        .unwrap_or_default()
}
pub fn user_dictionary_path() -> std::path::PathBuf {
    if let Some(path) = std::env::var_os("OMARCHY_IME_DICTIONARY") {
        return path.into();
    }
    let home = std::path::PathBuf::from(std::env::var_os("HOME").unwrap_or_default());
    load_app_config().resolve_dictionary_path(home)
}
pub fn runtime_pinyin_config() -> PinyinConfig {
    let settings = load_app_config();
    let defaults = PinyinConfig::default();
    PinyinConfig {
        page_size: settings.page_size.clamp(1, 9),
        max_edge_cost: if settings.fuzzy_typing {
            defaults.max_edge_cost
        } else {
            defaults.exact_cost
        },
        ..defaults
    }
}
pub fn installed_dictionary_path() -> std::path::PathBuf {
    std::path::PathBuf::from("/usr/share/omarchy-ime/pinyin.tsv")
}
pub fn load_default_pinyin() -> PinyinEngine {
    let lexicon = fs::read_to_string(user_dictionary_path())
        .ok()
        .and_then(|text| parse_lexicon(&text).ok())
        .or_else(|| {
            fs::read_to_string(installed_dictionary_path())
                .ok()
                .and_then(|text| parse_lexicon(&text).ok())
        })
        .unwrap_or_else(bundled_lexicon);
    PinyinEngine::new(lexicon, runtime_pinyin_config())
}
static RUNTIME_LEXICON: OnceLock<Arc<dyn pinyin::PinyinLexicon>> = OnceLock::new();
fn runtime_lexicon() -> Arc<dyn pinyin::PinyinLexicon> {
    RUNTIME_LEXICON
        .get_or_init(|| {
            let user = fs::read_to_string(user_dictionary_path())
                .ok()
                .and_then(|text| parse_lexicon(&text).ok());
            user.or_else(|| {
                fs::read_to_string(installed_dictionary_path())
                    .ok()
                    .and_then(|text| parse_lexicon(&text).ok())
            })
            .unwrap_or_else(bundled_lexicon)
        })
        .clone()
}
fn bundled_lexicon() -> Arc<dyn pinyin::PinyinLexicon> {
    let entries = read_pinyin_tsv(include_str!("../../../data/pinyin-starter.tsv"))
        .expect("bundled starter dictionary is valid");
    Arc::new(FstPinyinLexicon::from_sorted(entries).expect("bundled starter keys are sorted"))
}
pub fn bundled_pinyin() -> PinyinEngine {
    PinyinEngine::new(bundled_lexicon(), runtime_pinyin_config())
}

mod ffi;
pub use ffi::*;
