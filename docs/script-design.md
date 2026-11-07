# Unseen Script: build apps inside Unseen OS (design, draft 4)

Written 6 Nov 2026, branch `language`. Drafts 1 (`errand-design.md`), 2 and 3 were reviewed
by Sol; this draft folds in all three reviews. Nothing here touches experiment 4 or the frozen chore
language.

**Decisions by Yentl (6 Nov):**
- Name: **Unseen Script**.
- Main goal: **build apps** with it, full-screen programs like Mail or Calendar.
- Old chores keep working: yes, but it is not important.
- Order: core, then modules and tests, then build upwards.
- Look: **its own style** (familiar ideas, own spelling; not close to Python).

## 1. Why

Unseen OS tests whether a model can learn a computer from its manual alone. The chore language
is small: one page, 120 lines, no functions, no screen. Unseen Script lets a model **build a
real app** inside the OS: its own screen, keys, saved data and tests. No model has seen this
language, so it has to be learned from the manual. And "build an app" is a much richer task than
"count the mails".

Later, the same apps are the base for a SWE-bench-style benchmark (section 12): "add this
feature" or "fix this bug in the app", checked by hidden tests that press keys and read the
screen.

## 2. Principles

1. **Familiar ideas, own spelling.** Functions, lists, records, modules and tests are ideas
   every model knows. How they are written is Unseen Script's own: regular, all written down,
   learnable from the manual, not fully guessable.
2. **An app is a small loop.** The app has a **state** (one value). Three tasks: `start`
   makes the first state, `key` gives the new state after a key press, `draw` shows a state on
   the screen. The OS runs the loop. Simple to write, simple to test, simple to make safe.
3. **Every step is all or nothing.** A step is a `run`, the app's start, or one key press in an
   app. Before the first step, the whole program is checked (every page it uses). Within a
   step, its page changes, its new state and its new screen are kept together, and are only
   made real if the whole step finished without a mistake.
4. **Never a crash.** Every limit gives a normal mistake. Every step is bounded in time and
   memory, so Ctrl+Q always closes an app within one step.
5. **Easy to type through a command line.** No meaningful indentation, no tabs, no `$`,
   backtick, `#`, `\` or `|`. Text in single quotes; `''` inside a text is one quote mark.
6. **Good mistakes.** Every mistake names the page and line, says what was expected, and shows
   where it was called from.
7. **The manual is tested.** Every example in the reference is a test that runs from the first
   milestone on, and every example is also typed through PowerShell and bash to check the OS
   received exactly that text.

## 3. A first look: a shopping list app

`desk:apps:shop` (opened with `open desk:apps:shop`):

```
app 'Shopping list'
use 'desk:apps:shop-lib'

task start()
  give {items: load_items(), picked: 1, typing: ''}
done

task key(s, k)
  when k is 'down' and s.picked < count(s.items)
    set s.picked to s.picked + 1
  or when k is 'up' and s.picked > 1
    set s.picked to s.picked - 1
  or when k is 'enter' and s.typing is not ''
    add s.typing to s.items
    set s.typing to ''
    save_items(s.items)
  or when k is 'delete' and count(s.items) > 0
    set s.items to without(s.items, s.picked)
    set s.picked to max(min(s.picked, count(s.items)), 1)
    save_items(s.items)
  otherwise
    set s.typing to edit_text(s.typing, k)
  done
  give s
done

task draw(s)
  title('Shopping list - ' & count(s.items) & ' items')
  -- this small example only shows the first 20 items
  for each n in range(1, min(count(s.items), 20))
    when n is s.picked
      show(n, 2, '> ' & item(s.items, n), 'bright')
    otherwise
      show(n, 2, '  ' & item(s.items, n))
    done
  done
  show(22, 2, 'new: ' & s.typing)
  help('Up/Down pick  Enter add  Delete remove  Ctrl+Q close')
done
```

`desk:apps:shop-lib`:

```
fixed list_page is 'desk:shop-list'

task load_items()
  when exists(list_page)
    give lines(read(list_page))
  done
  give []
done

task save_items(items)
  write(list_page, join(items, newline()))
done
```

`desk:apps:shop-tests` (run with `test desk:apps:shop-tests`):

```
use 'desk:apps:shop'

test 'enter adds the typed item and saves it'
  pretend 'desk:shop-list' holds 'milk'
  launch 'desk:apps:shop'
  press 'b'
  press 'enter'
  check app_state().items is ['milk', 'b']
  check page_now('desk:shop-list') is 'milk' & newline() & 'b'
  check screen_line(2) has 'b'
