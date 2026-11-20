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
    pub wubi_dictionary_path: String,
    pub wubi_version: String,
    pub macros_path: String,
    pub user_data_path: String,
    pub fuzzy_typing: bool,
    pub double_pinyin: String,
}
impl Default for AppConfig {
    fn default() -> Self {
        Self {
            mode: "pinyin".into(),
            page_size: 9,
            dictionary_path: "~/.local/share/omarchy-ime/pinyin.tsv".into(),
            wubi_dictionary_path: "~/.local/share/omarchy-ime/wubi.tsv".into(),
            wubi_version: "86".into(),
            macros_path: "~/.config/omarchy-ime/macros.toml".into(),
            user_data_path: "~/.local/share/omarchy-ime/user.sqlite3".into(),
            fuzzy_typing: true,
            double_pinyin: "full".into(),
        }
    }
}
pub fn load_wubi(path: impl AsRef<Path>) -> io::Result<wubi::StaticWubiLexicon> {
    let text = fs::read_to_string(path)?;
    wubi::parse_tsv(&text).map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}
pub fn load_wubi_version(
    path: impl AsRef<Path>,
    version: wubi::WubiVersion,
) -> io::Result<wubi::StaticWubiLexicon> {
    let text = fs::read_to_string(path)?;
    wubi::parse_tsv_for_version(&text, Some(version))
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
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
        resolve_data_path(&self.dictionary_path, home)
    }
}
fn resolve_data_path(value: &str, home: impl AsRef<Path>) -> std::path::PathBuf {
    if let (Some(rest), Some(data_home)) = (
        value.strip_prefix("~/.local/share/"),
        std::env::var_os("XDG_DATA_HOME"),
    ) {
        return std::path::PathBuf::from(data_home).join(rest);
    }
    if let Some(rest) = value.strip_prefix("~/") {
        return home.as_ref().join(rest);
    }
    let path = Path::new(value);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        home.as_ref().join(path)
    }
}
fn resolve_config_path(value: &str, home: impl AsRef<Path>) -> std::path::PathBuf {
    if let (Some(rest), Some(config_home)) = (
        value.strip_prefix("~/.config/"),
        std::env::var_os("XDG_CONFIG_HOME"),
    ) {
        return std::path::PathBuf::from(config_home).join(rest);
    }
    resolve_data_path(value, home)
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
        double_pinyin: match settings.double_pinyin.as_str() {
            "xiaohe" => Some(pinyin::DoublePinyinScheme::Xiaohe),
            "natural-code" => Some(pinyin::DoublePinyinScheme::NaturalCode),
            _ => None,
        },
        ..defaults
    }
}
pub fn installed_dictionary_path() -> std::path::PathBuf {
    std::path::PathBuf::from("/usr/share/omarchy-ime/pinyin.tsv")
}
pub fn user_wubi_dictionary_path() -> std::path::PathBuf {
    let settings = load_app_config();
    if let Some(path) = std::env::var_os("OMARCHY_IME_WUBI_DICTIONARY") {
        return path.into();
    }
    resolve_data_path(
        &settings.wubi_dictionary_path,
        std::env::var_os("HOME")
            .map(std::path::PathBuf::from)
            .unwrap_or_default(),
    )
}
pub fn runtime_wubi_version() -> wubi::WubiVersion {
    match load_app_config().wubi_version.as_str() {
        "98" => wubi::WubiVersion::V98,
        "new-century" => wubi::WubiVersion::NewCentury,
        _ => wubi::WubiVersion::V86,
    }
}
pub fn runtime_macros() -> ime_macros::MacroTable {
    static MACROS: OnceLock<Arc<ime_macros::MacroTable>> = OnceLock::new();
    MACROS
        .get_or_init(|| {
            let settings = load_app_config();
            let path = resolve_config_path(
                &settings.macros_path,
                std::env::var_os("HOME")
                    .map(std::path::PathBuf::from)
                    .unwrap_or_default(),
            );
            let table = fs::read_to_string(path)
                .or_else(|_| fs::read_to_string("/usr/share/omarchy-ime/macros.toml"))
                .ok()
                .and_then(|text| ime_macros::MacroTable::parse(&text).ok())
                .unwrap_or_default();
            Arc::new(table)
        })
        .as_ref()
        .clone()
}
pub fn load_default_wubi(version: wubi::WubiVersion) -> Arc<wubi::StaticWubiLexicon> {
    static WUBI: OnceLock<Arc<wubi::StaticWubiLexicon>> = OnceLock::new();
    WUBI.get_or_init(|| {
        let table = load_wubi_version(user_wubi_dictionary_path(), version)
            .or_else(|_| load_wubi_version("/usr/share/omarchy-ime/wubi.tsv", version));
        let table = match table {
            Ok(table) => Ok(table),
            Err(original_error) => test_wubi_fallback(version, original_error),
        };
        Arc::new(table.unwrap_or_else(|error| {
            eprintln!("omarchy-ime: cannot load Wubi table {version:?}: {error}; Wubi mode will fall back to Pinyin-only lookup");
            wubi::StaticWubiLexicon::default()
        }))
    })
    .clone()
}
fn test_wubi_fallback(
    version: wubi::WubiVersion,
    original_error: io::Error,
) -> io::Result<wubi::StaticWubiLexicon> {
    #[cfg(test)]
    {
        let _ = original_error;
        wubi::parse_tsv_for_version(include_str!("../../../data/wubi.tsv"), Some(version))
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
    }
    #[cfg(not(test))]
    {
        let _ = version;
        Err(original_error)
    }
}
pub fn load_default_pinyin() -> PinyinEngine {
    PinyinEngine::new(runtime_lexicon(), runtime_pinyin_config())
}
static RUNTIME_BASE_LEXICON: OnceLock<Arc<dyn pinyin::PinyinLexicon>> = OnceLock::new();
static RUNTIME_LEXICON: OnceLock<Arc<dyn pinyin::PinyinLexicon>> = OnceLock::new();
fn runtime_lexicon() -> Arc<dyn pinyin::PinyinLexicon> {
    RUNTIME_LEXICON
        .get_or_init(|| {
            let base = RUNTIME_BASE_LEXICON
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
                .clone();
            Arc::new(ime_user_data::PersonalizedLexicon::new(
                base,
                runtime_user_data(),
            ))
        })
        .clone()
}
fn runtime_user_data() -> ime_user_data::LearningStore {
    static STORE: OnceLock<ime_user_data::LearningStore> = OnceLock::new();
    STORE.get_or_init(|| {
        #[cfg(test)]
        { ime_user_data::LearningStore::open_in_memory().expect("in-memory SQLite should open") }
        #[cfg(not(test))]
        {
        let settings = load_app_config();
        let path = resolve_data_path(
            &settings.user_data_path,
            std::env::var_os("HOME").map(std::path::PathBuf::from).unwrap_or_default(),
        );
        if let Some(parent) = path.parent() { let _ = fs::create_dir_all(parent); }
        ime_user_data::LearningStore::open(&path).unwrap_or_else(|error| {
            eprintln!("omarchy-ime: cannot open local learning database {}: {error}; using memory only", path.display());
            ime_user_data::LearningStore::open_in_memory().expect("in-memory SQLite should open")
        })
        }
    }).clone()
}
fn record_learning(code: &str, text: &str) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs());
    if let Err(error) = runtime_user_data().record_choice(code, text, now) {
        eprintln!("omarchy-ime: could not save local word usage: {error}");
    }
}
pub fn add_learned_phrase(code: &str, text: &str) -> Result<(), rusqlite::Error> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs());
    let store = runtime_user_data();
    if !store.is_persistent() {
        return Err(rusqlite::Error::InvalidQuery);
    }
    store.add_phrase(code, text, 1_000_000.0, now)
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
