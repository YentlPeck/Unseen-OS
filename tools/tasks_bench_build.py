"""The build tasks of experiment 5 (joined into TASKS by tools/tasks_bench.py): make an app
from a description. Each description says every behaviour the hidden tests check."""

NL = "\n"
BASE = "#today 2026-11-20\n#drawer apps\n"
CLOSE = " Every other key changes nothing (also Esc: only Ctrl+Q closes the app)."


def lines_of(*items):
    return " & newline() & ".join(f"'{i}'" for i in items)


def swap(text, old, new):
    assert text.count(old) == 1, (old, text.count(old))
    return text.replace(old, new)


NO_EFFECTS = "\n  check sent() is []\n  check events() is []"


def only_pages(*names):
    """Test lines: the test world's top-level pages are only these (no page written unasked)."""
    allowed = "[" + ", ".join(f"'{n}'" for n in names) + "]"
    return f"\n  for each p in pages('desk')\n    check {allowed} has p\n  done"


def make(path):
    return (f"Make an app on the page {path} (the drawer desk:apps is there already). ")


# ------------------------------------------------------------------------------- light --
LIGHT = """
app 'Light'
task start()
  set on to no
  when exists('desk:light')
    set on to read('desk:light') is 'on'
  done
  give on
done
task key(on, k)
  when k is ' '
    set on to not on
    when on
      write('desk:light', 'on')
    otherwise
      write('desk:light', 'off')
    done
  done
  give on
done
task draw(on)
  when on
    show(1, 1, 'Light: on')
  otherwise
    show(1, 1, 'Light: off')
  done
done
"""
LIGHT_HIDDEN = """
test 'it starts off without a page; space switches and saves'
  launch 'desk:apps:light'
  check screen_line(1) is 'Light: off'
  press ' '
  check screen_line(1) is 'Light: on'
  check page_now('desk:light') is 'on'
  press ' '
  check screen_line(1) is 'Light: off'
  check page_now('desk:light') is 'off'
done

test 'it starts on when the page says on'
  pretend 'desk:light' holds 'on'
  launch 'desk:apps:light'
  check screen_line(1) is 'Light: on'
  press ' '
  check page_now('desk:light') is 'off'
done

test 'any other page text means off'
  pretend 'desk:light' holds 'dim'
  launch 'desk:apps:light'
  check screen_line(1) is 'Light: off'
done

test 'other keys change nothing'
  pretend 'desk:light' holds 'on'
  launch 'desk:apps:light'
  press 'x'
  press 'enter'
  press 'esc'
  press 'o'
  check screen_line(1) is 'Light: on'
  check page_now('desk:light') is 'on'""" + only_pages("light") + NO_EFFECTS + """
  check app_open()
done
"""

# ------------------------------------------------------------------------------- match --
MATCH = """
app 'Match'
task start()
  give {a: 0, b: 0}
done
task key(s, k)
  when k is 'a'
    set s.a to s.a + 1
  or when k is 'b'
    set s.b to s.b + 1
  or when k is 'r'
    set s.a to 0
    set s.b to 0
  or when k is 'enter'
    write('desk:match', 'A: ' & s.a & newline() & 'B: ' & s.b)
  done
  give s
done
task draw(s)
  show(1, 1, 'A: ' & s.a)
  show(2, 1, 'B: ' & s.b)
done
"""
MATCH_HIDDEN = """
test 'a and b give points, enter saves both lines'
  launch 'desk:apps:match'
  check screen_line(1) is 'A: 0'
  check screen_line(2) is 'B: 0'
  press 'a'
  press 'b'
  press 'a'
  press 'a'
  check screen_line(1) is 'A: 3'
  check screen_line(2) is 'B: 1'
  check page_now('desk:match') is nothing()
  press 'enter'
  check page_now('desk:match') is """ + lines_of("A: 3", "B: 1") + """
done

test 'r sets both to 0'
  launch 'desk:apps:match'
  press 'a'
  press 'b'
  press 'b'
  press 'r'
  check screen_line(1) is 'A: 0'
  check screen_line(2) is 'B: 0'
  press 'b'
  press 'enter'
  check page_now('desk:match') is """ + lines_of("A: 0", "B: 1") + only_pages("match") + NO_EFFECTS + """
done

test 'it always starts at 0 and 0'
  pretend 'desk:match' holds """ + lines_of("A: 5", "B: 9") + """
  launch 'desk:apps:match'
  check screen_line(1) is 'A: 0'
  check screen_line(2) is 'B: 0'
done

test 'other keys change nothing'
  launch 'desk:apps:match'
  press 'a'
  press 'c'
  press 'up'
  press 'esc'
  press ' '
  check screen_line(1) is 'A: 1'
  check screen_line(2) is 'B: 0'
  check app_open()
done
"""

