"""Calibration tasks of experiment 5 (milestone B3, docs/benchmark-design.md): played once to set
the budgets and check the spread of difficulty. Never used in the experiment itself; the same
rules and grader as the experiment's tasks (tools/tasks_bench.py)."""

import bench_apps as A
from tasks_bench import (BASE, NL, NO_EFFECTS, SHOP_ABOUT, SHOP_DESK, SHOP_P2P, SCORES_ABOUT, SCORES_DESK, SCORES_P2P,
                         AGENDA_ABOUT, AGENDA_DESK, AGENDA_P2P, event, feature, fix, guarded, lines_of, only_pages, swap)
from tasks_bench_build import CLOSE, build

# ---------------------------------------------------------------- build: temperature --
TEMP = """
app 'Temperature'
task start()
  give ''
done
task key(t, k)
  when k is 'backspace' or '0123456789-.' has k
    give edit_text(t, k)
  done
  give t
done
task draw(t)
  show(1, 1, 'C: ' & t)
  when is_number(t)
    show(2, 1, 'F: ' & number(t) * 9 / 5 + 32)
  otherwise
    show(2, 1, 'F: ?')
  done
done
"""
TEMP_HIDDEN = """
test 'it starts empty, with F: ?'
  launch 'desk:apps:temp'
  check screen_line(1) is 'C:'
  check screen_line(2) is 'F: ?'
done

test 'typed degrees Celsius are shown in Fahrenheit'
  launch 'desk:apps:temp'
  press '2'
  press '5'
  check screen_line(1) is 'C: 25'
  check screen_line(2) is 'F: 77'
  press 'backspace'
  check screen_line(2) is 'F: 35.6'
done

test 'minus and a point work; other characters are not added'
  launch 'desk:apps:temp'
  press '-'
  check screen_line(2) is 'F: ?'
  press '4'
  press '0'
  press 'x'
  check screen_line(1) is 'C: -40'
  check screen_line(2) is 'F: -40'
  press 'backspace'
  press '.'
  press '5'
  check screen_line(1) is 'C: -4.5'
  check screen_line(2) is 'F: 23.9'
  check app_open()
done
"""

# ----------------------------------------------------------------- build: guest book --
BOOK = """
app 'Guest book'
task start()
  set names to []
  when exists('desk:guestbook')
    set names to lines(read('desk:guestbook'))
  done
  give {names: names, typing: ''}
done
task key(s, k)
  when k is 'enter'
    when s.typing is not ''
      add s.typing to s.names
      set s.typing to ''
      write('desk:guestbook', join(s.names, newline()))
    done
  otherwise
    set s.typing to edit_text(s.typing, k)
  done
  give s
done
task draw(s)
  show(1, 1, count(s.names) & ' guests')
  set r to 3
  for each name in reverse(s.names)
    when r <= 22
      show(r, 1, name)
    done
    set r to r + 1
  done
  show(23, 1, 'name: ' & s.typing)
done
"""
BOOK_HIDDEN = """
test 'the names are read and shown newest first'
  pretend 'desk:guestbook' holds """ + lines_of("Ada", "Ben", "Cleo") + """
  launch 'desk:apps:book'
  check screen_line(1) is '3 guests'
  check screen_line(3) is 'Cleo'
  check screen_line(5) is 'Ada'
  check screen_line(23) is 'name:'
done

test 'enter adds the typed name at the end of the page'
  pretend 'desk:guestbook' holds 'Ada'
  launch 'desk:apps:book'
  press 'D'
  press 'a'
  press 'n'
  check screen_line(23) is 'name: Dan'
  press 'enter'
  check page_now('desk:guestbook') is """ + lines_of("Ada", "Dan") + """
  check screen_line(3) is 'Dan'
  check screen_line(1) is '2 guests'
  check screen_line(23) is 'name:'
done

test 'the first name makes the page; enter with nothing typed does nothing'
  launch 'desk:apps:book'
  check screen_line(1) is '0 guests'
  press 'enter'
  check page_now('desk:guestbook') is nothing()
  press 'E'
  press 'enter'
  check page_now('desk:guestbook') is 'E'
done
"""

