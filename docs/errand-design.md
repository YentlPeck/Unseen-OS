# Errand: a real programming language for Unseen OS (design, draft 1)

Written 6 Nov 2026. Working name **Errand** (chores grow up into errands). Branch `language`.
Nothing here touches experiment 4 or the frozen chore language.

## 1. Why

Unseen OS tests whether a model can learn a computer from its manual alone. The chore language
is small (one page, 120 lines, no functions). A bigger language that no model has ever seen is a
much stronger test: with Python or JavaScript a model mostly uses what it already knows; with
Errand it has to read, try and learn.

It is also the base for a later benchmark in the style of SWE-bench (section 9): small
codebases written in Errand, a bug note, and hidden tests that check the fix.

## 2. Principles

1. **Familiar ideas, own spelling.** Functions, lists, records, modules, errors and tests are
   ideas every model knows. The way they are written is Errand's own, so a model can't guess
   it fully, but one page of the manual is enough to learn it. No trick syntax: every rule is
   regular and written down.
2. **Grows out of chores.** Errand keeps the chore look (English words, blocks end with `done`,
   `&` joins text, `is` compares, lists start at 1). Goal: **every valid chore is a valid
   Errand program with the same result**, so the chore tests become Errand's first regression
   tests. The old `do` command and chore engine stay exactly as they are (experiments 1-4 must
   stay reproducible); Errand gets its own commands.
3. **Easy to type through a command line.** No meaningful indentation or tabs, and no
   characters that shells treat specially when they can be avoided: no `$`, backtick, `#`, `\`,
   `|`. Text uses single quotes (double quotes also work).
4. **Same safety as chores.** The whole program (every page it uses) is checked before
   anything runs. While it runs it sees the world as it was at the start. Its changes are made
   together at the end, and only if it finished. Every limit gives a normal mistake, never a
   crash (the kernel stops on a panic).
5. **Good mistakes.** Every mistake names the page and line, says what was expected, and shows
   where it was called from.

## 3. A first look

A module page, `desk:shop:money`:

```
-- money tools for the shop
fixed tax_rate is 21

task invoice_total(mail)
  set amount to number(field(mail, 'amount'))
  give amount + amount * tax_rate / 100
done

task overdue(mails, day)
  set late to []
  for each m in mails
    when field(m, 'due') < day and not (field(m, 'paid') is 'yes')
      add m to late
    done
  done
  give late
done
```

A program page, `desk:shop:main`, run with `run desk:shop:main`:

```
use 'desk:shop:money'

set late to overdue(pages('mail:inbox'), today())
set report to {count: count(late), total: 0}
for each m in late
  set report.total to report.total + invoice_total(m)
done
say count(late) & ' late invoices, ' & round(report.total, 2) & ' in all'
write('desk:shop:late', report.count & ' / ' & report.total)
```

A test page, `desk:shop:tests`, run with `test desk:shop:tests`:

```
use 'desk:shop:money'

test 'tax is added'
  pretend 'mail:inbox:m1' holds 'amount: 100'
  check invoice_total('mail:inbox:m1') is 121
done

test 'paid mails are not late'
  pretend 'mail:inbox:m1' holds 'due: 2026-01-01' & newline() & 'paid: yes'
  check count(overdue(pages('mail:inbox'), '2026-11-06')) is 0
