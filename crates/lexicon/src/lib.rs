//! FST backed immutable lookup adapters. The FST stores key -> row index;
//! candidate payloads stay in a compact side table.
use fst::{Map, MapBuilder};
use pinyin::{Lexeme, PinyinLexicon};
use std::io;
use wubi::{WubiLexicon, WubiVersion};

#[derive(Debug)]
pub enum BuildError {
    Fst(fst::Error),
    UnsortedOrDuplicateKey,
    EmptyKeyOrCandidate,
    InvalidFrequency,
}
impl From<fst::Error> for BuildError {
    fn from(e: fst::Error) -> Self {
        Self::Fst(e)
    }
}

pub struct FstPinyinLexicon {
    map: Map<Vec<u8>>,
    rows: Vec<(String, Vec<Lexeme>)>,
}
impl FstPinyinLexicon {
    /// Build from sorted (unit-separator-delimited syllable key, candidate) pairs.
    pub fn from_sorted(input: Vec<(String, Lexeme)>) -> Result<Self, BuildError> {
        let mut rows: Vec<(String, Vec<Lexeme>)> = Vec::new();
        for (key, item) in input {
            if key.is_empty() || item.text.is_empty() {
                return Err(BuildError::EmptyKeyOrCandidate);
            }
            if !item.frequency.is_finite() || item.frequency < 0.0 {
                return Err(BuildError::InvalidFrequency);
            }
            if let Some((prev, items)) = rows.last_mut() {
                if prev == &key {
                    items.push(item);
                    continue;
                }
                if prev.as_str() > key.as_str() {
                    return Err(BuildError::UnsortedOrDuplicateKey);
                }
            }
            rows.push((key, vec![item]));
        }
        let mut builder = MapBuilder::memory();
        for (i, (key, _)) in rows.iter().enumerate() {
            builder.insert(key.as_bytes(), i as u64)?;
        }
        Ok(Self {
            map: builder.into_map(),
            rows,
        })
    }
}
impl PinyinLexicon for FstPinyinLexicon {
    fn lookup(&self, syllables: &[String]) -> Vec<Lexeme> {
        let key = encode_syllables(syllables);
        self.map
            .get(key.as_bytes())
            .and_then(|i| self.rows.get(i as usize))
            .map(|(_, v)| v.clone())
            .unwrap_or_default()
    }
}

type CandidateRows = Vec<(String, Vec<Lexeme>)>;

struct WubiMap {
    version: WubiVersion,
    map: Map<Vec<u8>>,
    rows: CandidateRows,
}

pub struct FstWubiLexicon {
    maps: Vec<WubiMap>,
}
impl FstWubiLexicon {
    pub fn from_version_rows(
        rows: Vec<(WubiVersion, Vec<(String, Lexeme)>)>,
    ) -> Result<Self, BuildError> {
        let mut maps = Vec::new();
        for (version, input) in rows {
            let mut entries: Vec<(String, Vec<Lexeme>)> = Vec::new();
            for (key, item) in input {
                if key.is_empty() || item.text.is_empty() {
                    return Err(BuildError::EmptyKeyOrCandidate);
                }
                if !item.frequency.is_finite() || item.frequency < 0.0 {
                    return Err(BuildError::InvalidFrequency);
                }
                if let Some((prev, items)) = entries.last_mut() {
                    if prev == &key {
                        items.push(item);
                        continue;
                    }
                    if prev.as_str() > key.as_str() {
                        return Err(BuildError::UnsortedOrDuplicateKey);
                    }
                }
                entries.push((key, vec![item]));
            }
            let mut builder = MapBuilder::memory();
            for (i, (key, _)) in entries.iter().enumerate() {
                builder.insert(key.as_bytes(), i as u64)?;
            }
            maps.push(WubiMap {
                version,
                map: builder.into_map(),
                rows: entries,
            });
        }
        Ok(Self { maps })
    }
}
impl WubiLexicon for FstWubiLexicon {
    fn lookup(&self, version: WubiVersion, code: &str) -> Vec<Lexeme> {
        self.maps
            .iter()
            .find(|entry| entry.version == version)
            .and_then(|entry| {
                entry
                    .map
                    .get(code.as_bytes())
                    .and_then(|i| entry.rows.get(i as usize))
            })
            .map(|(_, v)| v.clone())
            .unwrap_or_default()
    }
    fn reverse_lookup(&self, version: WubiVersion, character: char) -> Vec<String> {
        let mut codes = Vec::new();
        if let Some(entry) = self.maps.iter().find(|entry| entry.version == version) {
            for (code, items) in &entry.rows {
                if items.iter().any(|item| item.text == character.to_string()) {
                    codes.push(code.clone());
                }
            }
        }
        codes
    }
}

/// Read UTF-8 TSV (`key<TAB>text<TAB>frequency[<TAB>annotation]`). No downloads.
pub fn read_tsv(input: &str) -> io::Result<Vec<(String, Lexeme)>> {
    let mut out = Vec::new();
    for (line_no, line) in input.lines().enumerate() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<_> = line.split('\t').collect();
        if fields.len() < 3 || fields[0].is_empty() || fields[1].is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "line {}: expected nonempty key, text, and frequency",
                    line_no + 1
                ),
            ));
        }
        let frequency = fields[2]
            .parse::<f32>()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        if !frequency.is_finite() || frequency < 0.0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "frequency must be finite and nonnegative",
            ));
        }
        out.push((
            fields[0].to_owned(),
            Lexeme {
                text: fields[1].to_owned(),
                frequency,
                annotation: fields.get(3).map(|x| x.to_string()),
            },
        ));
    }
    out.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then_with(|| b.1.frequency.total_cmp(&a.1.frequency))
    });
    Ok(out)
}

/// Pinyin keys use spaces between syllables (`xian` vs `xi an` remain distinct).
pub fn read_pinyin_tsv(input: &str) -> io::Result<Vec<(String, Lexeme)>> {
    let mut rows = read_tsv(input)?;
    for (key, _) in &mut rows {
        *key = key.split_whitespace().collect::<Vec<_>>().join("\u{1f}");
    }
    rows.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then_with(|| b.1.frequency.total_cmp(&a.1.frequency))
    });
    Ok(rows)
}

pub fn encode_syllables(syllables: &[String]) -> String {
    syllables.join("\u{1f}")
}
