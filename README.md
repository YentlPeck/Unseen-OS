# Unseen OS

**A tiny operating system no AI has ever seen, built to test whether small AI models can
operate an unfamiliar computer, and whether reading the manual helps.**

Unseen OS is a small x86-64 operating system written in Rust. It boots in QEMU and has its own
vocabulary on purpose: folders are *drawers*, files are *pages*, the top drawer is the *desk*,
paths use `:` and saving is Ctrl+K. Because it didn't exist before November 2026, no model has
it in its training data. A model has to figure it out from the screen.

Version 0.2 has eight apps: a shell, **Notes**, **Files**, **Mail**, **Contacts**, **Calendar**,
**Settings** and a **Calculator**.

![The Mail app](docs/img/v2-mail.png)

## Two experiments, two answers

| | Experiment 1 (v0.1) | Experiment 2 (v0.2) |
|---|---|---|
| Apps | shell, Notes, Files | + Mail, Contacts, Calendar, Settings, Calculator |
| Tasks | 30 single-app tasks | 20 tasks, most needing 2-3 apps |
| Games | 120 (1 per task, player and condition) | 240 (3 per task, player and condition) |
| Pre-registered verdict | **The manual helps** (p = 0.013) | **No clear difference** (p = 1.0) |
| What really happened | models nearly always won; the verdict came from the audit | models nearly always won, with or without the manual |
| Clear effect *(exploratory)* | manual = ~25-30% fewer actions | manual = ~35% fewer actions |

Both experiments point to the same conclusion: **small models can operate a computer they have
never seen, and a manual mostly makes them faster rather than more successful.** Both also hit
a ceiling: the tasks were too easy to show whether a manual turns failures into wins.

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
  `tools/validate_tasks.py` proves all 55 tasks are solvable and that the strict scorer catches
  1086 planted mistakes.
- **The harness** is [Inspect](https://inspect.aisi.org.uk/) (`tools/inspect_eval.py`), with an
  append-only ledger of every attempt and frozen-file checks before running and analysing.

## Run it yourself (Windows)

Needs Rust, QEMU (`C:\Program Files\qemu`) and Python 3.12 with `inspect-ai`.

```powershell
.\run.ps1                                  # build the OS and open it in a QEMU window
python tools/validate_tasks.py             # prove all tasks are fair
inspect eval tools/inspect_eval.py -T player=robot -T taskset=v2 --model mockllm/model
python tools/analyze2.py                   # re-check experiment 2's verdict from the logs
```

## Next: a scripting language inside Unseen OS

Both experiments hit a ceiling, so version 3 adds a tiny scripting language that runs inside the
OS. That allows tasks too big to do by hand within the action limit (for example, process every
mail or contact), and a new question: **can a small model write working code in a language it
has never seen**, with and without the manual?

Built 4-5 Nov 2026 by Yentl Peck, directing Claude Opus (builder) and GPT Sol (reviewer).
