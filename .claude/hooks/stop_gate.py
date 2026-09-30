"""Bounded completion gate (Claude Code Stop hook).

Blocks a stop at most twice per session when critical paths changed and no
valid Codex review covers the current candidate. It never calls models,
installs tools, runs tests or changes Git, and it never blocks when
stop_hook_active is true. Anything it cannot decide in time is reported as
inconclusive and the stop is allowed; that never authorizes a commit.
"""
from __future__ import annotations

import fnmatch
import hashlib
import json
import os
import struct
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(os.environ.get("CLAUDE_PROJECT_DIR") or Path(__file__).resolve().parents[2])

MAX_CONTINUATIONS = 2
BUDGET_SECONDS = 4.0
STATE = ROOT / "work" / "loop-memory" / "state" / "stop_gate.json"
REVIEWS = ROOT / "work" / "reviews"
BLOCKING = ("blocker", "major")
# Kept identical to tools/repo_digest.py (tests enforce this). The gate never imports
# repository code, so a modified candidate cannot change how it is judged.
IDENTITY_EXCLUDES = ("work/reviews/", "work/loop-memory/", "docs/wiki/log.md")
# Inline copy of schemas/review-result.schema.json (tests enforce equality), so a
# modified candidate schema cannot weaken the gate.
_STR, _OPT_STR = {"type": "string"}, {"type": ["string", "null"]}
REVIEW_SCHEMA = {
    "type": "object", "additionalProperties": False, "required": ["verdict", "summary", "findings"],
    "properties": {
        "verdict": {"type": "string", "enum": ["pass", "findings", "inconclusive"]},
        "summary": _STR,
        "findings": {"type": "array", "items": {
            "type": "object", "additionalProperties": False,
            "required": ["id", "severity", "title", "detail", "evidence", "counterexample", "suggested_experiment", "verification_status"],
            "properties": {
                "id": _STR,
                "severity": {"type": "string", "enum": ["blocker", "major", "minor", "nit"]},
                "title": _STR,
                "detail": _STR,
                "evidence": {"type": "array", "items": {
                    "type": "object", "additionalProperties": False, "required": ["path", "line", "sha256"],
                    "properties": {"path": _STR, "line": {"type": ["integer", "null"]}, "sha256": _OPT_STR}}},
                "counterexample": _OPT_STR,
                "suggested_experiment": _OPT_STR,
                "verification_status": {"type": "string", "enum": ["verified", "unverified"]},
            }}},
    },
}
_TYPES = {"object": dict, "array": list, "string": str, "integer": int, "null": type(None)}


def _is_link(p: Path) -> bool:
    """True for a symlink or a Windows junction (realpath resolves both, on every supported Python)."""
    real = os.path.normcase(os.path.realpath(p))
    return p.is_symlink() or real != os.path.normcase(os.path.join(os.path.realpath(p.parent), p.name))


def _sha256(path: Path) -> str:
    data = path.read_bytes()
    if b"\0" not in data:
        data = data.replace(b"\r\n", b"\n")
    return hashlib.sha256(data).hexdigest()


def _git(args, timeout: float) -> str:
    return subprocess.run(["git", *args], cwd=ROOT, capture_output=True, check=True, timeout=timeout).stdout.decode("utf-8", "surrogateescape")


