# Unseen OS experiment 5, the Unseen App Benchmark: rules written before running

Written 7 Nov 2026, **before any experiment-5 game has been played.** Nothing here changes after
the first experiment-5 game starts. Any later change is added at the bottom with a date and a
reason, and reported as a deviation.

## Question

> When small AI models must **program for real** in a language that exists nowhere else (build,
> extend and fix full-screen apps in Unseen Script), how much does the manual help?

Earlier experiments asked this for using the OS (1, 2) and for small programs called chores (3,
4). Experiment 5 asks it for apps, graded by hidden tests in the style of SWE-bench. Plan and
milestones: [docs/benchmark-design.md](docs/benchmark-design.md); the pilot:
[docs/pilot5.md](docs/pilot5.md); calibration: [docs/calibration5.md](docs/calibration5.md).

## Setup

| | |
|---|---|
| OS | Unseen OS 0.3 with Unseen Script (milestones M1-M7), the image `out/unseen.bin` (hash in the manifest) |
| Players | **Haiku** (`claude-haiku-5-5` via `claude -p`) and **Luna** (`gpt-6-luna` via `codex exec`, reasoning effort high), with experiment 4's isolation, plus for Luna: one saved Windows-sandbox identity copied into every game's fresh Codex home, and plugins and every tool a player doesn't need switched off (`tools/apps_game.py`, docs/calibration5.md). The runner sets reasoning effort high and requires the saved identity whatever the environment says; every Luna game records its setup, and the analysis refuses a verdict if one differs |
| Conditions | **plain**: nothing in the prompt (the reference is on the computer, `guide script`). **reference**: the Unseen Script reference in the prompt, introduced by "Here is the reference of Unseen Script, the programming language of this computer (the text the command guide script shows):". **manual**: `docs/manual.md` in the prompt, introduced by "Here is the manual for this computer:" |
| Reference text | `kernel/src/script_guide.txt`, the text `guide script` shows; the manual holds it word for word (checked by `script/tests/reference.rs`). 255 lines, 13,545 characters, SHA-256 `6491d4ac...c7d4c` |
| Manual | `docs/manual.md`, 961 lines, 43,112 characters, SHA-256 `bb78d5ae...558f8` |
| Tasks | the 24 tasks of `tools/tasks_bench.py` (8 build, 8 feature, 8 fix; easy / medium / hard as labelled there), never played by any AI before this run |
| Games | each player plays each task **4 times** per condition: 2 x 24 x 3 x 4 = **576 games** |
| Limits | **250 actions** and **45 minutes** per game |
| Prompt | `PROMPT` in `tools/apps_game.py`: the game tool with the `line` action, the one-command rule, the condition's text (if any), the task's request and its scope sentence (`scope()` in `tools/tasks_bench.py`) |
| Harness | Inspect (`tools/inspect_apps.py`), one eval per player, all three conditions in the same eval, `--epochs 4`, samples shuffled with seed **20261108** (so conditions are mixed in time), at most **4 games at once per player**. Started with `tools/run_experiment5.py`. Expected run time about half a day |

## Scoring

A game is **won** if all of these hold:

- **every hidden test passes.** Grading copies the player's pages under `desk:apps` into a fresh
  desk and runs the task's hidden test page there with `test`; the verdict comes only from the
  OS's own `N of M passed` in the title bar (`tools/apps_grade.py`). Every hidden test also checks
  at its end that the app added no pages, mails or events nobody asked for;
- **nothing outside the allowed pages changed** on the player's desk (the task's `allowed`
  entries: `desk:apps:` and the named data pages);
