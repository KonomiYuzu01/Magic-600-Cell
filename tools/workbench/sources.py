"""Read-only data sources for the development workbench (standard library only, no Qt).

Nothing here writes. Transcripts, events, briefs, notes, reviews and ledgers are
read in place. Session status is reduced incrementally over each session's full
history (transcript, subagent transcripts and hook events), so an unresolved wait
never falls out of a read window. Every reader has a byte budget, every JSONL
reader skips and counts malformed lines, and one unreadable session never stops
the others from being listed.
"""
from __future__ import annotations

import hashlib
import json
import re
import time
from collections import deque
from dataclasses import dataclass, field
from datetime import datetime, timezone
from pathlib import Path

try:
    from . import paths
except ImportError:  # run as a script from tools/workbench
    import paths  # type: ignore[no-redef]

IDENTITY_BUDGET = 1024 * 1024
DOC_MAX = 2 * 1024 * 1024           # JSON documents: briefs, meta, review results, progress
LEDGER_MAX = 64 * 1024 * 1024       # per ledger file; beyond it the budget is reported incomplete
HISTORY_PER_REFRESH = 32 * 1024 * 1024
RECENT_RECORDS = 1500
SILENT_AFTER = 600          # seconds without output before an open tool call is flagged
CODEX_STALE_AFTER = 10800   # the wrapper's maximum --timeout (7200) plus its acceptance limit (1800) plus setup and finalization (1800)
LINK_WINDOW = 60            # seconds between a Bash command and a Codex call id
QUESTION_TOOLS = ("AskUserQuestion", "ExitPlanMode")
CALL_OUTPUT_RE = re.compile(r"\b(plan|review) (\d{8}T\d{6}Z-[0-9a-f]{8}):")
ID_RE = re.compile(r"\d{8}T\d{6}Z-[0-9a-f]{8}")
WRAPPER_RE = re.compile(r"(?:python3?|py|[A-Za-z0-9._/\\:-]*[\\/]python(?:\.exe)?)[ \t]+(?:\./)?tools[\\/]agents[\\/]codex_review\.py(?:[ \t]+[A-Za-z0-9._/\\:=-]+)*")


class _Oversized:
    def __repr__(self):
        return "OVERSIZED"


OVERSIZED = _Oversized()  # read_json result for a document over its byte budget


# ----------------------------------------------------------------- reading

def parse_lines(data: bytes, leading_fragment: bool) -> tuple[list, int]:
    """Complete JSON-object lines of `data`; a leading fragment (read started mid-line) and a
    trailing partial line (still being written) are not malformed and are skipped silently."""
    lines = data.split(b"\n")
    if leading_fragment:
        lines = lines[1:]
    if lines and not data.endswith(b"\n"):
        lines = lines[:-1]
    records, malformed = [], 0
    for line in lines:
        line = line.strip()
        if not line:
            continue
        try:
            rec = json.loads(line.decode("utf-8"))
        except (ValueError, UnicodeDecodeError):
            malformed += 1
            continue
        if isinstance(rec, dict):
            records.append(rec)
        else:
            malformed += 1
    return records, malformed


def read_head(path: Path, budget: int = IDENTITY_BUDGET) -> tuple[list, int]:
    try:
        with path.open("rb") as f:
            data = f.read(budget)
    except OSError:
        return [], 0
    return parse_lines(data, False)


def read_json(path: Path, limit: int = DOC_MAX):
    """A JSON document, None when missing or invalid, OVERSIZED when over `limit` bytes."""
    try:
        with path.open("rb") as f:
            data = f.read(limit + 1)
    except OSError:
        return None
    if len(data) > limit:
        return OVERSIZED
    try:
        return json.loads(data.decode("utf-8"))
    except (ValueError, UnicodeDecodeError):
        return None


class Tail:
    """Incremental JSONL follower from the start of a file: returns records appended since the
    last poll. A record longer than MAX_RECORD is skipped (counted in `oversized`)."""

    MAX_READ = 8 * 1024 * 1024
    MAX_RECORD = 16 * 1024 * 1024

    def __init__(self, path: Path):
        self.path = Path(path)
        self.offset = 0
        self.size = 0
        self.pending = b""
        self.skipping = False
        self.malformed = 0
        self.oversized = 0
        self.oversized_last = False   # the newest thing in the stream is an unreadable record

    @property
    def caught_up(self) -> bool:
        return self.offset >= self.size

    def poll(self, max_read: int | None = None) -> list:
        try:
            self.size = self.path.stat().st_size
        except OSError:
            return []
        if self.size < self.offset:  # truncated or replaced: start again
            self.__init__(self.path)
            self.size = self.path.stat().st_size if self.path.exists() else 0
        if self.size == self.offset:
            return []
        try:
            with self.path.open("rb") as f:
                f.seek(self.offset)
                data = f.read(min(self.size - self.offset, max_read or self.MAX_READ))
        except OSError:
            return []
        self.offset += len(data)
        if self.skipping:
            cut = data.find(b"\n")
            if cut < 0:
                return []
            data, self.skipping = data[cut + 1:], False
        data = self.pending + data
        cut = data.rfind(b"\n") + 1
        self.pending = data[cut:]
        records = []
        for line in data[:cut].split(b"\n"):
            if not line.strip():
                continue
            if len(line) > self.MAX_RECORD:
                self.oversized += 1
                self.oversized_last = True
                continue
            parsed, bad = parse_lines(line + b"\n", False)
            records += parsed
            self.malformed += bad
            self.oversized_last = False
        if len(self.pending) > self.MAX_RECORD:  # a record still growing past the limit
            self.pending, self.skipping = b"", True
            self.oversized += 1
            self.oversized_last = True
        return records


