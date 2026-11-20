use omarchy_ime::{load_pinyin, AppConfig};
use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn defaults_are_local_and_have_bounded_candidate_count() {
    let cfg = AppConfig::default();
    assert_eq!(cfg.mode, "pinyin");
    assert_eq!(cfg.page_size, 9);
    assert!(cfg
        .dictionary_path
        .starts_with("~/.local/share/omarchy-ime"));
    assert_eq!(cfg.wubi_version, "86");
    assert_eq!(cfg.double_pinyin, "full");
    assert!(cfg.wubi_dictionary_path.ends_with("/wubi.tsv"));
    assert!(cfg.user_data_path.ends_with("user.sqlite3"));
    assert!(cfg.macros_path.ends_with("macros.toml"));
}

#[test]
fn local_dictionary_loads_and_empty_dictionary_is_rejected() {
    let path = std::env::temp_dir().join(format!(
        "omarchy-ime-{}.tsv",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::write(&path, "ni hao\t你好\t100\n").unwrap();
    assert!(load_pinyin(path.to_str().unwrap()).is_ok());
    fs::write(&path, "# empty\n").unwrap();
    assert!(load_pinyin(path.to_str().unwrap()).is_err());
    let _ = fs::remove_file(path);
}

#[test]
fn reads_toml_runtime_settings_and_expands_home_paths() {
    let path = std::env::temp_dir().join(format!(
        "omarchy-ime-config-{}.toml",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::write(
        &path,
        "page_size = 5\nfuzzy_typing = false\ndictionary_path = \"~/dict.tsv\"\n",
    )
    .unwrap();
    let cfg = AppConfig::load(&path).unwrap();
    assert_eq!(cfg.page_size, 5);
    assert!(!cfg.fuzzy_typing);
    assert_eq!(
        cfg.resolve_dictionary_path("/tmp/example"),
        std::path::PathBuf::from("/tmp/example/dict.tsv")
    );
    let _ = fs::remove_file(path);
}

#[test]
fn shipped_dictionary_covers_basic_single_character_syllables() {
    use ime_core::{Context, EngineUpdate, InputEngine, Key, KeyEvent, Modifiers};
    use lexicon::{read_pinyin_tsv, FstPinyinLexicon};
    use pinyin::{PinyinConfig, PinyinEngine, PinyinLexicon};
    use std::sync::Arc;

    let entries = read_pinyin_tsv(include_str!("../../../data/rime-ice-pinyin.tsv")).unwrap();
    let lexicon = Arc::new(FstPinyinLexicon::from_sorted(entries).unwrap());
    assert!(lexicon.has_prefix(&["shuang".into()]));
    for (input, expected) in [("ni", "你"), ("ai", "爱"), ("chi", "吃"), ("xue", "学")] {
        let mut engine = PinyinEngine::new(lexicon.clone(), PinyinConfig::default());
        let mut candidates = None;
        for c in input.chars() {
            let update = engine.handle_key(
                KeyEvent {
                    key: Key::Character(c),
                    modifiers: Modifiers::default(),
                    repeat: false,
                },
                &Context::default(),
            );
            if let EngineUpdate::Candidates(list) = update {
                candidates = Some(list);
            }
        }
        assert_eq!(
            candidates.unwrap().items.first().unwrap().text,
            expected,
            "input {input}"
        );
    }
    let mut engine = PinyinEngine::new(
        lexicon.clone(),
        PinyinConfig {
            beam_width: 4,
            page_size: 9,
            ..PinyinConfig::default()
        },
    );
    let mut page = Vec::new();
    for c in "shi".chars() {
        if let EngineUpdate::Candidates(candidates) = engine.handle_key(
            KeyEvent {
                key: Key::Character(c),
                modifiers: Modifiers::default(),
                repeat: false,
            },
            &Context::default(),
        ) {
            page = candidates.items.into_iter().map(|c| c.text).collect();
        }
    }
    assert_eq!(
        page,
        ["是", "时", "事", "实", "十", "师", "使", "屎", "世"]
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>()
    );

    let mut one_slot = PinyinEngine::new(
        lexicon.clone(),
        PinyinConfig {
            beam_width: 4,
            page_size: 1,
            ..PinyinConfig::default()
        },
    );
    for c in "shi".chars() {
        if let EngineUpdate::Candidates(candidates) = one_slot.handle_key(
            KeyEvent {
                key: Key::Character(c),
                modifiers: Modifiers::default(),
                repeat: false,
            },
            &Context::default(),
        ) {
            if c == 'i' {
                assert_eq!(candidates.items[0].text, "是");
            }
        }
    }

    let mut engine = PinyinEngine::new(lexicon.clone(), PinyinConfig::default());
    let mut corrected = Vec::new();
    for c in "shoduao".chars() {
        if let EngineUpdate::Candidates(candidates) = engine.handle_key(
            KeyEvent {
                key: Key::Character(c),
                modifiers: Modifiers::default(),
                repeat: false,
            },
            &Context::default(),
        ) {
            corrected = candidates.items.into_iter().map(|c| c.text).collect();
        }
    }
    assert!(
        corrected.contains(&"收到".to_owned()),
        "transposition correction candidates: {corrected:?}"
    );

    let mut engine = PinyinEngine::new(lexicon.clone(), PinyinConfig::default());
    let mut corrected = Vec::new();
    for c in "shuansghou".chars() {
        if let EngineUpdate::Candidates(candidates) = engine.handle_key(
            KeyEvent {
                key: Key::Character(c),
                modifiers: Modifiers::default(),
                repeat: false,
            },
            &Context::default(),
        ) {
            corrected = candidates.items.into_iter().map(|c| c.text).collect();
        }
    }
    assert_eq!(corrected.first().map(String::as_str), Some("双手"));
}

#[test]
fn shipped_wubi_dictionary_loads_and_supports_reverse_lookup() {
    let wubi = wubi::parse_tsv(include_str!("../../../data/wubi.tsv")).unwrap();
    use wubi::{WubiLexicon, WubiVersion};
    assert!(wubi
        .lookup(WubiVersion::V86, "khk")
        .iter()
        .any(|x| x.text == "中"));
    assert!(wubi
        .reverse_lookup(WubiVersion::V86, '中')
        .contains(&"khk".to_owned()));
    assert!(wubi
        .lookup(WubiVersion::V98, "a")
        .iter()
        .any(|x| x.text == "工"));
}
