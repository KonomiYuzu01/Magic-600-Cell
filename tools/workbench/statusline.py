"""Claude Code status line for Magic 600 Cell: current step, open findings, Codex login, paid API budget.

Claude Code runs this on every status-line refresh in every session, so it is a
critical path: standard library only, no repository imports, bounded reads, no
writes, no network, no subprocesses. It prints exactly one line of at most
LINE_MAX characters; any error prints `workbench: unavailable`. Its figures match
tools/workbench/sources.py (tests compare both on fixtures).
"""
from __future__ import annotations

import json
import os
import re
import sys
from pathlib import Path

STDIN_MAX = 64 * 1024
DOC_MAX = 2 * 1024 * 1024
PROGRESS_MAX = 256 * 1024
LEDGER_MAX = 64 * 1024 * 1024   # per ledger file; beyond it the total is reported incomplete
REVIEWS_PER_CHECKOUT = 5
LINE_MAX = 200


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


def open_findings(roots: list) -> int:
    count = 0
    for c in roots:
        for meta_path in sorted((c / "work" / "reviews").glob("*/meta.json"), reverse=True)[:REVIEWS_PER_CHECKOUT]:
            meta = read_json(meta_path)
            if not isinstance(meta, dict) or meta.get("verdict") != "findings":
                continue
            result = read_json(meta_path.parent / "review.json")
            answered = read_json(meta_path.parent / "dispositions.json")
            answered = answered if isinstance(answered, dict) else {}
            if isinstance(result, dict) and isinstance(result.get("findings"), list):
                count += sum(1 for f in result["findings"] if isinstance(f, dict) and f.get("id") not in answered)
    return count


def budget(roots: list) -> dict:
    """Cumulative paid API amounts over the complete ledgers, read up to their size at open time."""
    paid, unknown, incomplete = {}, set(), False
    for c in roots:
        for f in sorted((c / "work" / "loop-memory" / "ledgers").glob("*.jsonl")):
            try:
                size = f.stat().st_size
                if size > LEDGER_MAX:
                    incomplete = True
                    continue
                with f.open("rb") as fh:
                    data = fh.read(size)  # a line appended after stat() is left for the next refresh
            except OSError:
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


def codex_login() -> str:
    try:
        home = Path(os.environ.get("CODEX_HOME") or Path.home() / ".codex")
        return "ready" if (home / "auth.json").is_file() else "not-ready"
    except (OSError, RuntimeError):
        return "unknown"


def line(main: Path) -> str:
    roots = checkouts(main)
    doc = read_json(main / "docs" / "progress" / "status.json", PROGRESS_MAX)
    step, ceiling = "step unknown", "?"
    if isinstance(doc, dict):
        if isinstance(doc.get("paid_api_ceiling_usd"), int):
            ceiling = str(doc["paid_api_ceiling_usd"])
        steps = doc.get("steps") if isinstance(doc.get("steps"), list) else []
        cur = next((s for s in steps if isinstance(s, dict) and s.get("id") == doc.get("current")), None)
        if cur:
            step = f"{clean(cur.get('id'), 12)} {clean(cur.get('title'), 60)} ({clean(str(cur.get('status', '')).replace('_', ' '), 16)})"
    b = budget(roots)
    if b["incomplete"]:
        api = "API incomplete (ledger over read limit)"
    else:
        api = f"API ${b['billed']:.2f}/${ceiling}"
        extra = [f"est ${b['estimated']:.2f}" if b["estimated"] else "", f"res ${b['reserved']:.2f}" if b["reserved"] else "",
                 f"{b['unknown_calls']} unknown" if b["unknown_calls"] else ""]
        extra = [e for e in extra if e]
        if extra:
            api += " (" + ", ".join(extra) + ")"
    return clean(f"{step} | findings {open_findings(roots)} | Codex {codex_login()} | {api}", LINE_MAX)


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
        print(line(main_dir) if main_dir else "workbench: unavailable")
    except Exception:
        print("workbench: unavailable")
    return 0


if __name__ == "__main__":
    sys.exit(main())