# ----------------------------------------------------------------- helpers

def digest(obj) -> str:
    """Key for matching a tool call across hook events and the transcript (same in the hook)."""
    blob = json.dumps(obj, sort_keys=True, separators=(",", ":"), ensure_ascii=False)
    return hashlib.sha256(blob.encode("utf-8", "surrogatepass")).hexdigest()[:16]


def ts(value) -> float | None:
    if not isinstance(value, str):
        return None
    try:
        dt = datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError:
        return None
    if dt.tzinfo is None:
        dt = dt.replace(tzinfo=timezone.utc)
    return dt.timestamp()


def call_time(call_id: str) -> float | None:
    try:
        return datetime.strptime(call_id[:16], "%Y%m%dT%H%M%SZ").replace(tzinfo=timezone.utc).timestamp()
    except ValueError:
        return None


def content_items(rec: dict) -> list:
    msg = rec.get("message")
    content = msg.get("content") if isinstance(msg, dict) else None
    if isinstance(content, str):
        return [{"type": "text", "text": content}]
    return [c for c in content if isinstance(c, dict)] if isinstance(content, list) else []


def text_of(item: dict) -> str:
    return item["text"] if isinstance(item.get("text"), str) else ""


def result_text(item: dict) -> str:
    content = item.get("content")
    if isinstance(content, str):
        return content
    if isinstance(content, list):
        return "\n".join(c["text"] for c in content if isinstance(c, dict) and isinstance(c.get("text"), str))
    return ""


def str_or_none(value):
    return value if isinstance(value, str) else None


def tool_summary(name, tool_input) -> str:
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


def age_text(seconds: float) -> str:
    if seconds < 3600:
        return f"{int(seconds // 60)} min"
    if seconds < 86400:
        return f"{seconds / 3600:.1f} h"
    return f"{seconds / 86400:.1f} d"


# ----------------------------------------------------------------- signals and status

@dataclass(order=True)
class Signal:
    t: float
    seq: int
    kind: str = field(compare=False)
    agent: str | None = field(compare=False, default=None)
    data: dict = field(compare=False, default_factory=dict)
    rank: int = field(compare=False, default=0)   # source: 0 transcript, 1 subagent transcript, 2 hook events


def order(s: Signal) -> tuple:
    """Time first; at equal times the transcript precedes subagents and hook events, then file order.
    The key never depends on when a batch was read, so incremental and one-shot reduction agree."""
    return (s.t, s.rank, s.seq)


def transcript_signals(records: list, start_seq: int = 0, agent: str | None = None) -> tuple[list, int]:
    """Signals from transcript records; `agent` attributes a subagent transcript. Records with
    fields of the wrong type are skipped and counted, never raised."""
    out, seq, bad = [], start_seq, 0
    for rec in records:
        try:
            t = ts(rec.get("timestamp"))
            if t is None:
                continue
            who = agent or ((str_or_none(rec.get("agentId")) or "sidechain") if rec.get("isSidechain") else None)
            kind = rec.get("type")
            items = content_items(rec)
            if kind == "user":
                results = [c for c in items if c.get("type") == "tool_result"]
                for c in results:
                    seq += 1
                    out.append(Signal(t, seq, "tool_result", who, {"id": str_or_none(c.get("tool_use_id"))}))
                if not results and not rec.get("isMeta"):
                    seq += 1
                    out.append(Signal(t, seq, "activity", who))
            elif kind == "assistant":
                seq += 1
                out.append(Signal(t, seq, "activity", who))
                uses = [c for c in items if c.get("type") == "tool_use"]
                for c in uses:
                    inp = c.get("input")
                    command = inp.get("command") if isinstance(inp, dict) else None
                    wrapper = (c.get("name") == "Bash" and isinstance(inp, dict)
                               and inp.get("run_in_background") is not True and isinstance(command, str)
                               and WRAPPER_RE.fullmatch(command.strip()) is not None)
                    seq += 1
                    out.append(Signal(t, seq, "tool_use", who, {"id": str_or_none(c.get("id")), "name": str_or_none(c.get("name")),
                                                                "key": digest(inp), "wrapper": wrapper}))
                msg = rec.get("message") if isinstance(rec.get("message"), dict) else {}
                if rec.get("isApiErrorMessage"):
                    seq += 1
                    out.append(Signal(t, seq, "turn_fail", who))
                elif msg.get("stop_reason") == "end_turn" and not uses:
                    seq += 1
                    out.append(Signal(t, seq, "turn_end", who))
        except (TypeError, AttributeError, ValueError):
            bad += 1
    rank = 1 if agent else 0
    for sig in out:
        sig.rank = rank
    return out, bad


