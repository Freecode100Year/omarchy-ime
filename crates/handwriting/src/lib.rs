//! Local template-based handwriting features. No model or network is needed.
mod features;
mod matcher;

pub use features::{extract_features, FeatureVector, StrokePoint};
pub use matcher::{match_templates, HandwritingMatch, HandwritingTemplate};

use ime_core::{Candidate, CandidateSource};

pub fn to_candidates(matches: &[HandwritingMatch]) -> Vec<Candidate> {
    matches
        .iter()
        .map(|m| Candidate {
            text: m.character.to_string(),
            annotation: Some(format!("手写 {:.3}", m.distance)),
            score: m.score,
            source: CandidateSource::Handwriting,
        })
        .collect()
}
