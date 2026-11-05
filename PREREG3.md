# Unseen OS experiment 3: rules written before running

Written 5 Nov 2026, **before any experiment-3 game has been played.** No AI has played any of the
16 test tasks; only the robot player (reference solutions) has. Nothing here changes after the
first experiment-3 game starts. Any later change is added at the bottom with a date and a reason,
and reported as a deviation.

## Question

> On a computer they have never seen, with tasks that are too big to do by hand and need a small
> program in a language specified nowhere else ("chores"), does having the manual help small AI
> models succeed?

The chore language uses familiar ideas (variables, loops, conditions); its syntax, built-ins and
page model are new. The claim is about an unseen language and system, not about unknowable
concepts. Design and its reviews: [docs/v3-design.md](docs/v3-design.md).

## Setup

| | |
|---|---|
| OS | Unseen OS 0.3: shell, Notes, Files, Mail, Contacts, Calendar, Settings, Calculator, and **chores** (`do <page>`, the reference `guide do`) |
| Players | **Haiku** (`claude-haiku-5-5` via `claude -p`) and **Luna** (`gpt-6-luna` via `codex exec`) |
| Tool isolation | as in experiment 2: every game runs the player's CLI from a fresh setup folder holding only the login (now the freshest of the saved login and the owner's live login, compared by the login's own expiry), deleted after the game. Haiku: only `Bash(python play.py <game>:*)`. Luna: computer use, apps and memories off; reasoning effort high; the Windows sandbox setting that lets it run commands |
| Conditions | **plain**: no manual in the prompt. The complete chore reference is still on the computer (`guide do`), to be read through the screen. **manual**: `docs/manual.md` in the prompt, which contains the same reference word for word plus explanations and 5 worked examples. So "manual" means: a fuller specification, given for free |
| Tasks | the 16 **test** tasks in `tools/tasks_v3.py` (`TASK_SETS["v3"]`): 5 *count* (save one number on `desk:answer`), 5 *bulk* (many mails, events or lines), 3 *fix* (repair a broken chore and run it), 3 *write* (write a reusable chore and run it). All use one generated world (seed 20261112: 100 contacts, 400 mails, 30 sent, 150 events; the date is frozen at 2026-11-12) |
| Games | each player plays each task **3 times** per condition: 2 x 16 x 2 x 3 = **192 games** |
| Limits | **100 actions** and 15 minutes per game |
| Prompt | as in experiment 2 (`tools/run_task.py` PROMPT), plus one task-independent line for every v3 game: "When the text you type contains quote marks, put double quotes around the whole text and use single quotes inside it." |
| Harness | Inspect, one eval per player, both conditions in the same eval, `--epochs 3`, samples shuffled with seed **20261113**, at most 2 games at once per player |

## Scoring

- **Strict end state**, as in experiment 2, against `tools/expected.json` (recorded from the
  reference solutions). Pages the task leaves alone must stay exactly the same.
- **Scratch drawer:** every world has `desk:chores`. Pages in it are ignored by all checks,
  except the task's own chore page in *fix* and *write* tasks.
