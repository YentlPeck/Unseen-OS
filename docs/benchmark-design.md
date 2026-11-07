# Experiment 5: the Unseen App Benchmark (plan, 7 Nov 2026)

**Question (website step 8): how much does the manual help when an AI has to program for
real?** Experiment 4 asked it for chores. Here a model builds, extends and fixes full-screen
apps in Unseen Script, a language that exists nowhere else, graded by hidden tests in the
style of SWE-bench. Design background: `docs/script-design.md` §12; the pilot that showed the
tasks and harness work: `docs/pilot5.md`.

## Conditions

| Condition | What the prompt holds before the task |
|---|---|
| plain | nothing (the OS's own `guide script` is there to be found, as `guide do` was in experiment 4) |
| reference | the `guide script` text only (`kernel/src/script_guide.txt`) |
| manual | the whole manual (`docs/manual.md`) |

## Tasks (B1)

About 24 tasks, never used before (the pilot's three are not reused):

- **build** (8): make an app from a description;
- **feature** (8): add something to an existing app (its tests are on the desk);
- **fix** (8): fix a bug from a short report.

Spread over easy / medium / hard. Feature and fix tasks start from a few base apps (a list,
a score table, a page viewer, an agenda), each with a visible test page.

Every task has, as in the pilot (`tools/tasks_apps.py`): a start desk, the request, the
allowed pages, the data page(s), a known solution, hidden tests (fail-to-pass and
pass-to-pass), and wrong solutions that the hidden tests must catch.

**Fair and checkable** (design §12):
- everything the hidden tests check is said in the request or follows from the existing app;
- hidden tests run on other data than the visible desk, so hard-coding fails;
- grading copies only the player's pages under `desk:apps` into a fresh desk; the verdict comes
  only from the OS's own `N of M passed`; any change outside the allowed pages fails;
- every task is validated without AI: the start desk fails the fail-to-pass tests and passes
  the pass-to-pass tests, the known solution (typed by a robot through the real harness)
  passes everything, every wrong solution is caught;
- Sol checks every task for fairness.
- the harness adds a scope sentence to every request (`scope()` in `tools/tasks_bench.py`): extra
  pages of the player's own may stay in `desk:apps` (tests, pages loaded with `use`), but the
  app must not read or change pages in `desk:apps` while it runs (the hidden tests' worlds
  don't have them); the named data page may change while the player tries the app; nothing
  else may change. Hidden tests also check that an app adds no pages, mails or events nobody
  asked for, and leaves its input pages as they were.

**B1 done (7 Nov):** 24 tasks (`tools/bench_apps.py`, `tools/tasks_bench.py`,
`tools/tasks_bench_build.py`), grader `tools/apps_grade.py`, validator `tools/bench5.py`.
Validation in QEMU (solutions typed for real): ALL GOOD, 359 checks; 87 named wrong solutions
and 96 planted side effects (an unasked event or page, in start and in key) all caught. Sol
checked fairness four times (10, 10, 4 and 1 findings, all fixed): every hidden test also ends
with the side-effect checks (`guarded()`). Difficulty labels stay provisional until B3.

## Harness (B2)

Games run inside Inspect as in experiment 4 (`tools/inspect_eval.py`'s pattern: one sample
per task and condition, a re-run for a harness error, an attempts ledger, frozen files), with
the app referee and tool of pilot round 2 (`tools/broker_apps.py`, `tools/play_apps.py`: the
`line` action and the one-command rule). Experiments 1-4 keep their files unchanged.

**B2 done (7 Nov):** `tools/apps_game.py` (one game), `tools/inspect_apps.py` (the Inspect task
`unseen_apps`), `tools/bench_robot.py`, `tools/audit_apps.py` (the one-command rule: exactly one
`python play.py <game> ...` per command, on top of the frozen audit), `tools/test_apps_harness.py`
(the attempt rules, with fake games). Compared with experiment 4's runner, the ledger is safe for
two Inspect processes (an interprocess lock; a cut-off last line is put aside), attempts are
rebuilt from the ledger after a restart, and an interrupted game keeps what its records show (an
audit failure, an OS crash, a harness error, a time-out). The robot passed 72 of 72 games (24
tasks x 3 conditions) through Inspect. Sol reviewed it three times (5, 2 and 0 findings).

## Calibration (B3)

A small run on separate tasks (written for calibration, never reused): set the action and
time budgets (the pilot used 200 actions and 45 minutes) and check that difficulty is spread
out (not everything passes, not everything fails). Results go in `docs/calibration5.md`.

**B3 done (7 Nov):** `docs/calibration5.md`. 5 calibration tasks (`tools/tasks_calib.py`), 30 games
at 2 per player, then 20 at 4 per player. Decided: Haiku 4 games at a time, Luna 2 (Luna stalls at
4); 250 actions and 45 minutes per game; a game in which no action reached the computer is a
harness error; every build request says rows start at column 1. Pass rates were high in every
condition, so actions per passed game must be a pre-registered measure too.

## Pre-registration (B4)

`PREREG5.md`, written and frozen before the run: hypotheses (manual vs plain, manual vs
reference, reference vs plain), the primary measure (games passed), epochs (about 4), the
analysis, and the rules for re-runs and audit failures. Sol checks it; then `tools/freeze.py`
gets experiment 5.

## The run (B5) and analysis (B6)

Haiku and Luna, 3 conditions, every task, about 4 epochs: about 576 games, 2 at a time per
player (roughly 1-2 days). Then the analysis as in experiment 4, the write-up, and a results
page for the website (step 8).

## Milestones

| | What | Done when |
|---|---|---|
| B1 | the task set | every task validated, and Sol rates every task fair |
| B2 | the harness in Inspect | a robot plays every task through it and passes |
| B3 | calibration | budgets set, difficulty spread out |
| B4 | pre-registration | written, checked by Sol, frozen |
| B5 | the run | every game played, re-runs by the rules |
| B6 | analysis and results page | written up as experiment 4 was |
