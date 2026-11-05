# Unseen OS v0.3 design: "chores", a tiny scripting language

**Status:** revised after review by GPT Sol, 5 Nov 2026. Nothing is built yet.

## Why

Experiments 1 and 2 hit a ceiling: about 97% of games were won with or without the manual, so
neither could show whether the manual turns failures into wins. v0.3 adds a scripting language
so tasks can be **too big to do by hand** within the action limit. It also asks a new question:

> Can a small model write working code for an unseen system, in a language specified nowhere
> else, and does the manual help?

The language uses familiar ideas (variables, loops, conditions), so models can bring general
programming skill. What is new is the exact syntax, the built-ins and the page model. We claim
an *unseen language and system*, not *unknowable concepts*.

## The language in one look

A **chore** is a page of instructions. `do <page>` in the shell carries it out.

```
-- count Omar's mails and save the answer
set total to 0
for each m in pages('mail:inbox')
  when field(m, 'from') is 'omar'
    set total to total + 1
  done
done
say 'Omar sent ' & total & ' mails'
write('desk:answer', total)
```

## Language rules (the contract)

**Lines and blocks**
- One statement per line, at most 78 characters (the editor's line limit).
- Blocks end with `done`; indentation is ignored.
- Keywords and variable names ignore capitals.
- `--` starts a comment.

**Statements**
- `set X to E`, `say E`, `stop`.
- `when C` … `otherwise` … `done`
- `for each X in L` … `done`
- `repeat N times` … `done`
- Action calls.

There is no `while` and there are no user functions, and every variable is global. `stop` ends the
whole chore *successfully*.

**Values**
- **Numbers:** whole numbers and decimals, shown the way the Calculator shows them (`163`, `12.5`).
- **Text:** written in `'...'` or `"..."`. Both work, so models can pick whatever quotes are easier
  in their shell. There are no escapes, so text can't contain its own quote mark.
- **Yes/no:** `yes`, `no`.
- **Lists.**

**Operators, from strongest to weakest**
1. `( )`
2. unary `-`
3. `* /`
4. `+ -`
5. `&`
6. `is`, `is not`, `<`, `>`, `<=`, `>=`, `has` (one comparison at a time)
7. `not` (so `not x is y` means "x is not y")
8. `and`
9. `or`

`and` and `or` stop early once the answer is known. Types are strict:
- `+ - * /` need numbers. `< > <= >=` compare two numbers, or two texts in alphabetical order
  (so dates written YYYY-MM-DD compare correctly).
- `&` joins anything into text.
- `is` compares two values of the same type exactly (capitals count).
- `has` is substring search in text (capitals count; use `lower()` to ignore them), or, with a
  list on the left, "is one of the items".
- `when` needs yes/no.

Any other type mix is an error that names the line, for example
`line 4: + needs two numbers, got text 'abc'`.

**Built-ins (reading)**

| Built-in | Contract |
|---|---|
| `pages(drawer)` | the pages directly in that drawer, full paths, in natural name order (`m2` before `m10`). This is a snapshot taken when called. Page order is *not* date order. |
| `read(page)` | the whole page text. A missing page is an error. |
| `field(page, key)` | value of the `key: value` header line (header = lines before the first blank line, as in `ui::field`). A missing key gives empty text `''`. |
| `body(page)` | the text after the first blank line (`''` if the page has none; use `read` for plain pages). |
| `exists(page)` | yes/no. |
| `lines(t)`, `words(t)` | lists of text. |
| `count(x)` | the length of a list or text. |
| `item(list, n)` | element n, counting from 1. Out of range is an error. |
| `number(t)` | text to number. Bad text is an error. |
| `lower(t)` | text in lower case. |
| `today()` | the date as `YYYY-MM-DD`, from a frozen experiment clock (not the PC's clock). |
| `weekday(d)` | the day name, e.g. `monday`. |
| `add_days(d, n)` | a new date `n` days later (or earlier, if n is negative). |

**Built-ins (actions)**

| Action | Contract |
|---|---|
| `write(page, text)` | create or replace a page outside the app drawers. Numbers are written as shown above. |
| `add(page, line)` | add a line to the end of such a page (creating it if needed). |
| `toss(page)` | remove such a page. App records can't be tossed or edited by chores. |
| `send(to, subject, text)` | send through the same validation and storage code as Mail. Unknown contacts are **refused** (an error; there is no override in v0.3). Field lengths and characters are checked as in Mail. |
| `event(date, time, title, with)` | add an event through Calendar's code, with the same checks. |

Paths always start from the desk (`mail:inbox`, `desk:answer`). The shell's current drawer does
not matter.

**All or nothing.** The whole chore is checked for syntax errors before anything runs. While it
runs, it reads the world *as it was when it started*. Its actions are collected and carried out
together **only if it finishes without an error** (or reaches `stop`). If it fails or hits a limit,
nothing changes. So a half-finished chore can't send 12 of 20 invitations, and running it again
after a fix can't send duplicates.

**Limits** (each gives a normal error, never a crash):

| Limit | Value |
|---|---|
| Source | 4 KB / 120 lines |
| Nesting | 16 deep |
| Text | 4 KB per value |
| List | 1,000 items |
| Output | 200 `say` lines |
| Actions | 200 |
| Steps | 50,000, charged per expression part, loop turn and built-in item |

Sizes are checked *before* memory is allocated. Brackets inside brackets are limited to 12 and an
expression tree to 48 levels. The worst case measured on the PC needs under 96 KB of stack, so v0.3
raises the kernel stack from 64 KB to 256 KB. 50,000 steps run in well under a second, so the harness never waits
on a busy OS.

**Running:** `do <page>` opens an Output screen. It shows the `say` lines, then either
`finished: N steps, M changes made` or the error with its line number, plus `nothing was
changed`. Up/Down scrolls; Esc goes back.

**Size estimate:** about 1,200 lines of no_std Rust (`kernel/src/chore.rs`). It needs:
- shared `mail::deliver` / `calendar::add_event` functions, which the apps will use too;
- an Output screen.

## Also in v0.3

- **Cursor position:** the inspector reports where the cursor is, and `play.py` adds a line
  under the unchanged screen text, e.g. `cursor: row 3, column 12`. In experiment 2 the only 2
  real losses came from a cursor the players couldn't see. To be tested on empty lines, full lines
  and the Mail reply form.
- **A full language reference inside the OS** (`guide do`, several browsable pages covering every
  built-in and rule). It is the same in both conditions, so no task depends on guessing the API.
- **Frozen clock:** the start file sets the date (`#today 2026-11-12`). `today()` and Calendar use
  it instead of the PC's local time.

## Experiment 3 (rules go in PREREG3, frozen before any test game)

**Conditions:**
- **plain:** the complete in-OS reference only, read through the screen at the cost of actions.
- **manual:** the full manual in the prompt, with explanations and worked examples on top of the
  same reference.

So the treatment is "a fuller spec, given for free", and the write-up will say that.

**World:** about 150 mails, 100 contacts and 80 events. Tasks are chosen so that `seek` and
browsing *can't* solve them within the budget. Each is checked with the best non-script strategy
we can find, run by the robot player.

**Task kinds, limited to what chores can do:**
1. **Add up or cross-check, then save the answer.** For example, total all invoice amounts in
   mails from jo; or count the people with events next week who aren't contacts.
2. **Bulk actions.** For example, invite every contact whose note says neighbour, or add a
   weekly event for every Monday in December.
3. **Fix a broken chore** (a saved page with a bug).
4. **Write a reusable chore**, saved at a path the task names.

**Scoring:**
- Every task: strict end-state check, as before. The prompt and scorer allow the declared chore
  pages; they are checked by behaviour, not by matching their text. Every other page is still
  checked exactly.
- Kinds 3 and 4: the harness also boots **3 fresh hidden worlds** (including one with zero
  matches and one with different counts). It copies in only the player's saved chore source,
  runs it unchanged with the same limits and clock, and checks the result. A win needs the
  visible world *and* all hidden worlds. Both are reported separately.
- Each coding task gets negative tests in `validate_tasks.py`: constant answers, wrong filters,
  missing outputs, duplicates, extra changes, and chores that pass only on the visible world. An
  independent second solution checks the scorer, so a single reference can't share the scorer's
  mistake.

**Budget:** measured, not guessed. Reference workflows run through `play.py` (open Notes, type,
save, run, read the error, one repair, run again), and the limit is set to that plus a margin.
Starting guess: 100 actions. A multi-key `press` counts as one action, as in experiments 1-2,
and that will be written into the rules.

**Shell quoting:** both quote styles work in the language. The prompt gives the same short,
task-independent note on typing quotes in both conditions. Representative chore-typing commands
are tested through both players' real shells and `audit.py` before freezing.

**Crashes:**
- The interpreter must never panic. A fuzz test with random and oversized programs runs before
  freezing.
- If a player's chore still crashes the OS, that game is a **loss** (listed, source kept), not a
  harness error, so it can't earn unlimited retries.
- Unrelated infrastructure failures follow experiment 2's rules.

**PREREG3 must also fix in advance:**
- the primary endpoint (per-task sign test, as in experiment 2);
- seeds and order;
- exclusions and retries;
- the smallest effect the design can detect, stated honestly (16 tasks is not 192 independent
  data points);
- separate calibration tasks;
- a calibration stopping rule written *before* calibration (target: roughly 30-70% wins without
  the manual, at most 3 calibration rounds, every round reported);
- the same isolation, freeze manifest, attempt ledger and run binding as experiment 2.

## Build order

1. **Language core:** `chore.rs`, with unit tests run on the host, fuzz tests and the limits.
2. **OS integration:** `do`, the Output screen, the shared Mail and Calendar functions, the
   cursor report, the frozen clock and the `guide do` reference.
3. **Manual chapter;** measure the real workflow cost with the robot player.
4. **Big world generator, tasks, hidden worlds, scorer changes and negative tests.**
5. **Sol review**, then calibration, PREREG3 and freeze, then the experiment.
