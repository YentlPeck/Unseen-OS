"""Run one task with one player and score it.

  python tools/run_task.py TASK PLAYER      (PLAYER: haiku | luna | script)

How a game is kept fair:
  * The player's folder (in the system temp dir, outside this project) holds
    only play.py and a game.json with the referee's port and a secret token.
    No start files, no tools, no paths into this project.
  * The referee (broker.py) is the only thing connected to the OS. It handles
    one action at a time and enforces the action and time limits.
  * At the end the referee closes first (no more actions possible), leftover
    player processes are killed, and only then are the files read and scored.
  * Every player event is audited: any command other than game-tool calls, or
    any other kind of tool use (file edits, MCP, web...), fails the audit.
    play.py and game.json must be unchanged at the end.
  * Our own failures (OS start, settle, bridge, player CLI errors) are marked
    as harness errors, separate from the player's result.
Harness records go to out/runs/<game id>/. `play_game` is also used by Inspect.
"""

import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import threading
import time
import uuid
from pathlib import Path

TOOLS = Path(__file__).resolve().parent
ROOT = TOOLS.parent
sys.path.insert(0, str(TOOLS))
from audit import audit  # noqa: E402
from broker import Broker  # noqa: E402
from tasks import TASKS, strict_check  # noqa: E402
from unseen import Machine, SettleError  # noqa: E402
from hidden import run_hidden  # noqa: E402

MAX_ACTIONS = 40
V3_TIP = ("\nWhen the text you type contains quote marks, put double quotes around the whole text and use "
          "single quotes inside it.")
TIME_LIMIT = 15 * 60  # seconds
MODELS = {"haiku": "claude-haiku-5-5", "luna": "gpt-6-luna"}
GAMES_DIR = Path(tempfile.gettempdir()) / "unseen-games"

PROMPT = """You are using a computer you have never seen before. You can only use it through one tool,
run from the current folder:

  python play.py {gid} look              see the screen
  python play.py {gid} type "some text"  type text on one line (this does NOT press Enter)
  python play.py {gid} press KEY [KEY]   press keys: enter, esc, backspace, tab, up, down,
                                         left, right, home, end, delete, space, or ctrl+<letter>

Every action shows you the screen afterwards. You have at most {max_actions} actions.
Do not run any other commands and do not read any files.{tips}
{manual}
Your task: {request}

Change only what the task asks for: leave everything else exactly as it was, and don't leave
extra pages or drawers behind. When the task is done (or you cannot finish it), stop and give a
one-line summary."""


# Every game gets a FRESH setup folder for the player's CLI, holding only the login:
# none of the owner's plugins, hooks, memory, personal instructions or MCP servers
# (claude-mem, Codex computer use, ...) and nothing left over from earlier games. The login
# is kept in one place (LOGINS) and copied in; if the CLI refreshed it during the game, the
# newer login is copied back. The game's folder is deleted afterwards.
CLEAN_HOMES = Path(os.environ.get("LOCALAPPDATA", tempfile.gettempdir())) / "unseen-os"
LOGINS = CLEAN_HOMES / "logins"
LOGIN_FILES = {"haiku": (Path.home() / ".claude" / ".credentials.json", "CLAUDE_CONFIG_DIR"),
               "luna": (Path.home() / ".codex" / "auth.json", "CODEX_HOME")}
LUNA_CONFIG = ('model = "gpt-6-luna"\nmodel_reasoning_effort = "high"\n\n'
               '[windows]\nsandbox = "unelevated"\n\n[features]\nmemories = false\n')
LUNA_TOOLS_OFF = ["--disable", "computer_use", "--disable", "apps", "--disable", "memories"]
_login_lock = threading.Lock()