done

test 'the picked item is shown bright'
  pretend 'desk:shop-list' holds 'milk' & newline() & 'eggs'
  launch 'desk:apps:shop'
  press 'down'
  check screen_line(2) has '> eggs'
  check screen_colour(2, 2) is 'bright'
done

test 'up and down on an empty list change nothing'
  launch 'desk:apps:shop'
  press 'down'
  press 'up'
  check app_state().picked is 1
  press 'x'
  press 'enter'
  check app_state().items is ['x']
done
```

## 4. The language

### Values

| Kind | Written | Notes |
|---|---|---|
| number | `12`, `3.5`, `-2` | as chores: 64-bit floating point, shown with at most 6 decimals, must stay below 10^12 |
| text | `'hello'`, `"hi"`, `'it''s'` | `''` inside single quotes is one quote mark; `newline()` gives a line break |
| yes / no | `yes`, `no` | answers of comparisons |
| list | `[1, 'a', yes]`, `[]` | any kinds; first item is 1 |
| record | `{name: 'Mira', age: 3}`, `{}` | named fields, kept in the order they were made; `r.name` reads one; a missing field is a mistake |
| nothing | `nothing()` | what a task gives when it ends without `give`. A function, not a word, so old chores that use `nothing` as a name keep working |

- **Values are copies.** `set b to a`, then changing `b` (or anything inside it, at any
  depth), never changes `a`. The same for task inputs and what a task gives. Inside, copies are
  shared until one side changes, so copying is cheap.
- **`is` / `is not`** compare two values of the same kind. Lists: the same items in the same
  order. Records: the same field names with equal values; field order doesn't count. Different
  kinds are a mistake (`1 is '1'`, as chores), **except `nothing()`**: anything can be
  compared with `nothing()`, so `x is nothing()` always works.
- **As text** (`&`, `say`, `show`, `text(...)`): lists as items joined by `, ` (as chores),
  records as `{name: Mira, age: 3}`, nothing as `nothing`.

### Names

- Capitals don't matter in names (as chores).
- **One scope per task**, and one for the main page. A name made inside a `when`, loop or
  `try` block still exists after it (as chores). Loop names too.
- A task sees only its inputs, its own names and the `fixed` values. Inputs are ordinary names:
  changing one changes only the task's copy.
- Using a name that has no value yet is a mistake **when that line runs** (as chores), not when
  the page is checked.
- **Clashes, found before anything runs:** two tasks or fixed values with the same name
  anywhere in the program; a task or fixed value with the name of a built-in function; `set`,
  `add ... to` or a loop using a fixed name. A plain name and a task may share a name: `total`
  is the name, `total(...)` the task.

### Lines

Each line is one instruction. A line is read in this order:

1. If the line is one complete call (`add('desk:x', 'hi')`, `save_items(l)`), it is a call,
   unless its first word is an instruction word: `say (1 + 2)` and `give (x)` are always the
   instruction. (So a task can't have the name of an instruction word: `set`, `add`, `say`,
   `stop`, `when`, `or`, `otherwise`, `done`, `for`, `repeat`, `while`, `skip`, `quit`, `give`,
   `task`, `fixed`, `use`, `app`, `fail`, `try`, `instead`, `test`, `check`, `pretend`,
   `launch`, `press`. A built-in action keeps its call: `add('desk:x', 'hi')`.)
2. Otherwise its first word (or first two, for `or when` and `for each`) decides the
   instruction, and the rest of the line must match that instruction's pattern exactly.
3. Anything else is a mistake, found before anything runs.

So the new instruction words are only special as the first word of a line. Everywhere else
they are ordinary names: `set skip to 1` and `say skip` work as in chores. In expressions
nothing new is a word: the new things are `nothing()` and other functions, `[...]`, `{...}`
and `.field`.

**Line patterns** (CAPITALS are parts; `EXPR` is an expression, `TARGET` see Targets):

```
set TARGET to EXPR          add EXPR to TARGET         say EXPR
when EXPR                   or when EXPR               otherwise          done
for each NAME in EXPR       repeat EXPR times          while EXPR
skip    quit    stop        give EXPR                  fail EXPR
try                         instead NAME
task NAME(NAME, ...)        fixed NAME is EXPR         use 'PAGE'         app 'TITLE'
CALL                        (a call to a function or task, on its own line)
test 'TITLE'                check EXPR                 launch 'PAGE'      press 'KEY'
pretend 'PATH' holds EXPR   pretend today is 'DATE'    (tests only, section 6)
```

Where a pattern has a word after an `EXPR` (`to`, `in`, `times`, `is` after `fixed NAME`), the
`EXPR` ends at that word. These words are chore keywords, which can never be names (`set`,
`to`, `say`, `stop`, `when`, `otherwise`, `done`, `for`, `each`, `in`, `repeat`, `times`, `and`,
`or`, `not`, `is`, `has`, `yes`, `no`), so this is never ambiguous. A doubled quote mark inside
a text is one quote mark, for both kinds of quotes (`'it''s'`, `"say ""hi"""`).

**Page order:** an app page starts with its `app` line; then come the `use` lines (on every
page); then `fixed` and `task` and, on a main page, its instructions. Comments and empty lines
can go anywhere.

**Expressions, strongest first** (the chore order, with two new levels at the top):

| Level | Forms |
|---|---|
| 1 | `( EXPR )`, numbers, texts, names, `[EXPR, ...]`, `{NAME: EXPR, ...}`, calls `f(EXPR, ...)` |
| 2 | `.NAME` after any level-1 form, any number of times: `r.who.name`, `start().items` |
| 3 | `-` in front |
| 4 | `*` `/` |
| 5 | `+` `-` |
| 6 | `&` |
| 7 | comparisons: `is`, `is not`, `<` `>` `<=` `>=`, `has`. One comparison at a time (as chores): `a < b < c` and `l has 1 < 2` are mistakes; use brackets (`l has (1 < 2)`) or `and` |
| 8 | `not` |
| 9 | `and` |
| 10 | `or` |

Lists and records may be empty (`[]`, `{}`); a comma after the last item is a mistake. A field
name is any word, even a keyword (mail pages have a `to:` line, so `m.to` must work);
capitals don't matter and they are kept in small letters. The same field twice in one `{...}`
is a mistake found before anything runs. `fields(page)` makes a record from the `key: value`
lines at the top of a page; keys that are not words (letters, digits and `_`, not starting with
a digit) are left out.

### Instructions

All chore instructions (`set`, `say`, `when`/`otherwise`, `for each`, `repeat`, `done`,
`stop`, call lines), plus:

| Instruction | Meaning |
|---|---|
| `or when CONDITION` | another branch inside `when`, before `otherwise` |
| `while CONDITION` ... `done` | repeat while the condition is yes |
| `skip` / `quit` | next turn of the innermost loop / end the innermost loop |
| `set TARGET to VALUE` | see Targets below |
| `add VALUE to TARGET` | put an item at the end of the list at TARGET |
| `task NAME(A, B)` ... `done` | a task (a function); only at the top level of a page |
| `give VALUE` | end the task, giving VALUE back |
| `fixed NAME is VALUE` | a constant (see below); only at the top level of a page |
| `use 'PAGE'` | load the tasks and fixed values of another page; at the top (after `app`) |
| `app 'TITLE'` | first line of an app page (section 5) |
| `fail VALUE` | stop with your own mistake |
| `try` ... `instead NAME` ... `done` | if a mistake happens in the try part, undo that part and run the instead part |
| `test`, `pretend`, `launch`, `press`, `check` | tests (section 6) |

**Targets.** A target is a name, then any number of `.field`, then at most one `at N` at the
end: `s`, `s.items`, `s.items at 2`, `s.who.name`. Every part before the last must already
exist and be a record (or a list, for `at`). The last field may be new; `at N` must be from 1
to the number of items. If any part fails, nothing changes. Deeper changes inside lists use
a name in between (`set r to item(rows, 2)`, change `r`, `set rows at 2 to r`).

**Fixed values** may use literals, lists and records, operators, earlier fixed values and the
built-in functions that never look at the world (`newline()`, `nothing()`, `upper(...)`, ...;
not `today()`, `read(...)` or tasks). They are worked out in loading order (section Modules)
before the first step. Nothing inside a fixed value can ever be changed.

### Tasks

- Called like built-in functions: `load_items()`. A task called on a line of its own throws
  away what it gives.
- Tasks can call each other and themselves.
- Calls are checked before running: a task or function that doesn't exist, or the wrong number
  of inputs, is a mistake found before anything runs.
- `stop` ends the whole `run` (as chores). In an app step it is a mistake (use `close()`); in a
  test it makes the test fail.

### Modules

- `use 'desk:apps:shop-lib'` loads that page. A used page may only hold `use`, `fixed`, `task`
  and comments, so loading a page never does anything by itself. An app page may be used (for
  example by its tests): its `app` line is then ignored.
- A page is known by its full path. Each page is loaded once; the pages it uses come first, in
  the order of its `use` lines. Two pages using the same third page is fine; a loop (`a` uses
  `b` uses `a`) is a mistake.

### Mistakes and try

- A mistake ends the step and nothing from that step is changed. The Output screen shows, for
  example, `Stopped: desk:apps:shop-lib line 5: number('12,5') is not a number` and below it
  `called from desk:apps:shop line 12`.
- `try` ... `instead problem` ... `done`: if a mistake happens in the try part, including
  `fail` and mistakes inside tasks it calls, **everything the try part did is undone**: names,
  `say` lines, screen output, planned changes and a `close()` go back to how they were at
  `try`. Then the instead part runs, with the message as text in `problem`.
- Running out of a limit (section 9), and a failed `check`, can't be caught.

### Reading and changing pages

- **Reads see the world as it was when the step began** (as chores): a page the step writes
  can't be read back in the same step. (The next key press sees it.)
- **Planned changes form a world of their own,** in order: each change is checked against the
  world as it would be after the changes before it. Last write wins, `add` after `write`
  extends it, `write` after `toss` makes the page again.
- Each change is checked **when it is planned**: `write` to an app drawer, a `send` to an
  address that isn't a contact, or a desk that would get too full is a normal mistake on that
  line (catchable by `try`). So making the changes at the end can't fail because of the
  program.
- `send` and `event` make pages the way the Mail and Calendar apps do (`mail:sent:s<N>`,
  `calendar:e<N>`, with the first free number).

### Built-in library

All chore functions and actions, plus about 35 more:

- **text:** `upper`, `trim`, `split(text, by)`, `join(list, by)`, `replace(text, old, new)`,
  `find(text, part)` (position, 0 if missing), `slice(text, from, to)`, `starts`, `ends`,
  `newline()`, `text(value)`, `pad(text, width)`
- **numbers:** `round(n, decimals)`, `floor`, `ceil`, `abs`, `min(a, b)`, `max(a, b)`,
  `is_number(text)`
- **lists:** `sum`, `sort`, `reverse`, `unique`, `first`, `last`, `slice(list, from, to)`,
  `range(from, to)`, `position(list, value)` (0 if missing), `without(list, n)`
- **records:** `keys(record)`, `has_key(record, key)`, `fields(page)`
- **dates:** `days_between(a, b)`, `month(date)`, `year(date)`
- **apps:** the screen functions of section 5, `close()`, and `edit_text(text, key)` (typing
  into a text: a letter, digit, space or sign adds it; `'backspace'` removes the last one;
  other keys change nothing)

Edge cases follow one rule each: `slice` counts from 1 and includes both ends, positions past
the end are moved to the end; `range(a, b)` with `b < a` is `[]`; `first`, `last` and `item`
on an empty list are a mistake; `sum([])` is 0; `sort` needs all numbers or all texts.

No functions as values in version 1 (no `map` with a task); loops do that job.

## 5. Apps

### The loop

An app page starts with `app 'TITLE'` and has the tasks `start()`, `key(state, k)` and
`draw(state)`. `open <page>` checks the program, then runs:

- **The start step:** `start()`, then `draw` with the state it gave.
- **A key step, for every key press:** `key(state, k)`, then `draw` with the state it gave
  (unless `close()` was called, see below).

**One step = the task and its draw, together.** They share one read snapshot and one budget.
At the end of a successful step, in this order: the planned page changes are made, the new
state becomes the app's state, the new screen is shown. If anything in the step fails
(`key`/`start` or `draw`), **none of that happens**: no changes, the old state and screen
are dropped, and the app closes with the Output screen showing the mistake. Earlier steps are
kept.

- `start` and `key` must `give` the state: ending without `give` is a mistake. The state can
  be any value (usually a record) and must fit the state budget (section 9).
- `close()` (only in `start` or `key`): the step goes on to the end of the task, which must
  still `give` a state; then its changes are made, `draw` is skipped and the app closes, back
  to where it was opened.

### What each part may do

The OS keeps track of which part is running. Helper tasks may be shared, but what they do is
checked **when it happens**, for the part that is running:

| Part | read pages | change pages (`write`, `send`, ...) | screen functions | `close()` | `say` |
|---|---|---|---|---|---|
| `run` (not an app) | yes | yes | no | no | yes |
| `start`, `key` | yes | yes | no | yes | no |
| `draw` | no | no | yes | no | no |
| a test's own lines | yes (test world) | yes (planned, never made) | no | no | yes |

So `draw` works only from the state. A not-allowed call is a mistake on that line, for
example `draw can't use write(...): only start and key can change pages`.

