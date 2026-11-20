use ime_user_data::{decayed_weight, LearningStore, PersonalizedLexicon};
use pinyin::{Lexeme, PinyinLexicon};
use std::sync::Arc;

struct EmptyLexicon;
impl PinyinLexicon for EmptyLexicon {
    fn lookup(&self, _: &[String]) -> Vec<Lexeme> {
        Vec::new()
    }
}

#[test]
fn learned_choice_appears_in_matching_pinyin_candidates() {
    let store = LearningStore::open_in_memory().unwrap();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    store.add_phrase("nihao", "你好", 1.0, now).unwrap();
    store.record_choice("nihao", "你好", now).unwrap();
    let lexicon = PersonalizedLexicon::new(Arc::new(EmptyLexicon), store);
    let candidates = lexicon.lookup(&["ni".into(), "hao".into()]);
    assert_eq!(candidates[0].text, "你好");
    assert!(candidates[0].frequency > 1.0);
}

#[test]
fn user_frequency_decays_by_half_over_configured_half_life() {
    assert!((decayed_weight(10.0, 100, 100 + 86_400, 86_400) - 5.0).abs() < 0.001);
    assert_eq!(decayed_weight(10.0, 200, 100, 86_400), 10.0);
}

#[test]
fn sqlite_store_uses_wal_for_persistent_paths() {
    let path = std::env::temp_dir().join(format!(
        "omarchy-ime-learning-{}.sqlite3",
        std::process::id()
    ));
    let store = LearningStore::open(&path).unwrap();
    assert!(store.is_persistent());
    store.record_choice("nihao", "你好", 10).unwrap();
    drop(store);
    let reopened = LearningStore::open(&path).unwrap();
    assert_eq!(reopened.lookup("nihao", 10).len(), 1);
    drop(reopened);
    let connection = rusqlite::Connection::open(&path).unwrap();
    let mode: String = connection
        .query_row("PRAGMA journal_mode", [], |row| row.get(0))
        .unwrap();
    assert_eq!(mode.to_ascii_lowercase(), "wal");
    drop(connection);
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(path.with_extension("sqlite3-wal"));
    let _ = std::fs::remove_file(path.with_extension("sqlite3-shm"));
}

#[cfg(unix)]
#[test]
fn learning_database_is_private_even_when_created_with_a_permissive_umask() {
    use std::os::unix::fs::PermissionsExt;
    let path = std::env::temp_dir().join(format!(
        "omarchy-ime-private-{}.sqlite3",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    std::fs::write(&path, []).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o666)).unwrap();
    let store = LearningStore::open(&path).unwrap();
    assert_eq!(
        std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    store.record_choice("test", "隐私", 1).unwrap();
    for suffix in ["-wal", "-shm"] {
        let mut sidecar = path.as_os_str().to_os_string();
        sidecar.push(suffix);
        let sidecar = std::path::PathBuf::from(sidecar);
        if sidecar.exists() {
            assert_eq!(
                std::fs::metadata(sidecar).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
    }
    assert!(store.is_persistent());
    drop(store);
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(path.with_extension("sqlite3-wal"));
    let _ = std::fs::remove_file(path.with_extension("sqlite3-shm"));
}

#[test]
fn in_memory_learning_store_reports_non_persistent() {
    assert!(!LearningStore::open_in_memory().unwrap().is_persistent());
}
