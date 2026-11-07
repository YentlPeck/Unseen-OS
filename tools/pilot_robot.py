"""A fake player for pilot M7 (tools/pilot5.py): types a task's known solution through
play.py, line by line (type, then Enter), exactly as Haiku or Luna would. Prints JSON lines
the harness reads like a player log.

  python pilot_robot.py TASK_ID   (stdin: the normal player prompt)
"""

import json
import re
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from tasks_apps import TASKS  # noqa: E402


def main():
    prompt = sys.stdin.read()
    gid = re.search(r"python play\.py (\S+) look", prompt).group(1)
    for text in TASKS[sys.argv[1]]["solution"]:
        for args in (["type", text], ["press", "enter"]):
            shown = f'"{args[1]}"' if args[0] == "type" else args[1]
            print(json.dumps({"type": "robot_command", "command": f"python play.py {gid} {args[0]} {shown}"}), flush=True)
            subprocess.run([sys.executable, "play.py", gid, *args], capture_output=True, check=True)
    print(json.dumps({"type": "robot_result", "text": "typed the known solution"}), flush=True)


if __name__ == "__main__":
    main()
