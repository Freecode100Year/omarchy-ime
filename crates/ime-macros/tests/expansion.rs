use chrono::Timelike;
use ime_core::{Context, EngineUpdate, InputEngine, Key, KeyEvent, Modifiers};
use ime_macros::MacroTable;

#[test]
fn expands_local_date_time_and_multiline_macro_text() {
    let table = MacroTable::parse(
        r#"
[[macro]]
trigger = ";;stamp"
text = "${date} ${time}\n完成"
position = 0
"#,
    )
    .unwrap();
    let now = chrono::Local::now()
        .with_hour(12)
        .unwrap()
        .with_minute(34)
        .unwrap()
        .with_second(56)
        .unwrap();
    let result = table.render_at(";;stamp", now);
    assert!(result.unwrap().contains("12:34:56\n完成"));
}

#[test]
fn rejects_empty_or_duplicate_triggers() {
    assert!(MacroTable::parse("[[macro]]\ntrigger=''\ntext='x'").is_err());
    assert!(MacroTable::parse(
        "[[macro]]\ntrigger='x'\ntext='a'\n[[macro]]\ntrigger='x'\ntext='b'"
    )
    .is_err());
    assert!(MacroTable::parse("[[macro]]\ntrigger='你好'\ntext='词'").is_err());
    assert!(MacroTable::parse("[[macro]]\ntrigger='date'\ntext='日期'").is_err());
}

#[test]
fn installed_macro_template_parses_and_exposes_date_trigger() {
    let table = MacroTable::parse(include_str!("../../../config/macros.toml")).unwrap();
    assert!(table.has_prefix(";;date"));
    assert!(table.render_at(";;date", chrono::Local::now()).is_some());
}

#[test]
fn digit_continuations_are_consumed_when_part_of_a_macro_trigger() {
    let table = MacroTable::parse("[[macro]]\ntrigger=\":1\"\ntext=\"one\"\n").unwrap();
    let mut engine = ime_macros::MacroEngine::new(Box::new(Passthrough), table, 9);
    let ctx = Context::default();
    let event = |c| KeyEvent {
        key: Key::Character(c),
        modifiers: Modifiers::default(),
        repeat: false,
    };
    assert!(matches!(
        engine.handle_key(event(':'), &ctx),
        EngineUpdate::Candidates(_)
    ));
    assert!(engine.wants_key(Key::Character('1')));
    assert!(matches!(
        engine.handle_key(event('1'), &ctx),
        EngineUpdate::Candidates(_)
    ));
    assert!(matches!(engine.select_candidate(0), EngineUpdate::Commit(ref text) if text == "one"));
}

struct Passthrough;
impl InputEngine for Passthrough {
    fn handle_key(&mut self, _: KeyEvent, _: &Context) -> EngineUpdate {
        EngineUpdate::PassThrough
    }
    fn select_candidate(&mut self, _: usize) -> EngineUpdate {
        EngineUpdate::Noop
    }
    fn reset(&mut self) {}
}

#[test]
fn macro_engine_shows_then_commits_multiline_expansion() {
    let table = MacroTable::parse("[[macro]]\ntrigger=';;x'\ntext='first\\nsecond'").unwrap();
    let mut engine = ime_macros::MacroEngine::new(Box::new(Passthrough), table, 9);
    let context = Context::default();
    let event = |key| KeyEvent {
        key,
        modifiers: Modifiers::default(),
        repeat: false,
    };
    for (index, c) in ";;x".chars().enumerate() {
        engine.handle_key(event(Key::Character(c)), &context);
        if index == 0 {
            assert!(engine.wants_key(Key::Character(';')));
        }
    }
    assert!(
        matches!(engine.select_candidate(0), EngineUpdate::Commit(ref text) if text == "first\nsecond")
    );
}