# ----------------------------------------------------------- feature: home and end --
SHOP_HOME = swap(A.SHOP, "  or when k is 'enter' and s.typing is not ''\n",
                 "  or when k is 'home'\n    set s.picked to 1\n  or when k is 'end'\n    set s.picked to max(count(s.items), 1)\n"
                 "  or when k is 'enter' and s.typing is not ''\n")
SHOP_HOME_HIDDEN = """
test 'end picks the last item, home the first'
  pretend 'desk:shop' holds """ + lines_of("leek", "kale", "corn", "peas") + """
  launch 'desk:apps:shop'
  press 'end'
  check screen_line(4) is '> peas'
  check screen_line(1) is '  leek'
  press 'up'
  press 'home'
  check screen_line(1) is '> leek'
  press 'end'
  press 'delete'
  check app_state().items is ['leek', 'kale', 'corn']
done

test 'home and end on an empty list do nothing'
  launch 'desk:apps:shop'
  press 'end'
  press 'home'
  check app_open()
  press 'z'
  press 'enter'
  check screen_line(1) is '> z'
done
""" + SHOP_P2P

# --------------------------------------------------------------- fix: down past the end --
SCORES_DOWN_BUG = swap(A.SCORES, "  or when k is 'down' and s.picked < count(s.players)\n", "  or when k is 'down' and s.picked <= count(s.players)\n")
SCORES_DOWN_HIDDEN = """
test 'down on the last player changes nothing'
  pretend 'desk:scores' holds """ + lines_of("Ola: 2", "Pim: 6") + """
  launch 'desk:apps:scores'
  press 'down'
  press 'down'
  check screen_line(2) is '> Pim: 6'
  press 'down'
  check screen_line(2) is '> Pim: 6'
  press '+'
  check page_now('desk:scores') is """ + lines_of("Ola: 2", "Pim: 7") + """
  check app_open()
done
""" + SCORES_P2P

# ------------------------------------------------------- feature: next day with events --
AGENDA_NEXT = swap(A.AGENDA, "    set s.day to add_days(s.day, -1)\n", """    set s.day to add_days(s.day, -1)
  or when k is 'enter'
    set next to ''
    for each e in s.events
      when e.date > s.day and (next is '' or e.date < next)
        set next to e.date
      done
    done
    when next is not ''
      set s.day to next
    done
""")
AGENDA_NEXT_HIDDEN = """
test 'enter goes to the next day that has events'
  pretend today is '2027-04-10'
""" + NL.join([event(1, "2027-04-25", "09:00", "Late"), event(2, "2027-04-14", "16:00", "Next"),
               event(3, "2027-04-02", "10:00", "Past"), event(4, "2027-04-14", "08:00", "Early")]) + """
  launch 'desk:apps:agenda'
  press 'enter'
  check screen_line(1) is '2027-04-14 wednesday'
  check screen_line(3) is '08:00 Early'
  press 'enter'
  check screen_line(1) is '2027-04-25 sunday'
  press 'enter'
  check screen_line(1) is '2027-04-25 sunday'
  check app_open()
done

test 'with no later events enter does nothing'
  pretend today is '2027-04-10'
""" + event(1, "2027-04-01", "10:00", "Gone") + """
  launch 'desk:apps:agenda'
  press 'enter'
  check screen_line(1) is '2027-04-10 saturday'
  press 'left'
  press 'left'
  press 'left'
  press 'left'
  press 'left'
  press 'left'
  press 'left'
  press 'left'
  press 'left'
  press 'enter'
  check screen_line(1) is '2027-04-01 thursday'
done
""" + AGENDA_P2P

