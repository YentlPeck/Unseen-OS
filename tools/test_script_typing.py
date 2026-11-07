"""Typing round trip for Unseen Script (design §10): every line of every example in
docs/script-reference.md is typed the way a player types it, through bash and through the real
powershell.exe, with a play.py that only prints the arguments it received. Passes if the audit
allows every command and play.py received exactly the line, for both shells.

  python tools/test_script_typing.py
"""

import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

TOOLS = Path(__file__).resolve().parent
sys.path.insert(0, str(TOOLS))
from audit import audit  # noqa: E402

PS = '"C:\\WINDOWS\\System32\\WindowsPowerShell\\v1.0\\powershell.exe" -Command '
# Git Bash (the shell the Claude player uses on Windows), not WSL's bash.
BASH = next((b for b in (os.environ.get("UNSEEN_BASH"), r"C:\Program Files\Git\bin\bash.exe", shutil.which("bash"))
             if b and Path(b).exists()), "bash")
ECHO = 'import json, sys\nprint("ARGV " + json.dumps(sys.argv[1:]))\n'


def example_lines():
    md = (TOOLS.parent / "docs" / "script-reference.md").read_text(encoding="utf-8")
    blocks = re.findall(r"```(?:script|tests|commands|page [^\n]*)\n(.*?)\n```", md, re.S)
    lines = [[l for l in b.split("\n") if l.strip()] for b in blocks]
    # in a lab block only the lines after the prompt are typed (the others are what it shows)
    for b in re.findall(r"```lab\n(.*?)\n```", md, re.S):
        blocks.append(b)
        lines.append([l[4:] for l in b.split("\n") if l.startswith((">   ", "... ")) and l[4:].strip()])
    return blocks, lines


def bash_quote(line):
    """Inside bash double quotes, \\ " $ and ` need a backslash."""
    return '"' + re.sub(r'([\\"$`])', r"\\\1", line) + '"'


def ps_quote(line):
    """Inside PowerShell double quotes, ` and $ need a backtick, and " is written twice."""
    return '"' + line.replace("`", "``").replace("$", "`$").replace('"', '""') + '"'


def run_shell(shell, inner, work):
    if shell == "bash":
        p = subprocess.run([BASH, "-c", inner], cwd=work, capture_output=True, text=True)
    else:
        p = subprocess.run(["powershell.exe", "-NoProfile", "-Command", inner], cwd=work,
                           capture_output=True, text=True)
    argvs = [json.loads(l[5:]) for l in p.stdout.splitlines() if l.startswith("ARGV ")]
    return p, argvs


def main():
    work = Path(tempfile.mkdtemp(prefix="typing-"))
    (work / "play.py").write_text(ECHO, encoding="utf-8")
    _, examples = example_lines()
    bad = total = 0
    for lines in examples:
        for shell in ("bash", "powershell"):
            q = bash_quote if shell == "bash" else ps_quote
            calls = [f"python play.py g1 type {q(l)}" for l in lines]
            for c in calls:
                shown = c if shell == "bash" else PS + "'" + c.replace("'", "''") + "'"
                if audit([shown], "g1"):
                    bad += 1
                    print(f"AUDIT REFUSES ({shell}): {c}")
            p, argvs = run_shell(shell, "; ".join(calls), work)
            got = [a[2] if len(a) == 3 and a[:2] == ["g1", "type"] else repr(a) for a in argvs]
            total += len(lines)
            if p.returncode != 0 or p.stderr.strip() or got != lines:
                bad += 1
                print(f"BAD ({shell}) stderr={p.stderr.strip()[:200]!r}")
                for want, have in zip(lines, got + [None] * len(lines)):
                    if want != have:
                        print(f"   want {want!r}\n   got  {have!r}")
    print(f"{total} lines typed through each shell")
    print("ALL GOOD" if not bad else f"{bad} PROBLEMS")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
