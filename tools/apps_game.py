"""One app game of experiment 5 (docs/benchmark-design.md): a player builds, extends or fixes an
Unseen Script app; the hidden tests grade it.

  python tools/apps_game.py TASK PLAYER [CONDITION]    (PLAYER: haiku | luna | robot)

How a game is kept fair is run_task.py's way (unchanged, frozen for experiments 1-4): the
player's folder holds only the game tool and a ticket; the referee is the only thing connected
to the OS; every command is audited; the player gets a clean setup with only its login. The app
games add (pilot round 2, docs/pilot5.md): the referee of tools/broker_apps.py (the `line`
action), the player's tool tools/play_apps.py, the app prompt with the one-command rule, and
the task's scope sentence. Grading: tools/apps_grade.py (hidden tests on a fresh desk).

Records go to $UNSEEN_APPS_RUNS/<game id>/ (default out/apps-dev/runs).
"""

import hashlib
import json
import os
import re
import shutil
import sys
import time
import uuid
from pathlib import Path

TOOLS = Path(__file__).resolve().parent
ROOT = TOOLS.parent
sys.path.insert(0, str(TOOLS))
import apps_grade as G  # noqa: E402
import run_task as R  # noqa: E402
from audit_apps import audit_apps  # noqa: E402
from broker_apps import AppBroker  # noqa: E402
from tasks import parse_start  # noqa: E402
from tasks_bench import TASKS, scope  # noqa: E402
from unseen import Machine  # noqa: E402

RUNS = Path(os.environ.get("UNSEEN_APPS_RUNS") or ROOT / "out" / "apps-dev" / "runs")
MAX_ACTIONS = 200
TIME_LIMIT = 45 * 60
CONDITIONS = ("plain", "reference", "manual")
INTROS = {"manual": "Here is the manual for this computer:",
          "reference": "Here is the reference of Unseen Script, the programming language of this computer "
                       "(the text the command guide script shows):"}
REFERENCE = ROOT / "kernel" / "src" / "script_guide.txt"
MANUAL = ROOT / "docs" / "manual.md"

PROMPT = """You are using a computer you have never seen before. You can only use it through one tool,
run from the current folder:

  python play.py {gid} look              see the screen
  python play.py {gid} type "some text"  type text on one line (this does NOT press Enter)
  python play.py {gid} line "some text"  type text on one line, then press Enter (one action)
  python play.py {gid} press KEY [KEY]   press keys: enter, esc, backspace, tab, up, down,
                                         left, right, home, end, delete, space, or ctrl+<letter>

Every action shows you the screen afterwards. You have at most {max_actions} actions.
Run every action as a command of its own: exactly one python play.py {gid} ... per command, with
the game name written out in full (no shell variables, no ; or && between calls, no loops).
Do not run any other commands and do not read any files.
When the text you type contains quote marks, put double quotes around the whole text and use single quotes inside it.
{context}
Your task: {request}

When the task is done (or you cannot finish it), stop and give a one-line summary."""


def context_text(condition):
    """The text a condition puts in the prompt (None for plain)."""
    if condition not in CONDITIONS:
        raise ValueError(f"unknown condition {condition!r}")
    if condition == "plain":
        return None
    return (MANUAL if condition == "manual" else REFERENCE).read_text(encoding="utf-8")


def context_block(condition, text):
    return f"\n{INTROS[condition]}\n\n{text}\n" if condition != "plain" else ""


def sha256(text):
    return hashlib.sha256(text.encode("utf-8")).hexdigest() if text else None


def new_game_id(task_id, player, condition):
    return f"{task_id}-{player}-{condition}-{uuid.uuid4().hex[:6]}"


def load_transcript(gid):
    try:
        rows = [json.loads(l) for l in (RUNS / gid / "transcript.jsonl").read_text(encoding="utf-8").splitlines() if l.strip()]
        return [t for t in rows if isinstance(t, dict) and "action" in t]
    except Exception:
        return []


def saved_result(gid):
    path = RUNS / gid / "result.json"
    if not path.exists():
        return None
    result = json.loads(path.read_text())
    result["transcript"] = load_transcript(gid)
    return result


def prompt_for(task_id, gid, condition, text, max_actions=MAX_ACTIONS):
    task = TASKS[task_id]
    return PROMPT.format(gid=gid, max_actions=max_actions, request=task["request"] + scope(task),
                         context=context_block(condition, text))


