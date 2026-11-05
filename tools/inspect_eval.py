"""Inspect harness for Unseen OS.

  inspect eval tools/inspect_eval.py -T player=haiku --model mockllm/model --max-samples 2
  inspect view                       (opens the log viewer in the browser)

Options (-T):
  player      haiku | luna | script   (script = reference solutions, to test the harness)
  conditions  plain,manual            (default: plain) which conditions to include
  manual      path to the manual      (default: docs/manual.md)
  level       easy | medium | hard    (default: all)
  tasks       comma-separated task ids (default: all 30)
  seed        shuffle seed for the sample order (default: none = task order)

Each sample is one (task, condition) game. The player is an outside command-line
agent (claude / codex), not an Inspect model, so `--model mockllm/model` is only
a placeholder. The player's steps are copied into the sample's messages so the
viewer can replay them.

Harness errors (OS did not start, settle failure, crash) get exactly one re-run
of that game, as PREREG.md says; a second failure counts as FAIL.
"""

import json
import os
import sys
import threading
import time
from pathlib import Path

import anyio
from inspect_ai import Task, task
from inspect_ai.dataset import MemoryDataset, Sample
from inspect_ai.model import ChatMessageAssistant, ChatMessageUser, ModelOutput
from inspect_ai.scorer import CORRECT, INCORRECT, Score, Target, accuracy, grouped, scorer, stderr
from inspect_ai.solver import Generate, TaskState, solver

TOOLS = Path(__file__).resolve().parent
sys.path.insert(0, str(TOOLS))
from run_task import MODELS, play_game  # noqa: E402
from tasks import TASK_SETS, TASKS  # noqa: E402

DEFAULT_MANUAL = TOOLS.parent / "docs" / "manual.md"


class HarnessError(RuntimeError):
    """Our side failed twice: Inspect marks the sample as errored (re-run later with eval-retry)."""


# Experiment runners set UNSEEN_LEDGER to their experiment's ledger. Everything else
# (robot tests, calibration) goes to out/dev, so it can never mix with an experiment's
# record. (Experiments 1-2 used out/attempts-ledger.jsonl; see the experiment-2 tag.)
LEDGER = Path(os.environ.get("UNSEEN_LEDGER") or TOOLS.parent / "out" / "dev" / "attempts-ledger.jsonl")
_ledger_lock = threading.Lock()


def record_attempt(entry):
    """Append-only list of every attempt, kept across eval-retry runs (never rewritten)."""
    LEDGER.parent.mkdir(parents=True, exist_ok=True)
    with _ledger_lock, open(LEDGER, "a", encoding="utf-8") as f:
        f.write(json.dumps({"time": time.strftime("%Y-%m-%d %H:%M:%S"), **entry}) + "\n")


SCORED = ("game", "passed", "rule", "audit_ok", "violations", "timed_out", "harness_errors", "actions",
          "seconds", "settle_failures", "visible_passed", "hidden", "os_crash", "summary")


def terminal(result):
    """The registered order: an audit failure stands, then an OS crash (a loss), then a
    harness error (re-run); anything else is a finished game. Terminal results are final."""
    return (not result.get("audit_ok")) or bool(result.get("os_crash")) or not result.get("harness_errors")


def earlier_terminal(cell):
    """A finished game already recorded for this cell (only in an experiment's own ledger):
    if an interruption lost the Inspect sample after the game was recorded, eval-retry must
    reuse that game, never play a replacement."""
    if not os.environ.get("UNSEEN_LEDGER") or not LEDGER.exists():
        return None
    for line in LEDGER.read_text(encoding="utf-8").splitlines():
        a = json.loads(line)
        if a.get("terminal") and all(a.get(k) == v for k, v in cell.items()):
            return a
    return None


