"""Experiment 5 tasks: validation (milestone B1 of docs/benchmark-design.md).

  python tools/bench5.py validate [--host] [TASK ...]

For every task, without any AI:
  * the start desk fails the fail-to-pass hidden tests and passes the others (pass-to-pass);
  * the visible test page (feature and fix tasks) passes on the start desk and after the fix;
  * the known solution, typed at the prompt, passes every hidden test;
  * every wrong solution is caught;
  * a page left outside the allowed places fails.
--host runs the tests on the PC (fast, while writing tasks; line commands applied in Python);
without it everything runs in QEMU, the solution typed for real.
"""

import re
import subprocess
import sys
import tempfile
from pathlib import Path

TOOLS = Path(__file__).resolve().parent
sys.path.insert(0, str(TOOLS))
import apps_grade as G  # noqa: E402
from tasks import parse_start  # noqa: E402
from tasks_bench import TASKS as BENCH  # noqa: E402
from tasks_calib import TASKS as CALIB  # noqa: E402

TASKS = {**BENCH, **CALIB}
from unseen import Machine  # noqa: E402

NL = "\n"


def typed_desk(task, lines):
    with tempfile.TemporaryDirectory() as tmp:
        m = Machine.start(task["start"], workdir=tmp)
        try:
            for l in lines:
                m.type(l)
                m.press("enter")
                screen = m.settle()
                if "Could not do that" in screen or "there is no line" in screen:
                    raise RuntimeError(f"the solution line was refused: {l}\n{screen}")
            return m.dump()
        finally:
            m.stop()


def visible_report(files, host):
    """The first line of the visible test page's report on a desk (None if there is none)."""
    pages = [p for p in files if p.startswith("desk:apps:") and p.endswith("-tests")]
    if not pages:
        return None
    start = "".join(f"#page {p[5:]}{NL}{t}{NL}#end{NL}" for p, t in files.items() if p.startswith("desk:apps:") and t is not None)
    if host:
        with tempfile.TemporaryDirectory() as tmp:
            f = Path(tmp) / "start.txt"
            f.write_text(start, encoding="utf-8")
            out = subprocess.run([str(G.HOST_RUNNER), str(f), pages[0]], capture_output=True, text=True).stdout
        return out.strip().split(NL)[-1]
    with tempfile.TemporaryDirectory() as tmp:
        m = Machine.start(start, workdir=tmp)
        try:
            m.type(f"test {pages[0]}")
            m.press("enter")
            return re.search(r"\d+ of \d+ tests passed|Stopped.*", m.settle(limit=60)).group(0)
        finally:
            m.stop()


def all_passed(report):
    m = re.match(r"(\d+) of (\d+) tests passed", report or "")
    return bool(m) and m.group(1) == m.group(2)


def validate(names, host):
    fails = []

    def check(name, ok, detail=""):
        print(("ok   " if ok else "FAIL ") + name + (f"  ({detail})" if detail and not ok else ""), flush=True)
        if not ok:
            fails.append(f"{tid}: {name}")

    for tid in names:
        task = TASKS[tid]
        print(f"== {tid} ({task['kind']}, {task['level']})")
        start = parse_start(task["start"])
        titles = G.hidden_titles(task)
        long = [l for l in task["solution"] if len(l.split(" ", 3)[-1]) > 78]
        check("solution lines fit the 78-character limit", not long, long[:1])
        g = G.grade(task, start, start, host)
        if task["f2p"] is None:
            check("the start desk fails every hidden test", g["tests"] == {} or not any(ok for ok, _ in g["tests"].values()), g)
        else:
            f2p = set(task["f2p"])
            check("f2p titles are hidden tests", f2p <= set(titles), f2p - set(titles))
            check("the start desk fails the fail-to-pass tests", all(t in g["tests"] and not g["tests"][t][0] for t in f2p), g)
            check("the start desk passes the pass-to-pass tests", all(g["tests"].get(t, [False])[0] for t in titles if t not in f2p), g)
            seen = visible_report(start, host)
            if task["kind"] == "feature":
                check("the visible tests pass on the start desk", all_passed(seen), seen)
            else:  # a bug may show in the visible tests too
                print(f"       visible tests on the start desk: {seen}")
        solved = G.apply_commands(start, task["solution"]) if host else typed_desk(task, task["solution"])
        for p, want in task.get("solved", {}).items():
            check(f"the solution's commands make {p} as intended", solved.get(p, "").strip(NL) == want.strip(NL))
        g = G.grade(task, solved, start, host)
        check("the known solution passes every hidden test", g["passed"], g)
        if task["kind"] != "build":
            check("the visible tests still pass after the solution", all_passed(visible_report(solved, host)), visible_report(solved, host))
        for name, pages in task["wrong"].items():
            files = dict(solved)
            files.update({p: t.strip(NL) for p, t in pages.items()})
            g = G.grade(task, files, start, host)
            caught = [t for t, (ok, _) in g["tests"].items() if not ok]
            check(f"wrong solution '{name}' is caught", not g["passed"], g)
            print(f"       caught by: {caught or g['problem']}")
        # side effects the request never asked for, put into the known solution's start()
        for p, app in task.get("solved", {}).items():
            key = next(l for l in app.split(NL) if l.startswith("task key("))
            for where, head in (("start", "task start()"), ("key", key)):
                for name, line in (("adds an event", "  event('2026-12-01', '10:00', 'X', '')"),
                                   ("writes an extra page", "  write('desk:unasked', 'x')")):
                    files = dict(solved)
                    files[p] = app.strip(NL).replace(head + NL, head + NL + line + NL, 1)
                    g = G.grade(task, files, start, host)
                    check(f"a solution that {name} in {where} is caught", not g["passed"], g)
        files = dict(solved)
        files["desk:notes-left"] = "left behind"
        check("a page left outside the allowed places fails", not G.grade(task, files, start, host)["passed"])
    print("ALL GOOD" if not fails else f"{len(fails)} FAILED:" + "".join(f"\n  {f}" for f in fails))
    return 1 if fails else 0


def main():
    args = sys.argv[1:]
    if args and args[0] == "validate":
        host = "--host" in args
        names = [a for a in args[1:] if not a.startswith("--")] or list(CALIB if "--calib" in args else BENCH)
        return validate(names, host)
    print(__doc__)
    return 2


if __name__ == "__main__":
    sys.exit(main())
