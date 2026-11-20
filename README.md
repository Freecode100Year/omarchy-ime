# omarchy-ime

Offline Simplified Chinese input-method framework for Omarchy. This repository currently contains the compositor-independent Rust engine prototype; Fcitx5 and native Wayland frontends are follow-on work.

## Workspace

- `ime-core`: common key, context, candidate, and commit API.
- `pinyin`: weighted syllable DAG, keyboard-aware edit costs, fuzzy syllables, and bounded beam decoding.
- `wubi`: injectable 86/98/New Century codebook interface and mixed pinyin/Wubi engine.
- `handwriting`: local normalized-grid/direction features and template matching.
- `lexicon`: in-memory FST adapters and a local TSV reader. No dictionary data is bundled.

## Build

Install a stable Rust toolchain and run `cargo build --workspace`. The FST dependency is `fst` 0.4.7; its map keys must be inserted in lexicographic order and its map values are `u64` indexes into the candidate side table. See the [fst crate documentation](https://docs.rs/fst/0.4.7/fst/) for the data structure and memory-mapped usage model.

The dictionary TSV format is UTF-8:

```text
# pinyin syllables separated by spaces; Wubi keys are unseparated
ni hao	你好	1000
ni	你	900
hao	好	700
```

Supply licensed data locally. This prototype does not download dictionaries, contact a service, or emit telemetry.

## Current limits

- This milestone is an engine prototype, not a usable desktop input method yet.
- Double-pinyin scheme maps, complete maintained syllable inventory, production n-gram data, domain-cell hot reload, macros, and SQLite learning remain follow-on work.
- The current Wubi codebook is an injected interface; code tables are not included.
- Handwriting uses feature-template matching; no ONNX model or recognition templates are included.
- Fcitx5, `zwp_input_method_v2`, layer-shell UI, Omarchy theme sync, and Hyprland/Vim mode integration are not implemented yet.

## Verification

`cargo test --workspace`, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo build --release --workspace` pass for this core prototype. Tests use small fixture dictionaries; app compatibility, corpus accuracy, memory use, and the <3 ms latency target have not been measured.