# ------------------------------------------------------------------------------- words --
WORDS = """
app 'Words'
task start()
  give ''
done
task key(t, k)
  when k is 'enter'
    when t is not ''
      add('desk:draft', t)
      give ''
    done
    give t
  done
  give edit_text(t, k)
done
task draw(t)
  show(1, 1, t)
  show(2, 1, 'Words: ' & count(words(t)))
done
"""
WORDS_HIDDEN = """
test 'the text and its number of words are shown'
  launch 'desk:apps:words'
  check screen_line(2) is 'Words: 0'
  press 'h'
  press 'i'
  press ' '
  press 'y'
  press 'o'
  press 'u'
  check screen_line(1) is 'hi you'
  check screen_line(2) is 'Words: 2'
  press 'backspace'
  press 'backspace'
  press 'backspace'
  check screen_line(1) is 'hi'
  check screen_line(2) is 'Words: 1'
done

test 'enter adds the text as a line of desk:draft and empties it'
  launch 'desk:apps:words'
  press 'g'
  press 'o'
  press 'enter'
  check page_now('desk:draft') is 'go'
  check screen_line(1) is ''
  check screen_line(2) is 'Words: 0'
  press 'n'
  press 'o'
  press 'w'
  press '!'
  press 'enter'
  check page_now('desk:draft') is """ + lines_of("go", "now!") + """
done

test 'a line is added at the end of an existing page'
  pretend 'desk:draft' holds """ + lines_of("dear Omar,", "hello") + """
  launch 'desk:apps:words'
  press 'b'
  press 'y'
  press 'e'
  press 'enter'
  check page_now('desk:draft') is """ + lines_of("dear Omar,", "hello", "bye") + """
done

test 'the line is saved as typed, spaces and all'
  launch 'desk:apps:words'
  press ' '
  press 'h'
  press 'i'
  press ' '
  press ' '
  press 'enter'
  check page_now('desk:draft') is ' hi  '""" + only_pages("draft") + NO_EFFECTS + """
done

test 'spaces in a row, and only spaces'
  launch 'desk:apps:words'
  press ' '
  press ' '
  check screen_line(2) is 'Words: 0'
  press 'a'
  press ' '
  press ' '
  press ' '
  press 'b'
  press ' '
  check screen_line(2) is 'Words: 2'
done

test 'enter with an empty text does nothing'
  launch 'desk:apps:words'
  press 'enter'
  check page_now('desk:draft') is nothing()
  press 'a'
  press 'backspace'
  press 'enter'
  check page_now('desk:draft') is nothing()
  check app_open()
done
"""

# ------------------------------------------------------------------------------- notes --
NOTES = """
app 'Notes'
task start()
  set names to []
  set firsts to []
  when exists('desk:notes')
    for each p in pages('desk:notes')
      add last(split(p, ':')) to names
      set ls to lines(read(p))
      when count(ls) is 0
        add '' to firsts
      otherwise
        add first(ls) to firsts
      done
    done
  done
  give {names: names, firsts: firsts, picked: 1}
done
task key(s, k)
  when k is 'up' and s.picked > 1
    set s.picked to s.picked - 1
  or when k is 'down' and s.picked < count(s.names)
    set s.picked to s.picked + 1
  done
  give s
done
task draw(s)
  when count(s.names) is 0
    show(1, 1, '(no notes)')
    give
  done
  for each n in range(1, min(count(s.names), 22))
    set mark to '  '
    when n is s.picked
      set mark to '> '
    done
    show(n, 1, mark & item(s.names, n))
  done
  show(23, 1, item(s.firsts, s.picked))
done
"""
NOTES_HIDDEN = """
test 'the notes are listed, the first line of the picked one on row 23'
  pretend 'desk:notes:groceries' holds """ + lines_of("buy eggs", "and milk") + """
  pretend 'desk:notes:ideas' holds 'paint the door'
  launch 'desk:apps:notes'
  check screen_line(1) is '> groceries'
  check screen_line(2) is '  ideas'
  check screen_line(23) is 'buy eggs'
  press 'down'
  check screen_line(2) is '> ideas'
  check screen_line(23) is 'paint the door'
  press 'down'
  check screen_line(2) is '> ideas'
  press 'up'
  press 'up'
  check screen_line(1) is '> groceries'
done

test 'the order is the order pages gives'
  pretend 'desk:notes:n10' holds 'ten'
  pretend 'desk:notes:n2' holds 'two'
  pretend 'desk:notes:n1' holds 'one'
  launch 'desk:apps:notes'
  check screen_line(1) is '> n1'
  check screen_line(2) is '  n2'
  check screen_line(3) is '  n10'
done

test 'an empty note shows an empty row 23'
  pretend 'desk:notes:blank' holds ''
  pretend 'desk:notes:full' holds 'text'
  launch 'desk:apps:notes'
  check screen_line(1) is '> blank'
  check screen_line(23) is ''
  press 'down'
  check screen_line(23) is 'text'
done

test 'no notes'
  launch 'desk:apps:notes'
  check screen_line(1) is '(no notes)'
  press 'down'
  press 'up'
  check app_open()
done

test 'it changes no pages'
  pretend 'desk:notes:a' holds 'one'
  pretend 'desk:notes:b' holds 'two'
  launch 'desk:apps:notes'
  press 'down'
  press 'enter'
  press 'x'
  press 'esc'
  press 'up'
  check pages('desk') is []
  check pages('desk:notes') is ['notes:a', 'notes:b']
  check page_now('desk:notes:a') is 'one'
  check page_now('desk:notes:b') is 'two'""" + NO_EFFECTS + """
done

test 'twelve notes are all listed'
""" + NL.join(f"  pretend 'desk:notes:t{i}' holds 'line {i}'" for i in range(1, 13)) + """
  launch 'desk:apps:notes'
  check screen_line(12) is '  t12'
  repeat 11 times
    press 'down'
  done
  check screen_line(12) is '> t12'
  check screen_line(23) is 'line 12'
done
"""

