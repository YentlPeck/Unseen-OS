"""Compute the experiment-4 result exactly as PREREG4.md says.

  python tools/analyze4.py

Reads ONLY out/experiment4/logs/ (the recorded run and its retries). Refuses a verdict unless:
no frozen file changed; every log of the run shows the registered settings; there are no
errored samples; there are exactly 576 unique games (2 players x 16 tasks x 3 conditions x 6
epochs); every game was played in its own condition with the registered prompt text; and for
every game the ledger holds exactly one finished attempt for that cell, which is the scored game.
Writes out/experiment4/results.json.

Primary (the only verdict): manual vs reference. Per task, d = manual win rate - reference win
rate (both players, 6 epochs: 12 games each); D = mean of d over the 16 tasks; exact two-sided
sign-flip permutation test over the tasks (all 2^16 sign patterns); 95% interval by inverting
the same test. Secondary (no verdict): reference vs plain, and manual vs plain.
"""

import json
import math
import random
import re
import statistics
import sys
from collections import Counter, defaultdict
from itertools import product
from pathlib import Path

from inspect_ai.log import read_eval_log

TOOLS = Path(__file__).resolve().parent
ROOT = TOOLS.parent
sys.path.insert(0, str(TOOLS))
import freeze  # noqa: E402
from reference import reference_text, sha256  # noqa: E402
from tasks import TASK_SETS, TASKS as TASKS_ALL  # noqa: E402

EXP = 4
EXP_DIR = ROOT / "out" / "experiment4"
LOG_DIR = EXP_DIR / "logs"
RUN_FILE = EXP_DIR / "run.json"
LEDGER = EXP_DIR / "attempts-ledger.jsonl"
PLAYERS = ["haiku", "luna"]
CONDITIONS = ["plain", "reference", "manual"]
EPOCHS = [1, 2, 3, 4, 5, 6]
SEED = 20261114
TASKS = sorted(TASK_SETS["v3"])
ALPHA = 0.05
MARGIN = 0.10  # "about as good": the interval's upper bound for manual - reference is below 10 points
GUIDE_MARK = "CHORES: the complete reference"


def fail(msg):
    sys.exit(f"NO VERDICT: {msg}")


# ---- statistics ------------------------------------------------------------------------------

SIGNS = list(product((1, -1), repeat=16))


def perm_p(d):
    """Exact two-sided sign-flip permutation p-value for mean(d) = 0 (d: one value per task)."""
    obs = abs(sum(d)) - 1e-12
    hits = sum(1 for s in SIGNS if abs(sum(si * di for si, di in zip(s, d))) >= obs)
    return hits / len(SIGNS)


def perm_interval(d, step=0.0025, tol=1e-5):
    """95% interval for D by inverting the permutation test: the shifts delta for which the
    test on (d - delta) gives p >= 0.05. (This assumes a location-shift model: each task's
    difference is a common shift plus noise symmetric around 0; see PREREG4.md.)
    Scan a grid of steps of 0.25 points plus the observed mean (always accepted: p = 1), then
    refine each end by bisection to 0.001 points, and round outwards to 0.1 points."""
    import numpy as np
    S = np.array(SIGNS, dtype=np.float64)
    dv = np.array(d, dtype=np.float64)

    def accepted(delta):
        x = dv - delta
        return (np.abs(S @ x) >= abs(x.sum()) - 1e-9).mean() >= ALPHA

    mean = float(dv.mean())
    grid = sorted({round(-1 + i * step, 6) for i in range(int(round(2 / step)) + 1)} | {mean})
    keep = [x for x in grid if accepted(x)]
    lo, hi = min(keep), max(keep)

    def refine(inside, outside):
        while abs(outside - inside) > tol:
            mid = (inside + outside) / 2
            inside, outside = (mid, outside) if accepted(mid) else (inside, mid)
        return inside

    if lo > -1:
        lo = refine(lo, max(-1.0, lo - step))
    if hi < 1:
        hi = refine(hi, min(1.0, hi + step))
    return math.floor(lo * 1000) / 1000, math.ceil(hi * 1000) / 1000


