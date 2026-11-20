#!/usr/bin/env python3
"""Convert pinned Rime-compatible Wubi 86/98 YAML tables to local TSV."""
from pathlib import Path
import unicodedata

root = Path(__file__).parent
sources = (
    ("86", root / "third_party/rime-wubi/wubi86.dict.yaml"),
    ("98", root / "third_party/oh-my-rime/wubi98_base.dict.yaml"),
)
rows = []
for version, source in sources:
    lines = source.read_text(encoding="utf-8").splitlines()
    try:
        start = lines.index("...") + 1
    except ValueError as error:
        raise SystemExit(f"{source} has no YAML document separator") from error
    for line in lines[start:]:
        if not line or line.lstrip().startswith("#"):
            continue
        fields = line.split("\t")
        if len(fields) not in (2, 3, 4):
            raise SystemExit(f"unexpected dictionary row in {source}: {line!r}")
        text, code = fields[:2]
        # The 98 table contains private-use-area emoji glyphs with no portable font.
        if any(unicodedata.category(char) == "Co" for char in text):
            continue
        weight = fields[2] if len(fields) >= 3 else "1"
        rows.append(f"{version}\t{code}\t{text}\t{weight}")

target = root / "wubi.tsv"
target.write_text("\n".join(rows) + "\n", encoding="utf-8")
print(f"wrote {len(rows)} rows to {target}")
