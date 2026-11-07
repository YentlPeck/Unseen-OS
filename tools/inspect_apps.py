"""Inspect harness for experiment 5, the Unseen App Benchmark (docs/benchmark-design.md).

  inspect eval tools/inspect_apps.py -T player=robot --model mockllm/model --max-samples 2
  inspect view

Options (-T):
  player      haiku | luna | robot   (robot = the known solutions, to test the harness)
  conditions  plain,reference,manual  (default: manual) which conditions to include
              reference = only the Unseen Script reference (kernel/src/script_guide.txt)
  kinds       build,feature,fix       (default: all)
  level       easy | medium | hard    (default: all)
  tasks       comma-separated task ids (default: all 24)
  seed        shuffle seed for the sample order (default: task order)
  max_actions actions per game (default 200)
  manifest    only recorded: the hash of the frozen manifest

Each sample is one (task, condition) game, played by an outside command-line agent (claude /
codex); `--model mockllm/model` is a placeholder. The game's actions and screens are copied
into the sample's messages so the viewer can replay them. The rules for attempts are
experiment 4's (tools/inspect_eval.py, unchanged): an audit failure and an OS crash stand;
a harness error gets exactly one re-run; every attempt goes to an append-only ledger
($UNSEEN_LEDGER, else out/apps-dev/attempts-ledger.jsonl); a finished game that was recorded is
never replayed.
"""

import contextlib
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
import apps_game as A  # noqa: E402
from audit_apps import audit_apps  # noqa: E402
from run_task import MODELS, player_events  # noqa: E402
from tasks_bench import TASKS  # noqa: E402

LEDGER = Path(os.environ.get("UNSEEN_LEDGER") or TOOLS.parent / "out" / "apps-dev" / "attempts-ledger.jsonl")
_ledger_lock = threading.Lock()
SCORED = ("game", "condition", "context_sha256", "passed", "rule", "audit_ok", "violations", "timed_out",
          "harness_errors", "actions", "seconds", "settle_failures", "hidden", "os_crash", "summary")


class HarnessError(RuntimeError):
    """Our side failed twice: Inspect marks the sample as errored (re-run later with eval-retry)."""


@contextlib.contextmanager
def ledger_lock():
    """The ledger is shared by the Inspect processes of both players: every read and write holds
    a lock on a lock file (an interprocess lock), and within a process the thread lock too."""
    LEDGER.parent.mkdir(parents=True, exist_ok=True)
    with _ledger_lock, open(LEDGER.with_name(LEDGER.name + ".lock"), "a+b") as f:
        if sys.platform == "win32":
            import msvcrt
            while True:
                try:
                    f.seek(0)
                    msvcrt.locking(f.fileno(), msvcrt.LK_LOCK, 1)
                    break
                except OSError:  # LK_LOCK gives up after 10 s: keep waiting
                    time.sleep(0.05)
            try:
                yield
            finally:
                f.seek(0)
                msvcrt.locking(f.fileno(), msvcrt.LK_UNLCK, 1)
        else:
            import fcntl
            fcntl.flock(f, fcntl.LOCK_EX)
            try:
                yield
            finally:
                fcntl.flock(f, fcntl.LOCK_UN)


def cut_off_tail():
    """A last line an interruption cut off (no new line at its end) is moved to LEDGER.cut and
    taken off the ledger, so the next row starts on a line of its own. Call with the lock held."""
    if not LEDGER.exists():
        return
    data = LEDGER.read_bytes()
    if not data or data.endswith(b"\n"):
        return
    keep = data[:data.rfind(b"\n") + 1]
    with open(LEDGER.with_name(LEDGER.name + ".cut"), "ab") as f:
        f.write(data[len(keep):] + b"\n")
    with open(LEDGER, "r+b") as f:
        f.truncate(len(keep))


def record_attempt(entry):
    with ledger_lock():
        cut_off_tail()
        with open(LEDGER, "a", encoding="utf-8") as f:
            f.write(json.dumps({"time": time.strftime("%Y-%m-%d %H:%M:%S"), **entry}) + "\n")
            f.flush()
            os.fsync(f.fileno())


