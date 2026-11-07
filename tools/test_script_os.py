"""End-to-end test of Unseen Script in the real OS (milestone M4), in QEMU.

  python tools/test_script_os.py

A robot types a two-page program with insert/edit/cut, tests it with test, runs it with
run, tries lines in the lab, uses the Notes upgrades (line numbers, Ctrl+G, Ctrl+F), and
finally runs the deepest programs the limits allow and checks the kernel stack and heap
(STACK and HEAP on the inspector).
"""

import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from unseen import Machine  # noqa: E402

STACK_SIZE = 256 * 1024
FAILS = []


def check(name, ok, detail=""):
    print(("ok   " if ok else "FAIL ") + name)
    if not ok:
        FAILS.append(name)
        print(detail)


def run(m, line, limit=30):
    m.type(line)
    m.press("enter")
    return m.settle(limit=limit)


def back(m):
    m.press("esc")
    return m.settle()


def page(m, path):
    dump = m.dump()
    return dump.get(path) if isinstance(dump, dict) else None


DOWN = [("calls", 0, 0), ("whens", 14, 0), ("brackets", 0, 10), ("mixed", 8, 6), ("most", 14, 10)]


NL = "\n"


def down_task(whens, brackets):
    return NL.join(
        ["task down(n)"] + ["when yes"] * whens + ["when n is 0", "give 0", "done"]
        + ["give " + "(1 + " * brackets + "down(n - 1)" + ")" * brackets]
        + ["done"] * whens + ["done"]
    )


def deep_pages():
    """The deepest programs the limits allow (tasks, blocks, brackets, values, try), as
    library pages (deeplib:*), programs (deep:*) and test pages (deeptest:*)."""
    out = {}
    for name, whens, brackets in DOWN:
        out[f"deeplib:{name}"] = down_task(whens, brackets)
        out[f"deep:{name}"] = NL.join([f"use 'deeplib:{name}'", "say down(100)"])
        out[f"deeptest:{name}"] = NL.join([f"use 'deeplib:{name}'", "test 'deep'", "  check down(100) is 0", "done"])
    out["deep:values"] = NL.join(["set v to [1]", "repeat 15 times", "set v to [v]", "done",
                                  "say v is v", "say v", "say count(v)"])
    out["deep:chain"] = "say " + " + ".join(["1"] * 47)
    out["deep:tries"] = NL.join(
        ["task t(n)"] + ["try"] * 14 + ["when n is 0", "give 0", "done", "give t(n - 1)"]
        + ["instead e", "fail e", "done"] * 14 + ["done", "say t(100)"]
    )
    return out


def start_files():
    # an empty desk is stamp #1: every page below is a change since it (re-check of M4)
    parts = ["#today 2026-11-07", "#stamp 2026-11-01 empty desk", "#drawer tools"]
    parts.append("#page hist:wide" + NL + NL.join(["x" * 69] * 3000) + NL + "#end")
    parts.append("#page hist:blank" + NL * 5000 + "#end")
    parts.append("#page wideout" + NL + NL.join(["repeat 64 times", "  say pad('x', 7600)", "done"]) + NL + "#end")
    for path, text in deep_pages().items():
        parts.append(f"#page {path}{NL}{text}{NL}#end")
    # a page of 700000 empty lines, and a program that says very many empty lines (review M4)
    parts.append("#page big" + NL * 700_000 + "#end")
    parts.append("#page many" + NL + NL.join([
        "set t to newline()", "repeat 12 times", "  set t to t & t", "done",
        "repeat 240 times", "  say t", "done"]) + NL + "#end")
    return NL.join(parts) + NL


