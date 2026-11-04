"""Run the pre-registered experiment (PREREG.md).

  python tools/run_experiment.py           start: both players, both conditions, all 30 tasks
  python tools/run_experiment.py --retry   re-run only ERRORED samples (harness errors) of the
                                           latest log per player, with inspect eval-retry

Refuses to run unless the frozen files are unchanged. Logs go to out/experiment/logs/.
"""

import subprocess
import sys
from pathlib import Path

TOOLS = Path(__file__).resolve().parent
ROOT = TOOLS.parent
sys.path.insert(0, str(TOOLS))
import freeze  # noqa: E402

LOG_DIR = ROOT / "out" / "experiment" / "logs"
PLAYERS = ["haiku", "luna"]
SEED = 20261104
PER_PLAYER_PARALLEL = 2


def latest_log(player):
    from inspect_ai.log import read_eval_log
    logs = sorted(LOG_DIR.glob("*.eval"), key=lambda p: p.stat().st_mtime, reverse=True)
    for p in logs:
        if read_eval_log(str(p), header_only=True).eval.task_args.get("player") == player:
            return p
    return None


def main():
    changed = freeze.check()
    if changed:
        sys.exit(f"refusing: frozen files changed: {changed}")
    mhash = freeze.manifest_hash()
    LOG_DIR.mkdir(parents=True, exist_ok=True)
    procs = []
    for player in PLAYERS:
        if "--retry" in sys.argv:
            log = latest_log(player)
            if log is None:
                continue
            cmd = ["inspect", "eval-retry", str(log), "--max-samples", str(PER_PLAYER_PARALLEL),
                   "--log-dir", str(LOG_DIR), "--no-fail-on-error", "--display", "plain"]
        else:
            cmd = ["inspect", "eval", "tools/inspect_eval.py", "--model", "mockllm/model",  # relative: Inspect needs it
                   "-T", f"player={player}", "-T", "conditions=plain,manual", "-T", f"seed={SEED}",
                   "-T", f"manifest={mhash}", "--max-samples", str(PER_PLAYER_PARALLEL),
                   "--log-dir", str(LOG_DIR), "--no-fail-on-error", "--display", "plain"]
        out = open(LOG_DIR.parent / f"console-{player}.txt", "a", encoding="utf-8")
        procs.append((player, subprocess.Popen(cmd, cwd=ROOT, stdout=out, stderr=subprocess.STDOUT)))
    for player, p in procs:
        p.wait()
        print(f"{player}: inspect finished with exit code {p.returncode}")


if __name__ == "__main__":
    main()
