//! Private, process-local cache backed by a user-owned SQLite WAL database.
use pinyin::{Lexeme, PinyinLexicon};
use rusqlite::{params, Connection};
use std::{
    borrow::Cow,
    collections::HashMap,
    path::Path,
    sync::{Arc, Mutex, RwLock},
};

const DEFAULT_HALF_LIFE: u64 = 30 * 24 * 60 * 60;

#[derive(Clone, Debug)]
struct Entry {
    weight: f32,
    uses: u64,
    last_used: u64,
}

#[derive(Clone)]
pub struct LearningStore {
    connection: Arc<Mutex<Connection>>,
    entries: Arc<RwLock<HashMap<String, HashMap<String, Entry>>>>,
    persistent: bool,
}

impl LearningStore {
    pub fn open(path: impl AsRef<Path>) -> rusqlite::Result<Self> {
        let path = path.as_ref();
        #[cfg(unix)]
        {
            use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
            let file = std::fs::OpenOptions::new()
                .create(true)
                .truncate(false)
                .write(true)
                .mode(0o600)
                .open(path)
                .map_err(|_| rusqlite::Error::InvalidPath(path.to_path_buf()))?;
            file.set_permissions(std::fs::Permissions::from_mode(0o600))
                .map_err(|_| rusqlite::Error::InvalidPath(path.to_path_buf()))?;
            for suffix in ["-wal", "-shm"] {
                let mut sidecar = path.as_os_str().to_os_string();
                sidecar.push(suffix);
                let sidecar = Path::new(&sidecar);
                if sidecar.exists() {
                    std::fs::set_permissions(sidecar, std::fs::Permissions::from_mode(0o600))
                        .map_err(|_| rusqlite::Error::InvalidPath(sidecar.to_path_buf()))?;
                }
            }
        }
        let connection = Connection::open(path)?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.pragma_update(None, "synchronous", "NORMAL")?;
        Self::from_connection(connection, true)
    }

    pub fn open_in_memory() -> rusqlite::Result<Self> {
        Self::from_connection(Connection::open_in_memory()?, false)
    }

    pub fn is_persistent(&self) -> bool {
        self.persistent
    }

    fn from_connection(connection: Connection, persistent: bool) -> rusqlite::Result<Self> {
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS learned_phrases (
                code TEXT NOT NULL,
                text TEXT NOT NULL,
                weight REAL NOT NULL DEFAULT 1.0,
                uses INTEGER NOT NULL DEFAULT 0,
                last_used INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY(code, text)
            );",
        )?;
        let entries = {
            let mut statement = connection
                .prepare("SELECT code,text,weight,uses,last_used FROM learned_phrases")?;
            let rows = statement
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        Entry {
                            weight: row.get::<_, f32>(2)?,
                            uses: row.get::<_, u64>(3)?,
                            last_used: row.get::<_, u64>(4)?,
                        },
                    ))
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            let mut index = HashMap::new();
            for (code, text, entry) in rows {
                index
                    .entry(code)
                    .or_insert_with(HashMap::new)
                    .insert(text, entry);
            }
            index
        };
        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
            entries: Arc::new(RwLock::new(entries)),
            persistent,
        })
    }

    pub fn add_phrase(
        &self,
        code: &str,
        text: &str,
        weight: f32,
        now: u64,
    ) -> rusqlite::Result<()> {
        let code = normalize_code(code);
        if code.is_empty()
            || code.len() > 128
            || !code.bytes().all(|b| b.is_ascii_lowercase())
            || text.is_empty()
            || text.len() > 4096
            || text.contains('\0')
            || !weight.is_finite()
            || weight < 0.0
        {
            return Err(rusqlite::Error::InvalidParameterName(
                "invalid learned phrase".into(),
            ));
        }
        let connection = self
            .connection
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        connection.execute(
            "INSERT INTO learned_phrases(code,text,weight,uses,last_used) VALUES (?1,?2,?3,0,?4)
             ON CONFLICT(code,text) DO UPDATE SET weight=excluded.weight,last_used=excluded.last_used",
            params![code, text, weight, now],
        )?;
        self.entries
            .write()
            .map_err(|_| rusqlite::Error::InvalidQuery)?
            .entry(code)
            .or_default()
            .insert(
                text.to_owned(),
                Entry {
                    weight,
                    uses: 0,
                    last_used: now,
                },
            );
        Ok(())
    }

    pub fn record_choice(&self, code: &str, text: &str, now: u64) -> rusqlite::Result<()> {
        let code = normalize_code(code);
        if code.is_empty()
            || code.len() > 128
            || !code.bytes().all(|b| b.is_ascii_lowercase())
            || text.is_empty()
            || text.len() > 4096
            || text.contains('\0')
        {
            return Ok(());
        }
        let connection = self
            .connection
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        connection.execute(
            "INSERT INTO learned_phrases(code,text,weight,uses,last_used) VALUES (?1,?2,1.0,1,?3)
             ON CONFLICT(code,text) DO UPDATE SET uses=uses+1,last_used=excluded.last_used",
            params![code, text, now],
        )?;
        let mut rows = self
            .entries
            .write()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let entry = rows
            .entry(code)
            .or_default()
            .entry(text.to_owned())
            .or_insert(Entry {
                weight: 1.0,
                uses: 0,
                last_used: now,
            });
        entry.uses = entry.uses.saturating_add(1);
        entry.last_used = now;
        Ok(())
    }

    pub fn lookup(&self, code: &str, now: u64) -> Vec<Lexeme> {
        let code = normalize_code(code);
        let Ok(rows) = self.entries.read() else {
            return Vec::new();
        };
        rows.get(&code)
            .into_iter()
            .flat_map(|entries| entries.iter())
            .map(|(text, entry)| Lexeme {
                text: text.clone(),
                frequency: decayed_weight(
                    entry.weight + entry.uses.min(1_000_000) as f32 * 500_000.0,
                    entry.last_used,
                    now,
                    DEFAULT_HALF_LIFE,
                ),
                annotation: Some("用户学习".into()),
            })
            .collect()
    }
}

