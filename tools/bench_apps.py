"""The base apps of experiment 5's feature and fix tasks (tools/tasks_bench.py), with their
visible test pages. Each is a small, working Unseen Script app."""

SHOP = """
app 'Shopping'
task start()
  set items to []
  when exists('desk:shop')
    set items to lines(read('desk:shop'))
  done
  give {items: items, picked: 1, typing: ''}
done
task save(items)
  write('desk:shop', join(items, newline()))
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
  title('Shopping - ' & count(s.items) & ' items')
  for each n in range(1, min(count(s.items), 20))
    set mark to '  '
    when n is s.picked
      set mark to '> '
    done
    show(n, 1, mark & item(s.items, n))
  done
  show(23, 1, 'new: ' & s.typing)
  help('Up/Down pick  Enter add  Delete remove')
done
"""

SHOP_TESTS = """
use 'desk:apps:shop'

test 'enter adds the typed item and saves the list'
  pretend 'desk:shop' holds 'rice'
  launch 'desk:apps:shop'
  press 'o'
  press 'i'
  press 'l'
  press 'enter'
  check app_state().items is ['rice', 'oil']
  check page_now('desk:shop') is 'rice' & newline() & 'oil'
  check screen_line(2) is '  oil'
done

test 'down picks the next item, delete removes it'
  pretend 'desk:shop' holds 'a' & newline() & 'b' & newline() & 'c'
  launch 'desk:apps:shop'
  press 'down'
  check screen_line(2) is '> b'
  press 'delete'
  check app_state().items is ['a', 'c']
  check screen_line(2) is '> c'
done
"""

SCORES = """
app 'Scores'
task start()
  set players to []
  when exists('desk:scores')
    for each l in lines(read('desk:scores'))
      set parts to split(l, ': ')
      add {name: item(parts, 1), points: number(item(parts, 2))} to players
    done
  done
  give {players: players, picked: 1}
done
task save(players)
  set out to []
  for each p in players
    add p.name & ': ' & p.points to out
  done
  write('desk:scores', join(out, newline()))
done
task change(p, by)
  set p.points to max(p.points + by, 0)
  give p
done
task key(s, k)
  when k is 'up' and s.picked > 1
    set s.picked to s.picked - 1
  or when k is 'down' and s.picked < count(s.players)
    set s.picked to s.picked + 1
  or when k is '+' and count(s.players) > 0
    set s.players at s.picked to change(item(s.players, s.picked), 1)
    save(s.players)
  or when k is '-' and count(s.players) > 0
    set s.players at s.picked to change(item(s.players, s.picked), -1)
    save(s.players)
  done
  give s
done
task draw(s)
  title('Scores')
  for each n in range(1, min(count(s.players), 20))
    set p to item(s.players, n)
    set mark to '  '
    when n is s.picked
      set mark to '> '
    done
    show(n, 1, mark & p.name & ': ' & p.points)
  done
  help('Up/Down pick  + and - change the points')
done
"""

SCORES_TESTS = """
use 'desk:apps:scores'

test 'plus adds a point to the picked player and saves'
  pretend 'desk:scores' holds 'Ana: 3' & newline() & 'Bo: 5'
  launch 'desk:apps:scores'
  press 'down'
  press '+'
  check screen_line(2) is '> Bo: 6'
  check page_now('desk:scores') is 'Ana: 3' & newline() & 'Bo: 6'
done

test 'minus takes a point away'
  pretend 'desk:scores' holds 'Cy: 2'
  launch 'desk:apps:scores'
  press '-'
  check screen_line(1) is '> Cy: 1'
done
"""

RECIPES = """
app 'Recipes'
task start()
  set names to []
  set texts to []
  when exists('desk:recipes')
    for each p in pages('desk:recipes')
      add last(split(p, ':')) to names
      add read(p) to texts
    done
  done
  give {names: names, texts: texts, picked: 1, open: no}
done
task key(s, k)
  when s.open
    when k is 'esc'
      set s.open to no
    done
  or when k is 'up' and s.picked > 1
    set s.picked to s.picked - 1
  or when k is 'down' and s.picked < count(s.names)
    set s.picked to s.picked + 1
  or when k is 'enter' and count(s.names) > 0
    set s.open to yes
  done
  give s
done
task draw(s)
  when s.open
    title(item(s.names, s.picked))
    set r to 1
    for each l in lines(item(s.texts, s.picked))
      when r <= 23
        show(r, 1, l)
      done
      set r to r + 1
    done
    help('Esc back to the list')
  otherwise
    title('Recipes - ' & count(s.names))
    for each n in range(1, min(count(s.names), 23))
      set mark to '  '
      when n is s.picked
        set mark to '> '
      done
      show(n, 1, mark & item(s.names, n))
    done
    help('Up/Down pick  Enter open')
  done
done
"""

RECIPES_TESTS = """
use 'desk:apps:recipes'

test 'the recipes are listed by name'
  pretend 'desk:recipes:pancakes' holds 'flour, eggs, milk'
  pretend 'desk:recipes:soup' holds 'water, leeks'
  launch 'desk:apps:recipes'
  check screen_line(1) is '> pancakes'
  check screen_line(2) is '  soup'
  check screen_line(0) is 'Recipes - 2'
done

test 'enter shows the recipe, esc goes back'
  pretend 'desk:recipes:tea' holds 'boil water' & newline() & 'add tea'
  launch 'desk:apps:recipes'
  press 'enter'
  check screen_line(0) is 'tea'
  check screen_line(2) is 'add tea'
  press 'esc'
  check screen_line(1) is '> tea'
  check app_open()
done
"""

AGENDA = """
app 'Agenda'
task start()
  set events to []
  for each p in pages('calendar')
    add fields(p) to events
  done
  give {events: events, day: today()}
done
task key(s, k)
  when k is 'right'
    set s.day to add_days(s.day, 1)
  or when k is 'left'
    set s.day to add_days(s.day, -1)
  done
  give s
done
task draw(s)
  title('Agenda')
  show(1, 1, s.day & ' ' & weekday(s.day))
  set rows to []
  for each e in s.events
    when e.date is s.day
      add e.time & ' ' & e.title to rows
    done
  done
  set rows to sort(rows)
  when count(rows) is 0
    show(3, 1, '(no events)')
  done
  set r to 3
  for each l in rows
    when r <= 23
      show(r, 1, l)
    done
    set r to r + 1
  done
  help('Left/Right change the day')
done
"""

AGENDA_TESTS = """
use 'desk:apps:agenda'

test 'it shows the events of today, by time'
  pretend today is '2026-12-01'
  pretend 'calendar:e1' holds 'date: 2026-12-01' & newline() & 'time: 15:00' & newline() & 'title: Dentist' & newline() & 'with: '
  pretend 'calendar:e2' holds 'date: 2026-12-01' & newline() & 'time: 09:30' & newline() & 'title: Standup' & newline() & 'with: team'
  launch 'desk:apps:agenda'
  check screen_line(1) is '2026-12-01 tuesday'
  check screen_line(3) is '09:30 Standup'
  check screen_line(4) is '15:00 Dentist'
done

test 'right goes to the next day'
  pretend today is '2026-12-01'
  launch 'desk:apps:agenda'
  press 'right'
  check screen_line(1) is '2026-12-02 wednesday'
  check screen_line(3) is '(no events)'
done
"""
