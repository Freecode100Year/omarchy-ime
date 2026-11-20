# Omarchy IME Application Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn the published algorithm prototype into a locally installable Fcitx5 Chinese IME package for Omarchy/Hyprland, with a Rust engine, offline data, user configuration, theme sync, and a documented native Wayland path.

**Architecture:** Keep the algorithm crates compositor independent. Add a Rust cdylib ABI with per-context handles, a small C++ Fcitx5 `InputMethodEngineV2` adapter, desktop installation files, and local configuration/data. Fcitx5 owns client protocol handling and candidate UI in the first usable release; native input-method-v2 and layer-shell remain a later backend until compositor support is verified.

**Tech Stack:** Rust workspace, C ABI, Fcitx5 Core addon API, CMake, TOML, SQLite, FST, shell/systemd user integration.

**Spec:** `docs/superpowers/specs/2026-11-19-omarchy-ime-design.md`

## Global Constraints

- All input and learning data stays local; no telemetry and no runtime network dependency.
- Engine key processing target is <3 ms and must be measured before making a performance claim.
- Do not claim app compatibility without verification on that app.
- Do not bundle third-party dictionary or handwriting corpora without suitable licenses.
- Fcitx5 is the initial compatibility backend; native Wayland support is separate and compositor dependent.

## Review Focus

- Empty or malformed system dictionaries must fail with actionable errors, not panic.
- Composition state must be isolated per Fcitx input context and reset on focus/security transitions.
- Control shortcuts, secure fields, and non-ASCII input must not be swallowed.
- Installation must remain user-local and reversible; no implicit sudo or network activity.
- Missing optional Wayland/Fcitx SDKs must be reported accurately and must not make Rust core fail to build.

---

### Task 1: User-facing runtime/config and offline dictionary loading

**Files:** Create `crates/omarchy-ime/Cargo.toml`, `src/lib.rs`, `src/main.rs`, config/data templates; modify workspace manifests.

**Interfaces:** Load local TSV dictionaries and build an engine; expose a local diagnostic CLI and version/config checks.

- [ ] Add failing tests for TSV validation, config defaults, and local dictionary lookup.
- [ ] Implement runtime configuration and a small clearly-labelled starter lexicon only; support user-provided dictionaries.
- [ ] Run workspace tests, fmt, clippy, and release build.

### Task 2: Rust C ABI for the Fcitx adapter

**Files:** `crates/omarchy-ime/src/ffi.rs`, `Cargo.toml`.

**Interfaces:** Opaque per-context engine handle; key event, candidate count/text, preedit, selection, reset, and last commit accessors. Returned strings remain valid until the next call or handle destruction.

- [ ] Add FFI behavior tests for compose/select/cancel, invalid index, secure context, and null pointers.
- [ ] Implement panic-contained C ABI and export headers.
- [ ] Build cdylib and run all Rust checks.

### Task 3: Fcitx5 adapter and packaging

**Files:** Create `frontend/fcitx5/{CMakeLists.txt,src/*,addon conf,inputmethod conf}` and package metadata.

**Interfaces:** Fcitx input contexts own Rust handles; candidate panel, preedit, and commits map to core ABI.

- [ ] Add a build smoke check that clearly skips when Fcitx SDK is absent.
- [ ] Implement adapter against official `InputMethodEngineV2` and `CommonCandidateList` API.
- [ ] Validate build and installation layout on an Omarchy/Arch host with Fcitx SDK.

### Task 4: User-local installer and Hyprland setup

**Files:** `install.sh`, `uninstall.sh`, `config/`, `systemd/`, Hyprland snippets and theme-sync hook.

- [ ] Test install path resolution with temporary HOME/XDG directories.
- [ ] Install user-local files only, enable Fcitx environment variables and user service, preserve existing config backups, and provide reversible uninstall.
- [ ] Add focus/editor-mode hook with explicit editor-provided state; window title alone is not treated as Vim mode.

### Task 5: Whole-project review and release checks

- [ ] Run all Rust checks and shell/CMake package checks.
- [ ] Review the entire branch for security, privacy, Wayland integration truthfulness, licenses, and install reversibility.
- [ ] Record host-dependent checks not possible in the build container; publish only after the review is clean.
