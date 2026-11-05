"""Prove every task is fair, and record the expected end states.

  python tools/validate_tasks.py

For each task this:
  1. boots the OS with the start files and checks the OS built exactly the
     desk the Python parser expects,
  2. runs the reference solution and records the end state (expected.json),
  3. checks the hand-written intent test: false at the start, true at the end,
  4. checks the STRICT scorer: start fails, end passes, and every planted
     mistake fails (extra page, blanked page, original left behind, result missing).
"""

import json
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import tasks as T  # noqa: E402
from hidden import run_hidden, steps_on  # noqa: E402

STEP_MARGIN = 25_000  # reference chores must use at most half of the 50000-step limit
from unseen import Machine  # noqa: E402


def run_reference(task_id):
    task = T.TASKS[task_id]
    with tempfile.TemporaryDirectory() as tmp:
        m = Machine.start(task["start"], workdir=tmp)
        try:
            start = m.dump()
            m.run_keys(task["solution"])
            m.settle()
            end = m.dump()
        finally:
            m.stop()
    return start, end


def planted_mistakes(start, end, flex, task=None):
    """Wrong end states that a strict scorer must reject."""
    if task and task.get("v3"):  # the scratch drawer's pages are ignored on purpose
        keep = task.get("chore_page")
        scratch = {p for p in set(start) | set(end) if p.startswith("desk:chores:") and p != keep}
        start = {k: v for k, v in start.items() if k not in scratch}
        end = {k: v for k, v in end.items() if k not in scratch}
        for p, v in end.items():
            if p.startswith("desk:mail:sent:") and p not in start:
                yield f"mail {p} sent twice", {**end, p + "x": v}
                break
    yield "extra page", {**end, "desk:zz-extra": "x"}
    for p, v in end.items():
        if isinstance(v, str) and p not in flex and start.get(p) != v and v.strip():
            yield f"blanked {p}", {**end, p: ""}
    for p, v in start.items():
        if p not in end and p not in flex:
            yield f"left behind {p}", {**end, p: v}
    for p in end:
        if p not in start and p not in flex:
            yield f"missing {p}", {k: v for k, v in end.items() if k != p and not k.startswith(p + ":")}
    for p, v in end.items():
        if p.startswith("desk:calendar:") and isinstance(v, str):
            yield f"event {p} added twice", {**end, p + "x": v}
    untouched = 0
    for p, v in end.items():
        if isinstance(v, str) and p not in flex:
            if start.get(p) == v:
                untouched += 1
                if task and task.get("v3") and untouched % 25:  # big worlds: a sample of untouched pages
                    continue
                yield f"blank line added to untouched {p}", {**end, p: v + "\n"}
            yield f"page {p} replaced by a drawer", {**end, p: None}


def check_v3(tid, task, problems):
    """Version 3: reference chores stay well inside the step limit; fix/write chores work on
    every hidden world, and every wrong chore fails on at least one."""
    page = task.get("chore_page", "desk:chores:job")
    long = [line for line in task["ref_chore"].split("\n") if len(line) > 78]
    if long:
        problems.append(f"{tid}: reference chore has lines Notes can't hold (over 78): {long}")
    start = task["start"]
    if page[len("desk:"):] in start:  # a fix task: run the fixed chore instead of the broken one
        start = start.split(f"#page {page[len('desk:'):]}\n")[0]
    finished, steps, screen = steps_on(start, page, task["ref_chore"])
    if not finished:
        problems.append(f"{tid}: reference chore did not finish on the visible world: {screen[:300]}")
    elif steps > STEP_MARGIN:
        problems.append(f"{tid}: reference chore needs {steps} steps (more than {STEP_MARGIN})")
    info = f"{steps} steps"
    if "baseline" in task and tid in T.TASK_SETS["v3"]:
        from tasks_v3 import BASELINE_MIN
        info += f", no-chore baseline >= {task['baseline']} actions"
        if task["baseline"] < BASELINE_MIN:
            problems.append(f"{tid}: can be done without chores in about {task['baseline']} actions "
                            f"(must be at least {BASELINE_MIN})")
    if task.get("hidden"):
        res = run_hidden(task, task["ref_chore"])
        if not all(r["ok"] for r in res):
            problems.append(f"{tid}: reference chore fails a hidden world: {[r['why'] for r in res]}")
        if any(r["steps"] and r["steps"] > STEP_MARGIN for r in res):
            problems.append(f"{tid}: reference chore too slow on a hidden world")
        caught = 0
        for name, src in task["wrong_chores"]:
            if all(r["ok"] for r in run_hidden(task, src)):
                problems.append(f"{tid}: wrong chore passes every hidden world: {name}")
            else:
                caught += 1
        info += f", hidden worlds ok, {caught}/{len(task['wrong_chores'])} wrong chores caught"
    print(f"  {tid}: {info}")
    return len(task.get("wrong_chores", [])), caught if task.get("hidden") else 0


