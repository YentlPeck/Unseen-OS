"""Bridge between the harness and Unseen OS running in QEMU.

QEMU listens on two local ports, and a Machine keeps BOTH connections open for
the whole game, so no other program can connect to them:
  * QMP (QEMU's control channel): key presses through the virtual keyboard.
  * Serial: the OS's hidden inspector (PING / KEYS / SCREEN / DUMP). Every
    reply is "<NAME> <length>" plus exactly that many bytes.

Settling is exact: the harness counts the key presses it sent, the OS counts
the key presses it handled, and we wait until the two match.
"""

import json
import re
import secrets
import socket
import subprocess
import threading
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
QEMU = r"C:\Program Files\qemu\qemu-system-x86_64.exe"
IMAGE = ROOT / "out" / "unseen.bin"

# US keyboard: every typeable character -> QEMU key code(s). (No newline or tab:
# `type` only types; Enter and Tab are separate key presses.)
_PLAIN = {" ": "spc", "-": "minus", "=": "equal", "[": "bracket_left", "]": "bracket_right",
          "\\": "backslash", ";": "semicolon", "'": "apostrophe", "`": "grave_accent",
          ",": "comma", ".": "dot", "/": "slash"}
_SHIFTED = dict(zip('!@#$%^&*()_+{}|:"~<>?', ["1", "2", "3", "4", "5", "6", "7", "8", "9", "0",
                "minus", "equal", "bracket_left", "bracket_right", "backslash", "semicolon",
                "apostrophe", "grave_accent", "comma", "dot", "slash"]))
CHARS = {**{c: (k,) for c, k in _PLAIN.items()}, **{c: ("shift", k) for c, k in _SHIFTED.items()}}
CHARS.update({c: (c,) for c in "abcdefghijklmnopqrstuvwxyz0123456789"})
CHARS.update({c: ("shift", c.lower()) for c in "ABCDEFGHIJKLMNOPQRSTUVWXYZ"})

KEYS = {"enter": "ret", "esc": "esc", "escape": "esc", "backspace": "backspace", "tab": "tab",
        "up": "up", "down": "down", "left": "left", "right": "right", "home": "home",
        "end": "end", "delete": "delete", "space": "spc"}


class SettleError(RuntimeError):
    pass


def free_port():
    with socket.socket() as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


def _connect(port, proc, tries=100):
    for _ in range(tries):
        if proc.poll() is not None:
            raise RuntimeError("QEMU exited during start-up")
        try:
            return socket.create_connection(("127.0.0.1", port), timeout=10)
        except OSError:
            time.sleep(0.05)
    raise RuntimeError(f"could not connect to QEMU on port {port}")


def keys_for_text(text):
    """Key combos for typing text. Raises ValueError for untypeable text."""
    if "\n" in text or "\r" in text:
        raise ValueError("type can't contain new lines; use press enter instead")
    missing = sorted({c for c in text if c not in CHARS})
    if missing:
        raise ValueError(f"these characters can't be typed: {''.join(missing)!r}")
    return [CHARS[c] for c in text]


def keys_for_names(names):
    combos = []
    for name in names:
        n = name.lower()
        if re.fullmatch(r"ctrl\+[a-z]", n):
            combos.append(("ctrl", n[5]))
        elif n in KEYS:
            combos.append((KEYS[n],))
        else:
            raise ValueError(f"unknown key '{name}'")
    return combos


