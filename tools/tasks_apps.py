"""The app tasks of pilot M7 (docs/script-design.md §11 and §12): build an app, add a feature
to an app, fix a bug in an app. Pilot tasks are never reused in an experiment.

Each task has:
  start     start files of the player's desk
  request   what the player is asked (the prompt adds the manual)
  allowed   path prefixes the player may change (everything else must stay exactly as it was)
  solution  the known solution: shell lines a robot types (each followed by Enter)
  hidden    the hidden test page; the grader runs it on a fresh desk that holds only the
            player's pages under desk:apps (and the app drawers), so data comes from pretend
  wrong     wrong solutions that the hidden tests must catch: {name: {page: new text}}
  f2p       titles of hidden tests that fail on the start desk (fail-to-pass); the others
            must pass there too (pass-to-pass)
"""

NL = "\n"


def page(path, text):
    return f"#page {path}{NL}{text.strip(NL)}{NL}#end{NL}"


def inserts(path, text, first=1):
    """The shell lines that type `text` into a page, one insert per line."""
    return [f"insert {path} {first + i} {line}" for i, line in enumerate(text.strip(NL).split(NL))]


BASE = "#today 2026-11-20\n#drawer apps\n"

# ---------------------------------------------------------------- A: build a tally app --

TALLY = """
app 'Tally'
task start()
  when exists('desk:tally')
    give number(read('desk:tally'))
  done
  give 0
done
task key(n, k)
  when k is '+'
    give n + 1
  or when k is '-' and n > 0
    give n - 1
  or when k is 'r'
    give 0
  or when k is 'enter'
    write('desk:tally', n)
  done
  give n
done
task draw(n)
  show(1, 1, 'Count: ' & n)
done
"""

TALLY_HIDDEN = """
test 'it starts at 0 when nothing is saved'
  launch 'desk:apps:tally'
  check app_open()
  check screen_line(1) is 'Count: 0'
  press '+'
  check screen_line(1) is 'Count: 1'
done

test 'it starts from the saved count; plus adds; enter saves'
  pretend 'desk:tally' holds '41'
  launch 'desk:apps:tally'
  check screen_line(1) is 'Count: 41'
  press '+'
  press '+'
  check screen_line(1) is 'Count: 43'
  check page_now('desk:tally') is '41'
  press 'enter'
  check page_now('desk:tally') is '43'
  check app_open()
done

test 'minus never goes below 0, r sets it to 0'
  pretend 'desk:tally' holds '2'
  launch 'desk:apps:tally'
  press '-'
  check screen_line(1) is 'Count: 1'
  press '-'
  press '-'
  check screen_line(1) is 'Count: 0'
  press '+'
  press '+'
  press '+'
  press 'r'
  check screen_line(1) is 'Count: 0'
  press 'enter'
  check page_now('desk:tally') is '0'
  check app_open()
done

test 'other keys change nothing'
  pretend 'desk:tally' holds '5'
  launch 'desk:apps:tally'
  press 'x'
  press 'up'
  press 'esc'
  press 'backspace'
  check screen_line(1) is 'Count: 5'
  check page_now('desk:tally') is '5'
  check app_open()
done
"""

# ------------------------------------------------- B: add a feature to a to-do app --

TODO = """
app 'To do'
task start()
  set items to []
  when exists('desk:todo')
    set items to lines(read('desk:todo'))
  done
  give {items: items, picked: 1, typing: ''}
done
task save(items)
  write('desk:todo', join(items, newline()))
done
task key(s, k)
  when k is 'up' and s.picked > 1
    set s.picked to s.picked - 1
  or when k is 'down' and s.picked < count(s.items)
    set s.picked to s.picked + 1
  or when k is 'enter' and s.typing is not ''
    add s.typing to s.items
    set s.typing to ''
    save(s.items)
  or when k is 'delete' and count(s.items) > 0
    set s.items to without(s.items, s.picked)
    set s.picked to max(min(s.picked, count(s.items)), 1)
    save(s.items)
  otherwise
    set s.typing to edit_text(s.typing, k)
  done
  give s
done
task draw(s)
  title('To do - ' & count(s.items) & ' items')
  for each n in range(1, min(count(s.items), 20))
    when n is s.picked
      show(n, 1, '> ' & item(s.items, n), 'bright')
    otherwise
      show(n, 1, '  ' & item(s.items, n))
    done
  done
  show(23, 1, 'new: ' & s.typing)
  help('Up/Down pick  Enter add  Delete remove')
done
"""

