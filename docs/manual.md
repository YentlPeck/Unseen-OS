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

Type a command and press Enter. `guide` lists all commands. The Up arrow brings back the
last command. Commands can be at most 200 characters long.

| Command | What it does |
|---|---|
| `guide` | list all commands |
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
| T | toss the chosen item (type `y`, then Enter, to confirm) |
| B | build a new drawer here (type the name, then Enter) |
| Esc | leave the Files app; while a question is shown at the bottom, Esc only cancels the question |

## Tips

- After a change, check it with `peek` or `read`.
- For a page with several lines, either `write` the first line and `add` the rest, or use Notes.
- To change part of an existing page without retyping it, use Notes and keep with Ctrl+K.
