"""The tasks of experiment 5 (docs/benchmark-design.md): build, feature and fix tasks for
Unseen Script apps. Never used before; the pilot's tasks (tools/tasks_apps.py) are not reused.

Each task:
  kind, level      build | feature | fix; easy | medium | hard
  start            start files of the player's desk
  request          what the player is asked (the harness adds the scope and the condition)
  allowed, data    path entries the player may change (':' at the end: a whole drawer), and
                   the data page(s) named in the scope sentence
  solution         the known solution: shell lines a robot types
  hidden           the hidden test page (run on a fresh desk with only the player's pages
                   under desk:apps; data comes from pretend)
  f2p              hidden tests that fail on the start desk (None: all of them, for build)
  wrong            wrong solutions the hidden tests must catch: {name: {page: new text}}
"""

import difflib

import bench_apps as A

NL = "\n"
BASE = "#today 2026-11-20\n#drawer apps\n"


def page(path, text):
    return f"#page {path}{NL}{text.strip(NL)}{NL}#end{NL}"


def swap(text, old, new, count=1):
    """text with old replaced by new; old must be there exactly `count` times."""
    assert text.count(old) == count, (old, text.count(old))
    return text.replace(old, new)


def inserts(path, text):
    return [f"insert {path} {i + 1} {line}" for i, line in enumerate(text.strip(NL).split(NL))]


def edit_commands(path, old, new):
    """Shell lines (edit / insert / cut) that turn page text `old` into `new`, last change first
    so the line numbers of the earlier changes stay right."""
    a, b = old.strip(NL).split(NL), new.strip(NL).split(NL)
    out = []
    for tag, i1, i2, j1, j2 in reversed(difflib.SequenceMatcher(None, a, b, autojunk=False).get_opcodes()):
        if tag == "equal":
            continue
        common = min(i2 - i1, j2 - j1)
        for k in range(common):
            out.append(f"edit {path} {i1 + k + 1} {b[j1 + k]}")
        for k, line in enumerate(b[j1 + common:j2]):
            out.append(f"insert {path} {i1 + common + 1 + k} {line}")
        if i2 - i1 > common:
            first, last = i1 + common + 1, i2
            out.append(f"cut {path} {first}" + (f" {last}" if last > first else ""))
    return out


def lines_of(*items):
    """A text with these lines, as Unseen Script: 'a' & newline() & 'b'."""
    return " & newline() & ".join(f"'{i}'" for i in items)


def event(n, date, time, title, with_=""):
    return f"  pretend 'calendar:e{n}' holds 'date: {date}' & newline() & 'time: {time}' & newline() & 'title: {title}' & newline() & 'with: {with_}'"


NO_EFFECTS = "\n  check sent() is []\n  check events() is []"


def only_pages(*names):
    """Test lines: the test world's top-level pages are only these (no page written unasked)."""
    allowed = "[" + ", ".join(f"'{n}'" for n in names) + "]"
    return f"\n  for each p in pages('desk')\n    check {allowed} has p\n  done"


def scope(task):
    """What the player may change, added to every request by the harness."""
    data = task["data"]
    named = ""
    if data:
        names = " and ".join(data)
        named = f" The page{'s' if len(data) > 1 else ''} {names} may change while you try the app out."
    return (" You may keep extra pages of your own inside the drawer desk:apps (for example test pages, or "
            "pages your app loads with use), but while the app runs it must not read or change pages in "
            "desk:apps." + named + " Everything else must stay exactly as it was.")

# ======================================================================= the shopping list ==

SHOP_DESK = BASE + page("apps:shop", A.SHOP) + page("apps:shop-tests", A.SHOP_TESTS) + page("shop", "bread\nmilk\napples")
SHOP_ABOUT = ("The app on the page desk:apps:shop is a shopping list, kept on the page desk:shop (one item "
              "per line); its tests are on desk:apps:shop-tests. ")
SHOP_P2P = """
test 'adding, picking and deleting still work'
  pretend 'desk:shop' holds """ + lines_of("pears", "nuts") + """
  launch 'desk:apps:shop'
  press 'j'
  press 'a'
  press 'm'
  press 'enter'
  check page_now('desk:shop') is """ + lines_of("pears", "nuts", "jam") + """
  press 'down'
  press 'delete'
  check page_now('desk:shop') is """ + lines_of("pears", "jam") + """
  check app_state().items is ['pears', 'jam']
  check screen_line(2) is '> jam'
  check screen_line(0) is 'Shopping - 2 items'""" + only_pages("shop") + NO_EFFECTS + """
  check app_open()
done

test 'down and up move the pick'
  pretend 'desk:shop' holds """ + lines_of("a1", "b2", "c3") + """
  launch 'desk:apps:shop'
  press 'down'
  press 'down'
  press 'up'
  check screen_line(2) is '> b2'
  check screen_line(3) is '  c3'
  press 'up'
  check screen_line(1) is '> a1'
done

test 'a long list shows its items up to row 20'
  pretend 'desk:shop' holds """ + lines_of(*[f"thing{i}" for i in range(1, 22)]) + """
  launch 'desk:apps:shop'
  check screen_line(20) is '  thing20'
  check screen_line(0) is 'Shopping - 21 items'
done
"""

# F1: an empty list says so
SHOP_EMPTY = swap(A.SHOP, "  title('Shopping - ' & count(s.items) & ' items')\n",
                  "  title('Shopping - ' & count(s.items) & ' items')\n  when count(s.items) is 0\n    show(1, 1, '(empty)')\n  done\n")
SHOP_EMPTY_HIDDEN = """
test 'an empty list shows (empty) on row 1'
  launch 'desk:apps:shop'
  check screen_line(1) is '(empty)'
  press 'x'
  check screen_line(1) is '(empty)'
  check screen_line(23) is 'new: x'
done

test 'with items there is no (empty)'
  pretend 'desk:shop' holds 'salt'
  launch 'desk:apps:shop'
  check screen_line(1) is '> salt'
done

test 'deleting the last item shows (empty) again'
  pretend 'desk:shop' holds 'salt'
  launch 'desk:apps:shop'
  press 'delete'
  check screen_line(1) is '(empty)'
  press 'k'
  press 'enter'
  check screen_line(1) is '> k'
  check app_open()
done
""" + SHOP_P2P

# F2: Ctrl+E takes the picked item back into the new: line
SHOP_EDIT = swap(A.SHOP, "  otherwise\n    set s.typing to edit_text(s.typing, k)\n",
                 "  or when k is 'ctrl+e' and count(s.items) > 0\n    set s.typing to item(s.items, s.picked)\n"
                 "    set s.items to without(s.items, s.picked)\n    set s.picked to max(min(s.picked, count(s.items)), 1)\n"
                 "    save(s.items)\n  otherwise\n    set s.typing to edit_text(s.typing, k)\n")
