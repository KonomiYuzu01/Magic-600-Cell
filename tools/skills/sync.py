"""Generate .claude/skills from the canonical .agents/skills and check both against tools/skills.lock.json.

  python tools/skills/sync.py               copy every locked skill to .claude/skills
  python tools/skills/sync.py --check       verify sources and copies without writing
  python tools/skills/sync.py --update-lock record new digests for project and owner skills
"""
from __future__ import annotations

import argparse
import json
import os
import shutil
import sys
import types
from pathlib import Path

def _load_source(name: str, path: Path):
    """Import a helper from its source text only; cached bytecode is never trusted."""
    module = types.ModuleType(name)
    module.__file__ = str(path)
    sys.modules[name] = module
    exec(compile(path.read_bytes(), str(path), "exec"), module.__dict__)
    return module


_digest = _load_source("repo_digest", Path(os.path.abspath(__file__)).parents[1] / "repo_digest.py")
ROOT, tree_digest = _digest.ROOT, _digest.tree_digest

LOCK = ROOT / "tools" / "skills.lock.json"
SOURCE = ROOT / ".agents" / "skills"
COPIES = ROOT / ".claude" / "skills"


def contained(path: Path) -> Path:
    """Refuse a write or removal target that is, or sits below, a link, or that leaves the repository."""
    base = ROOT.resolve()
    rel = path.relative_to(ROOT)
    probe = base
    for part in rel.parts:
        probe = probe / part
        if probe.is_symlink() or getattr(os.path, "isjunction", lambda _p: False)(probe) or (probe.exists() and probe.resolve() != probe):
            raise SystemExit(f"{rel.as_posix()}: passes through a link; refusing to write")
    return probe


def load_lock() -> dict:
    return json.loads(LOCK.read_text(encoding="utf-8"))


def check(lock: dict) -> list[str]:
    errors = []
    for name, entry in sorted(lock["skills"].items()):
        src, dst = SOURCE / name, COPIES / name
        if not src.is_dir():
            errors.append(f"{name}: missing source {src.relative_to(ROOT).as_posix()}")
            continue
        try:
            if tree_digest(src) != entry["digest"]:
                errors.append(f"{name}: source digest differs from the lockfile")
            if not dst.is_dir() or tree_digest(dst) != tree_digest(src):
                errors.append(f"{name}: .claude/skills copy is missing or stale")
        except ValueError as exc:
            errors.append(f"{name}: {exc}")
    unlocked = {p.name for p in SOURCE.iterdir() if p.is_dir()} - set(lock["skills"])
    errors += [f"{name}: present in .agents/skills but not locked" for name in sorted(unlocked)]
    if COPIES.is_dir():
        stray = {p.name for p in COPIES.iterdir() if p.is_dir()} - set(lock["skills"])
        errors += [f"{name}: .claude/skills copy has no locked source" for name in sorted(stray)]
    return errors


def sync(lock: dict) -> None:
    for name, entry in sorted(lock["skills"].items()):
        src = contained(SOURCE / name)
        try:
            digest = tree_digest(src)
        except ValueError as exc:
            raise SystemExit(f"{name}: {exc}; refusing to copy")
        if digest != entry["digest"]:
            raise SystemExit(f"{name}: source digest differs from the lockfile; run --update-lock after review")
        contained(COPIES / name)
    contained(COPIES).mkdir(parents=True, exist_ok=True)
    for name in sorted(lock["skills"]):
        src, dst = SOURCE / name, contained(COPIES / name)
        if dst.exists():
            shutil.rmtree(dst)
        shutil.copytree(src, dst, symlinks=True)


def update_lock(lock: dict) -> None:
    for name, entry in lock["skills"].items():
        if entry["origin"] == "third-party":
            continue  # changes only through bootstrap.py install-skill at a new pinned commit
        try:
            entry["digest"] = tree_digest(contained(SOURCE / name))
        except ValueError as exc:
            raise SystemExit(f"{name}: {exc}")
    contained(LOCK).write_text(json.dumps(lock, indent=2) + "\n", encoding="utf-8")


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--check", action="store_true")
    mode.add_argument("--update-lock", action="store_true")
    args = parser.parse_args(argv)
    lock = load_lock()
    if args.update_lock:
        update_lock(lock)
        return 0
    if not args.check:
        sync(lock)
    errors = check(lock)
    for e in errors:
        print(e)
    print("skills: ok" if not errors else f"skills: {len(errors)} problem(s)")
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
