"""Freeze (or check) the files that decide experiment 5's result (PREREG5.md).

  python tools/freeze5.py           write out/experiment5/manifest.json (refuses if it exists)
  python tools/freeze5.py --check   compare the files with the manifest

A separate file from tools/freeze.py, which is itself frozen for experiments 1-4. The manifest's
hash is recorded in every experiment-5 log (task arg `manifest`); the runner and the analysis
refuse to run if any frozen file changed.
"""

import hashlib
import json
import shutil
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DIR = ROOT / "out" / "experiment5"
MANIFEST = DIR / "manifest.json"
FILES = [
    "out/unseen.bin", "docs/manual.md", "kernel/src/script_guide.txt", "script/tests/reference.rs",
    "PREREG5.md", "PREREG4.md", "docs/benchmark-design.md", "docs/calibration5.md",
    # local modules the harness imports (run_task.py and tasks.py import these)
    "tools/hidden.py", "tools/reference.py", "tools/tasks_v2.py", "tools/tasks_v3.py", "tools/world_v3.py",
    "tools/tasks_bench.py", "tools/tasks_bench_build.py", "tools/bench_apps.py", "tools/tasks_calib.py",
    "tools/tasks.py", "tools/apps_grade.py", "tools/apps_game.py", "tools/inspect_apps.py",
    "tools/audit.py", "tools/audit_apps.py", "tools/run_task.py", "tools/broker.py", "tools/broker_apps.py",
    "tools/play_apps.py", "tools/unseen.py", "tools/bench_robot.py", "tools/bench5.py",
    "tools/analyze5.py", "tools/power5.py", "tools/run_experiment5.py", "tools/freeze5.py",
    "tools/test_apps_harness.py",
]


def hashes():
    return {f: hashlib.sha256((ROOT / f).read_bytes()).hexdigest() for f in FILES}


def manifest_hash():
    return hashlib.sha256(MANIFEST.read_bytes()).hexdigest()


def check():
    """Frozen files that changed (or the whole manifest, if there is none)."""
    if not MANIFEST.exists():
        return ["(no manifest: run tools/freeze5.py first)"]
    frozen = json.loads(MANIFEST.read_text())["files"]
    now = hashes()
    return sorted(f for f in set(frozen) | set(now) if frozen.get(f) != now.get(f))


def main():
    if "--check" in sys.argv:
        changed = check()
        print("unchanged" if not changed else "CHANGED: " + ", ".join(changed))
        return 1 if changed else 0
    if MANIFEST.exists():
        sys.exit("refusing: the manifest exists already")
    DIR.mkdir(parents=True, exist_ok=True)
    MANIFEST.write_text(json.dumps({"frozen": time.strftime("%Y-%m-%d %H:%M:%S"), "files": hashes()}, indent=2))
    shutil.copy(MANIFEST, ROOT / "docs" / f"manifest-exp5-{time.strftime('%Y%m%d-%H%M%S')}.json")
    print(f"frozen {len(FILES)} files; manifest hash {manifest_hash()}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
