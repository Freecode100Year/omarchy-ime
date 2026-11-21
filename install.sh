#!/usr/bin/env bash
set -euo pipefail
repo_url='https://github.com/Freecode100Year/omarchy-ime.git'

if ! command -v pacman >/dev/null; then
  echo 'Omarchy IME installation requires Arch Linux or Omarchy with pacman.' >&2
  exit 1
fi
if (( EUID == 0 )); then
  echo 'Run this installer as your regular user; it uses sudo only when pacman needs it.' >&2
  exit 1
fi
if ! command -v sudo >/dev/null; then
  echo 'Install and configure sudo, then rerun this installer as your regular user.' >&2
  exit 1
fi

bootstrap_packages=()
if ! command -v git >/dev/null; then bootstrap_packages+=(git); fi
if ! command -v makepkg >/dev/null; then bootstrap_packages+=(base-devel); fi
if ((${#bootstrap_packages[@]})); then
  echo "Installing bootstrap tools: ${bootstrap_packages[*]}"
  if ! sudo pacman -S --needed --noconfirm "${bootstrap_packages[@]}"; then
    echo 'Pacman could not install the bootstrap tools. Check /etc/pacman.d/mirrorlist; if a mirror reports "Operation too slow", select a current HTTPS mirror and rerun this command.' >&2
    exit 1
  fi
fi

script_source="${BASH_SOURCE[0]:-}"
source_root=''
if [[ -n "$script_source" && -f "$script_source" ]]; then
  source_root="$(cd -- "$(dirname -- "$script_source")" && pwd -P)"
fi

bootstrap_root=''
if [[ ! -f "$source_root/PKGBUILD" ]]; then
  bootstrap_root="$(mktemp -d "${TMPDIR:-/tmp}/omarchy-ime.XXXXXX")"
  trap 'if [[ -n "$bootstrap_root" ]]; then rm -rf -- "$bootstrap_root"; fi' EXIT
  source_root="$bootstrap_root/omarchy-ime"
  echo 'Fetching Omarchy IME source from GitHub...'
  git clone --quiet --branch main --single-branch "$repo_url" "$source_root"
fi

cd -- "$source_root"
hook="$HOME/.config/omarchy/hooks/theme-set.d/50-omarchy-ime-theme"
if [[ -e "$hook" ]] && ! cmp -s hooks/50-omarchy-ime-theme "$hook"; then
  echo "Refusing to overwrite existing hook: $hook" >&2; exit 1
fi
if ! makepkg --syncdeps --install --noconfirm; then
  echo 'Omarchy IME build or package installation failed. Review the pacman/build error above. If pacman reports a slow mirror, update /etc/pacman.d/mirrorlist to use a current HTTPS mirror, run sudo pacman -Syu, then rerun this installer.' >&2
  exit 1
fi
install -Dm755 hooks/50-omarchy-ime-theme "$hook"
"$HOME/.config/omarchy/hooks/theme-set.d/50-omarchy-ime-theme"
config_file="${XDG_CONFIG_HOME:-$HOME/.config}/omarchy-ime/config.toml"
if [[ ! -e "$config_file" ]]; then install -Dm644 config/omarchy-ime.toml "$config_file"; fi
macro_file="${XDG_CONFIG_HOME:-$HOME/.config}/omarchy-ime/macros.toml"
if [[ ! -e "$macro_file" ]]; then install -Dm644 config/macros.toml "$macro_file"; fi
# Make the running Fcitx5 load the new engine and add it to the input-method list.
# Omarchy's own Input Methods picker only lists its built-in catalog, so this is
# what makes "Omarchy 拼音" show up at all.
if [[ -n ${WAYLAND_DISPLAY:-} ]] && command -v omarchy-ime-enable >/dev/null; then
  omarchy-ime-enable || echo 'Run omarchy-ime-enable after logging in to the desktop to add Omarchy 拼音.' >&2
fi
cat <<'MSG'
Installed Omarchy IME. “Omarchy 拼音” has been added to your input methods.
  On Omarchy press Super + I to switch to it. 在 Omarchy 上按 Super + I 切换输入法。
  Also add the Wubi mix: omarchy-ime-enable --wubi
  Not showing? Check with: omarchy-ime-enable --status
Input and learning data stay local. The package includes a frequency-trimmed offline GPL-3.0 dictionary.
Optional personal dictionaries at ~/.local/share/omarchy-ime/pinyin.tsv and wubi.tsv take priority.
MSG
