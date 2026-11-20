//! Local TOML macro definitions and time expansion.
use chrono::{DateTime, Local};
use ime_core::{Candidate, CandidateSource, Context, EngineUpdate, InputEngine, Key, KeyEvent};
use serde::Deserialize;
use std::collections::HashSet;

#[derive(Clone, Debug, Deserialize)]
pub struct MacroDefinition {
    pub trigger: String,
    pub text: String,
    #[serde(default)]
    pub position: usize,
}

#[derive(Clone, Debug, Default)]
pub struct MacroTable {
    entries: Vec<MacroDefinition>,
}

#[derive(Deserialize)]
struct Config {
    #[serde(rename = "macro", default)]
    entries: Vec<MacroDefinition>,
}

impl MacroTable {
    pub fn parse(input: &str) -> Result<Self, String> {
        let config: Config = toml::from_str(input).map_err(|error| error.to_string())?;
        let mut triggers = HashSet::new();
        for entry in &config.entries {
            if entry.trigger.is_empty() || entry.text.is_empty() {
                return Err("macro trigger and text must not be empty".into());
            }
            if entry.trigger.len() > 64 || entry.text.len() > 16 * 1024 || entry.position > 1024 {
                return Err("macro trigger or expansion exceeds the size limit".into());
            }
            if !entry.trigger.bytes().all(|byte| byte.is_ascii_graphic())
                || !entry
                    .trigger
                    .as_bytes()
                    .first()
                    .is_some_and(u8::is_ascii_punctuation)
                || entry.text.contains('\0')
            {
                return Err(
                    "macro triggers must start with punctuation, use printable ASCII, and text must not contain NUL".into(),
                );
            }
            if !triggers.insert(entry.trigger.clone()) {
                return Err(format!("duplicate macro trigger: {}", entry.trigger));
            }
        }
        Ok(Self {
            entries: config.entries,
        })
    }

    pub fn has_prefix(&self, input: &str) -> bool {
        self.entries
            .iter()
            .any(|entry| entry.trigger.starts_with(input))
    }

    pub fn matches(&self, input: &str) -> Vec<&MacroDefinition> {
        let mut found: Vec<_> = self
            .entries
            .iter()
            .filter(|entry| entry.trigger == input)
            .collect();
        found.sort_by_key(|entry| entry.position);
        found
    }

    pub fn matching_prefix(&self, input: &str) -> Vec<&MacroDefinition> {
        let mut found: Vec<_> = self
            .entries
            .iter()
            .filter(|entry| entry.trigger.starts_with(input))
            .collect();
        found.sort_by_key(|entry| entry.position);
        found
    }

    pub fn render_at(&self, trigger: &str, now: DateTime<Local>) -> Option<String> {
        let entry = self.matches(trigger).into_iter().next()?;
        Some(
            entry
                .text
                .replace("${datetime}", &now.format("%Y-%m-%d %H:%M:%S").to_string())
                .replace("${date}", &now.format("%Y-%m-%d").to_string())
                .replace("${time}", &now.format("%H:%M:%S").to_string())
                .replace("\\n", "\n"),
        )
    }
}

/// Decorates any input engine with a bounded trigger buffer and local macros.
pub struct MacroEngine {
    inner: Box<dyn InputEngine>,
    table: MacroTable,
    page_size: usize,
    buffer: String,
    visible: Vec<String>,
}

impl MacroEngine {
    pub fn new(inner: Box<dyn InputEngine>, table: MacroTable, page_size: usize) -> Self {
        Self {
            inner,
            table,
            page_size: page_size.clamp(1, 9),
            buffer: String::new(),
            visible: Vec::new(),
        }
    }

