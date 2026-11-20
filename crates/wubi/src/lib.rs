use ime_core::{
    Candidate, CandidateList, CandidateSource, Context, EngineUpdate, InputEngine, Key, KeyEvent,
};
use pinyin::{decode, Lexeme, PinyinConfig, PinyinLexicon, SyllableDag};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WubiVersion {
    V86,
    V98,
    NewCentury,
}

pub trait WubiLexicon: Send + Sync {
    fn lookup(&self, version: WubiVersion, code: &str) -> Vec<Lexeme>;
    fn reverse_lookup(&self, _version: WubiVersion, _character: char) -> Vec<String> {
        Vec::new()
    }
}

#[derive(Clone, Debug)]
pub struct MixedConfig {
    pub version: WubiVersion,
    pub page_size: usize,
    pub max_input_len: usize,
    pub auto_commit_unique_four_code: bool,
}
impl Default for MixedConfig {
    fn default() -> Self {
        Self {
            version: WubiVersion::V86,
            page_size: 9,
            max_input_len: 64,
            auto_commit_unique_four_code: true,
        }
    }
}

pub struct MixedEngine {
    wubi: Arc<dyn WubiLexicon>,
    pinyin: Arc<dyn PinyinLexicon>,
    config: MixedConfig,
    buffer: String,
    candidates: CandidateList,
    reverse: bool,
}
impl MixedEngine {
    pub fn new(
        wubi: Arc<dyn WubiLexicon>,
        pinyin: Arc<dyn PinyinLexicon>,
        config: MixedConfig,
    ) -> Self {
        Self {
            wubi,
            pinyin,
            config,
            buffer: String::new(),
            candidates: CandidateList::default(),
            reverse: false,
        }
    }
    fn query(&mut self) -> EngineUpdate {
        let mut items = Vec::new();
        if self.reverse {
            if let Some(ch) = self.buffer.chars().next() {
                items.extend(
                    self.wubi
                        .reverse_lookup(self.config.version, ch)
                        .into_iter()
                        .map(|code| Candidate {
                            text: code,
                            annotation: Some("五笔反查".into()),
                            score: 0.0,
                            source: CandidateSource::Wubi,
                        }),
                );
            }
        } else {
            items.extend(
                self.wubi
                    .lookup(self.config.version, &self.buffer)
                    .into_iter()
                    .filter(|x| !x.text.is_empty())
                    .map(|x| Candidate {
                        text: x.text,
                        annotation: Some(format!("五笔 {}", self.buffer)),
                        score: if x.frequency.is_finite() && x.frequency >= 0.0 {
                            x.frequency
                        } else {
                            0.0
                        },
                        source: CandidateSource::Wubi,
                    }),
            );
            let dag = SyllableDag::from_input(&self.buffer, &PinyinConfig::default());
            items.extend(
                decode(&dag, self.pinyin.as_ref(), 16)
                    .into_iter()
                    .map(|mut x| {
                        x.annotation = Some("拼音".into());
                        x
                    }),
            );
        }
        items.sort_by(|a, b| {
            b.score
                .total_cmp(&a.score)
                .then_with(|| a.text.cmp(&b.text))
        });
        let mut seen = std::collections::HashSet::new();
        items.retain(|x| seen.insert(x.text.clone()));
        if !self.reverse && self.config.auto_commit_unique_four_code && self.buffer.len() == 4 {
            let exact = self.wubi.lookup(self.config.version, &self.buffer);
            if exact.len() == 1 && !exact[0].text.is_empty() {
                let text = exact[0].text.clone();
                self.reset();
                return EngineUpdate::Commit(text);
            }
        }
        self.candidates = CandidateList::new(self.buffer.clone(), items, self.config.page_size);
        EngineUpdate::Candidates(self.candidates.clone())
    }
}
impl InputEngine for MixedEngine {
    fn handle_key(&mut self, event: KeyEvent, ctx: &Context) -> EngineUpdate {
        if ctx.secure {
            self.reset();
            return EngineUpdate::PassThrough;
        }
        if event.modifiers.ctrl || event.modifiers.alt || event.modifiers.super_key {
            return EngineUpdate::PassThrough;
        }
        match event.key {
            Key::Character('Z') if !self.reverse => {
                let selected = self
                    .candidates
                    .items
                    .iter()
                    .find(|c| c.source == CandidateSource::Wubi)
                    .and_then(|c| c.text.chars().next());
                let surrounding = ctx.surrounding_text.as_ref().and_then(|s| {
                    let cursor = ctx.cursor_byte.unwrap_or(s.len()).min(s.len());
                    s.get(..cursor)
                        .and_then(|prefix| prefix.chars().next_back())
                });
                if let Some(ch) = selected.or(surrounding) {
                    self.buffer = ch.to_string();
                    self.reverse = true;
                    self.query()
                } else {
                    self.reverse = true;
                    self.buffer.clear();
                    self.query()
                }
            }
            Key::Character(c) if self.reverse && !c.is_ascii_control() => {
                if self.buffer.len().saturating_add(c.len_utf8()) > self.config.max_input_len {
                    let mut text = self
                        .candidates
                        .select(0)
                        .map(|candidate| candidate.text.clone())
                        .unwrap_or_else(|| self.buffer.clone());
                    text.push(c);
                    self.reset();
                    return EngineUpdate::Commit(text);
                }
                self.buffer.push(c);
                self.query()
            }
            Key::Character(c) if c.is_ascii_alphabetic() => {
                if self.buffer.len() >= self.config.max_input_len {
                    let mut text = self
                        .candidates
                        .select(0)
                        .map(|candidate| candidate.text.clone())
                        .unwrap_or_else(|| self.buffer.clone());
                    text.push(c.to_ascii_lowercase());
                    self.reset();
                    return EngineUpdate::Commit(text);
                }
                self.buffer.push(c.to_ascii_lowercase());
                self.query()
            }
            Key::Backspace if !self.buffer.is_empty() => {
                self.buffer.pop();
                self.query()
            }
            Key::Enter | Key::Space if !self.buffer.is_empty() => self.select_candidate(0),
            Key::Escape if !self.buffer.is_empty() || self.reverse => {
                self.reset();
                EngineUpdate::Noop
            }
            _ if self.buffer.is_empty() => EngineUpdate::PassThrough,
            _ => EngineUpdate::PassThrough,
        }
    }
    fn select_candidate(&mut self, index: usize) -> EngineUpdate {
        if let Some(c) = self.candidates.select(index) {
            let v = c.text.clone();
            self.reset();
            EngineUpdate::Commit(v)
        } else {
            EngineUpdate::Noop
        }
    }
    fn reset(&mut self) {
        self.buffer.clear();
        self.reverse = false;
        self.candidates = CandidateList::default();
    }
}
