# Calibration for experiment 5 (milestone B3, 7 Nov 2026)

Five calibration tasks (`tools/tasks_calib.py`, never used in the experiment): build a temperature
converter (easy), build a guest book (medium), add Home/End to the shopping list (easy), fix Down
past the last player in the scores app (medium), add "Enter goes to the next day with events" to
the agenda (hard). All validated in QEMU like the experiment's tasks; the robot passed all 15 games
(5 tasks x 3 conditions) through Inspect. Records: `out/calib5` (round 1), `out/calib5b` (round 2).

## Round 1: both players, all conditions, 2 games at a time per player

| | plain | reference | manual |
|---|---|---|---|
| Haiku | 5/5 (median 72 actions, 4.9 min) | 5/5 (37, 2.3 min) | 3/5 (46, 3.6 min) |
| Luna | 4/5 (51, 6.3 min) | 5/5 (46, 5.3 min) | 5/5 (37, 5.5 min) |

27 of 30 passed, no harness errors. Longest game 15 minutes; most actions 185 (Haiku, plain).

- **Haiku's two losses (manual) were the requests' fault**: both apps worked but drew from column 3,
  as the manual's counter example does; the requests never said the column. Every build request
  (experiment and calibration) now says everything starts at column 1. In round 2 both games passed.
- **Luna's loss was a correct audit refusal**: in PowerShell, `\"` is not an escape, so
  `count(s.items)` stood outside the quotes, where `(s.items)` would run as a command (the frozen
  audit of experiments 1-4).

## Round 2: 4 games at a time per player (manual and plain)

- **Haiku: 10/10 in 12 minutes**, no harness errors: 4 at a time works for Haiku.
- **Luna: 7/10 in 63 minutes.** Three games stalled for about 40 minutes with few actions: two
  ended in Codex errors (`Incomplete response returned, reason: max_output_tokens`), were harness
  errors and passed when re-run; one reached the time limit and counted as a loss. Another game had
  no action reach the computer at all (Luna said `look` hung) and counted as a loss. None of this
  happened at 2 at a time (round 1), so it comes from running Luna 4 at a time, not from the tasks.

## Decisions for the experiment (go into PREREG5.md)

- **Parallel games: Haiku 4, Luna 2.** Games are independent, so different numbers per player
  change only the time it takes.
- **Actions: 250 per game** (was 200; the most used was 185). **Time: 45 minutes** (longest 15).
- **No action reached the computer** (0 actions, audit passed): a harness error (one re-run), not a
  loss (`tools/apps_game.py`).
- **Ceiling.** Nearly every game passed in every condition (pass rates 0.8-1.0); the manual mostly
  saved actions (Haiku: median 72 actions without it, 37-46 with it). The calibration tasks are
  easier than the experiment's hard ones (sort, search, week view, inbox, planner), but the primary
  measure should not be pass rate alone: PREREG5 should also pre-register actions (and time) per
  passed game, as experiments 1 and 2 did.
