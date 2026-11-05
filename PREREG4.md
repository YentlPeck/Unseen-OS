# Unseen OS experiment 4: rules written before running

Written 5 Nov 2026, **before any experiment-4 game has been played.** Nothing here changes after
the first experiment-4 game starts. Any later change is added at the bottom with a date and a
reason, and reported as a deviation.

This is a **follow-up on a known benchmark**: it reuses experiment 3's 16 tasks, world and
players, and the experimenter knows experiment 3's results (README, `out/experiment3/`).

## Question

> On chore tasks (too big to do by hand, needing a small program in a language specified nowhere
> else), does the full manual help small AI models more than the chore reference alone, which
> they could also read on the screen?

Experiment 3 found that the full manual helps a lot (wins 18/96 -> 59/96). Experiment 4 adds a
middle condition to see how much of that comes from simply having the reference in the prompt.
The comparisons are of **packages**, not single mechanisms (see Limitations). Design and Sol's
review: [docs/v4-design.md](docs/v4-design.md).

## Setup

| | |
|---|---|
| OS | Unseen OS 0.3, the same image as experiment 3 (hash in the manifest) |
| Players | **Haiku** (`claude-haiku-5-5` via `claude -p`) and **Luna** (`gpt-6-luna` via `codex exec`), with the same isolation as experiment 3 |
| Conditions | **plain**: nothing in the prompt (the reference is on the computer, `guide do`). **reference**: the chore reference in the prompt, introduced by "Here is the chore reference for this computer (the same text \`guide do\` shows):". **manual**: `docs/manual.md` in the prompt, introduced by "Here is the manual for this computer:" (as in experiment 3) |
| Reference text | the ```text block of the Chores chapter of `docs/manual.md`, line endings `\n`, ending with one `\n` (`tools/reference.py`). The harness refuses to start unless it is identical to the `GUIDE` constant in `kernel/src/chores.rs` (the text `guide do` shows). 106 lines, 5,183 characters, SHA-256 `64b18729...b0283` |
| Tasks | the 16 test tasks of experiment 3 (`TASK_SETS["v3"]`), unchanged: 5 count, 5 bulk, 3 fix, 3 write; world seed 20261112, date 2026-11-12 |
| Games | each player plays each task **6 times** per condition: 2 x 16 x 3 x 6 = **576 games** |
| Limits | **100 actions** and 15 minutes per game |
| Prompt | as in experiment 3, with the condition's text (if any) where the manual was |
| Harness | Inspect, one eval per player, all three conditions in the same eval, `--epochs 6`, samples shuffled with seed **20261114** (so conditions are mixed in time), at most 2 games at once per player. Expected run time about 11-12 hours |

## Scoring

Exactly as in experiment 3 (PREREG3.md, "Scoring"): strict end state, scratch drawer
`desk:chores`, mails and events compared by content, one number for count tasks, independent
checkers for bulk tasks, and for fix/write tasks the player's chore must also pass on the same 3
hidden worlds.

## What counts

As in experiment 3 (PREREG3.md, "What counts"), with these changes:

- **Audit:** in PowerShell, a backslash is an ordinary character. It is now also allowed in
  unquoted words, but only in the game tool's arguments (`python play.py ...`), where it can at
  most change the text the tool receives, never what runs. **Comments:** an unquoted `#` at the
  start of a word (a comment in bash and PowerShell, also `<#`) makes the rest of the command
  fail, so quote marks inside a comment can't hide a command (found in Sol's final review; this
  hole also existed in experiments 2 and 3). Everything else is unchanged. Test cases: 31
  allowed, 51 rejected (`tools/audit.py`); every allowed PowerShell case is also run through the
  real `powershell.exe` (`tools/test_audit_argv.py`).
- **Keystrokes** are sent in batches of 6, and the next batch is only sent after the OS has
  handled every earlier key (`tools/unseen.py`). Experiment 3 lost keys in long actions (5
  harness errors). The stress test (`tools/stress_keys.py`) found 45% of keys lost in 426-key
  actions before the change and none afterwards (6 machines at once, 20,532 keys, text compared
  character by character).
- **A harness error ends the game at once** (the OS may be in a state the player never asked
  for), and the attempt gets one re-run as before. Whenever our side fails after the player has
  started (during the game, at the final check, in the clean-up or while checking hidden worlds),
  the player's commands are still audited, and an audit failure still stands (a loss, no
  re-run). Only a failure before the player starts skips the audit, because there is nothing to
  audit (`tools/test_harness_errors.py`).