    fn update_macros(&mut self) -> EngineUpdate {
        let now = Local::now();
        let definitions = self.table.matching_prefix(&self.buffer);
        let mut items = Vec::new();
        self.visible.clear();
        for entry in definitions {
            if let Some(text) = self.table.render_at(&entry.trigger, now) {
                self.visible.push(text.clone());
                items.push(Candidate {
                    text,
                    annotation: Some(entry.trigger.clone()),
                    score: -(entry.position as f32),
                    source: CandidateSource::Macro,
                });
            }
        }
        EngineUpdate::Candidates(ime_core::CandidateList::new(
            self.buffer.clone(),
            items,
            self.page_size,
        ))
    }
}

impl InputEngine for MacroEngine {
    fn handle_key(&mut self, event: KeyEvent, ctx: &Context) -> EngineUpdate {
        if ctx.secure {
            self.reset();
            return EngineUpdate::PassThrough;
        }
        if event.modifiers.ctrl || event.modifiers.alt || event.modifiers.super_key {
            if !self.buffer.is_empty() {
                self.reset();
            }
            return self.inner.handle_key(event, ctx);
        }
        match event.key {
            Key::Character(ch) if ch.is_ascii_graphic() => {
                let mut proposed = self.buffer.clone();
                proposed.push(ch);
                if (self.buffer.is_empty() && !self.inner.is_composing() || !self.buffer.is_empty())
                    && self.table.has_prefix(&proposed)
                {
                    self.buffer = proposed;
                    self.update_macros()
                } else if self.buffer.is_empty() {
                    self.inner.handle_key(event, ctx)
                } else {
                    self.reset();
                    EngineUpdate::Commit(proposed)
                }
            }
            Key::Backspace if !self.buffer.is_empty() => {
                self.buffer.pop();
                if self.buffer.is_empty() {
                    self.visible.clear();
                    EngineUpdate::Noop
                } else {
                    self.update_macros()
                }
            }
            Key::Escape if !self.buffer.is_empty() => {
                self.reset();
                EngineUpdate::Noop
            }
            Key::Space | Key::Enter if !self.buffer.is_empty() => {
                if self.table.matches(&self.buffer).is_empty() {
                    let mut text = std::mem::take(&mut self.buffer);
                    self.visible.clear();
                    if matches!(event.key, Key::Space) {
                        text.push(' ');
                    }
                    EngineUpdate::Commit(text)
                } else {
                    self.select_candidate(0)
                }
            }
            _ if !self.buffer.is_empty() => self.select_candidate(0),
            _ => self.inner.handle_key(event, ctx),
        }
    }

    fn select_candidate(&mut self, index: usize) -> EngineUpdate {
        if !self.buffer.is_empty() {
            if let Some(text) = self.visible.get(index).cloned() {
                self.reset();
                return EngineUpdate::Commit(text);
            }
            return EngineUpdate::Noop;
        }
        self.inner.select_candidate(index)
    }

    fn reset(&mut self) {
        self.buffer.clear();
        self.visible.clear();
        self.inner.reset();
    }

    fn is_composing(&self) -> bool {
        !self.buffer.is_empty() || self.inner.is_composing()
    }
    fn preedit(&self) -> &str {
        if self.buffer.is_empty() {
            self.inner.preedit()
        } else {
            &self.buffer
        }
    }
    fn flush(&mut self) -> EngineUpdate {
        if !self.buffer.is_empty() {
            if !self.table.matches(&self.buffer).is_empty() {
                return self.select_candidate(0);
            }
            let text = std::mem::take(&mut self.buffer);
            self.visible.clear();
            return EngineUpdate::Commit(text);
        }
        self.inner.flush()
    }

    fn wants_key(&self, key: Key) -> bool {
        if self.buffer.is_empty() && self.inner.is_composing() {
            return self.inner.wants_key(key);
        }
        let Key::Character(ch) = key else {
            return false;
        };
        if !ch.is_ascii_graphic() {
            return false;
        }
        if self.buffer.is_empty() && self.inner.is_composing() {
            return false;
        }
        let mut proposed = self.buffer.clone();
        proposed.push(ch);
        self.table.has_prefix(&proposed)
    }
}
