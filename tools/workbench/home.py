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
    raise NotImplementedError


def answer_states(note_list: list[dict]) -> dict[str, dict]:
    """Map `<ref>` (for example `ask 20260930T...`) to {"state", "note": id, "body": text} from one
    session's `sources.notes` entries whose text starts with ANSWER_RE. State: pending ->
    "queued"; emitted or received -> "sent" (delivery not confirmed); answered ->
    "acknowledged" (a later assistant text reply names the note id). The newest answer per ref
    wins (note time, then list order)."""
    raise NotImplementedError


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
    raise NotImplementedError


def machine_badge(flags: list[dict]) -> int:
    """Number of flags whose kind is in MACHINE_KINDS (failures and stalls)."""
    return sum(1 for f in flags if isinstance(f, dict) and f.get("kind") in MACHINE_KINDS)


def image_size(path: Path) -> tuple[int, int] | None:
    """(width, height) from at most IMAGE_HEAD_BYTES of a PNG, GIF, WebP or JPEG file, without
    decoding pixels; None when the format is unknown, the header is malformed or unreadable."""
    raise NotImplementedError


class GalleryScanner:
    """Bounded scan of visual outputs, repeated at most every SCAN_INTERVAL seconds."""

    def __init__(self, interval: float = SCAN_INTERVAL):
        raise NotImplementedError

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
        raise NotImplementedError


def progress_view(doc: dict | None, sessions: list) -> dict:
    """{"schema": 1 | 2 | None, "noChecklist": bool, "updated": str,
        "tracks": [{"id", "title", "percent" (float or None), "steps": [{"id", "title", "status",
                    "percent", "done", "total"}], "current": {"id", "title", "percent",
                    "remaining": [{"id", "title", "weight"}]} or None}],
        "sessions": [{"sid", "title", "done", "total"}]}
    Percentages only from `checklist`; a schema-1 document gives noChecklist true and None
    percentages. A track's current step is the document's `current` when it is in the track,
    else the first step of the track that is not done, else its last step."""
    raise NotImplementedError


def counts(cards: list[dict], items: list[dict]) -> dict:
    """{"for_you": cards with needsOwner, "gallery_new": items with isNew}."""
    return {"for_you": sum(1 for c in cards if c.get("needsOwner") is True),
            "gallery_new": sum(1 for i in items if i.get("isNew") is True)}


def snapshot(counts_: dict, now: float) -> dict:
    """The document the app writes to <data>/SNAPSHOT_NAME:
    {"schema": 1, "written": ISO UTC of now, "for_you": int, "gallery_new": int}."""
    return {"schema": 1, "written": datetime.fromtimestamp(now, timezone.utc).isoformat(timespec="seconds"),
            "for_you": int(counts_["for_you"]), "gallery_new": int(counts_["gallery_new"])}