def event_signals(events: list, start_seq: int = 0) -> list:
    out, seq = [], start_seq
    for ev in events:
        t = ts(ev.get("t"))
        if t is None:
            continue
        name, agent = ev.get("e"), str_or_none(ev.get("agent_id")) or None
        seq += 1
        if name in ("PostToolUse", "PostToolUseFailure"):
            out.append(Signal(t, seq, "post", agent, {"tool": str_or_none(ev.get("tool")), "key": str_or_none(ev.get("key"))}))
        elif name == "PermissionRequest":
            out.append(Signal(t, seq, "perm", agent, {"tool": str_or_none(ev.get("tool")), "key": str_or_none(ev.get("key")),
                                                    "summary": str_or_none(ev.get("summary"))}))
        elif name == "Notification" and "elicitation" in str(ev.get("ntype", "")):
            out.append(Signal(t, seq, "elicit", agent))
        elif name == "Stop":
            out.append(Signal(t, seq, "turn_end", None))
        elif name == "StopFailure":
            out.append(Signal(t, seq, "turn_fail", None))
        elif name == "SubagentStart" and agent:
            out.append(Signal(t, seq, "sub_start", agent, {"type": str_or_none(ev.get("agent_type"))}))
        elif name == "SubagentStop" and agent:
            out.append(Signal(t, seq, "sub_stop", agent))
        elif name == "SessionEnd":
            out.append(Signal(t, seq, "session_end", None, {"reason": str_or_none(ev.get("reason"))}))
    for sig in out:
        sig.rank = 2
    return out


@dataclass
class Status:
    status: str = "unknown"       # running | waiting | finished | failed | unknown
    detail: str = ""
    last_activity: float | None = None
    subagents: dict = field(default_factory=dict)
    waits: list = field(default_factory=list)
    wait_since: float | None = None
    failed_at: float | None = None
    last_any: float | None = None
    open_tools: list[dict] = field(default_factory=list)
    wait_items: list = field(default_factory=list)