# -------------------------------------------------------------------------------- pick --
PICK = """
app 'Pick'
task start()
  set people to []
  set names to []
  for each p in pages('contacts')
    add {name: field(p, 'name'), mail: field(p, 'mail')} to people
    add field(p, 'name') to names
  done
  give {people: people, names: sort(names), picked: 1}
done
task mail_of(s, name)
  for each x in s.people
    when x.name is name
      give x.mail
    done
  done
  give ''
done
task key(s, k)
  when k is 'up' and s.picked > 1
    set s.picked to s.picked - 1
  or when k is 'down' and s.picked < count(s.names)
    set s.picked to s.picked + 1
  or when k is 'enter' and count(s.names) > 0
    write('desk:picked', mail_of(s, item(s.names, s.picked)))
  done
  give s
done
task draw(s)
  for each n in range(1, min(count(s.names), 23))
    set mark to '  '
    when n is s.picked
      set mark to '> '
    done
    show(n, 1, mark & item(s.names, n))
  done
done
"""


def contact(page, name, mail, extra=""):
    t = f"'name: {name}' & newline() & 'mail: {mail}'" + (f" & newline() & '{extra}'" if extra else "")
    return f"  pretend 'contacts:{page}' holds {t}"


PICK_HIDDEN = """
test 'the names are sorted; enter writes the mail address of the picked one'
""" + NL.join([contact("c1", "Omar Haddad", "omar", "phone: 555-0101"), contact("c2", "Bea Lund", "bea"),
               contact("c3", "Lina Berg", "lina", "note: piano")]) + """
  launch 'desk:apps:pick'
  check screen_line(1) is '> Bea Lund'
  check screen_line(2) is '  Lina Berg'
  check screen_line(3) is '  Omar Haddad'
  press 'down'
  press 'down'
  press 'enter'
  check page_now('desk:picked') is 'omar'
  press 'up'
  press 'enter'
  check page_now('desk:picked') is 'lina'
  check app_open()
done

test 'up at the top and down at the bottom stay'
""" + NL.join([contact("x", "Zed Ames", "zed"), contact("y", "Ada Moss", "ada")]) + """
  launch 'desk:apps:pick'
  press 'up'
  check screen_line(1) is '> Ada Moss'
  press 'down'
  press 'down'
  press 'down'
  check screen_line(2) is '> Zed Ames'
  press 'enter'
  check page_now('desk:picked') is 'zed'
done

test 'no contacts: nothing is listed and enter does nothing'
  launch 'desk:apps:pick'
  check screen_line(1) is ''
  press 'enter'
  check page_now('desk:picked') is nothing()
  check app_open()
done

test 'enter writes only desk:picked'
""" + contact("p1", "Ivo Kern", "ivo") + """
  launch 'desk:apps:pick'
  press 'enter'
  check pages('desk') is ['picked']
  check count(pages('contacts')) is 1
  check page_now('contacts:p1') is 'name: Ivo Kern' & newline() & 'mail: ivo'""" + NO_EFFECTS + """
done

test 'eleven contacts are all listed'
""" + NL.join(contact(f"q{i}", f"Name{i:02d}", f"m{i}") for i in range(1, 12)) + """
  launch 'desk:apps:pick'
  check screen_line(11) is '  Name11'
  repeat 10 times
    press 'down'
  done
  press 'enter'
  check page_now('desk:picked') is 'm11'
done
"""

