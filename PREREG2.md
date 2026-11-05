# Unseen OS experiment 2: rules written before running

Written 4 Nov 2026, **before any experiment-2 game has been played.** Nothing here changes
after the first experiment-2 game starts; any change is added at the bottom with a date and a
reason, and reported as a deviation.

## Question

> On a computer they have never seen, with tasks that need several apps together, does reading
> the manual help small AI models succeed?

## Setup

| | |
|---|---|
| OS | Unseen OS 0.2: shell, Notes, Files, **Mail, Contacts, Calendar, Settings, Calculator** |
| Players | **Haiku** (`claude-haiku-5-5` via `claude -p`) and **Luna** (`gpt-6-luna` via `codex exec`) |
| Tool isolation | Every game runs the player's CLI from a **fresh setup folder** holding only the login (`CLAUDE_CONFIG_DIR` / `CODEX_HOME`; `fresh_home` in `tools/run_task.py`), deleted after the game: none of the owner's plugins, hooks, memory, personal instructions or MCP servers, nothing left from earlier games. Haiku: only `Bash(python play.py <game>:*)`, no MCP, default settings (as in experiment 1), no session saved. Luna: computer use, apps and memories switched off; the two settings Luna used in experiment 1 are kept (reasoning effort high, the Windows sandbox mode that lets it run commands) |
| Conditions | **plain**: no manual. **manual**: `docs/manual.md` (version with the apps) in the prompt |
| Tasks | the 20 **test** tasks in `tools/tasks_v2.py` (`TASK_SETS["v2"]`). Most are meant to need 2-3 apps, but some can be done with fewer (e.g. `party` needs only Mail, `start-mail` only Settings; sums can be done without the Calculator; the shell's `read`/`seek`/`copy` can stand in for browsing an app). Only the end state is scored |
| Games | each player plays each task **3 times** per condition: 2 x 20 x 2 x 3 = **240 games** |
| Limits | **60 actions** and 15 minutes per game |
| Tool | only `play.py` (look / type / press) through the referee; prompt in `tools/run_task.py` |
| Scoring | strict end-state check against `tools/expected.json`: exact for pages the task leaves alone; created/edited pages may differ only in trailing spaces and trailing blank lines; calendar events compared by content (duplicates count). Free-text results are checked by the `flex` tests in `tools/tasks_v2.py`: where a fact is needed, common ways of writing it are accepted (amounts like 163 / 163.00 / 163,00 but not -163; dates like 2026-11-14 / 14 November / Nov 14; times like 18:00 / 6 pm); where wording matters the request says it (e.g. "start your reply with the word Yes"); forwarded/saved mail text must contain Omar's whole message. Each task lists right and wrong example answers that `tools/validate_tasks.py` checks |
| Harness | Inspect, one eval per player, both conditions in the same eval, `--epochs 3`, samples shuffled with seed **20261105**, at most 2 games at once per player |

## What counts

- **Win:** the strict check passes, the audit is clean, and the game did not time out.
- **Loss:** everything else, including running out of actions or time.
- **Audit (changed after experiment 1, decided before experiment 2):** a game fails the audit
  if the player did anything except call the game tool. The rules and test cases are in
  `tools/audit.py`. Allowed shell glue around game-tool calls: `cd`/`Set-Location`, chaining with
  `;`/`&&`/`||`, silencing output (`>/dev/null`, `2>&1`, `| Out-Null`, `> $null`), a bash
  for-loop (`for i in 1 2 3`, `$(seq N)` or `{1..N}`) whose body only calls the game tool, `echo`
  of literal text, and a variable that holds exactly `python play.py <game>`. A mistyped call of
  the game tool is allowed (it only prints its usage). Arguments must be literal (no `$` or
  backtick in double quotes; nothing but plain characters outside quotes, so no backslash escapes and no PowerShell `--%`). Not allowed: `if`,
  any other program or command, pipes into other programs, writing files, command substitution,
  any other tool use (file edits, MCP, web, computer use), or changing `play.py`/`game.json`.
  Audit failures count as losses, are never re-run, and are all listed.
- **Harness errors:** our side failed (the OS, the referee or the bridge), or the player's CLI
  itself (in its own messages, not the model's text) reported an error, a blocked command or a
  missing login, or the CLI did nothing at all (no command, no message, no game action). A model
  that refuses or only makes mistyped calls is NOT a harness error: that game is a loss.
  Order of rules: audit failure first (always a loss, never re-run), then harness error (re-run),
  then time-out (a loss). A harness error gets one re-run of that game; if that also fails, the
  sample errors and is re-played with `tools/run_experiment2.py --retry` (Inspect `eval-retry`)
  until none are left, including games stopped by usage limits. Every attempt is written to an
  append-only ledger (`out/attempts-ledger.jsonl`) and all discarded attempts are reported. No
  completed game is re-run, dropped or replaced.
- **Which games count:** `tools/run_experiment2.py` can only be started once; it records the
  run (Inspect task id per player) in `out/experiment2/run.json`, and later only `--retry` is
  allowed (eval-retry keeps the task id). `tools/analyze2.py` uses the latest log of the recorded
  run per player and refuses a verdict unless: no frozen file changed; every log belongs to the
  recorded run; the logs show the registered settings (task set v2, both conditions, seed
  20261105, 60 actions, the default manual, 3 epochs, 2 games at once, the frozen manifest hash);
  there are no errored samples; there are exactly 240 unique games (2 players x 20 tasks x 2
  conditions x 3 epochs) and no underlying game is counted twice; and every scored game is in the
  attempts ledger.

## Main result (decided in advance)

The two players play the same tasks, so the main test counts **per task** (20 tasks):

- For each task: **score = (manual wins - plain wins)**, summed over both players and all 3
  games (from -6 to +6).
- Test: exact two-sided **sign test** over the tasks with a non-zero score.

| Verdict | Rule |
|---|---|
| **The manual helps** | p < 0.05 and more tasks with a positive score |
| **The manual hurts** | p < 0.05 and more tasks with a negative score |
| **No clear difference** | otherwise |

This tests these two models on these 20 tasks; it does not claim to generalise.

## Also reported (descriptive, no verdict)

- Win rate per player and condition (out of 60), and per task.
- The same per-task sign test for each player separately.
- How consistent the 3 repeats are: for each player, task and condition, how many of the 3
  games were won.
- Actions used in won games (median per player and condition).
- Every audit failure, timeout and harness re-run.
- Anything else is labelled **exploratory**.

## Tuning done before this file (practice tasks only)

The 5 practice tasks in `tools/tasks_v2.py` (`TASK_SETS["practice2"]`) were played by both
players, with and without the manual, in 3 rounds of 20 games, plus one Luna-only round of 10 games
and one single Luna test game. They are never part of the
experiment. They were used to choose the 60-action limit and showed three problems that were
fixed before this file: the audit flagged harmless shell habits (rule widened as above), Luna
had Codex's computer-use tool and other extra tools on, and both players loaded the owner's plugins and memory (both now run from clean setup folders; a third practice round checked this), Mail silently sent
to an address with the wrong capitals (it now warns once), and in the clean setup Luna's commands
were first blocked and counted as plain failures (fixed: the sandbox setting is kept, and a blocked
player or a game with zero actions is now a harness error). Final practice results before freezing:
Haiku 10/10 (round 3), Luna 10/10 (Luna-only round), all audits clean, no harness errors. The 20 test tasks were never played
by an AI before the experiment; only the robot test player (reference solutions) ran them.

## Limitations (known before running)

- 3 games per cell; no placebo (length-matched) control: "manual" adds information and
  prompt length together.
- Fixed tasks written by the experimenter; two models; subscription CLIs whose model versions
  could change during the run (games are mixed in time).

## Frozen files

`python tools/freeze.py --exp 2` writes `out/experiment2/manifest.json` (refuses to overwrite)
with SHA-256 hashes of the OS image, this file, the manual, the tasks and expected end states,
and all harness, runner, analysis and freeze code, plus a dated copy in `docs/`.
