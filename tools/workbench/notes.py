"""Owner notes: the workbench's only write.

A note is appended as one JSON line to the session's private inbox under
work/loop-memory/workbench/inbox/. The reporting hook delivers pending notes to
the main Claude thread at its next tool step. Writers and the hook share the
inbox lock (`inbox/<sid>.lock`, same protocol as `.claude/hooks/report_event.py`),
so the capacity check and the append are atomic. Notes over the byte limit, an
inbox that would exceed its cap, or a busy inbox are refused, so an accepted note
is never dropped.
"""
from __future__ import annotations

import json
import os
import secrets
import time
from datetime import datetime, timezone
from pathlib import Path

try:
    from . import paths
except ImportError:
    import paths  # type: ignore[no-redef]

NOTE_MAX_BYTES = 4000
INBOX_MAX_BYTES = 1024 * 1024
LOCK_WAIT = 2.0
LOCK_STALE = 2.0   # same as the hook


class NoteRejected(Exception):
    pass


def _acquire(lock: Path, wait: float):
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


def _release(fd, lock: Path) -> None:
    os.close(fd)
    try:
        lock.unlink()
    except OSError:
        pass


def append_note(data: Path, sid: str, text: str) -> str:
    if not paths.valid_sid(sid):
        raise NoteRejected("invalid session id")
    text = text.strip()
    if not text:
        raise NoteRejected("the note is empty")
    if len(text.encode("utf-8")) > NOTE_MAX_BYTES:
        raise NoteRejected(f"the note is longer than {NOTE_MAX_BYTES} bytes")
    now = datetime.now(timezone.utc)
    note_id = now.strftime("%Y%m%dT%H%M%SZ") + "-" + secrets.token_hex(4)
    line = (json.dumps({"id": note_id, "time": now.isoformat(timespec="seconds"), "text": text}, ensure_ascii=False) + "\n").encode("utf-8")
    inbox_dir = Path(data) / "inbox"
    inbox_dir.mkdir(parents=True, exist_ok=True)
    inbox, lock = inbox_dir / f"{sid}.jsonl", inbox_dir / f"{sid}.lock"
    fd = _acquire(lock, LOCK_WAIT)
    if fd is None:
        raise NoteRejected("the inbox is busy; try again")
    try:
        try:
            size = inbox.stat().st_size
        except FileNotFoundError:
            size = 0
        prefix = b""
        if size:
            with inbox.open("rb") as f:  # an interrupted earlier write: start this note on its own line
                f.seek(size - 1)
                prefix = b"" if f.read(1) == b"\n" else b"\n"
        if size + len(prefix) + len(line) > INBOX_MAX_BYTES:
            raise NoteRejected("this session's inbox is full")
        with inbox.open("ab") as f:
            f.write(prefix + line)
    finally:
        _release(fd, lock)
    return note_id
