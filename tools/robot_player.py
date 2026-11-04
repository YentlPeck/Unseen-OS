"""A fake AI player for testing the harness at zero cost.

Run by the harness like a real player (prompt on stdin, inside the game folder).
It plays the task's reference solution by calling `python play.py GAME ...`
exactly as Haiku or Luna would, so the referee, clean-up, audit and scoring
all run for real. Prints JSON lines the harness reads like a player log.

  python robot_player.py TASK_ID   (stdin: the normal player prompt)
"""

import json
import re
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from tasks import TASKS  # noqa: E402


def steps(solution):
    """Reference-solution key string -> list of play.py argument lists."""
    for part in re.split(r"(\{[^}]*\})", solution):
        if part.startswith("{"):
            name, _, times = part[1:-1].partition("*")
            name = {"bksp": "backspace", "del": "delete"}.get(name, name)
            yield ["press", *[name] * int(times or 1)]
        elif part:
            yield ["type", part]


def main():
    prompt = sys.stdin.read()
    gid = re.search(r"python play\.py (\S+) look", prompt).group(1)
    for args in steps(TASKS[sys.argv[1]]["solution"]):
        shown = " ".join(f'"{a}"' if args[0] == "type" else a for a in args[1:])
        print(json.dumps({"type": "robot_command", "command": f"python play.py {gid} {args[0]} {shown}"}), flush=True)
        subprocess.run([sys.executable, "play.py", gid, *args], capture_output=True, check=True)
    print(json.dumps({"type": "robot_result", "text": "played the reference solution"}), flush=True)


if __name__ == "__main__":
    main()