### Keys

- Keys reach `key` as text: `'a'`, `'A'`, `'7'`, `' '`, `'.'`, `'enter'`, `'esc'`,
  `'backspace'`, `'tab'`, `'delete'`, `'up'`, `'down'`, `'left'`, `'right'`, `'home'`, `'end'`,
  and `'ctrl+a'` to `'ctrl+z'`.
- **Ctrl+Q is never sent to the app**: the OS checks for it before anything else and closes the
  app. A step can't be interrupted, but every step is bounded (section 9), so it closes within
  one step (aimed at under half a second; measured in M5).
- Keys pressed while a step runs wait their turn. Ctrl+Q goes before them: when a step ends
  and a Ctrl+Q is waiting, the app closes and the keys waiting before it are dropped.
- **As built (M5):** keys are handled one at a time, in order, and Ctrl+Q closes the app as
  soon as it is read. The keyboard holds one key at a time and the harness waits for each key
  to be handled, so there is no queue of keys to skip. Measured in QEMU
  (tools/test_apps_os.py): a step of about 360000 steps takes 0.09 s, and a slow step followed
  by Ctrl+Q closes within the same 0.09 s.

### The screen

- The app has **rows 1 to 23 and columns 1 to 78**: app row r is screen row r, app column c is
  screen column c. Screen row 0 is the title bar and row 24 the help bar, as in the built-in
  apps; screen columns 0 and 79 stay empty as a margin.
