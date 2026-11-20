//! Shared, compositor-independent input method API.
mod candidate;
mod event;

pub use candidate::{Candidate, CandidateList, CandidateSource};
pub use event::{Context, EngineUpdate, InputEngine, Key, KeyEvent, Modifiers};
