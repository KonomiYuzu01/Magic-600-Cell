"""Content digests and the review source identity shared by project tooling.

Text files are hashed with CRLF normalized to LF so a Windows checkout with
automatic line-ending conversion produces the same digests as Linux.
"""
from __future__ import annotations

import hashlib
import json
import os
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

# Paths written by the review workflow itself. They never count as part of the
# candidate under review, so recording a review cannot invalidate it.
IDENTITY_EXCLUDES = ("work/reviews/", "work/loop-memory/", "docs/wiki/log.md")


def _is_link(p: Path) -> bool:
    """True for a symlink or a Windows junction (realpath resolves both, on every supported Python)."""
    real = os.path.normcase(os.path.realpath(p))
    return p.is_symlink() or real != os.path.normcase(os.path.join(os.path.realpath(p.parent), p.name))


def file_sha256(path: Path) -> str:
    data = Path(path).read_bytes()
    if b"\0" not in data:
        data = data.replace(b"\r\n", b"\n")
    return hashlib.sha256(data).hexdigest()


def tree_digest(directory: Path) -> str:
    """Digest of every file below a directory, independent of file order."""
    directory = Path(directory)
    h = hashlib.sha256()
    for path in [directory, *directory.rglob("*")]:
        if _is_link(path):
            raise ValueError(f"{path.name}: links are not allowed in a digested tree")
    # Sort by the POSIX relative path string: Path ordering is case-insensitive on
    # Windows, which would make the digest differ between platforms.
    files = {p.relative_to(directory).as_posix(): p for p in directory.rglob("*") if p.is_file()}
    for rel in sorted(files):
        h.update(f"{rel}\0{file_sha256(files[rel])}\n".encode())
    return h.hexdigest()


def files_digest(paths) -> str:
    """Digest of named files relative to ROOT; a missing file is part of the digest."""
    h = hashlib.sha256()
    for rel in sorted(paths):
        p = ROOT / rel
        h.update(f"{rel}\0{file_sha256(p) if p.is_file() else 'missing'}\n".encode())
    return h.hexdigest()


def _git(args, timeout: float) -> bytes:
    return subprocess.run(
        ["git", *args], cwd=ROOT, capture_output=True, check=True, timeout=timeout
    ).stdout


def changed_paths(timeout: float = 3.0) -> list[str]:
    """Staged, unstaged and untracked paths relative to HEAD, with rename sources."""
    raw = _git(["status", "--porcelain=v1", "-z", "--untracked-files=all"], timeout)
    parts = raw.decode("utf-8", "surrogateescape").split("\0")
    paths, i = [], 0
    while i < len(parts):
        entry = parts[i]
        i += 1
        if len(entry) < 4:
            continue
        status, path = entry[:2], entry[3:]
        paths.append(path)
        if "R" in status or "C" in status:
            paths.append(parts[i])  # rename or copy source
            i += 1
    return sorted(set(p for p in paths if not p.startswith(IDENTITY_EXCLUDES)))


def source_identity(timeout: float = 3.0) -> dict:
    """Identify the exact candidate: base commit, plus the working-tree content, mode and index entry of every change."""
    head = _git(["rev-parse", "HEAD"], timeout).decode().strip()
    paths = changed_paths(timeout)
    staged = {}
    if paths:
        # Index entries (mode and blob id) so that staged content is part of the identity:
        # a review of the working tree must not certify different staged content.
        wanted = set(paths)
        raw = _git(["ls-files", "--stage", "-z"], timeout).decode("utf-8", "surrogateescape")
        for rec in filter(None, raw.split("\0")):
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
            mode = "x" if os.access(p, os.X_OK) else "f"
            items.append([rel, mode, file_sha256(p), sorted(staged.get(rel, []))])
        else:
            items.append([rel, "deleted", None, sorted(staged.get(rel, []))])
    blob = json.dumps({"head": head, "changes": items}, sort_keys=True).encode()
    return {"head": head, "paths": [i[0] for i in items], "digest": hashlib.sha256(blob).hexdigest()}
