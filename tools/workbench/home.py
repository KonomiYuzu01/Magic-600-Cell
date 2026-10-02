"""Home view of the workbench: the owner's inbox ("For you"), the gallery and the progress view.

Pure derivations over data the caller supplies (sessions, Codex calls, run records,
attention flags) plus bounded reads of private files under work/loop-memory/workbench/
(asks, briefs, owner notes) and of visual outputs inside the checkouts. Nothing here
writes, starts processes or calls models. Malformed records are skipped and counted.

Answers never use a second channel: the app sends them as ordinary owner notes
(`notes.append_note`) whose text is `answer to <ref>: <body>` (see `answer_text`), and a
card's state is derived from that note's delivery state (`sources.notes`). Notes reach
Claude only at its next main-thread tool step, so a card whose answer is queued while its
session is not running still needs the owner (open the session and send any message).
"""
from __future__ import annotations

import hashlib
import heapq
import itertools
import math
import os
import re
from datetime import datetime, timezone
from pathlib import Path

try:
    from . import checklist, notes, paths, sources, watch
except ImportError:
    import checklist  # type: ignore[no-redef]
    import notes  # type: ignore[no-redef]
    import paths  # type: ignore[no-redef]
    import sources  # type: ignore[no-redef]
    import watch  # type: ignore[no-redef]

# ----------------------------------------------------------------- asks

ASK_MAX_BYTES = 16 * 1024
ASK_SCAN_MAX = 400      # directory entries listed per session; more are reported as skipped
ASKS_PER_SESSION = 100
ASK_KINDS = ("decision", "pick", "approve", "question")
QUESTION_MAX_BYTES = 500
CONTEXT_MAX_BYTES = 1000
OPTION_MAX_BYTES = 120
OPTIONS_MAX = 8
ATTACHMENTS_MAX = 8
ID_RE = re.compile(r"^\d{8}T\d{6}Z-[0-9a-f]{8}$")

# ----------------------------------------------------------------- answers

ANSWER_REFS = ("ask", "codex call", "waiting-for")
ANSWER_RE = re.compile(r"^answer to (ask|codex call|waiting-for) ([A-Za-z0-9:+._-]{1,64}): ")
REF_RE = re.compile(r"^(ask|codex call|waiting-for) [A-Za-z0-9:+._-]{1,64}$")
OWNER_WAIT_RE = re.compile(r"^\s*(the\s+)?owner\b", re.IGNORECASE)
MACHINE_KINDS = ("failed_turn", "stalled", "codex_failed", "run_failed", "run_overdue")

# ----------------------------------------------------------------- gallery

IMAGE_EXT = (".png", ".jpg", ".jpeg", ".gif", ".webp")
VECTOR_EXT = (".svg",)
PDF_EXT = (".pdf",)
VIDEO_EXT = (".mp4", ".webm")
DIAGRAM_EXT = (".mmd", ".drawio", ".typ")
SIBLING_EXT = (".svg", ".png")             # rendered siblings shown for a diagram or slides source
HEAD_BYTES = 4096                          # read to recognize Marp slides and marimo notebooks
IMAGE_HEAD_BYTES = 256 * 1024              # read by image_size
PREVIEW_PIXELS = 40_000_000                # decoded-size bound of an in-app raster preview
GALLERY_DIRS = ("work/gallery", "work/loop-memory/ui-vision")   # in every checkout
FIGURES_GLOB = "docs/**/figures"                                 # main checkout only
WALK_DEPTH = 6
WALK_ENTRIES = 5000
GALLERY_ITEMS = 300
FILE_MAX = 200 * 1024 * 1024
PREVIEW_MAX = 25 * 1024 * 1024
PREVIEW_TOTAL = 512 * 1024 * 1024
SCAN_INTERVAL = 10.0

SNAPSHOT_NAME = "home.json"   # read by tools/workbench/statusline.py

_MALFORMED = (AttributeError, KeyError, TypeError, ValueError, OverflowError, OSError, RuntimeError)


def _text(value, limit, line=False) -> bool:
    return (isinstance(value, str) and len(value.encode("utf-8")) <= limit
            and (not line or (bool(value.strip()) and "\r" not in value and "\n" not in value)))


