//! Stable, per-input-context C ABI. No callbacks cross the FFI boundary.
use ime_core::{CandidateList, Context, EngineUpdate, InputEngine, Key, KeyEvent, Modifiers};
use std::{
    ffi::{c_char, CString},
    panic::{catch_unwind, AssertUnwindSafe},
    ptr,
};

const KEY_BACKSPACE: u32 = 0x11_0000;
const KEY_ENTER: u32 = 0x11_0001;
const KEY_ESCAPE: u32 = 0x11_0002;
const KEY_SPACE: u32 = 0x11_0003;

pub struct EngineHandle {
    engine: Box<dyn InputEngine>,
    wubi_mode: bool,
    candidates: CandidateList,
    preedit: CString,
    commit: CString,
    items: Vec<CString>,
    surrounding_text: String,
    cursor_byte: Option<usize>,
}
fn build_engine(wubi_mode: bool) -> Box<dyn InputEngine> {
    let inner: Box<dyn InputEngine> = if wubi_mode {
        Box::new(wubi::MixedEngine::new(
            crate::load_default_wubi(crate::runtime_wubi_version()),
            crate::runtime_lexicon(),
            wubi::MixedConfig {
                version: crate::runtime_wubi_version(),
                page_size: crate::runtime_pinyin_config().page_size,
                pinyin_config: crate::runtime_pinyin_config(),
                ..wubi::MixedConfig::default()
            },
        ))
    } else {
        Box::new(pinyin::PinyinEngine::new(
            crate::runtime_lexicon(),
            crate::runtime_pinyin_config(),
        ))
    };
    Box::new(ime_macros::MacroEngine::new(
        inner,
        crate::runtime_macros(),
        crate::runtime_pinyin_config().page_size,
    ))
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
                self.preedit = cstring(self.engine.preedit());
                self.commit = cstring(&text);
                2
            }
            EngineUpdate::Noop => {
                self.candidates = CandidateList::default();
                self.preedit = cstring(self.engine.preedit());
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
        let wubi_mode = crate::load_app_config().mode.eq_ignore_ascii_case("wubi");
        Box::into_raw(Box::new(EngineHandle {
            engine: build_engine(wubi_mode),
            wubi_mode,
            candidates: CandidateList::default(),
            preedit: CString::default(),
            commit: CString::default(),
            items: Vec::new(),
            surrounding_text: String::new(),
            cursor_byte: None,
        }))
    })
    .unwrap_or(ptr::null_mut())
}
/// Selects Pinyin (`0`) or Wubi/Pinyin mixed mode (`1`) and clears composition.
/// Returns 1 when the mode is accepted, or -1 for invalid input/handle.
///
/// # Safety
/// `handle` must be null or a live handle created by this library and not concurrently accessed.
#[no_mangle]
pub unsafe extern "C" fn omarchy_ime_engine_set_mode(handle: *mut EngineHandle, mode: i32) -> i32 {
    let Some(h) = handle.as_mut() else { return -1 };
    if mode != 0 && mode != 1 {
        return -1;
    }
    catch_unwind(AssertUnwindSafe(|| {
        h.wubi_mode = mode == 1;
        h.engine = build_engine(h.wubi_mode);
        h.candidates = CandidateList::default();
        h.preedit = CString::default();
        h.commit = CString::default();
        h.items.clear();
        h.surrounding_text.clear();
        h.cursor_byte = None;
        1
    }))
    .unwrap_or(-1)
}
/// Returns current engine mode: 0 Pinyin, 1 Wubi/Pinyin mixed, -1 for null.
///
/// # Safety
/// `handle` must be null or point to a live handle.
#[no_mangle]
pub unsafe extern "C" fn omarchy_ime_engine_mode(handle: *const EngineHandle) -> i32 {
    handle.as_ref().map_or(-1, |h| i32::from(h.wubi_mode))
}
/// Returns nonzero when the current engine wants to consume this character
/// even though it is punctuation (used by configured macro triggers).
///
/// # Safety
/// `handle` must be null or point to a live handle.
#[no_mangle]
pub unsafe extern "C" fn omarchy_ime_engine_wants_key(
    handle: *const EngineHandle,
    key: u32,
) -> bool {
    let Some(h) = handle.as_ref() else {
        return false;
    };
    char::from_u32(key).is_some_and(|ch| h.engine.wants_key(Key::Character(ch)))
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
/// Key values: Unicode scalar for printable input; special keys use values
/// above Unicode (`0x110000` backspace through `0x110003` space). Modifiers:
/// ctrl=1, alt=2, super=4. Returns 1 candidates, 2 committed, 0 noop, -1 pass-through.
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
            KEY_BACKSPACE => Key::Backspace,
            KEY_ENTER => Key::Enter,
            KEY_ESCAPE => Key::Escape,
            KEY_SPACE => Key::Space,
            n => char::from_u32(n)
                .map(Key::Character)
                .unwrap_or(Key::Other(n)),
        };
        if secure {
            h.surrounding_text.clear();
            h.cursor_byte = None;
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
        if key == Key::Escape {
            h.surrounding_text.clear();
            h.cursor_byte = None;
        }
        if matches!(key, Key::Enter | Key::Space) {
            record_default_pinyin_choice(h);
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
                application_id: None,
                surrounding_text: (!h.surrounding_text.is_empty())
                    .then(|| h.surrounding_text.clone()),
                cursor_byte: h.cursor_byte,
                secure,
            },
        );
        let code = h.refresh(update);
        h.sync_items();
        code
    }))
    .unwrap_or(-1)
}
fn record_default_pinyin_choice(handle: &EngineHandle) {
    let Some(candidate) = handle.candidates.select(0) else {
        return;
    };
    if candidate.source != ime_core::CandidateSource::Pinyin {
        return;
    }
    let raw_code = handle.preedit.to_str().unwrap_or_default();
    let code = crate::runtime_pinyin_config()
        .double_pinyin
        .and_then(|scheme| pinyin::expand_double_pinyin(raw_code, scheme))
        .unwrap_or_else(|| raw_code.to_owned());
    crate::record_learning(&code, &candidate.text);
}