def bootstrap_interval(d, reps=10000, seed=SEED):
    rng = random.Random(seed)
    means = sorted(statistics.mean(rng.choice(d) for _ in d) for _ in range(reps))
    return round(means[int(0.025 * reps)], 4), round(means[int(0.975 * reps) - 1], 4)


def sign_test(pos, neg):
    import math
    n = pos + neg
    return 1.0 if n == 0 else min(1.0, 2 * sum(math.comb(n, i) for i in range(min(pos, neg) + 1)) / 2 ** n)


def contrast(games, a, b, players=PLAYERS, intervals=True):
    """a vs b: per-task win-rate differences, D, permutation p, intervals, and the experiment-3
    style sign test (descriptive)."""
    wins, n = defaultdict(int), defaultdict(int)
    for g in games:
        if g["player"] in players and g["condition"] in (a, b):
            wins[(g["task"], g["condition"])] += g["won"]
            n[(g["task"], g["condition"])] += 1
    d = [wins[(t, a)] / n[(t, a)] - wins[(t, b)] / n[(t, b)] for t in TASKS]
    out = {"per_task": {t: round(x, 4) for t, x in zip(TASKS, d)},
           "D": round(statistics.mean(d), 4), "p": round(perm_p(d), 5),
           "tasks_better": sum(x > 0 for x in d), "tasks_worse": sum(x < 0 for x in d),
           "sign_test_p": round(sign_test(sum(x > 0 for x in d), sum(x < 0 for x in d)), 5)}
    if intervals:
        out["ci95_permutation"] = perm_interval(d)
        out["ci95_bootstrap"] = bootstrap_interval(d)
    return out


def primary_verdict(c):
    if c["p"] < ALPHA and c["D"] > 0:
        v = "The manual helps more than the reference"
    elif c["p"] < ALPHA and c["D"] < 0:
        v = "The reference does better than the manual"
    else:
        hi = max(c["ci95_permutation"][1], c["ci95_bootstrap"][1])  # both intervals must agree
        v = "No difference detected" + (" (the reference is about as good, within 10 points)"
                                        if hi < MARGIN else "")
    return v


# ---- loading and checks ------------------------------------------------------------------------

def expected_args(player, mhash):
    return {"player": player, "taskset": "v3", "seed": SEED, "max_actions": 100, "manifest": mhash}


def check_settings(player, ev, mhash, path):
    args = ev.task_args
    want = expected_args(player, mhash)
    wrong = {k: args.get(k) for k, v in want.items() if args.get(k) != v}
    conds = args.get("conditions")
    conds = conds.split(",") if isinstance(conds, str) else conds
    if conds != CONDITIONS:
        wrong["conditions"] = conds
    for k in ("manual", "level", "tasks"):
        if args.get(k) is not None:
            wrong[k] = args.get(k)
    if ev.config.epochs != len(EPOCHS) or ev.config.max_samples != 2:
        wrong["epochs/max_samples"] = (ev.config.epochs, ev.config.max_samples)
    if wrong:
        fail(f"{player} log {path.name} was not run with the registered settings: {wrong}")


def guide_use(sample):
    """In one game: did the player open `guide do`, and how many actions showed the guide?"""
    shown = sum(1 for m in sample.messages if m.role == "user" and GUIDE_MARK in str(m.content))
    typed = any(m.role == "assistant" and re.search(r"guide\s+do", str(m.content)) for m in sample.messages)
    return {"guide_opened": shown > 0 or typed, "guide_screens": shown}