SHOP_EDIT_HIDDEN = """
test 'ctrl+e takes the picked item out and into the new line'
  pretend 'desk:shop' holds """ + lines_of("eggs", "flour", "sugar") + """
  launch 'desk:apps:shop'
  press 'down'
  press 'ctrl+e'
  check app_state().items is ['eggs', 'sugar']
  check screen_line(23) is 'new: flour'
  check screen_line(2) is '> sugar'
  check page_now('desk:shop') is """ + lines_of("eggs", "sugar") + """
  press 's'
  press 'enter'
  check page_now('desk:shop') is """ + lines_of("eggs", "sugar", "flours") + """
  check app_open()
done

test 'ctrl+e on the last item, and what was typed is replaced'
  pretend 'desk:shop' holds """ + lines_of("tea", "cocoa") + """
  launch 'desk:apps:shop'
  press 'z'
  press 'down'
  press 'ctrl+e'
  check screen_line(23) is 'new: cocoa'
  check screen_line(1) is '> tea'
  check app_state().items is ['tea']
done

test 'ctrl+e on an empty list does nothing'
  launch 'desk:apps:shop'
  press 'ctrl+e'
  check screen_line(23) is 'new:'
  check app_state().items is []
  check app_open()
done
""" + SHOP_P2P

# X1: Enter with nothing typed adds an empty item
SHOP_X1_BUG = swap(A.SHOP, "  or when k is 'enter' and s.typing is not ''\n", "  or when k is 'enter'\n")
SHOP_X1_HIDDEN = """
test 'enter with nothing typed does nothing'
  pretend 'desk:shop' holds """ + lines_of("rice", "beans") + """
  launch 'desk:apps:shop'
  press 'enter'
  check app_state().items is ['rice', 'beans']
  check page_now('desk:shop') is """ + lines_of("rice", "beans") + """
  check screen_line(0) is 'Shopping - 2 items'
done

test 'enter with nothing typed on an empty list'
  launch 'desk:apps:shop'
  press 'enter'
  press 'enter'
  check app_state().items is []
  check page_now('desk:shop') is nothing()
done
""" + SHOP_P2P

# X2: Up on the first item makes the pick disappear
SHOP_X2_BUG = swap(A.SHOP, "  when k is 'up' and s.picked > 1\n", "  when k is 'up' and s.picked > 0\n")
SHOP_X2_HIDDEN = """
test 'up on the first item changes nothing'
  pretend 'desk:shop' holds """ + lines_of("oats", "figs") + """
  launch 'desk:apps:shop'
  press 'up'
  check screen_line(1) is '> oats'
  press 'up'
  press 'delete'
  check app_open()
  check app_state().items is ['figs']
done

test 'up after down goes back to the first item'
  pretend 'desk:shop' holds """ + lines_of("oats", "figs", "kale") + """
  launch 'desk:apps:shop'
  press 'down'
  press 'down'
  press 'up'
  press 'up'
  press 'up'
  check screen_line(1) is '> oats'
  check screen_line(2) is '  figs'
done
""" + SHOP_P2P

# ============================================================================ the scores ==

SCORES_DESK = BASE + page("apps:scores", A.SCORES) + page("apps:scores-tests", A.SCORES_TESTS) + page("scores", "Mira: 4\nOmar: 2\nLina: 7")
SCORES_ABOUT = ("The app on the page desk:apps:scores keeps the points of some players on the page desk:scores "
                "(one line 'Name: points' per player); its tests are on desk:apps:scores-tests. ")
SCORES_P2P = """
test 'picking and changing points still work'
  pretend 'desk:scores' holds """ + lines_of("Kim: 1", "Lee: 0") + """
  launch 'desk:apps:scores'
  press 'down'
  press '+'
  check page_now('desk:scores') is """ + lines_of("Kim: 1", "Lee: 1") + """
  press '+'
  press 'up'
  press '-'
  check screen_line(1) is '> Kim: 0'
  check screen_line(2) is '  Lee: 2'
  check page_now('desk:scores') is """ + lines_of("Kim: 0", "Lee: 2") + only_pages("scores") + NO_EFFECTS + """
  check app_open()
done

test 'minus takes one point away'
  pretend 'desk:scores' holds """ + lines_of("Ray: 4", "Sam: 8") + """
  launch 'desk:apps:scores'
  press '-'
  check screen_line(1) is '> Ray: 3'
  check page_now('desk:scores') is """ + lines_of("Ray: 3", "Sam: 8") + """
  press '-'
  check screen_line(1) is '> Ray: 2'
  check page_now('desk:scores') is """ + lines_of("Ray: 2", "Sam: 8") + """
done

test 'minus never goes below 0'
  pretend 'desk:scores' holds """ + lines_of("Tom: 0", "Val: 5") + """
  launch 'desk:apps:scores'
  press '-'
  check screen_line(1) is '> Tom: 0'
  check page_now('desk:scores') is """ + lines_of("Tom: 0", "Val: 5") + """
done

test 'six players are all shown'
  pretend 'desk:scores' holds """ + lines_of("P1: 1", "P2: 2", "P3: 3", "P4: 4", "P5: 5", "P6: 6") + """
  launch 'desk:apps:scores'
  check screen_line(6) is '  P6: 6'
  check screen_line(4) is '  P4: 4'
done

"""

# F3: Ctrl+S sorts by points, most first, stable
SORTED_TASK = """task sorted(players)
  set rest to players
  set out to []
  while count(rest) > 0
    set best to 1
    for each n in range(1, count(rest))
      when item(rest, n).points > item(rest, best).points
        set best to n
      done
    done
    add item(rest, best) to out
    set rest to without(rest, best)
  done
  give out
done
"""
SCORES_SORT = swap(A.SCORES, "task key(s, k)\n", SORTED_TASK + "task key(s, k)\n")
SCORES_SORT = swap(SCORES_SORT, "  done\n  give s\ndone\ntask draw(s)",
                   "  or when k is 'ctrl+s'\n    set s.players to sorted(s.players)\n    set s.picked to 1\n    save(s.players)\n"
                   "  done\n  give s\ndone\ntask draw(s)")
SCORES_SORT_HIDDEN = """
test 'ctrl+s sorts by points, the most first, and saves'
  pretend 'desk:scores' holds """ + lines_of("Ana: 3", "Bo: 7", "Cy: 3", "Di: 9", "Ed: 0") + """
  launch 'desk:apps:scores'
  press 'down'
  press 'down'
  press 'ctrl+s'
  check screen_line(1) is '> Di: 9'
  check screen_line(2) is '  Bo: 7'
  check screen_line(3) is '  Ana: 3'
  check screen_line(4) is '  Cy: 3'
  check screen_line(5) is '  Ed: 0'
  check page_now('desk:scores') is """ + lines_of("Di: 9", "Bo: 7", "Ana: 3", "Cy: 3", "Ed: 0") + """
done

test 'equal points keep their order'
  pretend 'desk:scores' holds """ + lines_of("Zoe: 2", "Al: 2", "Max: 5", "Bea: 2") + """
  launch 'desk:apps:scores'
  press 'ctrl+s'
  check page_now('desk:scores') is """ + lines_of("Max: 5", "Zoe: 2", "Al: 2", "Bea: 2") + """
  press '+'
  check screen_line(1) is '> Max: 6'
done

test 'points are sorted as numbers'
  pretend 'desk:scores' holds """ + lines_of("Ann: 2", "Ben: 12", "Cal: 9", "Dot: 100", "Eve: 12") + """
  launch 'desk:apps:scores'
  press 'ctrl+s'
  check page_now('desk:scores') is """ + lines_of("Dot: 100", "Ben: 12", "Eve: 12", "Cal: 9", "Ann: 2") + """
done

test 'ctrl+s with no players'
  launch 'desk:apps:scores'
  press 'ctrl+s'
  check app_open()
  check screen_line(1) is ''
  check page_now('desk:scores') is nothing() or page_now('desk:scores') is ''
done
""" + SCORES_P2P