pub struct PersonalizedLexicon {
    base: Arc<dyn PinyinLexicon>,
    store: LearningStore,
}

impl PersonalizedLexicon {
    pub fn new(base: Arc<dyn PinyinLexicon>, store: LearningStore) -> Self {
        Self { base, store }
    }
}

impl PinyinLexicon for PersonalizedLexicon {
    fn lookup(&self, syllables: &[String]) -> Vec<Lexeme> {
        let mut result = self.base.lookup(syllables);
        let code = syllables.concat();
        for learned in self.store.lookup(&code, unix_now()) {
            if let Some(existing) = result.iter_mut().find(|item| item.text == learned.text) {
                existing.frequency = existing.frequency.max(learned.frequency);
                existing.annotation = learned.annotation;
            } else {
                result.push(learned);
            }
        }
        result
    }
    fn has_prefix(&self, syllables: &[String]) -> bool {
        self.base.has_prefix(syllables)
    }
    fn lookup_cow<'a>(&'a self, syllables: &[String]) -> Cow<'a, [Lexeme]> {
        let learned = self.store.lookup(&syllables.concat(), unix_now());
        if learned.is_empty() {
            return self.base.lookup_cow(syllables);
        }
        let mut result = self.base.lookup(syllables);
        for item in learned {
            if let Some(existing) = result.iter_mut().find(|base| base.text == item.text) {
                existing.frequency = existing.frequency.max(item.frequency);
                existing.annotation = item.annotation;
            } else {
                result.push(item);
            }
        }
        Cow::Owned(result)
    }
    fn bigram_score(&self, previous: &str, next: &str) -> f32 {
        self.base.bigram_score(previous, next)
    }
}

pub fn decayed_weight(weight: f32, last_used: u64, now: u64, half_life: u64) -> f32 {
    if !weight.is_finite() || weight <= 0.0 {
        return 0.0;
    }
    let age = now.saturating_sub(last_used) as f32;
    let half_life = half_life.max(1) as f32;
    weight * 2.0_f32.powf(-age / half_life)
}

fn normalize_code(code: &str) -> String {
    code.chars()
        .filter(|ch| !ch.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect()
}
fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}
