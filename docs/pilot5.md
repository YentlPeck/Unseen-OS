# Pilot M7: Haiku and Luna build, extend and fix Unseen Script apps (7 Nov 2026)

Two rounds: round 1 with the harness of experiments 1-4; round 2 with the batching fix
(the `line` action and the one-command rule, below). Round 2: **6 of 6 passed**.

A pilot checks that app tasks are fair and that the harness works. It is **not** an
experiment: one game per task and player, manual condition only, and these tasks are never
reused. Tasks: `tools/tasks_apps.py`; runner and grader: `tools/pilot5.py`; game records:
`out/pilot5/runs/`.

## Before the games

- `python tools/pilot5.py validate`: the start desks fail the fail-to-pass tests and pass the
  pass-to-pass tests; the known solutions, typed by a robot through the real harness, pass
  every hidden test; all 13 wrong solutions are caught (hard-coding, swallowing with `try`,
  not saving, a minus that resets, Esc closing, ...); pages left outside the allowed places
  fail; an app can't fake PASS lines.
- Sol checked the tasks for fairness: all three fair; six grader findings (verdict only from
  the OS's own title bar, exact allowed paths, two weak tally checks, the scope told to the
  player, grading errors as harness errors, page copying) were fixed before any game.

## Results (round 1)

| Task | Kind | Haiku | Luna |
|---|---|---|---|
| app-tally | build an app | PASS (4/4 hidden, 82 actions, 3 min) | PASS (4/4, 67 actions, 5 min) |
| app-todo-move | add a feature | FAIL: audit (3/5, 9 actions) | PASS (5/5, 118 actions, 10 min) |
| app-guests-fix | fix a bug | FAIL: audit (4/4 hidden, 13 actions) | PASS (4/4, 53 actions, 5 min) |

No harness errors, no OS crashes. Every app a player finished passed its hidden tests.

## What we learned

- **The tasks are fair and doable from the manual.** Both players built a working app from
  scratch; Luna also added the feature and fixed the bug, each time writing its own test page
  with `launch`/`press`, running `test`, and removing the page afterwards.
- **Haiku's two failures are the audit, not the language.** In both games Haiku tried to put
  several moves into one shell command (`P=...; python play.py $P type ...; ...`). Claude's
  permission rule refused it, so nothing reached the OS, but the audit counts any command
  that isn't one game-tool call. In app-guests-fix its fix was already correct (4/4 hidden
  tests). In app-todo-move it stopped after reading the app.
- **Why it happens:** an app is typed line by line, and every line costs two actions (type,
  then Enter). That invites batching. Before an experiment, choose one: say plainly in the
  prompt that each command must be exactly one `python play.py ...` call (no shell variables,
  no `;` or `&&`), or give the game tool a way to type a line and press Enter in one action.
  Either changes the harness for app tasks only; experiments 1-4 stay as they were.
- **Budgets:** 200 actions and 45 minutes were plenty (most actions used: 118, longest game:
  10 minutes).

## Round 2: the batching fix (same day)

For app tasks only (experiments 1-4 keep their files unchanged):

- the game tool has one more action, `line "text"`: type the text, then press Enter, as one
  action (`tools/broker_apps.py`, `tools/play_apps.py`, which the player gets as `play.py`);
- the prompt lists it and says: every action is a command of its own, exactly one
  `python play.py GAME ...` with the game name written out (no shell variables, no `;` or
  `&&`, no loops) (`PROMPT` in `tools/pilot5.py`).

The robot typed every known solution with `line` through the real harness and passed. Then
the same three tasks, one game each (records: `out/pilot5/runs-2/`):

| Task | Haiku | Luna |
|---|---|---|
| app-tally | PASS (4/4, 43 actions, 3 min) | PASS (4/4, 48 actions, 7 min) |
| app-todo-move | PASS (5/5, 38 actions, 3 min) | PASS (5/5, 33 actions, 5 min) |
| app-guests-fix | PASS (4/4, 41 actions, 3 min) | PASS (4/4, 12 actions, 3 min) |

No audit failures, no harness errors, no OS crashes. Haiku no longer batched commands (in its
to-do game: 32 `line`, 5 `press`, 1 `look`), and games took about half the actions of round 1.
The models played the same tasks as in round 1 but kept nothing between games, so this checks
the fix, not a learning effect. App experiments use the round-2 harness.
