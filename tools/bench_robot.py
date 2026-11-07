"""A fake player for experiment 5's harness (tools/apps_game.py): types a task's known solution
through play.py, one `line` action per line, exactly as Haiku or Luna would. Prints JSON lines
the harness reads like a player log.

  python bench_robot.py TASK_ID   (stdin: the normal player prompt)
"""

import json
import re
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from apps_game import TASKS  # noqa: E402


def main():
    prompt = sys.stdin.read()
    gid = re.search(r"python play\.py (\S+) look", prompt).group(1)
    for text in TASKS[sys.argv[1]]["solution"]:
        print(json.dumps({"type": "robot_command", "command": f'python play.py {gid} line "{text}"'}), flush=True)
        subprocess.run([sys.executable, "play.py", gid, "line", text], capture_output=True, check=True)
    print(json.dumps({"type": "robot_result", "text": "typed the known solution"}), flush=True)


if __name__ == "__main__":
    main()