def source_identity(timeout: float) -> dict:
    parts = _git(["status", "--porcelain=v1", "-z", "--untracked-files=all"], timeout).split("\0")
    paths, i = [], 0
    while i < len(parts):
        entry = parts[i]
        i += 1
        if len(entry) < 4:
            continue
        paths.append(entry[3:])
        if "R" in entry[:2] or "C" in entry[:2]:
            paths.append(parts[i])
            i += 1
    paths = sorted(set(p for p in paths if not p.startswith(IDENTITY_EXCLUDES)))
    head = _git(["rev-parse", "HEAD"], timeout).strip()
    staged = {}
    if paths:
        wanted = set(paths)
        for rec in filter(None, _git(["ls-files", "--stage", "-z"], timeout).split("\0")):
            meta, rel = rec.split("\t", 1)
            if rel in wanted:
                staged.setdefault(rel, []).append(meta)
    items = []
    for rel in paths:
        p = ROOT / rel
        if any(_is_link(ROOT.joinpath(*Path(rel).parts[:n])) for n in range(1, len(Path(rel).parts))):
            # Below a linked directory: never read through the link; the link itself is its own entry.
            items.append([rel, "under-link", None, sorted(staged.get(rel, []))])
        elif _is_link(p):  # the link itself, never its referent
            items.append([rel, "link", os.readlink(p), sorted(staged.get(rel, []))])
        elif p.is_file():
            items.append([rel, "x" if os.access(p, os.X_OK) else "f", _sha256(p), sorted(staged.get(rel, []))])
        else:
            items.append([rel, "deleted", None, sorted(staged.get(rel, []))])
    blob = json.dumps({"head": head, "changes": items}, sort_keys=True).encode()
    return {"head": head, "paths": [i[0] for i in items], "digest": hashlib.sha256(blob).hexdigest()}


def _conforms(data, schema: dict) -> bool:
    types = schema["type"] if isinstance(schema["type"], list) else [schema["type"]]
    if not any(isinstance(data, _TYPES[t]) and not isinstance(data, bool) for t in types):
        return False
    if "enum" in schema and data not in schema["enum"]:
        return False
    if isinstance(data, dict):
        props = schema["properties"]
        if set(data) - set(props) or set(schema["required"]) - set(data):
            return False
        return all(_conforms(data[k], props[k]) for k in data)
    if isinstance(data, list):
        return all(_conforms(item, schema["items"]) for item in data)
    return True


def well_formed(result) -> bool:
    """Fully re-validate the review result against the inline schema instead of trusting metadata."""
    return _conforms(result, REVIEW_SCHEMA)


class BadState(Exception):
    """Local gate state has the wrong shape."""


def emit(payload: dict | None) -> int:
    if payload:
        print(json.dumps(payload))
    return 0


def critical(paths: list[str]) -> list[str]:
    patterns = json.loads((ROOT / "tools" / "agents" / "critical_paths.json").read_text(encoding="utf-8"))["patterns"]
    if not isinstance(patterns, list) or not all(isinstance(p, str) for p in patterns):
        raise ValueError("critical_paths.json patterns must be a list of strings")
    return [p for p in paths if any(fnmatch.fnmatch(p, pat) for pat in patterns)]


def review_covers(identity: str, deadline: float) -> bool:
    """True when a valid review of this exact candidate has no unresolved blocking finding."""
    if not REVIEWS.is_dir():
        return False
    for meta_path in sorted(REVIEWS.glob("*/meta.json"), reverse=True):
        if time.monotonic() > deadline:
            return False
        try:
            meta = json.loads(meta_path.read_text(encoding="utf-8"))
            if not isinstance(meta, dict):
                continue
            if meta.get("kind") != "review" or not meta.get("valid"):
                continue
            if (meta.get("source_identity") or {}).get("digest") != identity:
                continue
            result = json.loads((meta_path.parent / "review.json").read_text(encoding="utf-8"))
        except (OSError, ValueError, AttributeError):
            continue
        if not well_formed(result) or result["verdict"] not in ("pass", "findings"):
            continue
        blocking = {f["id"] for f in result["findings"] if f["severity"] in BLOCKING}
        if not blocking:
            return True
        try:
            answers = json.loads((meta_path.parent / "dispositions.json").read_text(encoding="utf-8"))
        except (OSError, ValueError):
            answers = {}
        if not isinstance(answers, dict):
            continue  # malformed dispositions resolve nothing
        # An adopted fix changes the candidate and needs a new review; only an
        # evidence-backed rejection resolves a blocking finding in place.
        if all(answers.get(fid) == "reject_with_evidence" for fid in blocking):
            return True
    return False


