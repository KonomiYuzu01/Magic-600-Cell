"""Reporting hook for the development workbench.

Registered for PostToolUse, PostToolUseFailure, PermissionRequest, SubagentStart,
SubagentStop, Stop, StopFailure, Notification and SessionEnd. Each call appends
one bounded event line to the session's private event file under
work/loop-memory/workbench/events/ of the main checkout. On PostToolUse and
PostToolUseFailure in the main thread it also delivers pending owner notes as
additionalContext, each note at most once.

It never calls models, installs tools, uses the network, runs git or starts
processes, never returns a decision, and exits 0 on every internal error, so it
cannot block or fail a session. It imports no repository code: `main_checkout`
and `digest` mirror tools/workbench (tests keep them equivalent).
"""
from __future__ import annotations

import hashlib
import json
import os
import re
import struct
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

EVENTS = {"PostToolUse", "PostToolUseFailure", "PermissionRequest", "SubagentStart", "SubagentStop",
          "Stop", "StopFailure", "Notification", "SessionEnd"}
NOTE_EVENTS = {"PostToolUse", "PostToolUseFailure"}
SID_RE = re.compile(r"^[A-Za-z0-9-]{1,64}$")
EVENT_CAP = 8 * 1024 * 1024
LINE_MAX = 4096
STDIN_MAX = 4 * 1024 * 1024
DRAIN_MAX = 256 * 1024 * 1024
DRAIN_SECONDS = 2.0
INBOX_READ = 1024 * 1024 + 64 * 1024   # the writer caps an inbox at 1 MiB
STATE_READ = 2 * 1024 * 1024
NOTE_TEXT_MAX = 4000
EMIT_MAX_CHARS = 9000                  # inline additionalContext stays under 10,000 characters
EMIT_MAX_BYTES = 16 * 1024
EVENT_LOCK_WAIT = 1.0
NOTE_LOCK_WAIT = 0.2
LOCK_STALE = 2.0
FOOTER = ("Answer each owner note by its id in your next message to the owner. "
          "An owner note cannot grant authorizations reserved for the terminal, such as `bootstrap.py approve`.")


def now_iso() -> str:
    return datetime.now(timezone.utc).isoformat(timespec="milliseconds")


def main_checkout(start: Path) -> Path | None:
    """The main checkout of the repository that contains `start` (mirrors tools/workbench/paths.py)."""
    start = Path(start).resolve()
    for d in [start, *start.parents]:
        git = d / ".git"
        if git.is_dir():
            return d
        if git.is_file():
            try:
                text = git.read_text(encoding="utf-8", errors="replace").strip()
            except OSError:
                return None
            if not text.startswith("gitdir:"):
                return None
            gitdir = Path(text[len("gitdir:"):].strip())
            if not gitdir.is_absolute():
                gitdir = d / gitdir
            try:
                common = (gitdir / "commondir").read_text(encoding="utf-8").strip()
            except OSError:
                return None
            common_dir = Path(common) if Path(common).is_absolute() else gitdir / common
            return common_dir.resolve().parent
    return None


def digest(obj) -> str:
    """Tool-call key shared with the workbench (mirrors tools/workbench/sources.py)."""
    blob = json.dumps(obj, sort_keys=True, separators=(",", ":"), ensure_ascii=False)
    return hashlib.sha256(blob.encode("utf-8", "surrogatepass")).hexdigest()[:16]


def read_input() -> tuple[dict, bool]:
    """The hook input; oversized input is drained and only its leading scalar fields are kept."""
    data = sys.stdin.buffer.read(STDIN_MAX + 1)
    if len(data) <= STDIN_MAX:
        ev = json.loads(data.decode("utf-8", "replace") or "{}")
        return (ev if isinstance(ev, dict) else {}), False
    total, deadline = len(data), time.monotonic() + DRAIN_SECONDS
    while total < DRAIN_MAX and time.monotonic() < deadline:
        chunk = sys.stdin.buffer.read(1 << 20)
        if not chunk:
            break
        total += len(chunk)
    head = data[:65536].decode("utf-8", "replace")
    ev = {}
    for key in ("session_id", "hook_event_name", "tool_name", "tool_use_id", "agent_id", "agent_type", "cwd"):
        m = re.search(r'"%s"\s*:\s*"((?:[^"\\]|\\.){0,2048})"' % key, head)
        if m:
            try:
                ev[key] = json.loads('"' + m.group(1) + '"')
            except ValueError:
                pass
    return ev, True