- **Mails and events** are compared by content (order of sending doesn't matter; duplicates count).
- *Count* tasks: `desk:answer` must contain exactly one number, equal to the right value.
- *Bulk* tasks: exactly the expected new mails / events / lines, checked by an independent
  checker in Python, as well as the strict end state.
- *Fix* and *write* tasks: the visible world must be right, **and** the player's chore page, as it
  is at the end of the game, is re-run unchanged on **3 hidden worlds** (`tools/hidden.py`): one
  random world with other drawer sizes (250 mails, 80 contacts, 120 events), one with nothing to
  find, and a small one with another date (2026-12-03). On each, the chore must finish, the
  task's checker must accept the result, and it must change nothing except the declared outputs
  (pages in `desk:chores` excepted). A game wins only if the visible world and all 3 hidden
  worlds pass.
- **No-chore baselines:** every count and bulk task's cheapest known no-chore strategy needs at
  least 130 actions (lower bounds, cost model in `tools/tasks_v3.py`; budget 100). Players may
  still try any approach; only the end state counts.

## What counts

- **Win:** the checks above pass, the audit is clean, the game did not time out, and the OS did
  not stop responding.
- **Loss:** everything else, including running out of actions or time.
- **Audit:** as in experiment 2 (`tools/audit.py`), with one change decided during calibration
  (a harness bug, see below). Each command is read the way its own shell reads it. A command
  that Codex wrapped as `powershell.exe -Command "..."` is unwrapped with the Windows
  command-line rules and its inside is read as PowerShell. Any other command is read as bash,
  where `\"`, `\\`, `\$` and `` \` `` inside double quotes are plain escapes (also when finding
  where one command ends). `!` is allowed in plain words. In PowerShell, a word starting with `@`
  (here-strings and splatting) fails. Everything else is unchanged: only calls of the game tool
  with literal arguments (and `cd`, `echo` of literal text, counting loops, silencing output,
  chaining). No `$` or backtick in double quotes, no shell variables in arguments, no pipes into
  other programs, no `if`, no other programs, no file access. Test cases: 25 allowed, 39
  rejected. Audit failures count as losses, are never re-run, and are all listed.
- **OS crashes are losses:** if the OS stops answering while QEMU is still running (after a
  player's action, at the final check, or while a chore runs on a hidden world), the game is a
  loss marked `os_crash`, with no re-run, even if a harness error happened in the same game,
  and is listed.
- **Refused commands are not harness errors:** if the model asks for a command that its CLI's
  configured allowlist refuses (for Haiku: anything that doesn't start with
  `python play.py <game>`), the model may carry on; if it stops, that is a normal loss.
- **Harness errors** get one re-run. A harness error is: our side failed (the OS, the referee,
  QEMU); or the player's CLI itself reported an error of its own (a missing or expired login, or
  its sandbox or policy blocking commands that the configuration allows); or the CLI did nothing
  at all (no command, no message, no game action). If that fails too, the sample
  errors and is re-played with `tools/run_experiment3.py --retry` (Inspect `eval-retry`). Every
  attempt is written to this experiment's own append-only ledger
  (`out/experiment3/attempts-ledger.jsonl`), and all discarded attempts are reported. No completed
  game is re-run, dropped or replaced. A finished game (a win or a loss) is recorded in the
  ledger at once; if an interruption loses Inspect's copy of it, the retry reuses that recorded
  game instead of playing again. Order of rules: audit failure, then OS crash, then harness
  error, then time-out.
- **Which games count:** as in experiment 2. `tools/run_experiment3.py` starts only once and
  records each player's Inspect task id in `out/experiment3/run.json`. `tools/analyze3.py`
  refuses a verdict unless: no frozen file changed; every log belongs to the recorded run and
  shows the registered settings (task set v3, both conditions, seed 20261113, 100 actions, the
  default manual, 3 epochs, 2 games at once, the frozen manifest hash), checked in every log of
  the run, including retries; there are no errored samples; there are exactly 192 unique games;
  and for every game, the ledger holds exactly one finished attempt for that cell, which is the
  scored game.

## Main result (decided in advance)

The same test as experiment 2, per task (16 tasks):

- For each task: **score = (manual wins - plain wins)**, summed over both players and all 3
  games (from -6 to +6).
- Test: exact two-sided **sign test** over the tasks with a non-zero score.

| Verdict | Rule |
|---|---|
| **The manual helps** | p < 0.05 and more tasks with a positive score |
| **The manual hurts** | p < 0.05 and more tasks with a negative score |
| **No clear difference** | otherwise |

**What this test can detect:** only large, consistent effects. If all 16 tasks differ, 13 must
go the same way (12 of 14, 10 of 12, 9 of 10, 8 of 8, 6 of 6). This tests these two models on
these 16 tasks; it does not claim to generalise.

## Also reported (descriptive, no verdict)

- Win rate per player and condition (out of 48), per task, and per task kind.
- The same sign test for each player separately.
- *Fix*/*write* games that worked on the visible world but not on every hidden world.
- How consistent the 3 repeats are; actions used in won games (median).
- Every audit failure, OS crash, time-out and discarded attempt.
- Anything else is labelled **exploratory**.

## Calibration (practice tasks only)

Rules written before calibration: [docs/calibration3.md](docs/calibration3.md), which also has
the full log. In short: 5 practice tasks on a different world (seed 20261112 + 999), never part
of the experiment. Round 1 (20 games) failed because our audit refused Codex's own command
wrapping and plain escapes. That was fixed as described above. Round 2 (20 games) met all
stopping conditions: no harness errors, 3 of 5 wins without the manual for each player, and no
losses caused by tasks, the OS or the checkers. Two harness problems with the login copy were
found and fixed before round 1 counted. Nothing else changed.

## Changes since experiment 2 (all decided before this file)

OS 0.3 adds chores and the `guide do` reference. Players see the cursor position under every
screen. Contacts shows and keeps City and Birthday. `seek` lists matching pages only (no text
excerpt). The desk holds at most 1 MB of text. The kernel stack is 256 KB. Also new: the
budget (100), the world, the tasks, the hidden-world scoring, the scratch drawer, the audit
change, OS-crash losses and the per-experiment ledger. Experiment 2's exact state is the git
tag `experiment-2`.

## Limitations (known before running)

- 3 games per cell; no placebo (length-matched) control: "manual" adds information, worked
  examples and prompt length together.
- The 5 count tasks all read invoice mails (with different logic: sum, maximum, weekday, date
  comparison, filtered sum), so they are not independent of each other.
- The no-chore baselines are lower bounds under a stated cost model, not proofs. Players are
  free to try other ways.
- Haiku's CLI refuses commands that don't start with `python play.py <game>`. In calibration,
  Haiku sometimes stopped after such a refusal. This is the same rule as experiment 2 and
  applies in both conditions.
- Another project ran AI experiments with the same accounts on the same PC during the build and
  calibration, and may still be running during the experiment.
- Fixed tasks written by the experimenter; two models; subscription CLIs whose model versions
  could change during the run (games are mixed in time).

## Frozen files

`python tools/freeze.py --exp 3` writes `out/experiment3/manifest.json` (refuses to overwrite)
with SHA-256 hashes of the OS image, this file, the calibration log, the manual, all task files
and expected end states, and all harness, runner, analysis and freeze code, plus a dated copy
in `docs/`.