# F4: a total on row 23
SCORES_TOTAL = swap(A.SCORES, "  help('Up/Down pick  + and - change the points')\n",
                    "  set total to 0\n  for each p in s.players\n    set total to total + p.points\n  done\n"
                    "  show(23, 1, 'Total: ' & total)\n  help('Up/Down pick  + and - change the points')\n")
SCORES_TOTAL_HIDDEN = """
test 'row 23 shows the total of all points'
  pretend 'desk:scores' holds """ + lines_of("Ana: 3", "Bo: 11", "Cy: 0") + """
  launch 'desk:apps:scores'
  check screen_line(23) is 'Total: 14'
done

test 'the total follows plus and minus'
  pretend 'desk:scores' holds """ + lines_of("Ana: 3", "Bo: 11") + """
  launch 'desk:apps:scores'
  press '+'
  press 'down'
  press '-'
  press '-'
  check screen_line(23) is 'Total: 13'
done

test 'five players'
  pretend 'desk:scores' holds """ + lines_of("A: 1", "B: 2", "C: 4", "D: 8", "E: 15") + """
  launch 'desk:apps:scores'
  check screen_line(23) is 'Total: 30'
  check screen_line(5) is '  E: 15'
done

test 'no players: the total is 0'
  launch 'desk:apps:scores'
  check screen_line(23) is 'Total: 0'
done
""" + SCORES_P2P

# X3: minus goes below 0
SCORES_X3_BUG = swap(A.SCORES, "  set p.points to max(p.points + by, 0)\n", "  set p.points to p.points + by\n")
SCORES_X3_HIDDEN = """
test 'minus at 0 changes nothing'
  pretend 'desk:scores' holds """ + lines_of("Uma: 0", "Vic: 1") + """
  launch 'desk:apps:scores'
  press '-'
  check screen_line(1) is '> Uma: 0'
  press 'down'
  press '-'
  press '-'
  press '-'
  check screen_line(2) is '> Vic: 0'
  check page_now('desk:scores') is """ + lines_of("Uma: 0", "Vic: 0") + """
done
""" + SCORES_P2P

# X4: the page is saved as 'Name:points'
SCORES_X4_BUG = swap(A.SCORES, "    add p.name & ': ' & p.points to out\n", "    add p.name & ':' & p.points to out\n")
SCORES_X4_HIDDEN = """
test 'the page keeps the Name: points form'
  pretend 'desk:scores' holds """ + lines_of("Ana: 4", "Bo: 2") + """
  launch 'desk:apps:scores'
  press '+'
  check page_now('desk:scores') is """ + lines_of("Ana: 5", "Bo: 2") + """
done

test 'the app opens again after a change'
  pretend 'desk:scores' holds """ + lines_of("Ana: 4", "Bo: 2") + """
  launch 'desk:apps:scores'
  press 'down'
  press '-'
  press 'ctrl+q'
  launch 'desk:apps:scores'
  check app_open()
  check screen_line(2) is '  Bo: 1'
done
""" + SCORES_P2P

# ============================================================================ the recipes ==

RECIPES_DESK = (BASE + page("apps:recipes", A.RECIPES) + page("apps:recipes-tests", A.RECIPES_TESTS)
                + page("recipes:omelette", "2 eggs\nsalt") + page("recipes:porridge", "oats\nmilk")
                + page("recipes:salad", "lettuce\ntomato"))
RECIPES_ABOUT = ("The app on the page desk:apps:recipes lists the recipes, the pages in the drawer desk:recipes, "
                 "and shows one when Enter is pressed; its tests are on desk:apps:recipes-tests. ")


def recipes(*names):
    return NL.join(f"  pretend 'desk:recipes:{n}' holds 'about {n}'" for n in names)


def recipes_p2p(*top):
    return """
test 'opening a recipe and going back still work'
""" + recipes("bread", "curry", "dal") + """
  launch 'desk:apps:recipes'
  press 'down'
  press 'enter'
  check screen_line(0) is 'curry'
  check screen_line(1) is 'about curry'
  press 'esc'
  check screen_line(2) is '> curry'
  check screen_line(0) is 'Recipes - 3'
  check page_now('desk:recipes:bread') is 'about bread'
  check page_now('desk:recipes:curry') is 'about curry'
  check page_now('desk:recipes:dal') is 'about dal'
  check count(pages('desk:recipes')) is 3""" + only_pages(*top) + NO_EFFECTS + """
  check app_open()
done

test 'down and up move the pick'
""" + recipes("r-a", "r-b", "r-c") + """
  launch 'desk:apps:recipes'
  press 'down'
  press 'down'
  press 'up'
  check screen_line(2) is '> r-b'
  press 'enter'
  check screen_line(0) is 'r-b'
  press 'esc'
  press 'up'
  check screen_line(1) is '> r-a'
done

test 'twelve recipes are all listed'
""" + recipes(*[f"k{i}" for i in range(1, 13)]) + """
  launch 'desk:apps:recipes'
  check screen_line(12) is '  k12'
done
"""


RECIPES_P2P = recipes_p2p()

# F5: typing filters the list
RECIPES_FIND = swap(A.RECIPES, "  give {names: names, texts: texts, picked: 1, open: no}\n",
                    "  give {names: names, texts: texts, picked: 1, open: no, find: ''}\n")
RECIPES_FIND = swap(RECIPES_FIND, "task key(s, k)\n", """task shown(s)
  set out to []
  for each n in range(1, count(s.names))
    when item(s.names, n) has s.find
      add n to out
    done
  done
  give out
done
task key(s, k)
""")
RECIPES_FIND = swap(RECIPES_FIND, """  or when k is 'down' and s.picked < count(s.names)
    set s.picked to s.picked + 1
  or when k is 'enter' and count(s.names) > 0
    set s.open to yes
  done""", """  or when k is 'down' and s.picked < count(shown(s))
    set s.picked to s.picked + 1
  or when k is 'enter' and count(shown(s)) > 0
    set s.open to yes
  otherwise
    set f to edit_text(s.find, k)
    when f is not s.find
      set s.find to f
      set s.picked to 1
    done
  done""")
RECIPES_FIND = swap(RECIPES_FIND, """  when s.open
    title(item(s.names, s.picked))
    set r to 1
    for each l in lines(item(s.texts, s.picked))""", """  set sh to shown(s)
  when s.open
    set n to item(sh, s.picked)
    title(item(s.names, n))
    set r to 1
    for each l in lines(item(s.texts, n))""")
