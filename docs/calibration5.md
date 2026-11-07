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

## Luna's speed (found and fixed after round 2)

Timing every log line (`player.times`, and the referee's time per action) showed where Luna's
time went. In two games on the same task (reasoning effort high and medium: almost the same,
so effort was not the cause), Luna wrote its first message after 6 seconds but ran its first
command only after 104.3 seconds, the same in both. Probes of Codex alone found the cause:

- **the Windows sandbox** (`sandbox = "unelevated"`, as in experiments 1-4) makes a new sandbox
  identity (`cap_sid`) in every fresh Codex home, and the first command then takes about 70
  seconds; later commands take 0.3 seconds. Every game has a fresh home, so every game paid it;
- **since the Codex update of 7 Nov**, a fresh home also downloads a 63 MB plugin catalog and
  installs remote plugins: extra tools in what should be a clean setup.

Fix, for app games only (`tools/apps_game.py`; experiments 1-4 unchanged): one saved sandbox
identity is copied into every game's fresh home (it is only the sandbox's identity: no memory,
nothing from other games), and plugins and every tool a player doesn't need are switched off
(`LUNA_FEATURES_OFF`; the exact command is kept as `player_cmd.json` in each game's record).
Luna's first command now comes after 5-9 seconds and its longest pause is about 10-30 seconds.

**Round 3: Luna, 4 games at a time, with the fix** (`out/calib5c`): **9/10 in 15 minutes** (was
7/10 in 63 minutes), no stalls, no harness errors. The loss was Luna's own bug (its start gave
the state before reading the page).

## Decisions for the experiment (go into PREREG5.md)

- **Parallel games: 4 per player** (Luna too, after the speed fix).
- **Actions: 250 per game** (was 200; the most used was 185). **Time: 45 minutes** (longest 15).
- **No action reached the computer** (0 actions, audit passed): a harness error (one re-run), not a
  loss (`tools/apps_game.py`).
- **Ceiling.** Nearly every game passed in every condition (pass rates 0.8-1.0); the manual mostly
  saved actions (Haiku: median 72 actions without it, 37-46 with it). The calibration tasks are
  easier than the experiment's hard ones (sort, search, week view, inbox, planner), but the primary
  measure should not be pass rate alone: PREREG5 should also pre-register actions (and time) per
  passed game, as experiments 1 and 2 did.
