"""Experiment 5 analysis with deviation 1 (docs/deviations5.md): run the frozen tools/analyze5.py
unchanged, except that a `conditions` task argument which Inspect stored as a list is compared
as the comma-joined text the check expects.

Inspect's CLI turned `-T conditions=plain,reference,manual` into the list
['plain', 'reference', 'manual'] in the log, and the frozen check compares str(value) with the
text "plain,reference,manual", so it refused a run that used exactly the registered conditions.
Only that one value is changed, and only when it is exactly the registered list
['plain', 'reference', 'manual'] (analyze5.CONDITIONS); every other check, the
statistics and the report are the frozen code's.

  python tools/analyze5_d1.py
"""

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import analyze5  # noqa: E402

_latest_logs = analyze5.latest_logs


def latest_logs(run):
    logs = _latest_logs(run)
    for log in logs.values():
        args = log.eval.task_args
        c = args.get("conditions")
        if isinstance(c, list) and c == list(analyze5.CONDITIONS):
            args["conditions"] = ",".join(c)  # only the exact registered list; anything else still fails
    return logs


analyze5.latest_logs = latest_logs

if __name__ == "__main__":
    print("Deviation 1 applied (docs/deviations5.md): Inspect's list form of `conditions` is "
          "compared as comma-joined text.\n")
    sys.exit(analyze5.main())
