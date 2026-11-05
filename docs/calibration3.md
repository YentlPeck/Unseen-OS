# Experiment 3: calibration rules (written before any calibration game)

Written 5 Nov 2026, before any AI has played a version 3 task.

## What calibration is for

To check that the chore tasks are possible but not trivial for the two players, and to find
harness problems, **before** the experiment rules (PREREG3) are frozen. Calibration uses only the
5 practice tasks (`TASK_SETS["practice3"]` in `tools/tasks.py`), on a different world from the
test tasks. The 16 test tasks are never played by an AI before the experiment; only the robot
player (reference solutions) runs them.

## A round

- Both players (Haiku and Luna), all 5 practice tasks, both conditions (plain and manual),
  1 game each: 20 games.
- Harness exactly as planned for the experiment: clean setup folders, 100 actions, 15 minutes,
  audit, hidden worlds, attempts ledger in `out/dev`.

## Stopping rule

- Calibration stops after the first round in which **all** of these hold:
  1. there are no harness errors that are our fault, and no unexplained audit failures;
  2. without the manual, each player wins between 1 and 4 of its 5 practice games (some room
     both ways);
  3. there are no games lost to a fault in a task, the OS or the checker (such losses are fixed
     first).
- **At most 3 rounds.** If a round fails a condition, only these may change before the next
  round: harness or OS bugs, unclear wording in a request, and the action budget (in steps of
  25). Every change is written down below with the reason.
- If after 3 rounds condition 2 still fails (for example both players win 0 of 5 without the
  manual, or 5 of 5), the experiment still runs. PREREG3 then says so, and the expected floor or
  ceiling becomes a stated limitation. Tasks are not made easier or harder to chase a result.
- Every round is reported in the write-up, with its numbers.

## Log

(filled in after each round)

- **Round 1, first start (5 Nov, about 13:20) - stopped, not counted.** All Haiku games failed
  at once with "OAuth session expired": the harness's saved copy of the Claude login dated from
  4 Nov, and its refresh token no longer worked. This was a harness error (no game was played).
  Fix: `fresh_home` now takes the newer of the saved copy and the owner's live login before
  each game. The round was restarted from scratch. (Another project's Haiku/Luna experiment ran
  on the same PC during calibration.)
- **Round 1, second start - Haiku part restarted, not counted.** Haiku still failed: a failed
  game had copied its broken login back over the saved copy (it was the newest file). Fix: logins
  are now compared by their own expiry time, and a game's login is only kept if it is fresher and
  not broken. Luna's games in this start were unaffected and count; Haiku's part was run again.
- **Round 1 (counted): failed condition 1, so a round 2 follows.** 20 games, no harness errors,
  no OS crashes, every chore that finished passed its hidden worlds.

  | | without manual | with manual |
  |---|---|---|
  | Haiku | 1/5 | 4/5 |
  | Luna | 2/5 | 1/5 |

  9 games failed the audit. 8 of them were **our audit's fault**: Codex wraps every command as
  `powershell.exe -Command "..."`, with the inner quote marks written `\"` (Windows command-line
  escaping). Haiku wrote `\"` inside bash double quotes. The audit refused all backslashes, and
  the prompt's quoting tip makes inner quotes common. Real rule breaks: 1 Haiku game (a bash loop
  using `$n`, and `$'...\t...'` to type a Tab).

  Fix (a harness bug, allowed by the rules above): the audit now reads each command the way its
  own shell does. A Codex-wrapped command is unwrapped with the Windows command-line rules and
  read as PowerShell (a backslash is an ordinary character there). Other commands are read as
  bash (`\" \ \$ \`` inside double quotes are plain escapes). `!` is allowed in plain words.
  New test cases cover both shells, including text the two shells would read differently
  (`tools/audit.py`: 22/22 allowed, 34/34 rejected). Re-checked with the new audit, round 1's
  harmless 8 pass and the real one still fails.

  With the new audit (for information only; round 1 counts as above): Haiku 2/5 without, 4/5
  with the manual; Luna 4/5 without, 5/5 with the manual. One Haiku game ended early when Haiku
  tried a command it isn't allowed to run, was refused, and stopped (a normal loss).
- **Round 2 (counted): all three conditions hold, so calibration stops here.** 20 games, no
  harness errors, no OS crashes; every chore that finished passed its hidden worlds.

  | | without manual | with manual |
  |---|---|---|
  | Haiku | 3/5 | 2/5 |
  | Luna | 3/5 | 5/5 |

  Condition 1: 4 audit failures, all real rule breaks under the written rules (Haiku: shell
  variables such as `P=<game>; python play.py $P ...` in 2 games, and a pipe into `tail` in 1;
  Luna: one command whose inner quotes PowerShell would split, leaving brackets unquoted).
  Condition 2: 3 of 5 without the manual for each player. Condition 3: no game was lost to a task,
  OS or checker fault. Both lost `p3-moving` games (without the manual) sent mails by hand and ran
  out of actions, as the task intends. Two Haiku games ended early when Haiku tried a command it
  isn't allowed to run (its CLI only allows `python play.py <game> ...`, as in experiment 2), was
  refused, and stopped. These count as normal losses.

  Nothing was changed after round 2.

## After calibration (before freezing, no rules of the experiment changed by results)

Sol's final review (5 Nov) found an audit bypass (PowerShell here-strings `@"..."@` and
splatting `@name` passed), that bash `\"` was not respected when splitting chained commands,
and gaps in the retry rules (an OS crash with a CLI error could be re-run; a recorded finished
game could be replaced after an interruption). All were fixed before freezing. Re-checked with
the final audit, the calibration rounds' audit results do not change.