- `title(text)` sets the top bar (default: the app's TITLE). `help(text)` sets the bottom bar
  (default: `Ctrl+Q close`). The OS always adds `Ctrl+Q close` if it is missing.
- `show(row, col, text)` or `show(row, col, text, colour)`. Colours: `'normal'`, `'bright'`,
  `'accent'`, `'warn'`, `'select'`. Row and col must be whole numbers in range; text past
  column 78 is cut off; a line break in the text is a mistake.
- `fill(row, colour)` colours a whole row. `cursor(row, col)` shows the blinking cursor there
  (otherwise it is hidden).
- The screen is empty at the start of every `draw`. Inside, the OS keeps the app's last screen
  and repaints it when needed; it never runs `draw` again just to repaint.
- **As built (M5):** a `show` without a colour keeps the colour the place already has (normal,
  or the colour of a `fill`), so `fill(r, 'select')` then `show(r, 2, text)` shows a selected
  row. Any value can be shown (as `say` turns it into text). Characters a screen can't show are
  shown as `?`. A help text is cut so that `Ctrl+Q close` always fits. The screen functions
  are kept in order while `draw` runs, so `try` can undo the last ones; the screen is made
  from them when `draw` ends (at most 2000 screen calls per draw).

### In the kernel

A script app is one more kind of app on the OS's app stack (next to Mail, Notes, ...). It
holds the checked program, the state and its last screen. A mistake replaces it by the Output
viewer; Esc there goes back to where the app was opened.

**As built (M5):** the engine (`script/src/app.rs`) gives a step back as its planned changes
plus the new state and screen; the kernel (`kernel/src/programs.rs`) makes the changes the way
`run` does (all together on a copy of the desk), and only then keeps the state and screen
(`App::keep`). The state is copied into its own 256 KB budget at the end of each step, so the
step's memory is given back in full. `run` refuses an app page (`open` it); an app page may
only hold `app`, `use`, `fixed` and `task` lines, and `app` must be its first line.

## 6. Tests

A **test page** holds `use`, `fixed`, `task` and `test` lines (no loose instructions). `test
<page>` runs every test and shows one line each: `PASS title`, or `FAIL title: line N: ...`,
then how many passed. For a failed `check a is b` it shows both values. Only the tests of the
page that is tested run: tests on a page it uses are not run.

### Each test starts fresh

- Its own names, its own budgets, its own **test world**.
- **Code comes from the real desk, data from the test world.** The program's pages (`use`,
  `launch`) are read from the desk as usual. Everything the program reads (`pages`, `read`,
  `field`, `exists`, ...) comes from the test world: the empty app drawers (`mail:inbox`,
  `mail:sent`, `contacts`, `calendar`), the default settings, and what the test `pretend`s.
  The date is 2026-11-14 unless the test says otherwise.
