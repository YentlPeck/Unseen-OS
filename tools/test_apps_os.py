"""End-to-end test of Unseen Script apps in the real OS (milestone M5), in QEMU.

  python tools/test_apps_os.py

A robot builds the shopping list app of the design (three pages, typed line by line with
insert), runs its tests with test, then opens it with open and uses it by keys. It also
checks a failing step (the Output screen), Ctrl+Q, how long the slowest step takes, and the
kernel stack with the deepest app steps (STACK on the inspector).
"""

import re
import sys
import tempfile
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from unseen import Machine  # noqa: E402

STACK_SIZE = 256 * 1024
FAILS = []
NL = "\n"


def check(name, ok, detail=""):
    print(("ok   " if ok else "FAIL ") + name)
    if not ok:
        FAILS.append(name)
        print(detail)


def run(m, line, limit=30):
    m.type(line)
    m.press("enter")
    return m.settle(limit=limit)


def keys(m, *names, limit=30):
    m.press(*names)
    return m.settle(limit=limit)


def typed(m, text, limit=30):
    m.type(text)
    return m.settle(limit=limit)


def page(m, path):
    dump = m.dump()
    return dump.get(path) if isinstance(dump, dict) else None


SHOP_LIB = [
    "fixed list_page is 'desk:shop-list'",
    "",
    "task load_items()",
    "  when exists(list_page)",
    "    give lines(read(list_page))",
    "  done",
    "  give []",
    "done",
    "",
    "task save_items(items)",
    "  write(list_page, join(items, newline()))",
    "done",
]

SHOP = [
    "app 'Shopping list'",
    "use 'desk:apps:shop-lib'",
    "",
    "task start()",
    "  give {items: load_items(), picked: 1, typing: ''}",
    "done",
    "",
    "task key(s, k)",
    "  when k is 'down' and s.picked < count(s.items)",
    "    set s.picked to s.picked + 1",
    "  or when k is 'up' and s.picked > 1",
    "    set s.picked to s.picked - 1",
    "  or when k is 'enter' and s.typing is not ''",
    "    add s.typing to s.items",
    "    set s.typing to ''",
    "    save_items(s.items)",
    "  or when k is 'delete' and count(s.items) > 0",
    "    set s.items to without(s.items, s.picked)",
    "    set s.picked to max(min(s.picked, count(s.items)), 1)",
    "    save_items(s.items)",
    "  otherwise",
    "    set s.typing to edit_text(s.typing, k)",
    "  done",
    "  give s",
    "done",
    "",
    "task draw(s)",
    "  title('Shopping list - ' & count(s.items) & ' items')",
    "  -- this small example only shows the first 20 items",
    "  for each n in range(1, min(count(s.items), 20))",
    "    when n is s.picked",
    "      show(n, 2, '> ' & item(s.items, n), 'bright')",
    "    otherwise",
    "      show(n, 2, '  ' & item(s.items, n))",
    "    done",
    "  done",
    "  show(22, 2, 'new: ' & s.typing)",
    "  help('Up/Down pick  Enter add  Delete remove  Ctrl+Q close')",
    "done",
]

SHOP_TESTS = [
    "use 'desk:apps:shop'",
    "",
    "test 'enter adds the typed item and saves it'",
    "  pretend 'desk:shop-list' holds 'milk'",
    "  launch 'desk:apps:shop'",
    "  press 'b'",
    "  press 'enter'",
    "  check app_state().items is ['milk', 'b']",
    "  check page_now('desk:shop-list') is 'milk' & newline() & 'b'",
    "  check screen_line(2) has 'b'",
    "done",
    "",
    "test 'the picked item is shown bright'",
    "  pretend 'desk:shop-list' holds 'milk' & newline() & 'eggs'",
    "  launch 'desk:apps:shop'",
    "  press 'down'",
    "  check screen_line(2) has '> eggs'",
    "  check screen_colour(2, 2) is 'bright'",
    "done",
    "",
    "test 'up and down on an empty list change nothing'",
    "  launch 'desk:apps:shop'",
    "  press 'down'",
    "  press 'up'",
    "  check app_state().picked is 1",
    "  press 'x'",
    "  press 'enter'",
    "  check app_state().items is ['x']",
    "done",
]


def app(name, key, draw, start="  give 0"):
    return NL.join([f"app '{name}'", "task start()", start, "done", "task key(s, k)", key, "  give s", "done",
                    "task draw(s)", draw, "done"])


