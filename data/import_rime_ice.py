#!/usr/bin/env python3
"""Convert pinned Rime Ice 8105 and base dictionaries to bounded omarchy-ime TSV."""
from collections import defaultdict
from pathlib import Path
import sys

base_source = Path(sys.argv[1])
char_source = Path(sys.argv[2])
dest = Path(sys.argv[3])
rows = defaultdict(list)
inside = False
for line in base_source.read_text(encoding="utf-8").splitlines():
    if line.strip() == "...":
        inside = True
        continue
    if not inside or not line or line.startswith("#"):
        continue
    fields = line.split("\t")
    if len(fields) != 3:
        continue
    text, pinyin, weight = fields
    if not text or not pinyin or not weight.isdigit():
        continue
    # Keep high-frequency short words. This provides common daily vocabulary
    # while bounding the runtime FST and its in-memory candidate side table.
    score = int(weight)
    if len(text) > 4 or score < 5000:
        continue
    rows[pinyin].append((score, text))
# Import the pinned 8105 common-character table so ordinary single-character
# syllables have candidates as well as phrase completions.
inside = False
for line in char_source.read_text(encoding="utf-8").splitlines():
    if line.strip() == "...":
        inside = True
        continue
    if not inside or not line or line.startswith("#"):
        continue
    fields = line.split("\t")
    if len(fields) != 3:
        continue
    text, pinyin, weight = fields
    if len(text) != 1 or not pinyin or not weight.isdigit():
        continue
    rows[pinyin].append((int(weight), text))

with dest.open("w", encoding="utf-8", newline="\n") as out:
    out.write("# Rime Ice 8105 single-character + base phrase dictionaries; GPL-3.0; see THIRD_PARTY_NOTICES.md\n")
    for pinyin in sorted(rows):
        seen = set()
        ranked = sorted(rows[pinyin], key=lambda item: -item[0])
        for weight, text in ranked:
            if text in seen:
                continue
            seen.add(text)
            out.write(f"{pinyin}\t{text}\t{weight}\n")
            if len(seen) == 10:
                break