def take_continuation(session_id: str) -> int:
    """Atomically count one automatic continuation for this session; return the new count."""
    STATE.parent.mkdir(parents=True, exist_ok=True)
    lock = STATE.with_suffix(".lock")
    for _ in range(50):
        try:
            fd = os.open(lock, os.O_CREAT | os.O_EXCL | os.O_WRONLY)
            break
        except FileExistsError:
            try:
                if time.time() - lock.stat().st_mtime > 10:
                    lock.unlink(missing_ok=True)
            except FileNotFoundError:
                continue  # released between open and stat: retry at once
            time.sleep(0.02)
    else:
        return MAX_CONTINUATIONS + 1
    try:
        try:
            state = json.loads(STATE.read_text(encoding="utf-8"))
        except FileNotFoundError:
            state = {}
        except ValueError as exc:
            raise BadState("unreadable continuation counter") from exc
        previous = state.get(session_id, 0) if isinstance(state, dict) else None
        if not isinstance(previous, int) or isinstance(previous, bool) or previous < 0:
            raise BadState("malformed continuation counter")
        count = previous + 1
        state[session_id] = count
        tmp = STATE.with_suffix(".tmp")
        tmp.write_text(json.dumps(state), encoding="utf-8")
        os.replace(tmp, STATE)
        return count
    finally:
        os.close(fd)
        lock.unlink(missing_ok=True)


def main() -> int:
    started = time.monotonic()
    deadline = started + BUDGET_SECONDS
    try:
        event = json.loads(sys.stdin.read() or "{}")
    except ValueError:
        return emit({"systemMessage": "stop gate: inconclusive (unreadable hook input)"})
    if not isinstance(event, dict):
        return emit({"systemMessage": "stop gate: inconclusive (hook input is not an object)"})
    if event.get("stop_hook_active"):
        return emit(None)
    if struct.calcsize("P") * 8 != 64 or sys.version_info < (3, 9):
        return emit({"systemMessage": "stop gate: inconclusive (hook interpreter is not 64-bit CPython 3.9+)"})
    try:
        identity = source_identity(timeout=max(0.5, deadline - time.monotonic()))
    except Exception:  # git missing, timeout or unreadable tree
        return emit({"systemMessage": "stop gate: inconclusive (could not compute the candidate identity)"})
    try:
        changed = critical(identity["paths"])
    except Exception:  # missing or malformed critical-path configuration
        return emit({"systemMessage": "stop gate: inconclusive (critical-path list unreadable); do not commit critical changes without a valid review"})
    if not changed:
        return emit(None)
    try:
        covered = review_covers(identity["digest"], deadline)
    except Exception:  # unexpected record shape: never crash, never block
        return emit({"systemMessage": "stop gate: inconclusive (review records unreadable); do not commit critical changes without a valid review"})
    if covered:
        return emit(None)
    if time.monotonic() > deadline:
        return emit({"systemMessage": "stop gate: inconclusive (time budget exceeded); do not commit critical changes without a valid review"})
    try:
        count = take_continuation(str(event.get("session_id", "unknown")))
    except (OSError, BadState):
        return emit({"systemMessage": "stop gate: inconclusive (continuation counter unavailable or malformed); do not commit critical changes without a valid review"})
    if count > MAX_CONTINUATIONS:
        return emit({"systemMessage": "stop gate: continuation budget used; stopping as inconclusive. Critical changes stay uncommitted until a valid Codex review of the current candidate or an explicit owner exception."})
    listed = ", ".join(changed[:8]) + (" ..." if len(changed) > 8 else "")
    return emit({"decision": "block", "reason": (
        f"Critical paths changed without a valid Codex review of the current candidate ({listed}). "
        "Run python tools/agents/codex_review.py --kind review --packet <file> and answer every finding, "
        "or report the work as unreviewed and stop. "
        f"Automatic continuation {count} of {MAX_CONTINUATIONS}.")})


if __name__ == "__main__":
    sys.exit(main())
