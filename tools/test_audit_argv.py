"""Run every PowerShell command the audit allows through the real powershell.exe, with a
play.py that only prints the arguments it received. Passes if each command ran nothing but
play.py (every output line is play.py's report, no errors) and every call got the game id.

  python tools/test_audit_argv.py
"""

import json
import subprocess
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from audit import ALLOWED, audit, unwrap_shell  # noqa: E402

ECHO = 'import json, sys\nprint("ARGV " + json.dumps(sys.argv[1:]))\n'


def main():
    work = Path(tempfile.mkdtemp(prefix="argv-"))
    (work / "play.py").write_text(ECHO, encoding="utf-8")
    (work / "game.json").write_text("SECRET", encoding="utf-8")
    bad = 0
    for cmd in ALLOWED:
        if unwrap_shell(cmd)[1] != "powershell":
            continue
        assert not audit([cmd], "g1"), cmd
        # Run exactly what the audit reads: the unwrapped inside, given to PowerShell as -Command
        # (Codex shows some commands in a PowerShell-quoted form; this is what they run).
        inner = unwrap_shell(cmd)[0]
        p = subprocess.run(["powershell.exe", "-NoProfile", "-Command", inner], cwd=work,
                           capture_output=True, text=True)
        lines = [l for l in p.stdout.splitlines() if l.strip()]
        argvs = [json.loads(l[5:]) for l in lines if l.startswith("ARGV ")]
        calls = inner.count("python play.py")
        ok = (p.returncode == 0 and not p.stderr.strip() and len(argvs) == len(lines) == calls
              and "SECRET" not in p.stdout)
        bad += not ok
        print("ok  " if ok else "BAD ", cmd[len('"C:\\WINDOWS\\System32\\WindowsPowerShell\\v1.0\\powershell.exe" '):][:90])
        for a in argvs:
            print("       argv:", a)
        if not ok:
            print("       stderr:", p.stderr.strip()[:300])
    print("ALL GOOD" if not bad else f"{bad} PROBLEMS")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
