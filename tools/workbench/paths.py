"""Checkout discovery and private workbench locations for the development workbench.

Everything here reads Git metadata files directly and never runs git, so the
workbench, the status line and the reporting hook agree on the same layout.
`.claude/hooks/report_event.py` and `tools/workbench/statusline.py` carry their
own copies of `main_checkout` (they import no repository code); tests keep the
copies equivalent.
"""
from __future__ import annotations

import os
import re
from pathlib import Path

SID_RE = re.compile(r"^[A-Za-z0-9-]{1,64}$")
CALL_ID_RE = re.compile(r"^\d{8}T\d{6}Z-[0-9a-f]{8}$")


def valid_sid(sid) -> bool:
    return isinstance(sid, str) and SID_RE.match(sid) is not None


def main_checkout(start: Path) -> Path | None:
    """The main checkout of the repository that contains `start` (a worktree resolves to its main checkout)."""
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


def worktrees(main: Path) -> list[Path]:
    """Registered worktrees of `main`, wherever they are on disk; missing ones are skipped."""
    found = []
    base = Path(main) / ".git" / "worktrees"
    try:
        entries = sorted(base.iterdir())
    except OSError:
        return found
    for entry in entries:
        try:
            gitfile = Path((entry / "gitdir").read_text(encoding="utf-8").strip())
        except OSError:
            continue
        tree = gitfile.parent
        if tree.is_dir():
            found.append(tree.resolve())
    return found


def checkouts(main: Path) -> list[Path]:
    return [Path(main).resolve(), *worktrees(main)]


def slug(path: Path) -> str:
    """Claude Code's project directory name for a working directory."""
    return re.sub(r"[^A-Za-z0-9]", "-", str(path))


def claude_projects_dir() -> Path:
    base = os.environ.get("CLAUDE_CONFIG_DIR")
    return (Path(base) if base else Path.home() / ".claude") / "projects"


def codex_home() -> Path:
    return Path(os.environ.get("CODEX_HOME") or Path.home() / ".codex")


def data_root(main: Path) -> Path:
    """Private workbench data: events, briefs and owner notes. Git ignores work/."""
    return Path(main) / "work" / "loop-memory" / "workbench"


def inside(path: Path, roots) -> bool:
    """True when `path` resolves inside one of `roots` (case-insensitive where the OS is)."""
    try:
        target = os.path.normcase(str(Path(path).resolve()))
    except OSError:
        return False
    for root in roots:
        base = os.path.normcase(str(Path(root).resolve()))
        if target == base or target.startswith(base.rstrip("\\/") + os.sep):
            return True
    return False