done
```

## 4. The language

### Values

| Kind | Written | Notes |
|---|---|---|
| number | `12`, `3.5`, `-2` | as chores (6 decimals, below 10^12) |
| text | `'hello'`, `"hi"` | as chores; `newline()` gives a line break |
| yes / no | `yes`, `no` | answers of comparisons |
| nothing | `nothing` | what a task gives when it has no `give`; `x is nothing` tests it |
| list | `[1, 'a', yes]`, `[]` | items of any kind; first item is 1 |
| record | `{name: 'Mira', age: 3}`, `{}` | named fields; `r.name` reads one |

Values are **copies**: `set b to a` then changing `b` never changes `a`. This keeps programs
easy to reason about (and is simple to make safe in the kernel).

`is` compares any two values of the same kind, lists and records item by item.

### Instructions

Everything from chores (`set`, `say`, `when`/`otherwise`, `for each`, `repeat`, `done`,
`stop`, action lines), plus:

| Instruction | Meaning |
|---|---|
| `or when CONDITION` | another branch inside `when` (before `otherwise`) |
| `while CONDITION` ... `done` | repeat while the condition is yes |
| `skip` | go on with the next turn of the innermost loop |
| `quit` | end the innermost loop |
| `add VALUE to NAME` | put an item at the end of the list in NAME |
| `set NAME.FIELD to VALUE` | change (or make) one field of a record; `a.b.c` works too |
| `set NAME at N to VALUE` | change item N of the list in NAME |
| `task NAME(A, B)` ... `done` | define a task (a function); only at the top of a page |
| `give VALUE` | end the task and give VALUE back |
| `fixed NAME is VALUE` | a constant, visible in every task; only at the top of a page |
| `use 'PAGE'` | load the tasks and fixed values of another page; first lines only |
| `fail VALUE` | stop with a mistake of your own (VALUE is the message) |
| `try` ... `instead NAME` ... `done` | if a mistake happens in the try part, run the instead part with the message in NAME |
| `test 'TITLE'` ... `done` | a test (only on test pages, see Tests) |
| `check CONDITION` | in a test: the test fails if CONDITION is no |
| `pretend 'PATH' holds TEXT` | in a test: the test sees this page with this text |

### Tasks

- Called like built-in functions: `invoice_total(m)`. A task called as a line of its own
  throws away what it gives.
- A task sees only its own inputs, its own names and the `fixed` values. It can't see or change
  the names of the code that called it (no hidden globals).
- Tasks can call each other and themselves (at most 40 calls deep).
- Tasks may use actions (`write`, `send`, ...); they are collected like every other action.
- Two tasks with the same name, in one page or across used pages, is a mistake found before
  anything runs.

### Modules

- `use 'desk:shop:money'` loads that page. A used page may only hold `use`, `fixed`, `task` and
  comments: no loose instructions, so loading a page never does anything by itself.
- Pages can use pages that use pages (at most 16 pages per program). Using a page twice is
  fine; a loop (`a` uses `b` uses `a`) is a mistake.

### Mistakes and try

- A mistake ends the program and nothing is changed (as chores). The Output screen shows:
  `Stopped: desk:shop:money line 5: number('12,5') is not a number` and below it
  `called from desk:shop:main line 7`.
- `try` ... `instead problem` ... `done` catches mistakes from the try part, including `fail`
  and mistakes in tasks it calls. `problem` is the message text.
- Limits (steps, actions, call depth, sizes) can't be caught: they always end the program.

### Built-in library

Everything from chores (`pages`, `read`, `field`, `body`, `exists`, `lines`, `words`,
`count`, `item`, `number`, `lower`, `today`, `weekday`, `add_days`, and the actions `write`,
`add`, `toss`, `send`, `event`), plus about 30 more:

- **text:** `upper`, `trim`, `split(text, by)`, `join(list, by)`, `replace(text, old, new)`,
  `find(text, part)` (position, 0 if not there), `slice(text, from, to)`, `starts(text, part)`,
  `ends(text, part)`, `newline()`, `text(value)` (any value as text)
- **numbers:** `round(n, decimals)`, `floor`, `ceil`, `abs`, `min(a, b)`, `max(a, b)`,
  `is_number(text)`
- **lists:** `sum`, `sort` (numbers or texts), `reverse`, `unique`, `first`, `last`,
  `slice(list, from, to)`, `range(from, to)`, `position(list, value)`
- **records:** `keys(record)`, `has_key(record, key)`, `fields(page)` (all `key: value` lines at
  the top of a page as one record)
- **dates:** `days_between(a, b)`, `month(date)`, `year(date)`

No functions as values in version 1 (no `map` with a task). Loops do that job.

### Limits (going over one stops the program, never the OS)

400 lines and 16000 characters per page; 16 pages per program; 300 tasks; 40 calls deep;
blocks 16 deep; 8000 characters per text; 2000 items per list or fields per record;
500 say lines; 300 actions; 500000 steps.

## 5. Tools inside the OS

| Command | What it does |
|---|---|
| `run <page>` | run an Errand program (Output screen as for chores) |
| `test <page>` | run every test on a test page: one line per test, `PASS` or `FAIL line N: ...`; for a failed `check a is b` it shows both values. Tests never change anything. |
| `lab` | a try-it-out screen: type one line, see the value; names and `use` are kept; actions are only shown (`would write desk:x`), never made |
| `view <page> [from] [to]` | show a page with line numbers |
| `edit <page> <line> <text>` | replace one line of a page |
| `insert <page> <line> <text>` | put a new line before line N (N = last + 1 adds at the end) |
| `cut <page> <line>` | remove one line |
| `guide errand` | the complete Errand reference, on screen |

Notes gets line numbers, Ctrl+G (go to line) and Ctrl+F (find), and a bigger page limit, so
fixing line 147 of a long page doesn't take 146 Down presses.

## 6. How it is built

- **A separate crate, `errand/`** (`no_std` with `alloc`). The kernel uses it through a small
  `World` adapter, like chores. Because it is a normal crate, all of it can be tested on the PC
  with `cargo test`, quickly, without QEMU.
- **Stages:** lexer, parser (all used pages), checker (names, task inputs, use loops, block
  shape, `give` outside a task, ...), then a tree-walking runner with a step counter.
- **Never crash:** sizes checked before allocating; every recursion (parser, checker, runner,
  `is` on nested lists) has a depth limit; the kernel stack is measured with the deepest
  program the limits allow, and made bigger if needed.
- **Chores untouched:** `do` keeps using `chore.rs`. The superset goal (section 2) is checked by
  running every chore test through both engines and comparing.
- **Size guess:** 6000-8000 lines of Rust plus 2000 lines of tests; the manual chapter about
  800 lines.

## 7. How it is tested

1. **Unit tests** for each feature and each mistake message (host, `cargo test`).
2. **Chore regression:** every chore test gives the same result in Errand.
3. **Fuzzing:** many random and broken programs; none may panic or run past a limit.
4. **End to end:** a robot player writes, tests and fixes programs in QEMU with keystrokes only.
5. **Sol review** of the design (this file), then of the code at each milestone.

While experiment 4 is running, no QEMU runs and no heavy builds on this PC (they could slow
Luna's games). Host unit tests are light and fine.

## 8. Milestones

| | What | Done when |
|---|---|---|
| M1 | core: values, lists, records, blocks, `while`/`skip`/`quit`, tasks, `fixed`, library | unit tests pass; chore regression passes |
| M2 | `use`, `fail`, `try`/`instead`, tests (`test`, `check`, `pretend`), mistake trail | unit tests + fuzzing pass |
| M3 | in the OS: `run`, `test`, `lab`, `view`/`edit`/`insert`/`cut`, Notes upgrades | robot end-to-end passes in QEMU |
| M4 | manual chapter + `guide errand` | a robot following only the manual examples gets them all working |
| M5 | pilot: Haiku and Luna on ~5 small Errand tasks | tasks are fair and the harness works (not a result) |

## 9. Later: Unseen-bench (SWE-bench style)

- **Codebases:** 3-5 small programs written in Errand (an invoice tool, a calendar planner, a
  contact cleaner), 100-400 lines each over several pages, each with a test page.
- **Issues:** each task plants one real-looking bug and writes a short issue note, like a user
  would ("the late list includes invoices that were paid").
- **Grading:** hidden tests, as in SWE-bench: **fail-to-pass** (fail before the fix, must pass
  after) and **pass-to-pass** (passed before, must still pass). Run inside the OS, on hidden
  worlds.
- **Verified:** every task is solved once by a robot applying the known fix (proves it is
  solvable and that the tests catch the bug), and checked by Sol for fairness.
- Conditions: with and without the manual, as in experiments 1-4.

## 10. Questions for Yentl

1. **Name:** Errand, or something else?
2. **How strange:** this draft is "familiar ideas, own spelling". It could be stranger (harder
   to guess, more reliance on the manual) or closer to Python (easier, less of a test).
3. **Superset of chores:** keep (recommended: free regression tests, one family of languages),
   or a clean break?
4. **Size of version 1:** everything above, or first M1-M2 only and then decide?
