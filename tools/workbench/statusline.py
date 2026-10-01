"""Claude Code status line for Magic 600 Cell, for example `Step 0.4.1-1 12% · for you 2 · gallery +3`.

- The step percentage comes from the checklist in docs/progress/status.json
  (schema 2): done weight over total weight, counting only items with accepted
  evidence. A schema-1 file shows `no checklist`.
- `for you` (open inbox cards) and `gallery` (visual outputs of the last 24 hours)
  come from the snapshot the running workbench writes to
  work/loop-memory/workbench/home.json; without a fresh one the line says
  `workbench closed`.
- The paid API spend is appended only when some amount is not zero, a paid call
  has an unknown cost, or a ledger could not be listed or read in full (then no
  amount is shown).

Claude Code runs this on every status-line refresh in every session, so it is a
critical path: standard library only, no repository imports, bounded reads, no
writes, no network, no subprocesses. It prints exactly one UTF-8 line of at most
LINE_MAX characters; any error prints `workbench: unavailable`. Its figures match
tools/workbench/checklist.py and tools/workbench/sources.py (tests compare them
on fixtures).
"""
from __future__ import annotations

import itertools
import json
import math
import os
import re
import sys
import time
from datetime import datetime
from pathlib import Path

STDIN_MAX = 64 * 1024
DOC_MAX = 2 * 1024 * 1024
PROGRESS_MAX = 256 * 1024
LEDGER_MAX = 64 * 1024 * 1024   # per ledger file; beyond it the total is reported incomplete
LEDGER_TOTAL_MAX = 128 * 1024 * 1024   # over all ledger files of all checkouts
LEDGER_FILES_MAX = 64   # directory entries per ledger directory
WEIGHT_MAX = 1000   # mirrors checklist.WEIGHT_MAX
HOME_MAX = 4 * 1024
HOME_FRESH = 120     # seconds; the running workbench rewrites its snapshot at least every 60 s
LINE_MAX = 200
SEP = " \u00b7 "
SHA_RE = re.compile(r"^[0-9a-f]{7,40}$")  # evidence forms mirror tools/workbench/checklist.py
PR_RE = re.compile(r"^https://github\.com/KonomiYuzu01/Magic-600-Cell/pull/[1-9][0-9]{0,5}$")
PATH_RE = re.compile(r"^[A-Za-z0-9_.-]+(?:/[A-Za-z0-9_.-]+)*$")


def main_checkout(start: Path) -> Path | None:
    """Mirrors tools/workbench/paths.py."""
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


def checkouts(main: Path) -> list[Path]:
    found = [main]
    try:
        entries = sorted((main / ".git" / "worktrees").iterdir())
    except OSError:
        return found
    for entry in entries:
        try:
            tree = Path((entry / "gitdir").read_text(encoding="utf-8").strip()).parent
        except OSError:
            continue
        if tree.is_dir():
            found.append(tree.resolve())
    return found


def read_json(path: Path, limit: int = DOC_MAX):
    """A JSON document, or None when missing, invalid or over `limit` bytes."""
    try:
        with path.open("rb") as f:
            data = f.read(limit + 1)
        return None if len(data) > limit else json.loads(data.decode("utf-8"))
    except (OSError, ValueError, UnicodeDecodeError):
        return None


def clean(text, limit: int) -> str:
    """One line: control characters removed, whitespace collapsed, length capped."""
    text = re.sub(r"[\x00-\x1f\x7f]", " ", str(text))
    return " ".join(text.split())[:limit]


def evidence_ok(ref) -> bool:
    """Mirrors `checklist.evidence_kind(ref) is not None`."""
    if not isinstance(ref, str) or not ref or len(ref) > 200:
        return False
    if SHA_RE.fullmatch(ref) or PR_RE.fullmatch(ref):
        return True
    if PATH_RE.fullmatch(ref):
        parts = ref.split("/")
        return ".." not in parts and "." not in parts and parts[0].lower() != "work"
    return False


def step_percent(step) -> float | None:
    """Mirrors checklist.step_percent: done weight over total weight, None without a valid checklist."""
    items = step.get("items") if isinstance(step, dict) else None
    if not isinstance(items, list) or not items:
        return None
    total = done = 0
    for item in items:
        if not isinstance(item, dict):
            return None
        w = item.get("weight")
        if not isinstance(w, int) or isinstance(w, bool) or not 0 < w <= WEIGHT_MAX:
            return None
        total += w
        if item.get("done") is True and evidence_ok(item.get("evidence")):
            done += w
    return 100.0 * done / total


def home_counts(main: Path, now: float) -> tuple[int, int] | None:
    """(for you, gallery new) from a fresh workbench snapshot, else None."""
    doc = read_json(main / "work" / "loop-memory" / "workbench" / "home.json", HOME_MAX)
    if not isinstance(doc, dict) or doc.get("schema") != 1 or not isinstance(doc.get("written"), str):
        return None
    try:
        written = datetime.fromisoformat(doc["written"]).timestamp()
    except (ValueError, OverflowError, OSError):
        return None
    if not -60 <= now - written <= HOME_FRESH:
        return None
    counts = (doc.get("for_you"), doc.get("gallery_new"))
    if not all(isinstance(n, int) and not isinstance(n, bool) and 0 <= n < 1_000_000 for n in counts):
        return None
    return counts


