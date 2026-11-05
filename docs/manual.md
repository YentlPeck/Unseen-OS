# Unseen OS manual

## The basics

- **Pages** hold text (like files). **Drawers** hold pages and other drawers (like folders).
- The top drawer is called the **desk**.
- Names use the letters a-z and A-Z, digits, `-`, `_` and `.`, at most 24 characters, no
  spaces. `desk` can't be used as a name.
- A **path** joins names with `:`. Example: `letters:mira` is the page `mira` inside the drawer `letters`.

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

If a command can't be done, the computer explains why in yellow, and nothing is changed.

## Notes app (editing a page)

`notes <page>` opens the page full-screen. The top bar shows the page and the line number;
the bottom bar shows the keys.

- Type to insert text at the cursor. Arrow keys, Home and End move the cursor.
- Enter splits the line; Backspace and Delete remove characters (and can join lines).
- In Notes you can't type a line longer than 78 characters, or join two lines that together
  would be longer. (Longer lines made with `write` stay as they are.)
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
   keep it and run it again. **A chore that stops changes nothing**, so running it again is safe.

Lines can be at most 78 characters (the Notes limit). Indenting lines inside blocks is optional
but makes them easier to read.

### The language in short

```
-- a comment: the rest of the line is ignored
set count to 0                   -- give a name a value
say 'count: ' & count            -- show a line on the Output screen
when count > 3                   -- the lines up to done run only if yes
  say 'more than 3'
otherwise
  say '3 or less'
done
for each p in pages('contacts')  -- run the block once for every page
  say field(p, 'name')
done
repeat 3 times                   -- run the block 3 times
  say 'hi'
done
```

- **Values:** numbers (`12`, `3.5`), text in quotes (`'hello'`), `yes`/`no`, and lists (from
  `pages`, `lines`, `words`).
- **Operators:** `+ - * /` for numbers; `&` joins anything into text; `is`, `is not`, `<`, `>`,
  `<=`, `>=` compare; `has` checks if a text contains a text (or a list contains an item);
  `and`, `or`, `not` combine yes/no.
- **Reading:** `pages(drawer)`, `read(page)`, `field(page, key)`, `body(page)`, `exists(path)`,
  `lines(text)`, `words(text)`, `count(x)`, `item(list, n)`, `number(text)`, `lower(text)`,
  `today()`, `weekday(date)`, `add_days(date, n)`.
- **Actions** (each on a line of its own): `write(page, text)`, `add(page, text)`, `toss(page)`,
  `send(to, subject, text)`, `event(date, time, title, with)`.

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