# ------------------------------------------------------------------------------ budget --
BUDGET = """
app 'Budget'
task start()
  set amounts to []
  when exists('desk:spent')
    for each l in lines(read('desk:spent'))
      add number(l) to amounts
    done
  done
  give {amounts: amounts, typing: ''}
done
task save(amounts)
  write('desk:spent', join(amounts, newline()))
done
task key(s, k)
  when k is 'enter'
    when is_number(s.typing)
      add number(s.typing) to s.amounts
      set s.typing to ''
      save(s.amounts)
    done
  or when k is 'delete' and count(s.amounts) > 0
    set s.amounts to without(s.amounts, count(s.amounts))
    save(s.amounts)
  or when k is 'backspace' or '0123456789.' has k
    set s.typing to edit_text(s.typing, k)
  done
  give s
done
task draw(s)
  show(1, 1, 'Total: ' & sum(s.amounts))
  set r to 3
  for each a in s.amounts
    when r <= 22
      show(r, 1, a)
    done
    set r to r + 1
  done
  show(23, 1, 'add: ' & s.typing)
done
"""
BUDGET_HIDDEN = """
test 'the amounts are read, listed from row 3, and added up on row 1'
  pretend 'desk:spent' holds """ + lines_of("12", "3.5", "40") + """
  launch 'desk:apps:budget'
  check screen_line(1) is 'Total: 55.5'
  check screen_line(3) is '12'
  check screen_line(4) is '3.5'
  check screen_line(5) is '40'
  check screen_line(23) is 'add:'
done

test 'typing an amount and enter adds it and saves'
  pretend 'desk:spent' holds """ + lines_of("12", "3.5") + """
  launch 'desk:apps:budget'
  press '4'
  press 'x'
  press '.'
  press '2'
  press '5'
  check screen_line(23) is 'add: 4.25'
  press 'enter'
  check screen_line(1) is 'Total: 19.75'
  check screen_line(5) is '4.25'
  check screen_line(23) is 'add:'
  check page_now('desk:spent') is """ + lines_of("12", "3.5", "4.25") + """
done

test 'enter does nothing when the new amount is empty or not a number'
  launch 'desk:apps:budget'
  press 'enter'
  check page_now('desk:spent') is nothing()
  press '1'
  press '.'
  press '.'
  press '2'
  press 'enter'
  check page_now('desk:spent') is nothing()
  check screen_line(23) is 'add: 1..2'
  press 'backspace'
  press 'backspace'
  press 'backspace'
  press '7'
  press 'enter'
  check page_now('desk:spent') is '17'
  check screen_line(1) is 'Total: 17'
done

test 'many amounts are all listed'
  pretend 'desk:spent' holds """ + lines_of("1", "2", "3", "4", "5", "6") + """
  launch 'desk:apps:budget'
  check screen_line(8) is '6'
  check screen_line(1) is 'Total: 21'
done

test 'delete takes the last amount away and saves'
  pretend 'desk:spent' holds """ + lines_of("5", "6", "7") + """
  launch 'desk:apps:budget'
  press 'delete'
  check screen_line(1) is 'Total: 11'
  check screen_line(5) is ''
  check page_now('desk:spent') is """ + lines_of("5", "6") + """
  press 'delete'
  press 'delete'
  press 'delete'
  check screen_line(1) is 'Total: 0'
  check page_now('desk:spent') is ''""" + only_pages("spent") + NO_EFFECTS + """
  check app_open()
done
"""

# ------------------------------------------------------------------------------- inbox --
INBOX = """
app 'Inbox'
task start()
  set mails to []
  for each p in pages('mail:inbox')
    set m to {from: field(p, 'from'), subject: field(p, 'subject')}
    set m.body to body(p)
    add m to mails
  done
  give {mails: mails, picked: 1, open: no}
done
task key(s, k)
  when s.open
    when k is 'esc'
      set s.open to no
    done
  or when k is 'up' and s.picked > 1
    set s.picked to s.picked - 1
  or when k is 'down' and s.picked < count(s.mails)
    set s.picked to s.picked + 1
  or when k is 'enter' and count(s.mails) > 0
    set s.open to yes
  done
  give s
done
task draw(s)
  when s.open
    set m to item(s.mails, s.picked)
    show(1, 1, 'From: ' & m.from)
    show(2, 1, 'Subject: ' & m.subject)
    set r to 4
    for each l in lines(m.body)
      when r <= 23
        show(r, 1, l)
      done
      set r to r + 1
    done
    give
  done
  when count(s.mails) is 0
    show(1, 1, '(no mail)')
  done
  for each n in range(1, min(count(s.mails), 23))
    set m to item(s.mails, n)
    set mark to '  '
    when n is s.picked
      set mark to '> '
    done
    show(n, 1, mark & m.from & ': ' & m.subject)
  done
done
"""


