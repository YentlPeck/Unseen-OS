# Experiment 3 run notes (not a frozen file)

- **5 Nov 14:27** started (`tools/run_experiment3.py`, runner pid 40300). Frozen at commit
  d43b185 (manifest sha256 ca246f78...). run.json: haiku `Q4aoDqgkqJBmpMykd7pbZx`,
  luna `ZW46UjjeVgzphZsYPKzzZ6`. First games of both players checked: logged in and playing.
  Another project's experiment (Pixelforge, also Haiku/Luna) may be running on the same PC.
- **To resume after an interruption:** `python tools/run_experiment3.py --retry`, then
  `python tools/analyze3.py`.
- **5 Nov ~18:15** main run finished (both players, Inspect exit 0). 1 Haiku sample (overdue
  plain #2) had a harness error twice (OS lost keys) and errored, as the rules allow.
- **5 Nov ~18:20** `python tools/run_experiment3.py --retry` re-played it (exit 0).
  `python tools/analyze3.py`: all checks passed, verdict "The manual helps" (15-0, p = 0.0001).
  5 discarded attempts in total (all SettleError), 0 OS crashes, 2 time-outs, 28 audit failures.