class Reducer:
    """Per-agent status over a session's whole history, fed in time order. Waits clear only through
    their matching resolution; resumed or retried sessions become running again."""

    def __init__(self):
        self.main = "unknown"
        self.reason = None
        self.after_failure = False
        self.waits: dict = {}
        self.open_tools: dict = {}
        self.subagents: dict = {}
        self.last = None
        self.failed_at = None
        self.last_any = None

    def _clear_waits(self, agent):
        for k in [k for k, w in self.waits.items() if w["agent"] == agent]:
            del self.waits[k]

    def _sub(self, agent, t):
        return self.subagents.setdefault(agent, {"type": None, "status": "running", "since": t})

    def _clear_tools(self, agent):
        for k in [k for k, tool in self.open_tools.items() if tool["agent"] == agent]:
            del self.open_tools[k]

    def feed(self, s: Signal) -> None:
        self.last_any = s.t if self.last_any is None else max(self.last_any, s.t)
        if s.kind in ("activity", "tool_use", "tool_result", "post"):
            if s.agent is None:
                if self.main in ("idle", "failed", "ended", "unknown"):
                    self.main = "running"
                    self.failed_at = None
                self.last = s.t
            else:
                sub = self._sub(s.agent, s.t)
                if sub["status"] not in ("finished",):
                    sub["status"] = "running"
        if s.kind == "tool_use":
            self.open_tools[s.data.get("id")] = {"agent": s.agent, "name": s.data.get("name"), "key": s.data.get("key"),
                                               "t": s.t, "wrapper": s.data.get("wrapper") is True}
            if s.data.get("name") in QUESTION_TOOLS:
                self.waits[("q", s.data.get("id"))] = {"agent": s.agent, "t": s.t, "label": f"question ({s.data.get('name')})",
                                                    "tool": s.data.get("name"), "summary": ""}
        elif s.kind == "tool_result":
            use = self.open_tools.pop(s.data.get("id"), None)
            self.waits.pop(("q", s.data.get("id")), None)
            if use:
                self.waits.pop(("perm", use["agent"], use["name"], use["key"]), None)
        elif s.kind == "post":
            self.waits.pop(("perm", s.agent, s.data.get("tool"), s.data.get("key")), None)
            self.waits.pop(("elicit", s.agent), None)
        elif s.kind == "perm":
            self.waits[("perm", s.agent, s.data.get("tool"), s.data.get("key"))] = {
                "agent": s.agent, "t": s.t, "label": f"permission ({s.data.get('tool')})",
                "tool": s.data.get("tool"), "summary": s.data.get("summary") or ""}
        elif s.kind == "elicit":
            self.waits[("elicit", s.agent)] = {"agent": s.agent, "t": s.t, "label": "input requested", "tool": None, "summary": ""}
        elif s.kind == "turn_end":
            if s.agent is None:
                self.main, self.last = "idle", s.t
                self._clear_waits(None)  # a turn cannot end while one of its dialogs is open
            else:
                self._sub(s.agent, s.t)["status"] = "finished"
                self._clear_waits(s.agent)
            self._clear_tools(s.agent)
        elif s.kind == "turn_fail":
            if s.agent is None:
                self.main, self.last = "failed", s.t
                self.failed_at = s.t
                self._clear_waits(None)
            else:
                self._sub(s.agent, s.t)["status"] = "failed"
                self._clear_waits(s.agent)
            self._clear_tools(s.agent)
        elif s.kind == "sub_start":
            sub = self._sub(s.agent, s.t)
            sub.update(type=s.data.get("type") or sub.get("type"), status="running")
        elif s.kind == "sub_stop":
            self._sub(s.agent, s.t)["status"] = "finished"
            self._clear_waits(s.agent)
            self._clear_tools(s.agent)
        elif s.kind == "session_end":
            self.after_failure = self.main == "failed"
            self.main, self.reason, self.last = "ended", s.data.get("reason"), s.t
            self.waits.clear()
            self.open_tools.clear()
            for sub in self.subagents.values():
                if sub["status"] == "running":
                    sub["status"] = "finished"

    def snapshot(self, now: float | None = None) -> Status:
        now = time.time() if now is None else now
        st = Status(last_activity=self.last, subagents={k: dict(v) for k, v in self.subagents.items()},
                    waits=[w["label"] for w in self.waits.values()],
                    wait_since=min((w["t"] for w in self.waits.values()), default=None),
                    failed_at=self.failed_at, last_any=self.last_any,
                    open_tools=[{k: tool[k] for k in ("agent", "name", "t", "wrapper")}
                                for tool in sorted(self.open_tools.values(), key=lambda tool: tool["t"])],
                    wait_items=[{"key": ":".join(str(part) for part in
                                                (key[:1] + key[2:] if key[0] == "perm" else key[:1] if key[0] == "elicit" else key)),
                                 **wait} for key, wait in sorted(self.waits.items(), key=lambda pair: pair[1]["t"])])
        if self.waits:
            st.status, st.detail = "waiting", ", ".join(sorted(set(st.waits)))
        elif self.main == "running":
            st.status = "running"
            if self.last is not None and now - self.last > SILENT_AFTER and any(v["agent"] is None for v in self.open_tools.values()):
                st.detail = f"no output for {age_text(now - self.last)}"
        elif self.main == "idle":
            st.status = "waiting"
            st.detail = "turn ended" + (f"; idle {age_text(now - self.last)}" if self.last is not None else "")
        elif self.main == "failed":
            st.status, st.detail = "failed", "turn failed"
        elif self.main == "ended":
            st.status = "finished"
            st.detail = f"session ended ({self.reason or 'unknown reason'})" + (" after failure" if self.after_failure else "")
        return st


def reduce_status(signals: list, now: float | None = None) -> Status:
    r = Reducer()
    for s in sorted(signals, key=order):
        r.feed(s)
    return r.snapshot(now)


# ----------------------------------------------------------------- sessions

