# Experiment 5, the Unseen App Benchmark: results

Run 7–8 Nov 2026 under [PREREG5.md](../PREREG5.md) (frozen manifest `9f2002be…`, commit 3fb7216).
Full report: `out/experiment5/analysis.txt`, made by `python tools/analyze5_d1.py`, which is the
frozen `tools/analyze5.py` with one formatting deviation ([deviations5.md](deviations5.md); no
result changes).

## Question

> When small AI models must **program for real** in a language that exists nowhere else (build,
> extend and fix full-screen apps in Unseen Script), how much does the manual help?

Two players (Haiku, Luna). 24 tasks (8 build, 8 feature, 8 fix). Three conditions:

- **plain**: nothing in the prompt. The reference is on the computer (`guide script`).
- **reference**: the 255-line Unseen Script reference in the prompt.
- **manual**: the full 961-line manual in the prompt. It contains the reference word for word.

Each player played each task 4 times per condition: 576 games, each graded by hidden tests.

## Verdicts (pre-registered)

| | Result | 95% interval | p | Verdict |
|---|---|---|---|---|
| **Primary**: pass rate, manual vs plain | **+10.9 points** | +6.2 to +15.7 (bootstrap +6.8 to +15.6) | 0.0001 | the manual helps |
| **Key secondary**: actions in won games, manual vs plain | **−43%** | −53% to −32% (bootstrap −53% to −33%) | < 0.0001 | the manual helps |

Both tests are sign-flip tests over the 24 tasks. "Exact" describes the enumeration of all 2^24
sign patterns. The test assumes that, with no effect, each task's difference is equally likely to
be positive or negative, independently of the other tasks. Feature and fix tasks share four base
apps, so the tasks are not fully independent, and the nominal 5% is less exact (PREREG5.md). No
task was left out of the action measure. The action measure compares won games only, and the
won games differ between conditions.

## Wins

| | plain | reference | manual |
|---|---|---|---|
| Haiku | 86 / 96 (90%) | 95 / 96 (99%) | 91 / 96 (95%) |
| Luna | 71 / 96 (74%) | 80 / 96 (83%) | 87 / 96 (91%) |
| build | 52 / 64 | 58 / 64 | 60 / 64 |
| feature | 47 / 64 | 55 / 64 | 54 / 64 |
| fix | 58 / 64 | 62 / 64 | 64 / 64 |
| hard tasks | 20 / 32 | 25 / 32 | 24 / 32 |

Median actions and minutes in won games:

| | plain | reference | manual |
|---|---|---|---|
| Haiku | 60 actions, 2.8 min | 35, 2.2 min | 34, 2.2 min |
| Luna | 55 actions, 4.2 min | 26, 2.4 min | 27, 2.4 min |

## Descriptive comparisons (no verdict)

- **Per player, manual vs plain:**
  - Luna: +16.7 points (p = 0.001), −43% actions.
  - Haiku: +5.2 points (p = 0.125; Haiku was already at 90% without help), −42% actions.
- **Reference vs plain:** +9.4 points (p = 0.002), −47% actions.
- **Manual vs reference:** +1.6 points (p = 0.63, 95% −2.9 to +6.3), +8% actions (p = 0.23,
  95% −5% to +22%). No significant difference was detected. Luna leaned to the manual
  (+7.3 points, p = 0.09) and Haiku to the reference (−4.2 points, p = 0.25), but neither is
  significant.

## What it means

1. **With the manual in the prompt, the models won more often** (the primary verdict), and their
   won games used fewer actions (median 34 vs 60 for Haiku, 27 vs 55 for Luna). The action
   measure compares different won games in each condition. It is not a measured saving per job,
   and it says nothing exact about time.
2. **"Plain" did not mean no documentation.** In 165 of 192 plain games the player opened
   `guide script` on the computer. That is the same 255-line reference the other two conditions
   got in the prompt. So the comparison is between being given the text and having it on the
   computer to look up. This experiment shows the difference, not its mechanism.
3. **No significant difference was detected between the manual and the reference** (intervals
   above). That does not show that they are equal, nor which part of the text matters.
4. **Luna's pass rate rose more than Haiku's:** +17 points against +5. Haiku was already at 90%
   in plain, so it had little room to improve.
5. **Part of the pass-rate gap comes from audit failures** (next section). Of the games that
   passed every hidden test but were lost to the audit, 9 were in plain, 8 in reference and 1 in
   manual. All of them count as losses under the registered rules.
6. **The hard tasks stay hard.** `recipes-find` was won in only 5 of 24 games, all by Haiku.

## Losses that were not failed tests

- **22 audit failures** (the one-command rule: every command must be exactly one literal
  `python play.py <game> …` call): 17 Luna, 5 Haiku.
  - Unquoted text that PowerShell would read as code (`[]`, `{…}`, `(…, -1)`).
  - A missing game id.
  - Commands chained with `cd … &&`.
  - By condition: 11 plain, 8 reference, 3 manual. Of these, 9, 8 and 1 games had passed every
    hidden test. Under the registered rules they all count as losses.
- **0 OS crashes, 0 time-outs.**

## Harness errors and re-runs (all registered)

- **6 Codex errors**: "Incomplete response returned, reason: max_output_tokens". All were Luna,
  all in plain. Each game got its registered re-run, and all six cells then finished: 4 won, 2
  lost. This is a registered rule, not a deviation (PREREG5.md, "What counts").
- **8 games cut off** by the planned stop on 7 Nov at 23:55 (an outage), with nothing to recover.
  They were discarded and re-run in round 1 (8 Nov, 11:43, `--retry`).
- **No cell ended errored.** No finished game was replayed.

## Limits

- **Two players, both small models.** Larger models may need the reference less.
- **24 tasks.** The tasks were written by us; easy tasks are close to the ceiling for Haiku.
- **The reference and the manual share the same 255 lines.** This experiment cannot say which
  parts of the reference matter.
- **The audit rule costs some games** that were functionally correct. The rule is the same in
  every condition, but its impact differed: 9 such games in plain, 8 in reference, 1 in manual.
- **The tasks are not fully independent.** Feature and fix tasks share four base apps.
