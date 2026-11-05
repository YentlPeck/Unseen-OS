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

Type a command and press Enter. `guide` lists all commands; `guide <word>` explains one. The Up arrow brings back the
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
example, a mail is only sent by the Mail app).

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
- A reply fills in To and "Re: subject"; a forward fills in "Fwd: subject" and the old text.
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

## Tips

- After a change, check it with `peek` or `read`.
- For a page with several lines, either `write` the first line and `add` the rest, or use Notes.
- To change part of an existing page without retyping it, use Notes and keep with Ctrl+K.
