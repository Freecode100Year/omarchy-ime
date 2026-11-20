#!/usr/bin/env bash
set -euo pipefail
if ! command -v pacman >/dev/null; then echo 'uninstall.sh must run on Arch Linux.' >&2; exit 1; fi
if command -v omarchy-ime-theme-sync >/dev/null; then omarchy-ime-theme-sync --restore; fi
sudo pacman -R omarchy-ime-git
hook="$HOME/.config/omarchy/hooks/theme-set.d/50-omarchy-ime-theme"
script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
if [[ -f "$hook" ]] && cmp -s "$script_dir/hooks/50-omarchy-ime-theme" "$hook"; then rm "$hook"; fi
printf 'User dictionary and learning data were retained in %s\n' "${XDG_DATA_HOME:-$HOME/.local/share}/omarchy-ime"
