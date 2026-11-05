"""The registered order (audit failure, OS crash, harness error, time-out) must survive any
failure of our own side after the player started, and an interruption must never let a
finished game be replaced.

  python tools/test_harness_errors.py

Part 1 plays robot games in which our side is forced to fail at different points (the final
settle, the final dump, the clean-up, the hidden-world check), with and without a planted audit
violation. The result must keep its game id and audit, and inspect_eval's terminal() must make
the right call: an audit failure stands (a loss, no re-run), otherwise a harness error (re-run).

Part 2 uses a temporary experiment ledger: a game booked and finished but never entered in the
ledger (an interruption in between) is reused, not replayed; a booked game that never finished
is replayed.
"""

import json
import os
import sys
import tempfile
from pathlib import Path

os.environ.pop("UNSEEN_LEDGER", None)  # part 1 never touches an experiment's ledger
sys.path.insert(0, str(Path(__file__).resolve().parent))
import hidden  # noqa: E402
import inspect_eval  # noqa: E402
import run_task  # noqa: E402
from inspect_eval import terminal  # noqa: E402
from unseen import Machine, SettleError  # noqa: E402

TASK = "p3-fix-family"  # a fix task: has hidden worlds


class Boom(Exception):
    pass


def play(where, violation):
    saved = {"settle": Machine.settle, "dump": Machine.dump, "audit": run_task.audit,
             "kill": run_task.kill_leftovers, "hidden": run_task.run_hidden, "close": run_task.Broker.close}

    def close(self):
        saved["close"](self)
        self.machine._final = True  # from here on: the final check

    def settle(self, limit=15.0):
        if where == "settle" and getattr(self, "_final", False):
            raise SettleError("OS handled 1 of 2 keys (forced by the test)")
        return saved["settle"](self, limit)

    def dump(self):
        if where == "dump":
            raise ValueError("bad dump (forced by the test)")
        return saved["dump"](self)

    def kill(gid):
        if where == "cleanup":
            raise Boom("clean-up failed (forced by the test)")
        return saved["kill"](gid)

    def run_hidden(task, page):
        if where == "hidden":
            raise Boom("hidden check failed (forced by the test)")
        return saved["hidden"](task, page)

    run_task.Broker.close, Machine.settle, Machine.dump = close, settle, dump
    run_task.kill_leftovers, run_task.run_hidden = kill, run_hidden
    if violation:
        run_task.audit = lambda commands, gid: ["planted violation (test)"]
    try:
        return run_task.play_game(TASK, "robot", None, 100, condition="plain")
    finally:
        Machine.settle, Machine.dump, run_task.audit = saved["settle"], saved["dump"], saved["audit"]
        run_task.kill_leftovers, run_task.run_hidden, run_task.Broker.close = saved["kill"], saved["hidden"], saved["close"]


def part1():
    bad = 0
    for where in ("settle", "dump", "cleanup", "hidden", None):
        for violation in (False, True):
            r = play(where, violation)
            fails = where is not None
            want_terminal = violation or not fails
            ok = (r["game"] and terminal(r) == want_terminal and r["audit_ok"] == (not violation)
                  and bool(r["harness_errors"]) == fails and (not fails or not r["passed"])
                  and (fails or r["passed"]))
            bad += not ok
            print("ok  " if ok else "BAD ", f"fails at={where} violation={violation}:",
                  f"passed={r['passed']} audit_ok={r['audit_ok']} terminal={terminal(r)}",
                  f"harness_errors={r['harness_errors']}")
    return bad