TODO_TESTS = """
use 'desk:apps:todo'

test 'enter adds the typed item and saves the list'
  pretend 'desk:todo' holds 'call Mira'
  launch 'desk:apps:todo'
  press 'p'
  press 'a'
  press 'y'
  press 'enter'
  check app_state().items is ['call Mira', 'pay']
  check page_now('desk:todo') is 'call Mira' & newline() & 'pay'
done

test 'the picked item is shown with >'
  pretend 'desk:todo' holds 'a' & newline() & 'b'
  launch 'desk:apps:todo'
  press 'down'
  check screen_line(2) is '> b'
  check screen_line(1) is '  a'
done
"""

TODO_MOVE = """
  or when k is 'ctrl+u' and s.picked > 1
    set other to item(s.items, s.picked - 1)
    set s.items at s.picked - 1 to item(s.items, s.picked)
    set s.items at s.picked to other
    set s.picked to s.picked - 1
    save(s.items)
  or when k is 'ctrl+n' and s.picked < count(s.items)
    set other to item(s.items, s.picked + 1)
    set s.items at s.picked + 1 to item(s.items, s.picked)
    set s.items at s.picked to other
    set s.picked to s.picked + 1
    save(s.items)
"""

TODO_HIDDEN = """
test 'ctrl+u moves the picked item up and saves'
  pretend 'desk:todo' holds 'bread' & newline() & 'jam' & newline() & 'tea'
  launch 'desk:apps:todo'
  press 'down'
  press 'down'
  press 'ctrl+u'
  check app_state().items is ['bread', 'tea', 'jam']
  check screen_line(2) is '> tea'
  check page_now('desk:todo') is 'bread' & newline() & 'tea' & newline() & 'jam'
  press 'ctrl+u'
  check screen_line(1) is '> tea'
  check app_open()
done

test 'ctrl+n moves the picked item down and saves'
  pretend 'desk:todo' holds 'one' & newline() & 'two' & newline() & 'three'
  launch 'desk:apps:todo'
  press 'ctrl+n'
  check screen_line(2) is '> one'
  check page_now('desk:todo') is 'two' & newline() & 'one' & newline() & 'three'
  press 'ctrl+n'
  check screen_line(3) is '> one'
  check app_open()
done

test 'at the top and the bottom nothing changes'
  pretend 'desk:todo' holds 'x1' & newline() & 'x2'
  launch 'desk:apps:todo'
  press 'ctrl+u'
  check app_state().items is ['x1', 'x2']
  check page_now('desk:todo') is 'x1' & newline() & 'x2'
  press 'down'
  press 'ctrl+n'
  check app_state().items is ['x1', 'x2']
  check screen_line(2) is '> x2'
  check app_open()
done

test 'moving on an empty list does nothing'
  launch 'desk:apps:todo'
  press 'ctrl+u'
  press 'ctrl+n'
  check app_open()
  check screen_line(1) is ''
done

test 'adding, picking and deleting still work'
  pretend 'desk:todo' holds 'milk' & newline() & 'eggs'
  launch 'desk:apps:todo'
  press 'o'
  press 'k'
  press 'enter'
  check page_now('desk:todo') is 'milk' & newline() & 'eggs' & newline() & 'ok'
  press 'down'
  press 'delete'
  check app_state().items is ['milk', 'ok']
  check screen_line(2) is '> ok'
  check screen_line(23) is 'new:'
  press 'delete'
  check screen_line(1) is '> milk'
  check app_open()
done
"""

# the otherwise of key (draw has one too)
KEY_OTHERWISE = "  otherwise\n    set s.typing to edit_text(s.typing, k)\n"
assert TODO.count(KEY_OTHERWISE) == 1

# --------------------------------------------------------- C: fix a bug in an app --

