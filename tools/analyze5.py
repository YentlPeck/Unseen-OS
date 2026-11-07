"""The analysis of experiment 5, written before the run (PREREG5.md). Nothing here changes once the
experiment is frozen.

  python tools/analyze5.py              validate the run, then the verdicts and the tables
  python tools/analyze5.py --selftest   checks the statistics on made-up games

A game counts as WON if every hidden test passed, the audit passed, it did not time out and the
OS did not stop responding (as Inspect's scorer, tools/inspect_apps.py). Each cell (player, task,
condition, epoch) has exactly one final game: the ledger's terminal attempt.

Before any verdict the run is validated (PREREG5.md, "What counts"): the frozen files are
unchanged and run.json holds the same manifest; all 2 x 24 x 3 x 4 = 576 cells have exactly one
finished game; game ids are unique and name their own task, player and condition; each game used
its condition's registered text (SHA-256); every Luna game had the registered setup; and the
latest Inspect log of each player's bound run has the registered settings, all 288 samples
scored without error, and each sample's game and score agree with the ledger.

Primary: pass rate, manual vs plain. d_t = manual win rate - plain win rate of task t (pooled over
both players and all epochs); D = mean of d_t over the tasks; exact two-sided sign-flip test (all
2^24 sign patterns), alpha 0.05; 95% interval by inverting the test (location-shift model); a
task-level bootstrap interval.
Key secondary: actions in won games, manual vs plain. r_t = mean over the players who won t in
both conditions of ln(median actions of their won manual games / median actions of their won plain
games); R = mean of r_t over the tasks that have one; the same test and intervals; reported as
exp(R) - 1. A comparison of the games that were won, not a causal measure of efficiency.
"""

import json
import math
import random
import re
import statistics as st
import sys
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parent.parent
TOOLS = ROOT / "tools"
EXP = ROOT / "out" / "experiment5"
LEDGER = EXP / "attempts-ledger.jsonl"
PLAYERS = ("haiku", "luna")
CONDITIONS = ("plain", "reference", "manual")
EPOCHS = 4
SEED = 20261108
MAX_ACTIONS = 250
ALPHA = 0.05
INF = float("inf")


# ----------------------------------------------------------------------------- the games --

def won(r):
    return bool(r.get("passed")) and bool(r.get("audit_ok")) and not r.get("timed_out") and not r.get("os_crash")


def final_games(rows):
    """{(player, task, condition, epoch): terminal ledger row}. Two different terminal games for one
    cell are an error (a game must never be replayed)."""
    out = {}
    for r in rows:
        if not r.get("terminal") or "passed" not in r:
            continue
        cell = (r["player"], r["task"], r["condition"], r["epoch"])
        if cell in out and out[cell].get("game") != r.get("game"):
            raise RuntimeError(f"two finished games for {cell}: {out[cell].get('game')} and {r.get('game')}")
        out[cell] = r
    return out


# --------------------------------------------------------------------------- the statistics --

def _half_sums(x):
    n = len(x)
    pats = ((np.arange(2 ** n)[:, None] >> np.arange(n)) & 1) * 2 - 1
    return pats @ np.asarray(x, dtype=float)


