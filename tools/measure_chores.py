"""How many actions does writing, running and fixing a chore really cost?

  python tools/measure_chores.py

Plays reference workflows in the real OS the way a player must (every
`type` and every `press` through play.py is one action) and checks each one
works. The numbers set the action budget for experiment 3 (docs/v3-design.md).
"""

import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from unseen import Machine  # noqa: E402

WORLD = """#today 2026-11-12
#page contacts:omar
name: Omar Haddad
mail: omar
note: works at the bakery
#end
#page contacts:ana
name: Ana Costa
mail: ana
note: neighbour, has a cat
#end
#page contacts:jo
name: Jo Smith
mail: jo
note: neighbour, plumber
#end
#page mail:inbox:m1
from: jo
to: me
subject: Invoice

amount: 120
#end
#page mail:inbox:m2
from: omar
to: me
subject: Bread

Fresh bread.
#end
#page mail:inbox:m3
from: jo
to: me
subject: Invoice

amount: 43.5
#end
"""

# Reference chores of the sizes the tasks will need.
SUM = """-- add up Jo's invoices
set total to 0
for each m in pages('mail:inbox')
  when field(m, 'from') is 'jo' and field(m, 'subject') is 'Invoice'
    set total to total + number(item(words(body(m)), 2))
  done
done
say total
write('desk:answer', total)"""

INVITE = """for each c in pages('contacts')
  when field(c, 'note') has 'neighbour'
    send(field(c, 'mail'), 'Party', 'Party at my place on Saturday!')
  done
done"""

WEEKLY = """set d to '2026-12-07'
repeat 4 times
  event(d, '10:00', 'Team call', '')
  set d to add_days(d, 7)
done"""


class Player:
    """Plays actions exactly as play.py would and counts them."""

    def __init__(self, m):
        self.m, self.n = m, 0

    def type(self, text):
        self.n += 1
        self.m.type(text)
        return self.m.settle()

    def press(self, *keys):
        self.n += 1
        self.m.press(*keys)
        return self.m.settle()

    def command(self, text):
        self.type(text)
        return self.press("enter")


def write_chore(p, page, source):
    """Open Notes, type the chore line by line, keep it, leave."""
    p.command(f"notes {page}")
    lines = source.split("\n")
    for i, line in enumerate(lines):
        if line.strip():
            p.type(line)
        if i < len(lines) - 1:
            p.press("enter")
    p.press("ctrl+k", "esc")


def run_chore(p, page):
    s = p.command(f"do {page}")
    p.press("esc")
    return s


def main():
    results = []
    with tempfile.TemporaryDirectory() as tmp:
        for name, source in [("sum invoices (9 lines)", SUM), ("bulk invite (5 lines)", INVITE),
                             ("weekly events (5 lines)", WEEKLY)]:
            m = Machine.start(WORLD, workdir=tmp)
            try:
                p = Player(m)
                write_chore(p, "desk:job", source)
                written = p.n
                s = run_chore(p, "desk:job")
                ok = "Finished:" in s
                results.append((name, written, p.n, ok))
            finally:
                m.stop()

        # One repair cycle: a typo on line 5 of SUM, run, fix it in Notes, run again.
        m = Machine.start(WORLD, workdir=tmp)
        try:
            p = Player(m)
            write_chore(p, "desk:job", SUM.replace("set total to total +", "set total to totl +"))
            first = run_chore(p, "desk:job")
            before = p.n
            p.command("notes desk:job")
            broken = SUM.replace("set total to total +", "set total to totl +").splitlines()[4]
            p.press(*["down"] * 4, "end", *["backspace"] * len(broken))  # go to line 5, clear it
            p.type("    set total to total + number(item(words(body(m)), 2))")
            p.press("ctrl+k", "esc")
            s = run_chore(p, "desk:job")
            ok = "line 5:" in first and "Finished:" in s and m.dump().get("desk:answer") == "163.5"
            results.append(("one repair cycle (fix line 5, run again)", p.n - before, p.n - before, ok))
        finally:
            m.stop()

    print(f"{'workflow':45} {'write':>6} {'total':>6}  works")
    for name, written, total, ok in results:
        print(f"{name:45} {written:6} {total:6}  {'yes' if ok else 'NO'}")
    return 0 if all(r[3] for r in results) else 1


if __name__ == "__main__":
    sys.exit(main())
