"""SessionStart hook: print a bounded local project summary as session context.

It reads only repository files, Git metadata and whether Codex credentials
exist. It never calls models, installs tools, uses the network or reads
personal data.
"""
from __future__ import annotations

import importlib.util
import json
import os
import re
import shutil
import struct
import subprocess
import sys
from pathlib import Path

ROOT = Path(os.environ.get("CLAUDE_PROJECT_DIR") or Path(__file__).resolve().parents[2])


def git(*args: str) -> str:
    try:
        return subprocess.run(["git", *args], cwd=ROOT, capture_output=True, text=True, timeout=3).stdout.rstrip("\n")
    except (OSError, subprocess.SubprocessError):
        return ""


def current_phase() -> str:
    briefing = ROOT / "docs" / "development-guide" / "AGENT_BRIEFING.md"
    try:
        m = re.search(r"^Current phase:\s*(.+)$", briefing.read_text(encoding="utf-8"), re.M)
    except OSError:
        return "unknown (briefing missing)"
    return m.group(1).strip() if m else "unknown"


def wiki_log_tail(n: int = 5) -> list[str]:
    try:
        lines = (ROOT / "docs" / "wiki" / "log.md").read_text(encoding="utf-8").splitlines()
    except OSError:
        return []
    return [l for l in lines if l.startswith("## [")][-n:]


def open_findings() -> list[str]:
    out = []
    for meta_path in sorted((ROOT / "work" / "reviews").glob("*/meta.json"), reverse=True)[:5]:
        try:
            meta = json.loads(meta_path.read_text(encoding="utf-8"))
            if meta.get("verdict") != "findings":
                continue
            result = json.loads((meta_path.parent / "review.json").read_text(encoding="utf-8"))
            answered = {}
            disp = meta_path.parent / "dispositions.json"
            if disp.is_file():
                answered = json.loads(disp.read_text(encoding="utf-8"))
            pending = [f["id"] for f in result.get("findings", []) if f.get("id") not in answered]
            if pending:
                out.append(f"{meta['call_id']} ({meta['kind']}): unanswered {', '.join(pending[:6])}")
        except (OSError, ValueError, KeyError, TypeError):
            continue
    return out


def missing_tools() -> str:
    """Presence of planning tools from the lockfile data only; repository code is never imported or run."""
    try:
        lock = json.loads((ROOT / "tools" / "toolchain.lock.json").read_text(encoding="utf-8"))
        platform = {"win32": "windows", "darwin": "macos"}.get(sys.platform, "linux")
        bindir = "Scripts" if platform == "windows" else "bin"
        suffixes = [".exe", ".cmd", ""] if platform == "windows" else [""]
        missing = []
        for e in lock["tools"]:
            probe = e.get("probe")
            if "planning" not in e["profiles"] or platform not in e["platforms"] or not probe:
                continue
            name = probe[0]
            if name == "{python}":
                found = len(probe) < 3 or probe[1] != "-m" or importlib.util.find_spec(probe[2]) is not None
            elif name.startswith(("{venv}/", "{prefix}/")):
                base = (ROOT / e["venv"] / bindir if name.startswith("{venv}/") else ROOT / e["prefix"]) / name.split("/", 1)[1]
                found = any(base.with_name(base.name + s).is_file() for s in suffixes)
            else:
                found = shutil.which(name) is not None
            if not found:
                missing.append(e["id"])
        return ", ".join(missing) if missing else "none"
    except Exception:
        return "unknown"


def codex_login() -> str:
    try:
        home = Path(os.environ.get("CODEX_HOME") or Path.home() / ".codex")
        return "ready" if (home / "auth.json").is_file() else "not-ready"
    except (OSError, RuntimeError):
        return "unknown"


def main() -> int:
    try:
        sys.stdin.read()
    except OSError:
        pass
    lines = ["Magic 600 Cell session summary (local, bounded):"]
    if struct.calcsize("P") * 8 != 64:
        lines.append("- WARNING: hook interpreter is not 64-bit; hooks report inconclusive results.")
    lines.append(f"- Branch {git('rev-parse', '--abbrev-ref', 'HEAD') or '?'} at {git('rev-parse', '--short', 'HEAD') or '?'}")
    changed = [l for l in git("status", "--porcelain=v1").splitlines() if l.strip()]
    lines.append(f"- Uncommitted changes: {len(changed)} path(s)" + (": " + ", ".join(l[3:] for l in changed[:8]) if changed else ""))
    lines.append(f"- Current phase: {current_phase()}")
    findings = open_findings()
    lines.append("- Open review findings: " + ("; ".join(findings) if findings else "none recorded"))
    lines.append(f"- Planning tools missing: {missing_tools()} (install: python tools/toolchain/bootstrap.py install <id>)")
    lines.append(f"- Codex login: {codex_login()}")
    tail = wiki_log_tail()
    if tail:
        lines.append("- Recent wiki log:")
        lines += [f"  {l[3:]}" for l in tail]
    lines.append("Read docs/wiki/index.md before substantive work.")
    print("\n".join(lines))
    return 0


if __name__ == "__main__":
    sys.exit(main())
