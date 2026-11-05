"""Compute the experiment-3 result exactly as PREREG3.md says.

  python tools/analyze3.py

Reads ONLY out/experiment3/logs/ (latest log per player). Refuses a verdict unless:
no frozen file changed, every log records the frozen manifest hash, each log finished with no
errored samples, and there are exactly 192 unique games (2 players x 16 tasks x 2 conditions
x 3 epochs). Writes out/experiment3/results.json.
"""

import json
import math
import statistics
import sys
from collections import Counter, defaultdict
from pathlib import Path

from inspect_ai.log import read_eval_log

TOOLS = Path(__file__).resolve().parent
ROOT = TOOLS.parent
sys.path.insert(0, str(TOOLS))
import freeze  # noqa: E402
from tasks import TASK_SETS, TASKS as TASKS_ALL  # noqa: E402

EXP = 3
LOG_DIR = ROOT / "out" / "experiment3" / "logs"
RUN_FILE = ROOT / "out" / "experiment3" / "run.json"
LEDGER = ROOT / "out" / "experiment3" / "attempts-ledger.jsonl"
PLAYERS = ["haiku", "luna"]
CONDITIONS = ["plain", "manual"]
EPOCHS = [1, 2, 3]
TASKS = TASK_SETS["v3"]


def sign_test(pos, neg):
    """Exact two-sided sign test p-value."""
    n = pos + neg
    if n == 0:
        return 1.0
    return min(1.0, 2 * sum(math.comb(n, i) for i in range(min(pos, neg) + 1)) / 2 ** n)


def verdict(pos, neg):
    p = sign_test(pos, neg)
    v = "No clear difference"
    if p < 0.05 and pos > neg:
        v = "The manual helps"
    elif p < 0.05 and neg > pos:
        v = "The manual hurts"
    return {"tasks_manual_better": pos, "tasks_plain_better": neg, "p": round(p, 4), "verdict": v}


def fail(msg):
    sys.exit(f"NO VERDICT: {msg}")


def expected_args(player, mhash):
    return {"player": player, "taskset": "v3", "seed": 20261113, "max_actions": 100, "manifest": mhash}


def check_settings(player, ev, mhash, path):
    args = ev.task_args
    want = expected_args(player, mhash)
    wrong = {k: args.get(k) for k, v in want.items() if args.get(k) != v}
    conds = args.get("conditions")
    conds = conds.split(",") if isinstance(conds, str) else conds
    if conds != ["plain", "manual"]:
        wrong["conditions"] = conds
    for k in ("manual", "level", "tasks"):
        if args.get(k) is not None:
            wrong[k] = args.get(k)
    if ev.config.epochs != 3 or ev.config.max_samples != 2:
        wrong["epochs/max_samples"] = (ev.config.epochs, ev.config.max_samples)
    if wrong:
        fail(f"{player} log {path.name} was not run with the registered settings: {wrong}")


def load():
    changed = freeze.check(EXP)
    if changed:
        fail(f"frozen files changed: {changed}")
    mhash = freeze.manifest_hash(EXP)
    if not RUN_FILE.exists():
        fail("no run.json: the experiment was not started with tools/run_experiment3.py")
    run = json.loads(RUN_FILE.read_text())
    games = []
    for player in PLAYERS:
        logs = []
        for p in LOG_DIR.glob("*.eval"):
            e = read_eval_log(str(p), header_only=True).eval
            if e.task_args.get("player") != player:
                continue
            if e.task_id != run[player]:
                fail(f"{player}: log {p.name} belongs to another run (only the recorded run and its retries count)")
            logs.append((e.created, p))
        if not logs:
            fail(f"no log for {player}")
        for _, any_path in logs:  # every log of the run (the first start and each retry)
            check_settings(player, read_eval_log(str(any_path), header_only=True).eval, mhash, any_path)
        path = max(logs)[1]
        log = read_eval_log(str(path))
        if log.status != "success":
            fail(f"{player} log {path.name} has status {log.status}")
        for s in log.samples:
            if s.error:
                fail(f"{player} sample {s.id} epoch {s.epoch} errored (run tools/run_experiment3.py --retry)")
            sc = s.scores["end_state"]
            games.append({"player": player, "task": s.metadata["task"], "condition": s.metadata["condition"],
                          "epoch": s.epoch, "won": sc.value == "C", **(sc.metadata or {})})
    ids = Counter(g.get("game") for g in games)
    if any(n > 1 for n in ids.values()) or None in ids:
        fail("the same underlying game appears more than once (or a game id is missing)")
    cells = Counter((g["player"], g["task"], g["condition"], g["epoch"]) for g in games)
    expected = {(p, t, c, e) for p in PLAYERS for t in TASKS for c in CONDITIONS for e in EPOCHS}
    if set(cells) != expected or any(n != 1 for n in cells.values()):
        fail(f"games don't match the plan (missing {len(expected - set(cells))}, "
             f"extra {len(set(cells) - expected)}, duplicates {sum(1 for n in cells.values() if n > 1)})")
    return games, mhash


