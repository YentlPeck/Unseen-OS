# Experiment 4 run notes (not a frozen file)

- **5 Nov 20:27** started (`tools/run_experiment4.py`, runner pid 12884). Frozen at commit
  0ea6569 (manifest sha256 949ee880...). run.json: haiku `f2sTNPSNVArxrySQr3hKB3`,
  luna `bhDCGz5QATNFPyQCAkjWV7`. CLI versions at start: claude 2.1.318, codex-cli 0.166.0.
  576 games, expected 11-12 hours.
- **To resume after an interruption:** `python tools/run_experiment4.py --retry`, then
  `python tools/analyze4.py`.

## Paused 5 Nov 2026, ~23:40 (user's request: PC off overnight)

Runner (pid 12884) and its whole process tree stopped with taskkill /T /F. 4 games were in flight;
they are booked and unfinished, so `--retry` salvages or replays them by the written rules.
Resume: `python tools/run_experiment4.py --retry`.

## Resumed 6 Nov 2026, 10:26

`python tools/run_experiment4.py --retry` (runner pid 19352). Inspect recovered both interrupted
logs (`*-recovered.eval`) and started eval-retry logs with the same task ids (created 09:26 UTC);
analyze4 uses the newest log per player. Console output: runner-resume.txt.

## Deviation: Codex CLI updated during the run (6 Nov 2026)

Codex installed 0.167.0 at 10:27:03 (`~/.codex/packages/standalone/releases`), one minute after
the resume. Luna games before the pause (72 terminal) ran on 0.166.0; games started from 10:27 on
run on 0.167.0 (the 2 started at 10:26:43 likely still on 0.166.0). Same model and arguments
(run_task.py unchanged). Claude Code stayed 2.1.318. To report in the write-up, with a
before/after-pause check for Luna (extra, not part of the pre-registered analysis).

## Finished 6 Nov 2026, 20:21

Both inspect runs exited 0; `python tools/analyze4.py` passed all checks (576 games, ledger one
terminal attempt per cell). Primary: manual vs reference D = +19.8 points, p = 0.0026, 95% CI
[9.2, 30.6] (bootstrap [10.4, 29.7]), 13 tasks better / 2 worse: "the manual helps more than the
reference". CLI at end: claude 2.1.319, codex 0.167.0.

Extra checks (not pre-registered):
- Luna before vs after the pause (Codex update): plain 7/26 vs 22/70, reference 15/23 vs 46/73,
  manual 18/23 vs 55/73. No visible change.
- Task done ignoring the audit: haiku 23/64/74, luna 32/66/73 (plain/reference/manual). Audit
  failures: haiku 28/30/13, luna 5/8/0. Much of the manual's lead over the reference comes from
  fewer audit failures (rule breaks in how commands are sent).
- Claude Code 2.1.318 -> 2.1.319 at 19:41 (6 Nov), after Haiku's last game (~12:00): no Haiku
  game ran on 2.1.319.