def mail(page, frm, subject, *body):
    t = f"'from: {frm}' & newline() & 'to: me' & newline() & 'subject: {subject}' & newline() & ''"
    for b in body:
        t += f" & newline() & '{b}'"
    return f"  pretend 'mail:inbox:{page}' holds {t}"


INBOX_HIDDEN = """
test 'the list shows FROM: SUBJECT for every mail'
""" + NL.join([mail("m1", "omar", "Bread", "Fresh today."), mail("m2", "lina", "Lesson", "Can we move it?"),
               mail("m10", "mira", "Party", "Saturday at 8.")]) + """
  launch 'desk:apps:inbox'
  check screen_line(1) is '> omar: Bread'
  check screen_line(2) is '  lina: Lesson'
  check screen_line(3) is '  mira: Party'
done

test 'enter opens the picked mail; esc goes back with the same mail picked'
""" + NL.join([mail("m1", "omar", "Bread", "Fresh today."),
               mail("m2", "lina", "Lesson", "Hi,", "can we move it?", "Lina")]) + """
  launch 'desk:apps:inbox'
  press 'down'
  press 'enter'
  check screen_line(1) is 'From: lina'
  check screen_line(2) is 'Subject: Lesson'
  check screen_line(3) is ''
  check screen_line(4) is 'Hi,'
  check screen_line(5) is 'can we move it?'
  check screen_line(6) is 'Lina'
  press 'up'
  check screen_line(1) is 'From: lina'
  press 'esc'
  check screen_line(2) is '> lina: Lesson'
  press 'up'
  press 'enter'
  press 'down'
  check screen_line(1) is 'From: omar'
  press 'esc'
  check screen_line(1) is '> omar: Bread'
  check app_open()
  check pages('desk') is []
  check count(pages('mail:inbox')) is 2""" + NO_EFFECTS + """
done

test 'eleven mails are all listed'
""" + NL.join(mail(f"m{i}", f"p{i}", f"Note {i}", "x") for i in range(1, 12)) + """
  launch 'desk:apps:inbox'
  check screen_line(11) is '  p11: Note 11'
done

test 'esc in the list changes nothing'
""" + mail("m1", "omar", "Bread", "Fresh today.") + """
  launch 'desk:apps:inbox'
  press 'esc'
  check app_open()
  check screen_line(1) is '> omar: Bread'
done

test 'no mail'
  launch 'desk:apps:inbox'
  check screen_line(1) is '(no mail)'
  press 'enter'
  check screen_line(1) is '(no mail)'
  check app_open()
done
"""

