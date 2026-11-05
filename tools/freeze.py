"""Freeze (or check) the files that decide an experiment's result.

  python tools/freeze.py [--exp 2]           write the manifest (refuses if it exists)
  python tools/freeze.py [--exp 2] --check   compare the files with the manifest

Experiment 1 (default): out/experiment/manifest.json. Experiment 2: out/experiment2/manifest.json.
The manifest's hash is recorded in every experiment log (task arg `manifest`), and the
runner and analysis refuse to run if any frozen file has changed.
"""

import hashlib
import json
import shutil
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

COMMON = ["out/unseen.bin", "docs/manual.md", "tools/tasks.py", "tools/expected.json", "tools/validate_tasks.py", "tools/audit.py",
          "tools/run_task.py", "tools/play.py", "tools/broker.py", "tools/unseen.py", "tools/inspect_eval.py",
          "tools/robot_player.py", "tools/freeze.py"]
EXPERIMENTS = {
    1: {"dir": "out/experiment", "files": COMMON + ["PREREG.md", "tools/run_experiment.py", "tools/analyze.py"]},
    2: {"dir": "out/experiment2", "files": COMMON + ["PREREG2.md", "tools/tasks_v2.py", "tools/run_experiment2.py",
                                                     "tools/analyze2.py"]},
}


def _exp(n=None):
    if n is None:
        n = int(sys.argv[sys.argv.index("--exp") + 1]) if "--exp" in sys.argv else 1
    return n, EXPERIMENTS[n]


def manifest_path(n=None):
    return ROOT / _exp(n)[1]["dir"] / "manifest.json"


def hashes(n=None):
    return {f: hashlib.sha256((ROOT / f).read_bytes()).hexdigest() for f in _exp(n)[1]["files"]}


def manifest_hash(n=None):
    return hashlib.sha256(manifest_path(n).read_bytes()).hexdigest()


def check(n=None):
    """Frozen files that changed (empty = all good). Raises if not frozen yet."""
    frozen = json.loads(manifest_path(n).read_text())["files"]
    now = hashes(n)
    return [f for f, h in now.items() if frozen.get(f) != h] + [f for f in frozen if f not in now]


def main():
    n, _ = _exp()
    if "--check" in sys.argv:
        changed = check(n)
        print("all frozen files unchanged" if not changed else f"CHANGED: {changed}")
        return 1 if changed else 0
    path = manifest_path(n)
    if path.exists():
        print(f"refusing: {path} already exists (experiment {n} is frozen)")
        return 1
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps({"experiment": n, "frozen_at": time.strftime("%Y-%m-%d %H:%M:%S %z"),
                                "files": hashes(n)}, indent=2))
    shutil.copy(path, ROOT / "docs" / f"manifest-exp{n}-{time.strftime('%Y%m%d-%H%M%S')}.json")
    print(f"wrote {path}\nmanifest sha256: {manifest_hash(n)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
