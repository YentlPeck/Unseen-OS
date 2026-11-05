"""Run pre-registered experiment 3 (PREREG3.md).

  python tools/run_experiment3.py           start (only once): both players, both conditions,
                                            16 tasks, 3 epochs
  python tools/run_experiment3.py --retry   re-run only ERRORED samples of the run, with
                                            inspect eval-retry

The first start records which runs count (out/experiment3/run.json: the Inspect task_id
per player; eval-retry keeps the task_id). A second start is refused, so finished games can
never be replaced by a fresh run. Refuses to run if any frozen file changed.
"""

import json
import os
import subprocess
import sys
import time
from pathlib import Path

TOOLS = Path(__file__).resolve().parent
ROOT = TOOLS.parent
sys.path.insert(0, str(TOOLS))
import freeze  # noqa: E402

EXP = 3
EXP_DIR = ROOT / "out" / "experiment3"
LOG_DIR = EXP_DIR / "logs"
RUN_FILE = EXP_DIR / "run.json"
LEDGER = EXP_DIR / "attempts-ledger.jsonl"
PLAYERS = ["haiku", "luna"]
SEED = 20261113
EPOCHS = 3
MAX_ACTIONS = 100
PER_PLAYER_PARALLEL = 2


def logs_by_player():
    from inspect_ai.log import read_eval_log
    out = {}
    for p in LOG_DIR.glob("*.eval"):
        e = read_eval_log(str(p), header_only=True).eval
        out.setdefault(e.task_args.get("player"), []).append((e.created, e.task_id, p))
    return out


def main():
    changed = freeze.check(EXP)
    if changed:
        sys.exit(f"refusing: frozen files changed: {changed}")
    mhash = freeze.manifest_hash(EXP)
    retry = "--retry" in sys.argv
    if not retry and (RUN_FILE.exists() or any(LOG_DIR.glob("*.eval"))):
        sys.exit("refusing: experiment 3 was already started; use --retry for errored samples only")
    if retry and not RUN_FILE.exists():
        sys.exit("refusing: nothing to retry (no run.json)")
    LOG_DIR.mkdir(parents=True, exist_ok=True)
    if not retry:  # book the run BEFORE anything starts
        RUN_FILE.write_text(json.dumps({"started": time.strftime("%Y-%m-%d %H:%M:%S")}, indent=2))
    run = json.loads(RUN_FILE.read_text())
    if retry:  # repair a binding that an interruption left out (only if it is unambiguous)
        found = logs_by_player()
        for player in PLAYERS:
            ids = {x[1] for x in found.get(player, [])}
            if player not in run and len(ids) == 1:
                run[player] = ids.pop()
            elif player not in run and len(ids) > 1:
                sys.exit(f"refusing: {player} has logs from more than one run; check by hand")
        RUN_FILE.write_text(json.dumps(run, indent=2))

    procs = []
    for player in PLAYERS:
        mine = [x for x in logs_by_player().get(player, []) if x[1] == run.get(player)]
        if retry and mine:
            log = max(mine)[2]
            cmd = ["inspect", "eval-retry", str(log), "--max-samples", str(PER_PLAYER_PARALLEL),
                   "--log-dir", str(LOG_DIR), "--no-fail-on-error", "--display", "plain"]
        elif retry and player in run:
            continue  # bound run without logs can't happen; nothing to do
        else:  # first start, or a player whose run never began (no log at all): nothing was played
            cmd = ["inspect", "eval", "tools/inspect_eval.py", "--model", "mockllm/model",
                   "-T", f"player={player}", "-T", "taskset=v3", "-T", "conditions=plain,manual",
                   "-T", f"seed={SEED}", "-T", f"max_actions={MAX_ACTIONS}", "-T", f"manifest={mhash}",
                   "--epochs", str(EPOCHS), "--max-samples", str(PER_PLAYER_PARALLEL),
                   "--log-dir", str(LOG_DIR), "--no-fail-on-error", "--display", "plain"]
        out = open(EXP_DIR / f"console-{player}.txt", "a", encoding="utf-8")
        env = {**os.environ, "UNSEEN_LEDGER": str(LEDGER)}  # this experiment's own attempts ledger
        procs.append((player, subprocess.Popen(cmd, cwd=ROOT, stdout=out, stderr=subprocess.STDOUT, env=env)))

    # Record each player's run as soon as its log exists (if interrupted, --retry repairs it).
    for _ in range(300):
        found = logs_by_player()
        for p in PLAYERS:
            if p not in run and found.get(p):
                run[p] = found[p][0][1]
                RUN_FILE.write_text(json.dumps(run, indent=2))
        if all(p in run for p in PLAYERS):
            break
        time.sleep(1)
    else:
        print("WARNING: not every run was recorded yet; run --retry later to repair run.json")

    for player, p in procs:
        p.wait()
        print(f"{player}: inspect finished with exit code {p.returncode}")


if __name__ == "__main__":
    main()
