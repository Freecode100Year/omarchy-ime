#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$(readlink -f "$0")")"
if ! command -v pacman >/dev/null || ! command -v makepkg >/dev/null; then
  echo 'Omarchy IME packaging requires Arch Linux makepkg and pacman.' >&2; exit 1
fi
if ! command -v pkg-config >/dev/null || ! pkg-config --exists Fcitx5Core; then
  echo 'Install Fcitx5 development files first: sudo pacman -S fcitx5' >&2; exit 1
fi
hook="$HOME/.config/omarchy/hooks/theme-set.d/50-omarchy-ime-theme"
if [[ -e "$hook" ]] && ! cmp -s hooks/50-omarchy-ime-theme "$hook"; then
  echo "Refusing to overwrite existing hook: $hook" >&2; exit 1
fi
makepkg -si
install -Dm755 hooks/50-omarchy-ime-theme "$hook"
"$HOME/.config/omarchy/hooks/theme-set.d/50-omarchy-ime-theme"
config_file="${XDG_CONFIG_HOME:-$HOME/.config}/omarchy-ime/config.toml"
if [[ ! -e "$config_file" ]]; then install -Dm644 config/omarchy-ime.toml "$config_file"; fi
macro_file="${XDG_CONFIG_HOME:-$HOME/.config}/omarchy-ime/macros.toml"
if [[ ! -e "$macro_file" ]]; then install -Dm644 config/macros.toml "$macro_file"; fi
cat <<'MSG'
Installed Omarchy IME. Add “Omarchy 拼音” and/or “Omarchy 五笔拼音混输” to your Fcitx5 input methods, then restart Fcitx5:
  fcitx5-remote -r
Input and learning data stay local. The package includes a frequency-trimmed offline GPL-3.0 dictionary.
Optional personal dictionaries at ~/.local/share/omarchy-ime/pinyin.tsv and wubi.tsv take priority.
MSG
