"""Copy the clean, tracked Taste Lab page into a local public checkout."""

import argparse
import os
import stat
import subprocess
import sys
import tempfile
from collections import Counter
from dataclasses import dataclass
from pathlib import Path, PurePosixPath

ROOT = Path(__file__).resolve().parents[2]
SOURCE_PATHS = ("tools/tastelab/index.html", "tools/tastelab/page/", "tools/tastelab/core/")


class Refused(ValueError):
    pass


@dataclass(frozen=True)
class Plan:
    changes: tuple[tuple[str, str], ...]
    unchanged: int
    directories: tuple[str, ...]


def _git(root, *args):
    return subprocess.run(["git", "--no-optional-locks", "-C", str(root), *args],
                          check=True, capture_output=True).stdout


def tracked_source_paths(root):
    paths = _git(root, "ls-files", "-z", "--", *SOURCE_PATHS)
    return tuple(PurePosixPath(os.fsdecode(path)).relative_to("tools/tastelab").as_posix()
                 for path in paths.split(b"\0") if path)


def _lstat(path):
    try:
        info = os.lstat(path)
    except FileNotFoundError:
        return None
    if stat.S_ISLNK(info.st_mode) or (hasattr(Path, "is_junction") and path.is_junction()):
        raise Refused(f"Symbolic links and junctions are not allowed: {path}")
    return info


def read_source(root):
    dirty = _git(root, "status", "--porcelain=v1", "-z", "--", *SOURCE_PATHS)
    if dirty:
        names = []
        records = iter(dirty.split(b"\0"))
        for record in records:
            if record:
                names.append(os.fsdecode(record[3:]))
                if b"R" in record[:2] or b"C" in record[:2]:
                    names.append(os.fsdecode(next(records)))
        raise Refused("Source must be clean: " + ", ".join(sorted(set(names))))
    paths = tracked_source_paths(root)
    if not paths:
        raise Refused("The source set is empty")
    commit = _git(root, "rev-parse", "--verify", "HEAD").decode("ascii").strip()
    source = {}
    for name in paths:
        path = root / "tools" / "tastelab" / name
        info = _lstat(path)
        if info is None or not stat.S_ISREG(info.st_mode):
            raise Refused(f"Expected a source file: {path}")
        source[name] = path.read_bytes()
    return commit, source


def validate_target(target, root):
    target = target.resolve()
    if target.is_relative_to(root.resolve()):
        raise Refused("The target must not be the source repository or inside it")
    if not target.is_dir():
        raise Refused("The target must be an existing directory")
    for name in (".git", "LICENSE", ".nojekyll"):
        if not (target / name).exists():
            raise Refused(f"The target is missing {name}")
    if not ((target / ".git").is_dir() or (target / ".git").is_file()):
        raise Refused("The target .git must be a directory or file")
    return target


def _target_tree(target):
    files, directories = set(), []

    def visit(directory):
        info = _lstat(directory)
        if info is None:
            return
        if not stat.S_ISDIR(info.st_mode):
            raise Refused(f"Expected a managed directory: {directory}")
        directories.append(directory.relative_to(target).as_posix())
        for path in sorted(directory.iterdir()):
            info = _lstat(path)
            if stat.S_ISDIR(info.st_mode):
                visit(path)
            elif stat.S_ISREG(info.st_mode):
                files.add(path.relative_to(target).as_posix())
            else:
                raise Refused(f"Expected a managed file: {path}")

    index = target / "index.html"
    info = _lstat(index)
    if info is not None:
        if not stat.S_ISREG(info.st_mode):
            raise Refused(f"Expected a managed file: {index}")
        files.add("index.html")
    for name in ("page", "core"):
        visit(target / name)
    return files, directories


def compute_plan(source, target):
    """Compare bytes and validate all managed links without changing the target."""
    if not source:
        raise Refused("The source set is empty")
    files, directories = _target_tree(target)
    changes, unchanged = [], 0
    for name, data in source.items():
        path = target / name
        info = _lstat(path)
        if info is None:
            changes.append(("added", name))
        elif not stat.S_ISREG(info.st_mode):
            raise Refused(f"Expected a managed file: {path}")
        elif path.read_bytes() != data:
            changes.append(("changed", name))
        else:
            unchanged += 1
    changes.extend(("removed", name) for name in files - source.keys())
    return Plan(tuple(sorted(changes, key=lambda change: change[1])), unchanged, tuple(directories))


def apply_plan(source, target, plan):
    """Apply a validated plan with an atomic replacement for each written file."""
    for action, name in plan.changes:
        path = target / name
        if action == "removed":
            path.unlink()
            continue
        path.parent.mkdir(parents=True, exist_ok=True)
        temporary = None
        try:
            with tempfile.NamedTemporaryFile(dir=path.parent, prefix=f".{path.name}.", delete=False) as stream:
                temporary = Path(stream.name)
                stream.write(source[name])
            os.replace(temporary, path)
        finally:
            if temporary is not None:
                temporary.unlink(missing_ok=True)
    for name in reversed(plan.directories):
        directory = target / name
        if not any(directory.iterdir()):
            directory.rmdir()


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__, allow_abbrev=False)
    parser.add_argument("--target", type=Path, required=True)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args(argv)
    try:
        target = validate_target(args.target, ROOT)
        commit, source = read_source(ROOT)
        plan = compute_plan(source, target)
        if not args.check:
            apply_plan(source, target, plan)
        print(f"source {commit}")
        for action, name in plan.changes:
            print(f"{action} {name}")
        counts = Counter(action for action, _ in plan.changes)
        print(f"added {counts['added']} changed {counts['changed']} removed {counts['removed']} unchanged {plan.unchanged}")
        return 1 if args.check and plan.changes else 0
    except (Refused, OSError, subprocess.CalledProcessError) as error:
        print(f"Refused: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
