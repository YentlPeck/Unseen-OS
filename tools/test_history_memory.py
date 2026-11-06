"""Stamps under memory pressure, in the real OS (8 MB heap): the OS must keep working.

  python tools/test_history_memory.py

Start files hold 2000 pages and a page with 200,000 lines. We list the changes
against the huge page, then stamp again and again until the history is full,
and check that the OS still answers and still runs a chore after that.
"""

import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from unseen import Machine  # noqa: E402


def run(m, line):
    m.type(line)
    m.press("enter")
    return m.settle(limit=60)


def main():
    parts = ["#today 2026-11-06"]
    for i in range(2000):
        parts.append(f"#page d{i % 20}:p{i}\nline {i}\n#end")
    parts.append("#page big\n" + "\n" * 199_999 + "#end")
    parts.append("#page small\none\n#end")
    fails = []
    with tempfile.TemporaryDirectory() as tmp:
        m = Machine.start("\n".join(parts) + "\n", workdir=tmp)
        try:
            scr = run(m, "notes big")
            if "too many for Notes" not in scr:
                print(scr)
                fails.append("Notes refuses a page with too many lines")
            run(m, "write big now short")
            scr = run(m, "changes")
            print(scr)
            if "changed  desk:big" not in scr or "too long to compare" not in scr:
                fails.append("changes against the huge page")
            m.press("esc")
            m.settle()
            full_at = None
            for i in range(1, 100):
                run(m, f"write small {i}")
                scr = run(m, f"stamp n{i}")
                if "history is full" in scr:
                    full_at = i
                    break
                if f"Stamped #{i + 1}" not in scr:
                    print(scr)
                    fails.append(f"stamp {i}")
                    break
            print("history full after", full_at, "stamps")
            if full_at is None:
                fails.append("history never filled up")
            scr = run(m, "stamps")
            if "History used" not in scr:
                fails.append("stamps list")
            m.press("esc")
            m.settle()
            run(m, "write desk:count say 'still alive'")
            scr = run(m, "do desk:count")
            if "still alive" not in scr:
                print(scr)
                fails.append("chore after a full history")
            if not m.alive():
                fails.append("OS stopped answering")
        finally:
            m.stop()
    # A desk too big to stamp: the first stamp fails, says why, and the OS keeps working.
    many = "#today 2026-11-06\n" + "".join(f"#page p{i:05}\n#end\n" for i in range(20_000))
    with tempfile.TemporaryDirectory() as tmp:
        m = Machine.start(many, workdir=tmp)
        try:
            stamps = m._ask("STAMPS")
            print(stamps)
            if "were left out (more than 5000 in all" not in stamps or "S 1 -" not in stamps:
                fails.append("start files over the limit: left out, reported, desk still stamped")
            for k in range(20):
                scr = run(m, f"write desk:more{k} x")
                if "the desk is full (at most 5000 drawers and pages in all)" in scr:
                    break
            else:
                print(scr)
                fails.append("a full desk refuses new pages")
            scr = run(m, "changes")
            if "items changed since stamp #1" not in scr:
                print(scr)
                fails.append("changes on a full desk")
            m.press("esc")
            m.settle()
            run(m, "toss p00000")
            run(m, "write desk:count say 'still alive'")
            scr = run(m, "do desk:count")
            if "still alive" not in scr or not m.alive():
                print(scr)
                fails.append("OS keeps working with a full desk")
        finally:
            m.stop()

    # The heaviest case: 2000 pages with long names, a 900 KB page of 2999 lines in a
    # stamp, another 900 KB version of it on the desk, then list, bring and rewind.
    lines_a = "\n".join(f"{i:04} " + "a" * 293 for i in range(2999))
    lines_b = "\n".join(f"{i:04} " + "b" * 293 for i in range(2999))
    parts = ["#today 2026-11-06"]
    for i in range(2000):
        parts.append(f"#page drawer{i % 40:02}:{'n' * 18}{i:06}\n#end")
    parts.append("#stamp many pages")
    parts.append(f"#page big\n{lines_a}\n#end")
    parts.append("#stamp big page")
    parts.append(f"#page big\n{lines_b}\n#end")
    with tempfile.TemporaryDirectory() as tmp:
        m = Machine.start("\n".join(parts) + "\n", workdir=tmp)
        try:
            stamps = m._ask("STAMPS")
            print(stamps)
            if "S 2 1" not in stamps or "\nE " in stamps:
                fails.append("heavy start: 2 stamps, no errors")
            steps = [
                ("changes", "the rest is not shown"),
                ("compare 1", "the rest is not shown"),
                ("compare 1 2 big", "new page  desk:big"),
                ("stamps", "History used"),
            ]
            for line, want in steps:
                scr = run(m, line)
                m.press("end")
                scr += m.settle()
                if want not in scr:
                    print(scr)
                    fails.append(f"heavy: {line}")
                m.press("esc")
                m.settle()
            for line, want in [
                ("stamp b version", "history is full"),
                ("toss big", "Tossed"),
                ("bring big 2", "Brought back desk:big"),
                ("rewind 1", "Rewound the desk to stamp #1"),
                ("rewind 2", "Rewound the desk to stamp #2"),
                ("write desk:count say 'still alive'", "Wrote"),
                ("do desk:count", "still alive"),
            ]:
                scr = run(m, line)
                if want not in scr:
                    print(scr)
                    fails.append(f"heavy: {line}")
                    break
            if not m.alive():
                fails.append("heavy: OS stopped answering")
        finally:
            m.stop()
    print("ALL OK" if not fails else f"FAILED: {fails}")
    sys.exit(1 if fails else 0)


if __name__ == "__main__":
    main()
