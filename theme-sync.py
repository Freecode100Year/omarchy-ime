#!/usr/bin/env python3
"""Generate a Fcitx5 classic UI theme from Omarchy's active theme."""
import json, os, re, shutil, subprocess, sys, tomllib
from pathlib import Path

home = Path.home()
config_home = Path(os.environ.get("XDG_CONFIG_HOME", home / ".config"))
data_home = Path(os.environ.get("XDG_DATA_HOME", home / ".local/share"))
config = config_home / "fcitx5/conf/classicui.conf"
theme_dir = data_home / "fcitx5/themes/omarchy-sync"
state_dir = data_home / "omarchy-ime/theme-sync"
state_file = state_dir / "previous-theme"

def replace_setting(text, key, value):
    lines = [line for line in text.splitlines() if not line.startswith(key + "=")]
    if value is not None:
        lines.append(f"{key}={value}")
    return "\n".join(lines) + ("\n" if lines else "")

def restore():
    current = config.read_text(encoding="utf-8") if config.exists() else ""
    if not state_file.exists():
        print("omarchy-ime: no saved theme state; nothing to restore")
        return
    saved = json.loads(state_file.read_text(encoding="utf-8"))
    previous = saved.get("previous", saved)
    config.parent.mkdir(parents=True, exist_ok=True)
    for key in ("Theme", "Font"):
        managed = saved.get("managed", {}).get(key)
        active = next((line.split("=", 1)[1] for line in current.splitlines() if line.startswith(key + "=")), None)
        if managed is not None and active == managed:
            current = replace_setting(current, key, previous.get(key))
    config.write_text(current, encoding="utf-8")
    marker = theme_dir / ".omarchy-ime-owned"
    if marker.is_file():
        shutil.rmtree(theme_dir)
    state_file.unlink()
    try:
        subprocess.run(["fcitx5-remote", "-r"], check=False, timeout=3, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    except (OSError, subprocess.SubprocessError):
        pass
    print("omarchy-ime: restored previous Fcitx5 theme setting")

if "--restore" in sys.argv[1:]:
    restore()
    raise SystemExit(0)

state_home = Path(os.environ.get("XDG_STATE_HOME", home / ".local/state"))
source = Path(os.environ.get("OMARCHY_THEME_DIR", home / ".config/omarchy/current/theme"))
if not (source / "colors.toml").is_file():
    source = state_home / "omarchy/current/theme"
palette_path = source / "colors.toml"
if not palette_path.is_file():
    print(f"omarchy-ime: no Omarchy palette at {palette_path}", file=sys.stderr)
    raise SystemExit(1)
palette = tomllib.loads(palette_path.read_text(encoding="utf-8"))
def color(key, fallback):
    value = str(palette.get(key, fallback))
    if not re.fullmatch(r"#[0-9A-Fa-f]{6}([0-9A-Fa-f]{2})?", value):
        raise ValueError(f"invalid {key} color in {palette_path}: {value}")
    return value[:7]
background = color("background", "#202020")
foreground = color("foreground", "#eeeeee")
accent = color("accent", "#6688cc")
selection = color("selection", accent)
muted = color("muted", "#666666")
shell_path = source / "shell.toml"
shell = tomllib.loads(shell_path.read_text(encoding="utf-8")) if shell_path.exists() else {}
radius = os.environ.get("OMARCHY_IME_RADIUS", str(shell.get("popups", {}).get("corner-radius", palette.get("radius", 10))))
if not radius.isdigit() or int(radius) > 32:
    raise ValueError("OMARCHY_IME_RADIUS must be an integer from 0 to 32")
if theme_dir.exists() and not (theme_dir / ".omarchy-ime-owned").is_file():
    print(f"omarchy-ime: refusing to overwrite unowned theme directory {theme_dir}", file=sys.stderr)
    raise SystemExit(1)
theme_dir.mkdir(parents=True, exist_ok=True)
(theme_dir / ".omarchy-ime-owned").write_text("omarchy-ime\n", encoding="utf-8")
(theme_dir / "panel.svg").write_text(
    f'<svg xmlns="http://www.w3.org/2000/svg" width="320" height="80"><rect x="1" y="1" width="318" height="78" rx="{radius}" fill="{background}" stroke="{muted}" stroke-width="2"/></svg>\n', encoding="utf-8")
(theme_dir / "theme.conf").write_text(f'''[Metadata]\nName=Omarchy Sync\nVersion=1\nDescription=Generated from the active Omarchy palette\n\n[InputPanel]\nNormalColor={foreground}\nHighlightColor={foreground}\nHighlightCandidateColor={background}\nHighlightBackgroundColor={selection}\nSpacing=4\n[InputPanel/Background]\nImage=panel.svg\nBorderWidth=0\n[InputPanel/Background/Margin]\nLeft=8\nRight=8\nTop=8\nBottom=8\n[InputPanel/ContentMargin]\nLeft=4\nRight=4\nTop=4\nBottom=4\n[InputPanel/Highlight]\nColor={selection}\n[InputPanel/Highlight/Margin]\nLeft=4\nRight=4\nTop=2\nBottom=2\n[InputPanel/TextMargin]\nLeft=4\nRight=4\nTop=2\nBottom=2\n''', encoding="utf-8")

current = config.read_text(encoding="utf-8") if config.exists() else ""
previous = {key: next((line.split("=", 1)[1] for line in current.splitlines() if line.startswith(key + "=")), None) for key in ("Theme", "Font")}
current = replace_setting(current, "Theme", "omarchy-sync")
managed = {"Theme": "omarchy-sync"}
try:
    font = subprocess.run(["omarchy", "font", "current"], check=True, capture_output=True, text=True, timeout=3).stdout.strip()
except (OSError, subprocess.SubprocessError):
    font = ""
if font:
    # Preserve the current font size if one is set; otherwise use Fcitx's familiar 12pt size.
    match = re.search(r"(?m)^Font=\"?(.*?)\s+(\d+(?:\.\d+)?)\"?$", current)
    size = match.group(2) if match else "12"
    current = replace_setting(current, "Font", f'"{font} {size}"')
    managed["Font"] = f'"{font} {size}"'
state_dir.mkdir(parents=True, exist_ok=True)
if state_file.exists():
    saved = json.loads(state_file.read_text(encoding="utf-8"))
    saved.setdefault("managed", {}).update(managed)
else:
    saved = {"previous": previous, "managed": managed}
state_file.write_text(json.dumps(saved), encoding="utf-8")
config.parent.mkdir(parents=True, exist_ok=True)
config.write_text(current, encoding="utf-8")
try:
    subprocess.run(["fcitx5-remote", "-r"], check=False, timeout=3, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
except (OSError, subprocess.SubprocessError):
    pass
print(f"omarchy-ime: synchronized Fcitx5 theme from {palette_path}")