- **Booking:** each attempt's game id is written to the ledger ("started") before the game
  begins, and its result is saved whole (written, then renamed). If an interruption comes after a
  game finished but before its ledger entry, the retry uses that saved result instead of playing
  again. Whenever the harness itself fails, before any re-run it looks for the game's saved result
  (also its temporary copy) and otherwise audits the player's own log: a saved result stands, and
  a known audit failure always stands (a loss). Only an attempt with neither a result nor a
  violation found is a harness error that may be played again; it is listed as discarded (as
  experiment 2 did with games in progress during its pause). The log parser keeps every command
  it has seen even if later log lines have an unexpected shape (those are a harness error), and
  a command the audit cannot read fails the audit.
- **Which games count:** as in experiment 3, plus: `tools/analyze4.py` also refuses a verdict
  unless every scored game was played in its own condition with the registered text (the game
  id names the condition, and the result records the condition and the text's SHA-256).
- `tools/run_experiment4.py` records the `claude` and `codex` CLI versions at the start and the
  end of the run (and at each `--retry`).
- Outages (PC off, network or provider down) are resumed with `--retry` and listed.

## Main result (decided in advance)

**Primary, and the only verdict: manual vs reference.**

- For each task: **d = manual win rate - reference win rate**, pooled over both players and 6
  games each (12 games per condition), so d is between -1 and +1.
- **D = the mean of d over the 16 tasks** (the average difference in win rate).
- **Test:** exact two-sided sign-flip permutation test over the 16 tasks (all 65,536 sign
  patterns of d), alpha 0.05. Tasks are the unit, because both players play the same tasks.
- **Interval:** 95% interval for D by inverting the same test: every shift delta for which the
  test on (d - delta) gives p >= 0.05 (a grid of 0.25-point steps plus the observed mean, each
  end refined by bisection to 0.001 points and rounded outwards to 0.1 points). This interval
  assumes a **location-shift model** (each task's difference is a common shift plus noise that is
  symmetric around zero); with effects that differ between tasks it is approximate. A task-level
  bootstrap interval for the mean (10,000 resamples, seed 20261114) is also reported.

| Verdict | Rule |
|---|---|
| **The manual helps more than the reference** | p < 0.05 and D > 0 |
| **The reference does better than the manual** | p < 0.05 and D < 0 |
| **No difference detected** | otherwise. Then, only if the upper ends of **both** 95% intervals are below +10 points, we also say "the reference is about as good, within 10 points" (an approximate statement, see the interval's assumption) |

A "no difference detected" result does **not** show that the reference is enough, unless the
10-point rule above holds.

**What this test can detect** (`tools/power4.py`, simulated with experiment 3's per-task manual
win rates and the same effect on every task): a 10-point benefit of the manual is found about
59% of the time, 15 points about 91%, 20 points or more about 99%. With no real difference, it
wrongly finds a difference (in either direction) 3.8% of the time. Effects that differ between
tasks are harder to detect than this model assumes.

## Also reported (descriptive, no verdict)

- **reference vs plain** and **manual vs plain** (the latter replicates experiment 3 under the
  fixed harness), with the same statistic, test and intervals, and experiment 3's per-task sign
  test for comparison.
- The same three comparisons for each player (with both intervals).
- Win rates per player and condition (out of 96), per task, and per task kind.
- In each condition: how many games opened `guide do`, and how many actions showed it (from each
  game's saved transcript, also for games recovered from the ledger).
- Actions used in won games (median); fix/write games that passed the visible world but not every
  hidden world.
- Every audit failure, OS crash, time-out, discarded attempt and game recovered from the ledger.
- Anything else is labelled **exploratory**.

## Calibration (practice tasks only)

Rules written before calibration: [docs/calibration4.md](docs/calibration4.md). 10 games (5
practice tasks x 2 players, reference condition only), technical pass rules, at most 2 rounds,
only harness bugs may be fixed between rounds.

## Limitations (known before running)

- **Package comparisons.** *reference vs plain* bundles finding the reference, the cost of
  reading it through the 25-line screen, and where it sits (prompt or screen). *manual vs
  reference* bundles the OS chapters (including how to save a page in Notes), the extra chore
  explanations, five worked examples, "things to know", tips and length. The reference itself
  already has one small worked example. No placebo (length-matched) condition.
- Same tasks, world and models as experiment 3, and the experimenter knows its results.
- 16 tasks are the independent units; more epochs make each task's rate more precise but do not
  add task families. The 5 count tasks all read invoice mails.
- Subscription CLIs whose model versions could change during the run (versions are recorded;
  conditions are mixed in time).
- Another project may run AI experiments with the same accounts on the same PC during the run.

## Frozen files

`python tools/freeze.py --exp 4` writes `out/experiment4/manifest.json` (refuses to overwrite)
with SHA-256 hashes of the OS image, this file, PREREG3.md (whose scoring rules this file uses),
the design and calibration documents, the manual,
the reference source (`kernel/src/chores.rs`, `tools/reference.py`), all task files and expected
end states, and all harness, runner, analysis, power and test code, plus a dated copy in `docs/`.