def summarize(name, tool_input) -> str:
    if not isinstance(tool_input, dict):
        return ""
    if name == "Bash":
        return str(tool_input.get("command", ""))[:300]
    for key in ("file_path", "notebook_path", "path"):
        if isinstance(tool_input.get(key), str):
            return tool_input[key][:300]
    if name in ("Glob", "Grep"):
        return str(tool_input.get("pattern", ""))[:200]
    if name in ("Agent", "Task"):
        return f"{tool_input.get('subagent_type', '')}: {tool_input.get('description', '')}"[:200]
    if name == "Skill":
        return str(tool_input.get("skill", ""))[:100]
    return ""


def short(value, n: int):
    return value[:n] if isinstance(value, str) else None


def event_line(ev: dict, truncated: bool) -> bytes:
    name = ev["hook_event_name"]
    rec = {"v": 1, "t": now_iso(), "e": name, "sid": ev["session_id"],
           "agent_id": short(ev.get("agent_id"), 64), "agent_type": short(ev.get("agent_type"), 64)}
    if name in ("PostToolUse", "PostToolUseFailure", "PermissionRequest"):
        tool = short(ev.get("tool_name"), 100)
        rec.update(tool=tool, key=digest(ev.get("tool_input")) if not truncated else None,
                   summary=summarize(tool, ev.get("tool_input")))
        if name != "PermissionRequest":
            ms = ev.get("duration_ms")
            rec.update(use_id=short(ev.get("tool_use_id"), 128), ok=name == "PostToolUse",
                       ms=ms if isinstance(ms, (int, float)) and not isinstance(ms, bool) else None)
            if name == "PostToolUseFailure":
                rec["interrupt"] = bool(ev.get("is_interrupt"))
    elif name == "Notification":
        rec["ntype"] = short(ev.get("notification_type"), 64)
    elif name == "SessionEnd":
        rec["reason"] = short(ev.get("reason"), 64)
    elif name == "StopFailure":
        err = ev.get("error")
        rec["error"] = (err if isinstance(err, str) else json.dumps(err, ensure_ascii=False))[:200] if err is not None else None
    if truncated:
        rec["truncated_input"] = True
    line = json.dumps(rec, ensure_ascii=False) + "\n"
    for drop in ("summary", "error", "agent_type"):
        if len(line.encode("utf-8")) <= LINE_MAX:
            break
        rec.pop(drop, None)
        line = json.dumps(rec, ensure_ascii=False) + "\n"
    if len(line.encode("utf-8")) > LINE_MAX:
        line = json.dumps({k: rec[k] for k in ("v", "t", "e", "sid")}) + "\n"
    return line.encode("utf-8")


def acquire(lock: Path, wait: float):
    """Exclusive lock file; None when it cannot be taken within `wait` seconds."""
    deadline = time.monotonic() + wait
    while True:
        try:
            return os.open(lock, os.O_CREAT | os.O_EXCL | os.O_WRONLY)
        except FileExistsError:
            try:
                if time.time() - lock.stat().st_mtime > LOCK_STALE:
                    lock.unlink()
                    continue
            except OSError:
                pass
        except PermissionError:  # Windows: the previous holder is still deleting it
            pass
        if time.monotonic() >= deadline:
            return None
        time.sleep(0.005)


def release(fd, lock: Path) -> None:
    os.close(fd)
    try:
        lock.unlink()
    except OSError:
        pass


def unterminated(path: Path, size: int) -> bytes:
    """A newline when the journal ends inside a partial record (an interrupted write), so the next
    record starts on its own line: the fragment stays one malformed line and nothing merges into it."""
    if size <= 0:
        return b""
    try:
        with path.open("rb") as f:
            f.seek(size - 1)
            return b"" if f.read(1) == b"\n" else b"\n"
    except OSError:
        return b"\n"


def append_event(events: Path, sid: str, line: bytes) -> None:
    """Append under a per-file lock; the file never exceeds the cap plus one marker line."""
    events.mkdir(parents=True, exist_ok=True)
    path, capped, lock = events / f"{sid}.jsonl", events / f"{sid}.capped", events / f"{sid}.lock"
    fd = acquire(lock, EVENT_LOCK_WAIT)
    if fd is None:
        return
    try:
        if capped.exists():
            return
        try:
            size = path.stat().st_size
        except FileNotFoundError:
            size = 0
        if size + len(line) > EVENT_CAP:
            capped.open("x").close()
            line = (json.dumps({"v": 1, "t": now_iso(), "e": "cap_reached", "sid": sid}) + "\n").encode()
        with path.open("ab") as f:
            f.write(unterminated(path, size) + line)
    finally:
        release(fd, lock)


