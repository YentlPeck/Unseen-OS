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


def planted_mistakes(start, end, flex):
    """Wrong end states that a strict scorer must reject."""
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
        if isinstance(v, str) and p not in flex:
            yield f"page {p} replaced by a drawer", {**end, p: None}
            if start.get(p) == v:
                yield f"blank line added to untouched {p}", {**end, p: v + "\n"}


def main():
    problems, expected, starts = [], {}, {}
    for tid, task in T.TASKS.items():
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
    for tid, task in T.TASKS.items():
        start, end = starts[tid], expected[tid]
        if not T.strict_check(tid, end)[0]:
            problems.append(f"{tid}: strict check rejects the reference end state")
        if T.strict_check(tid, start)[0]:
            problems.append(f"{tid}: strict check accepts the start state")
        for name, wrong in planted_mistakes(start, end, task["flex"]):
            total += 1
            if T.strict_check(tid, wrong)[0]:
                problems.append(f"{tid}: planted mistake not caught: {name}")
            else:
                caught += 1

    # Answer-wording cases for the one free-text answer page.
    locker = T.TASKS["locker"]["flex"]["desk:letters:reply-mira"]
    for text, ok in [("2391", True), ("The code is 2391.", True), ("2391\nYour bike lock is 5520.", True),
                     ("12391", False), ("23910", False), ("5520 or 2391", False), ("", False)]:
        if locker(text) != ok:
            problems.append(f"locker answer {text!r} should be {'accepted' if ok else 'rejected'}")

    print(f"{len(T.TASKS)} tasks, {caught}/{total} planted mistakes caught")
    for p in problems:
        print("PROBLEM:", p)
    print("ALL GOOD" if not problems else f"{len(problems)} problem(s)")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
