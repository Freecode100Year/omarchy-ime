//! Bounded pinyin correction and beam decoding. Dictionary data is injected.
mod cost;
mod dag;
mod decoder;

pub use dag::{SyllableDag, SyllableEdge};
pub use decoder::{decode, Lexeme, PinyinConfig, PinyinEngine, PinyinLexicon};