RECIPES_FIND = swap(RECIPES_FIND, """    for each n in range(1, min(count(s.names), 23))
      set mark to '  '
      when n is s.picked
        set mark to '> '
      done
      show(n, 1, mark & item(s.names, n))
    done
""", """    when count(sh) is 0
      show(1, 1, '(nothing found)')
    done
    for each n in range(1, min(count(sh), 22))
      set mark to '  '
      when n is s.picked
        set mark to '> '
      done
      show(n, 1, mark & item(s.names, item(sh, n)))
    done
    show(23, 1, 'find: ' & s.find)
""")
RECIPES_FIND_HIDDEN = """
test 'typing shows only the recipes whose name has the text'
""" + recipes("apple-pie", "banana-bread", "lentil-soup", "pea-soup") + """
  launch 'desk:apps:recipes'
  press 's'
  press 'o'
  press 'u'
  press 'p'
  check screen_line(23) is 'find: soup'
  check screen_line(1) is '> lentil-soup'
  check screen_line(2) is '  pea-soup'
  check screen_line(3) is ''
  press 'down'
  press 'enter'
  check screen_line(0) is 'pea-soup'
  check screen_line(1) is 'about pea-soup'
done

test 'backspace takes a letter away and the pick goes to row 1'
""" + recipes("apple-pie", "banana-bread", "pea-soup") + """
  launch 'desk:apps:recipes'
  press 'p'
  press 'e'
  press 'a'
  check screen_line(1) is '> pea-soup'
  check screen_line(2) is ''
  press 'backspace'
  press 'backspace'
  check screen_line(23) is 'find: p'
  check screen_line(1) is '> apple-pie'
  check screen_line(2) is '  pea-soup'
  press 'down'
  press 'backspace'
  check screen_line(23) is 'find:'
  check screen_line(1) is '> apple-pie'
  check screen_line(2) is '  banana-bread'
done

test 'typing while a recipe is shown does nothing'
""" + recipes("cake", "pie") + """
  launch 'desk:apps:recipes'
  press 'enter'
  press 'z'
  press 'backspace'
  press 'q'
  check screen_line(0) is 'cake'
  press 'esc'
  check screen_line(23) is 'find:'
  check screen_line(1) is '> cake'
  check screen_line(2) is '  pie'
done

test 'esc goes back to the filtered list as it was'
""" + recipes("apple-pie", "banana-bread", "lentil-soup", "pea-soup") + """
  launch 'desk:apps:recipes'
  press 's'
  press 'o'
  press 'u'
  press 'p'
  press 'down'
  press 'enter'
  press 'x'
  press 'backspace'
  press 'esc'
  check screen_line(23) is 'find: soup'
  check screen_line(1) is '  lentil-soup'
  check screen_line(2) is '> pea-soup'
  check screen_line(3) is ''
done

test 'many recipes can be found'
""" + recipes(*[f"mix{i}" for i in range(1, 12)], "plain") + """
  launch 'desk:apps:recipes'
  press 'm'
  press 'i'
  check screen_line(11) is '  mix11'
  check screen_line(12) is ''
done

test 'nothing found'
""" + recipes("toast") + """
  launch 'desk:apps:recipes'
  press 'x'
  check screen_line(1) is '(nothing found)'
  press 'enter'
  check app_open()
  check screen_line(0) is 'Recipes - 1'
done
""" + RECIPES_P2P

# F6: Ctrl+F marks favourites
RECIPES_FAV = swap(A.RECIPES, "  give {names: names, texts: texts, picked: 1, open: no}\n",
                   "  set favs to []\n  when exists('desk:favourites')\n    set favs to lines(read('desk:favourites'))\n  done\n"
                   "  give {names: names, texts: texts, picked: 1, open: no, favs: favs}\n")
RECIPES_FAV = swap(RECIPES_FAV, "  or when k is 'enter' and count(s.names) > 0\n    set s.open to yes\n",
                   "  or when k is 'enter' and count(s.names) > 0\n    set s.open to yes\n"
                   "  or when k is 'ctrl+f' and count(s.names) > 0\n    set name to item(s.names, s.picked)\n"
                   "    when s.favs has name\n      set s.favs to without(s.favs, position(s.favs, name))\n    otherwise\n"
                   "      add name to s.favs\n    done\n    write('desk:favourites', join(s.favs, newline()))\n")
RECIPES_FAV = swap(RECIPES_FAV, "      show(n, 1, mark & item(s.names, n))\n",
                   "      set star to ''\n      when s.favs has item(s.names, n)\n        set star to ' *'\n      done\n"
                   "      show(n, 1, mark & item(s.names, n) & star)\n")
RECIPES_FAV_HIDDEN = """
test 'favourites are read from desk:favourites and marked with *'
""" + recipes("chili", "gumbo", "stew") + """
  pretend 'desk:favourites' holds 'stew'
  launch 'desk:apps:recipes'
  check screen_line(1) is '> chili'
  check screen_line(3) is '  stew *'
done

test 'ctrl+f adds the picked recipe at the end, and again takes it away'
""" + recipes("chili", "gumbo", "stew") + """
  pretend 'desk:favourites' holds 'stew'
  launch 'desk:apps:recipes'
  press 'down'
  press 'ctrl+f'
  check screen_line(2) is '> gumbo *'
  check page_now('desk:favourites') is """ + lines_of("stew", "gumbo") + """
  press 'down'
  press 'ctrl+f'
  check screen_line(3) is '> stew'
  check page_now('desk:favourites') is 'gumbo'
  check app_open()
done

test 'the first favourite makes the page; the last one leaves it empty'
""" + recipes("chili") + """
  launch 'desk:apps:recipes'
  press 'ctrl+f'
  check page_now('desk:favourites') is 'chili'
  check screen_line(1) is '> chili *'
  press 'ctrl+f'
  check page_now('desk:favourites') is ''
  check screen_line(1) is '> chili'
done

test 'ctrl+f does nothing while a recipe is shown'
""" + recipes("chili", "gumbo") + """
  launch 'desk:apps:recipes'
  set before to page_now('desk:favourites')
  press 'enter'
  press 'ctrl+f'
  check page_now('desk:favourites') is before
  check screen_line(0) is 'chili'
  press 'esc'
  check screen_line(1) is '> chili'
done

test 'ctrl+f with no recipes does nothing'
  launch 'desk:apps:recipes'
  set before to page_now('desk:favourites')
  press 'ctrl+f'
  check page_now('desk:favourites') is before
  check app_open()
done
""" + recipes_p2p("favourites")

# X5: only 9 recipes are listed
RECIPES_X5_BUG = swap(A.RECIPES, "    for each n in range(1, min(count(s.names), 23))\n", "    for each n in range(1, min(count(s.names), 9))\n")
RECIPES_X5_HIDDEN = """
test 'all recipes are listed, up to row 23'
""" + recipes(*[f"r{i}" for i in range(1, 24)]) + """
  launch 'desk:apps:recipes'
  check screen_line(10) is '  r10'
  check screen_line(23) is '  r23'
  repeat 22 times
    press 'down'
  done
  check screen_line(23) is '> r23'
  press 'enter'
  check screen_line(0) is 'r23'
done

test 'eleven recipes'
""" + recipes(*[f"dish{i}" for i in range(1, 12)]) + """
  launch 'desk:apps:recipes'
  check screen_line(11) is '  dish11'
  check screen_line(12) is ''
done
""" + RECIPES_P2P

# X6: Esc in a recipe closes the app
RECIPES_X6_BUG = swap(A.RECIPES, "    when k is 'esc'\n      set s.open to no\n", "    when k is 'esc'\n      close()\n")
RECIPES_X6_HIDDEN = """
test 'esc in a recipe goes back to the list, as it was'
""" + recipes("fish", "rice", "ramen") + """
  launch 'desk:apps:recipes'
  press 'down'
  press 'down'
  press 'enter'
  press 'esc'
  check app_open()
  check screen_line(3) is '> rice'
  check screen_line(0) is 'Recipes - 3'
  press 'up'
  press 'enter'
  check screen_line(0) is 'ramen'
done
""" + RECIPES_P2P

# ============================================================================= the agenda ==

