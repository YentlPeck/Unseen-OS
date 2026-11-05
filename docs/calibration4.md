# Experiment 4 calibration (rules written before any calibration game)

Written 5 Nov 2026, before any calibration game. Experiment 4 adds one condition, **reference**
(the chore reference that `guide do` shows, given in the prompt). Calibration checks only that
this condition and the harness changes work end to end with the real players. It is not a pilot
of the result.

## Games

The 5 practice tasks (`TASK_SETS["practice3"]`, seed 20261112 + 999, never part of any
experiment) x 2 players x the **reference** condition only = **10 games** per round, 100
actions, with the experiment's settings (2 games at once per player). Ledger: `out/dev`.

```
inspect eval tools/inspect_eval.py --model mockllm/model -T player=<haiku|luna> -T taskset=practice3
    -T conditions=reference -T max_actions=100 --max-samples 2 --log-dir out/calibration4
```

## Pass rules (technical only)

A round passes if all of these hold:

1. Every game logged in and played (the player made at least one game action).
2. Every game's prompt contains the reference text exactly once, with its introduction line,
   and not the manual's introduction (checked in `out/runs/<game>/prompt.txt`).
3. Every result and score records condition `reference` and the reference text's hash.
4. No harness error that the harness should have prevented (lost keystrokes, a login copy
   problem, a crash of our code). A harness error caused by something outside our control
   (network, the provider's service) is listed and does not fail the round.
5. No audit failure that is the audit's own fault (judged against the written audit rules; real
   rule breaks are fine and are listed).

Win counts are reported, but they are **not** a pass rule.

## Limits

At most **2 rounds**. Between rounds, only harness bugs may be fixed: no change to the tasks,
the prompt, the conditions, the reference text, the manual or the scoring. If round 2 also
fails, experiment 4 is not run until the problem is understood, and this file says why.

## Log

### Round 1 (5 Nov 2026, 19:45-19:55): passed

Logs: `out/calibration4/2026-11-05T18-45-50-00-00_*.eval` (the 4 Nov file in that folder is
older work with other task sets, not part of this calibration).

| Rule | Result |
|---|---|
| 1. every game logged in and played | yes, 10 of 10 (14 to 100 actions) |
| 2. reference exactly once in each prompt, with its introduction, no manual introduction | yes, 10 of 10 |
| 3. condition `reference` and the reference hash in every result and score | yes, 10 of 10 |
| 4. no harness error the harness should have prevented | none at all (0 harness errors, 0 re-runs) |
| 5. no audit failure that is the audit's fault | yes: 4 audit failures, all real rule breaks |

The 4 audit failures:

- Haiku, p3-moving: `P=<game>; for l in "..." ...; do python play.py $P type "$l"; ...` (shell
  variables in arguments).
- Luna, p3-fix-family, p3-moving, p3-write-ashford: unquoted brackets inside PowerShell (the
  inner quotes of `\"...\"` end PowerShell's string early, or no quotes at all). Run through the
  real `powershell.exe`, these are evaluated as code: `('family', '')` became an array, and
  `field(c, 'city')` made PowerShell try to run `c` as a command. Same kind as experiment 3's real
  Luna failures.

Wins (reported, not a pass rule): Haiku 4 of 5, Luna 2 of 5 (its 3 other games failed the
audit).

No changes after this round.
