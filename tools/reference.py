"""The chore reference for experiment 4's "reference" condition.

The text is the ```text block of the Chores chapter of docs/manual.md, with line endings
normalised to \\n (it ends with one \\n). reference_text() refuses to return it unless it is
identical to the GUIDE constant in kernel/src/chores.rs, the text `guide do` shows on the screen.
"""

import hashlib
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MANUAL = ROOT / "docs" / "manual.md"
CHORES_RS = ROOT / "kernel" / "src" / "chores.rs"

INTRO = "Here is the chore reference for this computer (the same text `guide do` shows):"


def guide_source():
    """The GUIDE constant as the OS shows it (Rust string: a leading "\\<newline>", \\" escapes)."""
    src = CHORES_RS.read_text(encoding="utf-8").replace("\r\n", "\n")
    body = src.split('pub const GUIDE: &str = "\\\n', 1)[1].split('";', 1)[0]
    if re.search(r'\\[^"]', body):
        raise ValueError("GUIDE uses an escape other than \\\" (extend guide_source)")
    return body.replace('\\"', '"')


def manual_block(manual_path=MANUAL):
    text = Path(manual_path).read_text(encoding="utf-8").replace("\r\n", "\n")
    chapter = text.split("\n## Chores", 1)[1].split("\n## ", 1)[0]
    blocks = re.findall(r"^```text\n(.*?)^```$", chapter, re.S | re.M)
    if len(blocks) != 1:
        raise ValueError(f"the Chores chapter must have exactly one ```text block (has {len(blocks)})")
    return blocks[0]


def reference_text(manual_path=MANUAL):
    block = manual_block(manual_path)
    if block != guide_source() + "\n":
        raise ValueError("the manual's reference block differs from the OS's GUIDE text")
    return block


def sha256(text):
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


if __name__ == "__main__":
    t = reference_text()
    print(f"reference: {len(t.splitlines())} lines, {len(t)} chars, sha256 {sha256(t)}")