def play_app_game(task_id, player, text=None, max_actions=MAX_ACTIONS, condition="plain", gid=None):
    """Play one game; returns the result (also saved as result.json, written whole)."""
    if (condition == "plain") != (text is None):
        raise ValueError(f"condition {condition!r} needs {'no' if condition == 'plain' else 'its'} text")
    task = TASKS[task_id]
    gid = gid or new_game_id(task_id, player, condition)
    if not re.fullmatch(rf"{re.escape(task_id)}-{re.escape(player)}-{condition}-[0-9a-f]{{6}}", gid):
        raise ValueError(f"game id {gid!r} does not match {task_id} / {player} / {condition}")
    run_dir, game_dir = RUNS / gid, R.GAMES_DIR / gid
    run_dir.mkdir(parents=True)
    game_dir.mkdir(parents=True)
    shutil.copy(TOOLS / "play_apps.py", game_dir / "play.py")
    summary, commands, others, cli_error, timed_out, exit_code = "", [], [], None, False, 0
    final_error, files, tampered, launched = None, None, [], False
    R.TIME_LIMIT = TIME_LIMIT  # run_player reads it at call time
    with open(run_dir / "qemu.log", "w") as qlog:
        machine = Machine.start(task["start"], workdir=run_dir, log=qlog)  # fails: nothing was played
        started = time.time()
        broker = AppBroker(machine, max_actions, run_dir / "transcript.jsonl", deadline=started + TIME_LIMIT)
        try:
            (game_dir / "game.json").write_text(json.dumps({"game": gid, "port": broker.port, "token": broker.token}))
            protected = {f: R.sha(game_dir / f) for f in ("play.py", "game.json")}
            prompt = prompt_for(task_id, gid, condition, text, max_actions)
            (run_dir / "prompt.txt").write_text(prompt, encoding="utf-8")
            launched = True
            if player == "robot":
                cmd = [sys.executable, str(TOOLS / "bench_robot.py"), task_id]
                timed_out, exit_code = R.run_player(cmd, prompt, game_dir, run_dir / "player.log")
            else:
                cmd = R.player_command(player, gid, game_dir)
                cmd[0] = shutil.which(cmd[0]) or cmd[0]
                env, home = R.fresh_home(player, gid)
                try:
                    timed_out, exit_code = R.run_player(cmd, prompt, game_dir, run_dir / "player.log", env)
                finally:
                    R.end_home(player, home)
            broker.close()  # no more actions from here on
            R.kill_leftovers(gid)
            if broker.os_crash is None:
                try:
                    machine.settle()
                    files = machine.dump()
                except Exception as e:
                    if machine.os_crashed():
                        broker.os_crash = f"at the end of the game: {e}"
                    else:
                        final_error = f"final check failed: {e!r}"
        except Exception as e:
            if not launched:
                raise
            final_error = final_error or f"our side failed after the player started: {e!r}"
        finally:
            if launched:
                tampered = R.changed_files(game_dir, protected)
            for step in (broker.close, machine.stop):
                try:
                    step()
                except Exception as e:
                    final_error = final_error or f"clean-up failed: {e!r}"
            shutil.rmtree(game_dir, ignore_errors=True)
    try:
        commands, others, summary, cli_error = R.player_events(
            "robot" if player == "robot" else player, (run_dir / "player.log").read_text(encoding="utf-8", errors="replace"))
    except Exception as e:
        final_error = final_error or f"player log unreadable: {e!r}"
    violations = audit_apps(commands, gid) + others + [f"changed {f}" for f in tampered]  # the audit comes first
    if exit_code not in (0, None) and not timed_out and not cli_error:
        cli_error = f"player CLI exited with code {exit_code}"
    if player != "robot" and not cli_error:
        cli_error = R.infrastructure_problem(run_dir / "player.log", commands, summary, broker.used)
    grade = {"passed": False, "tests": {}, "problem": None, "extra": []}
    os_crash = broker.os_crash
    if files is not None and not final_error:
        try:
            grade = G.grade(task, files, parse_start(task["start"]))
            if grade["problem"] and "stop responding" in grade["problem"]:
                os_crash = f"in the hidden tests: {grade['problem']}"
        except Exception as e:  # our side failed while grading: a harness error
            final_error = f"grading failed: {e!r}"
    passed = grade["passed"] and not os_crash and not final_error
    tests = grade["tests"]
    rule = (f"{sum(1 for ok, _ in tests.values() if ok)} of {len(tests)} hidden tests passed" if tests else (grade["problem"] or ""))
    if grade["extra"]:
        rule += f" | changed outside the allowed pages: {grade['extra'][:5]}"
    if os_crash:
        rule = f"the OS stopped responding ({os_crash})"
    if final_error:
        rule = final_error
    harness_errors = ([f"settle failures: {broker.settle_failures}"] if broker.settle_failures else []) \
        + [f"infra: {e}" for e in broker.infra_errors] + ([cli_error] if cli_error else []) \
        + ([final_error] if final_error else [])
    result = {"game": gid, "task": task_id, "kind": task["kind"], "level": task["level"], "player": player,
              "condition": condition, "context_sha256": sha256(text), "passed": passed,
              "visible_passed": passed, "hidden": grade, "rule": rule, "os_crash": os_crash,
              "actions": broker.used, "seconds": round(time.time() - started), "timed_out": timed_out,
              "settle_failures": broker.settle_failures, "harness_errors": harness_errors,
              "audit_ok": not violations, "violations": violations, "summary": summary,
              "files": {p: t for p, t in (files or {}).items() if G.allowed(task, p)}}
    tmp = run_dir / "result.json.tmp"
    tmp.write_text(json.dumps(result, indent=2))
    os.replace(tmp, run_dir / "result.json")
    result["transcript"] = load_transcript(gid)
    return result


def main():
    condition = sys.argv[3] if len(sys.argv) > 3 else "manual"
    r = play_app_game(sys.argv[1], sys.argv[2], context_text(condition), condition=condition)
    flags = ("" if r["audit_ok"] else "  AUDIT FAILED") + ("  HARNESS ERROR" if r["harness_errors"] else "")
    print(f"{'PASS' if r['passed'] else 'FAIL'}  {r['game']}  ({r['actions']} actions, {r['seconds']}s){flags}  {r['rule']}")


if __name__ == "__main__":
    main()
