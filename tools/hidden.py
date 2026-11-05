"""Re-run a player's chore, unchanged, on a task's hidden worlds.

For `fix` and `write` tasks (tasks_v3.py). Each hidden world is booted fresh
with only the chore page added; the chore is run with `do`, and the result is
judged by the task's independent checker. Only the declared outputs may
change: anything else the chore changed is a failure.
"""

import re
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import world_v3 as W  # noqa: E402
from tasks import parse_start  # noqa: E402
from unseen import Machine, SettleError  # noqa: E402


def other_changes(task, start, end):
    """Paths the chore changed that it was not asked to change."""
    page = task["chore_page"]
    bad = []
    for p in sorted(set(start) | set(end)):
        if p == page or start.get(p) == end.get(p) or p.startswith("desk:chores:"):
            continue
        if p in task.get("outputs", ()):
            continue
        if task.get("adds_mail") and p.startswith("desk:mail:sent:") and p not in start:
            continue
        bad.append(p)
    return bad


def steps_on(start_text, page, source):
    """Run a chore once on a start world: (finished, steps, screen). For validation."""
    with tempfile.TemporaryDirectory() as tmp:
        m = Machine.start(start_text + f"#page {page[len('desk:'):]}\n{source}\n#end\n", workdir=tmp)
        try:
            m.run_keys(f"do {page}{{enter}}")
            screen = m.settle()
        finally:
            m.stop()
    steps = re.search(r"Finished: (\d+) steps", screen)
    return bool(steps), int(steps.group(1)) if steps else None, screen


def run_hidden(task, source):
    """[{ok, why, steps}] for each hidden world."""
    if not isinstance(source, str):
        return [{"ok": False, "why": "the chore page is missing", "steps": None}]
    if any(line.startswith("#") for line in source.split("\n")):
        return [{"ok": False, "why": "a chore line starts with #", "steps": None}]
    page = task["chore_page"]
    out = []
    with tempfile.TemporaryDirectory() as tmp:
        for world in task["hidden"]:
            w = dict(world, pages=dict(world["pages"]))
            w["pages"][page[len("desk:"):]] = source
            start_text = W.render(w)
            m = Machine.start(start_text, workdir=tmp)  # a failure to boot is ours: it raises
            try:
                m.run_keys(f"do {page}{{enter}}")
                screen = m.settle()
                end = m.dump()
            except (SettleError, TimeoutError, RuntimeError) as e:
                if not m.os_crashed():
                    raise
                out.append({"ok": False, "why": f"the chore made the OS stop responding ({e})", "steps": None,
                            "os_crash": True})
                break  # a crash is final: the game is lost, nothing later may replace that
            finally:
                m.stop()
            start = parse_start(start_text)
            steps = re.search(r"Finished: (\d+) steps", screen)
            if not steps:
                line = next((l.strip() for l in screen.splitlines() if "Stopped" in l), "did not finish")
                out.append({"ok": False, "why": line, "steps": None})
                continue
            ok, why = task["check"](start, end, w["today"])
            extra = other_changes(task, start, end)
            if extra:
                ok, why = False, f"also changed {extra[:3]}"
            out.append({"ok": ok, "why": "ok" if ok else why, "steps": int(steps.group(1))})
    return out
