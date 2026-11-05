"""Inspect harness for Unseen OS.

  inspect eval tools/inspect_eval.py -T player=haiku --model mockllm/model --max-samples 2
  inspect view                       (opens the log viewer in the browser)

Options (-T):
  player      haiku | luna | script   (script = reference solutions, to test the harness)
  conditions  plain,reference,manual  (default: plain) which conditions to include
              (reference = only the chore reference that `guide do` shows, from the manual)
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
from reference import reference_text, sha256  # noqa: E402
from audit import audit  # noqa: E402
from run_task import (CONDITIONS, MODELS, check_condition, load_transcript, new_game_id,  # noqa: E402
                      play_game, player_events, saved_result)
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


SCORED = ("game", "condition", "context_sha256", "passed", "rule", "audit_ok", "violations", "timed_out", "harness_errors", "actions",
          "seconds", "settle_failures", "visible_passed", "hidden", "os_crash", "summary")


def terminal(result):
    """The registered order: an audit failure stands, then an OS crash (a loss), then a
    harness error (re-run); anything else is a finished game. Terminal results are final."""
    return (not result.get("audit_ok")) or bool(result.get("os_crash")) or not result.get("harness_errors")


def ledger_rows(cell):
    """This cell's rows in an experiment's own ledger (none outside an experiment)."""
    if not os.environ.get("UNSEEN_LEDGER") or not LEDGER.exists():
        return []
    rows = [json.loads(line) for line in LEDGER.read_text(encoding="utf-8").splitlines() if line.strip()]
    return [a for a in rows if all(a.get(k) == v for k, v in cell.items())]


def earlier_terminal(cell):
    """A finished game already recorded for this cell (only in an experiment's own ledger):
    if an interruption lost the Inspect sample after the game was recorded, eval-retry must
    reuse that game, never play a replacement."""
    return next((a for a in ledger_rows(cell) if a.get("terminal")), None)


def unrecorded_results(cell, text):
    """Games booked for this cell (a "started" row) whose result was saved but never entered
    in the ledger, because an interruption came in between: they are used, not replayed.
    (A booked game without a saved result never finished, and is listed as discarded.)"""
    rows = ledger_rows(cell)
    entered = {a.get("game") for a in rows if "passed" in a}  # rows that hold a game's result
    out = []
    for a in rows:
        if a.get("started") and a.get("game") not in entered:
            # the same rescue as after a crash: saved result, its temporary copy, or the player's log
            r = salvage(a["game"], cell["player"], cell["condition"], text, "interrupted (found on restart)")
            if r is not None and r.get("condition") == cell["condition"]:
                out.append(r)
    return out


def salvage(gid, player, condition, text, error):
    """After play_game itself failed: the game's result if it was written (result.json, or its
    complete temporary copy), else an audit of the player's own log. A known audit failure
    always stands (a loss, never replaced). None: no result and no violation found."""
    run = TOOLS.parent / "out" / "runs" / gid
    result = saved_result(gid)
    if result is None:
        try:
            result = json.loads((run / "result.json.tmp").read_text())
            result = result if isinstance(result, dict) and result.get("game") == gid and "passed" in result else None
        except Exception:
            result = None
        if result is not None:
            result["transcript"] = load_transcript(gid)
    if result is not None:
        return result
    try:
        commands, others, summary, _ = player_events(player, (run / "player.log").read_text(encoding="utf-8",
                                                                                            errors="replace"))
    except Exception:
        return None
    violations = audit(commands, gid) + others
    if not violations:
        return None
    return {"game": gid, "condition": condition, "context_sha256": sha256(text) if text else None,
            "passed": False, "visible_passed": False, "hidden": None, "os_crash": None, "timed_out": False,
            "rule": "audit failure (read from the player's log after our side failed)",
            "audit_ok": False, "violations": violations, "harness_errors": [f"crash: {error!r}"],
            "actions": None, "seconds": None, "settle_failures": 0, "summary": summary,
            "transcript": load_transcript(gid)}


def run_with_retry(task_id, player, manual_text, max_actions, condition, epoch=1):
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
        result.update(transcript=load_transcript(done.get("game")) if done.get("game") else [],
                      recovered_from_ledger=True, attempts=[{k: done.get(k) for k in SCORED}])
        return result
    for result in unrecorded_results(cell, manual_text):  # finished before an interruption, never entered
        attempts.append({k: result.get(k) for k in SCORED})
        record_attempt({**cell, **attempts[-1], "terminal": terminal(result), "recovered_from_result": True})
        if terminal(result):
            result.update(attempts=attempts, recovered_from_ledger=True)
            return result
    while len(attempts) < 2:
        gid = new_game_id(task_id, player, condition)
        record_attempt({**cell, "game": gid, "started": True, "terminal": False})  # booked before play
        try:
            result = play_game(task_id, player, manual_text, max_actions, condition=condition, gid=gid)
        except Exception as e:  # our code or QEMU failed
            result = salvage(gid, player, condition, manual_text, e)  # a saved result / known violation stands
            if result is None:
                attempts.append({"game": gid, "harness_errors": [f"crash: {e!r}"]})
                record_attempt({**cell, **attempts[-1], "terminal": False})
                continue
        attempts.append({k: result.get(k) for k in SCORED})
        record_attempt({**cell, **attempts[-1], "terminal": terminal(result)})
        if terminal(result):
            result["attempts"] = attempts
            return result
    raise HarnessError(f"harness error twice: {attempts}")


@solver
def unseen_player(player: str, texts: dict, max_actions: int):
    async def solve(state: TaskState, generate: Generate) -> TaskState:
        task_id = state.metadata["task"]
        condition = state.metadata["condition"]
        text = texts.get(condition)
        check_condition(condition, text)
        result = await anyio.to_thread.run_sync(run_with_retry, task_id, player, text, max_actions,
                                                condition, state.epoch)
        if result.get("condition") not in (None, condition):
            raise HarnessError(f"game played as {result.get('condition')}, sample is {condition}")
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
                     metadata={k: r.get(k) for k in ("game", "condition", "context_sha256",
                                                     "actions", "seconds", "timed_out", "settle_failures",
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
    if not conds or len(set(conds)) != len(conds) or any(c not in CONDITIONS for c in conds):
        raise ValueError(f"conditions must be distinct values from {CONDITIONS}, got {conds}")
    wanted = set(tasks.split(",") if isinstance(tasks, str) else tasks) if tasks else None
    manual_path = Path(manual) if manual else DEFAULT_MANUAL
    texts = {}
    if "manual" in conds:
        texts["manual"] = manual_path.read_text(encoding="utf-8")
    if "reference" in conds:
        texts["reference"] = reference_text(manual_path)
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
    return Task(dataset=dataset, solver=unseen_player(player, texts, max_actions), scorer=end_state(),
                metadata={"player": player, "conditions": conds, "seed": seed, "taskset": taskset, "max_actions": max_actions,
                          "manual": str(manual_path) if texts else None,
                          "context_sha256": {c: sha256(t) for c, t in texts.items()}})