class Machine:
    """One running copy of Unseen OS, with private connections to it."""

    @classmethod
    def start(cls, start_text=None, workdir=None, log=None):
        """Boot the OS. With start_text, the desk is built from it; without, the sample desk."""
        session = secrets.token_hex(8)
        module = None
        if start_text is not None:
            module = Path(workdir) / "start.txt"
            module.write_text(f"#session {session}\n{start_text}", newline="\n", encoding="utf-8")
        last_error = None
        for _ in range(5):  # retry with fresh ports if start-up fails
            m = cls()
            try:
                m._boot(module, session if module else "none", log)
                return m
            except (RuntimeError, OSError) as e:
                last_error = e
                m.stop()
        raise RuntimeError(f"Unseen OS did not start: {last_error}")

    def _boot(self, module, session, log):
        self.lock = threading.RLock()
        self.sent = 0
        self._buf = b""
        qmp, ser = free_port(), free_port()
        cmd = [QEMU, "-kernel", str(IMAGE), "-m", "128M", "-rtc", "base=localtime",
               "-display", "none", "-qmp", f"tcp:127.0.0.1:{qmp},server,wait=off",
               "-serial", f"tcp:127.0.0.1:{ser},server,wait=off"]
        if module:
            cmd += ["-initrd", str(module)]
        flags = getattr(subprocess, "CREATE_NO_WINDOW", 0)
        self.proc = subprocess.Popen(cmd, stdout=log or subprocess.DEVNULL, stderr=subprocess.STDOUT,
                                     creationflags=flags)
        self.qmp = _connect(qmp, self.proc)
        self.qmp_file = self.qmp.makefile("r")
        self.qmp_file.readline()  # greeting
        self._qmp_call("qmp_capabilities")
        self.serial = _connect(ser, self.proc)
        deadline = time.time() + 10
        while True:  # wait until the OS answers with OUR session code
            try:
                if self._ask("PING", timeout=0.5) == session:
                    return
                raise RuntimeError("wrong machine answered (session code mismatch)")
            except TimeoutError:
                if time.time() > deadline:
                    raise RuntimeError("OS did not answer PING")

    def stop(self):
        for s in ("qmp", "serial"):
            try:
                getattr(self, s).close()
            except Exception:
                pass
        proc = getattr(self, "proc", None)
        if proc and proc.poll() is None:
            proc.kill()
            proc.wait()

    # ---- keyboard ---------------------------------------------------------

    def _qmp_call(self, name, **args):
        self.qmp.sendall(json.dumps({"execute": name, "arguments": args}).encode() + b"\n")
        while True:
            reply = json.loads(self.qmp_file.readline())
            if "return" in reply or "error" in reply:
                return reply

    def send_combos(self, combos):
        with self.lock:
            for keys in combos:
                reply = self._qmp_call("send-key", keys=[{"type": "qcode", "data": k} for k in keys],
                                       **{"hold-time": 20})
                if "error" in reply:
                    raise RuntimeError(f"QEMU refused keys {keys}: {reply['error']['desc']}")
                self.sent += 1  # every combo is exactly one key press for the OS
                time.sleep(0.015)

    def type(self, text):
        self.send_combos(keys_for_text(text))

    def press(self, *names):
        self.send_combos(keys_for_names(names))

    def run_keys(self, keys):
        """Type a key string like 'notes x{enter}{down*3}hi{ctrl+k}{esc}' (harness only)."""
        alias = {"bksp": "backspace", "del": "delete"}
        # Wait for the OS after every part, as a player's actions do: hundreds of keys sent
        # without a pause can overflow the keyboard queue while the OS redraws or runs a chore.
        for part in re.split(r"(\{[^}]*\})", keys):
            if part.startswith("{"):
                name, _, times = part[1:-1].partition("*")
                self.press(*[alias.get(name, name)] * int(times or 1))
                self.settle()
            elif part:
                self.type(part)
                self.settle()

    # ---- inspector --------------------------------------------------------

    def _ask(self, command, timeout=5.0):
        """Send an inspector command and return its payload (text)."""
        with self.lock:
            self.serial.sendall(command.encode() + b"\n")
            want = {"PING": "PONG"}.get(command, command)
            deadline = time.time() + timeout
            while True:
                # Lines before our reply (e.g. log lines) are skipped.
                if b"\n" in self._buf:
                    line, rest = self._buf.split(b"\n", 1)
                    head = line.decode(errors="replace").split(" ")
                    if len(head) == 2 and head[0] in (want, "ERROR") and head[1].isdigit():
                        n = int(head[1])
                        if len(rest) >= n:
                            self._buf = rest[n:]
                            payload = rest[:n].decode(errors="replace")
                            if head[0] == "ERROR":
                                raise RuntimeError(payload)
                            return payload
                    else:
                        self._buf = rest
                        continue
                left = deadline - time.time()
                if left <= 0:
                    raise TimeoutError(f"no answer to {command}")
                self.serial.settimeout(left)
                try:
                    chunk = self.serial.recv(65536)
                except socket.timeout:
                    raise TimeoutError(f"no answer to {command}")
                if not chunk:
                    raise RuntimeError("serial connection closed")
                self._buf += chunk.replace(b"\r", b"")  # pages and screen never contain \r

    def keys_handled(self):
        return int(self._ask("KEYS"))

    def settle(self, limit=15.0):
        """Wait until the OS has handled every key we sent, then return the screen."""
        deadline = time.time() + limit
        while self.keys_handled() < self.sent:
            if time.time() > deadline:
                raise SettleError(f"OS handled {self.keys_handled()} of {self.sent} keys")
            time.sleep(0.02)
        return self.look()

    def screenshot(self, path):
        """Save the screen as a PNG (for the write-up; players only get text)."""
        with self.lock:
            reply = self._qmp_call("screendump", filename=str(path), format="png")
        if "error" in reply:
            raise RuntimeError(reply["error"]["desc"])

    def cursor(self):
        """Where the cursor is: (row, column), counted from 1 like the screen rows."""
        row, col = self._ask("CURSOR").split()
        return int(row), int(col)

    def look(self):
        """The screen as 25 lines of text."""
        return self._ask("SCREEN").rstrip("\n")

    def dump(self):
        """Every drawer and page: {path: None for a drawer, or the page text}."""
        body, files, i = self._ask("DUMP"), {}, 0
        while i < len(body):
            nl = body.index("\n", i)
            header = body[i:nl]
            if header.startswith("D "):
                files[header[2:]] = None
                i = nl + 1
            else:
                _, path, n = header.split(" ")  # names never contain spaces
                n = int(n)
                files[path] = body[nl + 1: nl + 1 + n]
                i = nl + 1 + n + 1
        return files
