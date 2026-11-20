import os
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

class ThemeSyncTest(unittest.TestCase):
    def test_generates_local_fcitx_theme_from_omarchy_palette(self):
        with tempfile.TemporaryDirectory() as tmp:
            home = Path(tmp)
            theme = home / ".config/omarchy/current/theme"
            theme.mkdir(parents=True)
            (theme / "colors.toml").write_text('background = "#101820"\nforeground = "#e0e0e0"\naccent = "#44aa88"\nselection = "#223344"\nmuted = "#667788"\n', encoding="utf-8")
            env = dict(os.environ, HOME=tmp, XDG_CONFIG_HOME=str(home / ".config"), XDG_DATA_HOME=str(home / ".local/share"))
            subprocess.run([str(ROOT / "theme-sync.py")], check=True, env=env, capture_output=True, text=True)
            theme_conf = home / ".local/share/fcitx5/themes/omarchy-sync/theme.conf"
            classic = home / ".config/fcitx5/conf/classicui.conf"
            self.assertIn("NormalColor=#e0e0e0", theme_conf.read_text())
            self.assertIn("HighlightBackgroundColor=#223344", theme_conf.read_text())
            self.assertIn("Theme=omarchy-sync", classic.read_text())
            restore = subprocess.run([str(ROOT / "theme-sync.py"), "--restore"], check=True, env=env, capture_output=True, text=True)
            self.assertFalse((home / ".local/share/fcitx5/themes/omarchy-sync").exists())
            self.assertFalse(classic.exists() and "Theme=omarchy-sync" in classic.read_text())

    def test_repeated_sync_restores_original_font_and_theme(self):
        with tempfile.TemporaryDirectory() as tmp:
            home = Path(tmp)
            palette = home / ".config/omarchy/current/theme"
            palette.mkdir(parents=True)
            (palette / "colors.toml").write_text('background = "#101820"\nforeground = "#e0e0e0"\naccent = "#44aa88"\n', encoding="utf-8")
            (palette.parent / "shell.toml").write_text('[popups]\ncorner-radius = 8\n', encoding="utf-8")
            bindir = home / "bin"
            bindir.mkdir()
            fake_omarchy = bindir / "omarchy"
            fake_omarchy.write_text("#!/bin/sh\ncat \"$FONT_FILE\"\n", encoding="utf-8")
            fake_omarchy.chmod(0o755)
            font_file = home / "font"
            font_file.write_text("First Font\n", encoding="utf-8")
            config = home / ".config/fcitx5/conf/classicui.conf"
            config.parent.mkdir(parents=True)
            config.write_text('Theme="my-theme"\nFont="Original Font 10"\n', encoding="utf-8")
            env = dict(os.environ, HOME=tmp, XDG_CONFIG_HOME=str(home / ".config"), XDG_DATA_HOME=str(home / ".local/share"), PATH=f"{bindir}:{os.environ['PATH']}", FONT_FILE=str(font_file))
            subprocess.run([str(ROOT / "theme-sync.py")], check=True, env=env, capture_output=True, text=True)
            font_file.write_text("Second Font\n", encoding="utf-8")
            subprocess.run([str(ROOT / "theme-sync.py")], check=True, env=env, capture_output=True, text=True)
            subprocess.run([str(ROOT / "theme-sync.py"), "--restore"], check=True, env=env, capture_output=True, text=True)
            self.assertIn('Theme="my-theme"', config.read_text())
            self.assertIn('Font="Original Font 10"', config.read_text())

    def test_restore_without_state_preserves_existing_theme_and_settings(self):
        with tempfile.TemporaryDirectory() as tmp:
            home = Path(tmp)
            env = dict(os.environ, HOME=tmp, XDG_CONFIG_HOME=str(home / ".config"), XDG_DATA_HOME=str(home / ".local/share"))
            config = home / ".config/fcitx5/conf/classicui.conf"
            config.parent.mkdir(parents=True)
            config.write_text("Theme=my-theme\nFont=My Font 13\n", encoding="utf-8")
            theme = home / ".local/share/fcitx5/themes/omarchy-sync"
            theme.mkdir(parents=True)
            (theme / "theme.conf").write_text("user data", encoding="utf-8")
            subprocess.run([str(ROOT / "theme-sync.py"), "--restore"], check=True, env=env, capture_output=True, text=True)
            self.assertEqual(config.read_text(), "Theme=my-theme\nFont=My Font 13\n")
            self.assertEqual((theme / "theme.conf").read_text(), "user data")

if __name__ == "__main__":
    unittest.main()