- **the audit passed** (`tools/audit_apps.py`: every command is exactly one literal
  `python play.py <this game> ...`, on top of experiment 4's audit, `tools/audit.py`);
- it did not reach the time limit, and the OS did not stop responding.

## What counts

As in experiment 4 (PREREG4.md, "What counts"), with the app harness:

- an **audit failure** always stands (a loss, no re-run), then an **OS crash** (a loss);
- a **harness error** (our side, or the player's CLI failing, or a game in which no action reached
  the computer) ends the game and gets **one re-run**; a second harness error leaves the sample
  errored. Attempts are counted per **retry round**: a restart of the same round keeps the attempts
  it used, and each explicit `run_experiment5.py --retry` starts the next round, in which such a cell
  gets two more attempts. A finished game is never replayed in any round, and every attempt of
  every round stays in the ledger and is listed in the results. A game that was interrupted gets
  its outcome from its own records under the round it was booked in: an audit failure, an OS
  crash or a time-out stands as a loss (with Luna's setup, which is written before the player
  starts); a harness error, or records that show nothing (then the game is listed as discarded),
  count as one of that round's two attempts;
- every attempt is booked in the ledger before it starts (`out/experiment5/attempts-ledger.jsonl`,
  an interprocess lock, a cut-off last line put aside); a finished game is never replayed; after a
  restart, attempts already used are counted again; an interrupted game keeps what its records
  show (an audit failure, an OS crash, a harness error, a time-out) (`tools/inspect_apps.py`,
  `tools/test_apps_harness.py`);
- outages (PC off, network or provider down) are resumed with `--retry` and listed;
- `tools/analyze5.py` refuses a verdict unless: the frozen files are unchanged and run.json holds
  the frozen manifest; run.json says 4 epochs; each of the 576 cells has exactly one finished game,
  and no game is the final game of two cells; every game id names its own task, player and
  condition; every game used its condition's registered text (SHA-256); every won game has a
  positive action count; every Luna game had the registered setup; and the latest Inspect log of
  each player's bound run has the registered settings (player, task set, conditions, seed, 250
  actions, manifest, 4 epochs), all 288 samples scored without error, and each sample's game and
  score agree with the ledger;
- `tools/run_experiment5.py` records the `claude` and `codex` CLI versions at the start and the
  end of the run (at the time of writing: Claude Code 2.1.319, codex-cli 0.167.0).

## Main results (decided in advance)

The unit is the **task** (both players play the same tasks): 24 tasks.

**Primary: pass rate, manual vs plain.**

- For each task: **d = manual win rate - plain win rate**, pooled over both players and 4 games
  each (8 games per condition), so d is between -1 and +1. **D = the mean of d over the 24 tasks.**
- **Test:** exact two-sided sign-flip permutation test over the 24 tasks (all 2^24 sign patterns),
  alpha 0.05. "Exact" describes the enumeration; the test is valid if, with no effect, each task's
  difference is as likely to be positive as negative, independently of the other tasks. Feature
  and fix tasks share four base apps, which can make tasks less independent than that and the
  nominal 5% less exact.
- **Interval:** 95% interval for D by inverting the same test: the shifts whose test gives
  p >= 0.05, assuming a location-shift model (each task's difference is a common shift plus noise
  symmetric around zero), as in experiment 4. Each end is the first rejected shift found (steps of
  0.0025, bisection to 0.0001), rounded outwards to 0.001. With 5 or fewer tasks no shift can be
  rejected (the smallest possible p is 2/32 > 0.05), and the interval is reported as unbounded.
  Also a task-level bootstrap interval (10,000 resamples, seed 20261108).

**Key secondary (its own verdict): actions in won games, manual vs plain.** Calibration found
nearly every game won in every condition, with the manual mainly saving actions, so this is
registered as a second verdict.

- For each task: **r = the mean, over the players who won the task in both conditions, of
  ln(median actions of their won manual games / median actions of their won plain games)**. Tasks
  where no player won in both conditions are left out; the left-out tasks, the players used for
  each task and their numbers of won games are reported.
  **R = the mean of r**, reported as the change in actions, exp(R) - 1.
