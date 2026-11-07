"""Tests of experiment 5's attempt rules (tools/inspect_apps.py), without QEMU or any AI:
games are faked. Run: python tools/test_apps_harness.py

  * a harness error gets exactly one re-run, also across a restart (attempts come from the ledger);
  * a finished game in the ledger is reused, never replayed;
  * an interrupted game whose records show a time-out or an OS crash is a loss, not replayed;
  * a ledger line cut off by an interruption is left out; two processes can write at once.
"""

import json
import multiprocessing
import os
import sys
import tempfile
from pathlib import Path

TOOLS = Path(__file__).resolve().parent
sys.path.insert(0, str(TOOLS))
FAILS = []


def check(name, ok, detail=""):
    print(("ok   " if ok else "FAIL ") + name + (f"  ({detail})" if detail and not ok else ""))
    if not ok:
        FAILS.append(name)


def fresh(tmp):
    os.environ["UNSEEN_LEDGER"] = str(Path(tmp) / "ledger.jsonl")
    os.environ["UNSEEN_APPS_RUNS"] = str(Path(tmp) / "runs")
    for m in ("inspect_apps", "apps_game"):
        sys.modules.pop(m, None)
    import inspect_apps as I
    return I


def fake_result(gid, condition, **kw):
    r = {"game": gid, "condition": condition, "context_sha256": None, "passed": False, "rule": "", "audit_ok": True,
         "violations": [], "timed_out": False, "harness_errors": [], "actions": 3, "seconds": 1, "settle_failures": 0,
         "hidden": None, "os_crash": None, "summary": "", "transcript": []}
    r.update(kw)
    return r


def writer(path, n, tag):
    os.environ["UNSEEN_LEDGER"] = path
    sys.modules.pop("inspect_apps", None)
    sys.path.insert(0, str(TOOLS))
    import inspect_apps as I
    for i in range(n):
        I.record_attempt({"tag": tag, "i": i, "pad": "x" * 500})