def _location(path, roots) -> tuple[Path, str, str] | None:
    p = Path(path).resolve()
    if not paths.inside(p, roots) or not p.is_file():
        return None
    root = max((Path(r).resolve() for r in roots if paths.inside(p, [r])), key=lambda r: len(r.parts))
    return p, p.relative_to(root).as_posix(), root.name


def _preview(path: Path, roots) -> tuple[str, list]:
    try:
        location = _location(path, roots)
        if location is None:
            return "", []
        p, _, _ = location
        size, ext = p.stat().st_size, p.suffix.lower()
        if ext in IMAGE_EXT:
            pixels = image_size(p)
            return (str(p) if pixels and pixels[0] * pixels[1] <= PREVIEW_PIXELS and size <= PREVIEW_MAX else "",
                    list(pixels) if pixels else [])
        if ext == ".svg" and size <= PREVIEW_MAX:
            return str(p), []
    except _MALFORMED:
        pass
    return "", []


def answer_text(ref: str, body: str) -> str:
    """`answer to <ref>: <body>`; `ref` is `ask <id>`, `codex call <id>` or `waiting-for <updated>`.
    Raises ValueError for another ref form, an empty body, a body with a line break, or a note
    longer than notes.NOTE_MAX_BYTES."""
    if not isinstance(ref, str) or not REF_RE.match(ref):
        raise ValueError("unknown answer reference")
    if not isinstance(body, str) or "\r" in body or "\n" in body or not body.strip():
        raise ValueError("an answer is one non-empty line")
    text = f"answer to {ref}: {body.strip()}"
    if len(text.encode("utf-8")) > notes.NOTE_MAX_BYTES:
        raise ValueError("the answer is too long")
    return text


def asks(data: Path, sid: str, roots: list | None = None) -> tuple[list[dict], int]:
    """The session's ask records (work/loop-memory/workbench/asks/<sid>/<id>.json), newest first.
    Reads at most the ASKS_PER_SESSION newest files, each at most ASK_MAX_BYTES; returns
    (records, skipped). A record is kept only when every field satisfies the rules in
    tools/workbench/ask.py. Its "attachments" become [{"index": original 1-based position,
    "path": str}] and keep only files that are still regular files inside `roots` (default: the
    checkouts of the main checkout that owns `data`; `paths.inside`); a failing attachment is
    dropped, not the whole record."""
    if not paths.valid_sid(sid):
        return [], 0
    records, skipped = [], 0
    try:
        roots = paths.checkouts(data.parents[2]) if roots is None else roots
        with os.scandir(data / "asks" / sid) as entries:
            listed = list(itertools.islice(entries, ASK_SCAN_MAX + 1))
            if len(listed) > ASK_SCAN_MAX:
                listed, skipped = listed[:ASK_SCAN_MAX], 1
            files = heapq.nlargest(ASKS_PER_SESSION, (e.name for e in listed if e.name.endswith(".json") and e.is_file()))
    except FileNotFoundError:
        return [], 0
    except _MALFORMED:
        return [], 1
    for name in files:
        try:
            p = data / "asks" / sid / name
            r = sources.read_json(p, ASK_MAX_BYTES)
            if not isinstance(r, dict) or type(r.get("schema")) is not int or r["schema"] != 1 \
                    or not isinstance(r.get("id"), str) or not ID_RE.fullmatch(r["id"]) or r["id"] != p.stem \
                    or r.get("session_id") != sid or sources.ts(r.get("created")) is None \
                    or r.get("kind") not in ASK_KINDS or not isinstance(r.get("blocking"), bool) \
                    or not _text(r.get("question"), QUESTION_MAX_BYTES, line=True) \
                    or not _text(r.get("context"), CONTEXT_MAX_BYTES):
                raise ValueError("invalid ask")
            options, attachments = r.get("options"), r.get("attachments")
            if not isinstance(options, list) or len(options) > OPTIONS_MAX \
                    or not all(_text(o, OPTION_MAX_BYTES, line=True) for o in options) \
                    or len(set(options)) != len(options) \
                    or not isinstance(attachments, list) or len(attachments) > ATTACHMENTS_MAX \
                    or not all(isinstance(a, str) for a in attachments):
                raise ValueError("invalid ask options or attachments")
            if r["kind"] == "decision" and len(options) < 2:
                raise ValueError("decision needs options")
            if r["kind"] == "pick" and (options or len(attachments) < 2
                                         or any(Path(a).suffix.lower() not in (*IMAGE_EXT, ".svg") for a in attachments)):
                raise ValueError("pick needs images")
            kept = []
            for index, attachment in enumerate(attachments, 1):
                try:
                    location = _location(attachment, roots)
                    if location:
                        kept.append({"index": index, "path": str(location[0])})
                except _MALFORMED:
                    pass
            records.append({**r, "attachments": kept})
        except _MALFORMED:
            skipped += 1
    return sorted(records, key=lambda r: sources.ts(r["created"]), reverse=True), skipped