def load():
    changed = freeze.check(EXP)
    if changed:
        fail(f"frozen files changed: {changed}")
    mhash = freeze.manifest_hash(EXP)
    if not RUN_FILE.exists():
        fail("no run.json: the experiment was not started with tools/run_experiment4.py")
    run = json.loads(RUN_FILE.read_text())
    want_sha = {"plain": None, "reference": sha256(reference_text()),
                "manual": sha256((ROOT / "docs" / "manual.md").read_text(encoding="utf-8"))}
    games = []
    for player in PLAYERS:
        logs = []
        for p in LOG_DIR.glob("*.eval"):
            e = read_eval_log(str(p), header_only=True).eval
            if e.task_args.get("player") != player:
                continue
            if e.task_id != run.get(player):
                fail(f"{player}: log {p.name} belongs to another run (only the recorded run and its retries count)")
            logs.append((e.created, p))
        if not logs:
            fail(f"no log for {player}")
        for _, any_path in logs:
            check_settings(player, read_eval_log(str(any_path), header_only=True).eval, mhash, any_path)
        path = max(logs)[1]
        log = read_eval_log(str(path))
        if log.status != "success":
            fail(f"{player} log {path.name} has status {log.status}")
        for s in log.samples:
            if s.error:
                fail(f"{player} sample {s.id} epoch {s.epoch} errored (run tools/run_experiment4.py --retry)")
            sc = s.scores["end_state"]
            meta = sc.metadata or {}
            cond = s.metadata["condition"]
            # (games recovered from the ledger carry the same fields: inspect_eval.SCORED)
            if meta.get("condition") != cond or meta.get("context_sha256") != want_sha[cond]:
                fail(f"{player} {s.id} #{s.epoch}: played as {meta.get('condition')} with text "
                     f"{meta.get('context_sha256')}, registered {cond} / {want_sha[cond]}")
            if f"-{cond}-" not in (meta.get("game") or ""):
                fail(f"{player} {s.id} #{s.epoch}: game id {meta.get('game')} is not a {cond} game")
            games.append({"player": player, "task": s.metadata["task"], "condition": cond,
                          "epoch": s.epoch, "won": sc.value == "C", **meta, **guide_use(s)})
    ids = Counter(g.get("game") for g in games)
    if any(n > 1 for n in ids.values()) or None in ids:
        fail("the same underlying game appears more than once (or a game id is missing)")
    cells = Counter((g["player"], g["task"], g["condition"], g["epoch"]) for g in games)
    expected = {(p, t, c, e) for p in PLAYERS for t in TASKS for c in CONDITIONS for e in EPOCHS}
    if set(cells) != expected or any(n != 1 for n in cells.values()):
        fail(f"games don't match the plan (missing {len(expected - set(cells))}, "
             f"extra {len(set(cells) - expected)}, duplicates {sum(1 for n in cells.values() if n > 1)})")
    return games, mhash, run


def check_ledger(games):
    frozen_at = json.loads(freeze.manifest_path(EXP).read_text())["frozen_at"][:19]
    ledger = defaultdict(list)
    if LEDGER.exists():
        for line in LEDGER.read_text(encoding="utf-8").splitlines():
            a = json.loads(line)
            if a["time"] >= frozen_at and a["player"] in PLAYERS and a["task"] in TASKS:
                ledger[(a["player"], a["task"], a["condition"], a["epoch"])].append(a)
    final_ids = {g["game"] for g in games}
    for g in games:  # no finished game may ever be replaced by another
        cell = (g["player"], g["task"], g["condition"], g["epoch"])
        finished = [a for a in ledger.get(cell, []) if a.get("terminal")]
        if len(finished) != 1 or finished[0].get("game") != g["game"]:
            fail(f"{cell}: the scored game must be the cell's one finished attempt "
                 f"(ledger has {[a.get('game') for a in finished]}, scored {g['game']})")
    out = {}
    for (p, t, c, e), v in sorted(ledger.items()):
        by_game = defaultdict(list)  # one discarded attempt = all rows of one game id (booking, crash, result)
        for a in v:
            if a.get("game") not in final_ids:
                by_game[a.get("game")].append(a)
        if by_game:
            out[f"{p} {t} {c} #{e}"] = [{"game": g, "rows": rows} for g, rows in by_game.items()]
    return out


