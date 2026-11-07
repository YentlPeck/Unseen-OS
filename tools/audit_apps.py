r"""The audit of app games (experiment 5): the one-command rule of the app prompt.

Every command must be exactly ONE call of the game tool for this game, written out:
  python play.py <this game> <args>
optionally with its output silenced. It must first pass the audit of experiments 1-4
(tools/audit.py, unchanged: literal arguments only); on top of that, no chains (; && || or new
lines), no loops, no aliases, no cd or echo, and the game name must be this game's.
(Pilot round 2, docs/pilot5.md: the prompt says so, and Claude's permission rule refuses chains
anyway, so Codex must be held to the same rule.)

  python tools/audit_apps.py      runs the acceptance / rejection cases
"""

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from audit import audit, split_chain, text, unwrap_shell, words  # noqa: E402


def one_call(command, gid):
    """Is the command exactly one literal call of the game tool for game `gid`?"""
    inner, shell = unwrap_shell(command)
    parts = list(split_chain(inner, shell))
    if len(parts) != 1:
        return False
    w = [text(x) for x in (words(parts[0], shell) or [])]
    return len(w) >= 3 and w[0] == "python" and w[1] == "play.py" and w[2] == gid


def audit_apps(commands, gid):
    """Commands that broke the rules: the frozen audit's, then the one-command rule."""
    bad = audit(commands, gid)
    for c in commands:
        if c in bad:
            continue
        try:
            ok = one_call(c, gid)
        except Exception:
            ok = False
        if not ok:
            bad.append(c if isinstance(c, str) else repr(c))
    return bad


PS = r'"C:\WINDOWS\System32\WindowsPowerShell\v1.0\powershell.exe" -Command '
ALLOWED = [
    'python play.py g1 look',
    'python play.py g1 type "peek letters"',
    'python play.py g1 line "insert desk:apps:x 1 app \'X\'"',
    'python play.py g1 line',
    'python play.py g1 press ctrl+k',
    'python play.py g1 look >/dev/null',
    PS + "'python play.py g1 line \"set s to 1\"'",
    PS + '"python play.py g1 type \\"for each c in pages(\'contacts\')\\""',
]
REJECTED = [
    'python play.py g1 look; python play.py g1 press enter',
    'python play.py g1 type "x" && python play.py g1 press enter',
    'cd . ; python play.py g1 look',
    'echo hi && python play.py g1 look',
    'for i in 1 2 3; do python play.py g1 press down; done',
    'P="python play.py g1"; $P look',
    'P=g1; python play.py $P look',
    'python play.py g2 look',
    'python play.py look',
    'python play.py g1 look\npython play.py g1 press enter',
    PS + "'python play.py g1 type \"guide\"; python play.py g1 press enter'",
    'python play.py g1 look; cat game.json',
]


def main():
    fails = [c for c in ALLOWED if audit_apps([c], "g1")] + [c for c in REJECTED if not audit_apps([c], "g1")]
    for c in fails:
        print("WRONG:", c)
    print("ALL GOOD" if not fails else f"{len(fails)} wrong")
    return 1 if fails else 0


if __name__ == "__main__":
    sys.exit(main())