def part2():
    bad = 0
    tmp = Path(tempfile.mkdtemp(prefix="ledger-"))
    os.environ["UNSEEN_LEDGER"] = str(tmp / "ledger.jsonl")
    inspect_eval.LEDGER = tmp / "ledger.jsonl"
    try:
        # (a) a finished game whose ledger entry was lost: booked + result.json, no entry
        r = run_task.play_game(TASK, "robot", None, 100, condition="plain",
                               gid=run_task.new_game_id(TASK, "robot", "plain"))
        cell = {"player": "robot", "task": TASK, "condition": "plain", "epoch": 1}
        inspect_eval.record_attempt({**cell, "game": r["game"], "started": True, "terminal": False})
        calls = []
        real = inspect_eval.play_game
        inspect_eval.play_game = lambda *a, **k: calls.append(1) or real(*a, **k)
        try:
            got = inspect_eval.run_with_retry(TASK, "robot", None, 100, "plain", 1)
        finally:
            inspect_eval.play_game = real
        ok = got["game"] == r["game"] and not calls and got.get("recovered_from_ledger") and got["transcript"]
        bad += not ok
        print("ok  " if ok else "BAD ", f"finished but unrecorded game is reused: {got['game']} (new games played: {len(calls)})")
        # ... and a second retry finds it as the cell's finished game
        again = inspect_eval.run_with_retry(TASK, "robot", None, 100, "plain", 1)
        ok = again["game"] == r["game"]
        bad += not ok
        print("ok  " if ok else "BAD ", "after that, the ledger has it as the finished game")
        # (b) a booked game that never finished (no result.json): replayed
        cell2 = {**cell, "epoch": 2}
        ghost = run_task.new_game_id(TASK, "robot", "plain")
        inspect_eval.record_attempt({**cell2, "game": ghost, "started": True, "terminal": False})
        got = inspect_eval.run_with_retry(TASK, "robot", None, 100, "plain", 2)
        ok = got["game"] != ghost and got["passed"]
        bad += not ok
        print("ok  " if ok else "BAD ", f"booked game that never finished is replayed: {ghost} -> {got['game']}")
        rows = [json.loads(l) for l in (tmp / "ledger.jsonl").read_text().splitlines()]
        print("     ledger rows:", [(a["epoch"], a["game"][-6:], a.get("started", False), a["terminal"]) for a in rows])
    finally:
        os.environ.pop("UNSEEN_LEDGER", None)
    return bad