GUESTS = """
app 'Guests'
task start()
  set guests to []
  when exists('desk:guests')
    set guests to lines(read('desk:guests'))
  done
  give {guests: guests, picked: 1, typing: ''}
done
task save(guests)
  write('desk:guests', join(guests, newline()))
done
task key(s, k)
  when k is 'up' and s.picked > 1
    set s.picked to s.picked - 1
  or when k is 'down' and s.picked < count(s.guests)
    set s.picked to s.picked + 1
  or when k is 'enter' and s.typing is not ''
    add s.typing to s.guests
    set s.typing to ''
    save(s.guests)
  or when k is 'delete' and count(s.guests) > 0
    set s.guests to without(s.guests, s.picked + 1)
    set s.picked to max(min(s.picked, count(s.guests)), 1)
    save(s.guests)
  otherwise
    set s.typing to edit_text(s.typing, k)
  done
  give s
done
task draw(s)
  title(count(s.guests) & ' guests')
  for each n in range(1, min(count(s.guests), 20))
    set mark to '  '
    when n is s.picked
      set mark to '> '
    done
    show(n, 1, mark & n & '. ' & item(s.guests, n))
  done
  show(23, 1, 'name: ' & s.typing)
  help('Up/Down pick  Enter add  Delete remove')
done
"""

GUESTS_TESTS = """
use 'desk:apps:guests'

test 'enter adds a guest'
  pretend 'desk:guests' holds 'Omar'
  launch 'desk:apps:guests'
  press 'L'
  press 'i'
  press 'n'
  press 'a'
  press 'enter'
  check app_state().guests is ['Omar', 'Lina']
  check screen_line(2) is '  2. Lina'
done
"""

GUESTS_HIDDEN = """
test 'delete removes the picked guest'
  pretend 'desk:guests' holds 'Ana' & newline() & 'Bo' & newline() & 'Cy'
  launch 'desk:apps:guests'
  press 'down'
  press 'delete'
  check app_state().guests is ['Ana', 'Cy']
  check page_now('desk:guests') is 'Ana' & newline() & 'Cy'
  check screen_line(2) is '> 2. Cy'
  check app_open()
done

test 'delete on the last guest removes it'
  pretend 'desk:guests' holds 'Ana' & newline() & 'Bo'
  launch 'desk:apps:guests'
  press 'down'
  press 'delete'
  check app_open()
  check app_state().guests is ['Ana']
  check screen_line(1) is '> 1. Ana'
  press 'delete'
  check app_state().guests is []
  check page_now('desk:guests') is ''
  press 'delete'
  check app_open()
done

test 'delete on the first guest'
  pretend 'desk:guests' holds 'Dee' & newline() & 'Eli' & newline() & 'Fay'
  launch 'desk:apps:guests'
  press 'delete'
  check app_state().guests is ['Eli', 'Fay']
  check screen_line(1) is '> 1. Eli'
done

test 'adding and picking still work'
  pretend 'desk:guests' holds 'Gus'
  launch 'desk:apps:guests'
  press 'H'
  press 'a'
  press 'l'
  press 'enter'
  press 'down'
  check screen_line(2) is '> 2. Hal'
  check screen_line(0) is '2 guests'
  check page_now('desk:guests') is 'Gus' & newline() & 'Hal'
  check app_open()
done
"""


def _line_of(text, wanted):
    lines = text.strip(NL).split(NL)
    return lines.index(wanted) + 1