def budget(roots: list) -> dict:
    """Cumulative paid API amounts over the complete ledgers, read up to their size at open time.
    A ledger that cannot be listed or read in full within the budgets marks the total incomplete."""
    paid, unknown, incomplete, read = {}, set(), False, 0
    for c in roots:
        directory = c / "work" / "loop-memory" / "ledgers"
        try:
            with os.scandir(directory) as it:
                names = [e.name for e in itertools.islice(it, LEDGER_FILES_MAX + 1)]
        except FileNotFoundError:
            continue
        except OSError:
            incomplete = True
            continue
        if len(names) > LEDGER_FILES_MAX:
            incomplete = True
            continue
        for name in sorted(n for n in names if n.endswith(".jsonl")):
            f = directory / name
            try:
                size = f.stat().st_size
                if size > LEDGER_MAX or read + size > LEDGER_TOTAL_MAX:
                    incomplete = True
                    continue
                read += size
                with f.open("rb") as fh:
                    data = fh.read(size)  # a line appended after stat() is left for the next refresh
            except FileNotFoundError:
                continue   # removed since the listing
            except OSError:
                incomplete = True
                continue
            for line in data.split(b"\n")[:-1]:
                if b"paid-api" not in line:
                    continue
                try:
                    r = json.loads(line.decode("utf-8"))
                except (ValueError, UnicodeDecodeError):
                    continue
                if not isinstance(r, dict) or r.get("channel") != "paid-api" or not isinstance(r.get("call_id"), str):
                    continue
                cost = r.get("cost") if isinstance(r.get("cost"), dict) else {}
                status, usd = cost.get("status"), cost.get("usd")
                if status in ("billed", "estimated", "reserved") and isinstance(usd, (int, float)) and not isinstance(usd, bool):
                    paid[(r["call_id"], status)] = float(usd)
                else:
                    unknown.add(r["call_id"])
    sums = {k: round(sum(v for (_, s), v in paid.items() if s == k), 2) for k in ("billed", "estimated", "reserved")}
    return {**sums, "unknown_calls": len(unknown), "incomplete": incomplete}


def line(main: Path, now: float | None = None) -> str:
    now = time.time() if now is None else now
    doc = read_json(main / "docs" / "progress" / "status.json", PROGRESS_MAX)
    step, ceiling = "step unknown", "?"
    if isinstance(doc, dict):
        if isinstance(doc.get("paid_api_ceiling_usd"), int):
            ceiling = str(doc["paid_api_ceiling_usd"])
        steps = doc.get("steps") if isinstance(doc.get("steps"), list) else []
        cur = next((s for s in steps if isinstance(s, dict) and s.get("id") == doc.get("current")), None)
        if cur:
            pct = step_percent(cur) if doc.get("schema") == 2 else None
            step = f"Step {clean(cur.get('id'), 12)} " + (f"{math.floor(pct)}%" if pct is not None else "no checklist")
    parts = [step]
    home = home_counts(main, now)
    parts.append(f"for you {home[0]}{SEP}gallery +{home[1]}" if home else "workbench closed")
    b = budget(checkouts(main))
    if b["incomplete"]:
        parts.append("API incomplete (a ledger was not read in full)")
    elif b["billed"] or b["estimated"] or b["reserved"] or b["unknown_calls"]:
        api = f"API ${b['billed']:.2f}/${ceiling}"
        extra = [f"est ${b['estimated']:.2f}" if b["estimated"] else "", f"res ${b['reserved']:.2f}" if b["reserved"] else "",
                 f"{b['unknown_calls']} unknown" if b["unknown_calls"] else ""]
        extra = [e for e in extra if e]
        if extra:
            api += " (" + ", ".join(extra) + ")"
        parts.append(api)
    return clean(SEP.join(parts), LINE_MAX)


def emit(text: str) -> None:
    """One UTF-8 line, whatever the console code page."""
    sys.stdout.buffer.write((text + "\n").encode("utf-8"))
    sys.stdout.flush()


def main() -> int:
    try:
        try:
            event = json.loads(sys.stdin.buffer.read(STDIN_MAX).decode("utf-8", "replace") or "{}")
        except (OSError, ValueError):
            event = {}
        start = None
        if isinstance(event, dict):
            ws = event.get("workspace") if isinstance(event.get("workspace"), dict) else {}
            start = ws.get("current_dir") or event.get("cwd")
        main_dir = (main_checkout(Path(start)) if isinstance(start, str) and start else None) \
            or main_checkout(Path(__file__).resolve().parent)
        emit(line(main_dir) if main_dir else "workbench: unavailable")
    except Exception:
        emit("workbench: unavailable")
    return 0


if __name__ == "__main__":
    sys.exit(main())