class SessionState:
    """Incremental state of one session, kept by the caller between refreshes."""

    def __init__(self, transcript: Path, events: Path, identity: tuple):
        self.identity = identity
        self.tail = Tail(transcript)
        self.events = Tail(events)
        self.subs: dict = {}
        self.reducer = Reducer()
        self.seq = 0
        self.bad = 0
        self.capped = False
        self.title = ""
        self.branch = ""
        self.recent: deque = deque(maxlen=RECENT_RECORDS)
        self.todos = None
        self.tasks: list = []
        self.files: dict = {}
        self.call_links: dict = {}
        self.codex_uses: deque = deque(maxlen=50)
        self.receipts: dict = {}
        self.answers: dict = {}
        self.pending: list = []      # signals held back until every lagging source has caught up to them
        self.frontier: dict = {}     # id(tail) -> newest timestamp read from that source

    def caught_up(self) -> bool:
        return self.tail.caught_up and self.events.caught_up and all(t.caught_up for t in self.subs.values())

    def unreadable_newest(self) -> bool:
        """The newest record of the transcript or of a subagent transcript could not be read."""
        return self.tail.oversized_last or any(t.oversized_last for t in self.subs.values())

    def _read(self, tail: Tail, budget: int, parse) -> int:
        """Read `tail` until it is caught up or `budget` bytes are spent; returns the bytes read.
        A record that spans a read boundary simply continues on the next poll."""
        spent = 0
        while spent < budget:
            before = tail.offset
            recs = tail.poll()
            spent += tail.offset - before
            if recs:
                parse(recs)
                stamps = [t for t in (ts(r.get("timestamp") or r.get("t")) for r in recs) if t is not None]
                if stamps:
                    self.frontier[id(tail)] = max(self.frontier.get(id(tail), float("-inf")), max(stamps))
            if tail.caught_up or tail.offset == before:
                break
        return spent

    def update(self, subagent_dir: Path) -> None:
        """Read what is new in every source and feed the reducer in global time order. While a
        source is still catching up on its history, signals newer than the newest record read
        from it are held back, so an older record read later can never overtake a newer one."""
        new: list = []

        def transcript(recs, agent=None):
            sigs, bad = transcript_signals(recs, self.seq, agent=agent)
            self.seq += len(sigs) + 1
            self.bad += bad
            new.extend(sigs)
            if agent is None:
                self._extract(recs)

        def events(recs):
            self.capped = self.capped or any(e.get("e") == "cap_reached" for e in recs)
            sigs = event_signals(recs, self.seq)
            self.seq += len(sigs) + 1
            new.extend(sigs)

        budget = HISTORY_PER_REFRESH
        budget -= self._read(self.tail, budget, transcript)
        budget -= self._read(self.events, max(budget, Tail.MAX_READ), events)
        for f in sorted(subagent_dir.glob("agent-*.jsonl")):
            agent = f.stem[len("agent-"):]
            tail = self.subs.setdefault(agent, Tail(f))
            entry = self.reducer.subagents.setdefault(agent, {"type": None, "status": "unknown", "since": None})
            entry["transcript"] = str(f)
            budget -= self._read(tail, max(budget, Tail.MAX_READ), lambda recs, a=agent: transcript(recs, a))
        self.pending += new
        lagging = [self.frontier.get(id(t), float("-inf"))
                   for t in (self.tail, self.events, *self.subs.values()) if not t.caught_up]
        limit = min(lagging) if lagging else float("inf")
        # Strictly older than the watermark: records sharing the frontier's timestamp may still be unread.
        ready = sorted((s for s in self.pending if s.t < limit), key=order)
        self.pending = [s for s in self.pending if s.t >= limit]
        for s in ready:
            self.reducer.feed(s)

    def _extract(self, records: list) -> None:
        for rec in records:
            try:
                self.recent.append(rec)
                t = ts(rec.get("timestamp")) or 0.0
                if rec.get("type") == "custom-title" and isinstance(rec.get("customTitle"), str):
                    self.title = rec["customTitle"]
                if isinstance(rec.get("gitBranch"), str):
                    self.branch = rec["gitBranch"]
                items = content_items(rec)
                if rec.get("type") == "assistant":
                    for c in items:
                        if c.get("type") == "text":
                            for nid in ID_RE.findall(text_of(c)):
                                self.answers.setdefault(nid, t)
                        elif c.get("type") == "tool_use":
                            self._tool_use(c, t)
                else:
                    blob = json.dumps(rec, ensure_ascii=False)
                    for nid in ID_RE.findall(blob):
                        self.receipts.setdefault(nid, t)
                    for c in items:
                        if c.get("type") == "tool_result":
                            for m in CALL_OUTPUT_RE.finditer(result_text(c)):
                                self.call_links[m.group(2)] = "call id"
            except (TypeError, AttributeError, ValueError):
                self.bad += 1

    def _tool_use(self, c: dict, t: float) -> None:
        name, inp = c.get("name"), c.get("input")
        if not isinstance(inp, dict):
            return
        if name == "TodoWrite" and isinstance(inp.get("todos"), list):
            self.todos = [{"content": str(x.get("content", "")), "status": str(x.get("status", ""))}
                          for x in inp["todos"] if isinstance(x, dict)]
        elif name == "TaskCreate":
            self.tasks.append({"content": str(inp.get("subject") or inp.get("description") or ""), "status": "pending"})
        elif name == "TaskUpdate":
            try:
                idx = int(str(inp.get("taskId", "")).lstrip("#")) - 1
            except ValueError:
                return
            if 0 <= idx < len(self.tasks) and isinstance(inp.get("status"), str):
                self.tasks[idx]["status"] = inp["status"]
        elif name == "Bash" and "codex_review.py" in str(inp.get("command", "")):
            self.codex_uses.append(t)
        for key in ("file_path", "notebook_path"):
            if isinstance(inp.get(key), str) and len(self.files) < 200:
                self.files.setdefault(inp[key], None)

    def task_list(self) -> list:
        return self.todos if self.todos is not None else self.tasks