def run_with_retry(task_id, player, manual_text, max_actions, condition="plain", epoch=1):
    """play_game with the PREREG rules:
    - an audit failure always stands (no re-run; it scores FAIL), and so does an OS crash,
    - a harness error gets one re-run; if that fails too, the sample errors out,
    - every attempt is recorded (in the result and in the ledger); a recorded finished game
      is never replayed (eval-retry reuses it)."""
    attempts = []
    cell = {"player": player, "task": task_id, "condition": condition, "epoch": epoch}
    done = earlier_terminal(cell)
    if done:
        result = {k: done.get(k) for k in SCORED}
        result.update(transcript=[], recovered_from_ledger=True, attempts=[{k: done.get(k) for k in SCORED}])
        return result
    for _ in range(2):
        try:
            result = play_game(task_id, player, manual_text, max_actions)
        except Exception as e:  # our code or QEMU failed, not the player
            attempts.append({"game": None, "harness_errors": [f"crash: {e!r}"]})
            record_attempt({**cell, **attempts[-1], "terminal": False})
            continue
        attempts.append({k: result.get(k) for k in SCORED})
        record_attempt({**cell, **attempts[-1], "terminal": terminal(result)})
        if terminal(result):
            result["attempts"] = attempts
            return result
    raise HarnessError(f"harness error twice: {attempts}")


@solver
def unseen_player(player: str, manual_text: str | None, max_actions: int):
    async def solve(state: TaskState, generate: Generate) -> TaskState:
        task_id = state.metadata["task"]
        text = manual_text if state.metadata["condition"] == "manual" else None
        result = await anyio.to_thread.run_sync(run_with_retry, task_id, player, text, max_actions,
                                                state.metadata["condition"], state.epoch)
        for step in result.pop("transcript"):
            state.messages.append(ChatMessageAssistant(content=f"{step['action']} {' '.join(step['args'])}".strip()))
            cursor = f"\n(cursor: {step['cursor']})" if step.get("cursor") else ""
            state.messages.append(ChatMessageUser(content=f"```\n{step['screen']}\n```{cursor}"))
        state.output = ModelOutput.from_content(model=MODELS.get(player, player), content=result["summary"] or "(no summary)")
        state.metadata["result"] = result
        return state

    return solve


@scorer(metrics=[accuracy(), stderr(), grouped(accuracy(), "condition"), grouped(accuracy(), "level")])
def end_state():
    async def score(state: TaskState, target: Target) -> Score:
        r = state.metadata["result"]
        # PREREG.md: an audit failure or a time-out always counts as FAIL.
        passed = r["passed"] and r["audit_ok"] and not r["timed_out"]
        explanation = (f"Rule: {r['rule']}" + ("" if r["audit_ok"] else " | AUDIT FAILED")
                       + (" | OS CRASH" if r.get("os_crash") else "")
                       + (" | TIMED OUT" if r["timed_out"] else ""))
        return Score(value=CORRECT if passed else INCORRECT, explanation=explanation,
                     metadata={k: r.get(k) for k in ("game", "actions", "seconds", "timed_out", "settle_failures",
                                                     "audit_ok", "violations", "harness_errors", "attempts",
                                                     "visible_passed", "hidden", "os_crash",
                                                     "recovered_from_ledger")})

    return score


@task
def unseen_os(player: str = "haiku", conditions: str | list[str] = "plain", manual: str | None = None,
              level: str | None = None, tasks: str | list[str] | None = None, seed: int | None = None,
              manifest: str | None = None, taskset: str = "v1", max_actions: int | None = None):
    # `manifest` is only recorded (in the log's task args): the hash of the frozen manifest.
    if max_actions is None:  # measured budgets: 40 for the v1/v2 tasks, 100 for chore tasks (v3)
        max_actions = 100 if taskset in ("v3", "practice3") else 40
    conds = conditions.split(",") if isinstance(conditions, str) else list(conditions)
    wanted = set(tasks.split(",") if isinstance(tasks, str) else tasks) if tasks else None
    manual_path = Path(manual) if manual else DEFAULT_MANUAL
    manual_text = manual_path.read_text(encoding="utf-8") if "manual" in conds else None
    samples = [
        Sample(id=f"{tid}:{cond}", input=t["request"], target=t["rule"],
               metadata={"task": tid, "condition": cond, "level": t["level"]})
        for tid, t in TASKS.items()
        for cond in conds
        if tid in TASK_SETS[taskset]
        and (level is None or t["level"] == level) and (wanted is None or tid in wanted)
    ]
    dataset = MemoryDataset(samples)
    if seed is not None:
        dataset.shuffle(seed=seed)
    return Task(dataset=dataset, solver=unseen_player(player, manual_text, max_actions), scorer=end_state(),
                metadata={"player": player, "conditions": conds, "seed": seed, "taskset": taskset, "max_actions": max_actions,
                          "manual": str(manual_path) if manual_text else None})