def main():
    with tempfile.TemporaryDirectory() as tmp:
        I = fresh(tmp)
        played = []

        def harness_error(task_id, player, text, max_actions, condition, gid):
            played.append(gid)
            return fake_result(gid, condition, harness_errors=["settle failures: 1"])
        I.A.play_app_game = harness_error
        try:
            I.run_with_retry("build-light", "luna", None, 200, "plain", 1)
            check("two harness errors end the sample", False)
        except I.HarnessError:
            check("two harness errors end the sample", len(played) == 2, played)
        played.clear()
        try:
            I.run_with_retry("build-light", "luna", None, 200, "plain", 1)
        except I.HarnessError:
            pass
        check("a restart does not give more attempts", played == [], played)
        os.environ["UNSEEN_RETRY_ROUND"] = "1"
        I = fresh(tmp)
        I.A.play_app_game = lambda task_id, player, text, max_actions, condition, gid: (played.append(gid), fake_result(gid, condition, passed=True))[1]
        r = I.run_with_retry("build-light", "luna", None, 200, "plain", 1)
        check("an explicit retry round plays an exhausted cell again", r["passed"] and len(played) == 1, played)
        played.clear()
        os.environ["UNSEEN_RETRY_ROUND"] = "2"
        I = fresh(tmp)
        I.A.play_app_game = lambda *a, **k: (_ for _ in ()).throw(AssertionError("replayed"))
        r = I.run_with_retry("build-light", "luna", None, 200, "plain", 1)
        check("a finished game is never replayed in a later round", r.get("recovered_from_ledger") and r["passed"])
        del os.environ["UNSEEN_RETRY_ROUND"]

        # one harness error, then a restart: exactly one more attempt
        I = fresh(tmp + "/b")
        played = []
        results = iter([fake_result("x", "plain", harness_errors=["infra: x"]), fake_result("y", "plain", passed=True)])

        def first_error(task_id, player, text, max_actions, condition, gid):
            played.append(gid)
            r = next(results)
            r["game"] = gid
            if r["harness_errors"]:
                raise_after = True
            return r
        I.A.play_app_game = first_error
        state = {"n": 0}

        def stop_after_one(task_id, player, text, max_actions, condition, gid):
            state["n"] += 1
            if state["n"] == 2:
                raise KeyboardInterrupt  # the run is interrupted before the second attempt finishes
            return first_error(task_id, player, text, max_actions, condition, gid)
        I.A.play_app_game = stop_after_one
        try:
            I.run_with_retry("build-light", "haiku", None, 200, "plain", 1)
        except KeyboardInterrupt:
            pass
        I = fresh(tmp + "/b")
        I.A.play_app_game = first_error
        try:  # the harness error and the interrupted attempt both count in round 0
            I.run_with_retry("build-light", "haiku", None, 200, "plain", 1)
            check("after a restart the earlier harness error and the interruption both count", False)
        except I.HarnessError:
            check("after a restart the earlier harness error and the interruption both count", True)
        os.environ["UNSEEN_RETRY_ROUND"] = "1"
        I = fresh(tmp + "/b")
        I.A.play_app_game = first_error
        r = I.run_with_retry("build-light", "haiku", None, 200, "plain", 1)
        check("the next retry round plays it again", r["passed"])
        del os.environ["UNSEEN_RETRY_ROUND"]

        # a finished game is reused
        I = fresh(tmp + "/b")
        I.A.play_app_game = lambda *a, **k: (_ for _ in ()).throw(AssertionError("replayed"))
        r = I.run_with_retry("build-light", "haiku", None, 200, "plain", 1)
        check("a recorded finished game is reused, never replayed", r.get("recovered_from_ledger") and r["passed"])

        # an interrupted game with a time-out in the player's log: a loss, not replayed
        I = fresh(tmp + "/c")
        gid = I.A.new_game_id("build-light", "haiku", "plain")
        run = I.A.RUNS / gid
        run.mkdir(parents=True)
        (run / "player.log").write_text("\n[harness] time limit reached; player process tree killed\n")
        (run / "transcript.jsonl").write_text(json.dumps({"n": 1, "action": "look", "args": []}) + "\n")
        I.record_attempt({"player": "haiku", "task": "build-light", "condition": "plain", "epoch": 1, "game": gid,
                          "started": True, "terminal": False})
        I.A.play_app_game = lambda *a, **k: (_ for _ in ()).throw(AssertionError("replayed"))
        r = I.run_with_retry("build-light", "haiku", None, 200, "plain", 1)
        check("an interrupted game that timed out after an action counts as a loss", r["timed_out"] and not r["passed"], r.get("rule"))

        # the same with no action at all: not played, so a harness error (re-run)
        I = fresh(tmp + "/c0")
        gid = I.A.new_game_id("build-light", "haiku", "plain")
        run = I.A.RUNS / gid
        run.mkdir(parents=True)
        (run / "player.log").write_text("\n[harness] time limit reached; player process tree killed\n")
        I.record_attempt({"player": "haiku", "task": "build-light", "condition": "plain", "epoch": 1, "game": gid,
                          "started": True, "terminal": False})
        played = []
        I.A.play_app_game = lambda task_id, player, text, max_actions, condition, gid: (played.append(gid), fake_result(gid, condition, passed=True))[1]
        r = I.run_with_retry("build-light", "haiku", None, 200, "plain", 1)
        check("an interrupted time-out with no action is re-run, not a loss", r["passed"] and len(played) == 1)

        # a salvaged Luna loss keeps the setup written before the player started
        I = fresh(tmp + "/c2")
        gid = I.A.new_game_id("build-light", "luna", "plain")
        run = I.A.RUNS / gid
        run.mkdir(parents=True)
        (run / "player.log").write_text("\n[harness] time limit reached; player process tree killed\n")
        (run / "luna_setup.json").write_text(json.dumps({"effort": "high", "sandbox_id_reused": True, "features_off": ["plugins"]}))
        (run / "transcript.jsonl").write_text(json.dumps({"n": 1, "action": "look", "args": []}) + "\n")
        I.record_attempt({"player": "luna", "task": "build-light", "condition": "plain", "epoch": 1, "game": gid,
                          "started": True, "terminal": False})
        I.A.play_app_game = lambda *a, **k: (_ for _ in ()).throw(AssertionError("replayed"))
        r = I.run_with_retry("build-light", "luna", None, 200, "plain", 1)
        row = [x for x in I.read_ledger() if x.get("game") == gid and not x.get("started")][-1]
        check("a salvaged Luna loss keeps Luna's setup in the ledger", (row.get("luna_setup") or {}).get("effort") == "high")

        # an interrupted booking with nothing to recover is recorded (discarded) in its own round
        os.environ["UNSEEN_RETRY_ROUND"] = "0"
        I = fresh(tmp + "/c3")
        gid = I.A.new_game_id("build-light", "haiku", "plain")
        I.record_attempt({"player": "haiku", "task": "build-light", "condition": "plain", "epoch": 1, "game": gid,
                          "started": True, "terminal": False, "round": 0})
        os.environ["UNSEEN_RETRY_ROUND"] = "1"
        I = fresh(tmp + "/c3")
        played = []
        I.A.play_app_game = lambda task_id, player, text, max_actions, condition, gid: (played.append(gid), fake_result(gid, condition, passed=True))[1]
        r = I.run_with_retry("build-light", "haiku", None, 200, "plain", 1)
        rows = I.read_ledger()
        disc = [x for x in rows if x.get("game") == gid and x.get("discarded")]
        check("an interrupted booking is recorded as discarded under its own round",
              len(disc) == 1 and disc[0]["round"] == 0 and r["passed"] and len(played) == 1)
        del os.environ["UNSEEN_RETRY_ROUND"]

        # the same with an OS crash in the referee's transcript
        I = fresh(tmp + "/d")
        gid = I.A.new_game_id("build-light", "luna", "plain")
        run = I.A.RUNS / gid
        run.mkdir(parents=True)
        (run / "player.log").write_text("")
        (run / "transcript.jsonl").write_text(json.dumps({"n": 7, "os_crash": "TimeoutError()"}) + "\n")
        I.record_attempt({"player": "luna", "task": "build-light", "condition": "plain", "epoch": 1, "game": gid,
                          "started": True, "terminal": False})
        I.A.play_app_game = lambda *a, **k: (_ for _ in ()).throw(AssertionError("replayed"))
        r = I.run_with_retry("build-light", "luna", None, 200, "plain", 1)
        check("an interrupted game with an OS crash counts as a loss", bool(r["os_crash"]) and not r["passed"], r.get("rule"))

        # a cut-off last line is left out; a broken line in the middle is an error
        I = fresh(tmp + "/e")
        I.record_attempt({"a": 1})
        with open(I.LEDGER, "a", encoding="utf-8") as f:
            f.write('{"a": 2, "cut')
        check("a cut-off last line is left out", [r["a"] for r in I.read_ledger()] == [1])
        I.LEDGER.write_text('{"a": 1}\n{"a": 2, "cut\n{"a": 3}\n', encoding="utf-8")  # broken in the middle
        try:
            I.read_ledger()
            check("a broken line in the middle is an error", False)
        except RuntimeError:
            check("a broken line in the middle is an error", True)

        # a row written after a cut-off line starts on a line of its own; the fragment is kept aside
        I = fresh(tmp + "/f")
        I.record_attempt({"a": 1})
        with open(I.LEDGER, "a", encoding="utf-8") as f:
            f.write('{"a": 2, "cut')
        I.record_attempt({"a": 3})
        I.record_attempt({"a": 4})
        cut = I.LEDGER.with_name(I.LEDGER.name + ".cut")
        check("after a cut-off line, new rows are read normally", [r["a"] for r in I.read_ledger()] == [1, 3, 4])
        check("the cut-off fragment is kept aside", cut.exists() and '"cut' in cut.read_text())

        # a complete finished row whose new line was lost is kept, and reused (not replayed)
        I = fresh(tmp + "/f2")
        row = {"player": "haiku", "task": "build-light", "condition": "plain", "epoch": 1, "game": "build-light-haiku-plain-aaaaaa",
               **{k: v for k, v in fake_result("build-light-haiku-plain-aaaaaa", "plain", passed=True).items() if k != "transcript"},
               "terminal": True}
        I.LEDGER.parent.mkdir(parents=True, exist_ok=True)
        I.LEDGER.write_text(json.dumps(row))  # no new line at its end
        I.A.play_app_game = lambda *a, **k: (_ for _ in ()).throw(AssertionError("replayed"))
        r = I.run_with_retry("build-light", "haiku", None, 200, "plain", 1)
        rows = I.read_ledger()
        check("a complete row without its new line is kept and its game reused",
              r["passed"] and any(x.get("terminal") and x.get("game") == row["game"] for x in rows))

        # a harness error and a time-out in the same interrupted game: a re-run, not a loss
        I = fresh(tmp + "/g")
        gid = I.A.new_game_id("build-light", "haiku", "plain")
        run = I.A.RUNS / gid
        run.mkdir(parents=True)
        (run / "player.log").write_text("\n[harness] time limit reached; player process tree killed\n")
        (run / "transcript.jsonl").write_text(json.dumps({"n": 3, "action": "look", "args": []}) + "\n"
                                              + json.dumps({"n": 4, "settle_error": "no answer"}) + "\n")
        I.record_attempt({"player": "haiku", "task": "build-light", "condition": "plain", "epoch": 1, "game": gid,
                          "started": True, "terminal": False})
        played = []

        def passes(task_id, player, text, max_actions, condition, gid):
            played.append(gid)
            return fake_result(gid, condition, passed=True)
        I.A.play_app_game = passes
        r = I.run_with_retry("build-light", "haiku", None, 200, "plain", 1)
        check("a time-out after a harness error is re-run, not a loss", r["passed"] and len(played) == 1
              and len(r["attempts"]) == 2, r.get("attempts"))

        # two processes writing at once
        path = str(Path(tmp) / "shared.jsonl")
        ps = [multiprocessing.Process(target=writer, args=(path, 300, t)) for t in ("haiku", "luna")]
        for p in ps:
            p.start()
        for p in ps:
            p.join()
        rows = [json.loads(l) for l in Path(path).read_text(encoding="utf-8").splitlines()]
        check("two processes writing at once lose and mix no rows", len(rows) == 600
              and {(r["tag"], r["i"]) for r in rows} == {(t, i) for t in ("haiku", "luna") for i in range(300)})
    print("ALL GOOD" if not FAILS else f"{len(FAILS)} FAILED: {FAILS}")
    return 1 if FAILS else 0


if __name__ == "__main__":
    sys.exit(main())
