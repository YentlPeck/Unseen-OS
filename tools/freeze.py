"""Freeze (or check) the files that decide the experiment's result.

  python tools/freeze.py           write out/experiment/manifest.json (refuses if it exists)
  python tools/freeze.py --check   compare the files with the manifest

The manifest's own hash is recorded in every experiment log (task arg
`manifest`), and analyze.py refuses to run if any frozen file has changed.
"""

import hashlib
import json
import shutil
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MANIFEST = ROOT / "out" / "experiment" / "manifest.json"
FILES = ["out/unseen.bin", "PREREG.md", "docs/manual.md", "tools/tasks.py", "tools/expected.json",
         "tools/validate_tasks.py", "tools/run_task.py", "tools/play.py", "tools/broker.py", "tools/unseen.py",
         "tools/inspect_eval.py", "tools/robot_player.py", "tools/run_experiment.py", "tools/analyze.py", "tools/freeze.py"]


def hashes():
    return {f: hashlib.sha256((ROOT / f).read_bytes()).hexdigest() for f in FILES}


def manifest_hash():
    return hashlib.sha256(MANIFEST.read_bytes()).hexdigest()


def check():
    """List of frozen files that changed (empty = all good). Raises if not frozen yet."""
    frozen = json.loads(MANIFEST.read_text())["files"]
    return [f for f, h in hashes().items() if frozen.get(f) != h] + [f for f in frozen if f not in FILES]


def main():
    if "--check" in sys.argv:
        changed = check()
        print("all frozen files unchanged" if not changed else f"CHANGED: {changed}")
        return 1 if changed else 0
    if MANIFEST.exists():
        print(f"refusing: {MANIFEST} already exists (the experiment is frozen)")
        return 1
    MANIFEST.parent.mkdir(parents=True, exist_ok=True)
    MANIFEST.write_text(json.dumps({"frozen_at": time.strftime("%Y-%m-%d %H:%M:%S %z"), "files": hashes()}, indent=2))
    shutil.copy(MANIFEST, ROOT / "docs" / f"manifest-{time.strftime('%Y%m%d-%H%M%S')}.json")
    print(f"wrote {MANIFEST}\nmanifest sha256: {manifest_hash()}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
