# omarchy-ime Core Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Establish a compileable Rust workspace with shared input abstractions and local implementations of pinyin decoding, Wubi/pinyin retrieval, and handwriting feature matching.

**Architecture:** Keep the first milestone compositor-independent. `ime-core` defines the engine and candidate API; algorithm crates implement it with injected dictionary interfaces. Protocol frontends, UI, persistence, and OS integration are follow-on plans after the core contracts stabilize.

**Tech Stack:** Rust workspace; `serde`/`toml` for configuration; `fst` for dictionary lookup; `rusqlite` is deferred to persistence milestone; protocol crates are deferred to frontend milestones.

**Spec:** `docs/superpowers/specs/2026-11-19-omarchy-ime-design.md`

## Global Constraints

- Core engine is offline and contains no telemetry.
- Engine key processing target is <3 ms and must be benchmarked before claiming it.
- Dictionary content is injected as licensed external assets, not bundled without license review.
- Application protocol compatibility is not part of this core milestone.

## Review Focus

- Empty input and invalid keys: return `Noop` or `PassThrough`, never panic (Task 1).
- Pinyin input with no full syllable path: preserve recoverable preedit and return stable candidates (Task 2).
- English spans mixed with valid pinyin: retain the English span rather than force segmentation (Task 2).
- Wubi inputs shorter/longer than four codes and unknown version: no accidental unique auto-commit (Task 3).
- Empty handwriting strokes and degenerate coordinates: return no matches rather than NaN scores or panic (Task 4).

---

### Task 1: Workspace and shared engine API

**Files:**
- Create: `Cargo.toml`
- Create: `crates/ime-core/Cargo.toml`
- Create: `crates/ime-core/src/lib.rs`
- Create: `crates/ime-core/src/candidate.rs`
- Create: `crates/ime-core/src/event.rs`
- Create: `crates/ime-core/tests/api.rs`

**Interfaces:**
- Produces `InputEngine::handle_key(KeyEvent, &Context) -> EngineUpdate`, `select_candidate(usize) -> EngineUpdate`, and `reset()`.
- Produces `Candidate`, `CandidateList`, `CandidateSource`, and `EngineUpdate` types as described in the design.

- [ ] Write API tests asserting empty candidate page behavior, pass-through handling, and stable candidate selection bounds.
- [ ] Run `cargo test -p ime-core`; confirm tests fail because workspace/types are absent.
- [ ] Implement the workspace and public API with explicit bounds checks and no engine-specific dependencies.
- [ ] Run `cargo test -p ime-core`; confirm all API tests pass.

### Task 2: Pinyin syllable DAG and decoder

**Files:**
- Create: `crates/pinyin/Cargo.toml`
- Create: `crates/pinyin/src/lib.rs`
- Create: `crates/pinyin/src/dag.rs`
- Create: `crates/pinyin/src/cost.rs`
- Create: `crates/pinyin/src/decoder.rs`
- Create: `crates/pinyin/tests/decoding.rs`

**Interfaces:**
- Consumes `ime_core::{InputEngine, KeyEvent, Context, EngineUpdate, Candidate, CandidateList}`.
- Produces `PinyinEngine::new(lexicon: Arc<dyn PinyinLexicon>, config: PinyinConfig)`, implementing `InputEngine`.
- Produces `SyllableDag::from_input(&str, &PinyinConfig) -> SyllableDag` and `decode(&SyllableDag, &dyn PinyinLexicon, beam_width: usize) -> Vec<Candidate>`.

- [ ] Add tests for exact full pinyin, a keyboard-adjacent typo, adjacent transposition (`shoduao` prefix/path recovery), fuzzy `zh/z`, mixed English boundary, and incomplete suffix preservation.
- [ ] Run `cargo test -p pinyin`; confirm the new tests fail.
- [ ] Implement DAG edges with bounded weighted edit distance, physical QWERTY neighbor cost, transposition, missing/extra-key costs, and configurable fuzzy initial/final mappings.
- [ ] Implement bounded Beam Search over dictionary words with frequency and bigram score hooks; enforce deterministic tie-breaking.
- [ ] Run `cargo test -p pinyin`; confirm tests pass and no path explosion occurs for configured maximum input length.