# -------------------------------------------------------------------------------- plan --
PLAN = """
app 'Plan'
task hours()
  set out to []
  for each h in range(8, 17)
    when h < 10
      add '0' & h & ':00' to out
    otherwise
      add h & ':00' to out
    done
  done
  give out
done
task start()
  set notes to []
  repeat 10 times
    add '' to notes
  done
  when exists('desk:plan')
    for each l in lines(read('desk:plan'))
      set n to position(hours(), slice(l, 1, 5))
      when n > 0
        set notes at n to slice(l, 7, count(l))
      done
    done
  done
  give {notes: notes, picked: 1, typing: ''}
done
task save(s)
  set out to []
  for each n in range(1, 10)
    when item(s.notes, n) is not ''
      add item(hours(), n) & ' ' & item(s.notes, n) to out
    done
  done
  write('desk:plan', join(out, newline()))
done
task key(s, k)
  when k is 'up' and s.picked > 1
    set s.picked to s.picked - 1
  or when k is 'down' and s.picked < 10
    set s.picked to s.picked + 1
  or when k is 'enter'
    set s.notes at s.picked to s.typing
    set s.typing to ''
    save(s)
  otherwise
    set s.typing to edit_text(s.typing, k)
  done
  give s
done
task draw(s)
  for each n in range(1, 10)
    set mark to '  '
    when n is s.picked
      set mark to '> '
    done
    set row to mark & item(hours(), n)
    when item(s.notes, n) is not ''
      set row to row & ' ' & item(s.notes, n)
    done
    show(n, 1, row)
  done
  show(23, 1, 'note: ' & s.typing)
done
"""
PLAN_HIDDEN = """
test 'ten hours are shown, with the notes read from desk:plan'
  pretend 'desk:plan' holds """ + lines_of("10:00 dentist", "15:00 call Omar") + """
  launch 'desk:apps:plan'
  check screen_line(1) is '> 08:00'
  check screen_line(3) is '  10:00 dentist'
  check screen_line(8) is '  15:00 call Omar'
  check screen_line(10) is '  17:00'
  check screen_line(11) is ''
  check screen_line(23) is 'note:'
done

test 'typing and enter set the picked hour and save the page in time order'
  pretend 'desk:plan' holds '16:00 gym'
  launch 'desk:apps:plan'
  press 'down'
  press 'r'
  press 'u'
  press 'n'
  check screen_line(23) is 'note: run'
  press 'enter'
  check screen_line(2) is '> 09:00 run'
  check screen_line(23) is 'note:'
  check page_now('desk:plan') is """ + lines_of("09:00 run", "16:00 gym") + """
done

test 'enter with nothing typed clears the note'
  pretend 'desk:plan' holds """ + lines_of("08:00 tea", "12:00 lunch") + """
  launch 'desk:apps:plan'
  press 'enter'
  check screen_line(1) is '> 08:00'
  check page_now('desk:plan') is '12:00 lunch'
  repeat 4 times
    press 'down'
  done
  press 'enter'
  check page_now('desk:plan') is ''
done

test 'up and down stay between 08:00 and 17:00'
  launch 'desk:apps:plan'
  press 'up'
  check screen_line(1) is '> 08:00'
  repeat 12 times
    press 'down'
  done
  check screen_line(10) is '> 17:00'
  press 'x'
  press 'enter'
  check page_now('desk:plan') is '17:00 x'
  check app_open()
done

test 'up picks the hour before'
  launch 'desk:apps:plan'
  press 'down'
  press 'down'
  press 'down'
  press 'up'
  check screen_line(3) is '> 10:00'
  press 'b'
  press 'enter'
  check page_now('desk:plan') is '10:00 b'""" + only_pages("plan") + NO_EFFECTS + """
done

test 'backspace takes back what was typed'
  launch 'desk:apps:plan'
  press 'c'
  press 'a'
  press 't'
  press 'backspace'
  press 'r'
  check screen_line(23) is 'note: car'
  press 'enter'
  check page_now('desk:plan') is '08:00 car'
  press 'down'
  press 'z'
  press 'backspace'
  press 'enter'
  check page_now('desk:plan') is '08:00 car'
  check pages('desk') is ['plan']
done
"""


def build(path, level, request, solution, hidden, wrong, data):
    return {"kind": "build", "level": level, "start": BASE, "request": make(path) + request,
            "allowed": ("desk:apps:",) + tuple(data), "data": data, "solution": None, "app": (path, solution),
            "hidden": hidden, "f2p": None, "wrong": {k: {path: v} for k, v in wrong.items()}}


