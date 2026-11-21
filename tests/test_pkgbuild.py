import subprocess
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


class PkgbuildTest(unittest.TestCase):
    def test_lto_is_disabled(self):
        # With makepkg's default LTO, rusqlite's bundled SQLite is compiled to GCC LTO
        # bytecode and linking the omarchy-ime binary fails (undefined sqlite3_* symbols).
        out = subprocess.run(
            ["bash", "-c", 'source ./PKGBUILD && printf "%s\\n" "${options[@]}"'],
            cwd=ROOT, check=True, capture_output=True, text=True,
        ).stdout.split()
        self.assertIn("!lto", out)


if __name__ == "__main__":
    unittest.main()
