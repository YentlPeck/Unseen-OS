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


LEDGER = TOOLS.parent / "out" / "attempts-ledger.jsonl"
_ledger_lock = threading.Lock()


def record_attempt(entry):
    """Append-only list of every attempt, kept across eval-retry runs (never rewritten)."""
    with _ledger_lock, open(LEDGER, "a", encoding="utf-8") as f:
        f.write(json.dumps({"time": time.strftime("%Y-%m-%d %H:%M:%S"), **entry}) + "\n")


def run_with_retry(task_id, player, manual_text, max_actions, condition="plain", epoch=1):
    """play_game with the PREREG rules:
    - an audit failure always stands (no re-run; it scores FAIL),
    - a harness error gets one re-run; if that fails too, the sample errors out,
    - every attempt is recorded (in the result and in the ledger)."""
    attempts = []
    cell = {"player": player, "task": task_id, "condition": condition, "epoch": epoch}
    for _ in range(2):
        try:
            result = play_game(task_id, player, manual_text, max_actions)
        except Exception as e:  # our code or QEMU failed, not the player
            attempts.append({"game": None, "harness_errors": [f"crash: {e!r}"]})
            record_attempt({**cell, **attempts[-1]})
            continue
        attempts.append({k: result.get(k) for k in ("game", "passed", "audit_ok", "violations",
                                                    "timed_out", "harness_errors", "actions")})
        record_attempt({**cell, **attempts[-1]})
        if not result["audit_ok"] or not result["harness_errors"]:
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
            state.messages.append(ChatMessageUser(content=f"```\n{step['screen']}\n```"))
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
                       + (" | TIMED OUT" if r["timed_out"] else ""))
        return Score(value=CORRECT if passed else INCORRECT, explanation=explanation,
                     metadata={k: r.get(k) for k in ("game", "actions", "seconds", "timed_out", "settle_failures",
                                                     "audit_ok", "violations", "harness_errors", "attempts")})

    return score


@task
def unseen_os(player: str = "haiku", conditions: str | list[str] = "plain", manual: str | None = None,
              level: str | None = None, tasks: str | list[str] | None = None, seed: int | None = None,
              manifest: str | None = None, taskset: str = "v1", max_actions: int = 40):
    # `manifest` is only recorded (in the log's task args): the hash of the frozen manifest.
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
