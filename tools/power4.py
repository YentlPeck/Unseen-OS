"""Power of experiment 4's primary test (decided before freezing, see PREREG4.md).

  python tools/power4.py

Model: each task's manual win rate is its experiment-3 manual win rate (both players pooled);
the reference win rate is that minus an effect (in points, the same for every task, kept within
0..1). Each condition gets 12 games per task (2 players x 6 epochs), drawn at random. The primary
test (exact sign-flip permutation test on the mean per-task difference, alpha 0.05) is run on
each simulated experiment. Reports the share of simulated experiments where it finds the effect.
"""

import json
import sys
from pathlib import Path

import numpy as np

TOOLS = Path(__file__).resolve().parent
sys.path.insert(0, str(TOOLS))
from analyze4 import SIGNS, TASKS  # noqa: E402

GAMES = 12
SIMS = 2000


def main():
    r = json.loads((TOOLS.parent / "out" / "experiment3" / "results.json").read_text())
    manual = np.array([int(r["win_rates_per_task"][f"{t} / manual"].split("/")[0]) / 6 for t in TASKS])
    S = np.array(SIGNS, dtype=np.int8).T.astype(np.float32)  # 16 x 65536
    rng = np.random.default_rng(20261114)
    print(f"experiment-3 manual win rates per task: {np.round(manual, 2).tolist()} (mean {manual.mean():.2f})")
    for effect in (0.0, 0.10, 0.15, 0.20, 0.25, 0.30):
        ref = np.clip(manual - effect, 0, 1)
        a = rng.binomial(GAMES, manual, size=(SIMS, 16)) / GAMES
        b = rng.binomial(GAMES, ref, size=(SIMS, 16)) / GAMES
        d = (a - b).astype(np.float32)
        obs = np.abs(d.sum(axis=1)) - 1e-6
        p = (np.abs(d @ S) >= obs[:, None]).mean(axis=1)
        found = ((p < 0.05) & (d.mean(axis=1) > 0)).mean()  # finds the manual's benefit
        either = (p < 0.05).mean()                           # any verdict other than "no difference"
        print(f"effect {effect * 100:4.0f} points (true mean difference {np.mean(manual - ref) * 100:4.1f}): "
              f"finds the benefit {found * 100:5.1f}%, any difference {either * 100:5.1f}%")


if __name__ == "__main__":
    main()