TASKS = {
    "build-light": build(
        "desk:apps:light", "easy",
        "It is a light switch. Row 1 shows exactly 'Light: on' or 'Light: off' from column 1. The space key "
        "switches the light, and saves the page desk:light at once, holding just 'on' or 'off'. When the app "
        "opens, the light is on if desk:light holds 'on', otherwise it is off." + CLOSE,
        LIGHT, LIGHT_HIDDEN,
        {"does not save": swap(LIGHT, "    when on\n      write('desk:light', 'on')\n    otherwise\n      write('desk:light', 'off')\n    done\n", ""),
         "always starts off": swap(LIGHT, "    set on to read('desk:light') is 'on'\n", "    set on to no\n"),
         "enter switches too": swap(LIGHT, "  when k is ' '\n", "  when k is ' ' or k is 'enter'\n")},
        ["desk:light"]),
    "build-match": build(
        "desk:apps:match", "easy",
        "It keeps the score of a match between A and B. Row 1 shows 'A: N' and row 2 'B: M' (from column 1), "
        "the points of A and of B. The key a gives A a point, the key b gives B a point, and the key r sets both "
        "to 0. Enter saves the page desk:match with exactly two lines, 'A: N' and 'B: M'. The app always starts "
        "at 0 and 0." + CLOSE,
        MATCH, MATCH_HIDDEN,
        {"r resets only A": swap(MATCH, "    set s.a to 0\n    set s.b to 0\n", "    set s.a to 0\n"),
         "one line": swap(MATCH, "'A: ' & s.a & newline() & 'B: ' & s.b", "'A: ' & s.a & ' B: ' & s.b"),
         "starts from the page": swap(MATCH, "task start()\n  give {a: 0, b: 0}\n",
                                      "task start()\n  when exists('desk:match')\n    set l to lines(read('desk:match'))\n"
                                      "    give {a: number(slice(item(l, 1), 4, 9)), b: number(slice(item(l, 2), 4, 9))}\n  done\n  give {a: 0, b: 0}\n")},
        ["desk:match"]),
    "build-words": build(
        "desk:apps:words", "medium",
        "It is for writing short lines. Typed characters (letters, digits, spaces and signs) are added to a "
        "text, and Backspace takes the last character away. Row 1 shows the text and row 2 shows 'Words: N', "
        "where N is the number of words in the text: the parts of it between spaces that are not empty (so 'a  b ' has 2 words and '  ' has 0). Enter adds the text as a "
        "new line at the end of the page desk:draft (the page is made if it isn't there yet), and then the text "
        "is empty again; when the text is empty, Enter does nothing. The app starts with an empty text.",
        WORDS, WORDS_HIDDEN,
        {"replaces the page": swap(WORDS, "      add('desk:draft', t)\n", "      write('desk:draft', t)\n"),
         "does not empty the text": swap(WORDS, "      add('desk:draft', t)\n      give ''\n", "      add('desk:draft', t)\n"),
         "counts characters": swap(WORDS, "count(words(t))", "count(t)"),
         "saves trimmed": swap(WORDS, "      add('desk:draft', t)\n", "      add('desk:draft', trim(t))\n"),
         "counts empty parts": swap(WORDS, "count(words(t))", "count(split(trim(t), ' '))")},
        ["desk:draft"]),
    "build-notes": build(
        "desk:apps:notes", "medium",
        "It shows the notes: the pages in the drawer desk:notes. It lists their names (the last part of the "
        "path, for example groceries for desk:notes:groceries), in the order pages(...) gives them, from row 1: "
        "the picked note as '> NAME', the others as '  NAME'. Up and Down change the picked note (the first one "
        "is picked when the app opens). Up on the first note and Down on the last one change nothing. Row 23 "
        "shows the first line of the picked note's text (an empty row if "
        "its text is empty). If there are no notes (or there is no drawer desk:notes), row 1 shows '(no notes)'. "
        "The app changes no pages.",
        NOTES, NOTES_HIDDEN,
        {"sorted as text": swap(NOTES, "    for each p in pages('desk:notes')\n", "    for each p in sort(pages('desk:notes'))\n"),
         "no (no notes)": swap(NOTES, "    show(1, 1, '(no notes)')\n", ""),
         "nine rows": swap(NOTES, "min(count(s.names), 22)", "min(count(s.names), 9)"),
         "last line": swap(NOTES, "        add first(ls) to firsts\n", "        add last(ls) to firsts\n")},
        []),
    "build-pick": build(
        "desk:apps:pick", "medium",
        "It lists the names of all contacts: for every page in the drawer contacts, the text after 'name:' on "
        "that page. The names are sorted alphabetically and shown from row 1, the picked one as '> NAME', the "
        "others as '  NAME'. Up and Down change the picked name (the first is picked when the app opens). Up on "
        "the first name and Down on the last one change nothing. Enter "
        "writes the mail address of the picked contact (the text after 'mail:' on its page) to the page "
        "desk:picked, and nothing else. With no contacts, nothing is listed and Enter does nothing.",
        PICK, PICK_HIDDEN,
        {"not sorted": swap(PICK, "names: sort(names)", "names: names"),
         "nine rows": swap(PICK, "min(count(s.names), 23)", "min(count(s.names), 9)"),
         "writes the name": swap(PICK, "    write('desk:picked', mail_of(s, item(s.names, s.picked)))\n", "    write('desk:picked', item(s.names, s.picked))\n"),
         "enter on no contacts": swap(PICK, "  or when k is 'enter' and count(s.names) > 0\n", "  or when k is 'enter'\n").replace(
             "    write('desk:picked', mail_of(s, item(s.names, s.picked)))\n",
             "    when count(s.names) > 0\n      write('desk:picked', mail_of(s, item(s.names, s.picked)))\n    otherwise\n      write('desk:picked', '')\n    done\n")},
        ["desk:picked"]),
    "build-budget": build(
        "desk:apps:budget", "medium",
        "It keeps a list of amounts spent on the page desk:spent (one number per line); when the app opens it "
        "reads that page, if it exists. Row 1 shows 'Total: N', all the amounts added up, and from row 3 the "
        "amounts are listed, one per row, in the order of the page. Typing digits and '.' builds a new amount, "
        "shown on row 23 as 'add: TEXT'; Backspace takes its last character away, and other characters are not "
        "added. Enter adds the new amount at the end of the list if it is a number, saves the page at once (one "
        "number per line) and empties the new amount; if it is empty or not a number, Enter does nothing. Delete "
        "takes the last amount away and saves the page. Numbers are shown and saved as Unseen Script writes them "
        "(for example 2.5, 10 and 4.25).",
        BUDGET, BUDGET_HIDDEN,
        {"letters are added": swap(BUDGET, "  or when k is 'backspace' or '0123456789.' has k\n", "  otherwise\n"),
         "three rows": swap(BUDGET, "    when r <= 22\n", "    when r <= 5\n"),
         "delete takes the first": swap(BUDGET, "    set s.amounts to without(s.amounts, count(s.amounts))\n", "    set s.amounts to without(s.amounts, 1)\n"),
         "does not save on delete": swap(BUDGET, "    set s.amounts to without(s.amounts, count(s.amounts))\n    save(s.amounts)\n", "    set s.amounts to without(s.amounts, count(s.amounts))\n")},
        ["desk:spent"]),
    "build-inbox": build(
        "desk:apps:inbox", "hard",
        "It is a mail reader for the drawer mail:inbox. The list shows one row per mail, from row 1, in the "
        "order pages(...) gives them: 'FROM: SUBJECT' (the texts after 'from:' and 'subject:' on the mail's "
        "page), the picked one starting with '> ' and the others with '  '. Up and Down change the picked mail "
        "(the first is picked when the app opens). Enter opens it: row 1 shows 'From: FROM', row 2 'Subject: "
        "SUBJECT', and from row 4 on the lines of its body (the text after the first empty line), one per row; "
        "while a mail is open, only Esc does something: it goes back to the list, with the same mail picked. In "
        "the list, Esc changes nothing. With no mails, row 1 shows '(no mail)' and Enter does nothing. The app "
        "changes no pages.",
        INBOX, INBOX_HIDDEN,
        {"nine rows": swap(INBOX, "min(count(s.mails), 23)", "min(count(s.mails), 9)"),
         "body from row 3": swap(INBOX, "    set r to 4\n", "    set r to 3\n"),
         "esc closes the app": swap(INBOX, "    when k is 'esc'\n      set s.open to no\n", "    when k is 'esc'\n      close()\n"),
         "down works in a mail": swap(INBOX, "      set s.open to no\n    done\n  or when k is 'up'",
                                      "      set s.open to no\n    or when k is 'down' and s.picked < count(s.mails)\n"
                                      "      set s.picked to s.picked + 1\n    done\n  or when k is 'up'"),
         "writes a page": swap(INBOX, "  give {mails: mails, picked: 1, open: no}\n", "  write('desk:read', 'yes')\n  give {mails: mails, picked: 1, open: no}\n"),
         "up works in a mail": swap(INBOX, "  when s.open\n    when k is 'esc'\n      set s.open to no\n    done\n  or when k is 'up'",
                                    "  when s.open and k is 'esc'\n    set s.open to no\n  or when k is 'up'")},
        []),
    "build-plan": build(
        "desk:apps:plan", "hard",
        "It is a day planner for the hours 08:00 to 17:00. From row 1 it shows 10 rows, one for each hour: "
        "'HH:00 NOTE' (for example '10:00 dentist'), or just 'HH:00' when that hour has no note; the picked "
        "hour's row starts with '> ', the others with '  '. Up and Down change the picked hour (08:00 is picked "
        "when the app opens); Up on 08:00 and Down on 17:00 change nothing. Typed characters build a new note, shown on row 23 as 'note: TEXT'; Backspace "
        "takes its last character away. Enter sets the picked hour's note to the typed text (an empty text "
        "clears the note), empties the typed text, and saves the page desk:plan at once: one line 'HH:00 NOTE' "
        "for each hour that has a note, in time order (an empty page when no hour has one). When the app opens "
        "it reads desk:plan, if it exists.",
        PLAN, PLAN_HIDDEN,
        {"saves the empty hours too": swap(PLAN, "    when item(s.notes, n) is not ''\n      add item(hours(), n) & ' ' & item(s.notes, n) to out\n    done\n",
                                           "    add item(hours(), n) & ' ' & item(s.notes, n) to out\n"),
         "up does nothing": swap(PLAN, "  when k is 'up' and s.picked > 1\n", "  when k is 'up' and no\n"),
         "does not read the page": swap(PLAN, "  when exists('desk:plan')\n", "  when no\n"),
         "enter keeps the typed text": swap(PLAN, "    set s.notes at s.picked to s.typing\n    set s.typing to ''\n", "    set s.notes at s.picked to s.typing\n")},
        ["desk:plan"]),
}
for t in TASKS.values():
    path, app = t.pop("app")
    t["solution"] = [f"insert {path} {i + 1} {line}" for i, line in enumerate(app.strip(NL).split(NL))]
    t["solved"] = {path: app}