def part3():
    """Sol's re-check: a saved result is never replaced; the tamper check and the log parser
    can't lose a violation."""
    bad = 0
    # (a) play_game saves its result, then something fails: the saved result stands
    tmp = Path(tempfile.mkdtemp(prefix="ledger-"))
    os.environ["UNSEEN_LEDGER"] = str(tmp / "ledger.jsonl")
    inspect_eval.LEDGER = tmp / "ledger.jsonl"
    real, real_audit = inspect_eval.play_game, run_task.audit
    calls = []

    def save_then_fail(*a, **k):
        calls.append(1)
        real(*a, **k)
        raise Boom("failed after saving the result (forced by the test)")

    inspect_eval.play_game = save_then_fail
    run_task.audit = lambda commands, gid: ["planted violation (test)"]
    try:
        got = inspect_eval.run_with_retry(TASK, "robot", None, 100, "plain", 1)
        ok = len(calls) == 1 and got["audit_ok"] is False and terminal(got)
        bad += not ok
        print("ok  " if ok else "BAD ", f"result saved, then a crash: the saved audit failure stands "
              f"(games played: {len(calls)}, audit_ok={got['audit_ok']})")
    finally:
        inspect_eval.play_game, run_task.audit = real, real_audit
        os.environ.pop("UNSEEN_LEDGER", None)
    # (b) the player replaced play.py with a folder: counted as a change, nothing escapes
    folder = Path(tempfile.mkdtemp(prefix="tamper-"))
    (folder / "game.json").write_text("x")
    (folder / "play.py").mkdir()
    changed = run_task.changed_files(folder, {"play.py": "abc", "game.json": run_task.sha(folder / "game.json")})
    ok = changed == ["play.py"]
    bad += not ok
    print("ok  " if ok else "BAD ", f"play.py replaced by a folder counts as changed: {changed}")
    # (c) a forbidden command followed by an event of an unexpected shape: the command is kept
    log = "\n".join([json.dumps({"type": "assistant", "message": {"content": [
        {"type": "tool_use", "id": "1", "name": "Bash", "input": {"command": "cat game.json"}}]}}), "null"])
    commands, others, final, cli_error = run_task.player_events("haiku", log)
    ok = commands == ["cat game.json"] and cli_error and run_task.audit(commands, "g1")
    bad += not ok
    print("ok  " if ok else "BAD ", f"odd log event keeps earlier commands: {commands}, error: {cli_error}")
    # (d) a null content item, an incomplete later event, and a non-text command keep what was seen
    h = json.dumps({"type": "assistant", "message": {"content": [None, {"type": "tool_use", "id": "1", "name": "Bash",
                                                                        "input": {"command": "cat game.json"}}]}})
    l = "\n".join([json.dumps({"type": "item.started", "item": {"id": "a", "type": "command_execution",
                                                                 "command": "Get-Content game.json"}}),
                   json.dumps({"type": "item.completed", "item": {"id": "a", "type": "command_execution"}}),
                   json.dumps({"type": "item.completed", "item": {"id": "b", "type": "command_execution", "command": []}})])
    got = (run_task.player_events("haiku", h)[0], run_task.player_events("luna", l)[0])
    ok = got == (["cat game.json"], ["Get-Content game.json"]) and run_task.audit([[]], "g1")
    bad += not ok
    print("ok  " if ok else "BAD ", f"malformed events keep commands: {got}; a non-text command fails the audit")
    # (e) saving the result fails, or the result is lost and play_game crashes: a known violation stands
    for case in ("replace fails", "result lost"):
        os.environ["UNSEEN_LEDGER"] = str(tmp / f"ledger-{case[:3]}.jsonl")
        inspect_eval.LEDGER = tmp / f"ledger-{case[:3]}.jsonl"
        real_replace, real_ie_audit = run_task.os.replace, inspect_eval.audit
        games = []

        def failing_replace(a, b):
            raise PermissionError("forced by the test")

        def lose_result(*a, **k):
            r = real(*a, **k)
            games.append(r["game"])
            (run_task.ROOT / "out" / "runs" / r["game"] / "result.json").unlink()
            raise Boom("crashed after the game (forced by the test)")

        def count(*a, **k):
            games.append(1)
            return real(*a, **k)

        run_task.audit = lambda commands, gid: ["planted violation (test)"]
        inspect_eval.audit = lambda commands, gid: ["planted violation (test)"]
        if case == "replace fails":
            run_task.os.replace = failing_replace
            inspect_eval.play_game = count
        else:
            inspect_eval.play_game = lose_result
        try:
            got = inspect_eval.run_with_retry(TASK, "robot", None, 100, "plain", 1)
            ok = len(games) == 1 and got["audit_ok"] is False and terminal(got)
        except Exception as e:
            ok, got = False, {"error": repr(e)}
        finally:
            run_task.os.replace, inspect_eval.audit, run_task.audit = real_replace, real_ie_audit, real_audit
            inspect_eval.play_game = real
            os.environ.pop("UNSEEN_LEDGER", None)
        bad += not ok
        print("ok  " if ok else "BAD ", f"{case}: the violation stands, no replacement (games played: {len(games)}, "
              f"audit_ok={got.get('audit_ok')}, rule={str(got.get('rule'))[:60]})")
    # (g) restart after an outage: a booked game with only a temporary result, or only a log that
    #     shows a violation, is rescued; no new game may be played
    for case in ("temporary result only", "violation in the log only"):
        os.environ["UNSEEN_LEDGER"] = str(tmp / f"ledger-g{case[:4]}.jsonl")
        inspect_eval.LEDGER = tmp / f"ledger-g{case[:4]}.jsonl"
        run_task.audit = lambda commands, gid: ["planted violation (test)"]
        real_ie_audit = inspect_eval.audit
        inspect_eval.audit = lambda commands, gid: ["planted violation (test)"]
        try:
            gid = run_task.new_game_id(TASK, "robot", "plain")
            cell = {"player": "robot", "task": TASK, "condition": "plain", "epoch": 1}
            inspect_eval.record_attempt({**cell, "game": gid, "started": True, "terminal": False})
            real(TASK, "robot", None, 100, condition="plain", gid=gid)
            run_dir = run_task.ROOT / "out" / "runs" / gid
            if case.startswith("temporary"):
                (run_dir / "result.json").rename(run_dir / "result.json.tmp")
            else:
                (run_dir / "result.json").unlink()
            inspect_eval.play_game = lambda *a, **k: (_ for _ in ()).throw(AssertionError("must not play"))
            got = inspect_eval.run_with_retry(TASK, "robot", None, 100, "plain", 1)
            ok = got["game"] == gid and got["audit_ok"] is False and terminal(got)
        except Exception as e:
            ok, got = False, {"error": repr(e)}
        finally:
            inspect_eval.play_game, inspect_eval.audit, run_task.audit = real, real_ie_audit, real_audit
            os.environ.pop("UNSEEN_LEDGER", None)
        bad += not ok
        print("ok  " if ok else "BAD ", f"restart, {case}: rescued without a new game ({got.get('game') or got})")
    # (f) deleting the game folder raises: the game still ends with its audit
    real_rmtree = run_task.shutil.rmtree
    run_task.shutil.rmtree = lambda *a, **k: (_ for _ in ()).throw(RecursionError("forced by the test"))
    run_task.audit = lambda commands, gid: ["planted violation (test)"]
    try:
        r = run_task.play_game(TASK, "robot", None, 100, condition="plain")
        ok = r["audit_ok"] is False and r["harness_errors"] and terminal(r)
    finally:
        run_task.shutil.rmtree, run_task.audit = real_rmtree, real_audit
    bad += not ok
    print("ok  " if ok else "BAD ", f"clean-up raises: audit kept (audit_ok={r['audit_ok']}, errors={r['harness_errors']})")
    return bad


def main():
    bad = part1() + part2() + part3()
    print("ALL GOOD" if not bad else f"{bad} PROBLEMS")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