def main():
    games, mhash, run = load()
    discarded = check_ledger(games)
    primary = contrast(games, "manual", "reference")
    result = {"manifest_sha256": mhash, "games": len(games),
              "primary": {"contrast": "manual vs reference", **primary, "verdict": primary_verdict(primary)},
              "secondary": {"reference vs plain": contrast(games, "reference", "plain"),
                            "manual vs plain": contrast(games, "manual", "plain")},
              "per_player": {pl: {k: contrast(games, a, b, [pl])
                                  for k, (a, b) in {"manual vs reference": ("manual", "reference"),
                                                    "reference vs plain": ("reference", "plain"),
                                                    "manual vs plain": ("manual", "plain")}.items()}
                             for pl in PLAYERS},
              "cli_versions": {k: run.get(k) for k in ("cli_versions_at_start", "cli_versions_at_end")}}

    def rate(sel):
        g = [x for x in games if sel(x)]
        return f"{sum(x['won'] for x in g)}/{len(g)}"

    result["win_rates"] = {f"{p} / {c}": rate(lambda x, p=p, c=c: x["player"] == p and x["condition"] == c)
                           for p in PLAYERS for c in CONDITIONS}
    result["win_rates_per_task"] = {f"{t} / {c}": rate(lambda x, t=t, c=c: x["task"] == t and x["condition"] == c)
                                    for t in TASKS for c in CONDITIONS}
    result["win_rates_per_kind"] = {
        f"{k} / {c}": rate(lambda x, k=k, c=c: TASKS_ALL[x["task"]]["v3"] == k and x["condition"] == c)
        for k in ("count", "bulk", "fix", "write") for c in CONDITIONS}
    result["actions_in_won_games"] = {f"{p} / {c}": statistics.median(
        [g["actions"] for g in games if g["player"] == p and g["condition"] == c and g["won"]] or [0])
        for p in PLAYERS for c in CONDITIONS}
    result["guide_do"] = {}
    for p in PLAYERS:
        for c in CONDITIONS:
            mine = [g for g in games if g["player"] == p and g["condition"] == c]
            opened = [g for g in mine if g["guide_opened"]]
            result["guide_do"][f"{p} / {c}"] = {
                "games_that_opened_it": f"{len(opened)}/{len(mine)}",
                "median_actions_showing_it_when_opened": statistics.median(
                    [g["guide_screens"] for g in opened] or [0])}
    tag = lambda g: f"{g['player']} {g['task']} {g['condition']} #{g['epoch']}"  # noqa: E731
    result["audit_failures"] = [f"{tag(g)}: {g.get('violations')}" for g in games if g.get("audit_ok") is False]
    result["timeouts"] = [tag(g) for g in games if g.get("timed_out")]
    result["os_crashes"] = [f"{tag(g)}: {g.get('os_crash')}" for g in games if g.get("os_crash")]
    result["visible_only"] = [tag(g) for g in games if g.get("visible_passed") and g.get("hidden")
                              and not all(h["ok"] for h in g["hidden"])]
    result["recovered_from_ledger"] = [tag(g) for g in games if g.get("recovered_from_ledger")]
    result["discarded_attempts"] = discarded

    out = EXP_DIR / "results.json"
    out.write_text(json.dumps(result, indent=2))
    print(json.dumps({k: v for k, v in result.items() if k not in ("win_rates_per_task", "discarded_attempts")},
                     indent=2))
    p = result["primary"]
    print(f"\nVERDICT (manual vs reference): {p['verdict']}  (D = {p['D']:+.3f}, p = {p['p']}, "
          f"95% interval {p['ci95_permutation']})")


if __name__ == "__main__":
    main()