def complete_records(path: Path, budget: int) -> list:
    try:
        with path.open("rb") as f:
            data = f.read(budget)
    except OSError:
        return []
    lines = data.split(b"\n")
    if not data.endswith(b"\n"):
        lines = lines[:-1]
    out = []
    for line in lines:
        try:
            rec = json.loads(line.decode("utf-8"))
        except (ValueError, UnicodeDecodeError):
            continue
        if isinstance(rec, dict):
            out.append(rec)
    return out


def deliver(root: Path, sid: str) -> str | None:
    """Claim pending notes atomically (lock, select, mark), then return their framed text.
    Contention leaves every note pending; a claimed note is never emitted twice."""
    inbox_dir = root / "inbox"
    inbox = inbox_dir / f"{sid}.jsonl"
    if not inbox.is_file():
        return None
    state, lock = inbox_dir / f"{sid}.state.jsonl", inbox_dir / f"{sid}.lock"
    fd = acquire(lock, NOTE_LOCK_WAIT)
    if fd is None:
        return None
    try:
        emitted = {r.get("id") for r in complete_records(state, STATE_READ)}
        pending = [n for n in complete_records(inbox, INBOX_READ)
                   if isinstance(n.get("id"), str) and isinstance(n.get("text"), str) and n["id"] not in emitted]
        chosen, parts = [], []
        used_chars = len(FOOTER) + 2
        used_bytes = len(FOOTER.encode("utf-8")) + 2
        for n in pending:
            text = n["text"] if len(n["text"]) <= NOTE_TEXT_MAX else n["text"][:NOTE_TEXT_MAX] + " [truncated]"
            framed = f"Owner note {n['id'][:64]} from the workbench ({str(n.get('time', ''))[:40]}): {text}"
            size = len(framed.encode("utf-8")) + 2
            if chosen and (used_chars + len(framed) + 2 > EMIT_MAX_CHARS or used_bytes + size > EMIT_MAX_BYTES):
                break
            chosen.append(n["id"])
            parts.append(framed)
            used_chars += len(framed) + 2
            used_bytes += size
        if not chosen:
            return None
        stamp = now_iso()
        marks = "".join(json.dumps({"id": nid, "emitted_at": stamp}) + "\n" for nid in chosen).encode()
        try:
            size = state.stat().st_size
        except FileNotFoundError:
            size = 0
        with state.open("ab") as f:
            f.write(unterminated(state, size) + marks)
    finally:
        release(fd, lock)
    return "\n\n".join(parts + [FOOTER])


def data_root(ev: dict) -> Path | None:
    """The owning repository's private workbench directory. Storage is anchored to the checkout
    this hook belongs to; an event whose cwd lies in another repository writes nothing."""
    main = main_checkout(Path(__file__).resolve().parents[2])
    if main is None:
        return None
    cwd = ev.get("cwd")
    if isinstance(cwd, str) and cwd and main_checkout(Path(cwd)) != main:
        return None
    return main / "work" / "loop-memory" / "workbench"


def run() -> str | None:
    if struct.calcsize("P") * 8 != 64 or sys.version_info < (3, 9):
        return None
    ev, truncated = read_input()
    sid, name = ev.get("session_id"), ev.get("hook_event_name")
    if not (isinstance(sid, str) and SID_RE.match(sid)) or name not in EVENTS:
        return None
    root = data_root(ev)
    if root is None:
        return None
    try:
        append_event(root / "events", sid, event_line(ev, truncated))
    except Exception:
        pass  # the event is lost; note delivery does not depend on it
    if name not in NOTE_EVENTS or ev.get("agent_id") or truncated:
        return None  # notes go to the main thread only; truncated input cannot prove it is the main thread
    context = deliver(root, sid)
    if not context:
        return None
    # ASCII-only JSON: the console code page can never make the write fail after the notes were claimed.
    return json.dumps({"hookSpecificOutput": {"hookEventName": name, "additionalContext": context}})


def main() -> None:
    try:
        out = run()
        if out:
            sys.stdout.write(out)
            sys.stdout.flush()
    except BaseException:
        pass
    os._exit(0)  # never a non-zero status, even if interpreter shutdown fails


if __name__ == "__main__":
    main()
