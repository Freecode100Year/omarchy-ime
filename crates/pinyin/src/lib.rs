//! Bounded pinyin correction and beam decoding. Dictionary data is injected.
mod cost;
mod dag;
mod decoder;
mod double_pinyin;

pub use dag::{SyllableDag, SyllableEdge};
pub use decoder::{decode, decode_with_limits, Lexeme, PinyinConfig, PinyinEngine, PinyinLexicon};
pub use double_pinyin::{encode_double_pinyin, expand_double_pinyin, DoublePinyinScheme};
