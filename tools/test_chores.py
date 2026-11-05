"""End-to-end checks of chores inside the real OS (QEMU), no AI involved.

  python tools/test_chores.py

Boots Unseen OS with a small world, runs chores with `do`, and checks the
Output screen, the files (DUMP), the frozen clock, `guide do`, the cursor
report, and that the worst-case chores neither crash nor hang the OS.
"""

import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from unseen import Machine  # noqa: E402

DEEP_EXPR = "say " + "(" * 12 + "1" + ")" * 12 + " & 1" + " + 1" * 44
WORST = "\n".join(["when yes"] * 16 + [DEEP_EXPR] + ["done"] * 16)

START = f"""#today 2026-11-12
#page contacts:omar
name: Omar Haddad
mail: omar
#end
#page contacts:lina
name: Lina Berg
mail: lina
note: piano teacher
#end
#page mail:inbox:m1
from: omar
to: me
subject: Bread

Fresh bread today.
#end
#page mail:inbox:m2
from: lina
to: me
subject: Lesson

Can we move it?
#end
#page mail:inbox:m10
from: omar
to: me
subject: Closed

Closed on Monday.
#end
#page count
-- count Omar's mails and save the answer
set total to 0
for each m in pages('mail:inbox')
  when field(m, 'from') is 'omar'
    set total to total + 1
  done
done
say 'Omar sent ' & total & ' mails'
write('desk:answer', total)
#end
#page broken
write('desk:never', 'x')
send('omar', 'Hi', 'hello')
say 'before'
say 1 + 'x'
#end
#page invite
for each c in pages('contacts')
  when field(c, 'note') has 'piano'
    send(field(c, 'mail'), 'Party', 'Come on ' & add_days(today(), 2) & '!')
    event(add_days(today(), 2), '19:00', 'Party', field(c, 'mail'))
  done
done
#end
#page worst
{WORST}
#end
#page spin
repeat 10000 times
  repeat 10000 times
  done
done
#end
#page stranger
send('nobody', 'Hi', 'x')
#end
"""

failures = []


def check(name, ok, detail=""):
    print(("PASS " if ok else "FAIL ") + name + ("" if ok else f"\n     {detail}"))
    if not ok:
        failures.append(name)


def run(m, keys):
    m.run_keys(keys)
    return m.settle()


def main():
    with tempfile.TemporaryDirectory() as tmp:
        m = Machine.start(START, workdir=tmp)
        try:
            s = run(m, "do count{enter}")
            files = m.dump()
            check("example: output", "Omar sent 2 mails" in s and "Finished:" in s, s)
            check("example: wrote answer", files.get("desk:answer") == "2", files.get("desk:answer"))
            check("example: changes listed", "wrote desk:answer" in s, s)
            check("cursor in viewer", m.cursor() == (3, 3), m.cursor())
            s = run(m, "{esc}")
            check("esc back to prompt", "desk >>" in s, s)

            s = run(m, "do broken{enter}")
            files = m.dump()
            check("broken: stopped on line 4", "Stopped: line 4:" in s and "Nothing was changed." in s, s)
            check("broken: output before the error kept", "before" in s, s)
            check("broken: nothing written or sent",
                  "desk:never" not in files and not any(p.startswith("desk:mail:sent:") for p in files), list(files))
            run(m, "{esc}")

            s = run(m, "do invite{enter}")
            files = m.dump()
            sent = {p: t for p, t in files.items() if p.startswith("desk:mail:sent:")}
            check("invite: one mail sent", list(sent.values()) == ["from: me\nto: lina\nsubject: Party\n\nCome on 2026-11-14!"], sent)
            events = [t for p, t in files.items() if p.startswith("desk:calendar:")]
            check("invite: event added", "date: 2026-11-14\ntime: 19:00\ntitle: Party\nwith: lina" in events, events)
            run(m, "{esc}")

            s = run(m, "do stranger{enter}")
            check("send to a non-contact is refused", "no contact has the address 'nobody'" in s, s)
            run(m, "{esc}")

            s = run(m, "do worst{enter}")
            check("worst-case chore runs (no stack overflow)", "Finished:" in s and "145" in s, s)
            run(m, "{esc}")

            s = run(m, "do spin{enter}")
            check("endless-ish loop stops at the step limit", "too many steps" in s, s)
            run(m, "{esc}")
            check("OS still answers after the hostile chores", m._ask("PING") is not None)

            s = run(m, "do nothing{enter}")
            check("do on a missing page", "no page called desk:nothing" in s, s)

            s = run(m, "clock{enter}")
            check("frozen clock", "2026-11-12" in s, s)

            s = run(m, "guide do{enter}")
            check("guide do opens the reference", "CHORES: the complete reference" in s and "lines 1-22 of" in s, s)
            s = run(m, "{end}")
            check("guide scrolls to the end", "50000 steps" in s, s)
            run(m, "{esc}")

            s = run(m, "guide{enter}")
            check("guide lists the Chores group", "Chores" in s and "do" in s, s)

            # the cursor report in the Mail reply form (experiment 2's hidden trap)
            run(m, "mail{enter}{down}{enter}")
            s = run(m, "r")
            row, col = m.cursor()
            check("cursor shown in the reply body", s.splitlines()[row - 1].strip() == "" and row > 6, (row, col, s))
            run(m, "{tab}")
            row, col = m.cursor()
            check("cursor moves to To after Tab", "To:" in m.look().splitlines()[row - 1], (row, col))
        finally:
            m.stop()
    print()
    print("ALL GOOD" if not failures else f"{len(failures)} FAILED: {failures}")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
