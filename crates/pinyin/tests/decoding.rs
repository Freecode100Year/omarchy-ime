use ime_core::{Context, EngineUpdate, InputEngine, Key, KeyEvent, Modifiers};
use pinyin::{decode, Lexeme, PinyinConfig, PinyinEngine, PinyinLexicon, SyllableDag};
use std::sync::Arc;

struct Fixture(Vec<(Vec<String>, Lexeme)>);
impl PinyinLexicon for Fixture {
    fn lookup(&self, syllables: &[String]) -> Vec<Lexeme> {
        self.0
            .iter()
            .filter(|(key, _)| key == syllables)
            .map(|(_, value)| value.clone())
            .collect()
    }
}
fn lexeme(text: &str) -> Lexeme {
    Lexeme {
        text: text.into(),
        frequency: 100.0,
        annotation: None,
    }
}

#[test]
fn decodes_transposed_typo_to_phrase() {
    let lex = Fixture(vec![(vec!["shou".into(), "dao".into()], lexeme("收到"))]);
    let dag = SyllableDag::from_input("shoduao", &PinyinConfig::default());
    assert!(decode(&dag, &lex, 32).iter().any(|c| c.text == "收到"));
}

#[test]
fn exact_candidate_ranks_before_inserted_letter_resegmentations() {
    let lex = Fixture(vec![(vec!["ni".into()], lexeme("你"))]);
    let dag = SyllableDag::from_input("ni", &PinyinConfig::default());
    let candidates = decode(&dag, &lex, 32);
    assert_eq!(
        candidates.first().map(|candidate| candidate.text.as_str()),
        Some("你")
    );
}

#[test]
fn engine_places_exact_candidate_first() {
    let lex = Arc::new(Fixture(vec![(vec!["ni".into()], lexeme("你"))]));
    let mut engine = PinyinEngine::new(lex, PinyinConfig::default());
    let context = Context::default();
    for c in "ni".chars() {
        let update = engine.handle_key(
            KeyEvent {
                key: Key::Character(c),
                modifiers: Modifiers::default(),
                repeat: false,
            },
            &context,
        );
        if c == 'i' {
            let EngineUpdate::Candidates(list) = update else {
                panic!("expected candidate update after the second key");
            };
            assert_eq!(
                list.items.first().map(|candidate| candidate.text.as_str()),
                Some("你")
            );
        }
    }
}

#[test]
fn engine_commits_literal_input_when_dictionary_has_no_match() {
    let lex = Arc::new(Fixture(Vec::new()));
    let mut engine = PinyinEngine::new(lex, PinyinConfig::default());
    let context = Context::default();
    for c in "ni".chars() {
        engine.handle_key(
            KeyEvent {
                key: Key::Character(c),
                modifiers: Modifiers::default(),
                repeat: false,
            },
            &context,
        );
    }
    let result = engine.handle_key(
        KeyEvent {
            key: Key::Space,
            modifiers: Modifiers::default(),
            repeat: false,
        },
        &context,
    );
    assert!(matches!(result, EngineUpdate::Commit(ref text) if text == "ni"));
}

#[test]
fn engine_commits_buffer_before_passing_an_overflowing_character() {
    let lex = Arc::new(Fixture(Vec::new()));
    let config = PinyinConfig {
        max_input_len: 1,
        ..PinyinConfig::default()
    };
    let mut engine = PinyinEngine::new(lex, config);
    let context = Context::default();
    engine.handle_key(
        KeyEvent {
            key: Key::Character('n'),
            modifiers: Modifiers::default(),
            repeat: false,
        },
        &context,
    );
    let result = engine.handle_key(
        KeyEvent {
            key: Key::Character('i'),
            modifiers: Modifiers::default(),
            repeat: false,
        },
        &context,
    );
    assert!(matches!(result, EngineUpdate::Commit(ref text) if text == "ni"));
}

#[test]
fn candidate_scores_stay_finite_for_large_frequencies() {
    let lex = Fixture(vec![
        (
            vec!["ni".into()],
            Lexeme {
                text: "你".into(),
                frequency: f32::MAX,
                annotation: None,
            },
        ),
        (
            vec!["ni".into()],
            Lexeme {
                text: "泥".into(),
                frequency: f32::MAX,
                annotation: None,
            },
        ),
    ]);
    let dag = SyllableDag::from_input("ni", &PinyinConfig::default());
    let result = decode(&dag, &lex, 32);
    assert!(result.iter().all(|candidate| candidate.score.is_finite()));
}

#[test]
fn preserves_best_completed_prefix_and_suffix() {
    let lex = Fixture(vec![(vec!["ni".into()], lexeme("你"))]);
    let dag = SyllableDag::from_input("niq", &PinyinConfig::default());
    let result = decode(&dag, &lex, 32);
    assert!(result.iter().any(|c| c.text == "你q"), "{result:?}");
}

#[test]
fn keeps_case_when_mixing_pinyin_with_english() {
    let lex = Fixture(vec![(vec!["ni".into()], lexeme("你"))]);
    let dag = SyllableDag::from_input("niSQL", &PinyinConfig::default());
    assert!(decode(&dag, &lex, 32).iter().any(|c| c.text == "你SQL"));
}

#[test]
fn public_decoder_handles_ascii_composition_followed_by_utf8_text() {
    let lex = Fixture(vec![(vec!["ni".into()], lexeme("你"))]);
    let dag = SyllableDag::from_input("ni你", &PinyinConfig::default());
    let candidates = decode(&dag, &lex, 8);
    assert!(candidates
        .iter()
        .any(|candidate| candidate.text.contains("你")));
}
