# Experiment 5: deviations from PREREG5.md

PREREG5.md is in the frozen manifest, so deviations are listed here instead of at its bottom.

## 1. The `conditions` check in the analysis (8 Nov 2026, after all 576 games)

**What happened.** `python tools/analyze5.py` refused a verdict with exactly two problems:

    haiku's Inspect run has conditions = ['plain', 'reference', 'manual'], registered plain,reference,manual
    luna's Inspect run has conditions = ['plain', 'reference', 'manual'], registered plain,reference,manual

Inspect's CLI stored `-T conditions=plain,reference,manual` as a list. The frozen check compares
`str(value)` with the text `plain,reference,manual`. The run used exactly the registered
conditions, in the registered order. The analysis selftest used a fake log that held the text
form, so it did not catch this.

**What was done.** `tools/analyze5_d1.py` (new, not frozen) imports the frozen `analyze5.py`
unchanged. Before validation, it replaces only that one value with its comma-joined text, and only
when the value is exactly the registered list `['plain', 'reference', 'manual']`. Any other list
(another set, another order, other splits) is left as it is and still fails the check. Every other check, the
statistics and the report are the frozen code's. Output: `out/experiment5/analysis.txt`.

**Effect on the results.** None. No game, score or statistic changes. Only a formatting check is
affected.

## Outage (registered, not a deviation)

The run was stopped by hand on 7 Nov at 23:55, with 462 of 576 games finished. It was resumed
on 8 Nov at 11:43 with `python tools/run_experiment5.py --retry` (round 1), as PREREG5.md
("outages") says. The 8 games that were cut off had no records to recover. They were listed as
discarded and re-run. See `out/experiment5/outages.txt` and the attempt list in the analysis.
