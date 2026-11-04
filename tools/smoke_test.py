"""Boot Unseen OS headless in QEMU, type some commands, save a screenshot.

Usage: python tools/smoke_test.py [--shot NAME] [step ...]
Each step is typed and followed by Enter. A step starting with "~" is typed
without Enter. Special keys: {enter} {esc} {up} {down} {left} {right}
{bksp} {ctrl-k} (any {ctrl-x}).
This is also the first piece of the future AI bridge: it presses keys the
way a person would (through the virtual keyboard) and looks at the screen.
"""

import json
import socket
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
QEMU = r"C:\Program Files\qemu\qemu-system-x86_64.exe"
PORT = 4444

# US keyboard layout: every printable ASCII character -> QEMU key code(s).
_PLAIN = {" ": "spc", "\n": "ret", "-": "minus", "=": "equal", "[": "bracket_left",
          "]": "bracket_right", "\\": "backslash", ";": "semicolon", "'": "apostrophe",
          "`": "grave_accent", ",": "comma", ".": "dot", "/": "slash"}
_SHIFTED = dict(zip('!@#$%^&*()_+{}|:"~<>?', ["1", "2", "3", "4", "5", "6", "7", "8", "9", "0",
                "minus", "equal", "bracket_left", "bracket_right", "backslash", "semicolon",
                "apostrophe", "grave_accent", "comma", "dot", "slash"]))
KEYNAMES = {**_PLAIN, **{c: ("shift", k) for c, k in _SHIFTED.items()}}
KEYNAMES.update({c: c for c in "abcdefghijklmnopqrstuvwxyz0123456789"})
KEYNAMES.update({c: ("shift", c.lower()) for c in "ABCDEFGHIJKLMNOPQRSTUVWXYZ"})
SPECIAL = {"enter": "ret", "esc": "esc", "up": "up", "down": "down", "left": "left",
           "right": "right", "bksp": "backspace", "home": "home", "end": "end", "del": "delete"}


class Qmp:
    def __init__(self, port):
        for _ in range(100):
            try:
                self.sock = socket.create_connection(("127.0.0.1", port))
                break
            except OSError:
                time.sleep(0.1)
        else:
            raise RuntimeError("could not connect to QEMU")
        self.file = self.sock.makefile("r")
        self.file.readline()  # greeting
        self.cmd("qmp_capabilities")

    def cmd(self, name, **args):
        self.sock.sendall(json.dumps({"execute": name, "arguments": args}).encode() + b"\n")
        while True:
            reply = json.loads(self.file.readline())
            if "return" in reply or "error" in reply:
                return reply

    def key(self, ch):
        if ch not in KEYNAMES:
            raise ValueError(f"no key for character {ch!r}")
        name = KEYNAMES[ch]
        self.press(*(name if isinstance(name, tuple) else (name,)))

    def press(self, *qcodes):
        reply = self.cmd("send-key", keys=[{"type": "qcode", "data": k} for k in qcodes])
        if "error" in reply:
            raise RuntimeError(f"QEMU refused keys {qcodes}: {reply['error']}")
        time.sleep(0.03)

    def type(self, text):
        i = 0
        while i < len(text):
            end = text.find("}", i)
            name = text[i + 1:end] if text[i] == "{" and end > i else ""
            if name in SPECIAL or name.startswith("ctrl-"):
                i = end + 1
                if name.startswith("ctrl-"):
                    self.press("ctrl", name[5:])
                else:
                    self.press(SPECIAL[name])
                continue
            self.key(text[i])
            i += 1


def main():
    args = sys.argv[1:]
    name = "screen"
    if args[:1] == ["--shot"]:
        name, args = args[1], args[2:]
    commands = args or ["guide", "about", "clock", "show Hello from a robot!", "ls"]
    serial = ROOT / "out" / "serial.log"
    shot = ROOT / "out" / f"{name}.png"
    qemu = subprocess.Popen([
        QEMU, "-kernel", str(ROOT / "out" / "unseen.bin"), "-m", "128M",
        "-rtc", "base=localtime", "-display", "none",
        "-serial", f"file:{serial}", "-qmp", f"tcp:127.0.0.1:{PORT},server,wait=off",
    ])
    try:
        qmp = Qmp(PORT)
        for _ in range(50):
            if serial.exists() and "ready" in serial.read_text(errors="replace"):
                break
            time.sleep(0.1)
        else:
            raise RuntimeError("OS did not report 'ready' on the serial port")
        for c in commands:
            qmp.type(c[1:] if c.startswith("~") else c + "{enter}")
            time.sleep(0.2)
        print(qmp.cmd("screendump", filename=str(shot), format="png"))
        print(serial.read_text(errors="replace"))
    finally:
        qemu.kill()


if __name__ == "__main__":
    main()
