"""Stress test for keystroke delivery (experiment 4 build step 3).

  python tools/stress_keys.py [machines] [rounds] [lines]

Boots N copies of the OS at once (as in a run: 2 games per player = 4 at once), opens a page
in Notes in each, and sends long single actions (several 70-character lines of random text,
with shifted and unshifted characters, and Enter between lines), the way the broker sends one
action. Afterwards it saves the page and compares it with the text that was sent, character for
character. Reports lost keys (handled < sent) and wrong text separately.
"""

import random
import sys
import tempfile
import threading
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from unseen import Machine, SettleError, keys_for_names, keys_for_text  # noqa: E402

CHARS = ("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
         " .,:;'\"()[]-_=+!?/&*#<>")


def one(idx, rounds, lines, seed, out):
    rng = random.Random(seed + idx)
    work = Path(tempfile.mkdtemp(prefix=f"stress{idx}-"))
    m = Machine.start("", workdir=work)
    sent_lines, losses, t0 = [], [], time.time()
    try:
        m.run_keys("notes desk:s{enter}")
        for r in range(rounds):
            combos = []
            for _ in range(lines):
                line = "".join(rng.choice(CHARS) for _ in range(70))
                sent_lines.append(line)
                combos += keys_for_text(line) + keys_for_names(["enter"])
            before = m.sent
            m.send_combos(combos)
            try:
                m.settle(limit=30)
            except SettleError as e:
                losses.append((r, len(combos), str(e)))
                m.sent = m.keys_handled()  # carry on counting from what the OS has
        m.press("ctrl+k")
        m.settle()
        files = m.dump()
        page = next((v for k, v in files.items() if k in ("s", "desk:s")), None)
        if page is None:
            raise RuntimeError(f"page not saved; dump has {sorted(files)[:10]}")
        want = "\n".join(sent_lines)
        got = page.rstrip("\n")
        bad = next((i for i, (a, b) in enumerate(zip(got, want)) if a != b), None)
        if bad is None and len(got) != len(want):
            bad = min(len(got), len(want))
        out[idx] = {"keys": m.sent, "losses": losses, "text_ok": got == want, "first_diff": bad,
                    "context": (want[max(0, (bad or 0) - 20):(bad or 0) + 20], got[max(0, (bad or 0) - 20):(bad or 0) + 20])
                    if bad is not None else None, "seconds": round(time.time() - t0)}
    except Exception as e:
        out[idx] = {"error": repr(e), "losses": losses}
    finally:
        m.stop()


def main():
    n = int(sys.argv[1]) if len(sys.argv) > 1 else 4
    rounds = int(sys.argv[2]) if len(sys.argv) > 2 else 5
    lines = int(sys.argv[3]) if len(sys.argv) > 3 else 6
    out = {}
    threads = [threading.Thread(target=one, args=(i, rounds, lines, 4242, out)) for i in range(n)]
    for t in threads:
        t.start()
    for t in threads:
        t.join()
    for i in range(n):
        print(i, out.get(i))
    ok = all(o.get("text_ok") and not o.get("losses") for o in out.values())
    print("ALL GOOD" if ok else "PROBLEMS FOUND")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
