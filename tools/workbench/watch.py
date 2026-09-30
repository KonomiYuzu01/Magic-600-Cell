"""Pure attention rules for the workbench; the caller supplies all data and the clock."""
from __future__ import annotations

import math

try:
    from . import sources
except ImportError:
    import sources  # type: ignore[no-redef]

WATCH_WINDOW = 24 * 3600
STALL_AFTER = 900
CODEX_EXPECTED = {"plan": 5400, "review": 5400, "implement": 7200}
WRAPPER_CAP_S = 7800
FAILED_RUNS = ("failed", "timed_out", "interrupted")


def _number(value) -> bool:
    if not isinstance(value, (int, float)) or isinstance(value, bool):
        return False
    try:
        return math.isfinite(value)
    except OverflowError:
        return False


def _optional_text(value) -> bool:
    return value is None or isinstance(value, str)


def _flag(kind, severity, target_kind, target_id, text, t) -> dict:
    return {"kind": kind, "severity": severity, "target_kind": target_kind,
            "target_id": target_id, "text": text, "t": float(t)}


def _session_flags(session, now) -> list[dict]:
    if not isinstance(session, sources.Session) or not isinstance(session.status, sources.Status) \
            or not isinstance(session.sid, str) or not session.sid:
        raise ValueError("invalid session")
    st = session.status
    if not isinstance(st.status, str) or not isinstance(st.waits, list) \
            or not all(isinstance(label, str) for label in st.waits) \
            or not all(t is None or _number(t) for t in (st.wait_since, st.failed_at, st.last_any)) \
            or not isinstance(st.subagents, dict) or not isinstance(st.open_tools, list):
        raise ValueError("invalid session status")
    if st.waits and st.wait_since is None:
        raise ValueError("wait has no time")
    for agent, sub in st.subagents.items():
        if not isinstance(agent, str) or not isinstance(sub, dict) or not isinstance(sub.get("status"), str):
            raise ValueError("invalid subagent")
    for tool in st.open_tools:
        if not isinstance(tool, dict) or not {"agent", "name", "t", "wrapper"}.issubset(tool) \
                or not _optional_text(tool["agent"]) or not _optional_text(tool["name"]) \
                or not _number(tool["t"]) or not isinstance(tool["wrapper"], bool):
            raise ValueError("invalid open tool")

    out = []
    if st.waits:
        text = f"waiting {sources.age_text(now - st.wait_since)}: {', '.join(sorted(set(st.waits)))}"
        out.append(_flag("owner_wait", 0, "session", session.sid, text, st.wait_since))
    if st.status != "unknown" and st.failed_at is not None and now - st.failed_at <= WATCH_WINDOW:
        text = f"turn failed {sources.age_text(now - st.failed_at)} ago"
        if st.status == "finished":
            text += "; session ended"
        out.append(_flag("failed_turn", 1, "session", session.sid, text, st.failed_at))
    if st.status == "running" and st.last_any is not None and STALL_AFTER < now - st.last_any <= WATCH_WINDOW:
        exempt = any(tool["wrapper"] and now - tool["t"] <= WRAPPER_CAP_S
                     and (tool["agent"] is None or st.subagents.get(tool["agent"], {}).get("status") == "running")
                     for tool in st.open_tools)
        if not exempt:
            text = f"no activity for {sources.age_text(now - st.last_any)}"
            if st.open_tools:
                oldest = min(st.open_tools, key=lambda tool: tool["t"])
                text += f"; open: {oldest['name'] or 'tool'}"
                if oldest["wrapper"] and now - oldest["t"] > WRAPPER_CAP_S:
                    text += f" (codex wrapper, {sources.age_text(now - oldest['t'])})"
            out.append(_flag("stalled", 2, "session", session.sid, text, st.last_any))
    return out


