"""The referee for app tasks (pilot M7 and later app experiments).

The same as broker.py (which stays as it was for experiments 1-4), with one more action:

  line <text>   type the text, then press Enter: one action (an app is typed line by line,
                and two actions per line invited players to batch commands; docs/pilot5.md)
"""

import time

from broker import Broker
from unseen import SettleError, keys_for_names, keys_for_text

USE = "use: look | type <text> | line <text> | press <key> [key ...]"


class AppBroker(Broker):
    def handle(self, req):
        if not isinstance(req, dict) or req.get("token") != self.token:
            return {"ok": False, "error": "wrong token"}
        action, args = req.get("action"), req.get("args") or []
        if not isinstance(args, list) or not all(isinstance(a, str) for a in args):
            return {"ok": False, "error": USE}
        try:  # check the request before it costs an action
            if action == "look":
                combos = []
            elif action == "type" and args:
                combos = keys_for_text(" ".join(args))
            elif action == "line":  # no text: just Enter (an empty line)
                combos = (keys_for_text(" ".join(args)) if args else []) + keys_for_names(["enter"])
            elif action == "press" and args:
                combos = keys_for_names(args)
            else:
                return {"ok": False, "error": USE}
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
                # as broker.py: a hang of the OS itself is a loss; anything else is our side's fault
                if self.machine.os_crashed():
                    self.os_crash = f"after action {n}: {e!r}"
                    self.open = False
                    self._log({"n": n, "os_crash": repr(e)})
                    return {"ok": False, "error": "The computer stopped responding. The game is over."}
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