def answer_states(note_list: list[dict]) -> dict[str, dict]:
    """Map `<ref>` (for example `ask 20260930T...`) to {"state", "note": id, "body": text} from one
    session's `sources.notes` entries whose text starts with ANSWER_RE. State: pending ->
    "queued"; emitted or received -> "sent" (delivery not confirmed); answered ->
    "acknowledged" (a later assistant text reply names the note id). The newest answer per ref
    wins (note time, then list order)."""
    out, latest = {}, {}
    states = {"pending": "queued", "emitted": "sent", "received": "sent", "answered": "acknowledged"}
    for index, note in enumerate(note_list):
        try:
            match = ANSWER_RE.match(note["text"])
            if match is None or note["state"] not in states:
                continue
            ref = match.group(1) + " " + match.group(2)
            stamp = (note["time"], index)
            if not isinstance(note["time"], str) or not isinstance(note["id"], str):
                continue
            if ref not in latest or stamp > latest[ref]:
                latest[ref] = stamp
                out[ref] = {"state": states[note["state"]], "note": note["id"], "body": note["text"][match.end():]}
        except _MALFORMED:
            continue
    return out


def inbox(sessions: list, calls: list[dict], links: dict, runs: list[dict], flags: list[dict],
          data: Path, roots: list, now: float) -> tuple[list[dict], int]:
    """The owner's cards, newest first, and the number of skipped malformed inputs.

    Card dict (every key always present):
      id            "ask:<id>" | "wait:<sid>:<agent or main>:<12 hex of sha1(wait key)>"
                    | "codex:<call-id>" | "run:<rid>" | "brief:<sid>:<updated>"
      kind          "ask" | "wait" | "failure" | "waiting_for"
      askKind       ask kind, else ""
      blocking      bool
      sid           owning session id, else ""
      sessionTitle  str
      t             float (created, wait start, failure time or brief update)
      age           sources.age_text(now - t)
      title         one line
      context       plain text
      detail        plain text (a permission wait: the tool and its command summary)
      options       list[str]; a pick ask: "picked <index>: <checkout name>/<rel>" per attachment
      attachments   list of {"index", "path", "rel", "checkout", "previewable"}
      answerRef     "ask <id>" | "codex call <id>" | "waiting-for <updated>" | "" (no reply possible)
      state         "open" | "queued" | "sent" | "acknowledged" (see answer_states)
      needsOwner    state "open", or "queued" while the owning session is not running
      answer        body of the answer note, else ""
      canOpenSession bool (a discovered session)
      files         list[str] of existing files to open (review report, run log)

    Sources:
      asks of every discovered session;
      owner waits from `session.status.wait_items` (no reply, no approval);
      failures from `flags` kinds codex_failed and run_failed, shown only when the owning
        session is finished or failed, idle (turn ended), or has a "stalled" flag in `flags`
        (watch's predicate, wrapper exemption included); unlinked calls and owner-launched
        runs show at once;
      briefs whose waiting_for matches OWNER_WAIT_RE, of sessions not finished, updated
        within watch.WATCH_WINDOW."""
    cards, skipped, by_sid, by_call, stalled = [], 0, {}, {}, set()
    for s in sessions:
        try:
            if not isinstance(s, sources.Session) or not paths.valid_sid(s.sid) \
                    or not isinstance(s.title, str) or not isinstance(s.status, sources.Status):
                raise ValueError("invalid session")
            by_sid[s.sid] = s
        except _MALFORMED:
            skipped += 1
    for call in calls:
        try:
            if not isinstance(call["call_id"], str):
                raise ValueError("invalid call")
            by_call[call["call_id"]] = call
        except _MALFORMED:
            skipped += 1
    valid_flags = []
    for flag in flags:
        try:
            kind, target = flag["kind"], flag["target_id"]
            if not isinstance(kind, str) or not isinstance(target, str):
                raise ValueError("invalid flag")
            valid_flags.append(flag)
            if kind == "stalled":
                stalled.add(target)
        except _MALFORMED:
            skipped += 1

    def card(cid, kind, sid, t, title, **fields):
        if not isinstance(t, (float, int)) or isinstance(t, bool) or not math.isfinite(t):
            raise ValueError("invalid card time")
        if not isinstance(title, str) or not isinstance(sid, str):
            raise ValueError("invalid card text")
        s = by_sid.get(sid)
        return {"id": cid, "kind": kind, "askKind": "", "blocking": False, "sid": sid,
                "sessionTitle": (s.title if s else "") or sid[:8], "t": float(t), "age": sources.age_text(now - t),
                "title": title, "context": "", "detail": "", "options": [], "attachments": [],
                "answerRef": "", "state": "open", "needsOwner": True, "answer": "", "canOpenSession": s is not None,
                "files": [], **fields}

    for sid, s in by_sid.items():
        try:
            records, bad = asks(data, sid, roots)
            skipped += bad
        except _MALFORMED:
            records = []
            skipped += 1
        for r in records:
            try:
                attachments = []
                for a in r["attachments"]:
                    location = _location(a["path"], roots)
                    if location:
                        p, rel, checkout = location
                        attachments.append({"index": a["index"], "path": str(p), "rel": rel, "checkout": checkout,
                                            "previewable": bool(_preview(p, roots)[0])})
                pick = r["kind"] == "pick"
                options = [f"picked {a['index']}: {a['checkout']}/{a['rel']}" for a in attachments] if pick else r["options"]
                cards.append(card(f"ask:{r['id']}", "ask", sid, sources.ts(r["created"]), r["question"],
                                  askKind=r["kind"], blocking=r["blocking"], context=r["context"], options=options,
                                  attachments=attachments, answerRef=f"ask {r['id']}",
                                  detail="Some attachments are no longer available." if pick and len(attachments) < 2 else ""))
            except _MALFORMED:
                skipped += 1
        try:
            waits = s.status.wait_items
            if not isinstance(waits, list):
                raise ValueError("invalid waits")
        except _MALFORMED:
            waits = []
            skipped += 1
        for w in waits:
            try:
                key, agent, label, tool, summary = (w[k] for k in ("key", "agent", "label", "tool", "summary"))
                if not all(isinstance(v, str) for v in (key, label, summary)) \
                        or any(v is not None and not isinstance(v, str) for v in (agent, tool)):
                    raise ValueError("invalid wait")
                title = (f"Permission: {tool}" if label.startswith("permission")
                         else f"Question in Claude Code ({tool})" if label.startswith("question")
                         else "Input requested in Claude Code")
                if agent:
                    title += f" (subagent {agent})"
                cid = f"wait:{sid}:{agent or 'main'}:{hashlib.sha1(key.encode()).hexdigest()[:12]}"
                cards.append(card(cid, "wait", sid, w["t"], title, detail=summary,
                                  context="Answer this in Claude Code; the workbench cannot approve or answer it."))
            except _MALFORMED:
                skipped += 1
        try:
            b = sources.brief(data, sid) if s.status.status != "finished" else None
            if b is not None:
                waiting, updated = b.get("waiting_for", ""), b.get("updated")
                t = sources.ts(updated)
                if isinstance(waiting, str) and OWNER_WAIT_RE.match(waiting) and t is not None and now - t <= watch.WATCH_WINDOW:
                    detail = " · ".join(f"{label}: {b[key]}" for key, label in (("goal", "Goal"), ("step", "Step")) if b.get(key))
                    ref = f"waiting-for {updated}"
                    cards.append(card(f"brief:{sid}:{updated}", "waiting_for", sid, t, "Waiting for you", context=waiting,
                                      detail=detail, answerRef=ref if REF_RE.fullmatch(ref) else ""))
        except _MALFORMED:
            skipped += 1
    for flag in valid_flags:
        try:
            kind, target = flag["kind"], flag["target_id"]
            if kind not in ("codex_failed", "run_failed"):
                continue
            sid = links.get(target, ("", ""))[0] if kind == "codex_failed" else ""
            s = by_sid.get(sid)
            if s and not (s.status.status in ("finished", "failed")
                          or (s.status.status == "waiting" and not s.status.waits) or sid in stalled):
                continue
            files = []
            if kind == "codex_failed":
                call = by_call.get(target)
                candidates = [Path(call["dir"]) / name for name in ("review.json", "report.md", "meta.json")] if call else []
            else:
                candidates = [data / "runs" / f"{target}.log"]
            for p in candidates:
                try:
                    if p.is_file():
                        files.append(str(p))
                except OSError:
                    pass
            cards.append(card(f"{'codex' if kind == 'codex_failed' else 'run'}:{target}", "failure", sid, flag["t"],
                              flag["text"], files=files, answerRef=f"codex call {target}" if s else ""))
        except _MALFORMED:
            skipped += 1
    for sid, s in by_sid.items():
        own_cards = [c for c in cards if c["sid"] == sid and c["answerRef"]]
        if not own_cards:
            continue
        try:
            answers = answer_states(sources.notes(data, sid, s.state))
            for c in own_cards:
                answer = answers.get(c["answerRef"])
                if answer:
                    c["state"], c["answer"] = answer["state"], answer["body"]
                    c["needsOwner"] = c["state"] == "queued" and s.status.status != "running"
        except _MALFORMED:
            skipped += 1
    return sorted(cards, key=lambda c: (-c["t"], c["id"])), skipped