def login_freshness(player, path):
    """How fresh a login file is (bigger is fresher; -1 if missing or broken). Haiku: the
    login's own expiry time. Luna: its last refresh time (else the file's date)."""
    try:
        d = json.loads(Path(path).read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return -1
    if player == "haiku":
        o = d.get("claudeAiOauth") or {}
        return o.get("expiresAt") or -1 if o.get("accessToken") else -1
    if not d.get("tokens") and not d.get("OPENAI_API_KEY"):
        return -1
    try:
        import datetime as _dt
        return _dt.datetime.fromisoformat(str(d["last_refresh"]).replace("Z", "+00:00")).timestamp() * 1000
    except (KeyError, ValueError):
        return Path(path).stat().st_mtime * 1000


def fresh_home(player, gid):
    """A new, empty setup folder for this game, with only the login (and Luna's two settings).
    Returns (env for the player process, folder)."""
    source, var = LOGIN_FILES[player]
    master = LOGINS / player / source.name
    with _login_lock:
        # Use the freshest login: the owner's own CLI keeps refreshing its login, and an old
        # copy's refresh token stops working. A fresh copy also means the game needn't refresh it.
        if login_freshness(player, source) > login_freshness(player, master):
            master.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy(source, master)
    home = CLEAN_HOMES / "games" / gid
    home.mkdir(parents=True)
    shutil.copy(master, home / source.name)
    if player == "luna":
        (home / "config.toml").write_text(LUNA_CONFIG)
    return {**os.environ, var: str(home)}, home


def end_home(player, home):
    """Keep a refreshed login, then delete the game's setup folder."""
    name = LOGIN_FILES[player][0].name
    used, master = home / name, LOGINS / player / name
    with _login_lock:
        # only a login the game really refreshed (fresher, not broken) is kept
        if used.exists() and login_freshness(player, used) > login_freshness(player, master):
            shutil.copy(used, master)
    shutil.rmtree(home, ignore_errors=True)


def player_command(player, gid, game_dir):
    if player == "haiku":
        return ["claude", "-p", "--model", MODELS["haiku"], "--tools", "Bash",
                "--allowedTools", f"Bash(python play.py {gid}:*)",
                "--disallowedTools", "Read,Glob,Grep,WebFetch,WebSearch,Agent,Task",
                "--strict-mcp-config", "--mcp-config", '{"mcpServers":{}}',
                "--permission-mode", "dontAsk", "--no-session-persistence",
                "--output-format", "stream-json", "--verbose"]
    if player == "luna":
        return ["codex", "exec", "-m", MODELS["luna"], "-C", str(game_dir), "--skip-git-repo-check",
                "-s", "workspace-write", "-c", "sandbox_workspace_write.network_access=true",
                *LUNA_TOOLS_OFF, "--json", "-"]
    if player.startswith("robot:"):  # test player: plays a task's reference solution through play.py
        return [sys.executable, str(TOOLS / "robot_player.py"), player.split(":", 1)[1]]
    raise ValueError(f"unknown player {player}")


def kill_leftovers(gid):
    """Kill leftover player processes that still call the game tool for this game.
    (Only 'play.py <gid>' processes: the OS copy also has the game id in its
    command line and must stay alive until the files are read.)"""
    ps = (f"Get-CimInstance Win32_Process | Where-Object {{ $_.CommandLine -like '*play.py {gid}*' -and "
          f"$_.Name -notlike 'qemu*' -and $_.ProcessId -ne $PID }} | "
          f"ForEach-Object {{ Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }}")
    subprocess.run(["powershell", "-NoProfile", "-Command", ps], capture_output=True)


def run_player(cmd, prompt, cwd, log_path, env=None):
    """Run the player. Returns (timed_out, exit code). Kills the process tree on time-out."""
    flags = getattr(subprocess, "CREATE_NEW_PROCESS_GROUP", 0) | getattr(subprocess, "CREATE_NO_WINDOW", 0)
    with open(log_path, "w", encoding="utf-8") as out:
        proc = subprocess.Popen(cmd, stdin=subprocess.PIPE, stdout=out, stderr=subprocess.STDOUT,
                                cwd=cwd, text=True, encoding="utf-8", creationflags=flags, env=env)
        try:
            proc.communicate(prompt, timeout=TIME_LIMIT)
            return False, proc.returncode
        except subprocess.TimeoutExpired:
            subprocess.run(["taskkill", "/PID", str(proc.pid), "/T", "/F"], capture_output=True)
            proc.wait()
            out.write("\n[harness] time limit reached; player process tree killed\n")
            return True, proc.returncode


LUNA_HARMLESS = {"agent_message", "reasoning", "todo_list"}


def player_events(player, log_text):
    """From the player's JSON log: (commands run, other tool uses, final message, CLI error)."""
    commands, others, final, cli_error = {}, [], "", None
    for line in log_text.splitlines():
        try:
            e = json.loads(line)
        except ValueError:
            continue
        if player == "haiku":
            if e.get("type") == "assistant":
                for c in e.get("message", {}).get("content", []):
                    if c.get("type") == "tool_use":
                        if c.get("name") == "Bash":
                            commands[c.get("id") or len(commands)] = c.get("input", {}).get("command", "")
                        else:
                            others.append(f"{c.get('name')}: {json.dumps(c.get('input'))[:200]}")
            elif e.get("type") == "result":
                final = e.get("result") or final
                if e.get("is_error"):
                    cli_error = f"claude result error: {e.get('subtype')} {str(e.get('result'))[:200]}"
        elif player.startswith("robot"):
            if e.get("type") == "robot_command":
                commands[len(commands)] = e["command"]
            elif e.get("type") == "robot_result":
                final = e.get("text", "")
        elif player == "luna":
            if e.get("type") in ("error", "turn.failed"):
                cli_error = f"codex {e.get('type')}: {json.dumps(e)[:300]}"
            item = e.get("item") or {}
            kind = item.get("type")
            if e.get("type") in ("item.started", "item.completed") and kind:
                if kind == "command_execution":
                    commands[item.get("id") or len(commands)] = item.get("command", "")
                elif kind == "agent_message":
                    final = item.get("text") or final
                elif kind not in LUNA_HARMLESS:
                    others.append(f"{kind}: {json.dumps(item)[:200]}")
    return list(commands.values()), others, final, cli_error


def infrastructure_problem(log_path, commands, summary, actions):
    """A reason if the game never really ran because of the player's CLI (not the model):
    - the CLI itself (not the model's text) logged that a command was blocked by its policy,
      or that it is not logged in. Only lines that are NOT JSON events are searched: those
      are the CLI's own messages; everything the model writes is inside JSON events;
    - or nothing happened at all: no command, no message, no game action.
    A model that refuses, or only makes mistyped calls, simply loses: that is behaviour."""
    if not log_path.exists():
        return "player CLI wrote no log"
    for line in log_path.read_text(encoding="utf-8", errors="replace").splitlines():
        if line.lstrip().startswith("{"):
            continue
        if re.search(r"blocked by policy|not logged in|not authenticated|please log in|login required", line, re.I):
            return f"player CLI: {line.strip()[:200]}"
    if actions == 0 and not commands and not summary.strip():
        return "player CLI did nothing (no command, no message, no game action)"
    return None


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest() if Path(path).exists() else None


def play_game(task_id, player, manual_text=None, max_actions=MAX_ACTIONS):
    """Play one game. Returns a result dict (also saved as result.json)."""
    task = TASKS[task_id]
    gid = f"{task_id}-{player}-{'manual' if manual_text else 'plain'}-{uuid.uuid4().hex[:6]}"
    run_dir = ROOT / "out" / "runs" / gid          # harness only
    game_dir = GAMES_DIR / gid                       # what the player sees
    run_dir.mkdir(parents=True)
    game_dir.mkdir(parents=True)
    shutil.copy(TOOLS / "play.py", game_dir / "play.py")

    summary, commands, others, cli_error, timed_out, exit_code = "", [], [], None, False, 0
    with open(run_dir / "qemu.log", "w") as qlog:
        machine = Machine.start(task["start"], workdir=run_dir, log=qlog)
        started = time.time()
        broker = Broker(machine, max_actions, run_dir / "transcript.jsonl", deadline=started + TIME_LIMIT)
        (game_dir / "game.json").write_text(json.dumps({"game": gid, "port": broker.port, "token": broker.token}))
        protected = {f: sha(game_dir / f) for f in ("play.py", "game.json")}
        try:
            if player == "script":
                machine.run_keys(task["solution"])
                screen = machine.settle()
                (run_dir / "transcript.jsonl").write_text(
                    json.dumps({"n": 1, "action": "solution", "args": [task["solution"]], "screen": screen}) + "\n")
                summary = "ran the reference solution"
            else:
                manual = f"\nHere is the manual for this computer:\n\n{manual_text}\n" if manual_text else ""
                prompt = PROMPT.format(gid=gid, max_actions=max_actions, request=task["request"], manual=manual,
                                       tips=V3_TIP if task.get("v3") else "")
                (run_dir / "prompt.txt").write_text(prompt, encoding="utf-8")
                cmd = player_command(f"robot:{task_id}" if player == "robot" else player, gid, game_dir)
                cmd[0] = shutil.which(cmd[0]) or cmd[0]
                if player in LOGIN_FILES:
                    env, home = fresh_home(player, gid)
                    try:
                        timed_out, exit_code = run_player(cmd, prompt, game_dir, run_dir / "player.log", env)
                    finally:
                        end_home(player, home)
                else:
                    timed_out, exit_code = run_player(cmd, prompt, game_dir, run_dir / "player.log")
                commands, others, summary, cli_error = player_events(
                    player, (run_dir / "player.log").read_text(encoding="utf-8", errors="replace"))
            broker.close()                          # no more actions from here on
            if player != "script":
                kill_leftovers(gid)
            files = None
            if broker.os_crash is None:
                try:
                    machine.settle()
                    files = machine.dump()
                except (SettleError, TimeoutError, RuntimeError) as e:
                    if not machine.os_crashed():
                        raise  # our side failed: a harness error (re-run)
                    broker.os_crash = f"at the end of the game: {e}"
            if files is None:  # the player's actions crashed the OS: scored as a loss
                files = {}
            tampered = [f for f, h in protected.items() if sha(game_dir / f) != h]
        finally:
            broker.close()
            machine.stop()
            shutil.rmtree(game_dir, ignore_errors=True)

    if exit_code not in (0, None) and not timed_out and not cli_error:
        cli_error = f"player CLI exited with code {exit_code}"
    if player not in ("script", "robot") and not cli_error:
        cli_error = infrastructure_problem(run_dir / "player.log", commands, summary, broker.used)
    passed, rule = strict_check(task_id, files)
    if broker.os_crash:
        passed, rule = False, f"the OS stopped responding ({broker.os_crash})"
    visible_passed, hidden = passed, None
    os_crash = broker.os_crash
    if task.get("hidden") and not os_crash:  # fix/write: the chore must also work on hidden worlds
        hidden = run_hidden(task, files.get(task["chore_page"]))
        if not all(h["ok"] for h in hidden):
            passed = False
            rule += " | hidden worlds: " + "; ".join(h["why"] for h in hidden if not h["ok"])
        crashed = [h for h in hidden if h.get("os_crash")]
        if crashed:
            os_crash = f"on a hidden world: {crashed[0]['why']}"
    violations = audit(commands, gid) + others + [f"changed {f}" for f in tampered] if player != "script" else []
    harness_errors = ([f"settle failures: {broker.settle_failures}"] if broker.settle_failures else []) \
        + [f"infra: {e}" for e in broker.infra_errors] + ([cli_error] if cli_error else [])
    transcript = []
    if (run_dir / "transcript.jsonl").exists():
        transcript = [json.loads(l) for l in (run_dir / "transcript.jsonl").read_text(encoding="utf-8").splitlines()]
    result = {"game": gid, "task": task_id, "level": task["level"], "player": player, "manual": bool(manual_text),
              "passed": passed, "visible_passed": visible_passed, "hidden": hidden, "rule": rule,
              "os_crash": os_crash,
              "actions": broker.used if player != "script" else 1,
              "seconds": round(time.time() - started), "timed_out": timed_out,
              "settle_failures": broker.settle_failures, "harness_errors": harness_errors,
              "audit_ok": not violations, "violations": violations, "summary": summary, "files": files}
    (run_dir / "result.json").write_text(json.dumps(result, indent=2))
    result["transcript"] = [t for t in transcript if "action" in t]
    return result


def main():
    r = play_game(sys.argv[1], sys.argv[2])
    flags = ("" if r["audit_ok"] else "  AUDIT FAILED") + ("  HARNESS ERROR" if r["harness_errors"] else "")
    print(f"{'PASS' if r['passed'] else 'FAIL'}  {r['game']}  ({r['actions']} actions, {r['seconds']}s){flags}")
    if not r["passed"]:
        print("  ", r["rule"])


if __name__ == "__main__":
    main()