- **Nothing a test does is ever made on the real desk.**

### Test lines

| Line | Meaning |
|---|---|
| `test 'TITLE'` ... `done` | one test |
| `pretend 'PATH' holds TEXT` | a page in the test world (with the drawers above it); only at the top of a test, before any other line |
| `pretend today is 'DATE'` | the test world's date; also only at the top |
| `check CONDITION` | the test fails here if CONDITION is no |
| `launch 'PAGE'` | open an app in the test, exactly as `open` does: checks it and runs the start step |
| `press 'KEY'` | one key step of the launched app, exactly as in the OS (`'ctrl+q'` closes it) |

**Inside a test, steps work as in the OS.** Each `launch` and `press` is a real step: its own
read snapshot, its own step budget, and at its end its changes are made **in the test world**,
so the next press sees them. A failing step closes the app as in the OS; the test can check
that (`app_mistake()`), and goes on.

A test may also call tasks directly (`check total(...) is 5`), for small unit tests. What the
test's own lines may do is in the table of section 5. Their planned changes are never made:
`after(PATH)` shows them, but `launch` and `press` steps don't see them (they see only the
test world).

After the app has closed, `app_state()` and the screen checks show its last state and screen;
another `press` makes the test fail.

**As built (M5):** a `press` after the app closed (or with no `launch` before it) fails the
test, like a failed check. `launch` and `press` can't be in the try part of a `try` (a step's
changes are made in the test world and can't be undone; review of M5). The test's own lines
and every app step, also a failed one, share the 5000000 steps of the test: each step may use
at most what is left. `app_state()` gives a copy counted by the test, so keeping old states
can't hold memory outside the budgets. The test world holds at most 5000 drawers and pages,
as the desk, and a made mail or event takes the first number no page or drawer has. A `launch` that fails (a page that is not an app,
or a mistake in `start`) closes the app at once: `app_open()` is no and `app_mistake()` says
why. `launch` and `press` work only in a test's own lines (not in tasks), so a step never runs
deep inside another program. The changes of a step are made in the test world as the kernel
makes them (a mail as `mail:sent:sN`, an event as `calendar:eN`), within the test world's
1 MB. `screen_line(0)` is the title bar and `screen_line(24)` the help bar.