def read_ledger():
    """Every complete row. A last line cut off by an interruption is left out; a broken line
    anywhere else is an error (the ledger must be read by hand)."""
    with ledger_lock():
        if not LEDGER.exists():
            return []
        lines = [l for l in LEDGER.read_text(encoding="utf-8").split("\n") if l.strip()]
    rows = []
    for i, line in enumerate(lines):
        try:
            rows.append(json.loads(line))
        except ValueError:
            if i == len(lines) - 1:
                break
            raise RuntimeError(f"the ledger {LEDGER} has a broken line {i + 1}; check it by hand")
    return rows


def terminal(result):
    """An audit failure stands, then an OS crash (a loss), then a harness error (re-run)."""
    return (not result.get("audit_ok")) or bool(result.get("os_crash")) or not result.get("harness_errors")


def ledger_rows(cell):
    if not os.environ.get("UNSEEN_LEDGER"):
        return []
    return [a for a in read_ledger() if all(a.get(k) == v for k, v in cell.items())]


def salvage(gid, player, condition, text, error):
    """After play_app_game itself failed: its saved result (or the complete temporary copy), else
    an audit of the player's own log. A known audit failure always stands."""
    run = A.RUNS / gid
    result = A.saved_result(gid)
    if result is None:
        try:
            result = json.loads((run / "result.json.tmp").read_text())
            result = result if isinstance(result, dict) and result.get("game") == gid and "passed" in result else None
        except Exception:
            result = None
        if result is not None:
            result["transcript"] = A.load_transcript(gid)
    if result is not None:
        return result
    try:
        log = (run / "player.log").read_text(encoding="utf-8", errors="replace")
        commands, others, summary, cli_error = player_events("robot" if player == "robot" else player, log)
    except Exception:
        return None
    violations = audit_apps(commands, gid) + others
    # What the game's records already show. The order is the normal one: an audit failure stands,
    # then an OS crash (a loss); then a harness error of ours (the referee's settle or infra
    # errors, or the player's CLI failing) means a re-run, even if the time limit was reached too;
    # only a time-out with none of these is a loss.
    os_crash, ours = None, []
    try:
        for line in (run / "transcript.jsonl").read_text(encoding="utf-8").splitlines():
            row = json.loads(line) if line.strip() else {}
            if not isinstance(row, dict):
                continue
            if row.get("os_crash"):
                os_crash = f"after action {row.get('n')}: {row['os_crash']}"
            for k in ("settle_error", "infra_error"):
                if row.get(k):
                    ours.append(f"{k} after action {row.get('n')}: {row[k]}")
    except (OSError, ValueError):
        pass
    if cli_error:
        ours.append(cli_error)
    timed_out = "[harness] time limit reached" in log
    if not violations and not os_crash and not timed_out:
        return None
    rule = ("audit failure" if violations else "the OS stopped responding" if os_crash
            else "harness error" if ours else "time limit reached")
    return {"game": gid, "condition": condition, "context_sha256": A.sha256(text), "passed": False, "hidden": None,
            "os_crash": os_crash, "timed_out": timed_out, "rule": rule + " (read from the game's records after our side failed)",
            "audit_ok": not violations, "violations": violations,
            "harness_errors": ([f"crash: {error!r}"] + ours) if violations or os_crash or ours else [],
            "actions": None, "seconds": None, "settle_failures": 0, "summary": summary, "transcript": A.load_transcript(gid)}


def unrecorded_results(cell, text):
    rows = ledger_rows(cell)
    entered = {a.get("game") for a in rows if "passed" in a or ("harness_errors" in a and not a.get("started"))}
    out = []
    for a in rows:
        if a.get("started") and a.get("game") not in entered:
            r = salvage(a["game"], cell["player"], cell["condition"], text, "interrupted (found on restart)")
            if r is not None and r.get("condition") == cell["condition"]:
                out.append(r)
    return out


def used_attempts(rows):
    """The finished attempts the ledger already holds for a cell (harness errors), one per game:
    they count toward the two attempts, also after a restart. A booking that never finished is
    not one (unrecorded_results looks at those)."""
    seen, out = set(), []
    for a in rows:
        if a.get("started") or a.get("terminal") or a.get("game") in seen:
            continue
        if "harness_errors" in a:
            seen.add(a.get("game"))
            out.append({k: a.get(k) for k in SCORED})
    return out


