use std::ops::Range;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CandidateSource {
    Pinyin,
    Wubi,
    Handwriting,
    Macro,
    User,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Candidate {
    pub text: String,
    pub annotation: Option<String>,
    /// Larger values rank earlier. Engine scores are finite by contract.
    pub score: f32,
    pub source: CandidateSource,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CandidateList {
    pub items: Vec<Candidate>,
    pub page: usize,
    pub page_size: usize,
    pub preedit: String,
    pub cursor: usize,
    pub segments: Vec<Range<usize>>,
}

impl CandidateList {
    pub fn new(preedit: impl Into<String>, items: Vec<Candidate>, page_size: usize) -> Self {
        Self {
            items,
            page: 0,
            page_size: page_size.max(1),
            preedit: preedit.into(),
            cursor: 0,
            segments: Vec::new(),
        }
    }
    pub fn current_page(&self) -> &[Candidate] {
        let start = self
            .page
            .saturating_mul(self.page_size)
            .min(self.items.len());
        &self.items[start..start.saturating_add(self.page_size).min(self.items.len())]
    }
    pub fn select(&self, index: usize) -> Option<&Candidate> {
        self.current_page().get(index)
    }
    pub fn page_count(&self) -> usize {
        self.items.len().div_ceil(self.page_size.max(1))
    }
}