def machine_badge(flags: list[dict]) -> int:
    """Number of flags whose kind is in MACHINE_KINDS (failures and stalls)."""
    return sum(1 for f in flags if isinstance(f, dict) and f.get("kind") in MACHINE_KINDS)


def image_size(path: Path) -> tuple[int, int] | None:
    """(width, height) from at most IMAGE_HEAD_BYTES of a PNG, GIF, WebP or JPEG file, without
    decoding pixels; None when the format is unknown, the header is malformed or unreadable."""
    try:
        with path.open("rb") as f:
            head = f.read(IMAGE_HEAD_BYTES)
        w = h = 0
        if head.startswith(b"\x89PNG\r\n\x1a\n"):
            if len(head) < 33 or head[8:16] != b"\0\0\0\x0dIHDR":
                return None
            w, h = int.from_bytes(head[16:20], "big"), int.from_bytes(head[20:24], "big")
        elif head[:6] in (b"GIF87a", b"GIF89a"):
            if len(head) < 13:
                return None
            w, h = int.from_bytes(head[6:8], "little"), int.from_bytes(head[8:10], "little")
        elif head[:4] == b"RIFF" and head[8:12] == b"WEBP":
            if len(head) < 20:
                return None
            kind, size = head[12:16], int.from_bytes(head[16:20], "little")
            if int.from_bytes(head[4:8], "little") < 12 + size + size % 2:
                return None
            body = head[20:]
            if kind == b"VP8 " and size >= 10 and len(body) >= 10:
                if body[0] & 1 or body[3:6] != b"\x9d\x01\x2a":
                    return None
                w, h = int.from_bytes(body[6:8], "little") & 0x3fff, int.from_bytes(body[8:10], "little") & 0x3fff
            elif kind == b"VP8L" and size >= 5 and len(body) >= 5:
                if body[0] != 0x2f or body[4] & 0xe0:
                    return None
                bits = int.from_bytes(body[1:5], "little")
                w, h = (bits & 0x3fff) + 1, ((bits >> 14) & 0x3fff) + 1
            elif kind == b"VP8X" and size >= 10 and len(body) >= 10:
                w, h = int.from_bytes(body[4:7], "little") + 1, int.from_bytes(body[7:10], "little") + 1
            else:
                return None
        elif head.startswith(b"\xff\xd8"):
            pos = 2
            while pos < len(head):
                if head[pos] != 0xff:
                    return None
                while pos < len(head) and head[pos] == 0xff:
                    pos += 1
                if pos == len(head):
                    return None
                marker = head[pos]
                pos += 1
                if marker in (0x00, 0xd8, 0xd9, 0xda):
                    return None
                if marker == 0x01 or 0xd0 <= marker <= 0xd7:
                    continue
                if pos + 2 > len(head):
                    return None
                length = int.from_bytes(head[pos:pos + 2], "big")
                if length < 2 or pos + length > len(head):
                    return None
                if 0xc0 <= marker <= 0xcf and marker not in (0xc4, 0xc8, 0xcc):
                    if length < 8 or length != 8 + 3 * head[pos + 7]:
                        return None
                    h, w = int.from_bytes(head[pos + 3:pos + 5], "big"), int.from_bytes(head[pos + 5:pos + 7], "big")
                    break
                pos += length
        return (w, h) if w > 0 and h > 0 else None
    except _MALFORMED:
        return None