def task_scores(games, players):
    score = defaultdict(int)
    for g in games:
        if g["player"] in players:
            score[g["task"]] += (1 if g["won"] else 0) * (1 if g["condition"] == "manual" else -1)
    return {t: score[t] for t in TASKS}


def main():
    games, mhash = load()
    scores = task_scores(games, PLAYERS)
    pos, neg = sum(v > 0 for v in scores.values()), sum(v < 0 for v in scores.values())
    result = {"manifest_sha256": mhash, "games": len(games), "main": verdict(pos, neg), "task_scores": scores}
    result["per_player"] = {}
    for pl in PLAYERS:
        s = task_scores(games, [pl])
        result["per_player"][pl] = verdict(sum(v > 0 for v in s.values()), sum(v < 0 for v in s.values()))

    wins = defaultdict(lambda: [0, 0])
    per_task = defaultdict(lambda: [0, 0])
    repeat = defaultdict(int)
    for g in games:
        wins[(g["player"], g["condition"])][0] += g["won"]
        wins[(g["player"], g["condition"])][1] += 1
        per_task[(g["task"], g["condition"])][0] += g["won"]
        per_task[(g["task"], g["condition"])][1] += 1
        repeat[(g["player"], g["task"], g["condition"])] += g["won"]
    result["win_rates"] = {f"{p} / {c}": f"{w}/{n}" for (p, c), (w, n) in sorted(wins.items())}
    result["win_rates_per_task"] = {f"{t} / {c}": f"{w}/{n}" for (t, c), (w, n) in sorted(per_task.items())}
    result["repeat_consistency"] = {f"{p} / {c}": dict(sorted(Counter(v for (pp, t, cc), v in repeat.items()
                                                                      if pp == p and cc == c).items()))
                                    for p in PLAYERS for c in CONDITIONS}
    result["actions_in_won_games"] = {f"{p} / {c}": statistics.median(
        [g["actions"] for g in games if g["player"] == p and g["condition"] == c and g["won"]] or [0])
        for p in PLAYERS for c in CONDITIONS}
    tag = lambda g: f"{g['player']} {g['task']} {g['condition']} #{g['epoch']}"  # noqa: E731
    result["audit_failures"] = [f"{tag(g)}: {g.get('violations')}" for g in games if g.get("audit_ok") is False]
    result["timeouts"] = [tag(g) for g in games if g.get("timed_out")]
    result["os_crashes"] = [f"{tag(g)}: {g.get('os_crash')}" for g in games if g.get("os_crash")]
    # Also reported (no verdict): fix/write games that worked on the visible desk but not on every
    # hidden one, and wins per task kind.
    result["visible_only"] = [tag(g) for g in games if g.get("visible_passed") and g.get("hidden")
                              and not all(h["ok"] for h in g["hidden"])]
    kinds = defaultdict(lambda: [0, 0])
    for g in games:
        k = (TASKS_ALL[g["task"]]["v3"], g["player"], g["condition"])
        kinds[k][0] += g["won"]
        kinds[k][1] += 1
    result["win_rates_per_kind"] = {f"{k} / {p} / {c}": f"{w}/{n}" for (k, p, c), (w, n) in sorted(kinds.items())}
    # Every attempt ever made for these games (also across eval-retry runs), from the ledger.
    frozen_at = json.loads(freeze.manifest_path(EXP).read_text())["frozen_at"][:19]
    ledger = defaultdict(list)
    if LEDGER.exists():
        for line in LEDGER.read_text(encoding="utf-8").splitlines():
            a = json.loads(line)
            if a["time"] >= frozen_at and a["player"] in PLAYERS and a["task"] in TASKS:
                ledger[(a["player"], a["task"], a["condition"], a["epoch"])].append(a)
    final_ids = {g["game"] for g in games}
    missing = [g for g in final_ids if not any(a.get("game") == g for v in ledger.values() for a in v)]
    if missing:
        fail(f"{len(missing)} scored games are missing from the attempts ledger")
    for g in games:  # no finished game may ever be replaced by another
        cell = (g["player"], g["task"], g["condition"], g["epoch"])
        finished = [a for a in ledger.get(cell, []) if a.get("terminal")]
        if len(finished) != 1 or finished[0].get("game") != g["game"]:
            fail(f"{cell}: the scored game must be the cell's one finished attempt "
                 f"(ledger has {[a.get('game') for a in finished]}, scored {g['game']})")
    result["discarded_attempts"] = {f"{p} {t} {c} #{e}": [a for a in v if a.get("game") not in final_ids]
                                    for (p, t, c, e), v in sorted(ledger.items())
                                    if any(a.get("game") not in final_ids for a in v)}

    out = ROOT / "out" / "experiment3" / "results.json"
    out.write_text(json.dumps(result, indent=2))
    print(json.dumps(result, indent=2))
    m = result["main"]
    print(f"\nVERDICT: {m['verdict']}  (tasks better with manual {m['tasks_manual_better']}, "
          f"better without {m['tasks_plain_better']}, p={m['p']})")


if __name__ == "__main__":
    main()
