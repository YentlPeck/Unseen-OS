# Experiment 4 design (draft 2, 5 Nov 2026)

## Question

Experiment 3 found that the manual helps a lot on chore tasks (wins 18/96 -> 59/96, 15 tasks
to 0). But "manual" bundled several things: the chore reference **in the prompt** instead of only
on the screen, explanations of the OS and its apps (including how to save a page), chore
explanations, five worked examples, "things to know", and extra prompt length. Experiment 4
adds a middle condition:

> Does the full manual help more than giving only the reference text (which the model could
> also read on the screen)?

These are **package** comparisons, not single mechanisms. *reference vs plain* bundles finding
the reference, the cost of reading it 25 lines at a time, and where it sits (prompt vs screen).
*manual vs reference* bundles the OS chapters, the extra chore explanations and examples, and
length. The reference itself already has one small worked example.

## Conditions (3)

| Condition | In the prompt |
|---|---|
| **plain** | nothing (as in experiment 3; the reference is on the computer via `guide do`) |
| **reference** | exactly the text `guide do` shows, introduced by one line: "Here is the chore reference for this computer (the same text `guide do` shows):" |
| **manual** | `docs/manual.md` (as in experiment 3; contains the same reference word for word) |

The reference text is extracted from the ```text block of the Chores chapter of
`docs/manual.md`, with line endings normalised to `\n`. A check (in validation and in the
analysis) proves it is identical to the `GUIDE` constant in `kernel/src/chores.rs`, and the OS
image is built from that source (the image hash is frozen, as before).

## Tasks, world, players, limits

Unchanged from experiment 3: the 16 test tasks of `TASK_SETS["v3"]` (no edits), the same world
(seed 20261112, date 2026-11-12), hidden worlds, scratch drawer, strict scoring, 100 actions,
15 minutes, Haiku (`claude -p`) and Luna (`codex exec`), same isolation and prompt (plus the line
above in the reference condition only).

Games: 2 players x 16 tasks x 3 conditions x **6 epochs** = **576 games**. Measured speed in
experiment 3: 192 games in about 3 h 50 min (Luna is the slower player), so about **11-12 hours**,
run overnight. 2 games at once per player. New shuffle seed. Own ledger
`out/experiment4/attempts-ledger.jsonl`.

This is a **follow-up on a known benchmark**: same tasks, and the experimenter knows the
experiment-3 results. The plain and manual conditions also replicate experiment 3 under revised
instrumentation (audit and keystroke fixes below); that replication is reported, not part of
the verdict.

## Main result (decided in advance)

**Primary (the only verdict): manual vs reference.**

- For each task t: d_t = (manual win rate - reference win rate), pooled over both players and 6
  epochs (12 games per condition per task, so d_t is between -1 and +1).
- Statistic: D = mean of d_t over the 16 tasks (the average win-rate difference, in points).
- Test: exact two-sided **sign-flip permutation test** over the 16 tasks (all 2^16 sign patterns
  of the d_t), alpha = 0.05. Tasks are the unit, because both players play the same tasks.
- Interval: 95% confidence interval for D from inverting the same permutation test (shift
  method); a task-level bootstrap interval is also reported.

| Verdict | Rule |
|---|---|
| **The manual helps more than the reference** | p < 0.05 and D > 0 |
| **The reference does better than the manual** | p < 0.05 and D < 0 |
| **No difference detected** | otherwise; we then also report whether the 95% interval excludes a 10-point benefit of the manual (upper bound < +10 points: "the reference is about as good, within 10 points") |

**Secondary (reported, no verdict): reference vs plain**, the same statistic and test. Also
manual vs plain (the replication of experiment 3), with experiment 3's per-task sign test for
comparison.

Power (to be checked by simulation before freezing, with experiment 3's per-task win rates):
Sol's calculation for a per-task sign test gave about 75-82% power for 60% vs 40% with 6
epochs, and only 10-14% for 60% vs 50%. The permutation test on win rates uses the size of each
difference, so it should be at least as powerful; the simulation decides the stated "smallest
effect this can detect".

## Also reported (descriptive)

Win rates per player x condition (out of 96) and per task kind; the tests per player; actions in
won games; in plain games, whether and how long players read `guide do`; visible-only fix/write
wins; every audit failure, OS crash, time-out and discarded attempt.

## Changes to build before freezing (from Sol's review)

1. **Explicit condition everywhere.** Today `inspect_eval.py` only passes text for "manual" and
   `play_game()` infers the condition from whether text was given, so a third condition would
   silently run as plain. Pass a validated condition (`plain|reference|manual`) through the
   solver, `play_game`, game ids, the prompt, results, scores, the ledger cell and recovery. Write
   `tools/analyze4.py` with explicit pairwise contrasts (analyze3 subtracts every non-manual
   condition, which would be wrong). Check all 576 cells, each condition's prompt text and hash,
   runner arguments, run.json, retries and the freeze list.
2. **Audit.** A backslash is literal in PowerShell syntax, but Windows PowerShell 5.1 re-quotes
   arguments for native programs, so a quoted argument with spaces that ends in a backslash can
   merge with the next argument. New PowerShell-only rule: a backslash is accepted inside a word,
   except at the end of a quoted argument that contains a space, or directly before a quote.
   Test cases check the real `argv` that Python receives through powershell.exe, including
   trailing backslashes, adjacent quotes and experiment 3's three refused commands. Expansions,
   operators and `--%` stay refused. Bash is unchanged.
3. **Lost keystrokes.** `send_combos()` sends a key every 15 ms (20 ms hold) and only settles
   after the whole action. Likely causes: QEMU's bounded input queue, or the guest not polling the
   keyboard while it handles a key synchronously. Measure first, then switch to bounded delivery
   (send a batch, wait until the OS has handled it, then the next), never blindly resending a
   batch whose fate is unknown. Verify the exact typed text, not only counters, with repeated keys,
   modifiers, long batches and 4 games at once.
4. **Audit before exceptions.** A final `settle()` can raise before the audit runs, so an
   attempt with an audit violation could be discarded as a harness error. Keep the game id,
   commands and audit result through exceptions and apply the registered order (audit failure,
   OS crash, harness error, time-out). An unrecoverable delivery failure ends the attempt.
5. **Operations, decided in advance:** conditions mixed in time by the shuffled sample order
   (one eval per player, all 3 conditions); record model and CLI versions at start and end; one
   re-run per harness error, then `eval-retry`; outages (PC off, network) are resumed with
   `--retry` and listed.
6. **Calibration (reference condition only, 5 practice tasks x 2 players = 10 games).** Pass
   rules are technical only: every game logs in and plays; the prompt contains the reference
   exactly once; no harness error that the harness should have prevented; no lost keystrokes. At
   most 2 rounds; only harness bugs may be fixed between rounds (no task or prompt changes). Win
   counts are reported but are not a pass rule.
7. PREREG4.md, final review by Sol, freeze, run.

## Limitations (known now)

- Same tasks, world and models as experiment 3; the experimenter knows those results.
- Package comparisons only (see above). No placebo (length-matched) condition.
- 16 tasks are the independent units; more epochs make each task's win rate more precise but do
  not add task families. The 5 count tasks all read invoice mails.