### Task 3: Wubi lookup and mixed retrieval

**Files:**
- Create: `crates/wubi/Cargo.toml`
- Create: `crates/wubi/src/lib.rs`
- Create: `crates/wubi/src/codebook.rs`
- Create: `crates/wubi/src/mixed.rs`
- Create: `crates/wubi/tests/mixed_lookup.rs`

**Interfaces:**
- Consumes `ime_core` candidate and engine types plus `PinyinLexicon` lookup abstraction.
- Produces `WubiVersion::{V86,V98,NewCentury}`, `WubiLexicon::lookup(version, code)`, and `MixedEngine` with both lexicon sources.

- [ ] Add tests for version-isolated lookups, four-code unique commit policy, parallel pinyin/Wubi candidate merge with code annotation, Z reverse lookup, and invalid/unknown input behavior.
- [ ] Run `cargo test -p wubi`; confirm tests fail.
- [ ] Implement injected codebook lookup and deterministic candidate merging; only auto-commit when the active policy confirms a unique exact four-code result.
- [ ] Run `cargo test -p wubi`; confirm tests pass.

### Task 4: Handwriting features and template matcher

**Files:**
- Create: `crates/handwriting/Cargo.toml`
- Create: `crates/handwriting/src/lib.rs`
- Create: `crates/handwriting/src/features.rs`
- Create: `crates/handwriting/src/matcher.rs`
- Create: `crates/handwriting/tests/matching.rs`

**Interfaces:**
- Produces `StrokePoint { x: f32, y: f32, stroke_id: u16 }`, `HandwritingTemplate`, `FeatureVector`, `extract_features(&[StrokePoint]) -> Option<FeatureVector>`, and `match_templates(&FeatureVector, &[HandwritingTemplate], limit: usize) -> Vec<HandwritingMatch>`.

- [ ] Add tests for translated/scaled same-shape recognition, stroke segmentation, empty input, degenerate coordinates, and stable score ordering.
- [ ] Run `cargo test -p handwriting`; confirm tests fail.
- [ ] Implement coordinate normalization, fixed-size direction-chain/grid features, and elastic distance scoring with finite score checks.
- [ ] Run `cargo test -p handwriting`; confirm tests pass.

### Task 5: Offline FST dictionary format and integration

**Files:**
- Create: `crates/lexicon/Cargo.toml`
- Create: `crates/lexicon/src/lib.rs`
- Create: `crates/lexicon/src/pinyin.rs`
- Create: `crates/lexicon/src/wubi.rs`
- Create: `crates/lexicon/tests/fst_lookup.rs`

**Interfaces:**
- Produces a documented binary dictionary record format and read-only `FstPinyinLexicon`/`FstWubiLexicon` implementations usable by Tasks 2 and 3.

- [ ] Add tests constructing a small temporary fixture and checking exact/prefix lookup, malformed files, and missing assets.
- [ ] Run `cargo test -p lexicon`; confirm tests fail.
- [ ] Implement FST-backed lookups with mmap where supported and a portable read-only fallback; reject malformed data with typed errors.
- [ ] Run `cargo test -p lexicon`; confirm tests pass.

### Task 6: Workspace-level integration and performance measurement

**Files:**
- Modify: workspace manifests and crate integration points
- Create: `crates/ime-core/benches/key_latency.rs`
- Create: `README.md`

**Interfaces:**
- Integrates the three engine crates via the shared candidate API; no compositor frontend is introduced in this milestone.

- [ ] Add an integration test that submits a pinyin key sequence, selects a candidate, exercises Wubi mixed lookup, and evaluates handwriting matches through common `Candidate` conversion.
- [ ] Run `cargo test --workspace`; record failures before integration.
- [ ] Add a Criterion benchmark for engine key updates on representative input lengths and fixed fixture lexicons.
- [ ] Run `cargo test --workspace` and `cargo bench -p ime-core --bench key_latency`; report measured latency without treating it as an app-level guarantee.
- [ ] Document dictionary asset format, runtime offline behavior, and known compatibility work deferred to later plans.