AGENDA_DESK = (BASE + page("apps:agenda", A.AGENDA) + page("apps:agenda-tests", A.AGENDA_TESTS)
               + page("calendar:e1", "date: 2026-11-20\ntime: 10:00\ntitle: Dentist\nwith: ")
               + page("calendar:e2", "date: 2026-11-21\ntime: 18:30\ntitle: Dinner\nwith: lina"))
AGENDA_ABOUT = ("The app on the page desk:apps:agenda shows the calendar's events of one day, starting with "
                "today; its tests are on desk:apps:agenda-tests. ")
AGENDA_P2P = """
test 'the day and its events are still shown, and right still works'
  pretend today is '2027-03-04'
""" + event(1, "2027-03-05", "08:15", "Run") + NL + event(2, "2027-03-05", "07:45", "Swim") + NL + event(3, "2027-03-04", "12:00", "Lunch") + """
  launch 'desk:apps:agenda'
  check screen_line(1) is '2027-03-04 thursday'
  check screen_line(3) is '12:00 Lunch'
  press 'right'
  check screen_line(1) is '2027-03-05 friday'
  check screen_line(3) is '07:45 Swim'
  check screen_line(4) is '08:15 Run'
  check count(pages('calendar')) is 3""" + only_pages() + NO_EFFECTS + """
  check app_open()
done

test 'right then left comes back to the same day'
  pretend today is '2027-03-04'
""" + event(1, "2027-03-03", "18:00", "Choir") + """
  launch 'desk:apps:agenda'
  press 'right'
  press 'left'
  check screen_line(1) is '2027-03-04 thursday'
  press 'left'
  check screen_line(1) is '2027-03-03 wednesday'
  check screen_line(3) is '18:00 Choir'
done
"""

# F7: t goes back to today
AGENDA_TODAY = swap(A.AGENDA, "  give {events: events, day: today()}\n", "  give {events: events, day: today(), today: today()}\n")
AGENDA_TODAY = swap(AGENDA_TODAY, "    set s.day to add_days(s.day, -1)\n", "    set s.day to add_days(s.day, -1)\n  or when k is 't'\n    set s.day to s.today\n")
AGENDA_TODAY_HIDDEN = """
test 't goes back to today'
  pretend today is '2027-01-30'
""" + event(1, "2027-01-30", "09:00", "Gym") + """
  launch 'desk:apps:agenda'
  press 'right'
  press 'right'
  press 'right'
  check screen_line(1) is '2027-02-02 tuesday'
  press 't'
  check screen_line(1) is '2027-01-30 saturday'
  check screen_line(3) is '09:00 Gym'
  press 'left'
  press 't'
  check screen_line(1) is '2027-01-30 saturday'
done

test 't on today changes nothing'
  pretend today is '2026-12-31'
  launch 'desk:apps:agenda'
  press 't'
  check screen_line(1) is '2026-12-31 thursday'
  check app_open()
done
""" + AGENDA_P2P

# F8: w switches to a week view
AGENDA_WEEK = swap(A.AGENDA, "  give {events: events, day: today()}\n", "  give {events: events, day: today(), week: no}\n")
AGENDA_WEEK = swap(AGENDA_WEEK, "    set s.day to add_days(s.day, -1)\n", "    set s.day to add_days(s.day, -1)\n  or when k is 'w'\n    set s.week to not s.week\n")
AGENDA_WEEK = swap(AGENDA_WEEK, "  set rows to []\n  for each e in s.events\n", """  when s.week
    for each d in range(0, 6)
      set day to add_days(s.day, d)
      set n to 0
      for each e in s.events
        when e.date is day
          set n to n + 1
        done
      done
      show(3 + d, 1, day & ' ' & weekday(day) & ': ' & n)
    done
    help('Left/Right change the day')
    give
  done
  set rows to []
  for each e in s.events
""")
AGENDA_WEEK_HIDDEN = """
test 'w shows the week: one row per day with its number of events'
  pretend today is '2026-12-28'
""" + NL.join([event(1, "2026-12-28", "09:00", "A"), event(2, "2026-12-30", "10:00", "B"), event(3, "2026-12-30", "11:00", "C"),
               event(4, "2027-01-03", "12:00", "D"), event(5, "2027-01-04", "13:00", "E"), event(6, "2026-12-27", "14:00", "F")]) + """
  launch 'desk:apps:agenda'
  press 'w'
  check screen_line(1) is '2026-12-28 monday'
  check screen_line(3) is '2026-12-28 monday: 1'
  check screen_line(4) is '2026-12-29 tuesday: 0'
  check screen_line(5) is '2026-12-30 wednesday: 2'
  check screen_line(6) is '2026-12-31 thursday: 0'
  check screen_line(7) is '2027-01-01 friday: 0'
  check screen_line(8) is '2027-01-02 saturday: 0'
  check screen_line(9) is '2027-01-03 sunday: 1'
  check screen_line(10) is ''
done

test 'every day of the week is counted, and left moves the week back'
  pretend today is '2027-01-01'
""" + NL.join([event(n, d, f"{8 + k}:00".rjust(5, "0"), f"E{n}") for n, (d, k) in enumerate(
        [("2026-12-31", 0), ("2027-01-01", 0), ("2027-01-02", 0), ("2027-01-02", 1), ("2027-01-03", 0), ("2027-01-04", 0),
         ("2027-01-04", 1), ("2027-01-04", 2), ("2027-01-05", 0), ("2027-01-06", 0), ("2027-01-06", 1), ("2027-01-07", 0)], 1)]) + """
  launch 'desk:apps:agenda'
  press 'w'
  check screen_line(3) is '2027-01-01 friday: 1'
  check screen_line(4) is '2027-01-02 saturday: 2'
  check screen_line(5) is '2027-01-03 sunday: 1'
  check screen_line(6) is '2027-01-04 monday: 3'
  check screen_line(7) is '2027-01-05 tuesday: 1'
  check screen_line(8) is '2027-01-06 wednesday: 2'
  check screen_line(9) is '2027-01-07 thursday: 1'
  press 'left'
  check screen_line(1) is '2026-12-31 thursday'
  check screen_line(3) is '2026-12-31 thursday: 1'
  check screen_line(4) is '2027-01-01 friday: 1'
  check screen_line(8) is '2027-01-05 tuesday: 1'
  check screen_line(9) is '2027-01-06 wednesday: 2'
done

test 'right moves the week, and w goes back to the day'
  pretend today is '2026-12-28'
""" + NL.join([event(1, "2026-12-29", "09:00", "Yoga"), event(2, "2027-01-04", "13:00", "Work")]) + """
  launch 'desk:apps:agenda'
  press 'w'
  press 'right'
  check screen_line(1) is '2026-12-29 tuesday'
  check screen_line(3) is '2026-12-29 tuesday: 1'
  check screen_line(9) is '2027-01-04 monday: 1'
  press 'w'
  check screen_line(3) is '09:00 Yoga'
  check screen_line(4) is ''
  check app_open()
done
""" + AGENDA_P2P

# X7: events are not sorted by time
AGENDA_X7_BUG = swap(A.AGENDA, "  set rows to sort(rows)\n", "")
AGENDA_X7_HIDDEN = """
test 'the events of a day are shown by time, earliest first'
  pretend today is '2026-11-24'
""" + NL.join([event(1, "2026-11-24", "15:00", "Tea"), event(2, "2026-11-24", "08:00", "Walk"),
               event(3, "2026-11-25", "07:00", "Bus"), event(4, "2026-11-24", "11:30", "Call")]) + """
  launch 'desk:apps:agenda'
  check screen_line(3) is '08:00 Walk'
  check screen_line(4) is '11:30 Call'
  check screen_line(5) is '15:00 Tea'
  check screen_line(6) is ''
done
""" + AGENDA_P2P

