"""Pilot M7: Haiku and Luna build, extend and fix small Unseen Script apps.

  python tools/pilot5.py validate          check the tasks (no AI): the start desk fails the
                                           fail-to-pass tests, the known solution (typed by a
                                           robot through the real harness) passes all, and
                                           every wrong solution is caught
  python tools/pilot5.py play PLAYER [TASK ...]   play games (PLAYER: haiku | luna | robot)
  python tools/pilot5.py report            a table of the games played

A pilot checks that the tasks are fair and the harness works; it is not an experiment, and its
tasks are never reused. The games use the existing referee, audit and clean player setup of
run_task.py (unchanged); only the tasks and the grading are new. Grading copies the player's
pages under desk:apps into a fresh desk and runs hidden test pages there with `test`, so the
app is checked through its real loop (launch and press), on data the player never saw.
"""

import hashlib
import json
import re
import shutil
import sys
import tempfile
import threading
import time
from pathlib import Path

TOOLS = Path(__file__).resolve().parent
ROOT = TOOLS.parent
sys.path.insert(0, str(TOOLS))
import run_task as R  # noqa: E402
from audit import audit  # noqa: E402
from broker import Broker  # noqa: E402
from tasks import parse_start  # noqa: E402
from tasks_apps import TASKS  # noqa: E402
from unseen import Machine, SettleError  # noqa: E402

OUT = ROOT / "out" / "pilot5"
MAX_ACTIONS = 200
TIME_LIMIT = 45 * 60
R.TIME_LIMIT = TIME_LIMIT  # run_player reads it at call time
MANUAL = ROOT / "docs" / "manual.md"
NL = "\n"
# What a player may change, added to every request (the grader checks exactly this).
SCOPE = (" You may keep extra pages of your own inside the drawer desk:apps (for example test pages "
         "or helper pages), and the page {data} may change while you try the app out. Everything "
         "outside those must stay exactly as it was.")


def hidden_titles(task):
    return re.findall(r"^test '([^']*)'", task["hidden"], re.M)


def run_hidden(task, files):
    """Run the hidden tests on a fresh desk with the player's pages under desk:apps.
    Gives (results {title: (ok, why)}, problem or None)."""
    parts = ["#today 2026-11-20", "#drawer apps"]
    for path, text in sorted(files.items()):
        if not path.startswith("desk:apps:") or text is None:
            continue
        if "#end" in text.split(NL):  # the start files' page delimiter
            return {}, f"{path} has a line that is just #end"
        parts.append(f"#page {path[5:]}{NL}{text}{NL}#end")
    parts.append(f"#page grader:hidden{NL}{task['hidden'].strip(NL)}{NL}#end")
    with tempfile.TemporaryDirectory() as tmp:
        m = Machine.start(NL.join(parts) + NL, workdir=tmp)
        try:
            m.type("test grader:hidden")
            m.press("enter")
            screen = m.settle(limit=120)
        except (SettleError, TimeoutError, RuntimeError) as e:
            if m.os_crashed():
                return {}, f"the hidden tests made the OS stop responding ({e})"
            raise
        finally:
            m.stop()
    # The verdict comes only from the title bar, which the OS writes itself ("N of M passed",
    # or "failed"): an app's own texts can't reach it. The PASS / FAIL lines below are only
    # read to say which tests failed (an app's mistake text could imitate them).
    rows = screen.split(NL)
    bar = re.search(r"(\d+) of (\d+) passed\s*$", rows[0]) if rows else None
    titles = hidden_titles(task)
    if not bar:
        stopped = re.search(r"Stopped: (.{0,200})", " ".join(l.strip() for l in rows))
        return {}, "the hidden tests did not run: " + (stopped.group(1) if stopped else rows[0].strip())
    passed, total = int(bar.group(1)), int(bar.group(2))
    if total != len(titles):
        return {}, f"the hidden test page has {len(titles)} tests but {total} ran"
    out = {}
    body = [l.strip() for l in rows[1:]]
    for title in titles:
        if passed == total:
            out[title] = (True, "")
        elif f"PASS {title}" in body:
            out[title] = (True, "(as the screen says)")
        else:
            why = next((l[len(f"FAIL {title}: "):] for l in body if l.startswith(f"FAIL {title}: ")), "failed")
            out[title] = (False, why[:160])
    if passed < total and all(ok for ok, _ in out.values()):
        return out, f"only {passed} of {total} hidden tests passed"
    return out, None


_MISSING = object()


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


def grade(task_id, files):
    """{passed, tests, problem, extra}: every hidden test passes and nothing outside the
    allowed pages changed."""
    task = TASKS[task_id]
    tests, problem = run_hidden(task, files)
    extra = other_changes(task, parse_start(task["start"]), files)
    passed = bool(tests) and all(ok for ok, _ in tests.values()) and not extra and not problem
    return {"passed": passed, "tests": {t: list(v) for t, v in tests.items()}, "problem": problem, "extra": extra}