TASKS = {
    "app-tally": {
        "kind": "build",
        "start": BASE,
        "request": (
            "Make an app on the page desk:apps:tally (the drawer desk:apps is there already). "
            "It counts things. When it opens, the count is the number on the page desk:tally if "
            "that page exists, otherwise 0. Row 1 shows exactly 'Count: N' from column 1, where N "
            "is the count. The key + adds 1. The key - takes 1 away, but the count never goes "
            "below 0. The key r sets the count to 0. Enter saves the count: the page desk:tally "
            "then holds just the number. Every other key changes nothing (also Esc: only Ctrl+Q "
            "closes the app). Make sure it works before you finish."),
        "allowed": ("desk:apps:", "desk:tally"), "data": "desk:tally",
        "solution": inserts("desk:apps:tally", TALLY),
        "hidden": TALLY_HIDDEN,
        "f2p": None,  # every hidden test fails without the app
        "wrong": {
            "no save": {"desk:apps:tally": TALLY.replace("    write('desk:tally', n)\n", "    give n\n")},
            "goes below 0": {"desk:apps:tally": TALLY.replace("k is '-' and n > 0", "k is '-'")},
            "ignores the saved count": {"desk:apps:tally": TALLY.replace("    give number(read('desk:tally'))\n", "    give 0\n")},
            "minus resets": {"desk:apps:tally": TALLY.replace("    give n - 1\n", "    give 0\n")},
            "plus stuck at 0": {"desk:apps:tally": TALLY.replace("  when k is '+'\n", "  when k is '+' and n > 0\n")},
            "esc closes": {"desk:apps:tally": TALLY.replace("  or when k is 'enter'", "  or when k is 'esc'\n    close()\n  or when k is 'enter'")},
        },
    },
    "app-todo-move": {
        "kind": "feature",
        "start": BASE + page("apps:todo", TODO) + page("apps:todo-tests", TODO_TESTS)
        + page("todo", "buy stamps\nwater the plants\ncall the bank"),
        "request": (
            "The app on the page desk:apps:todo keeps a to-do list on the page desk:todo (one item "
            "per line); its tests are on desk:apps:todo-tests. Add two keys to the app: Ctrl+U "
            "moves the picked item one place up, and Ctrl+N moves it one place down. The moved item "
            "stays picked, and the list is saved to desk:todo at once. Ctrl+U on the first item "
            "and Ctrl+N on the last item change nothing. Everything else must keep working as it "
            "does now."),
        "allowed": ("desk:apps:", "desk:todo"), "data": "desk:todo",
        "solution": inserts("desk:apps:todo", TODO_MOVE, _line_of(TODO, "  otherwise")),
        "hidden": TODO_HIDDEN,
        "f2p": ["ctrl+u moves the picked item up and saves", "ctrl+n moves the picked item down and saves"],
        "wrong": {
            "only ctrl+u": {"desk:apps:todo": TODO.replace(KEY_OTHERWISE, "\n".join(TODO_MOVE.strip("\n").split("\n")[:6]) + "\n" + KEY_OTHERWISE)},
            "does not save": {"desk:apps:todo": TODO.replace(KEY_OTHERWISE, TODO_MOVE.lstrip("\n").replace("    save(s.items)\n", "") + KEY_OTHERWISE)},
            "pick stays": {"desk:apps:todo": TODO.replace(KEY_OTHERWISE, TODO_MOVE.lstrip("\n").replace("    set s.picked to s.picked - 1\n", "").replace("    set s.picked to s.picked + 1\n", "") + KEY_OTHERWISE)},
            "breaks delete": {"desk:apps:todo": TODO.replace(KEY_OTHERWISE, TODO_MOVE.lstrip("\n") + KEY_OTHERWISE).replace(
                "    set s.picked to max(min(s.picked, count(s.items)), 1)\n", "")},
        },
    },
    "app-guests-fix": {
        "kind": "fix",
        "start": BASE + page("apps:guests", GUESTS) + page("apps:guests-tests", GUESTS_TESTS)
        + page("guests", "Omar\nLina\nMira"),
        "request": (
            "Bug report for the app on the page desk:apps:guests (a guest list, kept on the page "
            "desk:guests; its tests are on desk:apps:guests-tests): Delete removes the wrong guest, "
            "the one below the picked guest, and on the last guest the app stops with a mistake. "
            "Fix it: Delete must remove the picked guest. After that the pick stays on the same row "
            "(or on the new last guest), and Delete on an empty list does nothing. Don't change "
            "anything else."),
        "allowed": ("desk:apps:", "desk:guests"), "data": "desk:guests",
        "solution": [f"edit desk:apps:guests {_line_of(GUESTS, '    set s.guests to without(s.guests, s.picked + 1)')}"
                     "     set s.guests to without(s.guests, s.picked)"],
        "hidden": GUESTS_HIDDEN,
        "f2p": ["delete removes the picked guest", "delete on the last guest removes it", "delete on the first guest"],
        "wrong": {
            "always the first": {"desk:apps:guests": GUESTS.replace("without(s.guests, s.picked + 1)", "without(s.guests, 1)")},
            "caught with try": {"desk:apps:guests": GUESTS.replace(
                "    set s.guests to without(s.guests, s.picked + 1)\n",
                "    try\n      set s.guests to without(s.guests, s.picked + 1)\n    instead p\n    done\n")},
            "last one only": {"desk:apps:guests": GUESTS.replace("without(s.guests, s.picked + 1)", "without(s.guests, count(s.guests))")},
        },
    },
}