@dataclass
class Session:
    sid: str
    transcript: Path
    project_dir: Path
    cwd: str
    checkout: Path
    title: str = ""
    branch: str = ""
    status: Status = field(default_factory=Status)
    malformed: int = 0
    capped: bool = False
    events_path: Path | None = None
    state: SessionState | None = None


def candidate_dirs(projects: Path, checkouts: list) -> list[Path]:
    slugs = [paths.slug(c) for c in checkouts]
    try:
        entries = [d for d in projects.iterdir() if d.is_dir()]
    except OSError:
        return []
    return sorted(d for d in entries if any(d.name == s or d.name.startswith(s + "-") for s in slugs))


CWD_RE = re.compile(rb'"cwd"\s*:\s*"((?:[^"\\]|\\.){1,4096})"')


def identity(transcript: Path) -> tuple[str | None, str, int]:
    """(cwd, title, malformed) from the start of a transcript, within a fixed budget. A first
    record larger than the budget still names its cwd near its start."""
    records, malformed = read_head(transcript)
    cwd, title = None, ""
    if not records:
        try:
            with transcript.open("rb") as f:
                m = CWD_RE.search(f.read(64 * 1024))
            cwd = json.loads(b'"' + m.group(1) + b'"') if m else None
        except (OSError, ValueError):
            cwd = None
    for rec in records:
        if cwd is None and isinstance(rec.get("cwd"), str):
            cwd = rec["cwd"]
        if not title and rec.get("type") == "user" and not rec.get("isMeta"):
            texts = [text_of(c) for c in content_items(rec) if c.get("type") == "text"]
            if texts and texts[0].strip():
                title = texts[0].strip().splitlines()[0][:80]
    return cwd, title, malformed


def discover_sessions(main: Path, projects: Path | None = None, now: float | None = None,
                      cache: dict | None = None) -> list[Session]:
    """Every Claude Code session of this repository. `cache` (kept by the caller between
    refreshes) holds each session's incremental state."""
    projects = projects or paths.claude_projects_dir()
    roots = paths.checkouts(main)
    data = paths.data_root(main)
    cache = {} if cache is None else cache
    now = time.time() if now is None else now
    sessions = []
    for d in candidate_dirs(projects, roots):
        for transcript in sorted(d.glob("*.jsonl")):
            sid = transcript.stem
            if not paths.valid_sid(sid):
                continue
            key = str(transcript)
            try:
                if key not in cache:
                    cwd, title, bad = identity(transcript)
                    if not cwd:  # no record near the start names it: only an exact project-name match counts
                        cwd = next((str(r) for r in roots if d.name == paths.slug(r)), None)
                    cache[key] = SessionState(transcript, data / "events" / f"{sid}.jsonl", (cwd, title, bad))
                state = cache[key]
                cwd, title, head_bad = state.identity
                if not cwd or not paths.inside(Path(cwd), roots):
                    continue  # an unrelated project whose name shares a prefix, or no evidence of membership
                checkout = max((r for r in roots if paths.inside(Path(cwd), [r])), key=lambda r: len(str(r)))
                state.update(d / sid / "subagents")
                s = Session(sid=sid, transcript=transcript, project_dir=d, cwd=cwd, checkout=checkout,
                            title=state.title or title, branch=state.branch, capped=state.capped,
                            events_path=data / "events" / f"{sid}.jsonl", state=state,
                            malformed=head_bad + state.tail.malformed + state.events.malformed + state.bad
                            + sum(t.malformed for t in state.subs.values()))
                s.status = state.reducer.snapshot(now)
                if not state.caught_up():
                    done = state.tail.offset / max(state.tail.size, 1)
                    s.status.status, s.status.detail = "unknown", f"reading history ({done:.0%})"
                elif state.unreadable_newest():
                    # The newest transcript record cannot be read, so newer activity cannot be ruled out.
                    s.status.status, s.status.detail = "unknown", "record exceeds read budget"
            except Exception as exc:  # one unreadable session never hides the others
                s = Session(sid=sid, transcript=transcript, project_dir=d, cwd="", checkout=Path(main))
                s.status = Status(status="unknown", detail=f"unreadable ({type(exc).__name__})")
            sessions.append(s)
    sessions.sort(key=lambda s: s.status.last_activity or 0, reverse=True)
    return sessions


# ----------------------------------------------------------------- summaries

def brief(data: Path, sid: str) -> dict | None:
    if not paths.valid_sid(sid):
        return None
    b = read_json(data / "briefs" / f"{sid}.json", limit=64 * 1024)
    return b if isinstance(b, dict) else None


def task_list(records: list) -> list[dict]:
    """The latest task list in `records` (used for records outside a SessionState)."""
    state = SessionState(Path("."), Path("."), (None, "", 0))
    state._extract(records)
    return state.task_list()