def down_task(whens, brackets):
    return NL.join(
        ["task down(n)"] + ["when yes"] * whens + ["when n is 0", "give 0", "done"]
        + ["give " + "(1 + " * brackets + "down(n - 1)" + ")" * brackets]
        + ["done"] * whens + ["done"]
    )


ROOT = Path(__file__).resolve().parent.parent


def manual_blocks():
    """(kind, text) of the fenced blocks of the manual's Unseen Script chapter (M6)."""
    md = (ROOT / "docs" / "manual.md").read_text(encoding="utf-8")
    chapter = md.split("## Unseen Script", 1)[1].split(NL + "## ", 1)[0]
    return re.findall(r"```([^" + NL + r"]*)" + NL + r"(.*?)" + NL + "```", chapter, re.S)


def start_files():
    parts = ["#today 2026-11-07"]
    for kind, text in manual_blocks():
        if kind.startswith("page "):
            parts.append(f"#page {kind[5:]}{NL}{text}{NL}#end")
    pages = {
        # a step that fails, and one that never uses a key (Ctrl+Q still closes it)
        "apps:broken": app("Broken", "  when k is 'x'\n    give 1 / 0\n  done\n  set s to s + 1",
                           "  show(1, 1, 'keys: ' & s)"),
        "apps:deaf": app("Deaf", "", "  show(1, 1, 'I ignore every key')\n  help('no keys here')"),
        # about the most work one step may do
        "apps:slow": app("Slow", "  repeat 300 times\n    repeat 300 times\n      set x to 1\n    done\n  done\n  set s to s + 1",
                         "  show(1, 1, 'steps: ' & s)"),
        # the deepest steps the limits allow, in key and in draw
        "deeplib:down": down_task(14, 10),
        "apps:deep": "app 'Deep'\nuse 'deeplib:down'\ntask start()\n  give 0\ndone\ntask key(s, k)\n  give down(7)\ndone\n"
                     "task draw(s)\n  show(1, 1, 'deep: ' & down(7))\ndone",
        "apps:deep-tests": "use 'deeplib:down'\ntest 'deep'\n  launch 'desk:apps:deep'\n  press 'a'\n"
                           "  check down(7) is 70\n  check app_open()\ndone",
    }
    for path, text in pages.items():
        parts.append(f"#page {path}{NL}{text}{NL}#end")
    return NL.join(parts) + NL


def type_page(m, path, lines):
    scr = ""
    for i, line in enumerate(lines):
        scr = run(m, f"insert {path} {i + 1} {line}")
    return scr