# X8: Left goes forward
AGENDA_X8_BUG = swap(A.AGENDA, "    set s.day to add_days(s.day, -1)\n", "    set s.day to add_days(s.day, 1)\n")
AGENDA_X8_HIDDEN = """
test 'left goes to the day before'
  pretend today is '2026-12-01'
""" + event(1, "2026-11-30", "20:00", "Film") + """
  launch 'desk:apps:agenda'
  press 'left'
  check screen_line(1) is '2026-11-30 monday'
  check screen_line(3) is '20:00 Film'
  press 'left'
  press 'right'
  press 'right'
  check screen_line(1) is '2026-12-01 tuesday'
done
""" + AGENDA_P2P


def feature(about, desk, app_page, base, solved, request, hidden, f2p, wrong, level, data, extra_allowed=()):
    return {"kind": "feature", "level": level, "start": desk, "request": about + request,
            "allowed": ("desk:apps:",) + tuple(data) + tuple(extra_allowed), "data": data,
            "solution": edit_commands(app_page, base, solved), "hidden": hidden, "f2p": f2p,
            "wrong": {k: {app_page: v} for k, v in wrong.items()}, "solved": {app_page: solved}}


def fix(about, desk, app_page, buggy, fixed, request, hidden, f2p, wrong, level, data):
    t = feature(about, desk.replace(page(app_page[5:], fixed), page(app_page[5:], buggy)), app_page, buggy, fixed,
                request, hidden, f2p, wrong, level, data)
    t["kind"] = "fix"
    return t


def desk_with(desk, app_page, base, text):
    return desk.replace(page(app_page[5:], base), page(app_page[5:], text))