def summary(data: Path, s: Session) -> dict:
    b = brief(data, s.sid)
    if b:
        return {"source": "brief", **{k: str(b.get(k, "")) for k in ("goal", "step", "doing", "why", "waiting_for", "updated")}}
    tasks = s.state.task_list() if s.state else []
    current = next((t["content"] for t in tasks if t["status"] == "in_progress"), "")
    return {"source": "task list" if tasks else "none", "goal": "", "step": current, "doing": "", "why": "",
            "waiting_for": "", "updated": "", "tasks": tasks}


# ----------------------------------------------------------------- owner notes

def notes(data: Path, sid: str, state: SessionState | None = None) -> list[dict]:
    """Owner notes with their delivery state: pending, emitted, received (its id appears in a
    non-assistant transcript record after emission), answered (a later assistant text reply
    names the id; tool inputs and thinking never count)."""
    if not paths.valid_sid(sid):
        return []
    inbox, _ = read_head(data / "inbox" / f"{sid}.jsonl", budget=2 * 1024 * 1024)
    marks, _ = read_head(data / "inbox" / f"{sid}.state.jsonl", budget=2 * 1024 * 1024)
    emitted = {}
    for rec in marks:
        if isinstance(rec.get("id"), str):
            emitted.setdefault(rec["id"], ts(rec.get("emitted_at")))
    out = []
    for n in inbox:
        nid = n.get("id")
        if not isinstance(nid, str):
            continue
        entry = {"id": nid, "time": str(n.get("time", "")), "text": str(n.get("text", "")), "state": "pending"}
        if nid in emitted:
            entry["state"] = "emitted"
            at = emitted[nid] or 0
            seen = state.receipts.get(nid) if state else None
            if seen is not None and seen >= at - 5:
                entry["state"] = "received"
                answered = state.answers.get(nid)
                if answered is not None and answered >= seen:
                    entry["state"] = "answered"
        out.append(entry)
    return out


# ----------------------------------------------------------------- ledgers and budget

class Ledgers:
    """Complete, incremental reading of the private model-call ledgers (append-only JSONL).
    A ledger larger than LEDGER_MAX is not read and makes the totals incomplete."""

    def __init__(self):
        self.tails: dict = {}
        self.paid: dict = {}
        self.unknown: set = set()
        self.subscription: set = set()
        self.codex: dict = {}
        self.incomplete = False

    def update(self, checkouts: list) -> None:
        self.incomplete = False
        for c in checkouts:
            for f in sorted((Path(c) / "work" / "loop-memory" / "ledgers").glob("*.jsonl")):
                try:
                    if f.stat().st_size > LEDGER_MAX:
                        self.incomplete = True
                        continue
                except OSError:
                    continue
                tail = self.tails.setdefault(str(f), Tail(f))
                while True:  # the file is under LEDGER_MAX, so reading it completely is bounded
                    before = tail.offset
                    for r in tail.poll():
                        self._add(r)
                    if tail.caught_up or tail.offset == before:
                        break  # a record spanning a read boundary continues on the next poll, never stops the read
                if tail.oversized:
                    self.incomplete = True  # a record too large to read: the total cannot be complete

    def _add(self, r: dict) -> None:
        cid = r.get("call_id")
        if not isinstance(cid, str):
            return
        if r.get("agent") == "codex":
            self.codex[cid] = r
        cost = r.get("cost") if isinstance(r.get("cost"), dict) else {}
        if r.get("channel") == "subscription":
            self.subscription.add(cid)
        elif r.get("channel") == "paid-api":
            status, usd = cost.get("status"), cost.get("usd")
            if status in ("billed", "estimated", "reserved") and isinstance(usd, (int, float)) and not isinstance(usd, bool):
                self.paid[(cid, status)] = float(usd)
            else:
                self.unknown.add(cid)

    def budget(self) -> dict:
        sums = {k: round(sum(v for (_, s), v in self.paid.items() if s == k), 2) for k in ("billed", "estimated", "reserved")}
        return {**sums, "unknown_calls": len(self.unknown), "subscription_calls": len(self.subscription),
                "incomplete": self.incomplete}


def budget(checkouts: list, ledgers: Ledgers | None = None) -> dict:
    """Paid API amounts kept apart by cost status; subscription calls counted separately."""
    ledgers = ledgers or Ledgers()
    ledgers.update(checkouts)
    return ledgers.budget()


# ----------------------------------------------------------------- Codex calls