def _call_flags(call, now, failed_launches) -> list[dict]:
    if not isinstance(call, dict) or not isinstance(call.get("call_id"), str) or not call["call_id"] \
            or not isinstance(call.get("status"), str) or not _number(call.get("time")) \
            or not _optional_text(call.get("kind")):
        raise ValueError("invalid call")
    problems, exit_code = call.get("problems", []), call.get("acceptance_exit")
    if not isinstance(problems, list) or not all(isinstance(p, str) for p in problems) \
            or (exit_code is not None and (not isinstance(exit_code, int) or isinstance(exit_code, bool))):
        raise ValueError("invalid call result")
    acceptance_failed = call.get("kind") == "implement" and call["status"] == "finished" and exit_code not in (None, 0)
    if (call["status"] != "failed" and not acceptance_failed) or now - call["time"] > WATCH_WINDOW \
            or call["call_id"] in failed_launches:
        return []
    problem = problems[0] if problems else (f"acceptance failed (exit {exit_code})" if acceptance_failed else "failed")
    text = f"{call.get('kind') or 'call'} {call['call_id']}: {problem}"
    return [_flag("codex_failed", 1, "call", call["call_id"], text, call["time"])]


def _run_flags(run, now) -> list[dict]:
    if not isinstance(run, dict) or not all(isinstance(run.get(k), str) for k in ("rid", "kind", "run", "title", "status")) \
            or not run["rid"] or run["kind"] not in ("registry", "codex") \
            or not _optional_text(run.get("reason")) or not _optional_text(run.get("call_id")):
        raise ValueError("invalid run")
    exits = run.get("exits", [])
    if not isinstance(exits, list) or not all(code is None or (isinstance(code, int) and not isinstance(code, bool)) for code in exits):
        raise ValueError("invalid run exits")
    stamps = []
    for key in ("ended", "started", "created"):
        value = run.get(key)
        t = sources.ts(value)
        if value is not None and t is None:
            raise ValueError("invalid run time")
        stamps.append(t)
    status = run["status"]
    if status in FAILED_RUNS:
        t = next((t for t in stamps if t is not None), None)
        if t is None:
            raise ValueError("failed run has no time")
        if now - t > WATCH_WINDOW:
            return []
        if run["kind"] == "registry":
            text = f"{run['title']}: {status}"
            if run.get("reason"):
                text += f"; {run['reason']}"
        else:
            code = exits[-1] if exits else None
            detail = {2: "refused or invalid (exit 2)", 3: "timeout (exit 3)", 4: "acceptance failed (exit 4)"}.get(code)
            detail = detail or (f"exit {code}" if code is not None else status)
            text = f"Codex {run['run']}: {detail}"
            if run.get("call_id"):
                text += f" {run['call_id']}"
        return [_flag("run_failed", 1, "run", run["rid"], text, t)]
    if run["kind"] == "codex" and status == "running":
        expected = CODEX_EXPECTED.get(run["run"].removeprefix("codex-"))
        if expected is None:
            return []
        started = stamps[1]
        if started is None:
            raise ValueError("running launch has no start time")
        if now - started > expected:
            text = (f"Codex {run['run']} running for {sources.age_text(now - started)}, "
                    f"beyond the expected {sources.age_text(expected)}; Stop ends it")
            return [_flag("run_overdue", 2, "run", run["rid"], text, started)]
    return []


def flags(sessions, calls, runs, now) -> list[dict]:
    """Attention flags ordered by severity, newest first, then target id; count malformed items."""
    flags.skipped = 0
    if not _number(now):
        flags.skipped = 1
        return []
    groups = []
    for group in (sessions, calls, runs):
        if not isinstance(group, (list, tuple)):
            flags.skipped += 1
            group = ()
        groups.append(group)
    sessions, calls, runs = groups
    out, failed_launches = [], set()
    malformed = (AttributeError, KeyError, TypeError, ValueError, OverflowError, OSError)
    for run in runs:
        try:
            out.extend(_run_flags(run, now))
            if run["kind"] == "codex" and run["status"] in FAILED_RUNS and run.get("call_id"):
                failed_launches.add(run["call_id"])
        except malformed:
            flags.skipped += 1
    for session in sessions:
        try:
            out.extend(_session_flags(session, now))
        except malformed:
            flags.skipped += 1
    for call in calls:
        try:
            out.extend(_call_flags(call, now, failed_launches))
        except malformed:
            flags.skipped += 1
    return sorted(out, key=lambda flag: (flag["severity"], -flag["t"], flag["target_id"]))


flags.skipped = 0