def main():
    only = sys.argv[1] if len(sys.argv) > 1 else None
    todo = T.TASK_SETS[only] if only else list(T.TASKS)
    problems, starts = [], {}
    expected = json.loads(T.EXPECTED_FILE.read_text()) if only and T.EXPECTED_FILE.exists() else {}
    for tid in todo:
        task = T.TASKS[tid]
        lines = task["start"].splitlines()
        if not task["start"].isascii() or any(len(l) > 76 for l in lines):
            problems.append(f"{tid}: start files must be plain ASCII with lines of at most 76 characters")
        start, end = run_reference(tid)
        starts[tid] = start
        expected[tid] = end
        if T.canon(start) != T.canon(T.parse_start(task["start"])):
            problems.append(f"{tid}: OS desk differs from the parsed start files")
        if task["intent"](start):
            problems.append(f"{tid}: intent already true at start")
        if not task["intent"](end):
            problems.append(f"{tid}: reference solution fails the intent test")

    T.EXPECTED_FILE.write_text(json.dumps(expected, indent=1, sort_keys=True))
    T._expected = None  # reload

    caught = total = 0
    for tid in todo:
        task = T.TASKS[tid]
        start, end = starts[tid], expected[tid]
        if not T.strict_check(tid, end)[0]:
            problems.append(f"{tid}: strict check rejects the reference end state")
        if T.strict_check(tid, start)[0]:
            problems.append(f"{tid}: strict check accepts the start state")
        for name, wrong in planted_mistakes(start, end, task["flex"], task):
            total += 1
            if T.strict_check(tid, wrong)[0]:
                problems.append(f"{tid}: planted mistake not caught: {name}")
            else:
                caught += 1
        # Hand-written answer variants: these must be accepted / rejected.
        for path, text in task.get("right", []):
            if not T.strict_check(tid, {**end, path: text})[0]:
                problems.append(f"{tid}: correct answer rejected: {text!r}")
        for path, text in task.get("wrong", []):
            total += 1
            if T.strict_check(tid, {**end, path: text})[0]:
                problems.append(f"{tid}: wrong answer accepted: {text!r}")
            else:
                caught += 1

    # Answer-wording cases for the one free-text answer page.
    if "locker" in todo:
        locker = T.TASKS["locker"]["flex"]["desk:letters:reply-mira"]
        for text, ok in [("2391", True), ("The code is 2391.", True), ("2391\nYour bike lock is 5520.", True),
                         ("12391", False), ("23910", False), ("5520 or 2391", False), ("", False)]:
            if locker(text) != ok:
                problems.append(f"locker answer {text!r} should be {'accepted' if ok else 'rejected'}")

    # Version 3: step limits, hidden worlds and wrong chores.
    v3 = [tid for tid in todo if T.TASKS[tid].get("v3")]
    if v3:
        print("version 3 chores:")
    for tid in v3:
        n, c = check_v3(tid, T.TASKS[tid], problems)
        total += n
        caught += c

    print(f"{len(todo)} tasks, {caught}/{total} planted mistakes caught")
    for p in problems:
        print("PROBLEM:", p)
    print("ALL GOOD" if not problems else f"{len(problems)} problem(s)")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
