use ime_core::{Context, EngineUpdate, InputEngine, Key, KeyEvent, Modifiers};
use pinyin::{
    encode_double_pinyin, expand_double_pinyin, DoublePinyinScheme, Lexeme, PinyinConfig,
    PinyinEngine, PinyinLexicon,
};
use std::sync::Arc;

#[test]
fn xiaohe_maps_initial_and_final_pairs_to_full_pinyin() {
    assert_eq!(
        expand_double_pinyin("nihc", DoublePinyinScheme::Xiaohe),
        Some("nihao".into())
    );
    for (code, syllable) in [("gy", "gun"), ("iy", "chun"), ("ky", "kun")] {
        assert_eq!(
            expand_double_pinyin(code, DoublePinyinScheme::Xiaohe).as_deref(),
            Some(syllable)
        );
    }
}

struct PhraseLexicon;
impl PinyinLexicon for PhraseLexicon {
    fn lookup(&self, syllables: &[String]) -> Vec<Lexeme> {
        if syllables == ["ni", "hao"] {
            vec![Lexeme {
                text: "你好".into(),
                frequency: 100.0,
                annotation: None,
            }]
        } else if syllables == ["ni"] {
            vec![Lexeme {
                text: "你".into(),
                frequency: 90.0,
                annotation: None,
            }]
        } else {
            Vec::new()
        }
    }
}

#[test]
fn xiaohe_input_uses_the_same_candidate_engine_as_full_pinyin() {
    let mut engine = PinyinEngine::new(
        Arc::new(PhraseLexicon),
        PinyinConfig {
            double_pinyin: Some(DoublePinyinScheme::Xiaohe),
            ..PinyinConfig::default()
        },
    );
    let context = Context::default();
    let mut result = EngineUpdate::Noop;
    for c in "nihc".chars() {
        result = engine.handle_key(
            KeyEvent {
                key: Key::Character(c),
                modifiers: Modifiers::default(),
                repeat: false,
            },
            &context,
        );
    }
    assert!(matches!(result, EngineUpdate::Candidates(ref candidates)
        if candidates.preedit == "nihc" && candidates.items.first().is_some_and(|item| item.text == "你好")));
}

#[test]
fn natural_code_uses_its_own_final_map() {
    assert_eq!(
        expand_double_pinyin("nihk", DoublePinyinScheme::NaturalCode),
        Some("nihao".into())
    );
    assert_eq!(
        expand_double_pinyin("by", DoublePinyinScheme::NaturalCode),
        Some("bing".into())
    );
}

#[test]
fn incomplete_last_pair_keeps_the_complete_syllable_prefix() {
    assert_eq!(
        expand_double_pinyin("nih", DoublePinyinScheme::Xiaohe),
        Some("ni".into())
    );
}

#[test]
fn common_syllables_round_trip_through_both_pinned_layouts() {
    let syllables = [
        "an", "ai", "ao", "er", "shuo", "shang", "zhong", "bing", "gui", "ju", "yue", "guo", "xue",
        "xiang", "liang", "ren", "shi", "zhu", "nve", "nv",
    ];
    for scheme in [DoublePinyinScheme::Xiaohe, DoublePinyinScheme::NaturalCode] {
        for syllable in syllables {
            let code = encode_double_pinyin(syllable, scheme)
                .unwrap_or_else(|| panic!("no {scheme:?} code for {syllable}"));
            let decoded = expand_double_pinyin(&code, scheme).unwrap();
            assert_eq!(decoded, syllable, "{scheme:?}: {syllable} -> {code}");
        }
    }
}

#[test]
fn bundled_dictionary_syllable_inventory_round_trips() {
    use std::collections::HashSet;
    let syllables: HashSet<&str> = include_str!("../../../data/rime-ice-pinyin.tsv")
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .filter_map(|line| line.split('\t').next())
        .flat_map(str::split_whitespace)
        .filter(|value| value.bytes().all(|b| b.is_ascii_lowercase()))
        .collect();
    let expected_unsupported = ["biang", "lo", "yo"];
    for scheme in [DoublePinyinScheme::Xiaohe, DoublePinyinScheme::NaturalCode] {
        let mut checked = 0;
        let mut unsupported = Vec::new();
        for syllable in &syllables {
            let Some(code) = encode_double_pinyin(syllable, scheme) else {
                unsupported.push(*syllable);
                continue;
            };
            assert_eq!(
                expand_double_pinyin(&code, scheme).as_deref(),
                Some(*syllable),
                "{scheme:?}: {syllable} -> {code}"
            );
            checked += 1;
        }
        assert!(
            checked > 300,
            "only {checked} dictionary syllables mapped for {scheme:?}"
        );
        unsupported.sort_unstable();
        assert_eq!(
            unsupported, expected_unsupported,
            "unsupported inventory for {scheme:?}"
        );
    }
}

#[test]
fn selection_retains_the_unpaired_next_initial() {
    let mut engine = PinyinEngine::new(
        Arc::new(PhraseLexicon),
        PinyinConfig {
            double_pinyin: Some(DoublePinyinScheme::Xiaohe),
            ..PinyinConfig::default()
        },
    );
    let context = Context::default();
    for c in "nih".chars() {
        engine.handle_key(
            KeyEvent {
                key: Key::Character(c),
                modifiers: Modifiers::default(),
                repeat: false,
            },
            &context,
        );
    }
    assert!(matches!(engine.select_candidate(0), EngineUpdate::Commit(ref text) if text == "你"));
    assert!(engine.is_composing());
    let update = engine.handle_key(
        KeyEvent {
            key: Key::Character('a'),
            modifiers: Modifiers::default(),
            repeat: false,
        },
        &context,
    );
    assert!(matches!(update, EngineUpdate::Candidates(ref list) if list.preedit == "ha"));
}

#[test]
fn handles_zero_initial_and_zh_ch_sh_initials() {
    assert_eq!(
        expand_double_pinyin("aa", DoublePinyinScheme::Xiaohe),
        Some("a".into())
    );
    assert_eq!(
        expand_double_pinyin("ai", DoublePinyinScheme::Xiaohe),
        Some("ai".into())
    );
    assert_eq!(
        expand_double_pinyin("gv", DoublePinyinScheme::Xiaohe),
        Some("gui".into())
    );
    assert_eq!(
        expand_double_pinyin("vi", DoublePinyinScheme::Xiaohe),
        Some("zhi".into())
    );
    assert_eq!(
        expand_double_pinyin("ii", DoublePinyinScheme::NaturalCode),
        Some("chi".into())
    );
}
