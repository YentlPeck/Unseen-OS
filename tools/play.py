"""The only tool the AI player gets. Usage (run inside the game folder):

  python play.py GAME look              see the screen
  python play.py GAME type "some text"  type text (does NOT press Enter)
  python play.py GAME press KEY [KEY]   press keys: enter, esc, backspace, tab, up,
                                        down, left, right, home, end, delete,
                                        space, or ctrl+<letter> (e.g. ctrl+k)

Every action prints the screen afterwards, and where the cursor is.
Actions are limited per game.
"""

import json
import socket
import sys
from pathlib import Path


def main():
    cfg = json.loads((Path(__file__).resolve().parent / "game.json").read_text())
    args = sys.argv[1:]
    if len(args) < 2 or args[0] != cfg["game"]:
        print(__doc__)
        return 2
    request = {"token": cfg["token"], "action": args[1], "args": args[2:]}
    with socket.create_connection(("127.0.0.1", cfg["port"]), timeout=60) as s:
        s.sendall(json.dumps(request).encode() + b"\n")
        reply = json.loads(s.makefile("r").readline())
    if not reply["ok"]:
        print(f"Error: {reply['error']}")
        return 1
    print(f"--- screen after action {reply['n']}/{reply['max']} ---")
    print(reply["screen"])
    print(f"--- end of screen (cursor: {reply['cursor']}) ---")
    return 0


if __name__ == "__main__":
    sys.exit(main())
