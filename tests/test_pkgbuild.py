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




class SystemSqliteTest(unittest.TestCase):
    def test_sqlite_is_not_bundled(self):
        # Compiling the bundled 9 MB sqlite3.c crashed GCC on small machines (Try
        # Omarchy); link the system SQLite instead.
        for toml in ROOT.glob("crates/*/Cargo.toml"):
            text = toml.read_text()
            if "rusqlite" in text:
                self.assertNotIn("bundled", text, toml)
        out = subprocess.run(
            ["bash", "-c", 'source ./PKGBUILD && printf "%s\\n" "${depends[@]}"'],
            cwd=ROOT, check=True, capture_output=True, text=True,
        ).stdout.split()
        self.assertIn("sqlite", out)

    def test_build_jobs_follow_memory(self):
        self.assertIn("MemAvailable", (ROOT / "PKGBUILD").read_text())

    def test_installer_builds_on_disk(self):
        # /tmp is RAM (tmpfs) on Omarchy.
        text = (ROOT / "install.sh").read_text()
        self.assertNotIn('mktemp -d "${TMPDIR:-/tmp}', text)


class SonameTest(unittest.TestCase):
    def test_cdylib_declares_soname(self):
        # Without a SONAME the Fcitx5 front end records the library's absolute build
        # path, which install.sh deletes, so the addon cannot load after install.
        build_rs = (ROOT / "crates/omarchy-ime/build.rs").read_text()
        self.assertIn("-Wl,-soname,libomarchy_ime.so", build_rs)
        lib = ROOT / "target/release/libomarchy_ime.so"
        if lib.exists():
            dynamic = subprocess.run(["readelf", "-d", str(lib)], capture_output=True, text=True).stdout
            self.assertIn("Library soname: [libomarchy_ime.so]", dynamic)


if __name__ == "__main__":
    unittest.main()