/// Sets a UTF-8 surrounding-text snapshot for operations such as Wubi reverse
/// lookup. Cursor offset is in Unicode scalar values; the core stores bytes.
///
/// # Safety
/// `handle` must be null or live. `text` must reference `len` readable bytes.
#[no_mangle]
pub unsafe extern "C" fn omarchy_ime_engine_set_surrounding(
    handle: *mut EngineHandle,
    text: *const u8,
    len: usize,
    cursor_chars: usize,
) -> i32 {
    let Some(h) = handle.as_mut() else { return -1 };
    h.surrounding_text.clear();
    h.cursor_byte = None;
    if len > 4 || (len > 0 && text.is_null()) {
        return -1;
    }
    catch_unwind(AssertUnwindSafe(|| {
        let bytes = if len == 0 {
            &[]
        } else {
            std::slice::from_raw_parts(text, len)
        };
        let Ok(value) = std::str::from_utf8(bytes) else {
            return -1;
        };
        h.surrounding_text.push_str(value);
        h.cursor_byte = Some(
            value
                .char_indices()
                .nth(cursor_chars)
                .map_or(value.len(), |(index, _)| index),
        );
        1
    }))
    .unwrap_or(-1)
}

/// Flushes an active composition as its best candidate, or as literal text if
/// the composition has no candidate. Used before a punctuation key is passed
/// to the focused application.
///
/// # Safety
/// `handle` must be null or a live engine handle.
#[no_mangle]
pub unsafe extern "C" fn omarchy_ime_engine_flush(handle: *mut EngineHandle) -> i32 {
    let Some(h) = handle.as_mut() else { return -1 };
    catch_unwind(AssertUnwindSafe(|| {
        record_default_pinyin_choice(h);
        let update = h.engine.flush();
        let result = h.refresh(update);
        h.sync_items();
        result
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
        if let Some(candidate) = h.candidates.select(index) {
            if candidate.source == ime_core::CandidateSource::Pinyin {
                let raw_code = h.preedit.to_str().unwrap_or_default();
                let code = crate::runtime_pinyin_config()
                    .double_pinyin
                    .and_then(|scheme| pinyin::expand_double_pinyin(raw_code, scheme))
                    .unwrap_or_else(|| raw_code.to_owned());
                crate::record_learning(&code, &candidate.text);
            }
        }
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
            assert_eq!(omarchy_ime_engine_key(a, KEY_SPACE, 0, false), 2);
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
    #[test]
    fn mode_can_switch_and_run_wubi_through_the_same_ffi() {
        unsafe {
            let h = omarchy_ime_engine_new();
            assert_eq!(omarchy_ime_engine_mode(h), 0);
            assert_eq!(omarchy_ime_engine_set_mode(h, 1), 1);
            assert_eq!(omarchy_ime_engine_mode(h), 1);
            for c in ['k', 'h', 'k'] {
                assert_eq!(omarchy_ime_engine_key(h, c as u32, 0, false), 1);
            }
            assert!((0..omarchy_ime_engine_candidate_count(h)).any(|i| {
                CStr::from_ptr(omarchy_ime_engine_candidate(h, i))
                    .to_str()
                    .unwrap()
                    == "中"
            }));
            assert_eq!(omarchy_ime_engine_set_mode(h, 0), 1);
            assert_eq!(omarchy_ime_engine_mode(h), 0);
            assert!(CStr::from_ptr(omarchy_ime_engine_preedit(h))
                .to_bytes()
                .is_empty());
            omarchy_ime_engine_free(h);
        }
    }

    #[test]
    fn printable_unicode_above_latin1_does_not_collide_with_control_keys() {
        unsafe {
            let handle = omarchy_ime_engine_new();
            assert_eq!(omarchy_ime_engine_key(handle, 'n' as u32, 0, false), 1);
            assert_eq!(omarchy_ime_engine_key(handle, 0x0100, 0, false), -1);
            assert_eq!(
                CStr::from_ptr(omarchy_ime_engine_preedit(handle))
                    .to_str()
                    .unwrap(),
                "n"
            );
            omarchy_ime_engine_free(handle);
        }
    }

    #[test]
    fn invalid_surrounding_snapshot_clears_old_value() {
        unsafe {
            let handle = omarchy_ime_engine_new();
            let previous = "中".as_bytes();
            assert_eq!(
                omarchy_ime_engine_set_surrounding(handle, previous.as_ptr(), previous.len(), 1),
                1
            );
            assert_eq!((*handle).surrounding_text, "中");
            assert_eq!(
                omarchy_ime_engine_set_surrounding(handle, previous.as_ptr(), 5, 1),
                -1
            );
            let snapshot = &(*handle).surrounding_text;
            assert!(snapshot.is_empty());
            omarchy_ime_engine_free(handle);
        }
    }

    #[test]
    fn selected_double_pinyin_suffix_stays_visible_and_does_not_swallow_space_or_enter() {
        unsafe {
            let handle = omarchy_ime_engine_new();
            let inner: Box<dyn InputEngine> = Box::new(pinyin::PinyinEngine::new(
                crate::bundled_lexicon(),
                pinyin::PinyinConfig {
                    double_pinyin: Some(pinyin::DoublePinyinScheme::Xiaohe),
                    ..pinyin::PinyinConfig::default()
                },
            ));
            (*handle).engine = Box::new(ime_macros::MacroEngine::new(
                inner,
                ime_macros::MacroTable::default(),
                9,
            ));
            for ch in ['n', 'i', 'h'] {
                assert_eq!(omarchy_ime_engine_key(handle, ch as u32, 0, false), 1);
            }
            assert_eq!(omarchy_ime_engine_select(handle, 0), 2);
            assert_eq!(
                CStr::from_ptr(omarchy_ime_engine_preedit(handle))
                    .to_str()
                    .unwrap(),
                "h"
            );
            assert_eq!(omarchy_ime_engine_key(handle, KEY_SPACE, 0, false), -1);
            assert_eq!(omarchy_ime_engine_key(handle, KEY_ENTER, 0, false), -1);
            assert_eq!(
                CStr::from_ptr(omarchy_ime_engine_preedit(handle))
                    .to_str()
                    .unwrap(),
                "h"
            );
            assert_eq!(omarchy_ime_engine_flush(handle), 2);
            assert_eq!(
                CStr::from_ptr(omarchy_ime_engine_commit(handle))
                    .to_str()
                    .unwrap(),
                "h"
            );
            assert!(CStr::from_ptr(omarchy_ime_engine_preedit(handle))
                .to_bytes()
                .is_empty());
            omarchy_ime_engine_free(handle);
        }
    }
}
