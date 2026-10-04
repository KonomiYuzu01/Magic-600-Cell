"""Private checkout-local locations and input guards; no network or Git lookup."""
from __future__ import annotations

import os
import re
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PACKAGE = Path(__file__).resolve().parent
PRIVATE_BASE = ROOT / "work" / "loop-memory" / "tastelab"
USER_AGENT = "Magic600-TasteLab/1.0 (+https://github.com/KonomiYuzu01/Magic-600-Cell)"
MAX_IMAGES = 20_000
MAX_BYTES = 10 * 1024 ** 3
ORIGINAL_MAX_BYTES = 2 * 1024 ** 2
ORIGINAL_SHARE = 0.8
THUMB_EDGE = 512
TIERS = ("A", "B")
VERDICTS = ("like", "dislike", "skip")
NOTE_MAX = 300
SESSION_MARKERS = ("session.sqlite3", "engine.lock")


class Refused(Exception):
    """An input or location that Taste Lab must not use."""


def now_iso() -> str:
    return datetime.now(timezone.utc).isoformat(timespec="seconds").replace("+00:00", "Z")


def local_day(iso: str | None = None) -> str:
    moment = datetime.now(timezone.utc) if iso is None else datetime.fromisoformat(iso.replace("Z", "+00:00"))
    return moment.astimezone().date().isoformat()


def valid_sha(sha) -> bool:
    return isinstance(sha, str) and re.fullmatch(r"[0-9a-f]{64}", sha) is not None


def _is_link(path: Path) -> bool:
    try:
        return path.is_symlink() or path.is_junction()
    except OSError:
        return True


def check_input(path, *, kind="input") -> Path:
    """Check without resolving links or creating anything, including missing paths."""
    path = Path(os.path.abspath(Path(path).expanduser()))
    for part in (path, *path.parents):
        if _is_link(part):
            raise Refused(f"{kind}: path passes through a link or junction")
        if any((part / marker).exists() for marker in SESSION_MARKERS):
            raise Refused(f"{kind}: path is inside a session directory")
    return path


def default_data_root() -> Path:
    return PRIVATE_BASE / "library"


def data_root(override=None) -> Path:
    """Both roots use the same guard before a caller creates directories or opens SQLite."""
    path = Path(os.path.abspath(default_data_root() if override is None else Path(override).expanduser()))
    base = Path(os.path.abspath(PRIVATE_BASE))
    if path == base or not path.is_relative_to(base):
        raise Refused("data folder: must be a folder inside work/loop-memory/tastelab/")
    return check_input(path, kind="data folder")


def load_yaml(path: Path):
    import yaml
    with open(path, encoding="utf-8") as fh:
        return yaml.safe_load(fh)
