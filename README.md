# Unseen OS

**A tiny operating system no AI has ever seen, built to test whether small AI models can
operate an unfamiliar computer, and whether reading the manual helps.**

Unseen OS is a small x86-64 operating system written in Rust. It boots in QEMU and has its own
vocabulary on purpose: folders are *drawers*, files are *pages*, the top drawer is the *desk*,
paths use `:` and saving is Ctrl+K. Because it didn't exist before November 2026, no model has
it in its training data. A model has to figure it out from the screen.

Version 0.3 has eight apps: a shell, **Notes**, **Files**, **Mail**, **Contacts**, **Calendar**,
**Settings** and a **Calculator**, plus **chores**: a tiny scripting language of its own.

![The Mail app](docs/img/v2-mail.png)

## Three experiments

| | Experiment 1 (v0.1) | Experiment 2 (v0.2) | Experiment 3 (v0.3) |
|---|---|---|---|
| Apps | shell, Notes, Files | + Mail, Contacts, Calendar, Settings, Calculator | + chores (scripting) |
| Tasks | 30 single-app tasks | 20 tasks, most needing 2-3 apps | 16 tasks too big to do by hand |
| Games | 120 | 240 | 192 |
| Pre-registered verdict | **The manual helps** (p = 0.013) | **No clear difference** (p = 1.0) | **The manual helps** (p = 0.0001) |
| What really happened | models nearly always won; the verdict came from the audit | models nearly always won, with or without the manual | without the manual, the big tasks were never solved |

Experiments 1 and 2 showed that **small models can operate a computer they have never seen**,
and that on easy tasks a manual mostly makes them faster. Experiment 3 made the tasks hard enough
to need a program in a language that exists nowhere else, and there **the manual made the
difference between failing and succeeding**: wins went from 18/96 to 59/96.

## Experiment 3: writing code in an unseen language

Unseen OS 0.3 adds **chores**: small programs typed into a page and run with `do <page>`. The
language uses familiar ideas (variables, loops, conditions), but its syntax, built-ins and page
model are new. Its complete reference is on the computer itself (`guide do`).

```text
set total to 0
for each m in pages('mail:inbox')
  when field(m, 'subject') is 'Invoice'
    set total to total + number(item(words(body(m)), 2))
  done
done
write('desk:answer', total)
```

The world has 100 contacts, 400 mails, 30 sent mails and 150 events. Each of the **16 tasks**
needs at least 130 actions without a chore, but players get only 100:

- **5 count** tasks (for example, "how many invoices are due on a weekend?"),
- **5 bulk** tasks (send many mails, add many events, write a phone book),
- **3 fix** tasks (repair a broken chore and run it),
- **3 write** tasks (write a reusable chore and run it).

A fix or write game only wins if the player's chore *also* works on 3 hidden worlds it never saw
(different sizes, nothing to find, another date). As before, each task was played 3 times
without and 3 times with the manual. The manual holds the same reference word for word, plus
explanations and 5 worked examples. Rules frozen before the first game: [PREREG3.md](PREREG3.md).
Language design and reviews: [docs/v3-design.md](docs/v3-design.md).

### Results

| | Without manual | With manual |
|---|---|---|
| Haiku wins | 5/48 | 25/48 |
| Luna wins | 13/48 | 34/48 |
| Median actions in won games *(not part of the verdict)* | Haiku 91 · Luna 47 | Haiku 51 · Luna 34.5 |

| Task kind (both players) | Without manual | With manual |
|---|---|---|
| count | 0/30 | 12/30 |
| bulk | 0/30 | 17/30 |
| fix | 11/18 | 15/18 |
| write | 7/18 | 15/18 |

**Pre-registered verdict: "The manual helps."** Per-task sign test: 15 tasks scored better with
the manual, none better without, 1 tied (p = 0.0001). The same holds for each player separately
(Haiku p = 0.0005, Luna p = 0.0002).

- **Without the manual, no count or bulk task was ever solved** (0 of 60 games). The models
  could find the reference on screen (`guide do`), but reading it 25 lines at a time and then
  writing a working chore didn't fit in 100 actions.
- **Fixing a chore is easier than writing one.** With a broken chore in front of them, the models
  won 11 of 18 fix games without the manual.
- No game crashed the OS, and no fix or write game worked on the visible world but failed a
  hidden one.

Results: [out/experiment3/results.json](out/experiment3/results.json). Every step of every game:
`inspect view --log-dir out/experiment3/logs`.

### Honest notes

