use ime_core::{
    Candidate, CandidateList, CandidateSource, Context, EngineUpdate, InputEngine, Key, KeyEvent,
};
use pinyin::{decode, Lexeme, PinyinConfig, PinyinLexicon, SyllableDag};
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, OnceLock},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WubiVersion {
    V86,
    V98,
    NewCentury,
}

pub trait WubiLexicon: Send + Sync {
    fn lookup(&self, version: WubiVersion, code: &str) -> Vec<Lexeme>;
    fn lookup_exact(&self, version: WubiVersion, code: &str) -> Vec<Lexeme> {
        self.lookup(version, code)
    }
    fn reverse_lookup(&self, _version: WubiVersion, _character: char) -> Vec<String> {
        Vec::new()
    }
}

/// In-memory index for local, versioned TSV code tables. The caller may mmap or
/// parse the source at startup; key processing performs no I/O.
#[derive(Default)]
pub struct StaticWubiLexicon {
    rows: Vec<((WubiVersion, String), Vec<Lexeme>)>,
    prefixes: HashMap<(WubiVersion, String), Vec<usize>>,
    reverse: OnceLock<HashMap<(WubiVersion, char), Vec<String>>>,
}

impl WubiLexicon for StaticWubiLexicon {
    fn lookup(&self, version: WubiVersion, code: &str) -> Vec<Lexeme> {
        let mut rows = self.lookup_exact(version, code);
        if let Some(indices) = self.prefixes.get(&(version, code.to_owned())) {
            for index in indices {
                if self.rows[*index].0 .1 == code {
                    continue;
                }
                rows.extend(self.rows[*index].1.iter().cloned());
            }
        }
        rows.sort_by(|a, b| {
            b.frequency
                .total_cmp(&a.frequency)
                .then_with(|| a.text.cmp(&b.text))
        });
        rows
    }
    fn lookup_exact(&self, version: WubiVersion, code: &str) -> Vec<Lexeme> {
        self.rows
            .binary_search_by(|((v, key), _)| (*v, key.as_str()).cmp(&(version, code)))
            .ok()
            .map(|index| self.rows[index].1.clone())
            .unwrap_or_default()
    }
    fn reverse_lookup(&self, version: WubiVersion, character: char) -> Vec<String> {
        self.reverse
            .get_or_init(|| {
                let mut reverse: HashMap<(WubiVersion, char), HashSet<String>> = HashMap::new();
                for ((version, code), entries) in &self.rows {
                    for entry in entries {
                        let mut chars = entry.text.chars();
                        if let (Some(ch), None) = (chars.next(), chars.next()) {
                            reverse
                                .entry((*version, ch))
                                .or_default()
                                .insert(code.clone());
                        }
                    }
                }
                reverse
                    .into_iter()
                    .map(|(key, codes)| {
                        let mut codes: Vec<_> = codes.into_iter().collect();
                        codes.sort_unstable();
                        (key, codes)
                    })
                    .collect()
            })
            .get(&(version, character))
            .cloned()
            .unwrap_or_default()
    }
}

/// Parses `version<TAB>code<TAB>text<TAB>frequency` local dictionary rows.
/// Version is exactly `86`, `98`, or `new-century`; blank lines and comments
/// are ignored. An empty table is rejected rather than silently disabling IME.
pub fn parse_tsv(input: &str) -> Result<StaticWubiLexicon, String> {
    parse_tsv_for_version(input, None)
}

/// Parses only the selected code table, reducing startup time and resident
/// memory when a combined 86/98 file is shipped.
pub fn parse_tsv_for_version(
    input: &str,
    selected_version: Option<WubiVersion>,
) -> Result<StaticWubiLexicon, String> {
    let mut result = StaticWubiLexicon::default();
    let mut rows: HashMap<(WubiVersion, String), Vec<Lexeme>> = HashMap::new();
    for (line_no, line) in input.lines().enumerate() {
        let line = line.trim_end_matches('\r');
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        let fields: Vec<_> = line.split('\t').collect();
        if fields.len() != 4 {
            return Err(format!(
                "line {}: expected four tab-separated columns",
                line_no + 1
            ));
        }
        let version = match fields[0] {
            "86" => WubiVersion::V86,
            "98" => WubiVersion::V98,
            "new-century" => WubiVersion::NewCentury,
            _ => return Err(format!("line {}: unsupported Wubi version", line_no + 1)),
        };
        if selected_version.is_some_and(|selected| selected != version) {
            continue;
        }
        let code = fields[1].to_ascii_lowercase();
        if code.is_empty()
            || code.len() > 64
            || !code.bytes().all(|b| b.is_ascii_lowercase())
            || fields[2].is_empty()
        {
            return Err(format!("line {}: code/text must be non-empty", line_no + 1));
        }
        let frequency: f32 = fields[3]
            .parse()
            .map_err(|_| format!("line {}: invalid frequency", line_no + 1))?;
        if !frequency.is_finite() || frequency < 0.0 {
            return Err(format!(
                "line {}: frequency must be finite and non-negative",
                line_no + 1
            ));
        }
        let lexeme = Lexeme {
            text: fields[2].to_owned(),
            frequency,
            annotation: None,
        };
        rows.entry((version, code.clone()))
            .or_default()
            .push(lexeme);
    }
    if rows.is_empty() {
        return Err("Wubi dictionary has no entries".into());
    }
    result.rows = rows.into_iter().collect();
    result.rows.sort_unstable_by(|a, b| a.0.cmp(&b.0));
    for (index, ((version, code), _)) in result.rows.iter().enumerate() {
        for end in 1..=code.len() {
            result
                .prefixes
                .entry((*version, code[..end].to_owned()))
                .or_default()
                .push(index);
        }
    }
    for indices in result.prefixes.values_mut() {
        indices.sort_unstable_by(|left, right| {
            let left_row = &result.rows[*left];
            let right_row = &result.rows[*right];
            let left_freq = left_row
                .1
                .iter()
                .map(|item| item.frequency)
                .fold(0.0, f32::max);
            let right_freq = right_row
                .1
                .iter()
                .map(|item| item.frequency)
                .fold(0.0, f32::max);
            right_freq
                .total_cmp(&left_freq)
                .then_with(|| left_row.0.cmp(&right_row.0))
        });
        indices.truncate(64);
    }
    Ok(result)
}

