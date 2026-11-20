use crate::{cost::weighted_word_osa, SyllableDag};
use ime_core::{
    Candidate, CandidateList, CandidateSource, Context, EngineUpdate, InputEngine, Key, KeyEvent,
};
use std::{borrow::Cow, sync::Arc};

#[derive(Clone, Debug, PartialEq)]
pub struct Lexeme {
    pub text: String,
    pub frequency: f32,
    pub annotation: Option<String>,
}

pub trait PinyinLexicon: Send + Sync {
    /// Return entries for this complete syllable sequence, including multi-character words.
    fn lookup(&self, syllables: &[String]) -> Vec<Lexeme>;
    /// Borrow immutable rows when the backend can avoid cloning candidate data.
    fn has_prefix(&self, _syllables: &[String]) -> bool {
        false
    }
    fn lookup_cow<'a>(&'a self, syllables: &[String]) -> Cow<'a, [Lexeme]> {
        Cow::Owned(self.lookup(syllables))
    }
    /// Return a log-probability contribution (normally zero or negative).
    fn bigram_score(&self, _previous: &str, _next: &str) -> f32 {
        0.0
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PinyinConfig {
    pub max_edge_cost: f32,
    pub exact_cost: f32,
    pub fuzzy_cost: f32,
    pub beam_width: usize,
    pub page_size: usize,
    pub max_input_len: usize,
}
impl Default for PinyinConfig {
    fn default() -> Self {
        Self {
            max_edge_cost: 1.8,
            exact_cost: 0.01,
            fuzzy_cost: 0.45,
            beam_width: 4,
            page_size: 9,
            max_input_len: 64,
        }
    }
}

#[derive(Clone)]
struct Path {
    pos: usize,
    pending: Vec<String>,
    pending_start: usize,
    pending_cost: f32,
    text: String,
    last_word: String,
    annotation: Option<String>,
    score: f32,
    correction_cost: f32,
}

pub fn decode(dag: &SyllableDag, lexicon: &dyn PinyinLexicon, beam_width: usize) -> Vec<Candidate> {
    decode_with_limits(dag, lexicon, beam_width, beam_width)
}

/// Decode with independent search-state and visible-candidate limits.
pub fn decode_with_limits(
    dag: &SyllableDag,
    lexicon: &dyn PinyinLexicon,
    beam_width: usize,
    candidate_limit: usize,
) -> Vec<Candidate> {
    let candidate_limit = candidate_limit.max(1);
    let mut found = decode_segment(dag, lexicon, beam_width, candidate_limit);
    let bytes = dag.original.as_bytes();
    // Let one unknown English span compete with one decoded pinyin span, retaining
    // original case on the literal side (for example `SQLni` -> `SQL你`).
    let boundaries: Vec<usize> = bytes
        .windows(2)
        .enumerate()
        .filter_map(|(i, pair)| {
            let a = pair[0].is_ascii_lowercase();
            let b = pair[1].is_ascii_lowercase();
            (pair[0].is_ascii_alphabetic() && pair[1].is_ascii_alphabetic() && a != b)
                .then_some(i + 1)
        })
        .take(4)
        .collect();
    for split in boundaries {
        let left = SyllableDag::from_input(&dag.original[..split], &dag.config);
        let right = SyllableDag::from_input(&dag.original[split..], &dag.config);
        let left_items = decode_segment(&left, lexicon, beam_width, candidate_limit);
        let right_items = decode_segment(&right, lexicon, beam_width, candidate_limit);
        for l in &left_items {
            for r in &right_items {
                let left_english = l.annotation.as_deref() == Some("英文");
                let right_english = r.annotation.as_deref() == Some("英文");
                if left_english == right_english {
                    continue;
                }
                found.push(Candidate {
                    text: format!("{}{}", l.text, r.text),
                    annotation: Some("中英混输".into()),
                    score: l.score + r.score,
                    source: CandidateSource::Pinyin,
                });
            }
        }
    }
    found.sort_by(|a, b| {
        let incomplete_a = a.annotation.as_deref() == Some("未完成输入");
        let incomplete_b = b.annotation.as_deref() == Some("未完成输入");
        let corrected_a = a.annotation.as_deref() == Some("纠错输入");
        let corrected_b = b.annotation.as_deref() == Some("纠错输入");
        incomplete_a
            .cmp(&incomplete_b)
            .then_with(|| corrected_a.cmp(&corrected_b))
            .then_with(|| b.score.total_cmp(&a.score))
            .then_with(|| a.text.cmp(&b.text))
    });
    let mut seen = std::collections::HashSet::new();
    found.retain(|c| c.score.is_finite() && seen.insert(c.text.clone()));
    found.truncate(candidate_limit);
    found
}

fn decode_segment(
    dag: &SyllableDag,
    lexicon: &dyn PinyinLexicon,
    beam_width: usize,
    candidate_limit: usize,
) -> Vec<Candidate> {
    let n = dag.input.len();
    let mut beams: Vec<Vec<Path>> = vec![Vec::new(); n + 1];
    let mut best_completed: Vec<Option<Path>> = vec![None; n + 1];
    let mut corrected_completed: Vec<Candidate> = Vec::new();
    let mut exact_suffix = vec![false; n + 1];
    exact_suffix[n] = true;
    for pos in (0..n).rev() {
        exact_suffix[pos] = dag.edges[pos]
            .iter()
            .any(|edge| edge.cost <= dag.config.exact_cost && exact_suffix[edge.end]);
    }
    beams[0].push(Path {
        pos: 0,
        pending: Vec::new(),
        pending_start: 0,
        pending_cost: 0.0,
        text: String::new(),
        last_word: String::new(),
        annotation: None,
        score: 0.0,
        correction_cost: 0.0,
    });
    for pos in 0..n {
        if beams[pos].is_empty() {
            continue;
        }
        let paths = std::mem::take(&mut beams[pos]);
        let mut touched = Vec::new();
        let mut was_touched = vec![false; n + 1];
        for path in paths {
            for edge in dag.edges.get(pos).into_iter().flatten() {
                let mut next = path.clone();
                next.pos = edge.end;
                next.score -= edge.cost * 12.0;
                if next.pending.is_empty() {
                    next.pending_start = pos;
                }
                next.pending_cost += edge.cost;
                next.pending.push(edge.syllable.clone());
                beams[edge.end].push(next.clone());
                if !was_touched[edge.end] {
                    touched.push(edge.end);
                    was_touched[edge.end] = true;
                }
                let entries = lexicon.lookup_cow(&next.pending);
                let total_frequency: f64 = entries
                    .iter()
                    .filter(|item| {
                        !item.text.is_empty() && item.frequency.is_finite() && item.frequency >= 0.0
                    })
                    .map(|item| item.frequency.max(0.000_001) as f64)
                    .sum();
                for item in entries.iter() {
                    if item.text.is_empty() || !item.frequency.is_finite() || item.frequency < 0.0 {
                        continue;
                    }
                    let bigram = lexicon.bigram_score(&next.last_word, &item.text);
                    if !bigram.is_finite() {
                        continue;
                    }
                    let conditional_frequency = (item.frequency.max(0.000_001) as f64
                        / total_frequency.max(f64::MIN_POSITIVE))
                    .max(f64::MIN_POSITIVE);
                    let word_score = conditional_frequency.ln() as f32 + bigram.min(0.0);
                    if !word_score.is_finite() {
                        continue;
                    }
                    let mut completed = next.clone();
                    let target: String = next.pending.concat();
                    let observed = &dag.input.as_bytes()[next.pending_start..edge.end];
                    let word_cost = weighted_word_osa(observed, target.as_bytes(), 2)
                        .unwrap_or(next.pending_cost)
                        .min(next.pending_cost);
                    completed.correction_cost = path.correction_cost + word_cost;
                    completed.score += (next.pending_cost - word_cost) * 12.0;
                    completed.text.push_str(&item.text);
                    completed.last_word = item.text.clone();
                    completed.annotation = item.annotation.clone();
                    completed.pending.clear();
                    completed.pending_start = edge.end;
                    completed.pending_cost = 0.0;
                    completed.score += word_score;
                    if !completed.score.is_finite() {
                        continue;
                    }
                    if edge.end == n && completed.correction_cost > dag.config.exact_cost {
                        corrected_completed.push(Candidate {
                            text: completed.text.clone(),
                            annotation: Some("纠错输入".into()),
                            score: completed.score,
                            source: CandidateSource::Pinyin,
                        });
                        corrected_completed.sort_by(|a, b| b.score.total_cmp(&a.score));
                        corrected_completed.dedup_by(|a, b| a.text == b.text);
                        corrected_completed.truncate(candidate_limit);
                    }
                    if edge.end < n {
                        let replace = best_completed[edge.end].as_ref().is_none_or(|current| {
                            completed.correction_cost < current.correction_cost
                                || (completed.correction_cost == current.correction_cost
                                    && completed.score > current.score)
                        });
                        if replace {
                            best_completed[edge.end] = Some(completed.clone());
                        }
                    }
                    beams[edge.end].push(completed);
                }
            }
        }
        // Only beams reached by this position changed. Sorting every input slot
        // on every iteration made decoding quadratic in input length and wasted
        // most work on empty or already-consumed beams.
        for index in touched {
            let b = &mut beams[index];
            b.sort_by(|a, b| {
                let complete_a = index == n && a.pending.is_empty() && !a.text.is_empty();
                let complete_b = index == n && b.pending.is_empty() && !b.text.is_empty();
                complete_b
                    .cmp(&complete_a)
                    .then_with(|| a.correction_cost.total_cmp(&b.correction_cost))
                    .then_with(|| b.score.total_cmp(&a.score))
                    .then_with(|| a.text.cmp(&b.text))
                    .then_with(|| a.pending.len().cmp(&b.pending.len()))
            });
            let keep = if index == n {
                candidate_limit + beam_width.max(1)
            } else {
                beam_width.max(1)
            };
            let mut retained: Vec<Path> = b.iter().take(keep).cloned().collect();
            let mut prefix_paths: Vec<Path> = b
                .iter()
                .filter(|path| path.pending.len() == 1 && lexicon.has_prefix(&path.pending))
                .cloned()
                .collect();
            prefix_paths.sort_by(|a, b| {
                b.pending[0]
                    .len()
                    .cmp(&a.pending[0].len())
                    .then_with(|| {
                        (a.correction_cost + a.pending_cost)
                            .total_cmp(&(b.correction_cost + b.pending_cost))
                    })
                    .then_with(|| b.score.total_cmp(&a.score))
            });
            for path in prefix_paths.into_iter().take(4) {
                if !retained.iter().any(|kept| {
                    kept.pos == path.pos && kept.pending == path.pending && kept.text == path.text
                }) {
                    retained.push(path);
                }
            }
            *b = retained;
        }
    }
    let mut found: Vec<Candidate> = beams[n]
        .iter()
        .filter(|p| {
            p.pending.is_empty()
                && !p.text.is_empty()
                && p.score.is_finite()
                && p.correction_cost <= dag.config.exact_cost
        })
        .map(|p| Candidate {
            text: p.text.clone(),
            annotation: p.annotation.clone(),
            score: p.score,
            source: CandidateSource::Pinyin,
        })
        .collect();
    found.truncate(candidate_limit);
    for corrected in corrected_completed {
        if found.len() >= candidate_limit {
            break;
        }
        if !found.iter().any(|item| item.text == corrected.text) {
            found.push(corrected);
        }
    }
    if !dag.input.is_empty() {
        let prefix = best_completed
            .iter()
            .take(n)
            .flatten()
            .filter(|p| !p.text.is_empty() && !exact_suffix[p.pos])
            .min_by(|a, b| {
                a.correction_cost
                    .total_cmp(&b.correction_cost)
                    .then_with(|| b.score.total_cmp(&a.score))
            });
        if let Some(path) = prefix {
            let mut text = path.text.clone();
            if path.pos < dag.original.len() {
                text.push_str(&dag.original[path.pos..]);
            }
            found.push(Candidate {
                text,
                annotation: Some("未完成输入".into()),
                score: path.score,
                source: CandidateSource::Pinyin,
            });
        } else if found.is_empty() {
            found.push(Candidate {
                text: dag.original.clone(),
                annotation: Some(
                    if dag.english.iter().all(|v| *v) {
                        "英文"
                    } else {
                        "原样"
                    }
                    .into(),
                ),
                score: 0.0,
                source: CandidateSource::Pinyin,
            });
        }
    }
    found.sort_by(|a, b| {
        let incomplete_a = a.annotation.as_deref() == Some("未完成输入");
        let incomplete_b = b.annotation.as_deref() == Some("未完成输入");
        incomplete_a
            .cmp(&incomplete_b)
            .then_with(|| b.score.total_cmp(&a.score))
            .then_with(|| a.text.cmp(&b.text))
    });
    let mut seen = std::collections::HashSet::new();
    found.retain(|c| seen.insert(c.text.clone()));
    found.truncate(candidate_limit);
    found
}

pub struct PinyinEngine {
    pub config: PinyinConfig,
    lexicon: Arc<dyn PinyinLexicon>,
    buffer: String,
    candidates: CandidateList,
}
impl PinyinEngine {
    pub fn new(lexicon: Arc<dyn PinyinLexicon>, config: PinyinConfig) -> Self {
        Self {
            config,
            lexicon,
            buffer: String::new(),
            candidates: CandidateList::default(),
        }
    }
    pub fn preedit(&self) -> &str {
        &self.buffer
    }
    fn refresh(&mut self) -> EngineUpdate {
        if self.buffer.is_empty() {
            self.candidates = CandidateList::default();
            return EngineUpdate::Noop;
        }
        let dag = SyllableDag::from_input(&self.buffer, &self.config);
        let items = decode_with_limits(
            &dag,
            self.lexicon.as_ref(),
            self.config.beam_width,
            self.config.page_size,
        );
        self.candidates = CandidateList::new(self.buffer.clone(), items, self.config.page_size);
        EngineUpdate::Candidates(self.candidates.clone())
    }
}
impl InputEngine for PinyinEngine {
    fn handle_key(&mut self, event: KeyEvent, ctx: &Context) -> EngineUpdate {
        if ctx.secure {
            self.reset();
            return EngineUpdate::PassThrough;
        }
        match event.key {
            Key::Character(c)
                if c.is_ascii_alphabetic()
                    && !event.modifiers.ctrl
                    && !event.modifiers.alt
                    && !event.modifiers.super_key =>
            {
                if self.buffer.len() >= self.config.max_input_len {
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
                self.refresh()
            }
            Key::Backspace
                if !self.buffer.is_empty()
                    && !event.modifiers.ctrl
                    && !event.modifiers.alt
                    && !event.modifiers.super_key =>
            {
                self.buffer.pop();
                self.refresh()
            }
            Key::Enter | Key::Space
                if !self.buffer.is_empty()
                    && !event.modifiers.ctrl
                    && !event.modifiers.alt
                    && !event.modifiers.super_key =>
            {
                self.select_candidate(0)
            }
            Key::Escape
                if !self.buffer.is_empty()
                    && !event.modifiers.ctrl
                    && !event.modifiers.alt
                    && !event.modifiers.super_key =>
            {
                self.reset();
                EngineUpdate::Noop
            }
            _ if self.buffer.is_empty() => EngineUpdate::PassThrough,
            _ => EngineUpdate::PassThrough,
        }
    }
    fn select_candidate(&mut self, index: usize) -> EngineUpdate {
        if let Some(candidate) = self.candidates.select(index) {
            let text = candidate.text.clone();
            self.reset();
            EngineUpdate::Commit(text)
        } else {
            EngineUpdate::Noop
        }
    }
    fn reset(&mut self) {
        self.buffer.clear();
        self.candidates = CandidateList::default();
    }
}