def codex_calls(checkouts: list, now: float | None = None, ledgers: Ledgers | None = None) -> list[dict]:
    now = time.time() if now is None else now
    ledgers = ledgers or Ledgers()
    ledgers.update(checkouts)
    calls = []
    for c in checkouts:
        base = Path(c) / "work" / "reviews"
        try:
            dirs = sorted((d for d in base.iterdir() if d.is_dir() and paths.CALL_ID_RE.match(d.name)), reverse=True)
        except OSError:
            continue
        for d in dirs:
            meta = read_json(d / "meta.json")
            rec = ledgers.codex.get(d.name)
            started = call_time(d.name)
            known = meta if isinstance(meta, dict) else rec or {}
            acceptance = meta.get("acceptance") if isinstance(meta, dict) else None
            exit_code = acceptance.get("exit_code") if isinstance(acceptance, dict) else None
            call = {"call_id": d.name, "checkout": str(c), "dir": str(d), "time": started,
                    "kind": str_or_none(known.get("kind")), "requested": known.get("requested") if isinstance(known.get("requested"), dict) else None,
                    "verdict": None, "findings": None, "blocking": None, "problems": [],
                    "acceptance_exit": exit_code if isinstance(exit_code, int) and not isinstance(exit_code, bool) else None}
            if meta is OVERSIZED:
                call["status"], call["problems"] = "unknown", ["meta.json is larger than the read budget"]
            elif isinstance(meta, dict):
                if meta.get("valid") is True:
                    call["status"], call["verdict"] = "finished", str_or_none(meta.get("verdict"))
                    result = read_json(d / "review.json")
                    if isinstance(result, dict) and isinstance(result.get("findings"), list):
                        fs = [f for f in result["findings"] if isinstance(f, dict)]
                        call["findings"] = len(fs)
                        call["blocking"] = [str(f.get("id")) for f in fs if f.get("severity") in ("blocker", "major")]
                else:
                    problems = meta.get("problems")
                    call["status"], call["problems"] = "failed", [str(p) for p in problems] if isinstance(problems, list) else []
            elif rec is not None:
                call["status"] = "failed" if rec.get("outcome") in ("timeout", "error", "invalid") else "finished"
                call["verdict"] = str_or_none(rec.get("outcome"))
            elif (d / "packet.md").is_file():
                call["status"] = "running" if started is None or now - started <= CODEX_STALE_AFTER else "failed"
                if call["status"] == "failed":
                    call["problems"] = ["stale: no result and no ledger record"]
            else:
                call["status"] = "unknown"
            calls.append(call)
    calls.sort(key=lambda c: c["call_id"], reverse=True)
    return calls


def link_calls(sessions: list[Session], calls: list[dict]) -> dict:
    """call_id -> (sid, how). After return by the printed call id; while running by time."""
    links = {}
    for s in sessions:
        if s.state is None:
            continue
        for cid, how in s.state.call_links.items():
            links[cid] = (s.sid, how)
    for s in sessions:
        if s.state is None or not s.cwd:
            continue
        for call in calls:
            if call["call_id"] in links or call.get("status") != "running" or call.get("time") is None:
                continue
            if not paths.inside(Path(s.cwd), [call["checkout"]]):
                continue
            if any(abs(t - call["time"]) <= LINK_WINDOW for t in s.state.codex_uses):
                links[call["call_id"]] = (s.sid, "time")
    return links


# ----------------------------------------------------------------- progress, findings, login, files

def progress(main: Path) -> dict | None:
    doc = read_json(Path(main) / "docs" / "progress" / "status.json", limit=256 * 1024)
    return doc if isinstance(doc, dict) else None


def current_step(doc: dict | None) -> dict | None:
    if not doc or not isinstance(doc.get("steps"), list):
        return None
    return next((s for s in doc["steps"] if isinstance(s, dict) and s.get("id") == doc.get("current")), None)


def open_findings(checkouts: list) -> list[str]:
    """Unanswered findings in the latest reviews of each checkout (same rule as the SessionStart hook)."""
    out = []
    for c in checkouts:
        for meta_path in sorted((Path(c) / "work" / "reviews").glob("*/meta.json"), reverse=True)[:5]:
            meta = read_json(meta_path)
            if not isinstance(meta, dict) or meta.get("verdict") != "findings":
                continue
            result = read_json(meta_path.parent / "review.json")
            answered = read_json(meta_path.parent / "dispositions.json")
            answered = answered if isinstance(answered, dict) else {}
            if not isinstance(result, dict) or not isinstance(result.get("findings"), list):
                continue
            out += [f"{meta.get('call_id')}:{f.get('id')}" for f in result["findings"]
                    if isinstance(f, dict) and f.get("id") not in answered]
    return out


def codex_login() -> str:
    try:
        return "ready" if (paths.codex_home() / "auth.json").is_file() else "not-ready"
    except (OSError, RuntimeError):
        return "unknown"


def referenced_files(state: SessionState | None, roots: list) -> list[str]:
    """Files a session touched (tool inputs) that still exist inside a checkout."""
    if state is None:
        return []
    return [p for p in state.files if Path(p).is_file() and paths.inside(Path(p), roots)]