#[derive(Clone, Debug)]
pub struct MixedConfig {
    pub version: WubiVersion,
    pub page_size: usize,
    pub max_input_len: usize,
    pub auto_commit_unique_four_code: bool,
    pub pinyin_config: PinyinConfig,
}
impl Default for MixedConfig {
    fn default() -> Self {
        Self {
            version: WubiVersion::V86,
            page_size: 9,
            max_input_len: 64,
            auto_commit_unique_four_code: true,
            pinyin_config: PinyinConfig::default(),
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
        let exact_codes: HashSet<String> = if self.reverse {
            HashSet::new()
        } else {
            self.wubi
                .lookup_exact(self.config.version, &self.buffer)
                .into_iter()
                .map(|item| item.text)
                .collect()
        };
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
            let pinyin_input = self
                .config
                .pinyin_config
                .double_pinyin
                .and_then(|scheme| pinyin::expand_double_pinyin(&self.buffer, scheme))
                .unwrap_or_else(|| {
                    if self.config.pinyin_config.double_pinyin.is_some() {
                        String::new()
                    } else {
                        self.buffer.clone()
                    }
                });
            let dag = SyllableDag::from_input(&pinyin_input, &self.config.pinyin_config);
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
            let a_exact = a.source == CandidateSource::Wubi && exact_codes.contains(&a.text);
            let b_exact = b.source == CandidateSource::Wubi && exact_codes.contains(&b.text);
            b_exact.cmp(&a_exact).then_with(|| {
                b.score
                    .total_cmp(&a.score)
                    .then_with(|| a.text.cmp(&b.text))
            })
        });
        let mut seen = std::collections::HashSet::new();
        items.retain(|x| seen.insert(x.text.clone()));
        if !self.reverse && self.config.auto_commit_unique_four_code && self.buffer.len() == 4 {
            let exact = self.wubi.lookup_exact(self.config.version, &self.buffer);
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
            Key::Enter | Key::Space if !self.buffer.is_empty() => {
                if self.config.pinyin_config.double_pinyin.is_some()
                    && self.buffer.len() % 2 == 1
                    && self.candidates.items.is_empty()
                {
                    EngineUpdate::PassThrough
                } else {
                    self.select_candidate(0)
                }
            }
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
            let pending = if c.source == CandidateSource::Pinyin
                && self.config.pinyin_config.double_pinyin.is_some()
                && self.buffer.len() % 2 == 1
            {
                self.buffer
                    .chars()
                    .last()
                    .map(|ch| ch.to_string())
                    .unwrap_or_default()
            } else {
                String::new()
            };
            self.buffer = pending;
            self.reverse = false;
            self.candidates = CandidateList::default();
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
    fn is_composing(&self) -> bool {
        !self.buffer.is_empty() || self.reverse
    }
    fn preedit(&self) -> &str {
        &self.buffer
    }
    fn flush(&mut self) -> EngineUpdate {
        if !self.candidates.items.is_empty() {
            return self.select_candidate(0);
        }
        if !self.buffer.is_empty() {
            let text = std::mem::take(&mut self.buffer);
            self.reverse = false;
            self.candidates = CandidateList::default();
            return EngineUpdate::Commit(text);
        }
        EngineUpdate::PassThrough
    }
    fn wants_key(&self, key: Key) -> bool {
        self.reverse
            && self.buffer.is_empty()
            && matches!(key, Key::Character(ch) if !ch.is_ascii_digit())
    }
}
