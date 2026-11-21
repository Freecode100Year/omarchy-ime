# omarchy-ime

A local Rust Chinese input engine with an optional Fcitx5 adapter for Omarchy/Hyprland. There is no telemetry or runtime network access. This repository is under active development; the first integration path is Fcitx5, which handles Wayland client protocols and renders the candidate panel using its configured UI theme.

## Install on Omarchy / Arch

Run the installer as your regular user from a terminal:

```sh
curl -fsSL https://raw.githubusercontent.com/Freecode100Year/omarchy-ime/main/install.sh | bash
```

The installer fetches the source tree, installs `git` and `base-devel` if needed, then lets `makepkg` install the declared build dependencies and build the package. It uses `sudo` for package-manager operations. To install from an existing checkout instead, run `./install.sh` from the repository directory. If pacman reports a slow or unavailable mirror, update `/etc/pacman.d/mirrorlist` to use a current HTTPS mirror and rerun the installer.

`fcitx5-gtk`, `fcitx5-qt`, and `fcitx5-configtool` are optional integrations/configuration tools and can be installed with pacman when needed.

Add **Omarchy 拼音** in Fcitx5 configuration, then add the following to `~/.config/hypr/hyprland.conf` and log out/in:

```ini
exec-once = fcitx5 -d
```

Fcitx5 provides a Wayland frontend by default. Avoid forcing `GTK_IM_MODULE`/`QT_IM_MODULE` globally for Wayland clients; set them per application only if that client needs an IM module. For Xwayland, add `env = XMODIFIERS,@im=fcitx`.

`Ctrl+Space` toggles input. `theme-sync.py` generates an Fcitx5 Classic UI theme from `~/.config/omarchy/current/theme/colors.toml`; the installer registers it in Omarchy's `theme-set.d` hook. You can run it manually after changing the palette. `./uninstall.sh` removes the Arch package and retains user data; the user hook can be removed at `~/.config/omarchy/hooks/theme-set.d/50-omarchy-ime-theme`.

## Dictionary

The package includes a frequency-trimmed Simplified Chinese dictionary from Rime Ice under GPL-3.0-only, Rime Wubi 86 under LGPL-3.0, and Mint's Wubi 98 table under GPL-3.0. Pinned commits and notices are in `data/THIRD_PARTY_NOTICES.md`. Add “Omarchy 拼音” and “Omarchy 五笔拼音混输” in Fcitx to switch using your configured Fcitx method hotkey. For a personal UTF-8 pinyin TSV, put it at `~/.local/share/omarchy-ime/pinyin.tsv`; it takes priority over the system dictionary:

```text
# pinyin syllables separated by spaces; UTF-8; frequency is a nonnegative number
ni hao\t你好\t1000
ni\t你\t900
hao\t好\t700
```

Validate a personal file with `omarchy-ime --check ~/.local/share/omarchy-ime/pinyin.tsv`; measure local key processing with `omarchy-ime --bench nihao 1000`. Benchmarks vary by CPU and dictionary, and are not a latency guarantee. The Rust process loads one read-only FST per process and shares it across Fcitx contexts. It tries the user dictionary, then the installed system dictionary, then the small original starter. No dictionary is downloaded at runtime.

Set `double_pinyin = "xiaohe"` or `"natural-code"` in `~/.config/omarchy-ime/config.toml` for those double-pinyin layouts; the default is `"full"`. Both Fcitx method entries use the same setting.

Wubi uses `~/.local/share/omarchy-ime/wubi.tsv` when present, otherwise the packaged 86/98 tables. Its format is `version<TAB>code<TAB>text<TAB>frequency`, with version `86`, `98`, or `new-century`. Set `wubi_version` in `config.toml` to select the table. New Century is supported by the loader but is not bundled.

Macros are read from `${XDG_CONFIG_HOME:-~/.config}/omarchy-ime/macros.toml` (a starter file is installed). Each `[[macro]]` has a punctuation-prefixed `trigger`, `text`, and optional `position`; `${date}`, `${time}`, and `${datetime}` are expanded locally. Use `\n` for multiline text. Macro triggers may contain digits after the first punctuation key. Macros are per-input-context and commit through the normal Fcitx candidate path.

Pinyin choices are recorded locally in a mode-0600 SQLite WAL at `user_data_path` and decay with a 30-day half-life. Add a personal phrase with `omarchy-ime --learn nihao 你好`; restart Fcitx5 after using this command so the running engine reloads its in-memory personal-word index. Learned phrases are indexed alongside the read-only system lexicon. These features never contact a network service.

## Workspace

- `ime-core`: compositor-independent event, context, candidate, and commit API.
- `pinyin`: weighted syllable DAG, typo/fuzzy costs, bounded beam decoding.
- `wubi`: mixed Wubi/pinyin engine and versioned local codebook interface.
- `handwriting`: normalized-grid/direction features and template matching.
- `ime-macros`: TOML macro matching and local time expansion.
- `ime-user-data`: SQLite WAL learning store with an in-memory candidate index.
- `lexicon`: FST adapters and TSV parsing.
- `omarchy-ime`: local runtime, CLI diagnostics, and C ABI.
- `frontend/fcitx5`: C++ Fcitx5 glue around the Rust ABI (Fcitx5 itself handles Wayland protocol/client-specific behavior).

## Current limits

The included Pinyin dictionary is a bounded, frequency-filtered subset of Rime Ice. Wubi 86 and 98 are connected through the Fcitx method; New Century code tables must be supplied by the user. Macros and SQLite learning are connected. The handwriting matcher still has no drawing panel, shipped recognition templates, or model backend. Native `zwp_input_method_v2`, custom `wlr-layer-shell` windows, and automatic Vim/Neovim mode detection are also not implemented. Kitty/Ghostty/Alacritty/Electron/GTK/Qt compatibility depends on the exact frontend protocols and needs target-machine verification. No <3 ms latency or memory target is claimed until benchmarked with real dictionaries.

## Verification

Rust checks run in this repository: `cargo test --workspace`, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo build --release --workspace`. The Fcitx5 C++ adapter compiled against Debian Fcitx5 Core 5.1.12 and its staged install layout/RPATH were checked. The Arch/Omarchy target has not been exercised here, so rebuild and verify there before relying on compatibility with specific Wayland clients.