# ---------------------------------------------------------------------------- games --

def play(task_id, player, condition="manual"):
    task = TASKS[task_id]
    manual_text = MANUAL.read_text(encoding="utf-8") if condition == "manual" else None
    R.check_condition(condition, manual_text)
    gid = f"{task_id}-{player}-{condition}-{hashlib.sha1(str(time.time()).encode()).hexdigest()[:6]}"
    run_dir, game_dir = OUT / "runs" / gid, R.GAMES_DIR / gid
    run_dir.mkdir(parents=True)
    game_dir.mkdir(parents=True)
    shutil.copy(TOOLS / "play.py", game_dir / "play.py")
    summary, commands, others, cli_error, timed_out, exit_code = "", [], [], None, False, 0
    final_error, files, tampered = None, None, []
    with open(run_dir / "qemu.log", "w") as qlog:
        machine = Machine.start(task["start"], workdir=run_dir, log=qlog)
        started = time.time()
        broker = Broker(machine, MAX_ACTIONS, run_dir / "transcript.jsonl", deadline=started + TIME_LIMIT)
        try:
            (game_dir / "game.json").write_text(json.dumps({"game": gid, "port": broker.port, "token": broker.token}))
            protected = {f: R.sha(game_dir / f) for f in ("play.py", "game.json")}
            prompt = R.PROMPT.format(gid=gid, max_actions=MAX_ACTIONS, request=task["request"] + SCOPE.format(data=task["data"]),
                                     manual=R.context_block(condition, manual_text), tips=R.V3_TIP)
            (run_dir / "prompt.txt").write_text(prompt, encoding="utf-8")
            if player == "robot":
                cmd = [sys.executable, str(TOOLS / "pilot_robot.py"), task_id]
                timed_out, exit_code = R.run_player(cmd, prompt, game_dir, run_dir / "player.log")
            else:
                cmd = R.player_command(player, gid, game_dir)
                cmd[0] = shutil.which(cmd[0]) or cmd[0]
                env, home = R.fresh_home(player, gid)
                try:
                    timed_out, exit_code = R.run_player(cmd, prompt, game_dir, run_dir / "player.log", env)
                finally:
                    R.end_home(player, home)
            broker.close()
            R.kill_leftovers(gid)
            if broker.os_crash is None:
                try:
                    machine.settle()
                    files = machine.dump()
                except Exception as e:
                    final_error = f"final check failed: {e!r}"
        except Exception as e:
            final_error = final_error or f"our side failed: {e!r}"
        finally:
            tampered = R.changed_files(game_dir, protected)
            for step in (broker.close, machine.stop):
                try:
                    step()
                except Exception as e:
                    final_error = final_error or f"clean-up failed: {e!r}"
            shutil.rmtree(game_dir, ignore_errors=True)
    events = "robot" if player == "robot" else player
    commands, others, summary, cli_error = R.player_events(
        events, (run_dir / "player.log").read_text(encoding="utf-8", errors="replace"))
    violations = audit(commands, gid) + others + [f"changed {f}" for f in tampered]
    if exit_code not in (0, None) and not timed_out and not cli_error:
        cli_error = f"player CLI exited with code {exit_code}"
    if player != "robot" and not cli_error:
        cli_error = R.infrastructure_problem(run_dir / "player.log", commands, summary, broker.used)
    if files is None:
        g = {"passed": False, "tests": {}, "problem": final_error or broker.os_crash, "extra": []}
    else:
        try:
            g = grade(task_id, files)
        except Exception as e:  # our side failed while grading: a harness error, not the player's
            final_error = final_error or f"grading failed: {e!r}"
            g = {"passed": False, "tests": {}, "problem": final_error, "extra": []}
    result = {"game": gid, "task": task_id, "player": player, "condition": condition,
              "passed": g["passed"] and not violations, "grade": g, "audit_ok": not violations,
              "violations": violations, "os_crash": broker.os_crash, "actions": broker.used,
              "seconds": round(time.time() - started), "timed_out": timed_out,
              "harness_errors": ([cli_error] if cli_error else []) + ([final_error] if final_error else [])
              + [f"infra: {e}" for e in broker.infra_errors],
              "summary": summary, "files": {p: t for p, t in (files or {}).items() if p.startswith(("desk:apps:", "desk:tally", "desk:todo", "desk:guests"))}}
    (run_dir / "result.json").write_text(json.dumps(result, indent=2))
    return result


