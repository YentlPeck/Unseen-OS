"""What experiment 5's tests can detect (PREREG5.md), by simulation. Not part of the verdicts.

  python tools/power5.py

24 tasks; per task and condition 8 games (2 players x 4 epochs), pooled. Pass rates: each task's
plain rate is drawn between 0.45 and 0.95 (calibration: most games passed, the experiment's hard
tasks should pass less), the manual adds a fixed number of points, capped at 1 (so on tasks near 1
the added points are smaller than stated). Actions: per
player and task, won games' actions are log-normal with the spread seen in calibration (sd 0.45
of ln actions), and the manual changes the median by a fixed factor. The tests are the
pre-registered sign-flip tests; here their p-value is estimated with 20,000 random sign patterns
(the real analysis enumerates all of them), 1,000 simulated experiments per row.
"""

import math
import random
import statistics as st

import numpy as np

TASKS, GAMES_PER_PLAYER, PLAYERS, SIMS, FLIPS, SEED = 24, 4, 2, 1000, 20_000, 20261108


def p_value(d, rng):
    d = np.asarray(d)
    signs = rng.choice([-1.0, 1.0], size=(FLIPS, len(d)))
    return (np.count_nonzero(np.abs(signs @ d) >= abs(d.sum()) - 1e-12) + 1) / (FLIPS + 1)


def pass_power(effect, rng, nrng):
    """(share finding a benefit, share finding a harm): both are wrong verdicts when effect = 0."""
    hits = harms = 0
    for _ in range(SIMS):
        d = []
        for _ in range(TASKS):
            base = rng.uniform(0.45, 0.95)
            n = GAMES_PER_PLAYER * PLAYERS
            plain = sum(rng.random() < base for _ in range(n)) / n
            manual = sum(rng.random() < min(1.0, base + effect) for _ in range(n)) / n
            d.append(manual - plain)
        if p_value(d, nrng) < 0.05:
            hits += st.mean(d) > 0
            harms += st.mean(d) < 0
    return hits / SIMS, harms / SIMS


def action_power(factor, rng, nrng, win=0.8):
    """(share finding fewer actions, share finding more actions). Assumes every task is won in 80%
    of games in both conditions and the same factor on every task."""
    hits = harms = 0
    for _ in range(SIMS):
        r = []
        for _ in range(TASKS):
            logs = []
            for _ in range(PLAYERS):
                mu = math.log(rng.uniform(25, 90))
                wins = lambda m: [math.exp(rng.gauss(m, 0.45)) for _ in range(GAMES_PER_PLAYER) if rng.random() < win]
                a, b = wins(mu + math.log(factor)), wins(mu)
                if a and b:
                    logs.append(math.log(st.median(a) / st.median(b)))
            if logs:
                r.append(st.mean(logs))
        if r and p_value(r, nrng) < 0.05:
            hits += st.mean(r) < 0
            harms += st.mean(r) > 0
    return hits / SIMS, harms / SIMS


def main():
    rng, nrng = random.Random(SEED), np.random.default_rng(SEED)
    print("pass rate, manual vs plain (points added -> verdict 'helps' / verdict 'does worse'):")
    for e in (0.0, 0.05, 0.10, 0.15, 0.20):
        a, b = pass_power(e, rng, nrng)
        print(f"  {e * 100:4.0f} points: helps {a:.0%}, does worse {b:.0%}")
    print("actions in won games (change -> verdict 'saves actions' / verdict 'costs actions'):")
    for f in (1.0, 0.9, 0.8, 0.7, 0.6):
        a, b = action_power(f, rng, nrng)
        print(f"  {(f - 1) * 100:+4.0f}%: saves {a:.0%}, costs {b:.0%}")


if __name__ == "__main__":
    main()
