//! Bounded pinyin correction and beam decoding. Dictionary data is injected.
mod cost;
mod dag;
mod decoder;

pub use dag::{SyllableDag, SyllableEdge};
pub use decoder::{decode, decode_with_limits, Lexeme, PinyinConfig, PinyinEngine, PinyinLexicon};