### What a test can look at

- `page_now(PATH)`: the text of a page in the test world now (`nothing()` if there is none);
  after a `press`, that includes what the step changed
- `after(PATH)`: the same, but also counting changes the test's own lines have planned and not
  yet made
- `sent()` and `events()`: the mails and events made or planned, as records
  `{to, subject, text}` and `{date, time, title, with}`
- `said()`: the `say` lines so far, as a list
- `app_state()`: the launched app's state
- `screen_line(row)` and `screen_colour(row, col)`: the app's screen after the last step
- `app_open()`: yes while the app is open; `app_mistake()`: the text of the mistake that closed
  it (`''` if none)

### Passing

A test **passes only if it reaches its `done`** with at least one `check` and every check yes.
A failed check ends that test. A mistake in the test's own lines, `stop`, going over the
whole-test budget, or a test with no checks is a FAIL.

## 7. Tools in the OS

| Command | What it does |
|---|---|
| `run <page>` | run a program (not an app); Output screen as for chores |
| `open <page>` | start an app |
| `test <page>` | run the tests on a test page |
| `lab` | try-out screen: type one line, see its value; names, tasks and `use` are kept; changes are only shown (`would write desk:x`), never made |
| `view <page> [from] [to]` | show a page with line numbers (at most 20 lines at a time) |
| `edit <page> <line> <text>` | replace one line |
| `insert <page> <line> <text>` | put a new line before line N (N = last + 1 adds at the end; line 1 of a missing page makes it) |
| `cut <page> <line> [to]` | remove one line, or lines from..to |
| `guide script` | the complete Unseen Script reference, on screen |

Notes gets line numbers, Ctrl+G (go to line) and Ctrl+F (find; long lines scroll sideways).
Lines made with `edit`/`insert` are at most 78 characters (the Notes width); a command,
including `edit` and its path, is at most 200. The text of `edit`/`insert` starts after exactly
one space, so indents are kept.