- The same test and intervals as the primary.
- **This compares the games that were won, not a causal effect on efficiency.** Within a task, the
  manual may let a player win hard, long attempts that it loses without the manual, which makes
  the manual look more expensive; a task won only with the manual drops out of this measure.

| Verdict | Rule |
|---|---|
| **The manual helps (pass rate)** | primary p < 0.05 and D > 0 |
| **The manual does worse (pass rate)** | primary p < 0.05 and D < 0 |
| **The manual saves actions** | secondary p < 0.05 and R < 0 |
| **The manual costs actions** | secondary p < 0.05 and R > 0 |
| **No difference detected** | for that measure, otherwise |

The two verdicts are reported separately and **neither is corrected for the other**: each answers its
own question ("does it make more games succeed", "does it make succeeding cheaper"), each at 5%.
There is **no experiment-wide 5% guarantee**, and **no combined rule**: the experiment does not
conclude "the manual helps" because either one of the two verdicts says so.

**What the tests can detect** (`tools/power5.py`: 24 tasks, 8 games per condition, plain rates
drawn between 0.45 and 0.95, the same effect on every task, capped at a rate of 1, so on tasks
near 1 the added points are smaller): a pass-rate benefit of 10 points gives the verdict "helps"
about 55% of the time, 15 points 88%, 20 points 99%. With no effect the test wrongly says "helps"
2% and "does worse" 2% of the time. Actions (assuming every task is won in 80% of games in both
conditions and the same factor on every task, spread of ln actions 0.45 as in calibration): 10%
fewer actions give "saves actions" about 40% of the time, 20% fewer 95%, 30% or more 100%; with no
effect, "saves" 3% and "costs" 2%. Calibration saw roughly 40-50% fewer actions with the manual.

## Also reported (descriptive, no verdict)

- **reference vs plain** and **manual vs reference**, both measures, with the same statistic and
  intervals.
- The same comparisons for each player.
- Wins per player and condition (out of 96), per task, per kind (build / feature / fix) and per
  level; median actions and minutes in won games.
- In each condition: how many games opened `guide script` (from the transcripts).
- Every audit failure, OS crash, time-out, harness error and re-run (with its retry round), and
  every game recovered from the ledger.
- The intervals of every descriptive comparison; wins per task (per player and condition).
- Anything else is labelled **exploratory**.

## Limitations (known before running)

- **Package comparisons**, as in experiment 4: *reference vs plain* bundles finding the reference
  and reading it through the screen; *manual vs reference* bundles the OS chapters, the worked
  examples (the manual's counter app among them), "things to know" and length. No placebo
  condition.
- **Ceiling:** calibration's easier tasks were won in most games in every condition, so the
  pass-rate verdict may find no difference even if the manual matters; the action verdict is there
  for that.
- 24 tasks are the units; feature and fix tasks share four base apps, so tasks are not fully
  independent. Epochs make each task's rate more precise, they don't add task families.
- The experimenter wrote the tasks, the language and the manual, and knows the pilot's and
  calibration's results; Sol checked every task for fairness.
- Subscription CLIs whose model versions could change during the run (versions are recorded;
  conditions are mixed in time). Luna's Codex setup differs from experiments 1-4 (sandbox
  identity reused, plugins off; docs/calibration5.md), so Luna's numbers are not directly
  comparable with earlier experiments.
- Another project may run AI work with the same accounts on the same PC during the run.

## Frozen files

`python tools/freeze5.py` writes `out/experiment5/manifest.json` (refuses to overwrite) with
SHA-256 hashes of exactly the files in `FILES` (`tools/freeze5.py`): the OS image, the manual, the
reference and the test that checks the manual holds it, this file and PREREG4.md (whose rules it
inherits), the plan and calibration documents, all task files, and every local module the
experiment's code imports (the harness, the runner, the analysis, the power simulation, the task
validator and the harness tests), plus a dated copy in `docs/`. The runner and the analysis refuse
to run if any of them changed.