def line(r):
    tests = r["grade"]["tests"]
    good = sum(1 for ok, _ in tests.values() if ok)
    flags = ("" if r["audit_ok"] else "  AUDIT FAILED") + ("  HARNESS: " + "; ".join(r["harness_errors"]) if r["harness_errors"] else "")
    extra = f"  changed {r['grade']['extra']}" if r["grade"]["extra"] else ""
    return (f"{'PASS' if r['passed'] else 'FAIL'}  {r['task']:<15} {r['player']:<6} {good}/{len(tests)} hidden tests, "
            f"{r['actions']} actions, {r['seconds']}s{extra}{flags}")


# ------------------------------------------------------------------------ validation --

def typed_desk(task_id, lines):
    """The desk after typing shell lines on the task's start desk."""
    with tempfile.TemporaryDirectory() as tmp:
        m = Machine.start(TASKS[task_id]["start"], workdir=tmp)
        try:
            for l in lines:
                m.type(l)
                m.press("enter")
                m.settle()
            return m.dump()
        finally:
            m.stop()


def visible_tests(task_id, files):
    """The task's own visible test page (if any) on a desk: the report's first line."""
    pages = [p for p in files if p.startswith("desk:apps:") and p.endswith("-tests")]
    if not pages:
        return None
    with tempfile.TemporaryDirectory() as tmp:
        start = "".join(f"#page {p[5:]}{NL}{t}{NL}#end{NL}" for p, t in files.items() if p.startswith("desk:apps:") and t is not None)
        m = Machine.start(start, workdir=tmp)
        try:
            m.type(f"test {pages[0]}")
            m.press("enter")
            return re.search(r"\d+ of \d+ tests passed", m.settle(limit=60)).group(0)
        finally:
            m.stop()


def validate():
    fails = []

    def check(name, ok, detail=""):
        print(("ok   " if ok else "FAIL ") + name + (f"  ({detail})" if detail and not ok else ""))
        if not ok:
            fails.append(name)

    for tid, task in TASKS.items():
        print(f"== {tid}")
        start = parse_start(task["start"])
        titles = hidden_titles(task)
        g = grade(tid, start)
        if task["f2p"] is None:
            check("the start desk fails every hidden test", not any(ok for ok, _ in g["tests"].values()), g)
        else:
            f2p = set(task["f2p"])
            check("f2p titles are hidden tests", f2p <= set(titles), f2p - set(titles))
            check("the start desk fails the fail-to-pass tests", all(not g["tests"][t][0] for t in f2p), g["tests"])
            check("the start desk passes the pass-to-pass tests",
                  all(g["tests"][t][0] for t in titles if t not in f2p), g["tests"])
            seen = visible_tests(tid, {p: t for p, t in start.items()})
            check("the visible tests pass on the start desk", seen and seen.split()[0] == seen.split()[2], seen)
        solved = typed_desk(tid, task["solution"])
        g = grade(tid, solved)
        check("the known solution, typed at the prompt, passes everything", g["passed"], g)
        seen = visible_tests(tid, solved)
        check("the visible tests still pass after the solution", seen is None or seen.split()[0] == seen.split()[2], seen)
        for name, pages in task["wrong"].items():
            files = dict(solved)
            files.update(pages)
            g = grade(tid, files)
            caught = [t for t, (ok, _) in g["tests"].items() if not ok]
            check(f"wrong solution '{name}' is caught", not g["passed"], g)
            print(f"       caught by: {caught}")
        files = dict(solved)
        files["desk:notes"] = "left behind"
        check("a page left outside the allowed places fails", not grade(tid, files)["passed"])
        for path, text in ((task["data"] + "-old", "x"), ("desk:notes", "<none>")):
            files = dict(solved)
            files[path] = text
            check(f"a new page {path} holding {text!r} fails", other_changes(task, start, files) == [path])
        # an app's own text can't fake a pass (its mistakes show on the test report)
        files = dict(solved)
        fake = "  ".join(f"PASS {t}" for t in titles)
        app = next(p for p in files if p.startswith("desk:apps:") and not p.endswith("-tests"))
        files[app] = files[app].replace("task start()\n", f"task start()\n  fail '{fake}'\n", 1)
        check("an app can't fake PASS lines", not grade(tid, files)["passed"])
        r = play(tid, "robot")
        check("the robot plays the solution through the real harness and passes", r["passed"], line(r))
    print("ALL GOOD" if not fails else f"{len(fails)} FAILED: {fails}")
    return 1 if fails else 0


def report():
    rows = [json.loads(p.read_text()) for p in sorted((OUT / "runs").glob("*/result.json"))]
    for r in rows:
        if r["player"] != "robot":
            print(line(r))


def main():
    if sys.argv[1] == "validate":
        return validate()
    if sys.argv[1] == "report":
        return report()
    if sys.argv[1] == "play":
        player = sys.argv[2]
        for tid in sys.argv[3:] or list(TASKS):
            print(line(play(tid, player)), flush=True)
        return 0
    print(__doc__)
    return 2


if __name__ == "__main__":
    sys.exit(main())
