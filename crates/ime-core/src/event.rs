use crate::CandidateList;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Character(char),
    Backspace,
    Enter,
    Escape,
    Space,
    Tab,
    Left,
    Right,
    Up,
    Down,
    Other(u32),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub super_key: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyEvent {
    pub key: Key,
    pub modifiers: Modifiers,
    pub repeat: bool,
}

#[derive(Clone, Debug, Default)]
pub struct Context {
    pub application_id: Option<String>,
    pub surrounding_text: Option<String>,
    pub cursor_byte: Option<usize>,
    pub secure: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum EngineUpdate {
    Candidates(CandidateList),
    Commit(String),
    PassThrough,
    ShowHandwriting,
    Noop,
}

pub trait InputEngine {
    fn handle_key(&mut self, key: KeyEvent, ctx: &Context) -> EngineUpdate;
    fn select_candidate(&mut self, index: usize) -> EngineUpdate;
    fn reset(&mut self);
}
