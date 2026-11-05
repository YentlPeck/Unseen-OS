"""Every chore example in the manual (and in `guide do`) must really work.

  python tools/test_manual_examples.py

Takes each code block of the manual's Chores chapter, plus the example in the
kernel's GUIDE text, puts them on pages, runs each with `do` in the real OS
and checks it Finished (and, where it matters, what it did).
"""

import re
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from unseen import Machine  # noqa: E402

ROOT = Path(__file__).resolve().parent.parent

WORLD = """#today 2026-11-12
#page contacts:omar
name: Omar Haddad
mail: omar
phone: 555-0404
#end
#page contacts:lina
name: Lina Berg
mail: lina
phone: 555-0505
#end
#page mail:inbox:m1
from: omar
to: me
subject: Team Meeting moved

See you then.
#end
#page mail:inbox:m2
from: lina
to: me
subject: Lesson

Can we move it?
#end
#page recipes:soup
soup
#end
#page recipes:cake
cake
#end
#page costs
12.5
30
7.5
#end
"""

# What the examples must do, in order.
EXPECT = [
    ("count pages", lambda s, f: f.get("desk:how-many") == "2"),
    ("phone list", lambda s, f: f.get("desk:phones") == "Lina Berg: 555-0505\nOmar Haddad: 555-0404"),
    ("find mails", lambda s, f: "omar - Team Meeting moved" in s and "lina" not in s.split("Finished")[1]),
    ("add up lines", lambda s, f: "total: 50" in s),
    ("next 3 Fridays", lambda s, f: sorted(t.split("\n")[0] for p, t in f.items() if p.startswith("desk:calendar:"))
     == ["date: 2026-11-13", "date: 2026-11-20", "date: 2026-11-27"]),
]


def fenced_blocks(text):
    """(info, code) of every ``` block, e.g. ("text", ...) for ```text."""
    out, info, buf = [], None, []
    for line in text.split("\n"):
        if line.startswith("```"):
            if info is None:
                info, buf = line[3:].strip(), []
            else:
                out.append((info, "\n".join(buf) + "\n"))
                info = None
        elif info is not None:
            buf.append(line)
    return out


def chapter():
    text = (ROOT / "docs" / "manual.md").read_text(encoding="utf-8")
    return text.split("## Chores", 1)[1].split("\n## ", 1)[0]


def chapter_blocks():
    """The chore examples: plain ``` blocks (the reference is a ```text block)."""
    return [code for info, code in fenced_blocks(chapter()) if info == ""]


def guide_text():
    src = (ROOT / "kernel" / "src" / "chores.rs").read_text(encoding="utf-8")
    return src.split('pub const GUIDE: &str = "\\\n', 1)[1].split('";', 1)[0].replace('\\"', '"')


def guide_example():
    block = guide_text().split("EXAMPLE\n", 1)[1].split("\n\n", 1)[0]
    return "\n".join(line[2:] for line in block.split("\n"))


def main():
    refs = [code for info, code in fenced_blocks(chapter()) if info == "text"]
    if refs != [guide_text() + "\n"]:
        print("FAIL the manual must contain the guide do reference word for word")
        return 1
    print("PASS manual contains the guide do reference word for word")
    blocks = chapter_blocks()
    assert len(blocks) == len(EXPECT), f"manual has {len(blocks)} chore blocks, test expects {len(EXPECT)}"
    cases = list(zip([e[0] for e in EXPECT], blocks, [e[1] for e in EXPECT]))
    cases.append(("guide do example", guide_example(), lambda s, f: "Omar sent 1 mails" in s))
    failures = []
    with tempfile.TemporaryDirectory() as tmp:
        for name, code, ok in cases:
            too_long = [line for line in code.split("\n") if len(line) > 78]
            m = Machine.start(WORLD + f"#page job\n{code.rstrip()}\n#end\n", workdir=tmp)
            try:
                m.run_keys("do job{enter}")
                screen = m.settle()
                files = m.dump()
                good = "Finished:" in screen and ok(screen, files) and not too_long
                print(("PASS " if good else "FAIL ") + name)
                if not good:
                    failures.append(name)
                    print(screen if not too_long else f"lines over 78 characters: {too_long}")
            finally:
                m.stop()
    print("\nALL GOOD" if not failures else f"\n{len(failures)} FAILED: {failures}")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
