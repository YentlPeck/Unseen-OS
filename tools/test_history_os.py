"""End-to-end test of stamps (version history) in the real OS, in QEMU.

  python tools/test_history_os.py

Boots Unseen OS with start files that hold a small history, types the history
commands through the virtual keyboard, and checks the screen, the desk (DUMP)
and the history (STAMPS) after each step.
"""

import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from unseen import Machine  # noqa: E402

START = """#today 2026-11-06
#page tools:total
set total to 0
for each m in pages('mail:inbox')
  set total to total + 1
done
say total
#end
#stamp 2026-11-01 first version
#page tools:total
set total to 1
for each m in pages('mail:inbox')
  set total to total + 1
done
say total
#end
#page notes:todo
fix the count
#end
#stamp 2026-11-02 start at one
"""

FAILS = []


def check(name, ok, detail=""):
    print(("ok   " if ok else "FAIL ") + name)
    if not ok:
        FAILS.append(name)
        print(detail)


def run(m, line):
    m.type(line)
    m.press("enter")
    return m.settle()


def page(m, path):
    dump = m.dump()
    return dump.get(path) if isinstance(dump, dict) else None


def main():
    with tempfile.TemporaryDirectory() as tmp:
        m = Machine.start(START, workdir=tmp)
        try:
            s = m._ask("STAMPS")
            check("start files make 2 stamps, desk on #2",
                  s.startswith("BASE 2\n") and "S 1 - 2026-11-01 first version" in s
                  and "S 2 1 2026-11-02 start at one" in s, s)

            scr = run(m, "guide")
            check("guide lists the History words", "History" in scr and "rewind" in scr, scr)

            scr = run(m, "stamps")
            check("stamps opens a list with the base marked",
                  "* #2" in scr and "#1" in scr and "no changes since" in scr, scr)
            m.press("esc")
            m.settle()

            scr = run(m, "stamp nothing")
            check("stamp with no change is refused", "nothing changed since stamp #2" in scr, scr)

            run(m, "write notes:todo fixed")
            scr = run(m, "changes")
            check("changes shows the edit",
                  "changed  desk:notes:todo" in scr and "1 - fix the count" in scr and "1 + fixed" in scr, scr)
            m.press("esc")
            m.settle()

            scr = run(m, "rewind 1")
            check("rewind refuses unstamped changes", "not stamped yet" in scr, scr)

            scr = run(m, "stamp fixed the todo")
            check("stamp makes #3", "Stamped #3: fixed the todo (1 change since #2)." in scr, scr)

            scr = run(m, "compare 1 3")
            check("compare 1 3 shows both changes",
                  "3 items changed from stamp #1 to stamp #3" in scr and "1 + set total to 1" in scr, scr)
            m.press("esc")
            m.settle()

            scr = run(m, "rewind #1")
            check("rewind #1 works", "Rewound the desk to stamp #1" in scr, scr)
            d = m.dump()
            check("desk is as in #1", "notes:todo" not in str(d) and "set total to 0" in str(d), str(d)[:600])

            scr = run(m, "bring desk:notes:todo 3")
            check("bring a page from a later stamp", "Brought back desk:notes:todo" in scr, scr)
            check("brought page has the text", "fixed" in str(m.dump()))

            scr = run(m, "stamp mixed")
            s = m._ask("STAMPS")
            check("stamp after rewind is made on #1", "S 4 1 2026-11-06 mixed" in s and s.startswith("BASE 4"), s)

            scr = run(m, "compare 1 3 notes")
            check("compare with a filter", "changed in desk:notes from stamp #1" in scr
                  and "desk:notes:todo" in scr and "tools" not in scr, scr)
            m.press("esc")
            m.settle()
            check("no start-file errors", not any(l.startswith("E ") for l in m._ask("STAMPS").splitlines()))
            scr = run(m, "compare x")
            check("bad stamp number is explained", "is not a stamp number" in scr, scr)
            check("OS still alive", m.alive())
        finally:
            m.stop()
    print("ALL OK" if not FAILS else f"{len(FAILS)} FAILED: {FAILS}")
    sys.exit(1 if FAILS else 0)


if __name__ == "__main__":
    main()