**How the lab works (M4):** it keeps the lines that worked and runs all of them again with each
new line. The desk can't change while the lab is open and a program is deterministic, so the
earlier lines do exactly what they did before and only what the new line adds is shown. A lab
line therefore means exactly what it means on a page (same checks, limits and messages). A
value line is checked as `say VALUE`, then shown as you would type it. The kept lines must stay
within one program's limits; a full lab keeps room for one more value line. A fingerprint of
what the kept lines said and planned is checked on every run: a line that would change it (a
`fixed` or `use` that an earlier `try` now sees differently) is refused. The lab's date is
fixed when it opens.

**Lines of a page (M4):** a page's lines are its text split at each new line, as Notes shows
them; an empty page is one empty line. `view`/`edit`/`insert`/`cut` work on byte ranges of the
text, so a page with very many lines costs no more memory than its text. An Output screen keeps
at most 6400 screen lines and says how many more there were; the last line of its text is
always shown (a cut history listing ends with how to see less).

**Desk room (M4 re-check):** planned changes count the desk's growth exactly as the kernel
counts it: bytes of page text (a toss frees exactly its text; a mail page adds exactly its
25 bytes of header lines and an event page its 28 bytes of field names, besides the texts; a
mail's body is trimmed at the end, as Mail does), and separately the number of pages. The bookkeeping of each
change counts in the step memory, not as desk text. So a change the engine accepts is never
refused when the changes are made.

## 8. How the harness and hidden tests use apps

- The harness keeps sending keys as now. A key counts as handled when its step has finished:
  changes made and the new screen shown (or the Output screen, after a mistake). The existing
  key acknowledgement waits for that.
- Hidden tests are test pages that the grader adds to a fresh copy of the allowed pages after
  the game (section 12) and runs with `test`. They use the same `launch` and `press` as above,
  so they check the app through the real loop.

## 9. Limits and safety (the kernel has 16 MB of memory and a small stack)

Sizes are counted as **live** memory: what exists at the same moment, with a fixed overhead per
value (an item or field counts its own size plus 32 bytes). They are checked **before** anything
is allocated, with arithmetic that can't overflow, so the OS heap never runs out:

- every allocation goes through one counter; lists and texts get their exact size reserved in
  advance (`try_reserve`), so growing them never asks the heap for more than was counted;
- a built-in that makes a result (`join`, `replace`, `sort`, turning a value into text, ...)
  works out the result's size first and counts it, together with any working space, before
  making it;
- **checking the program counts too:** tokens, the parse tree, the checked code and the
  working-out of fixed values are counted against the program budget while they are made, and
  the page size limits come first, so a huge page is refused before it is read in.

| Budget | Size | What counts |
|---|---|---|
| program | 1 MB | the checked code of all pages (tokens and parse tree while checking), fixed values |
| app state | 256 KB | the state kept between steps |
| step | 1 MB | everything a step makes at its peak: names, values, copies kept for `try`, planned changes, `say` lines, the new screen |
| work per step | 500000 steps | copying, comparing, joining or searching a text or list costs one step per 16 characters or per item; `sort` costs `n x log n`; so no single step can run long |
| work per test | 5000000 steps | all steps of one test together |
| test world | 1 MB | a test's world: pretended pages and the changes its steps made |
| making changes | 1 MB | the new pages built when a step's changes are made (never more than the planned changes, which the step budget already limits) |

At their peak, all of these together stay below 5 MB. The heap is 16 MB since M4 (it was 8 MB):
next to a running program it holds the desk (at most 1 MB, plus a copy while changes are made)
and the history of stamps (at most 2 MB). Only one program runs at a time.

**Depth:** at most 40 task calls deep; blocks 16 deep; brackets 12 deep in one line; values
nested at most 16 deep (a list in a record in a list ...); and at most 256 levels in all,
across all running tasks: every block and every value being worked out is one level, a `try`
two, a task call four (weighted by the stack each really uses). Measured in M3: the deepest
program the limits allow needs at most 80 KB of stack (release build; tools: script/examples
stack.rs and stackfile.rs); the kernel stack is 256 KB. Measured in the real OS in M4 (the
inspector's STACK probe, tools/test_script_os.py): the deepest programs through `run`, `test`
and `lab` leave a watermark of 61 KB; in M5 (tools/test_apps_os.py) the deepest app steps,
also pressed from inside a test, leave 51 KB (the free stack is painted at boot; the probe finds the
deepest byte written, which is what could harm the memory below the stack). Every part of the engine that walks
code or a value (checking, running, copying, comparing, turning into text, freeing) has a depth
limit, so the kernel stack can't overflow. The stack is measured with the deepest program the
limits allow, and made bigger if needed.

**Per program:** 400 lines and 16000 characters per page; 16 pages; 300 tasks.
**Per value:** 8000 characters per text; 2000 items per list or fields per record. **Per
step:** 500 `say` lines; 300 planned changes.

## 10. How it is built and tested

- **A separate crate, `script/`** (`no_std` with `alloc`), used by the kernel through a small
  `World` adapter, like chores. It runs on the PC with `cargo test`, without QEMU.
- **Stages:** lexer, parser (all used pages), checker (calls, clashes, use loops, block shape,
  `give` outside a task, ...), tree-walking runner with the budgets of section 9.
- **A reference that runs:** `docs/script-reference.md` grows with each milestone. Every example
  in it is a test with its exact expected result, and every example is typed through PowerShell
  and bash (as `tools/test_audit_argv.py` does) to check the OS receives exactly that text.
  Both checks run from M1 on.
- **Chores untouched:** `do` keeps using `chore.rs` (experiments 1-4 must stay reproducible).
  The chore tests also run through Unseen Script; "same result" means the same output and
  changes. Step counts may differ, and a chore that failed only because of the old, smaller
  limits may now finish.
- **Other tests:** unit tests for each feature and each mistake message; fuzzing (many random
  and broken programs and values: no panic, no stack overflow, no budget overrun); robot players
  in QEMU at M4 and M5; Sol reviews the code at each milestone.
- While experiment 4 runs: no QEMU and no heavy builds on this PC.

## 11. Milestones (core first, then build upwards)

| | What | Done when |
|---|---|---|
| M1 | **expressions and blocks**: values, lists, records, names, targets, `when`/`or when`, loops, `skip`/`quit`, budgets, and **every chore function and action** (so the chore regression can run) | unit tests, reference examples, typing round trip, chore regression, fuzzing pass |
| M2 | **tasks and library**: `task`/`give`, `fixed`, clashes, the ~35 new library functions | the same checks pass |
| M3 | **modules and tests**: `use`, `fail`, `try`/`instead`, test pages, `pretend`, `page_now`/`after`/`sent`/`said` (no apps yet) | the same checks pass |
| M4 | **in the OS**: `run`, `test`, `lab`, `view`/`edit`/`insert`/`cut`, Notes upgrades | robot in QEMU: writes a two-page program with `edit`/`insert`, tests it with `test`, runs it with `run` |
| M5 | **apps**: `app`, `open`, the loop, screen functions, Ctrl+Q, `launch`/`press` and the app checks in tests | robot in QEMU: builds the shopping list app, its tests pass, and it works by keys |
| M6 | **manual** chapter and `guide script` | every manual example passes (it is the reference, written up) |
| M7 | pilot: Haiku and Luna build 2-3 small apps | the tasks are fair and the harness works (a check, not a result) |

## 12. Later: app benchmark (SWE-bench style)

- **Task kinds:** build a small app from a description; add a feature to an existing app; fix
  a bug from a short issue note ("Delete removes the wrong item").
- **Grading by hidden tests** that launch the app, press keys and read screens and pages, as in
  SWE-bench: fail-to-pass (fail before, must pass after) and pass-to-pass (must keep passing).
- **Fair and checkable:**
  - which pages may be changed, the start page, and the budgets are written down per task;
  - grading copies only the allowed pages into a fresh, hidden world; the player never sees the
    hidden tests or the grader;
  - each hidden test has a time limit in the grader too (on top of the OS budgets);
  - every task is checked three ways: the unchanged app fails the fail-to-pass tests, the known
    fix passes everything, and **wrong fixes are caught** (hard-coding the visible example,
    swallowing mistakes with `try`, deleting tests, unrelated changes);
  - hidden worlds use other data than the visible example, so hard-coding fails, but nothing a
    task needs is left unsaid;
  - Sol checks every task for fairness.
- **Experiment rules as before:** tasks, seeds, versions, retries and conditions frozen before
  the run; pilot tasks never reused; with vs without the manual. We measure how much the manual
  helps; we don't claim the models' prior knowledge plays no part.

## 13. Style (decided)

**Its own style** (Yentl, 6 Nov): familiar ideas, own spelling. A model knows what a function
or a list is, but has to read the manual to write one correctly (`task ... give ... done`,
`or when`, `add x to list`). Not close to Python (that would test learning from the manual
less), and not deliberately strange (that would be hard for people too).
