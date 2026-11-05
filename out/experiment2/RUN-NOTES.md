# Experiment 2 run notes (not a frozen file)

- **4 Nov 23:16** started (`tools/run_experiment2.py`). run.json: haiku `QmTLuDfr3h7JNNjjBNqjHd`,
  luna `cNkrUQvqXofk2nsAWgFB7b`.
- **5 Nov 00:07 paused on purpose** (the owner turned the PC off overnight). All runner, Inspect,
  player and QEMU processes were stopped.
  - Haiku: finished before the pause, **120/120 games, log status success**.
  - Luna: **71/120 games finished and saved** (log status `started`). The 2 Luna games that were
    in progress at 00:07 were cut off before scoring; they were never scored and are re-played.
  - No frozen file was changed. Before resuming, a robot test confirmed that `inspect eval-retry`
    on an interrupted log keeps the finished samples and only runs the rest (same task id), and
    that eval-retry on a finished log re-runs nothing (it copies the same games).
- **To resume:** `python tools/run_experiment2.py --retry` (as PREREG2 says for interrupted or
  errored samples), then `python tools/analyze2.py`.
- **5 Nov ~10:00 resumed** with `python tools/run_experiment2.py --retry` (runner pid 41400). Frozen-file
  check passed first. Note: a separate project's experiment (Pixelforge arena, also Haiku + Luna) was
  running on the same PC at the same time, so the PC and the Luna account were shared during the rest.
- **5 Nov ~10:40 finished.** Both runners exited 0; the 2 cut-off Luna games were re-played. `analyze2.py`:
  240 games, verdict **No clear difference** (tasks better with manual 1, without 2, p=1.0). 4 audit
  failures, no timeouts. The only 2 real losses (move-lesson, manual, one per player) were the same
  mistake: after R (reply) the cursor starts in the message body, the players pressed Tab twice
  expecting To, and typed into To/Subject. The screen text shows no cursor and the manual does not
  say where a reply starts.
