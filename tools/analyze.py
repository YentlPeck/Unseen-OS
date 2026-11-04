"""Compute the experiment result exactly as PREREG.md says.

  python tools/analyze.py

Reads ONLY out/experiment/logs/: the latest log per player (eval-retry logs
contain all samples). Refuses to give a verdict unless:
  * no frozen file has changed and every log records the frozen manifest hash,
  * each player's log finished, has no errored samples, and
  * there are exactly 120 unique games: 2 players x 30 tasks x 2 conditions.
Writes out/experiment/results.json and prints the verdict.
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
from tasks import TASKS  # noqa: E402

LOG_DIR = ROOT / "out" / "experiment" / "logs"
PLAYERS = ["haiku", "luna"]
CONDITIONS = ["plain", "manual"]
PILOT_TASKS = {"copy-list", "change-word"}


def binom_two_sided(k, n):
    """Exact two-sided binomial test p-value, p = 0.5."""
    if n == 0:
        return 1.0
    tail = sum(math.comb(n, i) for i in range(min(k, n - k) + 1)) / 2 ** n
    return min(1.0, 2 * tail)


def verdict(b, c):
    p = binom_two_sided(b, b + c)
    if p < 0.05 and b > c:
        v = "The manual helps"
    elif p < 0.05 and c > b:
        v = "The manual hurts"
    else:
        v = "No clear difference"
    return {"b_manual_only": b, "c_plain_only": c, "p": round(p, 4), "verdict": v}


def fail(msg):
    sys.exit(f"NO VERDICT: {msg}")


def load():
    changed = freeze.check()
    if changed:
        fail(f"frozen files changed: {changed}")
    mhash = freeze.manifest_hash()
    games = []
    for player in PLAYERS:
        logs = []
        for p in LOG_DIR.glob("*.eval"):
            log = read_eval_log(str(p), header_only=True)
            if log.eval.task_args.get("player") == player:
                logs.append((p.stat().st_mtime, p))
        if not logs:
            fail(f"no log for {player}")
        path = max(logs)[1]
        log = read_eval_log(str(path))
        if log.status != "success":
            fail(f"{player} log {path.name} has status {log.status}")
        if log.eval.task_args.get("manifest") != mhash:
            fail(f"{player} log was not run with the frozen manifest")
        for s in log.samples:
            if s.error:
                fail(f"{player} sample {s.id} errored (run tools/run_experiment.py --retry)")
            score = s.scores["end_state"]
            games.append({"player": player, "task": s.metadata["task"], "condition": s.metadata["condition"],
                          "level": s.metadata["level"], "passed": score.value == "C", **(score.metadata or {})})
    cells = Counter((g["player"], g["task"], g["condition"]) for g in games)
    expected = {(p, t, c) for p in PLAYERS for t in TASKS for c in CONDITIONS}
    if set(cells) != expected or any(n != 1 for n in cells.values()):
        missing, extra = expected - set(cells), set(cells) - expected
        dup = [k for k, n in cells.items() if n > 1]
        fail(f"games don't match the plan (missing {len(missing)}, extra {len(extra)}, duplicate {len(dup)})")
    return games, mhash


def main():
    games, mhash = load()
    by = {(g["player"], g["task"], g["condition"]): g for g in games}
    prs = [(p, t, by[(p, t, "plain")]["passed"], by[(p, t, "manual")]["passed"]) for p in PLAYERS for t in TASKS]

    def bc(sel):
        return (sum(1 for *_, pl, ma in sel if ma and not pl), sum(1 for *_, pl, ma in sel if pl and not ma))

    result = {"manifest_sha256": mhash, "games": len(games), "pairs": len(prs), "main": verdict(*bc(prs))}
    result["main_without_pilot_tasks"] = verdict(*bc([x for x in prs if x[1] not in PILOT_TASKS]))
    result["per_player"] = {pl: verdict(*bc([x for x in prs if x[0] == pl])) for pl in PLAYERS}

    # Sensitivity: one score per TASK (both players together), sign test over tasks.
    task_score = {t: sum((ma - pl) for p, tt, pl, ma in prs if tt == t) for t in TASKS}
    up, down = sum(1 for v in task_score.values() if v > 0), sum(1 for v in task_score.values() if v < 0)
    result["task_level_sign_test"] = {"tasks_manual_better": up, "tasks_plain_better": down,
                                      "p": round(binom_two_sided(up, up + down), 4)}

    rates = defaultdict(lambda: [0, 0])
    for g in games:
        for key in [(g["player"], g["condition"], "all"), (g["player"], g["condition"], g["level"])]:
            rates[key][0] += g["passed"]
            rates[key][1] += 1
    result["pass_rates"] = {" / ".join(k): f"{w}/{n}" for k, (w, n) in sorted(rates.items())}
    both = [(by[(p, t, "plain")]["actions"], by[(p, t, "manual")]["actions"]) for p, t, pl, ma in prs if pl and ma]
    result["actions_when_won_both"] = {"games": len(both),
                                       "median_plain": statistics.median(a for a, _ in both) if both else None,
                                       "median_manual": statistics.median(m for _, m in both) if both else None}
    tag = lambda g: f"{g['player']} {g['task']} {g['condition']}"  # noqa: E731
    result["audit_failures"] = [f"{tag(g)}: {g.get('violations')}" for g in games if g.get("audit_ok") is False]
    result["timeouts"] = [tag(g) for g in games if g.get("timed_out")]
    result["harness_reruns"] = [f"{tag(g)}: {g.get('attempts')}" for g in games if len(g.get("attempts") or []) > 1]
    result["pilot_games"] = "see PREREG.md, section 'Pilot games already played'"

    out = ROOT / "out" / "experiment" / "results.json"
    out.write_text(json.dumps(result, indent=2))
    print(json.dumps(result, indent=2))
    m = result["main"]
    print(f"\nVERDICT: {m['verdict']}  (b={m['b_manual_only']}, c={m['c_plain_only']}, p={m['p']})")


if __name__ == "__main__":
    main()