def main():
    with tempfile.TemporaryDirectory() as tmp:
        m = Machine.start(start_files(), workdir=tmp)
        try:
            scr = run(m, "guide")
            check("guide lists the Programs words", "Programs" in scr and "lab" in scr and "insert" in scr, scr)

            # ---- review M4: a huge page (first, then tossed: it makes DUMP slow) ----
            scr = run(m, "view big 1 1", limit=60)
            check("view of a 699999-line page", "lines 1-1 of 699999" in scr, scr)
            scr = run(m, "edit big 1 x", limit=60)
            check("edit of a 699999-line page", "Line 1 of desk:big is now:" in scr, scr)
            scr = run(m, "insert big 2 y", limit=60)
            check("insert into a 699999-line page", "(700000 lines now)" in scr, scr)
            scr = run(m, "cut big 1 2", limit=60)
            check("cut of a 700000-line page", "(699998 lines now)" in scr, scr)
            scr = run(m, "run many", limit=120)
            check("an output of very many lines is cut", "Finished:" in scr and m.alive(), scr)
            m.press("end")
            scr = m.settle()
            check("the cut is said", "more lines are not shown" in scr, scr)
            back(m)
            run(m, "toss big", limit=60)
            scr = run(m, "run wideout", limit=120)
            m.press("end")
            scr = m.settle()
            import re
            total = re.search(r"lines \d+-\d+ of (\d+)", scr)
            check("very long lines are cut at the screen limit too",
                  "more lines are not shown" in scr and total and int(total.group(1)) <= 6401, scr)
            back(m)
            scr = run(m, "changes", limit=120)
            m.press("end")
            scr = m.settle()
            check("a cut history listing still shows how to see less", "To see less at once" in scr, scr)
            back(m)

            # ---- a library page, typed line by line with insert ----
            for i, line in enumerate([
                "-- money tools",
                "fixed rate is 21",
                "task tax(amount)",
                "  give amount * rate / 10",
                "done",
            ]):
                scr = run(m, f"insert tools:money {i + 1} {line}")
            check("insert makes the page and adds lines", "Put in line 5 of desk:tools:money (5 lines now)" in scr, scr)
            check("insert keeps the indent", page(m, "desk:tools:money").split("\n")[3] == "  give amount * rate / 10",
                  page(m, "desk:tools:money"))

            scr = run(m, "view tools:money")
            check("view shows numbered lines", "desk:tools:money lines 1-5 of 5:" in scr and "   4    give amount * rate / 10" in scr, scr)

            # ---- a program page that uses it ----
            run(m, "insert tools:main 1 use 'tools:money'")
            run(m, "insert tools:main 2 say 'oops'")
            run(m, "insert tools:main 3 say 'tax: ' & tax(200)")
            run(m, "insert tools:main 4 write('desk:answer', tax(200))")
            scr = run(m, "cut tools:main 2")
            check("cut removes a line", "Cut line 2 of desk:tools:main (3 lines now)." in scr, scr)

            # ---- tests for the library ----
            for i, line in enumerate([
                "use 'tools:money'",
                "test 'tax of 100'",
                "  check tax(100) is 21",
                "done",
                "test 'the rate'",
                "  check rate is 21",
                "done",
            ]):
                run(m, f"insert tools:checks {i + 1} {line}")
            scr = run(m, "test tools:checks")
            check("test shows a failing test with both values",
                  "1 of 2 tests passed" in scr and "FAIL tax of 100: line 3: check failed: got 210, expected 21" in scr
                  and "PASS the rate" in scr, scr)
            back(m)

            # ---- fix the bug with edit, test again, run ----
            scr = run(m, "edit tools:money 4   give amount * rate / 100")
            check("edit replaces one line", "Line 4 of desk:tools:money is now:" in scr, scr)
            scr = run(m, "test tools:checks")
            check("the tests pass after the fix", "2 of 2 tests passed" in scr and "FAIL" not in scr, scr)
            back(m)
            scr = run(m, "run tools:main")
            check("run finishes and says its lines", "Finished:" in scr and "tax: 42" in scr and "wrote desk:answer" in scr, scr)
            back(m)
            check("run made its change", page(m, "desk:answer") == "42", page(m, "desk:answer"))

            # ---- a mistake on the used page names it, and changes nothing ----
            run(m, "edit tools:money 4   give amount * rate / 0")
            scr = run(m, "run tools:main")
            check("a mistake names the used page and the calling line",
                  "Stopped: desk:tools:money line 4: dividing by 0" in scr and "called from desk:tools:main line 2" in scr
                  and "Nothing was changed." in scr, scr)
            back(m)
            run(m, "edit tools:money 4   give amount * rate / 100")

            # ---- the command's own mistakes ----
            scr = run(m, "edit tools:money 9 x")
            check("edit refuses a line that isn't there", "there is no line 9; the page has lines 1-5" in scr, scr)
            scr = run(m, "cut tools:money two")
            check("cut refuses a word for a number", "'two' is not a line number" in scr, scr)
            scr = run(m, "insert tools:money 1 " + "x" * 79)
            check("insert refuses a line longer than 78", "a line is at most 78" in scr, scr)
            scr = run(m, "run mail:inbox")
            check("run refuses a drawer", "Could not do that" in scr, scr)

            # ---- the lab ----
            scr = run(m, "lab")
            check("lab opens", "Lab - try Unseen Script" in scr, scr)
            scr = run(m, "1 + 2")
            check("lab shows a value", ">   1 + 2\n 3" in scr, scr)
            run(m, "use 'tools:money'")
            scr = run(m, "tax(100)")
            check("lab keeps use lines", ">   tax(100)\n 21" in scr, scr)
            run(m, "set items to ['milk']")
            scr = run(m, "items")
            check("lab keeps names", " ['milk']" in scr, scr)
            scr = run(m, "write('desk:labpage', 'x')")
            check("lab only shows changes", "would write desk:labpage" in scr, scr)
            run(m, "when yes")
            scr = run(m, "  say 'in a block'")
            check("lab waits for the end of a block", "... " in scr and "in a block\n" not in scr.split("...")[-1], scr)
            scr = run(m, "done")
            check("lab runs the block at done", " in a block" in scr, scr)
            scr = run(m, "1 / 0")
            check("lab shows a mistake", "mistake: dividing by 0 is not possible" in scr, scr)
            back(m)
            check("lab made no change", page(m, "desk:labpage") is None)

            # ---- Notes: line numbers, go to line, find ----
            scr = run(m, "notes tools:money")
            check("Notes shows line numbers", "   1 -- money tools" in scr and "   4   give amount" in scr, scr)
            m.press("ctrl+g")
            m.settle()
            m.type("4")
            m.press("enter")
            scr = m.settle()
            check("Ctrl+G goes to a line", "line 4/5" in scr, scr)
            m.press("ctrl+f")
            m.settle()
            m.type("rate")
            m.press("enter")
            scr = m.settle()
            check("Ctrl+F finds a word from the cursor on", "Found 'rate' on line 4." in scr, scr)
            m.press("ctrl+f")
            m.press("enter")
            scr = m.settle()
            check("Ctrl+F Enter finds the next, going round", "Found 'rate' on line 2." in scr, scr)
            back(m)

            run(m, "lab")
            for line in ["set t to newline()", "repeat 12 times", "  set t to t & t", "done", "repeat 240 times", "  say t"]:
                run(m, line, limit=60)
            scr = run(m, "done", limit=120)
            check("the lab shows a say of very many lines", m.alive() and "Lab - try" in scr, scr)
            back(m)
            run(m, "insert empty 1")
            scr = run(m, "insert empty 2 say 1")
            check("an empty line is a line", "(2 lines now)" in scr, scr)
            scr = run(m, "cut empty 1 2")
            check("cutting every line leaves an empty page", "it is empty now" in scr, scr)
            heap = int(m._ask("HEAP"))
            print(f"     heap in use after these: {heap // 1024} KB")

            # ---- the deepest programs: the stack and the heap ----
            before = int(m._ask("STACK"))
            for path in deep_pages():
                if path.startswith("deeplib:"):
                    continue
                word = "test" if path.startswith("deeptest:") else "run"
                scr = run(m, f"{word} {path}", limit=120)
                body = [l for l in scr.split(NL)[1:] if l.strip()]
                print(f"     {word} {path}: {body[0].strip()[:90] if body else '?'}")
                check(f"{path} ends with a result or a mistake, not a crash",
                      m.alive() and any(w in scr for w in ("Finished:", "Stopped:", "tests passed")), scr)
                back(m)
            for name, _, _ in DOWN:
                run(m, "lab")  # a fresh lab for each: every page has its own task down
                run(m, f"use 'deeplib:{name}'", limit=120)
                scr = run(m, "down(100)", limit=120)
                check(f"the lab runs deeplib:{name}", m.alive() and "mistake:" in scr, scr)
                back(m)
            stack = int(m._ask("STACK"))
            heap = int(m._ask("HEAP"))
            print(f"     stack used at most {stack // 1024} KB of {STACK_SIZE // 1024} KB (before the deep programs: {before // 1024} KB); heap in use {heap // 1024} KB")
            check("the deepest programs leave at least half the stack free", stack < STACK_SIZE // 2, f"{stack}")
            check("the OS still answers", m.alive())
        finally:
            m.stop()
    print("ALL GOOD" if not FAILS else f"{len(FAILS)} FAILED: {FAILS}")
    return 1 if FAILS else 0


if __name__ == "__main__":
    sys.exit(main())
