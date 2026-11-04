# Unseen OS experiment: rules written before running

Written 4 Nov 2026, **before any game with the manual has been played.**
Nothing in this file may change after the first experiment game starts. If something must
change, the change is added at the bottom with a date and a reason, and the result is
reported as "deviated from plan".

## Question

> Can small AI models operate a computer they have never seen, and does reading the
> manual help them?

## Setup

| | |
|---|---|
| Players | **Haiku** (`claude-haiku-5-5` via `claude -p`) and **Luna** (`gpt-6-luna` via `codex exec`) |
| Conditions | **plain**: no manual. **manual**: `docs/manual.md` included in the prompt |
| Tasks | all 30 in `tools/tasks.py` (10 easy, 10 medium, 10 hard) |
| Games | each player plays each task once per condition: 2 x 30 x 2 = **120 games** |
| Limits | 40 actions and 15 minutes per game |
| Tool | only `play.py` (look / type / press), through the referee; prompt in `tools/run_task.py` |
| Scoring | strict end-state check against `tools/expected.json` (`strict_check` in `tools/tasks.py`) |
| Harness | Inspect (`tools/inspect_eval.py`), one eval per player, both conditions in the same eval |
| Order | samples shuffled with a fixed seed (**20261104**), so plain and manual games are mixed in time |
| Parallel | at most 2 games at once per player (4 in total) |

## What counts

- **Win (PASS):** `strict_check` passes after the game, the audit is clean, and the game did
  not time out.
- **Strict check details:** the final files must match the expected end state exactly in
  which drawers and pages exist. Pages the task leaves alone must be byte-for-byte unchanged.
  Pages the task creates or edits may differ only in trailing spaces at line ends and blank
  lines at the very end. One page (`locker`: `letters:reply-mira`) is a free-text answer: its
  first line must contain the code 2391 as a whole number and no other code.
- **Loss (FAIL):** everything else, including running out of actions or time.
- **Audit failure:** the player did anything other than call the game tool (any other
  command, file edit, MCP or web use, changing `play.py`/`game.json`). Counted as **FAIL**,
  listed in the report, and never re-run, even if the same game also had a harness error.
- **Harness error:** our side failed (the OS did not start, the referee or bridge failed, a
  settle failure, the player CLI exited with an error or reported an error). That game is
  played once more, the same way. If the second attempt also has a harness error, Inspect
  marks the sample as errored. Errored samples are re-played with
  `python tools/run_experiment.py --retry` (Inspect `eval-retry`) until none are left. This
  includes games stopped by a subscription usage limit: the run is paused and resumed later.
  No completed game is ever re-run, dropped or replaced. Every attempt is listed.
- **Which games count:** only the latest log per player in `out/experiment/logs/`.
  `tools/analyze.py` refuses to give a verdict unless there are exactly 120 unique games
  (2 players x 30 tasks x 2 conditions), no errored samples, the logs record the frozen
  manifest hash, and no frozen file has changed.

## Main result (decided in advance)

For each task and player we have a pair (plain result, manual result). Over all 60 pairs
(both players together):

- **b** = pairs where manual PASS and plain FAIL
- **c** = pairs where plain PASS and manual FAIL

Test: exact two-sided McNemar test (binomial test on b out of b + c, p = 0.5).

| Verdict | Rule |
|---|---|
| **The manual helps** | p < 0.05 and b > c |
| **The manual hurts** | p < 0.05 and c > b |
| **No clear difference** | otherwise |

**What this tests:** whether including this manual changed results **for these two models on
these 30 tasks**, one game per cell. It does not claim to generalise to other tasks or models.
Because both players play the same tasks, their results are not fully independent, so a
task-level check is also reported (below). If the task-level check disagrees with the main
verdict, the write-up says so in the first paragraph.

## Also reported (descriptive, no verdict)

- **Task-level check:** for each task, (manual wins - plain wins) over both players; a two-sided
  sign test over tasks with a non-zero score.

- Pass rate per player, condition and level (easy / medium / hard).
- The same McNemar numbers per player (30 pairs each).
- Actions used, for games won in both conditions (median per condition).
- Every audit failure, harness error and timeout.
- The pilot games (below) and a version of the main result **without** the 2 pilot tasks.

## Pilot games already played (no manual)

All 6 paid games played on 4 Nov while building the harness, before this file was written.
None used the manual. Harness versions changed between them; none of them count.

| Time | Game | Harness version | Result | Actions | Audit |
|---|---|---|---|---|---|
| 19:26 | copy-list, Haiku | phase 3 (no referee) | PASS | 19 | not audited |
| 19:27 | copy-list, Luna | phase 3 (no referee) | PASS | 19 | not audited |
| 19:39 | change-word, Haiku (via Inspect) | phase 4 | PASS | 20 | not audited |
| 19:39 | copy-list, Haiku (via Inspect) | phase 4 | PASS | 17 | not audited |
| 19:52 | change-word, Haiku | referee version | PASS | 19 | clean (chained game calls) |
| 19:52 | copy-list, Luna | referee version | PASS | 15 | clean |

`copy-list` and `change-word` stay in the experiment; the report also shows the main result
without them.

## Limitations (known before running)

- **One game per cell:** no estimate of how much a model's result varies from run to run.
- **No placebo control:** the manual adds both information and extra prompt length. The
  experiment cannot separate the two; it only answers "did including this manual help?".
- Fixed tasks written by the experimenter; two models; subscription CLIs whose model
  versions could change during the run (games are mixed in time to spread this out).

## Frozen files

Before the first game, `tools/freeze.py` writes `out/experiment/manifest.json` with the
SHA-256 hash of every file that affects the result (OS image, tasks, expected end states,
manual, prompt, harness, experiment runner, analysis and freeze code), and keeps a dated copy
in `docs/`. It refuses to overwrite an existing manifest. Every experiment log records the
manifest's hash, and `tools/run_experiment.py` and `tools/analyze.py` refuse to run if any
frozen file has changed.

## Changes after the first freeze

- **4 Nov, 20:10 → re-frozen:** the first freeze (`docs/manifest-20261104-201011.json`) was
  followed by a start attempt that crashed **before any game was played** (Inspect does not
  accept an absolute path to the task file; no log, no player process, no game). Fix: one line
  in `tools/run_experiment.py` (pass `tools/inspect_eval.py` as a relative path). Nothing else
  changed. The experiment was then frozen again; the new manifest is the one that counts.
- **4 Nov, second start aborted:** the run started at about 20:15 and was stopped at about
  20:55. A harness bug made **every** game attempt end in a harness error: the clean-up step
  that kills leftover player processes also matched the OS copy (its command line contains
  the game id), so the OS was killed right before the files were read. **No game was scored.**
  The players did play, but no outcome was computed or looked at. The aborted logs are kept in
  `out/experiment/aborted-1/`. Fix: clean-up now only matches `play.py <game id>` processes
  and never QEMU. New test player `robot` (`tools/robot_player.py`) plays the reference
  solutions through the real `play.py` path, so the clean-up, audit and scoring path is tested
  without cost: 60/60 robot games passed (no errors, clean audits) before the restart. The
  models keep no memory between games, so the aborted attempts cannot influence the restart.
  The full experiment was restarted from scratch with a new freeze.