class GalleryScanner:
    """Bounded scan of visual outputs, repeated at most every SCAN_INTERVAL seconds."""

    def __init__(self, interval: float = SCAN_INTERVAL):
        self.interval = interval
        self.last_scan = None
        self.items = []

    def scan(self, main: Path, roots: list, sessions: list, runs: list[dict], ask_records: list[dict],
             now: float, force: bool = False) -> list[dict]:
        """Gallery items, newest first (at most GALLERY_ITEMS). Returns the cached list when the
        last scan is younger than the interval and `force` is false.

        Item dict (every key always present):
          path, rel, checkout      resolved path, checkout-relative posix path, checkout name
          type                     "image" | "vector" | "pdf" | "video" | "diagram" | "slides" | "notebook"
          ext, mtime, size
          preview                  in-app previewable file (itself or a rendered sibling), else "";
                                   a raster preview needs image_size known and within PREVIEW_PIXELS
          pixels                   [width, height] of the raster file or raster preview when known, else []
          source                   the editable source file (itself when there is none)
          origins                  subset of "referenced", "run-output", "ask-attachment", "gallery", "ui-vision", "figures"
          sessions                 session ids that touched or produced it, newest first
          isNew                    mtime within watch.WATCH_WINDOW of now"""
        if not force and self.last_scan is not None and now - self.last_scan < self.interval:
            return self.items
        found, session_times = {}, {}

        def add(path, origin, sid=None):
            try:
                location = _location(path, roots)
                if location is None:
                    return
                p, rel, checkout = location
                st = p.stat()
                if st.st_size > FILE_MAX:
                    return
                key = str(p)
                item = found.setdefault(key, {"path": key, "rel": rel, "checkout": checkout, "ext": p.suffix.lower(),
                                              "mtime": st.st_mtime, "size": st.st_size, "source": key, "preview": "",
                                              "pixels": [], "origins": [], "sessions": [],
                                              "isNew": now - st.st_mtime <= watch.WATCH_WINDOW})
                if origin not in item["origins"]:
                    item["origins"].append(origin)
                if isinstance(sid, str) and sid and sid not in item["sessions"]:
                    item["sessions"].append(sid)
            except _MALFORMED:
                pass

        for s in sessions:
            try:
                session_times[s.sid] = s.status.last_any or 0
                for path in sources.referenced_files(s.state, roots):
                    add(path, "referenced", s.sid)
            except _MALFORMED:
                continue
        for run in runs:
            try:
                t = sources.ts(run.get("ended"))
                if t is None:
                    t = sources.ts(run.get("started"))
                if t is None or now - t > watch.WATCH_WINDOW:
                    continue
                checkout = Path(run["checkout"])
                outputs = run["outputs"]
                if not isinstance(outputs, list):
                    continue
                for output in outputs:
                    if isinstance(output, str):
                        add(checkout / output, "run-output")
            except _MALFORMED:
                continue
        for record in ask_records:
            try:
                sid = record["session_id"]
                for attachment in record["attachments"]:
                    try:
                        add(attachment["path"], "ask-attachment", sid)
                    except _MALFORMED:
                        continue
            except _MALFORMED:
                continue

        visited = 0

        def walk(directory, depth=0, collect=True):
            nonlocal visited
            if visited >= WALK_ENTRIES:
                return
            try:
                with os.scandir(directory) as entries:
                    while visited < WALK_ENTRIES:
                        try:
                            entry = next(entries)
                        except StopIteration:
                            return
                        visited += 1
                        try:
                            if entry.is_symlink() or (hasattr(entry, "is_junction") and entry.is_junction()):
                                continue
                            if entry.is_file(follow_symlinks=False):
                                if collect:
                                    yield Path(entry.path)
                            elif entry.is_dir(follow_symlinks=False) and depth < WALK_DEPTH:
                                yield from walk(entry.path, depth + 1, collect or entry.name == "figures")
                        except OSError:
                            continue
            except OSError:
                return

        starts = [(Path(root), subdir, "gallery" if subdir == "work/gallery" else "ui-vision")
                  for root in roots for subdir in GALLERY_DIRS]
        starts.append((Path(main), "docs", "figures"))
        for root, subdir, origin in starts:
            try:
                root = root.resolve()
                start = root / subdir
                # Check the start's ancestors too: scandir must never enter a linked work or docs directory.
                chain = [root.joinpath(*Path(subdir).parts[:i]) for i in range(1, len(Path(subdir).parts) + 1)]
                if any(p.is_symlink() or (hasattr(p, "is_junction") and p.is_junction()) for p in chain):
                    continue
                if not paths.inside(start, roots):
                    continue
                for path in walk(start, collect=origin != "figures"):
                    add(path, origin)
            except _MALFORMED:
                continue

        items, folded = [], set()
        for item in found.values():
            try:
                p, ext = Path(item["path"]), item["ext"]
                siblings = []
                if ext in IMAGE_EXT:
                    item["type"] = "image"
                    item["preview"], item["pixels"] = _preview(p, roots)
                elif ext == ".svg":
                    item["type"] = "vector"
                    item["preview"], item["pixels"] = _preview(p, roots)
                elif ext in PDF_EXT:
                    item["type"] = "pdf"
                elif ext in VIDEO_EXT:
                    item["type"] = "video"
                elif ext in DIAGRAM_EXT:
                    item["type"] = "diagram"
                    siblings = [p.with_suffix(e) for e in SIBLING_EXT] + [p.with_name(p.name + e) for e in SIBLING_EXT]
                elif ext in (".md", ".py"):
                    with p.open("rb") as f:
                        head = f.read(HEAD_BYTES)
                    if ext == ".py" and b"marimo.App(" in head:
                        item["type"] = "notebook"
                    elif ext == ".md" and head.splitlines()[:1] == [b"---"]:
                        front = []
                        for line in head.splitlines()[1:]:
                            if line.strip() in (b"---", b"..."):
                                break
                            front.append(line.strip())
                        if b"marp: true" not in front:
                            continue
                        item["type"] = "slides"
                        siblings = [p.with_suffix(e) for e in (".png", ".svg")]
                    else:
                        continue
                else:
                    continue
                for sibling in siblings:
                    preview, pixels = _preview(sibling, roots)
                    if preview:
                        item["preview"], item["pixels"] = preview, pixels
                        folded.add(preview)
                        break
                item["sessions"].sort(key=lambda sid: (-(session_times.get(sid) or 0), sid))
                items.append(item)
            except _MALFORMED:
                continue
        items = sorted((i for i in items if i["path"] not in folded), key=lambda i: (-i["mtime"], i["path"]))[:GALLERY_ITEMS]
        total = 0
        for item in items:
            if item["preview"]:
                try:
                    size = Path(item["preview"]).stat().st_size
                    if size <= PREVIEW_MAX and total + size <= PREVIEW_TOTAL:
                        total += size
                    else:
                        item["preview"] = ""
                except OSError:
                    item["preview"] = ""
        self.last_scan, self.items = now, items
        return items


