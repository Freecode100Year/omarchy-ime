# omarchy-ime Design

## Intent

Build a high-performance, offline Simplified Chinese input method framework for Omarchy/Hyprland. The engine is Rust and has no telemetry. Pinyin, Wubi, and handwriting share candidate, macro, and commit abstractions. Fcitx5 is the initial application-compatibility backend; a native Wayland `zwp_input_method_v2` backend follows behind the same engine API. A layer-shell UI reads Omarchy theme values from `~/.config/omarchy/current/theme/`.

The <3 ms target applies to engine key processing and must be measured; it is not a guarantee for IPC, rendering, or optional model inference. Compatibility with individual terminals and Electron/Chromium applications must be verified against their actual protocol support.

## Approaches

1. Fcitx5 backend with Rust engine: practical compatibility, but not all-Rust.
2. Native Wayland backend: mostly Rust, but client support varies and needs validation.
3. Dual backend (selected): share one Rust engine, ship Fcitx5 integration first, then native Wayland; broadest reach with higher maintenance cost.

## Architecture

Workspace crates: `ime-core`, `pinyin`, `wubi`, `handwriting`, `lexicon`, `user-data`, `macros`, `frontend-fcitx`, `frontend-wayland`, `candidate-ui`, and the `omarchy-ime` daemon. Engine modules produce `EngineUpdate`; frontends translate key/stroke events and commit operations. Candidate UI consumes common `CandidateList` data.

```rust
pub trait InputEngine {
    fn handle_key(&mut self, key: KeyEvent, ctx: &Context) -> EngineUpdate;
    fn select_candidate(&mut self, index: usize) -> EngineUpdate;
    fn reset(&mut self);
}
```

`Candidate` carries text, optional annotation, score, and source. `CandidateList` carries candidates, page/page size, and preedit. `EngineUpdate` covers candidates, commit, pass-through, handwriting activation, and no-op. Macros participate in candidate processing and commits.

## Engine components

- Pinyin: weighted syllable DAG with keyboard adjacency, transposition, missing/extra key, and configurable fuzzy-initial/final costs; Beam Search combines lexicon and n-gram scores. English spans remain intact when their path cost is better than syllable segmentation.
- Wubi: versioned external code tables for 86/98/New Century; four-code exact lookup, concurrent pinyin retrieval, annotated merged candidates, and Z reverse lookup/learning.
- Handwriting: `(x, y, stroke_id)` samples, normalized direction-chain/grid features, and elastic template matching. ONNX Runtime is an optional backend, not required for baseline operation.
- Lexicon: memory-mapped/FST read-only system dictionaries and hot-pluggable cell dictionaries.
- User data: SQLite WAL for custom vocabulary and usage weights, with time decay.
- Macros: TOML trigger, candidate position, multiline expansion, and dynamic date/time values.
- Hyprland: focus/window IPC integration. Vim/Neovim Normal mode requires editor-provided mode state; window focus alone is insufficient.

## Dependency direction

`ime-core` stays dependency-light. Proposed dependencies include `fst` and mmap for dictionaries, `rusqlite` for learning, `serde`/`toml` for configuration, `wayland-client` plus generated protocol bindings and `smithay-client-toolkit` for native UI, layer-shell protocol bindings, and optional `ort`. Fcitx5 integration boundary must be validated before selecting a Rust ABI bridge or local IPC implementation.

## Integration and delivery order

1. Workspace and common API; pure Rust algorithm core.
2. Lexicon, decoding, Wubi, macros, and local learning.
3. Fcitx5 backend and candidate UI.
4. Handwriting panel and template matching.
5. Native Wayland backend, theme synchronization, and Hyprland/editor state integration.
6. Verify target applications and benchmark latency and memory.

## Constraints and non-goals

- All input and learning data stays local; no telemetry or network dependency at runtime.
- Model inference and dictionary data are optional external assets.
- Do not claim app compatibility without verification on that app.
- Do not bundle copyrighted third-party dictionaries or handwriting corpora without suitable licenses.
