//! Stable, per-input-context C ABI. No callbacks cross the FFI boundary.
use ime_core::{CandidateList, Context, EngineUpdate, InputEngine, Key, KeyEvent, Modifiers};
use std::{
    ffi::{c_char, CString},
    panic::{catch_unwind, AssertUnwindSafe},
    ptr,
};

pub struct EngineHandle {
    engine: pinyin::PinyinEngine,
    candidates: CandidateList,
    preedit: CString,
    commit: CString,
    items: Vec<CString>,
}
fn cstring(s: &str) -> CString {
    CString::new(s).unwrap_or_else(|_| CString::default())
}
impl EngineHandle {
    fn refresh(&mut self, update: EngineUpdate) -> i32 {
        self.commit = CString::default();
        match update {
            EngineUpdate::Candidates(list) => {
                self.candidates = list;
                self.preedit = cstring(&self.candidates.preedit);
                1
            }
            EngineUpdate::Commit(text) => {
                self.candidates = CandidateList::default();
                self.preedit = CString::default();
                self.commit = cstring(&text);
                2
            }
            EngineUpdate::Noop => {
                self.candidates = CandidateList::default();
                self.preedit = CString::default();
                0
            }
            EngineUpdate::PassThrough => -1,
            EngineUpdate::ShowHandwriting => 3,
        }
    }
    fn sync_items(&mut self) {
        self.items = self
            .candidates
            .current_page()
            .iter()
            .map(|c| cstring(&c.text))
            .collect();
    }
}