def run_with_retry(task_id, player, text, max_actions, condition, epoch=1):
    cell = {"player": player, "task": task_id, "condition": condition, "epoch": epoch}
    rows = ledger_rows(cell)
    attempts = used_attempts(rows)
    done = next((a for a in rows if a.get("terminal")), None)
    if done:
        result = {k: done.get(k) for k in SCORED}
        result.update(transcript=A.load_transcript(done.get("game")) if done.get("game") else [],
                      recovered_from_ledger=True, attempts=[{k: done.get(k) for k in SCORED}])
        return result
    for result in unrecorded_results(cell, text):
        attempts.append({k: result.get(k) for k in SCORED})
        record_attempt({**cell, **attempts[-1], "terminal": terminal(result), "recovered_from_result": True})
        if terminal(result):
            result.update(attempts=attempts, recovered_from_ledger=True)
            return result
    while len(attempts) < 2:
        gid = A.new_game_id(task_id, player, condition)
        record_attempt({**cell, "game": gid, "started": True, "terminal": False})  # booked before play
        try:
            result = A.play_app_game(task_id, player, text, max_actions, condition=condition, gid=gid)
        except Exception as e:
            result = salvage(gid, player, condition, text, e)
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
def app_player(player: str, texts: dict, max_actions: int):
    async def solve(state: TaskState, generate: Generate) -> TaskState:
        task_id, condition = state.metadata["task"], state.metadata["condition"]
        text = texts.get(condition)
        result = await anyio.to_thread.run_sync(run_with_retry, task_id, player, text, max_actions, condition, state.epoch)
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


@scorer(metrics=[accuracy(), stderr(), grouped(accuracy(), "condition"), grouped(accuracy(), "kind"),
                 grouped(accuracy(), "level")])
def hidden_tests():
    async def score(state: TaskState, target: Target) -> Score:
        r = state.metadata["result"]
        passed = bool(r["passed"]) and r["audit_ok"] and not r["timed_out"] and not r.get("os_crash")
        explanation = (f"Rule: {r['rule']}" + ("" if r["audit_ok"] else " | AUDIT FAILED")
                       + (" | OS CRASH" if r.get("os_crash") else "") + (" | TIMED OUT" if r["timed_out"] else ""))
        return Score(value=CORRECT if passed else INCORRECT, explanation=explanation,
                     metadata={k: r.get(k) for k in ("game", "condition", "context_sha256", "actions", "seconds",
                                                     "timed_out", "settle_failures", "audit_ok", "violations",
                                                     "harness_errors", "attempts", "hidden", "os_crash",
                                                     "recovered_from_ledger")})

    return score


def _list(x):
    return [v for v in (x.split(",") if isinstance(x, str) else list(x)) if v] if x else None


@task
def unseen_apps(player: str = "robot", conditions: str | list[str] = "manual", kinds: str | list[str] | None = None,
                level: str | None = None, tasks: str | list[str] | None = None, seed: int | None = None,
                max_actions: int = A.MAX_ACTIONS, manifest: str | None = None):
    conds = _list(conditions)
    if not conds or len(set(conds)) != len(conds) or any(c not in A.CONDITIONS for c in conds):
        raise ValueError(f"conditions must be distinct values from {A.CONDITIONS}, got {conds}")
    texts = {c: A.context_text(c) for c in conds if c != "plain"}
    wanted, kind_set = set(_list(tasks) or []), set(_list(kinds) or [])
    samples = [
        Sample(id=f"{tid}:{cond}", input=t["request"], target="every hidden test passes",
               metadata={"task": tid, "condition": cond, "kind": t["kind"], "level": t["level"]})
        for tid, t in TASKS.items()
        for cond in conds
        if (not wanted or tid in wanted) and (not kind_set or t["kind"] in kind_set) and (level is None or t["level"] == level)
    ]
    dataset = MemoryDataset(samples)
    if seed is not None:
        dataset.shuffle(seed=seed)
    return Task(dataset=dataset, solver=app_player(player, texts, max_actions), scorer=hidden_tests(),
                metadata={"player": player, "conditions": conds, "seed": seed, "max_actions": max_actions,
                          "context_sha256": {c: A.sha256(t) for c, t in texts.items()}})