def main():
    with tempfile.TemporaryDirectory() as tmp:
        m = Machine.start(start_files(), workdir=tmp)
        try:
            scr = run(m, "guide")
            check("guide lists open", "open" in scr, scr)

            # ---- M6: guide script, and the manual's examples at the prompt ----
            scr = run(m, "guide script")
            check("guide script opens the reference", "UNSEEN SCRIPT: the complete reference" in scr, scr)
            keys(m, "esc")
            commands = [b for k, b in manual_blocks() if k == "commands"]
            for line in commands[0].split(NL):
                scr = run(m, line)
            check("the manual's first program runs", "Finished:" in scr and "Hello Mira!" in scr, scr)
            keys(m, "esc")
            scr = run(m, commands[1])
            check("the manual's counter app opens", "The count is 0" in scr and "Ctrl+Q close" in scr, scr)
            keys(m, "up")
            keys(m, "up")
            scr = keys(m, "down")
            check("it counts by keys", "The count is 1" in scr, scr)
            keys(m, "enter")
            check("Enter saves the count", page(m, "desk:count") == "1", page(m, "desk:count"))
            keys(m, "ctrl+q")

            # ---- the robot builds the shopping list app ----
            scr = type_page(m, "apps:shop-lib", SHOP_LIB)
            check("the library page is typed", f"({len(SHOP_LIB)} lines now)" in scr, scr)
            type_page(m, "apps:shop", SHOP)
            type_page(m, "apps:shop-tests", SHOP_TESTS)
            check("the app page is as typed", page(m, "desk:apps:shop") == NL.join(SHOP), page(m, "desk:apps:shop"))

            # ---- its tests pass ----
            scr = run(m, "test apps:shop-tests", limit=60)
            check("the shopping list tests pass", "3 of 3 tests passed" in scr and "FAIL" not in scr, scr)
            check("tests change nothing on the desk", page(m, "desk:shop-list") is None, page(m, "desk:shop-list"))
            keys(m, "esc")

            # ---- it works by keys ----
            scr = run(m, "open apps:shop")
            check("open shows the app", "Shopping list - 0 items" in scr and "new:" in scr and "Ctrl+Q close" in scr, scr)
            m.type("milk")
            scr = keys(m, "enter")
            check("typing and Enter adds an item", "> milk" in scr and "Shopping list - 1 items" in scr, scr)
            check("the step made its change", page(m, "desk:shop-list") == "milk", page(m, "desk:shop-list"))
            m.type("eggs")
            scr = m.settle()
            check("the typed text is shown", "new: eggs" in scr, scr)
            keys(m, "enter")
            m.type("tea")
            keys(m, "enter")
            scr = keys(m, "down")
            check("Down picks the next item", "> eggs" in scr and "  milk" in scr, scr)
            scr = keys(m, "delete")
            check("Delete removes the picked item", "eggs" not in scr and "> tea" in scr, scr)
            check("and saves the list", page(m, "desk:shop-list") == "milk\ntea", page(m, "desk:shop-list"))
            m.type("bread")
            keys(m, "backspace", "backspace")
            scr = m.settle()
            check("Backspace takes back a letter", "new: bre" in scr and "new: brea" not in scr, scr)
            scr = keys(m, "ctrl+q")
            check("Ctrl+Q closes the app", ">>" in scr and "Shopping list" not in scr, scr)
            scr = run(m, "open apps:shop")
            check("open again loads the saved list", "Shopping list - 2 items" in scr and "> milk" in scr, scr)
            keys(m, "ctrl+q")

            # ---- a failing step ----
            scr = run(m, "open apps:broken")
            scr = typed(m, "a")
            check("a step that works", "keys: 1" in scr, scr)
            scr = typed(m, "x")
            check("a failing step shows the mistake on the Output screen",
                  "Stopped: desk:apps:broken line 7: dividing by 0" in scr and "The app was closed" in scr, scr)
            scr = keys(m, "esc")
            check("Esc goes back to where it was opened", ">>" in scr, scr)
            scr = run(m, "open apps:deaf")
            check("the OS adds Ctrl+Q close to the help", "no keys here   Ctrl+Q close" in scr, scr)
            scr = keys(m, "ctrl+q")
            check("Ctrl+Q closes an app that ignores keys", ">>" in scr, scr)
            scr = run(m, "open notes:none")
            check("open refuses a page that isn't there", "Could not do that" in scr, scr)
            run(m, "insert plain 1 say 'hi'")
            scr = run(m, "open plain")
            check("open refuses a page that is not an app", "this page is not an app" in scr, scr)
            keys(m, "esc")
            scr = run(m, "run apps:shop")
            check("run refuses an app page", "desk:apps:shop: this page is an app: open it with open" in scr, scr)
            keys(m, "esc")

            # ---- the slowest step, and Ctrl+Q right after it ----
            run(m, "open apps:slow")
            t = time.time()
            m.type("a")
            m.wait_handled(limit=60)
            slow = time.time() - t
            scr = m.settle()
            print(f"     a step of about 360000 steps took {slow:.2f} s in QEMU")
            check("the slow step finishes", "steps: 1" in scr, scr)
            t = time.time()
            m.type("a")
            m.press("ctrl+q")
            m.wait_handled(limit=60)
            scr = m.settle()
            print(f"     a slow step and then Ctrl+Q took {time.time() - t:.2f} s")
            check("Ctrl+Q after a slow step closes the app", ">>" in scr, scr)

            # ---- the deepest app steps: the stack ----
            before = int(m._ask("STACK"))
            scr = run(m, "open apps:deep", limit=60)
            check("a deep start step", "deep: 70" in scr, scr)
            scr = typed(m, "a", limit=60)
            check("a deep key step", "deep: 70" in scr and m.alive(), scr)
            keys(m, "ctrl+q")
            scr = run(m, "test apps:deep-tests", limit=60)
            check("a deep app step inside a test", "1 of 1 tests passed" in scr, scr)
            keys(m, "esc")
            stack = int(m._ask("STACK"))
            heap = int(m._ask("HEAP"))
            print(f"     stack used at most {stack // 1024} KB of {STACK_SIZE // 1024} KB (before: {before // 1024} KB); heap in use {heap // 1024} KB")
            check("the deepest app steps leave at least half the stack free", stack < STACK_SIZE // 2, f"{stack}")
            check("the OS still answers", m.alive())
        finally:
            m.stop()
    print("ALL GOOD" if not FAILS else f"{len(FAILS)} FAILED: {FAILS}")
    return 1 if FAILS else 0


if __name__ == "__main__":
    sys.exit(main())