def sign_flip_p(d):
    """Exact two-sided sign-flip p-value for mean(d) = 0: the share of all 2^n sign patterns whose
    |sum| is at least the observed |sum| (the patterns are split in two halves, so 24 tasks take
    seconds)."""
    d = [float(v) for v in d]
    n = len(d)
    if n == 0:
        return float("nan")
    obs = abs(sum(d))
    h = n // 2
    a, b = _half_sums(d[:h]), _half_sums(d[h:])
    eps = 1e-9 * max(1.0, obs)
    count = 0
    for chunk in np.array_split(a, max(1, len(a) // 512)):
        count += int(np.count_nonzero(np.abs(chunk[:, None] + b[None, :]) >= obs - eps))
    return count / float(2 ** n)


def shift_interval(d, alpha=ALPHA):
    """95% interval for the mean shift by inverting the sign-flip test: the shifts delta for which
    the test on d - delta gives p >= alpha. With n tasks the smallest possible p is 2 / 2^n, so for
    n <= 5 no shift is ever rejected and the interval is the whole line (-inf, +inf). Otherwise each
    end is found by walking out from the mean in steps of 0.0025 to the first rejected shift and
    bisecting to 0.0001; the end reported is the REJECTED side of that bracket, rounded outwards to
    0.001, so the interval always contains the true boundary."""
    d = np.asarray(d, dtype=float)
    n = len(d)
    if n == 0:
        return (float("nan"), float("nan"))
    if 2.0 / 2 ** n >= alpha:
        return (-INF, INF)
    m = float(d.mean())
    ok = lambda delta: sign_flip_p(d - delta) >= alpha
    span = float(np.abs(d).max() * 4 + 1)

    def edge(direction):
        prev = m
        while abs(prev - m) <= span:
            x = prev + direction * 0.0025
            if not ok(x):
                inside, outside = prev, x
                while abs(outside - inside) > 0.0001:
                    mid = (inside + outside) / 2
                    inside, outside = (mid, outside) if ok(mid) else (inside, mid)
                return outside
            prev = x
        return direction * INF  # never rejected within reach (should not happen for n > 5)
    low, high = edge(-1), edge(+1)
    low = low if math.isinf(low) else math.floor(low * 1000) / 1000
    high = high if math.isinf(high) else math.ceil(high * 1000) / 1000
    return (low, high)


def bootstrap_interval(d, n=10_000, seed=SEED):
    rng = random.Random(seed)
    d = list(d)
    if not d:
        return (float("nan"), float("nan"))
    means = sorted(st.mean(rng.choice(d) for _ in d) for _ in range(n))
    return (means[int(0.025 * n)], means[int(0.975 * n) - 1])


def test(d):
    d = list(d)
    return {"n": len(d), "mean": st.mean(d) if d else float("nan"), "p": sign_flip_p(d),
            "interval": shift_interval(d), "bootstrap": bootstrap_interval(d)}


# ----------------------------------------------------------------------------- the measures --

def games_of(games, task, cond, players):
    return [g for (p, t, c, _), g in games.items() if t == task and c == cond and p in players]


def win_rate_diffs(games, tasks, a, b, players=PLAYERS):
    """d_t = win rate in condition a - win rate in condition b, pooled over the players."""
    out = []
    for t in tasks:
        ga, gb = games_of(games, t, a, players), games_of(games, t, b, players)
        if ga and gb:
            out.append(sum(map(won, ga)) / len(ga) - sum(map(won, gb)) / len(gb))
    return out


def action_ratios(games, tasks, a, b, players=PLAYERS, detail=None):
    """r_t = mean over the players who won t in both a and b of ln(median actions in their won a
    games / median actions in their won b games). Tasks without such a player are left out (and
    listed in `detail`, if given)."""
    out = []
    for t in tasks:
        logs, used = [], []
        for p in players:
            wa = [g["actions"] for g in games_of(games, t, a, (p,)) if won(g)]
            wb = [g["actions"] for g in games_of(games, t, b, (p,)) if won(g)]
            if wa and wb:
                logs.append(math.log(st.median(wa) / st.median(wb)))
                used.append(f"{p} ({len(wa)} vs {len(wb)} won)")
        if logs:
            out.append(st.mean(logs))
        if detail is not None:
            detail.append((t, used))
    return out


def pct(x):
    return "unbounded" if math.isinf(x) else f"{(math.exp(x) - 1) * 100:+.0f}%"


def pts(x):
    return "unbounded" if math.isinf(x) else f"{x * 100:+.1f}"


def verdict(name, res, higher_is_better=True):
    if not res["n"]:
        return f"{name}: no tasks to compare"
    good = res["mean"] > 0 if higher_is_better else res["mean"] < 0
    if res["p"] < ALPHA:
        return f"{name}: {'the manual helps' if good else 'the manual does worse'} (p = {res['p']:.4f})"
    return f"{name}: no difference detected (p = {res['p']:.4f})"


# ------------------------------------------------------------------------------- the report --

def report(games, tasks_meta, rows=None):
    tasks = sorted(tasks_meta)
    out = []
    w = test(win_rate_diffs(games, tasks, "manual", "plain"))
    detail = []
    a = test(action_ratios(games, tasks, "manual", "plain", detail=detail))
    out.append("PRIMARY  " + verdict("pass rate, manual vs plain", w))
    out.append(f"   D = {pts(w['mean'])} points over {w['n']} tasks; 95% interval {pts(w['interval'][0])} to "
               f"{pts(w['interval'][1])}; bootstrap {pts(w['bootstrap'][0])} to {pts(w['bootstrap'][1])}")
    out.append("KEY SECONDARY  " + verdict("actions in won games, manual vs plain", a, higher_is_better=False))
    if a["n"]:
        out.append(f"   actions {pct(a['mean'])} over {a['n']} tasks; 95% interval {pct(a['interval'][0])} to "
                   f"{pct(a['interval'][1])}; bootstrap {pct(a['bootstrap'][0])} to {pct(a['bootstrap'][1])}")
    left_out = [t for t, used in detail if not used]
    out.append(f"   left out (no player won them in both conditions): {len(left_out)} {left_out}")
    out.append("   players used per task: " + "; ".join(f"{t}: {', '.join(u)}" for t, u in detail if u))
    out.append("   (the two verdicts are separate questions; neither is corrected for the other, and there is"
               " no experiment-wide 5% guarantee)")
    out.append("")
    out.append("Descriptive (no verdict):")
    for x, y in (("manual", "plain"), ("reference", "plain"), ("manual", "reference")):
        for label, players in (("both players", PLAYERS),) + tuple((p, (p,)) for p in PLAYERS):
            if (x, y) == ("manual", "plain") and label == "both players":
                continue  # that is the verdict above
            wr = test(win_rate_diffs(games, tasks, x, y, players))
            ar = test(action_ratios(games, tasks, x, y, players))
            out.append(f"  {x} vs {y}, {label}: pass rate {pts(wr['mean'])} points (p = {wr['p']:.3f}, 95% "
                       f"{pts(wr['interval'][0])} to {pts(wr['interval'][1])}, bootstrap {pts(wr['bootstrap'][0])} to "
                       f"{pts(wr['bootstrap'][1])}); actions "
                       + (f"{pct(ar['mean'])} (p = {ar['p']:.3f}, 95% {pct(ar['interval'][0])} to {pct(ar['interval'][1])}, "
                          f"bootstrap {pct(ar['bootstrap'][0])} to {pct(ar['bootstrap'][1])}, {ar['n']} tasks)"
                          if ar["n"] else "no tasks"))
    out.append("")
    out.append("Wins per player and condition:")
    for p in PLAYERS:
        out.append(f"  {p}: " + ", ".join(
            f"{c} {sum(map(won, gs))}/{len(gs)}" for c in CONDITIONS
            for gs in [[g for (pp, _, cc, _), g in games.items() if pp == p and cc == c]]))
    for key in ("kind", "level"):
        out.append(f"Wins by {key}:")
        for v in sorted({m[key] for m in tasks_meta.values()}):
            out.append(f"  {v}: " + ", ".join(
                f"{c} {sum(map(won, gs))}/{len(gs)}" for c in CONDITIONS
                for gs in [[g for (_, t, cc, _), g in games.items() if cc == c and tasks_meta[t][key] == v]]))
    out.append("Wins per task (out of 8 per condition; Haiku + Luna):")
    for t in tasks:
        cells = []
        for c in CONDITIONS:
            h = sum(map(won, games_of(games, t, c, ("haiku",))))
            l_ = sum(map(won, games_of(games, t, c, ("luna",))))
            cells.append(f"{c} {h}+{l_}")
        out.append(f"  {t:<20} {tasks_meta[t]['kind']:<8} {tasks_meta[t]['level']:<7} " + "  ".join(cells))
    out.append("Median actions and minutes in won games:")
    for p in PLAYERS:
        row = []
        for c in CONDITIONS:
            gs = [g for (pp, _, cc, _), g in games.items() if pp == p and cc == c and won(g)]
            if gs:
                row.append(f"{c} {st.median(g['actions'] for g in gs):.0f} actions, {st.median(g['seconds'] for g in gs) / 60:.1f} min ({len(gs)} won)")
        out.append(f"  {p}: " + "; ".join(row))
    out.append(f"Audit failures {sum(1 for g in games.values() if not g.get('audit_ok'))}, OS crashes "
               f"{sum(1 for g in games.values() if g.get('os_crash'))}, time-outs {sum(1 for g in games.values() if g.get('timed_out'))}")
    out.append("Final games that were lost other than by failing hidden tests:")
    for cell, g in sorted(games.items()):
        why = [w for w, bad in (("AUDIT FAILURE", not g.get("audit_ok")), ("OS CRASH", g.get("os_crash")),
                                ("TIME-OUT", g.get("timed_out"))) if bad]
        if why:
            out.append(f"  {cell}: {g.get('game')} {', '.join(why)}" + (f" {g.get('violations')}" if not g.get("audit_ok") else ""))
    if rows is not None:
        out.append("")
        out.append("Every booked attempt, with its round and what became of it:")
        outcome = {}
        for r in rows:
            if not r.get("started") and not r.get("reused_from_ledger") and r.get("game"):
                outcome.setdefault(r["game"], []).append(r)
        finals = {g["game"] for g in games.values()}
        for r in rows:
            if not r.get("started"):
                continue
            o = outcome.get(r["game"], [])
            last = o[-1] if o else None
            if last is None:
                what = "NO OUTCOME RECORDED"
            elif last.get("discarded"):
                what = "discarded (interrupted, nothing to recover): a harness error"
            elif last.get("terminal"):
                what = ("final game: " if r["game"] in finals else "terminal but not final (!): ") + (
                    "won" if won(last) else "lost") + (" (recovered from its saved result)" if last.get("recovered_from_result") else "")
            else:
                what = f"harness error, re-run: {last.get('harness_errors')}"
            out.append(f"  {r.get('time')} round {r.get('round', 0)} {r.get('player')} {r.get('task')} {r.get('condition')} "
                       f"epoch {r.get('epoch')} {r['game']}: {what}")
        reused = [r for r in rows if r.get("reused_from_ledger")]
        out.append(f"Finished games reused from the ledger (not replayed): {len(reused)}")
        for r in reused:
            out.append(f"  {r.get('time')} round {r.get('round', 0)} {r.get('player')} {r.get('task')} {r.get('condition')} "
                       f"epoch {r.get('epoch')} {r.get('game')}")
    return "\n".join(out)


# ------------------------------------------------------------------------------- validation --

def registered_hashes():
    import hashlib
    sha = lambda p: hashlib.sha256((ROOT / p).read_text(encoding="utf-8").encode("utf-8")).hexdigest()
    return {"plain": None, "reference": sha("kernel/src/script_guide.txt"), "manual": sha("docs/manual.md")}


def validate(rows, run, tasks, logs):
    """Problems that forbid a verdict (an empty list: the run is valid). `logs`: {player: eval log}."""
    sys.path.insert(0, str(TOOLS))
    import freeze5
    from apps_game import LUNA_FEATURES_OFF
    problems = []
    changed = freeze5.check()
    if changed:
        problems.append(f"frozen files changed: {changed}")
    mhash = freeze5.manifest_hash() if freeze5.MANIFEST.exists() else None
    if run.get("manifest") != mhash:
        problems.append(f"run.json manifest {run.get('manifest')} is not the frozen manifest {mhash}")
    if run.get("epochs") != EPOCHS:
        problems.append(f"run.json says {run.get('epochs')} epochs, the registration {EPOCHS}")
    try:
        games = final_games(rows)
    except RuntimeError as e:
        return problems + [str(e)]
    expected = {(p, t, c, e) for p in PLAYERS for t in tasks for c in CONDITIONS for e in range(1, EPOCHS + 1)}
    missing, extra = sorted(expected - set(games)), sorted(set(games) - expected)
    if missing:
        problems.append(f"{len(missing)} cells have no finished game, e.g. {missing[:5]}")
    if extra:
        problems.append(f"{len(extra)} finished games are not part of the run, e.g. {extra[:5]}")
    ids = [g.get("game") for g in games.values()]
    if len(set(ids)) != len(ids):
        problems.append("one game is the final game of more than one cell")
    hashes = registered_hashes()
    for (p, t, c, e), g in games.items():
        if not re.fullmatch(rf"{re.escape(t)}-{p}-{c}-[0-9a-f]{{6}}", g.get("game") or ""):
            problems.append(f"{(p, t, c, e)}: game id {g.get('game')} does not name its own task, player and condition")
        if g.get("context_sha256") != hashes[c]:
            problems.append(f"{(p, t, c, e)}: played with another text than the registered {c} text")
        if won(g) and not (isinstance(g.get("actions"), int) and g["actions"] > 0):
            problems.append(f"{(p, t, c, e)}: a won game without a positive action count ({g.get('actions')})")
        if p == "luna":
            s = g.get("luna_setup") or {}
            if s.get("effort") != "high" or not s.get("sandbox_id_reused") or s.get("features_off") != LUNA_FEATURES_OFF:
                problems.append(f"{(p, t, c, e)}: Luna's setup was not the registered one ({s})")
    for p in PLAYERS:
        log = logs.get(p)
        if log is None:
            problems.append(f"no Inspect log for {p}'s bound run {run.get(p)}")
            continue
        args = log.eval.task_args
        want = {"player": p, "taskset": "bench", "conditions": "plain,reference,manual", "seed": SEED,
                "max_actions": MAX_ACTIONS, "manifest": mhash}
        for k, v in want.items():
            if str(args.get(k)) != str(v):
                problems.append(f"{p}'s Inspect run has {k} = {args.get(k)}, registered {v}")
        if log.eval.config.epochs != EPOCHS:
            problems.append(f"{p}'s Inspect run has {log.eval.config.epochs} epochs")
        samples = log.samples or []
        keys = [(str(s.id), s.epoch) for s in samples]
        want_keys = {(f"{t}:{c}", e) for t in tasks for c in CONDITIONS for e in range(1, EPOCHS + 1)}
        if len(keys) != len(set(keys)) or set(keys) != want_keys:
            problems.append(f"{p}'s latest Inspect log does not hold every (task, condition, epoch) exactly once "
                            f"({len(keys)} samples, {len(set(keys))} distinct, {len(want_keys - set(keys))} missing)")
        for s in samples:
            t, c = str(s.id).split(":")
            g = games.get((p, t, c, s.epoch))
            score = next(iter((s.scores or {}).values()), None)
            if s.error or score is None:
                problems.append(f"{p} {s.id} epoch {s.epoch}: the sample has no score ({s.error})")
            elif g is None or (score.metadata or {}).get("game") != g.get("game"):
                problems.append(f"{p} {s.id} epoch {s.epoch}: the Inspect sample's game is not the ledger's final game")
            elif (score.value == "C") != won(g):
                problems.append(f"{p} {s.id} epoch {s.epoch}: Inspect's score and the ledger disagree")
    return problems


def latest_logs(run):
    from inspect_ai.log import read_eval_log
    best = {}
    for path in (EXP / "logs").glob("*.eval"):
        head = read_eval_log(str(path), header_only=True)
        for p in PLAYERS:
            if head.eval.task_id == run.get(p) and (p not in best or head.eval.created > best[p][0]):
                best[p] = (head.eval.created, path)
    return {p: read_eval_log(str(path)) for p, (_, path) in best.items()}


def opened_guide(game_id):
    try:
        rows = (EXP / "runs" / game_id / "transcript.jsonl").read_text(encoding="utf-8").splitlines()
    except OSError:
        return None
    return any(r.get("action") in ("type", "line") and "guide script" in " ".join(r.get("args") or [])
               for r in (json.loads(l) for l in rows if l.strip()))


def main():
    if "--selftest" in sys.argv:
        return selftest()
    sys.path.insert(0, str(TOOLS))
    from tasks_bench import TASKS
    rows = [json.loads(l) for l in LEDGER.read_text(encoding="utf-8").splitlines() if l.strip()]
    run = json.loads((EXP / "run.json").read_text())
    problems = validate(rows, run, sorted(TASKS), latest_logs(run))
    if problems:
        print(f"refusing a verdict: {len(problems)} problems")
        for p in problems[:40]:
            print("  " + p)
        return 1
    games = final_games(rows)
    print(report(games, {t: {"kind": TASKS[t]["kind"], "level": TASKS[t]["level"]} for t in TASKS}, rows))
    print("Games that opened guide script:")
    for c in CONDITIONS:
        seen = [opened_guide(g["game"]) for (_, _, cc, _), g in games.items() if cc == c]
        print(f"  {c}: {sum(1 for x in seen if x)} of {len(seen)} ({sum(1 for x in seen if x is None)} without a transcript)")
    return 0


def selftest():
    fails = []

    def check(name, ok):
        print(("ok   " if ok else "FAIL ") + name)
        if not ok:
            fails.append(name)

    d = [0.3, -0.1, 0.25, 0.4, 0.05, -0.2, 0.15]
    direct = sum(1 for k in range(2 ** len(d)) if abs(sum(v if (k >> i) & 1 else -v for i, v in enumerate(d))) >= abs(sum(d)) - 1e-12) / 2 ** len(d)
    check("sign-flip p equals a direct enumeration", abs(sign_flip_p(d) - direct) < 1e-12)
    check("all zero differences give p = 1", sign_flip_p([0.0] * 10) == 1.0)
    check("24 equal positive differences give the smallest p", abs(sign_flip_p([0.1] * 24) - 2 / 2 ** 24) < 1e-15)
    lo, hi = shift_interval([0.2, 0.1, 0.3, 0.25, 0.15, 0.2, 0.05, 0.35])
    check("the interval holds the mean and excludes 0 for clear effects", lo < 0.2 < hi and lo > 0)
    check("with 5 or fewer tasks the interval is unbounded", shift_interval([0.3, 0.4, 0.2, 0.5, 0.1]) == (-INF, INF))
    d6 = [0.3, 0.4, 0.2, 0.5, 0.1, 0.35]
    lo6, hi6 = shift_interval(d6)
    check("the interval's ends are rejected shifts (it contains the boundary)",
          sign_flip_p(np.asarray(d6) - lo6) < ALPHA and sign_flip_p(np.asarray(d6) - hi6) < ALPHA
          and sign_flip_p(np.asarray(d6) - (lo6 + 0.002)) >= ALPHA)
    rng = random.Random(1)
    tasks = {f"t{i}": {"kind": ["build", "feature", "fix"][i % 3], "level": ["easy", "medium", "hard"][i % 3]} for i in range(24)}
    games = {}
    for p in PLAYERS:
        for t in tasks:
            for c in CONDITIONS:
                for e in range(1, EPOCHS + 1):
                    pwin = {"plain": 0.6, "reference": 0.7, "manual": 0.85}[c]
                    acts = {"plain": 80, "reference": 55, "manual": 45}[c] * rng.uniform(0.7, 1.3)
                    games[(p, t, c, e)] = {"game": f"{t}-{p}-{c}-{e:06x}", "passed": rng.random() < pwin, "audit_ok": True,
                                           "timed_out": False, "os_crash": None, "actions": round(acts), "seconds": 300}
    w = test(win_rate_diffs(games, sorted(tasks), "manual", "plain"))
    a = test(action_ratios(games, sorted(tasks), "manual", "plain"))
    check("made-up effect: the pass-rate test finds it", w["p"] < ALPHA and w["mean"] > 0)
    check("made-up effect: the action test finds fewer actions", a["p"] < ALPHA and a["mean"] < 0)
    none = {k: dict(g, passed=(random.Random(hash(k) & 0xffff).random() < 0.7)) for k, g in games.items()}
    check("no made-up effect: a moderate p", test(win_rate_diffs(none, sorted(tasks), "manual", "plain"))["p"] > 0.01)
    try:
        final_games([{"terminal": True, "passed": True, "player": "haiku", "task": "t", "condition": "plain", "epoch": 1, "game": "a"},
                     {"terminal": True, "passed": False, "player": "haiku", "task": "t", "condition": "plain", "epoch": 1, "game": "b"}])
        check("two finished games for one cell are refused", False)
    except RuntimeError:
        check("two finished games for one cell are refused", True)
    # validation, on a complete made-up run (the frozen-file check is replaced by a stub)
    from types import SimpleNamespace as NS
    sys.path.insert(0, str(TOOLS))
    import freeze5
    from apps_game import LUNA_FEATURES_OFF
    real_check, real_hash, real_exists = freeze5.check, freeze5.manifest_hash, freeze5.MANIFEST
    freeze5.check, freeze5.manifest_hash = (lambda: []), (lambda: "m1")
    freeze5.MANIFEST = Path(__file__)  # exists
    try:
        h = registered_hashes()
        names = sorted(tasks)
        rows = []
        for (p, t, c, e), g in games.items():
            gid = f"{t}-{p}-{c}-{(hash((p, t, c, e)) & 0xffffff):06x}"
            rows.append({**g, "game": gid, "player": p, "task": t, "condition": c, "epoch": e, "terminal": True,
                         "context_sha256": h[c],
                         "luna_setup": {"effort": "high", "sandbox_id_reused": True, "features_off": LUNA_FEATURES_OFF} if p == "luna" else None})

        def logs_for(rows):
            fin = final_games(rows)
            out = {}
            for p in PLAYERS:
                samples = [NS(id=f"{t}:{c}", epoch=e, error=None,
                              scores={"hidden_tests": NS(value="C" if won(g) else "I", metadata={"game": g["game"]})})
                           for (pp, t, c, e), g in fin.items() if pp == p]
                out[p] = NS(eval=NS(task_args={"player": p, "taskset": "bench", "conditions": "plain,reference,manual",
                                               "seed": SEED, "max_actions": MAX_ACTIONS, "manifest": "m1"},
                                    config=NS(epochs=EPOCHS)), samples=samples)
            return out
        run = {"manifest": "m1", "epochs": EPOCHS, "haiku": "a", "luna": "b"}
        check("validation accepts a complete, correct run", validate(rows, run, names, logs_for(rows)) == [])
        first_win = next(j for j, row in enumerate(rows) if won(row))
        cases = {
            "a missing cell": rows[1:],
            "a game in another condition's id": [dict(rows[0], game=rows[0]["game"].replace(f"-{rows[0]['condition']}-", "-xx-"))] + rows[1:],
            "another prompt text": [dict(rows[0], context_sha256="zz")] + rows[1:],
            "Luna at medium effort": [dict(r, luna_setup={**r["luna_setup"], "effort": "medium"}) if r["player"] == "luna" else r for r in rows],
            "a won game without actions": [dict(row, actions=None) if i == first_win else row for i, row in enumerate(rows)],
        }
        for name, bad in cases.items():
            check(f"validation refuses {name}", validate(bad, run, names, logs_for(rows)) != [])
        check("validation refuses fewer epochs in run.json", validate(rows, dict(run, epochs=1), names, logs_for(rows)) != [])
        wrong_log = logs_for(rows)
        wrong_log["haiku"].eval.task_args["seed"] = 1
        check("validation refuses an Inspect run with other settings", validate(rows, run, names, wrong_log) != [])
        dup = logs_for(rows)
        dup["haiku"].samples[1] = dup["haiku"].samples[0]
        check("validation refuses an Inspect log with a sample twice and one missing", validate(rows, run, names, dup) != [])
        flipped = logs_for(rows)
        s0 = flipped["luna"].samples[0]
        s0.scores["hidden_tests"].value = "I" if s0.scores["hidden_tests"].value == "C" else "C"
        check("validation refuses Inspect scores that disagree with the ledger", validate(rows, run, names, flipped) != [])
    finally:
        freeze5.check, freeze5.manifest_hash, freeze5.MANIFEST = real_check, real_hash, real_exists
    booked = [{"started": True, "game": "g1", "round": 0, "player": "haiku", "task": "t0", "condition": "plain", "epoch": 1},
              {"game": "g1", "round": 0, "harness_errors": ["x"], "terminal": False}]
    text = report(games, tasks, booked)
    check("the report has every promised part", all(s in text for s in (
        "PRIMARY", "KEY SECONDARY", "bootstrap", "left out", "reference vs plain", "manual vs reference",
        "manual vs plain, haiku", "Wins per task", "Wins by kind", "Median actions", "lost other than",
        "Every booked attempt", "harness error, re-run", "reused from the ledger")))
    print(text.split("\n")[0])
    print("ALL GOOD" if not fails else f"{len(fails)} FAILED: {fails}")
    return 1 if fails else 0


if __name__ == "__main__":
    sys.exit(main())