TASKS = {
    "calib-build-temp": build(
        "desk:apps:temp", "easy",
        "It converts degrees Celsius to Fahrenheit. Typing digits, '-' and '.' builds a text of degrees Celsius, "
        "and Backspace takes its last character away; other characters are not added. Row 1 shows 'C: TEXT'. Row 2 "
        "shows 'F: N', where N is the text as a number times 9, divided by 5, plus 32, shown as Unseen Script shows "
        "numbers (for example 77 or 35.6); while the text is not a number (also when it is empty), row 2 shows "
        "'F: ?'. The app starts with an empty text and changes no pages.",
        TEMP, TEMP_HIDDEN,
        {"letters added": swap(TEMP, "  when k is 'backspace' or '0123456789-.' has k\n    give edit_text(t, k)\n  done\n  give t\n", "  give edit_text(t, k)\n"),
         "wrong formula": swap(TEMP, "number(t) * 9 / 5 + 32", "number(t) * 2 + 30")},
        []),
    "calib-build-book": build(
        "desk:apps:book", "medium",
        "It is a guest book, kept on the page desk:guestbook (one name per line); when the app opens it reads that "
        "page, if it exists. Row 1 shows 'N guests', the number of names. From row 3 the names are shown, one per "
        "row, the newest (the last on the page) first. Typed characters build a new name, shown on row 23 as "
        "'name: TEXT'; Backspace takes its last character away. Enter adds the typed name at the end of the page "
        "desk:guestbook (making the page if it isn't there), saves it at once and empties the typed name; with "
        "nothing typed, Enter does nothing.",
        BOOK, BOOK_HIDDEN,
        {"oldest first": swap(BOOK, "  for each name in reverse(s.names)\n", "  for each name in s.names\n"),
         "new name at the start": swap(BOOK, "      add s.typing to s.names\n", "      set s.names to reverse(s.names)\n      add s.typing to s.names\n      set s.names to reverse(s.names)\n")},
        ["desk:guestbook"]),
    "calib-shop-home": feature(
        SHOP_ABOUT, SHOP_DESK, "desk:apps:shop", A.SHOP, SHOP_HOME,
        "Add two keys: Home picks the first item and End picks the last item. With an empty list they do nothing. "
        "Everything else must keep working as it does now.",
        SHOP_HOME_HIDDEN, ["end picks the last item, home the first"],
        {"end picks one too far": swap(SHOP_HOME, "    set s.picked to max(count(s.items), 1)\n", "    set s.picked to count(s.items) + 1\n"),
         "no home": swap(SHOP_HOME, "  or when k is 'home'\n    set s.picked to 1\n", "")},
        "easy", ["desk:shop"]),
    "calib-scores-fix-down": fix(
        SCORES_ABOUT, SCORES_DESK, "desk:apps:scores", SCORES_DOWN_BUG, A.SCORES,
        "Bug report: Down on the last player moves the pick past the end (no player has '>' any more), and then + "
        "makes the app stop with a mistake. Down on the last player must change nothing. Fix it, and don't change "
        "anything else.",
        SCORES_DOWN_HIDDEN, ["down on the last player changes nothing"],
        {"only + made safe": swap(SCORES_DOWN_BUG, "  or when k is '+' and count(s.players) > 0\n", "  or when k is '+' and s.picked <= count(s.players)\n"),
         "down wraps to the top": swap(SCORES_DOWN_BUG, "  or when k is 'down' and s.picked <= count(s.players)\n    set s.picked to s.picked + 1\n",
                                       "  or when k is 'down' and s.picked < count(s.players)\n    set s.picked to s.picked + 1\n  or when k is 'down'\n    set s.picked to 1\n")},
        "medium", ["desk:scores"]),
    "calib-agenda-next": feature(
        AGENDA_ABOUT, AGENDA_DESK, "desk:apps:agenda", A.AGENDA, AGENDA_NEXT,
        "Add a key: Enter goes to the next day after the shown day that has an event (the earliest such day). If "
        "no later day has an event, Enter does nothing. Everything else must keep working as it does now.",
        AGENDA_NEXT_HIDDEN, ["enter goes to the next day that has events"],
        {"next day always": swap(AGENDA_NEXT, "    when next is not ''\n      set s.day to next\n    done\n", "    set s.day to add_days(s.day, 1)\n"),
         "the last day with events": swap(AGENDA_NEXT, "(next is '' or e.date < next)", "(next is '' or e.date > next)")},
        "hard", []),
}
for _id, _t in TASKS.items():
    if "app" in _t:  # a build task: its known solution is the app, typed line by line
        _path, _app = _t.pop("app")
        _t["solution"] = [f"insert {_path} {i + 1} {line}" for i, line in enumerate(_app.strip(NL).split(NL))]
        _t["solved"] = {_path: _app}
    _t["hidden"] = guarded(_t["hidden"], [d[len("desk:"):] for d in _t["data"] if d.count(":") == 1])
