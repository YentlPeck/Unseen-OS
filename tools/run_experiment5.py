"""Run pre-registered experiment 5, the Unseen App Benchmark (PREREG5.md).

  python tools/run_experiment5.py           start (only once): both players, all 3 conditions
                                            (plain, reference, manual), 24 tasks, 4 epochs
  python tools/run_experiment5.py --retry   re-run only ERRORED samples, with inspect eval-retry; each
                                            --retry is a new retry round (PREREG5.md): a cell that
                                            ended with two harness errors gets two more attempts, and
                                            a finished game is never replayed

The first start records which runs count (out/experiment5/run.json: the Inspect task_id per
player). A second start is refused, so finished games can never be replaced by a fresh run.
Refuses to run if any frozen file changed (tools/freeze5.py).
"""

import json
import os
import shutil
import subprocess
import sys
import time
from pathlib import Path

TOOLS = Path(__file__).resolve().parent
ROOT = TOOLS.parent
sys.path.insert(0, str(TOOLS))
import freeze5  # noqa: E402
from apps_game import LUNA_SANDBOX_ID  # noqa: E402

EXP_DIR = ROOT / "out" / "experiment5"
LOG_DIR = EXP_DIR / "logs"
RUNS_DIR = EXP_DIR / "runs"
RUN_FILE = EXP_DIR / "run.json"
LEDGER = EXP_DIR / "attempts-ledger.jsonl"
PLAYERS = ["haiku", "luna"]
SEED = 20261108
EPOCHS = 4
MAX_ACTIONS = 250
PER_PLAYER_PARALLEL = 4


def logs_by_player():
    from inspect_ai.log import read_eval_log
    out = {}
    for p in LOG_DIR.glob("*.eval"):
        e = read_eval_log(str(p), header_only=True).eval
        out.setdefault(e.task_args.get("player"), []).append((e.created, e.task_id, p))
    return out


def cli_versions():
    out = {}
    for name in ("claude", "codex"):
        try:
            exe = shutil.which(name) or name
            out[name] = subprocess.run([exe, "--version"], capture_output=True, text=True, timeout=60).stdout.strip()
        except Exception as e:  # recorded, never fatal
            out[name] = f"unknown ({e!r})"
    return out


def note(run, key, value):
    run.setdefault(key, []).append({"time": time.strftime("%Y-%m-%d %H:%M:%S"), **value})
    RUN_FILE.write_text(json.dumps(run, indent=2))


def main():
    changed = freeze5.check()
    if changed:
        sys.exit(f"refusing: frozen files changed: {changed}")
    mhash = freeze5.manifest_hash()
    if not LUNA_SANDBOX_ID.exists():
        sys.exit(f"refusing: Luna's saved sandbox identity is missing ({LUNA_SANDBOX_ID}); see docs/calibration5.md")
    retry = "--retry" in sys.argv
    if not retry and (RUN_FILE.exists() or any(LOG_DIR.glob("*.eval"))):
        sys.exit("refusing: experiment 5 was already started; use --retry for errored samples only")
    if retry and not RUN_FILE.exists():
        sys.exit("refusing: nothing to retry (no run.json)")
    LOG_DIR.mkdir(parents=True, exist_ok=True)
    if not retry:  # book the run BEFORE anything starts
        RUN_FILE.write_text(json.dumps({"started": time.strftime("%Y-%m-%d %H:%M:%S"), "epochs": EPOCHS,
                                        "manifest": mhash, "round": 0}, indent=2))
    run = json.loads(RUN_FILE.read_text())
    if run.get("manifest") != mhash:
        sys.exit(f"refusing: run.json was started with manifest {run.get('manifest')}, the files now give {mhash}")
    if retry:  # a new retry round
        run["round"] = run.get("round", 0) + 1
        RUN_FILE.write_text(json.dumps(run, indent=2))
    if retry:  # repair a binding that an interruption left out (only if it is unambiguous)
        found = logs_by_player()
        for player in PLAYERS:
            ids = {x[1] for x in found.get(player, [])}
            if player not in run and len(ids) == 1:
                run[player] = ids.pop()
            elif player not in run and len(ids) > 1:
                sys.exit(f"refusing: {player} has logs from more than one run; check by hand")
        RUN_FILE.write_text(json.dumps(run, indent=2))

    note(run, "cli_versions_at_start", cli_versions())
    # the registered setup is set here, whatever the environment says (PREREG5.md, Setup)
    env = {**os.environ, "UNSEEN_LEDGER": str(LEDGER), "UNSEEN_APPS_RUNS": str(RUNS_DIR),
           "UNSEEN_LUNA_EFFORT": "high", "UNSEEN_REQUIRE_SANDBOX_ID": "1", "UNSEEN_RETRY_ROUND": str(run["round"])}
    note(run, "rounds", {"round": run["round"]})
    procs = []
    for player in PLAYERS:
        mine = [x for x in logs_by_player().get(player, []) if x[1] == run.get(player)]
        if retry and mine:
            cmd = ["inspect", "eval-retry", str(max(mine)[2]), "--max-samples", str(PER_PLAYER_PARALLEL),
                   "--log-dir", str(LOG_DIR), "--no-fail-on-error", "--display", "plain"]
        elif retry and player in run:
            continue
        else:  # first start, or a player whose run never began (no log at all)
            cmd = ["inspect", "eval", "tools/inspect_apps.py", "--model", "mockllm/model",
                   "-T", f"player={player}", "-T", "taskset=bench", "-T", "conditions=plain,reference,manual",
                   "-T", f"seed={SEED}", "-T", f"max_actions={MAX_ACTIONS}", "-T", f"manifest={mhash}",
                   "--epochs", str(EPOCHS), "--max-samples", str(PER_PLAYER_PARALLEL),
                   "--log-dir", str(LOG_DIR), "--no-fail-on-error", "--display", "plain"]
        out = open(EXP_DIR / f"console-{player}.txt", "a", encoding="utf-8")
        procs.append((player, subprocess.Popen(cmd, cwd=ROOT, stdout=out, stderr=subprocess.STDOUT, env=env)))

    for _ in range(300):  # record each player's run as soon as its log exists
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
    run = json.loads(RUN_FILE.read_text())
    note(run, "cli_versions_at_end", cli_versions())


if __name__ == "__main__":
    main()
