"""Grading app tasks (experiment 5; the same rules as the pilot's tools/pilot5.py).

A task is a dict (tools/tasks_bench.py). Grading copies the player's pages under desk:apps into
a fresh desk (the BASE of the task set, without the player's data), runs the task's hidden
test page there with `test`, and fails any change outside the allowed pages.

  in QEMU (the real grading):  grade(task, files)
  on the PC (fast, while writing tasks):  grade(task, files, host=True)
"""

import re
import subprocess
import sys
import tempfile
from pathlib import Path

TOOLS = Path(__file__).resolve().parent
ROOT = TOOLS.parent
sys.path.insert(0, str(TOOLS))
from unseen import Machine, SettleError  # noqa: E402

NL = "\n"
GRADING_TODAY = "2026-11-20"
HOST_RUNNER = ROOT / "script" / "target" / "release" / "examples" / ("runtests.exe" if sys.platform == "win32" else "runtests")
_MISSING = object()


def hidden_titles(task):
    return re.findall(r"^test '([^']*)'", task["hidden"], re.M)


def grading_start(task, files):
    """Start files of the fresh grading desk, or (None, why) if a page can't be copied."""
    parts = [f"#today {GRADING_TODAY}", "#drawer apps"] + list(task.get("grading_extra", []))
    for path, text in sorted(files.items()):
        if not path.startswith("desk:apps:") or text is None:
            continue
        if "#end" in text.split(NL):  # the start files' page delimiter
            return None, f"{path} has a line that is just #end"
        parts.append(f"#page {path[5:]}{NL}{text}{NL}#end")
    parts.append(f"#page grader:hidden{NL}{task['hidden'].strip(NL)}{NL}#end")
    return NL.join(parts) + NL, None


def _verdict(task, summary, lines):
    """{title: (ok, why)} and a problem, from the summary 'N of M ...' and the report lines."""
    titles = hidden_titles(task)
    m = re.search(r"(\d+) of (\d+)", summary or "")
    if not m:
        return {}, "the hidden tests did not run: " + (summary or "").strip()[:200]
    passed, total = int(m.group(1)), int(m.group(2))
    if total != len(titles):
        return {}, f"the hidden test page has {len(titles)} tests but {total} ran"
    out = {}
    for title in titles:
        if passed == total:
            out[title] = (True, "")
        elif f"PASS {title}" in lines:
            out[title] = (True, "(as the report says)")
        else:
            why = next((l[len(f"FAIL {title}: "):] for l in lines if l.startswith(f"FAIL {title}: ")), "failed")
            out[title] = (False, why[:160])
    if passed < total and all(ok for ok, _ in out.values()):
        return out, f"only {passed} of {total} hidden tests passed"
    return out, None


def run_hidden_qemu(task, start):
    with tempfile.TemporaryDirectory() as tmp:
        m = Machine.start(start, workdir=tmp)
        try:
            m.type("test grader:hidden")
            m.press("enter")
            screen = m.settle(limit=120)
            rows = screen.split(NL)
            body = [l.strip() for l in rows[1:]]
            # a long report scrolls: read it to its end (the title bar says "lines A-B of C")
            for _ in range(20):
                more = re.search(r"lines \d+-(\d+) of (\d+)", rows[0])
                if not more or int(more.group(1)) >= int(more.group(2)):
                    break
                m.press(*["down"] * 20)
                rows = m.settle().split(NL)
                body += [l.strip() for l in rows[1:]]
        except (SettleError, TimeoutError, RuntimeError) as e:
            if m.os_crashed():
                return {}, f"the hidden tests made the OS stop responding ({e})"
            raise
        finally:
            m.stop()
    top = screen.split(NL)
    # The verdict comes only from the title bar, which the OS writes itself (an app's own
    # texts can't reach it): "N of M passed", maybe followed by "lines A-B of C" when the
    # report scrolls. The PASS / FAIL lines are only read to say which tests failed.
    bar = re.search(r"(\d+ of \d+) passed", top[0]) if top else None
    if not bar:
        stopped = re.search(r"Stopped: (.{0,200})", " ".join(l.strip() for l in top))
        return {}, "the hidden tests did not run: " + (stopped.group(1) if stopped else top[0].strip())
    return _verdict(task, bar.group(1), body)


def run_hidden_host(task, start):
    with tempfile.TemporaryDirectory() as tmp:
        f = Path(tmp) / "start.txt"
        f.write_text(start, encoding="utf-8")
        p = subprocess.run([str(HOST_RUNNER), str(f), "grader:hidden"], capture_output=True, text=True, encoding="utf-8")
    lines = p.stdout.rstrip(NL).split(NL)
    if p.returncode != 0 or not lines:
        return {}, f"host runner failed: {p.stderr.strip()[-300:]}"
    if lines[-1].startswith("Stopped"):
        return {}, "the hidden tests did not run: " + lines[-1]
    return _verdict(task, lines[-1], lines[:-1])


def allowed(task, path):
    """An allowed entry ending in ':' is a drawer and everything in it; any other is one page."""
    return any(path.startswith(a) if a.endswith(":") else path == a for a in task["allowed"])


def other_changes(task, start, end):
    bad = []
    for p in sorted(set(start) | set(end)):
        if start.get(p, _MISSING) is not _MISSING and p in end and start[p] == end[p]:
            continue
        if allowed(task, p):
            continue
        bad.append(p)
    return bad


def grade(task, files, start_files, host=False):
    """{passed, tests, problem, extra}: every hidden test passes and nothing outside the allowed
    pages changed. start_files: the desk the player started from (parse_start of the task)."""
    start, why = grading_start(task, files)
    if start is None:
        tests, problem = {}, why
    else:
        tests, problem = (run_hidden_host if host else run_hidden_qemu)(task, start)
    extra = other_changes(task, start_files, files)
    passed = bool(tests) and all(ok for ok, _ in tests.values()) and not extra and not problem
    return {"passed": passed, "tests": {t: list(v) for t, v in tests.items()}, "problem": problem, "extra": extra}


# ------------------------------------------------- line commands, on the PC (for writing tasks) --

def apply_commands(files, lines):
    """The desk after the shell lines insert / edit / cut (as kernel/src/programs.rs does them;
    only for the host checks while writing tasks: the QEMU validation types them for real)."""
    files = dict(files)
    for l in lines:
        word, rest = l.split(" ", 1)
        path, rest = rest.split(" ", 1) if " " in rest else (rest, "")
        path = path if path.startswith("desk:") else "desk:" + path
        n, text = (rest.split(" ", 1) + [""])[:2] if word != "cut" else (rest, "")
        page = files.get(path)
        rows = page.split(NL) if page is not None else None
        if word == "insert":
            if rows is None:
                assert n == "1", l
                rows = [text]
            else:
                rows.insert(int(n) - 1, text)
        elif word == "edit":
            rows[int(n) - 1] = text
        elif word == "cut":
            a, *b = n.split(" ")
            a, b = int(a), int(b[0]) if b else int(a)
            del rows[a - 1:b]
            rows = rows or [""]
        else:
            raise ValueError(l)
        files[path] = NL.join(rows)
    return files