- **28 games failed the audit** and count as losses. 23 were Haiku, all real rule breaks
  (`$(...)`, `$'...'`, shell variables), in both conditions. 5 were Luna: 2 real (unquoted
  brackets and `&` in PowerShell). The other 3 (all without the manual) reached the correct end
  state but were refused only for a backslash in an unquoted word. PowerShell treats a backslash
  as an ordinary character, so these are arguably **audit false positives**. They stay losses
  under the frozen rules. Counted as wins, Luna would be 16/48 without the manual, and the
  verdict would not change (still 15 tasks to 0).
- **2 games timed out** (15 minutes). **5 attempts were discarded** and re-played under the rules
  because the OS lost keystrokes in long games (a harness error, not the player's fault). One
  game hit this twice and was re-played with `eval-retry`. All are listed in results.json.
- The 5 count tasks all read invoice mails, so they are not independent of each other.
- No placebo (length-matched) control: "manual" adds the reference in the prompt, worked examples
  and prompt length together. Without it, the reference was only on the screen.
- Calibration on 5 separate practice tasks found and fixed an audit bug before the experiment
  ([docs/calibration3.md](docs/calibration3.md)). GPT Sol reviewed the language, the tasks and the
  harness before freezing.
- Another project ran AI experiments with the same accounts on the same PC during the run.

## Experiment 2: multi-app tasks

Two small models, **Claude Haiku** and **GPT Luna**, played **20 tasks** that need several apps
together (e.g. "Lina asked to move a lesson (see your mail). Change it in the calendar, then reply
to Lina with the new date."). Each task was played **3 times without and 3 times with** the
[manual](docs/manual.md): 240 games, at most 60 actions each. Players could only look at the
80x25 text screen and press keys. Every game was scored on the real end state of the OS's files.

The rules were written down and all files frozen (SHA-256 manifest) **before** the first game:
see [PREREG2.md](PREREG2.md).

### Results

| | Without manual | With manual |
|---|---|---|
| Haiku wins | 59/60 | 58/60 |
| Luna wins | 59/60 | 58/60 |
| Median actions in won games *(not part of the verdict)* | Haiku 29 · Luna 22 | Haiku 19 · Luna 14.5 |

**Pre-registered verdict: "No clear difference."** Per-task sign test: 1 task scored better with
the manual, 2 better without, 17 tied (p = 1.0). The same holds for each player separately.

Only 6 of 240 games were lost:

- **4 failed the audit** (the player used shell features the rules don't allow, such as
  `$(printf ...)` or a PowerShell wrapper with nested quotes). All 4 are listed in
  [results.json](out/experiment2/results.json).
- **2 were real mistakes, and both were the same mistake.** In `move-lesson` (with the manual),
  Haiku and Luna both pressed R to reply, then pressed Tab twice to get from "To" to the
  message. But a reply starts with the cursor *already in the message*, and the text screen shows
  no cursor. So both models typed their reply into the To or Subject field. The manual didn't
  say where a reply starts. **A hidden piece of screen state tripped up two different models in
  exactly the same way**, which is the most useful finding of this run for anyone building
  agent-friendly interfaces.

No game timed out and no game needed a harness re-run. Results:
[out/experiment2/results.json](out/experiment2/results.json). Every step of every game:
`inspect view --log-dir out/experiment2/logs`.

### Honest notes

- **Ceiling effect:** 97.5% of games were won in both conditions, so this experiment could not
  show a benefit on success even if one exists. Harder tasks are the next step (see below).
- **Pause:** the run was paused overnight on purpose (the PC was switched off) and resumed with
  Inspect `eval-retry`, as PREREG2 allows. 71 Luna games were finished and kept; the 2 games in
  progress were never scored and were played again. Details:
  [RUN-NOTES.md](out/experiment2/RUN-NOTES.md).
- During the second half, another experiment (a different project, also using Haiku and Luna) ran
  on the same PC.
- No placebo (length-matched) control: "manual" adds information and prompt length together.
- The harness was reviewed three times by a second model (GPT Sol) before launch, and every issue
  found was fixed before freezing.

## Experiment 1: single-app tasks

Haiku and Luna each played **30 tasks** (10 easy, 10 medium, 10 hard) in Notes, Files and the
shell, once without and once with the manual: 120 games, at most 40 actions each. Rules:
[PREREG.md](PREREG.md).

| | Without manual | With manual |
|---|---|---|
| **Official score** (pre-registered rules) | Haiku 20/30 · Luna 27/30 | Haiku 28/30 · Luna 29/30 |
| Score counting only real cheating *(exploratory)* | Haiku 28/30 · Luna 29/30 | Haiku 30/30 · Luna 29/30 |
| Median actions, tasks won in both *(exploratory)* | Haiku 18 · Luna 16.5 | Haiku 13 · Luna 13 |

**Pre-registered verdict: "The manual helps"** (exact McNemar test over 60 task pairs:
12 pairs won only with the manual, 2 only without, p = 0.013; task-level check p = 0.039).

**Important caveat:** this verdict is driven by the audit. Any command other than a plain call of
the game tool counted as a loss. 15 games were flagged, 12 of them without the manual, and 14 of
the 15 were harmless shell habits (mostly Haiku writing `cd . && python play.py ...`). Only one
was real tool misuse (Luna called Codex's built-in computer-use tool once). Counting only real
misuse, the difference is not significant (3 vs 1 pairs, p = 0.63). Experiment 2 used a fairer
audit that was written down in advance.

**What holds up most clearly** *(exploratory)*: among tasks won in both conditions, the manual
version needed fewer actions in 47 of the 49 pairs where the count differed.

Results: [out/experiment/results.json](out/experiment/results.json). Logs: `out/experiment/logs/`.

### Honest notes (experiment 1)

- One game per cell; no placebo control. See *Limitations* in PREREG.md.
- 6 pilot games were played while building the harness; they are listed in PREREG.md and the
  result also holds without the 2 pilot tasks (p = 0.023).
- Two start attempts failed before any game was scored (a path format error, then a clean-up bug
  that killed the OS before scoring). Both are documented in PREREG.md; the experiment was
  restarted from scratch with a new freeze each time.
- **Players were not fully isolated.** Haiku ran with the owner's Claude Code plugins loaded,
  including a memory plugin (claude-mem) that added the same start-up text to all 60 Haiku games.
  That text contained no task words, but it is extra context we did not intend. Luna had Codex's
  computer-use tool available. Both were fixed for experiment 2: every game runs from a fresh,
  empty setup folder that holds only the login.

## How it works

```
 AI player ──► play.py ──► referee (broker.py) ──► QEMU ──► Unseen OS
 (Haiku/Luna)  look/type/   token, 1 action at a     keyboard    shell + 7 apps,
               press only   time, limits, audit      + inspector in-memory file system
```

- **The OS** (`kernel/`): multiboot start-up code that switches the CPU to 64-bit mode, a VGA text
  screen, a polled PS/2 keyboard, an in-memory file system, a shell and seven apps built on a
  small shared UI kit (`ui.rs`: text fields, forms, lists). A hidden serial "inspector" lets the
  harness read the screen as text and dump all files for scoring.
- **The referee** (`tools/broker.py`) is the only thing connected to the OS. The player's folder
  holds nothing but the game tool and a secret token, and every player command is audited
  (`tools/audit.py`, with its own allowed/rejected test cases).
- **The tasks** (`tools/tasks.py`, `tools/tasks_v2.py`): each has start files, a plain-words
  request, a reference solution, and right and wrong example answers.
  `tools/validate_tasks.py` proves every task is solvable and that the strict scorer catches
  planted mistakes.
- **Chores** (`kernel/src/chore.rs`): a tokenizer, parser and interpreter in `no_std` Rust, with
  hard limits on steps, text, lists and depth. A chore's changes apply all at once, and only if
  it succeeds. Host tests (`chore-tests/`) include 40,000 fuzzed programs.
- **The harness** is [Inspect](https://inspect.aisi.org.uk/) (`tools/inspect_eval.py`), with an
  append-only ledger of every attempt and frozen-file checks before running and analysing.

## Run it yourself (Windows)

Needs Rust, QEMU (`C:\Program Files\qemu`) and Python 3.12 with `inspect-ai`.

```powershell
.\run.ps1                                  # build the OS and open it in a QEMU window
python tools/validate_tasks.py             # prove all tasks are fair
inspect eval tools/inspect_eval.py -T player=robot -T taskset=v2 --model mockllm/model
git checkout experiment-3                  # each experiment's exact state is a git tag
python tools/analyze3.py                   # re-check experiment 3's verdict from the logs
```

The analysis scripts refuse to run if any frozen file changed, so re-check an experiment from its
own tag (`experiment-1`, `experiment-2`, `experiment-3`): later versions of the harness change
shared files.

Built 4-5 Nov 2026 by Yentl Peck, directing Claude Opus (builder) and GPT Sol (reviewer).