/// Creates a handle with the built-in demonstration lexicon. Returns null on panic.
#[no_mangle]
pub extern "C" fn omarchy_ime_engine_new() -> *mut EngineHandle {
    catch_unwind(|| {
        Box::into_raw(Box::new(EngineHandle {
            engine: pinyin::PinyinEngine::new(
                crate::runtime_lexicon(),
                crate::runtime_pinyin_config(),
            ),
            candidates: CandidateList::default(),
            preedit: CString::default(),
            commit: CString::default(),
            items: Vec::new(),
        }))
    })
    .unwrap_or(ptr::null_mut())
}
/// Destroys a handle returned by `omarchy_ime_engine_new`.
///
/// # Safety
/// `handle` must be null or a live, uniquely owned handle created by this library.
#[no_mangle]
pub unsafe extern "C" fn omarchy_ime_engine_free(handle: *mut EngineHandle) {
    if !handle.is_null() {
        let _ = catch_unwind(AssertUnwindSafe(|| drop(Box::from_raw(handle))));
    }
}
/// Key values: Unicode scalar for printable input, 0x100 backspace, 0x101 enter, 0x102 escape, 0x103 space. Modifiers: ctrl=1, alt=2, super=4. Returns 1 candidates, 2 committed, 0 consumed/noop, -1 pass-through.
///
/// # Safety
/// `handle` must be null or a live handle created by this library and not concurrently accessed.
#[no_mangle]
pub unsafe extern "C" fn omarchy_ime_engine_key(
    handle: *mut EngineHandle,
    key: u32,
    modifiers: u32,
    secure: bool,
) -> i32 {
    let Some(h) = handle.as_mut() else { return -1 };
    catch_unwind(AssertUnwindSafe(|| {
        let key = match key {
            0x100 => Key::Backspace,
            0x101 => Key::Enter,
            0x102 => Key::Escape,
            0x103 => Key::Space,
            n => char::from_u32(n)
                .map(Key::Character)
                .unwrap_or(Key::Other(n)),
        };
        if secure {
            let update = h.engine.handle_key(
                KeyEvent {
                    key: Key::Escape,
                    modifiers: Modifiers::default(),
                    repeat: false,
                },
                &Context {
                    secure: true,
                    ..Context::default()
                },
            );
            let _ = h.refresh(update);
            h.candidates = CandidateList::default();
            h.preedit = CString::default();
            h.items.clear();
            return -1;
        }
        let update = h.engine.handle_key(
            KeyEvent {
                key,
                modifiers: Modifiers {
                    shift: modifiers & 8 != 0,
                    ctrl: modifiers & 1 != 0,
                    alt: modifiers & 2 != 0,
                    super_key: modifiers & 4 != 0,
                },
                repeat: false,
            },
            &Context {
                secure,
                ..Context::default()
            },
        );
        let code = h.refresh(update);
        h.sync_items();
        code
    }))
    .unwrap_or(-1)
}
///
/// # Safety
/// `handle` must be null or a live handle created by this library and not concurrently accessed.
#[no_mangle]
pub unsafe extern "C" fn omarchy_ime_engine_select(handle: *mut EngineHandle, index: usize) -> i32 {
    let Some(h) = handle.as_mut() else { return -1 };
    catch_unwind(AssertUnwindSafe(|| {
        let update = h.engine.select_candidate(index);
        let code = h.refresh(update);
        h.sync_items();
        code
    }))
    .unwrap_or(-1)
}
///
/// # Safety
/// `handle` must be null or point to a live handle created by this library.
#[no_mangle]
pub unsafe extern "C" fn omarchy_ime_engine_candidate_count(handle: *const EngineHandle) -> usize {
    handle.as_ref().map_or(0, |h| h.items.len())
}
///
/// # Safety
/// `handle` must be null or point to a live handle; returned text is borrowed until the next mutating call.
#[no_mangle]
pub unsafe extern "C" fn omarchy_ime_engine_candidate(
    handle: *const EngineHandle,
    index: usize,
) -> *const c_char {
    handle
        .as_ref()
        .and_then(|h| h.items.get(index))
        .map_or(ptr::null(), |s| s.as_ptr())
}
///
/// # Safety
/// `handle` must be null or point to a live handle; returned text is borrowed until the next mutating call.
#[no_mangle]
pub unsafe extern "C" fn omarchy_ime_engine_preedit(handle: *const EngineHandle) -> *const c_char {
    handle.as_ref().map_or(ptr::null(), |h| h.preedit.as_ptr())
}
///
/// # Safety
/// `handle` must be null or point to a live handle; returned text is borrowed until the next mutating call.
#[no_mangle]
pub unsafe extern "C" fn omarchy_ime_engine_commit(handle: *const EngineHandle) -> *const c_char {
    handle.as_ref().map_or(ptr::null(), |h| h.commit.as_ptr())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CStr;
    #[test]
    fn ffi_isolates_context_and_commits_first_candidate() {
        unsafe {
            let a = omarchy_ime_engine_new();
            let b = omarchy_ime_engine_new();
            assert_eq!(omarchy_ime_engine_key(a, 'n' as u32, 0, false), 1);
            assert_eq!(omarchy_ime_engine_key(a, 'i' as u32, 0, false), 1);
            assert_eq!(omarchy_ime_engine_key(b, 'w' as u32, 0, false), 1);
            assert_eq!(
                CStr::from_ptr(omarchy_ime_engine_preedit(a))
                    .to_str()
                    .unwrap(),
                "ni"
            );
            assert_eq!(omarchy_ime_engine_key(a, 0x103, 0, false), 2);
            assert_eq!(
                CStr::from_ptr(omarchy_ime_engine_commit(a))
                    .to_str()
                    .unwrap(),
                "你"
            );
            omarchy_ime_engine_free(a);
            omarchy_ime_engine_free(b);
        }
    }
    #[test]
    fn secure_transition_clears_ffi_candidates_but_passthrough_preserves_them() {
        unsafe {
            let h = omarchy_ime_engine_new();
            for c in ['n', 'i'] {
                assert_eq!(omarchy_ime_engine_key(h, c as u32, 0, false), 1);
            }
            assert!(omarchy_ime_engine_candidate_count(h) > 0);
            assert_eq!(omarchy_ime_engine_key(h, '中' as u32, 0, false), -1);
            assert!(omarchy_ime_engine_candidate_count(h) > 0);
            assert_eq!(omarchy_ime_engine_key(h, 0, 0, true), -1);
            assert_eq!(omarchy_ime_engine_candidate_count(h), 0);
            assert_eq!(
                CStr::from_ptr(omarchy_ime_engine_preedit(h)).to_bytes(),
                b""
            );
            omarchy_ime_engine_free(h);
        }
    }
    #[test]
    fn secure_context_passes_through_without_composition() {
        unsafe {
            let h = omarchy_ime_engine_new();
            assert_eq!(omarchy_ime_engine_key(h, 'n' as u32, 0, true), -1);
            omarchy_ime_engine_free(h);
        }
    }
}
