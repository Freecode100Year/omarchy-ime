# omarchy-ime

A local Rust Chinese input engine with an optional Fcitx5 adapter for Omarchy/Hyprland. There is no telemetry or runtime network access. This repository is under active development; the first integration path is Fcitx5, which handles Wayland client protocols and renders the candidate panel using its configured UI theme.

## Install on Omarchy / Arch

Install the Fcitx5 development package and build tools, clone this repository, then run:

```sh
sudo pacman -S --needed fcitx5 fcitx5-gtk fcitx5-qt fcitx5-configtool base-devel cmake rust
./install.sh
```

Add **Omarchy 拼音** in Fcitx5 configuration, then add the following to `~/.config/hypr/hyprland.conf` and log out/in:

```ini
exec-once = fcitx5 -d
```

Fcitx5 provides a Wayland frontend by default. Avoid forcing `GTK_IM_MODULE`/`QT_IM_MODULE` globally for Wayland clients; set them per application only if that client needs an IM module. For Xwayland, add `env = XMODIFIERS,@im=fcitx`.

`Ctrl+Space` toggles input. `theme-sync.py` generates an Fcitx5 Classic UI theme from `~/.config/omarchy/current/theme/colors.toml`; the installer registers it in Omarchy's `theme-set.d` hook. You can run it manually after changing the palette. `./uninstall.sh` removes the Arch package and retains user data; the user hook can be removed at `~/.config/omarchy/hooks/theme-set.d/50-omarchy-ime-theme`.

## Dictionary

The package includes a frequency-trimmed Simplified Chinese dictionary from Rime Ice under GPL-3.0-only; the upstream license and exact pinned source commit are recorded in `data/THIRD_PARTY_NOTICES.md`. For an additional personal UTF-8 TSV, put it at `~/.local/share/omarchy-ime/pinyin.tsv`; it takes priority over the system dictionary:

```text
# pinyin syllables separated by spaces; UTF-8; frequency is a nonnegative number
ni hao\t你好\t1000
ni\t你\t900
hao\t好\t700
```

Validate a personal file with `omarchy-ime --check ~/.local/share/omarchy-ime/pinyin.tsv`; measure local key processing with `omarchy-ime --bench nihao 1000`. Benchmarks vary by CPU and dictionary, and are not a latency guarantee. The Rust process loads one read-only FST per process and shares it across Fcitx contexts. It tries the user dictionary, then the installed system dictionary, then the small original starter. No dictionary is downloaded at runtime.

## Workspace

- `ime-core`: compositor-independent event, context, candidate, and commit API.
- `pinyin`: weighted syllable DAG, typo/fuzzy costs, bounded beam decoding.
- `wubi`: mixed Wubi/pinyin engine and versioned injected codebook interface.
- `handwriting`: normalized-grid/direction features and template matching.
- `lexicon`: FST adapters and TSV parsing.
- `omarchy-ime`: local runtime, CLI diagnostics, and C ABI.
- `frontend/fcitx5`: C++ Fcitx5 glue around the Rust ABI (Fcitx5 itself handles Wayland protocol/client-specific behavior).

## Current limits

The included dictionary is a bounded, frequency-filtered subset of Rime Ice; it is useful for common Simplified Chinese typing but does not contain the full upstream corpus. Wubi tables, macros, SQLite learning, and handwriting panel/model integration are not yet connected to the Fcitx adapter. Native `zwp_input_method_v2`, custom `wlr-layer-shell` windows, and automatic Vim/Neovim mode detection are also not implemented. Kitty/Ghostty/Alacritty/Electron/GTK/Qt compatibility depends on the exact frontend protocols and needs target-machine verification. No <3 ms latency or memory target is claimed until benchmarked with real dictionaries.

## Verification

Rust checks run in this repository: `cargo test --workspace`, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo build --release --workspace`. The Fcitx5 C++ adapter compiled against Debian Fcitx5 Core 5.1.12 and its staged install layout/RPATH were checked. The Arch/Omarchy target has not been exercised here, so rebuild and verify there before relying on compatibility with specific Wayland clients.
