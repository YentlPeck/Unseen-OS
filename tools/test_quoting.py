"""Do chore lines survive the players' real shells exactly, and does the audit accept them?

  python tools/test_quoting.py

Haiku runs commands through Git Bash, Luna through Windows PowerShell 5.1. Each
sample chore line is sent the way the prompt's tip says (double quotes around
the whole text, single quotes inside it) to a stand-in play.py that prints the
arguments it received. They must arrive unchanged, and audit.py must accept
the command. A known trap (single quotes outside, double quotes inside, which
PowerShell 5.1 breaks) is shown for the record.
"""

import json
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from audit import audit  # noqa: E402

LINES = [
    "say 'Omar sent ' & total & ' mails'",
    "when field(m, 'from') is 'omar'",
    "write('desk:answer', total)",
    "set t to 'Street party on Saturday at 15:00. Hope you can come!'",
    "when x <= 3 and y >= 2 or not done_yet",
    "-- a comment, with (brackets) & signs!",
    "send(w, 'Reminder', t & ' at ' & field(e, 'time') & '!')",
    "say (1 + 2) * 3 / 4 - -1",
    "when not (words(written) has field(m, 'from'))",
    "set who to 'contacts:' & field(m, 'from')",
    "    event(item(w, 3), item(w, 5), 'Meeting', field(m, 'from'))",
    "say 'a;b|c>d<e'",
    "say 'Thanks, I will pay ' & amount & ' euro.'",
]
STUB = "import json, sys\nprint(json.dumps(sys.argv[3:]))\n"
BASH = shutil.which("bash") or r"C:\Program Files\Git\bin\bash.exe"


def run(shell, command, cwd):
    if shell == "bash":
        args = [BASH, "-c", command]
    else:
        args = ["powershell", "-NoProfile", "-NonInteractive", "-Command", command]
    r = subprocess.run(args, cwd=cwd, capture_output=True, text=True, timeout=60)
    try:
        return json.loads(r.stdout.strip().splitlines()[-1])
    except (IndexError, json.JSONDecodeError):
        return ["<no output>", r.stderr.strip()[:200]]


def main():
    failures = 0
    with tempfile.TemporaryDirectory() as tmp:
        (Path(tmp) / "play.py").write_text(STUB)
        for line in LINES:
            command = f'python play.py g1 type "{line}"'
            for shell in ("bash", "powershell"):
                got = run(shell, command, tmp)
                ok = got == [line]
                audit_ok = not audit([command], "g1")
                if not (ok and audit_ok):
                    failures += 1
                    print(f"FAIL {shell:10} {line!r} -> {got} audit_ok={audit_ok}")
        print(f"{len(LINES)} lines x 2 shells: {'all arrived unchanged and passed the audit' if not failures else f'{failures} failed'}")
        trap = 'python play.py g1 type \'say "hello"\''
        print("known trap, single outside / double inside:",
              {s: run(s, trap, tmp) for s in ("bash", "powershell")})
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
