# Unseen OS manual

## The basics

- **Pages** hold text (like files). **Drawers** hold pages and other drawers (like folders).
- The top drawer is called the **desk**.
- Names use the letters a-z and A-Z, digits, `-`, `_` and `.`, at most 24 characters, no
  spaces. `desk` can't be used as a name.
- A **path** joins names with `:`. Example: `letters:mira` is the page `mira` inside the drawer `letters`.
- The desk holds at most 5000 drawers and pages in all (the apps' drawers included), all pages
  together at most 1 MB of text, and a path is at most 120 characters long.

## Where you are

The prompt shows the drawer you are in, for example `desk:letters >> `.

Paths are read **from the drawer you are in**. If you are in `desk:lists` and type
`move shop letters`, the computer looks for the drawer `desk:lists:letters`.
To mean a place starting from the top, begin the path with `desk:`, like `desk:letters`.

## Typing commands

Type a command and press Enter. `guide` lists all commands; `guide <word>` explains one
(`guide do` opens the chore reference). The Up arrow brings back the
last command. Commands can be at most 200 characters long.

| Command | What it does |
|---|---|
| `guide [word]` | list all commands, or explain one |
| `here` | show which drawer you are in |
| `peek [drawer]` | list what is in a drawer (the current one if none is given) |
| `enter <drawer>` | go into a drawer |
| `leave` | go out to the drawer above |
| `read <page>` | show a page |
| `write <page> <text>` | make a page with this text; if the page exists, its text is **replaced**. The text is one line. |
| `add <page> <text>` | add one line to the end of an existing page |
| `build <drawer>` | make a new, empty drawer (the drawer it goes in must exist) |
| `rename <name> <new name>` | rename a page or drawer; it stays in the same drawer. The new name is a name, not a path. |
| `move <name> <drawer>` | move a page or drawer (with everything inside it) into another drawer |
| `copy <page> <new page>` | copy a page; the new page must not exist yet |
| `toss <name>` | throw away a page, or a drawer that is empty |
| `seek <word>` | list pages whose name or text contains the word (capitals don't matter) |
| `notes <page>` | open a page in the Notes app (a new page is made when you keep it) |
| `files` | open the Files app |
| `show <text>` | print text on the screen |
| `wipe` | clear the screen |
| `clock` | show the date and time |
| `about` | about this computer |
| `do <page>` | carry out the chore written on a page (see Chores below) |
| `mail` | open the Mail app |
| `contacts` | open the Contacts app |
| `calendar` | open the Calendar app |
| `settings` | open the Settings app |
| `calc` | open the Calculator app |
| `stamp <note>` | save the desk as it is now as a new numbered stamp (see History below) |
| `stamps` | list the stamps |
| `changes [page or drawer]` | show what changed since the stamp the desk is on |
| `compare <stamp> [stamp] [page or drawer]` | show what changed from one stamp to another, or to the desk now |
| `bring <page> <stamp>` | bring back one page as it was in a stamp |
| `rewind <stamp>` | put the whole desk back as it was in a stamp |

If a command can't be done, the computer explains why in yellow, and nothing is changed.

## Notes app (editing a page)

`notes <page>` opens the page full-screen. The top bar shows the page and the line number;
the bottom bar shows the keys.

- Type to insert text at the cursor. Arrow keys, Home and End move the cursor.
- Enter splits the line; Backspace and Delete remove characters (and can join lines).
- In Notes you can't type a line longer than 78 characters, or join two lines that together
  would be longer. (Longer lines made with `write` stay as they are.)
- Notes holds at most 3000 lines: a longer page can't be opened in Notes (`read` shows it).
- **Ctrl+K keeps (saves) the page.** The bottom bar then says "Kept."
- **Esc leaves.** If there are changes that were not kept, the first Esc only warns you; a
  second Esc leaves and throws the changes away.

## Files app (browsing)

`files` opens a list of the current drawer. Up/Down choose an item.

| Key | What it does |
|---|---|
| Enter | open the chosen drawer, or open the chosen page in Notes |
| Backspace | go up to the drawer above |
| N | rename the chosen item (type the new name, then Enter) |
| M | move the chosen item (type the drawer, then Enter) |
| T | toss the chosen item (then Ctrl+K to confirm, Esc to keep) |
| B | build a new drawer here (type the name, then Enter) |
| Esc | leave the Files app; while a question is shown at the bottom, Esc only cancels the question |

## Apps that look after their own drawers

Mail, Contacts, Calendar and Settings keep their data as pages in their own drawers:
`mail:inbox`, `mail:sent`, `contacts`, `calendar` and `system:settings`. You can `peek`, `read`,
`seek` and `copy` from these drawers in the shell, but **only the app can change them** (for
example, a mail is only sent by the Mail app, or by a chore's `send`).

The same keys work in every app: Up/Down choose, Enter opens, Esc goes back, **Ctrl+K confirms**
(keep, send, toss), and letter keys do the actions shown in the bottom bar. To throw something
away: press T, then Ctrl+K (Esc keeps it). In a form, Up/Down/Tab move between fields.
If typing didn't fit in a field or line, the app refuses to keep or send until you fix it with
Backspace.

### Mail (`mail`)

- The list shows the **Inbox**; Tab switches to **Sent**. Enter opens a message.
- While reading: **R** reply, **F** forward, **T** toss, Up/Down scroll, Esc back.
- **W** writes a new message. Fields: To (one mail address, like `mira`), Subject, and the
  message itself. Tab moves between them. **Ctrl+K sends.** Esc twice throws the draft away.
- A reply fills in To and "Re: subject" and starts with the cursor in the message; a forward
  fills in "Fwd: subject" and the old text.
- If To is not the mail address of one of your contacts, the first Ctrl+K only warns you
  (addresses must match exactly); a second Ctrl+K sends anyway.

### Contacts (`contacts`)

- A list of people with name, mail address and phone; the note of the chosen person is shown
  below. **A** adds a person, **Enter** opens the chosen person to change them, **T** tosses.
- Fields: Name, Mail (one word, it is the person's address on this computer), Phone, Note.
  **Ctrl+K keeps.** Esc twice goes back without keeping.

### Calendar (`calendar`)

- A list of events sorted by date and time. **A** adds, **Enter** opens to change, **T** tosses.
- Fields: Date (`YYYY-MM-DD`), Time (`HH:MM`, 24-hour), Title, With (who it is with).
  **Ctrl+K keeps.** Esc twice goes back without keeping.

### Settings (`settings`)

- **name** (shown in the top bar), **theme** (blue, black, brown, purple), **start** (the app
  that opens when the computer starts: shell, files, mail, calendar, contacts) and **clock**
  (24h or 12h).
- Up/Down choose a setting; Left/Right (or Enter) change a choice; Enter edits the name (Enter
  again when done). **Ctrl+K keeps.**

### Calculator (`calc`)

- Type a sum with `+ - * / ( )` and press Enter; Up brings back the last sum. Esc leaves.
- Whole answers are exact. Others show up to 6 decimals, with "about" if they were rounded.
  Numbers must be below 1000000000000.

## History (stamps)

The computer can keep earlier versions of your desk. A **stamp** is a saved copy of every
drawer and page at one moment, with a number (`#1`, `#2`, ...), a date and a short note.
The drawers looked after by an app (`mail`, `contacts`, `calendar`, `system`) are **not** part
of stamps: a rewind never un-sends a mail.

- When the computer starts, the desk is already stamped: `#1` is the desk as it started (some
  desks come with more stamps; `stamps` lists them), unless it is too big to stamp (see the
  limits below).
- Once there is a stamp, the desk is always **on** one stamp. `stamps` marks it with `*`.
- `changes` compares the desk with that stamp. Each changed item is listed as `new`, `gone` or
  `changed`; for a changed page you see the lines that went **out** (`-`) and came **in** (`+`),
  with their line numbers, and one unchanged line around them. `changes tools` shows only what
  is inside the drawer `tools` (or only that page).
- `stamp fixed the totals` saves the desk as the next stamp, on top of the one the desk is on.
  The note is one line, at most 60 characters. Stamping when nothing changed is refused.
- `compare 2 4` shows what changed from stamp #2 to stamp #4; `compare 2` shows what changed
  from stamp #2 to the desk now. Add a page or drawer to see only that part:
  `compare 2 4 tools`. Stamp numbers can be written `2` or `#2` (a number is always read as a
  stamp, so for a page named `3` write `desk:3`).
- `rewind 2` puts the **whole desk** back as it was in stamp #2. Later stamps are kept, so you
  can rewind forward again. A rewind is refused while the desk has changes that are not
  stamped yet (stamp them first), so nothing is lost.
- `bring tools:total 2` brings back **one page** as it was in stamp #2 (its drawer is made again
  if needed). It is refused if that page has changes that are not stamped yet, or if one of the
  drawers above it is now a page. A page that is gone can always be brought back, even if
  tossing it was not stamped.
- A stamp made after a rewind is made on top of the stamp you rewound to; `stamps` shows this
  as `(made on #2)`.
- At most 99 stamps, and the history has room for 2 MB. Every text is kept only once, however
  many stamps (or names) it appears under; each drawer and page in a stamp also takes a little
  room (so a desk with many pages fills the history sooner). `stamps` shows how much is used.
  A desk with more than 5000 drawers and pages (not counting the app drawers) can't be stamped.

`stamps`, `changes` and `compare` open a screen you can scroll with Up/Down; Esc goes back.
A very long list is cut off, and says so: then name one page or drawer to see less at once.
Pages longer than 3000 lines are not compared line by line (you see only that they changed).
Like every command, a history command that can't be done changes nothing.

## Chores (small programs)

A **chore** is a page of instructions that the computer carries out for you. Chores are useful
when a job is too big to do by hand, like checking every mail or every contact.

`guide do` opens the complete chore reference on the screen (Up/Down scroll, Esc back). This
chapter explains the same rules with examples.

### Writing and running a chore

1. `notes desk:mychore` opens a new page in Notes. Type one instruction per line (press Enter
   between lines), then **Ctrl+K** to keep it and **Esc** to leave.
2. `do desk:mychore` runs it. An Output screen shows the result on the first line (Finished or
   Stopped), then everything the chore said with `say`, then every change it made. Esc goes back.
3. If it stopped, the first line names the line with the mistake, e.g.
   `Stopped: line 5: total has no value yet ...`. Open the page in Notes again, fix that line,
   keep it and run it again. **A chore that stops because of a mistake changes nothing**, so running it again is safe. (`stop` is different: it ends the chore on purpose, and the chore counts as finished, so its changes are made.)

Lines can be at most 78 characters (the Notes limit). Indenting lines inside blocks is optional
but makes them easier to read.

### The complete reference

This is exactly the text `guide do` shows on the screen.

```text
CHORES: the complete reference
A chore is a page of instructions. Write it (for example in Notes), then
run it from the prompt with:   do <page>      e.g.  do desk:count
An Output screen shows what the chore said and whether it worked.

EXAMPLE
  -- count Omar's mails and save the answer
  set total to 0
  for each m in pages('mail:inbox')
    when field(m, 'from') is 'omar'
      set total to total + 1
    done
  done
  say 'Omar sent ' & total & ' mails'
  write('desk:answer', total)

LINES
  One instruction per line. Spaces at the start of a line don't matter.
  -- starts a comment: the rest of the line is ignored.
  Capitals don't matter in words and names (SET = set, Total = total).

INSTRUCTIONS
  set NAME to VALUE       give NAME a value (names: letters, digits, _)
  say VALUE               show a line on the Output screen
  when CONDITION          the lines up to done run only if CONDITION is yes
  otherwise               (optional, inside when) lines for when it is no
  for each NAME in LIST   the lines up to done run once for every item
  repeat NUMBER times     the lines up to done run NUMBER times (0-10000)
  done                    ends a when, for each or repeat block
  stop                    ends the chore here; it still counts as finished
  ACTION(...)             one of the actions below, on a line of its own

VALUES
  numbers    12   3.5   -2
  text       'hello' or "hello" (no line breaks; a text can't contain
             its own quote mark, so use "it's" or 'say "hi"')
  yes, no    the answers of comparisons
  lists      made by pages(...), lines(...) and words(...)
  A name must get a value with set before it is used.

OPERATORS, strongest first
  ( )            brackets
  -              minus in front: -x
  * /            multiply, divide (numbers only)
  + -            add, subtract (numbers only)
  &              join into text: 'total: ' & 5 gives 'total: 5'
  is, is not     the same value? Both sides must be the same kind, and
                 capitals count ('A' is not 'a')
  < > <= >=      compare two numbers, or two texts in alphabetical order
                 (dates written 2026-11-14 compare correctly as text)
  has            text has text: 'has a cat' has 'cat' is yes;
                 list has text: yes if the text is one of the items
  not            not yes is no; not x is y means x is not y
  and, or        combine yes/no answers (and is stronger than or)
  One comparison at a time: a < b and b < c, not a < b < c.
  Mixing kinds is a mistake: 1 + '2' fails. Use number('2').

NUMBERS
  Shown and written like 163, 12.5 or 0.333333 (at most 6 decimals).
  They must stay below 1000000000000. Dividing by 0 is a mistake.

READING
  pages(DRAWER)      list of the pages directly in DRAWER, as paths like
                     mail:inbox:m1, in name order (m2 before m10)
  read(PAGE)         all the text of PAGE
  field(PAGE, KEY)   the value of a 'key: value' line at the top of PAGE
                     (up to the first empty line); '' if there is none
  body(PAGE)         the text after the first empty line ('' if none)
  exists(PATH)       yes if there is a page or drawer at PATH
  lines(TEXT)        list of the lines of TEXT
  words(TEXT)        list of the words of TEXT
  count(X)           number of items in a list, or characters in a text
  item(LIST, N)      item number N of LIST (the first is 1)
  number(TEXT)       text to number: number('12.5') is 12.5
  lower(TEXT)        TEXT in small letters
  today()            today's date, like 2026-11-12
  weekday(DATE)      the day of DATE, like monday
  add_days(DATE, N)  the date N days after DATE (before it if N is below 0)
  Paths always start at the desk: 'notes:list' and 'desk:notes:list' are
  the same page, wherever you are.

ACTIONS (each on a line of its own)
  write(PAGE, TEXT)              make PAGE with this text, or replace it
  add(PAGE, TEXT)                add a line at the end of PAGE (makes it
                                 if needed)
  toss(PAGE)                     throw PAGE away
  send(TO, SUBJECT, TEXT)        send a mail like the Mail app does; TO
                                 must be the mail address of a contact
  event(DATE, TIME, TITLE, WITH) add an event like the Calendar app does:
                                 DATE like 2026-11-14, TIME like 18:00,
                                 WITH can be ''
  write, add and toss can't change the pages of the apps (the drawers
  mail, contacts, calendar and system). Use send and event for those.

ALL OR NOTHING
  The whole page is checked first: a mistake on any line stops the chore
  before anything runs. While it runs, it sees the pages as they were
  when it started. Its changes are made together at the end, and only if
  it finished without a mistake. If it stops, nothing is changed, so you
  can fix the page and run it again.

LIMITS (going over one is a mistake that stops the chore)
  120 lines and 4096 characters per chore; blocks 16 deep; brackets 12
  deep; 4096 characters per text; 1000 items per list; 100 names;
  200 say lines; 200 actions; 50000 steps (every part of a calculation
  and every turn of a loop is a step).
```

### Examples

Count the pages in a drawer and save the number:

```
set n to count(pages('desk:recipes'))
write('desk:how-many', n)
```

Make a list of every contact's phone number, one line each:

```
for each c in pages('contacts')
  add('desk:phones', field(c, 'name') & ': ' & field(c, 'phone'))
done
```

Find mails whose subject mentions a word, ignoring capitals:

```
for each m in pages('mail:inbox')
  when lower(field(m, 'subject')) has 'meeting'
    say field(m, 'from') & ' - ' & field(m, 'subject')
  done
done
```

Add up numbers written on the lines of a page (one number per line):

```
set total to 0
for each line in lines(read('desk:costs'))
  set total to total + number(line)
done
say 'total: ' & total
```

Put an event on the next 3 Fridays, starting from today:

```
set d to today()
set added to 0
repeat 21 times
  set d to add_days(d, 1)
  when weekday(d) is 'friday' and added < 3
    event(d, '17:00', 'Drinks', '')
    set added to added + 1
  done
done
```

### Things to know

- **Paths start at the desk** inside chores: `'notes:list'` means `desk:notes:list`, wherever
  you are. `pages(...)` gives full paths like `mail:inbox:m3`, ready to use in `read`,
  `field` and `body`.
- **`field` reads a page**, not a text: `field(m, 'from')`. To take a piece out of a text, use
  `lines`, `words` and `item`, e.g. `item(words('amount: 120'), 2)` is `'120'`.
- **Text is not a number.** Values read from pages are text: use `number(...)` before adding
  them up, and compare them as text (`field(e, 'time') is '18:00'`) or turn them into numbers
  first. `1 + '2'` and `1 is '1'` are mistakes.
- **Capitals count** in `is` and `has`: use `lower(...)` on both sides to ignore them.
- **Dates** are text written `YYYY-MM-DD`, so `<` and `>` compare them correctly.
- **Mail and events:** `write`, `add` and `toss` can't change the app drawers (`mail`,
  `contacts`, `calendar`, `system`). Use `send` (the address must belong to a contact) and
  `event` (date `YYYY-MM-DD`, time `HH:MM`).
- **A chore sees the pages as they were when it started.** Its changes are made together at the
  end, so a page it writes can't be read back in the same run.
- **Quotes:** text can be written in `'single'` or `"double"` quotes. Single quotes are usually
  easier to type through a command line.

## Tips

- After a change, check it with `peek` or `read`.
- For a page with several lines, either `write` the first line and `add` the rest, or use Notes.
- To change part of an existing page without retyping it, use Notes and keep with Ctrl+K.
