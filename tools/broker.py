"""The referee between the AI player and the OS.

It runs inside the harness. The player's `play.py` sends it one request per
action over a local socket, with a secret token. The referee:
  * is the only thing connected to the OS (keys + inspector),
  * handles one action at a time (a lock covers budget, keys, settling, logging),
  * enforces the action limit and the time limit, and stops accepting actions
    when the game ends,
  * records its own failures (QEMU / bridge) separately as harness errors, so
    they are never mistaken for the player's mistakes.
"""

import json
import secrets
import socket
import threading
import time
from pathlib import Path

from unseen import SettleError, keys_for_names, keys_for_text


class Broker:
    def __init__(self, machine, max_actions, transcript_path, deadline=None):
        self.machine = machine
        self.max_actions = max_actions
        self.transcript = Path(transcript_path)
        self.deadline = deadline
        self.token = secrets.token_hex(16)
        self.used = 0
        self.open = True
        self.settle_failures = 0
        self.infra_errors = []
        # Set when the OS stopped answering after a player's action while QEMU still ran
        # (a player-triggered crash or hang): the game ends and counts as a loss, no re-run.
        self.os_crash = None
        self.lock = threading.Lock()
        self.server = socket.socket()
        self.server.bind(("127.0.0.1", 0))
        self.server.listen()
        self.port = self.server.getsockname()[1]
        threading.Thread(target=self._serve, daemon=True).start()

    def _serve(self):
        while True:
            try:
                conn, _ = self.server.accept()
            except OSError:
                return  # closed
            threading.Thread(target=self._client, args=(conn,), daemon=True).start()

    def _client(self, conn):
        with conn:
            try:
                req = json.loads(conn.makefile("r").readline())
            except Exception as e:
                reply = {"ok": False, "error": f"bad request: {e}"}
            else:
                reply = self.handle(req)
            try:
                conn.sendall(json.dumps(reply).encode() + b"\n")
            except OSError:
                pass

    def _log(self, entry):
        with open(self.transcript, "a", encoding="utf-8") as log:
            log.write(json.dumps(entry) + "\n")

    def handle(self, req):
        if not isinstance(req, dict) or req.get("token") != self.token:
            return {"ok": False, "error": "wrong token"}
        action, args = req.get("action"), req.get("args") or []
        if not isinstance(args, list) or not all(isinstance(a, str) for a in args):
            return {"ok": False, "error": "use: look | type <text> | press <key> [key ...]"}
        try:  # check the request before it costs an action
            if action == "look":
                combos = []
            elif action == "type" and args:
                combos = keys_for_text(" ".join(args))
            elif action == "press" and args:
                combos = keys_for_names(args)
            else:
                return {"ok": False, "error": "use: look | type <text> | press <key> [key ...]"}
        except ValueError as e:
            return {"ok": False, "error": str(e)}

        with self.lock:
            if self.os_crash:
                return {"ok": False, "error": "The game is over (the computer stopped responding)."}
            if self.settle_failures or self.infra_errors:
                return {"ok": False, "error": "The game is over (harness error)."}
            if not self.open or (self.deadline and time.time() > self.deadline):
                return {"ok": False, "error": "The game is over (time is up)."}
            if self.used >= self.max_actions:
                return {"ok": False, "error": f"No actions left (limit {self.max_actions}). Stop and report what you did."}
            self.used += 1
            n = self.used
            try:
                self.machine.send_combos(combos)
                screen = self.machine.settle()
                row, col = self.machine.cursor()
                cursor = f"row {row}, column {col}"
            except Exception as e:
                # A hang or crash of the OS itself (QEMU still running, OS silent) follows the
                # player's own actions: a loss, not a re-run. Anything else is our side's fault.
                if self.machine.os_crashed():
                    self.os_crash = f"after action {n}: {e!r}"
                    self.open = False
                    self._log({"n": n, "os_crash": repr(e)})
                    return {"ok": False, "error": "The computer stopped responding. The game is over."}
                # Our side failed (keys not delivered, QMP / serial / QEMU trouble): the game ends
                # here, because the OS may now be in a state the player never asked for. The
                # attempt is a harness error (re-run), but its commands are still audited.
                self.open = False
                if isinstance(e, SettleError):
                    self.settle_failures += 1
                    self._log({"n": n, "settle_error": str(e)})
                else:
                    self.infra_errors.append(repr(e))
                    self._log({"n": n, "infra_error": repr(e)})
                return {"ok": False, "error": "The computer did not respond (harness error). The game is over."}
            self._log({"n": n, "action": action, "args": args, "screen": screen, "cursor": cursor})
            return {"ok": True, "n": n, "max": self.max_actions, "screen": screen, "cursor": cursor}

    def close(self):
        """No more actions after this (called before scoring)."""
        with self.lock:
            self.open = False
        try:
            self.server.close()
        except OSError:
            pass