TASKS = {
    # ---- features
    "shop-empty": feature(
        SHOP_ABOUT, SHOP_DESK, "desk:apps:shop", A.SHOP, SHOP_EMPTY,
        "When the list is empty, row 1 must show '(empty)' (from column 1). Everything else must keep "
        "working as it does now.",
        SHOP_EMPTY_HIDDEN, ["an empty list shows (empty) on row 1", "deleting the last item shows (empty) again"],
        {"up does nothing": swap(SHOP_EMPTY, "  when k is 'up' and s.picked > 1\n", "  when k is 'up' and no\n"),
         "on row 2": swap(SHOP_EMPTY, "show(1, 1, '(empty)')", "show(2, 1, '(empty)')"),
         "only when it opens": swap(swap(SHOP_EMPTY, "  when count(s.items) is 0\n    show(1, 1, '(empty)')", "  when s.empty\n    show(1, 1, '(empty)')"),
                                    "picked: 1, typing: ''}", "picked: 1, typing: '', empty: count(items) is 0}")},
        "easy", ["desk:shop"]),
    "shop-edit": feature(
        SHOP_ABOUT, SHOP_DESK, "desk:apps:shop", A.SHOP, SHOP_EDIT,
        "Add a key: Ctrl+E takes the picked item out of the list and puts its text on the new: line (row 23), "
        "replacing anything typed there, so it can be changed and added again with Enter. The list is saved to "
        "desk:shop at once, and the pick then stays on the same row, or goes to the new last item, as after "
        "Delete. With an empty list Ctrl+E does nothing. Everything else must keep working as it does now.",
        SHOP_EDIT_HIDDEN, ["ctrl+e takes the picked item out and into the new line", "ctrl+e on the last item, and what was typed is replaced"],
        {"does not save": swap(SHOP_EDIT, "    save(s.items)\n  otherwise", "  otherwise"),
         "keeps the item": swap(SHOP_EDIT, "    set s.items to without(s.items, s.picked)\n    set s.picked to max(min(s.picked, count(s.items)), 1)\n    save(s.items)\n  otherwise", "  otherwise"),
         "delete not saved": swap(SHOP_EDIT, "    set s.picked to max(min(s.picked, count(s.items)), 1)\n    save(s.items)\n  or when k is 'ctrl+e'",
                                  "    set s.picked to max(min(s.picked, count(s.items)), 1)\n  or when k is 'ctrl+e'"),
         "adds to the typed text": swap(SHOP_EDIT, "    set s.typing to item(s.items, s.picked)\n", "    set s.typing to s.typing & item(s.items, s.picked)\n")},
        "medium", ["desk:shop"]),
    "scores-sort": feature(
        SCORES_ABOUT, SCORES_DESK, "desk:apps:scores", A.SCORES, SCORES_SORT,
        "Add a key: Ctrl+S sorts the players by points, the most points first; players with the same points "
        "keep the order they had. The sorted list is saved to desk:scores at once, and the pick goes to row 1. "
        "Everything else must keep working as it does now.",
        SCORES_SORT_HIDDEN, ["ctrl+s sorts by points, the most first, and saves", "equal points keep their order",
                             "points are sorted as numbers"],
        {"fewest first": swap(SCORES_SORT, "item(rest, n).points > item(rest, best).points", "item(rest, n).points < item(rest, best).points"),
         "equal points turned round": swap(SCORES_SORT, "item(rest, n).points > item(rest, best).points", "item(rest, n).points >= item(rest, best).points"),
         "does not save": swap(SCORES_SORT, "    set s.picked to 1\n    save(s.players)\n", "    set s.picked to 1\n"),
         "sorted as text": swap(SCORES_SORT, "item(rest, n).points > item(rest, best).points", "text(item(rest, n).points) > text(item(rest, best).points)"),
         "minus resets": swap(SCORES_SORT, "  set p.points to max(p.points + by, 0)\n", "  when by < 0\n    set p.points to 0\n  otherwise\n    set p.points to p.points + by\n  done\n")},
        "hard", ["desk:scores"]),
    "scores-total": feature(
        SCORES_ABOUT, SCORES_DESK, "desk:apps:scores", A.SCORES, SCORES_TOTAL,
        "Row 23 must show 'Total: N' (from column 1), where N is all the players' points added up (0 when there "
        "are no players). Everything else must keep working as it does now.",
        SCORES_TOTAL_HIDDEN, ["row 23 shows the total of all points", "the total follows plus and minus", "five players",
                              "no players: the total is 0"],
        {"only when it opens": swap(swap(swap(SCORES_TOTAL, "  set total to 0\n  for each p in s.players\n    set total to total + p.points\n  done\n", ""),
                                         "'Total: ' & total", "'Total: ' & s.total"),
                                    "  give {players: players, picked: 1}\n",
                                    "  set total to 0\n  for each p in players\n    set total to total + p.points\n  done\n  give {players: players, picked: 1, total: total}\n"),
         "minus at 0 tosses the page": swap(SCORES_TOTAL, "  or when k is '-' and count(s.players) > 0\n",
                                             "  or when k is '-' and count(s.players) > 0 and item(s.players, s.picked).points is 0\n    toss('desk:scores')\n  or when k is '-' and count(s.players) > 0\n"),
         "three rows": swap(SCORES_TOTAL, "min(count(s.players), 20)", "min(count(s.players), 3)"),
         "on row 22": swap(SCORES_TOTAL, "show(23, 1, 'Total: ' & total)", "show(22, 1, 'Total: ' & total)"),
         "first three only": swap(SCORES_TOTAL, "  for each p in s.players\n    set total to total + p.points\n", "  for each p in slice(s.players, 1, 3)\n    set total to total + p.points\n"),
         "plus not saved": swap(SCORES_TOTAL, "    set s.players at s.picked to change(item(s.players, s.picked), 1)\n    save(s.players)\n", "    set s.players at s.picked to change(item(s.players, s.picked), 1)\n"),
         "below 0": swap(SCORES_TOTAL, "  set p.points to max(p.points + by, 0)\n", "  set p.points to p.points + by\n")},
        "easy", ["desk:scores"]),
    "recipes-find": feature(
        RECIPES_ABOUT, RECIPES_DESK, "desk:apps:recipes", A.RECIPES, RECIPES_FIND,
        "Add finding: in the list, typed characters (letters, digits, spaces and signs) make a search text, "
        "shown on row 23 as 'find: TEXT'; Backspace takes its last character away. Only the recipes whose name "
        "contains the search text are listed (from row 1, in the same order and form as now); with an empty "
        "search text all are listed. Each time the search text changes, the pick goes to row 1. Up, Down and "
        "Enter work on the listed recipes. If no recipe is listed, row 1 shows '(nothing found)'. While a "
        "recipe is shown, typing does nothing, and Esc goes back to the list with the search text and the pick as they were. Everything else must keep working as it does now.",
        RECIPES_FIND_HIDDEN, ["typing shows only the recipes whose name has the text", "backspace takes a letter away and the pick goes to row 1",
                              "typing while a recipe is shown does nothing", "esc goes back to the filtered list as it was",
                              "many recipes can be found", "nothing found"],
        {"up does nothing": swap(RECIPES_FIND, "  or when k is 'up' and s.picked > 1\n", "  or when k is 'up' and no\n"),
         "pick stays": swap(RECIPES_FIND, "      set s.find to f\n      set s.picked to 1\n", "      set s.find to f\n"),
         "names that start with it": swap(RECIPES_FIND, "    when item(s.names, n) has s.find\n", "    when starts(item(s.names, n), s.find)\n"),
         "enter opens the unfiltered one": swap(RECIPES_FIND, "    set n to item(sh, s.picked)\n", "    set n to s.picked\n"),
         "esc clears the search": swap(RECIPES_FIND, "    when k is 'esc'\n      set s.open to no\n", "    when k is 'esc'\n      set s.open to no\n      set s.find to ''\n"),
         "nine rows": swap(RECIPES_FIND, "min(count(sh), 22)", "min(count(sh), 9)")},
        "hard", []),
    "recipes-fav": feature(
        RECIPES_ABOUT, RECIPES_DESK, "desk:apps:recipes", A.RECIPES, RECIPES_FAV,
        "Add favourites, kept on the page desk:favourites (one recipe name per line). When the app opens it "
        "reads that page, if it exists. In the list, a favourite is shown with ' *' after its name. Ctrl+F in "
        "the list makes the picked recipe a favourite (its name is added at the end of the page) or, if it is "
        "one already, takes it away from the page; the page is saved at once. With an empty list Ctrl+F does "
        "nothing, and so does Ctrl+F while a recipe is shown. Everything else must keep working as it does now.",
        RECIPES_FAV_HIDDEN, ["favourites are read from desk:favourites and marked with *",
                             "ctrl+f adds the picked recipe at the end, and again takes it away",
                             "the first favourite makes the page; the last one leaves it empty"],
        {"does not save": swap(RECIPES_FAV, "    write('desk:favourites', join(s.favs, newline()))\n", ""),
         "does not read the page": swap(RECIPES_FAV, "    set favs to lines(read('desk:favourites'))\n", "    set favs to []\n"),
         "adds at the start": swap(RECIPES_FAV, "      add name to s.favs\n", "      set s.favs to reverse(s.favs)\n      add name to s.favs\n      set s.favs to reverse(s.favs)\n"),
         "marks the recipe page": swap(RECIPES_FAV, "    write('desk:favourites', join(s.favs, newline()))\n",
                                       "    write('desk:favourites', join(s.favs, newline()))\n    write('desk:recipes:' & name, name & ' *')\n")},
        "medium", ["desk:favourites"]),
    "agenda-today": feature(
        AGENDA_ABOUT, AGENDA_DESK, "desk:apps:agenda", A.AGENDA, AGENDA_TODAY,
        "Add a key: t goes back to today (the day the app opened on). Everything else must keep working as it "
        "does now.",
        AGENDA_TODAY_HIDDEN, ["t goes back to today"],
        {"left does nothing": swap(AGENDA_TODAY, "    set s.day to add_days(s.day, -1)\n", "    set s.day to s.day\n"),
         "t does nothing": swap(AGENDA_TODAY, "    set s.day to s.today\n", "    set s.day to s.day\n"),
         "t goes a day back": swap(AGENDA_TODAY, "    set s.day to s.today\n", "    set s.day to add_days(s.day, -1)\n")},
        "easy", []),
    "agenda-week": feature(
        AGENDA_ABOUT, AGENDA_DESK, "desk:apps:agenda", A.AGENDA, AGENDA_WEEK,
        "Add a week view: the key w switches between the day view (as now) and the week view, and w again goes "
        "back. The week view still shows the day on row 1 as now; then, from row 3, it shows 7 rows, one for "
        "each day starting with the shown day: 'DATE WEEKDAY: N', for example '2026-11-20 friday: 2', where N is "
        "the number of events on that day. Left and Right still change the shown day by one, in both views. "
        "Everything else must keep working as it does now.",
        AGENDA_WEEK_HIDDEN, ["w shows the week: one row per day with its number of events", "right moves the week, and w goes back to the day",
                             "every day of the week is counted, and left moves the week back"],
        {"starts tomorrow": swap(AGENDA_WEEK, "      set day to add_days(s.day, d)\n", "      set day to add_days(s.day, d + 1)\n"),
         "w does not go back": swap(AGENDA_WEEK, "    set s.week to not s.week\n", "    set s.week to yes\n"),
         "six days": swap(AGENDA_WEEK, "    for each d in range(0, 6)\n", "    for each d in range(0, 5)\n"),
         "sixth day not counted": swap(AGENDA_WEEK, "        when e.date is day\n", "        when e.date is day and d is not 5\n"),
         "left ignored in the week": swap(AGENDA_WEEK, "  or when k is 'left'\n", "  or when k is 'left' and not s.week\n")},
        "medium", []),
    # ---- fixes
    "shop-fix-enter": fix(
        SHOP_ABOUT, SHOP_DESK, "desk:apps:shop", SHOP_X1_BUG, A.SHOP,
        "Bug report: pressing Enter when nothing is typed adds an empty item to the list (and saves it). Enter "
        "with nothing typed must do nothing. Fix it, and don't change anything else.",
        SHOP_X1_HIDDEN, ["enter with nothing typed does nothing", "enter with nothing typed on an empty list"],
        {"hides empty items": swap(SHOP_X1_BUG, "    show(n, 1, mark & item(s.items, n))\n", "    when item(s.items, n) is not ''\n      show(n, 1, mark & item(s.items, n))\n    done\n"),
         "enter never adds": swap(SHOP_X1_BUG, "  or when k is 'enter'\n", "  or when k is 'enter' and no\n")},
        "easy", ["desk:shop"]),
    "shop-fix-up": fix(
        SHOP_ABOUT, SHOP_DESK, "desk:apps:shop", SHOP_X2_BUG, A.SHOP,
        "Bug report: on the first item, Up makes the pick disappear (no item has '>' any more), and then Delete "
        "makes the app stop with a mistake. Up on the first item must change nothing. Fix it, and don't change "
        "anything else.",
        SHOP_X2_HIDDEN, ["up on the first item changes nothing", "up after down goes back to the first item"],
        {"only delete made safe": swap(SHOP_X2_BUG, "  or when k is 'delete' and count(s.items) > 0\n", "  or when k is 'delete' and count(s.items) > 0 and s.picked > 0\n"),
         "up goes to the last": swap(SHOP_X2_BUG, "  when k is 'up' and s.picked > 0\n    set s.picked to s.picked - 1\n",
                                     "  when k is 'up' and s.picked > 1\n    set s.picked to s.picked - 1\n  or when k is 'up'\n    set s.picked to count(s.items)\n")},
        "easy", ["desk:shop"]),
    "scores-fix-minus": fix(
        SCORES_ABOUT, SCORES_DESK, "desk:apps:scores", SCORES_X3_BUG, A.SCORES,
        "Bug report: minus takes a player's points below 0 (a player can get -1). Points must never go below 0: "
        "minus on a player with 0 points changes nothing. Fix it, and don't change anything else.",
        SCORES_X3_HIDDEN, ["minus at 0 changes nothing", "minus never goes below 0"],
        {"only shown as 0": swap(SCORES_X3_BUG, "    show(n, 1, mark & p.name & ': ' & p.points)\n", "    show(n, 1, mark & p.name & ': ' & max(p.points, 0))\n"),
         "stops at 1": swap(SCORES_X3_BUG, "  set p.points to p.points + by\n", "  set p.points to max(p.points + by, 1)\n"),
         "minus resets": swap(SCORES_X3_BUG, "  set p.points to p.points + by\n", "  when by < 0\n    set p.points to 0\n  otherwise\n    set p.points to p.points + by\n  done\n")},
        "easy", ["desk:scores"]),
    "scores-fix-save": fix(
        SCORES_ABOUT, SCORES_DESK, "desk:apps:scores", SCORES_X4_BUG, A.SCORES,
        "Bug report: after + or -, the scores page has lines like 'Ana:4' instead of 'Ana: 4', and the next time "
        "the app opens it stops with a mistake. The page must keep the form 'Name: points'. Fix it, and don't "
        "change anything else.",
        SCORES_X4_HIDDEN, ["the page keeps the Name: points form", "the app opens again after a change",
                           "picking and changing points still work", "minus takes one point away", "minus never goes below 0"],
        {"reads both forms, saves wrong": swap(SCORES_X4_BUG, "      set parts to split(l, ': ')\n", "      set parts to split(replace(l, ': ', ':'), ':')\n"),
         "two spaces": swap(SCORES_X4_BUG, "    add p.name & ':' & p.points to out\n", "    add p.name & ':  ' & p.points to out\n")},
        "easy", ["desk:scores"]),
    "recipes-fix-nine": fix(
        RECIPES_ABOUT, RECIPES_DESK, "desk:apps:recipes", RECIPES_X5_BUG, A.RECIPES,
        "Bug report: with more than 9 recipes, only the first 9 are listed; Down goes on to the others, but they "
        "are not shown. Every recipe must be listed, one per row (as many as fit on the screen, up to row 23). "
        "Fix it, and don't change anything else.",
        RECIPES_X5_HIDDEN, ["all recipes are listed, up to row 23", "eleven recipes", "twelve recipes are all listed"],
        {"twenty rows": swap(RECIPES_X5_BUG, "min(count(s.names), 9)", "min(count(s.names), 20)"),
         "twelve rows": swap(RECIPES_X5_BUG, "min(count(s.names), 9)", "min(count(s.names), 12)")},
        "easy", []),
    "recipes-fix-esc": fix(
        RECIPES_ABOUT, RECIPES_DESK, "desk:apps:recipes", RECIPES_X6_BUG, A.RECIPES,
        "Bug report: while a recipe is shown, Esc closes the whole app. It must go back to the list, as the list "
        "was before Enter (the same recipe picked). Fix it, and don't change anything else.",
        RECIPES_X6_HIDDEN, ["esc in a recipe goes back to the list, as it was", "opening a recipe and going back still work",
                            "down and up move the pick"],
        {"pick goes to row 1": swap(RECIPES_X6_BUG, "      close()\n", "      set s.open to no\n      set s.picked to 1\n"),
         "esc does nothing": swap(RECIPES_X6_BUG, "      close()\n", "      set s.open to yes\n")},
        "easy", []),
    "agenda-fix-order": fix(
        AGENDA_ABOUT, AGENDA_DESK, "desk:apps:agenda", AGENDA_X7_BUG, A.AGENDA,
        "Bug report: the events of a day are shown in the order they were made, not by time. They must be shown "
        "by time, the earliest first. Fix it, and don't change anything else.",
        AGENDA_X7_HIDDEN, ["the events of a day are shown by time, earliest first", "the day and its events are still shown, and right still works"],
        {"latest first": swap(AGENDA_X7_BUG, "  when count(rows) is 0\n", "  set rows to reverse(sort(rows))\n  when count(rows) is 0\n"),
         "turned round": swap(AGENDA_X7_BUG, "  when count(rows) is 0\n", "  set rows to reverse(rows)\n  when count(rows) is 0\n")},
        "easy", []),
    "agenda-fix-left": fix(
        AGENDA_ABOUT, AGENDA_DESK, "desk:apps:agenda", AGENDA_X8_BUG, A.AGENDA,
        "Bug report: Left goes to the next day, like Right. It must go to the day before. Fix it, and don't change "
        "anything else.",
        AGENDA_X8_HIDDEN, ["left goes to the day before", "right then left comes back to the same day"],
        {"two days back": swap(AGENDA_X8_BUG, "  or when k is 'left'\n    set s.day to add_days(s.day, 1)\n", "  or when k is 'left'\n    set s.day to add_days(s.day, -2)\n"),
         "left does nothing": swap(AGENDA_X8_BUG, "  or when k is 'left'\n    set s.day to add_days(s.day, 1)\n", "  or when k is 'left'\n    set s.day to s.day\n")},
        "easy", []),
}

# the recipes the hidden tests pretend (recipes() above) all hold 'about NAME': none may change
RECIPES_KEPT = """
  when exists('desk:recipes')
    for each p in pages('desk:recipes')
      check page_now(p) is 'about ' & last(split(p, ':'))
    done
  done"""


def guarded(hidden, tops, extra=""):
    """Every test of a hidden page also checks, at its end, that nothing was done that nobody asked
    for: no top-level pages but the task's own, no mails, no events (and `extra`)."""
    out = []
    for line in hidden.split(NL):
        if line == "done":
            out.extend((only_pages(*tops) + NO_EFFECTS + extra).strip(NL).split(NL))
        out.append(line)
    return NL.join(out)


# the build tasks (tools/tasks_bench_build.py)
import tasks_bench_build as _build  # noqa: E402

TASKS.update(_build.TASKS)
for _id, _t in TASKS.items():
    _t["hidden"] = guarded(_t["hidden"], [d[len("desk:"):] for d in _t["data"] if d.count(":") == 1],
                           RECIPES_KEPT if _id.startswith("recipes-") else "")
