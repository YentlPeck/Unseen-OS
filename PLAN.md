# Unseen OS — plan

*A computer no AI has ever seen.* We build a tiny operating system with its own
apps, then test whether small AI models can learn to use it, and whether a
manual helps.

Written 4 Nov 2026. Side project (main project: Pixelforge Guard duty).

## Finish line (fixed before we start — no growing past this)

1. The OS boots in a virtual PC and has 3 apps.
2. 30 tasks, scored automatically by **Inspect**.
3. Haiku and Luna tested **with and without the manual**, rules written down before the test.
4. A leaderboard site, a one-page write-up, and a public repo (audience decided by Yentl before publishing).

## Key choices

| Choice | Decision | Why |
|---|---|---|
| OS language | Rust, runs in **QEMU** (free virtual PC) | Same language as the emulator; real OS, not a fake |
| Screen | Text mode, 80×25 characters | Small models can read it exactly as text; no image reading needed |
| Unfamiliar on purpose | Own command names and rules, used consistently (not Linux/Windows look-alikes) | Models can't lean on memory, so we measure real figuring-out |
| Agent controls | `type(text)`, `press(key)`, `read_screen()` | Like a person at a keyboard |
| Checking results | A hidden "inspector" channel prints the OS's files after each task | The score comes from the real end state, not from what the model claims |
| Models | **Haiku** (via `claude -p`) and **Luna** (via `codex exec`), same setup as Pixelforge; locked to one game tool | Cheap small models Yentl already uses; no API key needed (changed 4 Nov, Ollama dropped) |
| Main test tool | **Inspect** (UK AI Security Institute) | Named in job ads; has a replay viewer |
| Side test tool | **Promptfoo** | Compare manual versions (short vs long) on quick tasks |

## Apps inside the OS

- **Shell**: the command line, with its own commands.
- **Notes**: a small text editor.
- **Files**: a file manager to browse, move and rename.
- **Later (Yentl wants more apps):** Mail (inbox with fake messages, "reply to Mira"), and others, added after the first 3 work.

## Phases

| # | Phase | Proof it's done |
|---|---|---|
| 1 | **It boots**: kernel, text screen, keyboard, shell with 5 commands | Screenshot of the shell running in QEMU |
| 2 | **Files + apps**: file system, Notes, Files (+ Mail) | Each app usable by hand |
| 3 | **Agent bridge**: Python sends keys and reads the screen; a scripted robot solves 1 task | Robot passes a task end to end |
| 4 | **Tasks + Inspect**: 30 tasks in 3 levels (easy / medium / hard), auto-scored | Inspect runs all 30 and opens the viewer |
| 5 | **The experiment**: rules written first, then models × (no manual / manual) | Results table + simple graph |
| 6 | **Publish**: write-up, leaderboard site, repo | Links work |

Codex reviews after phase 2 and after phase 4.

## Pilot log (before experiment rules are written)

- 4 Nov: `copy-list` played by Haiku and Luna (both PASS); `copy-list` + `change-word` by Haiku through Inspect (both PASS). These were plumbing checks, no manual. Flag them in the write-up.
- 4 Nov (after Codex phase-4 fixes): `copy-list` by Luna (PASS, 15 actions, audit clean) and `change-word` by Haiku (PASS, 19 actions, audit clean after allowing chains of game calls).

## The experiment question

> Can small AI models operate a computer they've never seen — and how much does reading the manual help?