def progress_view(doc: dict | None, sessions: list) -> dict:
    """{"schema": 1 | 2 | None, "noChecklist": bool, "updated": str,
        "tracks": [{"id", "title", "percent" (float or None), "steps": [{"id", "title", "status",
                    "percent", "done", "total"}], "current": {"id", "title", "percent",
                    "remaining": [{"id", "title", "weight"}]} or None}],
        "sessions": [{"sid", "title", "done", "total"}]}
    Percentages only from `checklist`; a schema-1 document gives noChecklist true and None
    percentages. Tracks come from checklist.tracks, in its order. A track's current step is the document's `current` when it is in the track,
    else the first step of the track that is not done, else its last step. Never raises: a
    document that cannot be derived gives the schema-1 view."""
    try:
        return _progress_view(doc, sessions)
    except _MALFORMED:
        return _progress_view(None, sessions)


def _progress_view(doc, sessions) -> dict:
    doc = doc if isinstance(doc, dict) else {}
    try:
        no_checklist = bool(checklist.validate(doc))
    except _MALFORMED:
        no_checklist = True
    steps = doc.get("steps")
    steps = [s for s in steps if isinstance(s, dict)
             and all(isinstance(s.get(k), str) for k in ("id", "title", "status"))] if isinstance(steps, list) else []
    tracks = []
    for tid, title in checklist.tracks(doc):
        source_steps = [s for s in steps if s.get("track") == tid]
        view_steps = []
        for s in source_steps:
            items = s.get("items", []) if not no_checklist else []
            view_steps.append({"id": s["id"], "title": s["title"], "status": s["status"],
                               "percent": None if no_checklist else checklist.step_percent(s),
                               "done": sum(i["weight"] for i in items if checklist.counted(i)),
                               "total": sum(i["weight"] for i in items)})
        current = next((s for s in source_steps if s["id"] == doc.get("current")), None)
        if current is None:
            current = next((s for s in source_steps if s["status"] != "done"), source_steps[-1] if source_steps else None)
        current_view = None
        if current is not None:
            current_view = {"id": current["id"], "title": current["title"],
                            "percent": None if no_checklist else checklist.step_percent(current),
                            "remaining": [{k: i[k] for k in ("id", "title", "weight")} for i in checklist.remaining(current)
                                          if isinstance(i.get("id"), str) and isinstance(i.get("title"), str)
                                          and type(i.get("weight")) is int and i["weight"] > 0]}
        tracks.append({"id": tid, "title": title, "percent": None if no_checklist else checklist.track_percent(doc, tid),
                       "steps": view_steps, "current": current_view})
    tasks = []
    for s in sessions:
        try:
            entries = s.state.task_list() if s.state else []
            if entries:
                tasks.append({"sid": s.sid, "title": s.title or s.sid[:8],
                              "done": sum(1 for e in entries if e.get("status") == "completed"), "total": len(entries)})
        except _MALFORMED:
            continue
    schema = doc.get("schema")
    return {"schema": schema if type(schema) is int and schema in (1, 2) else None, "noChecklist": no_checklist,
            "updated": doc.get("updated") if isinstance(doc.get("updated"), str) else "", "tracks": tracks, "sessions": tasks}


def counts(cards: list[dict], items: list[dict]) -> dict:
    """{"for_you": cards with needsOwner, "gallery_new": items with isNew}."""
    return {"for_you": sum(1 for c in cards if c.get("needsOwner") is True),
            "gallery_new": sum(1 for i in items if i.get("isNew") is True)}


def snapshot(counts_: dict, now: float) -> dict:
    """The document the app writes to <data>/SNAPSHOT_NAME:
    {"schema": 1, "written": ISO UTC of now, "for_you": int, "gallery_new": int}."""
    return {"schema": 1, "written": datetime.fromtimestamp(now, timezone.utc).isoformat(timespec="seconds"),
            "for_you": int(counts_["for_you"]), "gallery_new": int(counts_["gallery_new"])}
