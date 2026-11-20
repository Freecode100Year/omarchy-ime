use ime_core::{Context, EngineUpdate, InputEngine, Key, KeyEvent, Modifiers};
use pinyin::{Lexeme, PinyinLexicon};
use std::sync::Arc;
use wubi::{MixedConfig, MixedEngine, WubiLexicon, WubiVersion};

struct WubiFixture;
impl WubiLexicon for WubiFixture {
    fn lookup(&self, _: WubiVersion, code: &str) -> Vec<Lexeme> {
        if code == "aaaa" {
            vec![Lexeme {
                text: "中".into(),
                frequency: 10.0,
                annotation: None,
            }]
        } else {
            Vec::new()
        }
    }
    fn reverse_lookup(&self, _: WubiVersion, character: char) -> Vec<String> {
        if character == '中' {
            vec!["aaaa".into()]
        } else {
            Vec::new()
        }
    }
}
struct PinyinFixture;
impl PinyinLexicon for PinyinFixture {
    fn lookup(&self, _: &[String]) -> Vec<Lexeme> {
        Vec::new()
    }
}
struct DoublePinyinFixture;
impl PinyinLexicon for DoublePinyinFixture {
    fn lookup(&self, syllables: &[String]) -> Vec<Lexeme> {
        if syllables == ["ni"] {
            vec![Lexeme {
                text: "你".into(),
                frequency: 10.0,
                annotation: None,
            }]
        } else {
            Vec::new()
        }
    }
}
fn key(c: char) -> KeyEvent {
    KeyEvent {
        key: Key::Character(c),
        modifiers: Modifiers::default(),
        repeat: false,
    }
}

#[test]
fn commits_only_unique_four_code_exact_match() {
    let mut engine = MixedEngine::new(
        Arc::new(WubiFixture),
        Arc::new(PinyinFixture),
        MixedConfig::default(),
    );
    let mut result = EngineUpdate::Noop;
    for c in "aaaa".chars() {
        result = engine.handle_key(key(c), &Context::default());
    }
    assert!(matches!(result,EngineUpdate::Commit(ref text) if text=="中"));
}

#[test]
fn passes_control_shortcuts_through() {
    let mut engine = MixedEngine::new(
        Arc::new(WubiFixture),
        Arc::new(PinyinFixture),
        MixedConfig::default(),
    );
    let shortcut = KeyEvent {
        key: Key::Character('z'),
        modifiers: Modifiers {
            ctrl: true,
            ..Modifiers::default()
        },
        repeat: false,
    };
    assert!(matches!(
        engine.handle_key(shortcut, &Context::default()),
        EngineUpdate::PassThrough
    ));
}

#[test]
fn lowercase_z_stays_available_for_input() {
    let mut engine = MixedEngine::new(
        Arc::new(WubiFixture),
        Arc::new(PinyinFixture),
        MixedConfig::default(),
    );
    let result = engine.handle_key(key('z'), &Context::default());
    assert!(matches!(result, EngineUpdate::Candidates(ref list)
        if list.preedit == "z" && list.items.iter().all(|candidate| candidate.annotation.as_deref() != Some("五笔反查"))));
}

#[test]
fn z_reverse_looks_up_the_character_before_the_cursor() {
    let mut engine = MixedEngine::new(
        Arc::new(WubiFixture),
        Arc::new(PinyinFixture),
        MixedConfig::default(),
    );
    let context = Context {
        surrounding_text: Some("中".into()),
        cursor_byte: Some(3),
        ..Context::default()
    };
    let result = engine.handle_key(key('Z'), &context);
    assert!(matches!(result, EngineUpdate::Candidates(ref candidates)
        if candidates.items.iter().any(|candidate| candidate.text == "aaaa")));
}

#[test]
fn mixed_engine_commits_literal_input_when_both_dictionaries_miss() {
    let mut engine = MixedEngine::new(
        Arc::new(WubiFixture),
        Arc::new(PinyinFixture),
        MixedConfig::default(),
    );
    let context = Context::default();
    for c in "ni".chars() {
        engine.handle_key(key(c), &context);
    }
    assert!(
        matches!(engine.handle_key(KeyEvent { key:Key::Space, modifiers:Modifiers::default(), repeat:false },&context),
        EngineUpdate::Commit(ref text) if text=="ni")
    );
}

#[test]
fn exact_short_code_ranks_before_high_frequency_completions() {
    let table = Arc::new(wubi::parse_tsv("86\ta\t工\t1\n86\taqi\t区\t9999\n").unwrap());
    let mut engine = MixedEngine::new(table, Arc::new(PinyinFixture), MixedConfig::default());
    let result = engine.handle_key(key('a'), &Context::default());
    assert!(
        matches!(result, EngineUpdate::Candidates(ref list) if list.items.first().is_some_and(|item| item.text == "工"))
    );
}

#[test]
fn mixed_engine_preserves_unpaired_pinyin_initial_after_selection() {
    let config = MixedConfig {
        pinyin_config: pinyin::PinyinConfig {
            double_pinyin: Some(pinyin::DoublePinyinScheme::Xiaohe),
            ..pinyin::PinyinConfig::default()
        },
        ..MixedConfig::default()
    };
    let mut engine = MixedEngine::new(Arc::new(WubiFixture), Arc::new(DoublePinyinFixture), config);
    let context = Context::default();
    for c in "nih".chars() {
        engine.handle_key(key(c), &context);
    }
    assert!(matches!(engine.select_candidate(0), EngineUpdate::Commit(ref text) if text == "你"));
    assert!(engine.is_composing());
    let update = engine.handle_key(key('a'), &context);
    assert!(matches!(update, EngineUpdate::Candidates(ref list) if list.preedit == "ha"));
}

#[test]
fn mixed_engine_commits_buffer_before_overflow_character() {
    let config = MixedConfig {
        max_input_len: 1,
        ..MixedConfig::default()
    };
    let mut engine = MixedEngine::new(Arc::new(WubiFixture), Arc::new(PinyinFixture), config);
    let context = Context::default();
    engine.handle_key(key('n'), &context);
    assert!(
        matches!(engine.handle_key(key('i'), &context), EngineUpdate::Commit(ref text) if text=="ni")
    );
}

#[test]
fn mixed_engine_accepts_natural_code_pairs() {
    let config = MixedConfig {
        pinyin_config: pinyin::PinyinConfig {
            double_pinyin: Some(pinyin::DoublePinyinScheme::NaturalCode),
            ..pinyin::PinyinConfig::default()
        },
        ..MixedConfig::default()
    };
    let mut engine = MixedEngine::new(Arc::new(WubiFixture), Arc::new(PinyinFixture), config);
    let context = Context::default();
    engine.handle_key(key('b'), &context);
    let result = engine.handle_key(key('y'), &context);
    assert!(
        matches!(result, EngineUpdate::Candidates(ref candidates) if candidates.preedit == "by")
    );
}
