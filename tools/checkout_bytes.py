"""Check, and optionally restore, the exact committed bytes of a working tree.

`.gitattributes` disables line-ending conversion, so every checkout must hold
the committed blob bytes: model assets and build inputs are hashed raw. A
working tree checked out before that rule may still hold CRLF copies of LF
files, and Git may keep listing restored files as modified until their index
entries are refreshed.

`--fix` repairs only those two cases. A file qualifies only if it is a regular
file reached without links, its blob is text with LF endings only, and its
bytes are exactly that blob with every LF written as CRLF (or already equal to
the blob while Git lists it as modified). Each file is moved into a backup
directory inside the Git directory, checked again, and written back from the
index with `git checkout-index`, which never overwrites a file saved in the
meantime. The tool never deletes or stages anything: every original stays in
the backup directory, whose location is printed, until the owner removes it.
Any other difference is reported and left alone, and a tree with unmerged
entries or with line-ending conversion still enabled is refused.

Usage: python tools/checkout_bytes.py [--fix]
"""
from __future__ import annotations

import argparse
import datetime
import os
import posixpath
import stat
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def _git(root: Path, *args: str, **kw) -> bytes:
    return subprocess.run(["git", *args], cwd=root, capture_output=True, check=True, **kw).stdout


def index_entries(root: Path):
    """Return ({path: (mode, blob)} for stage-0 regular files, sorted unmerged paths)."""
    entries, unmerged = {}, set()
    for entry in filter(None, _git(root, "ls-files", "-s", "-z").split(b"\0")):
        meta, path = entry.split(b"\t", 1)
        mode, blob, stage = meta.split()
        name = path.decode("utf-8")
        if stage != b"0":
            unmerged.add(name)
        elif mode in (b"100644", b"100755"):
            entries[name] = (mode.decode(), blob.decode())
    return entries, sorted(unmerged)


def read_blobs(root: Path, blobs):
    proc = subprocess.Popen(["git", "cat-file", "--batch"], cwd=root, stdin=subprocess.PIPE, stdout=subprocess.PIPE)
    try:
        for blob in blobs:
            proc.stdin.write(blob.encode() + b"\n")
            proc.stdin.flush()
            header = proc.stdout.readline().split()
            if len(header) != 3 or header[1] != b"blob":
                raise RuntimeError(f"git cat-file could not read blob {blob}")
            data = proc.stdout.read(int(header[2]))
            proc.stdout.read(1)
            yield blob, data
    finally:
        proc.stdin.close()
        proc.stdout.close()
        proc.wait()


def modified(root: Path) -> set:
    """Paths Git reports as modified in the working tree."""
    out = _git(root, "status", "--porcelain=v1", "-z", "--untracked-files=no", "--no-renames")
    return {e[3:].decode("utf-8") for e in out.split(b"\0") if e[1:2] == b"M"}


def _inside(root: Path, path: str) -> str:
    """Where root/path is when nothing below the checkout root is a link.

    The root itself is resolved first, so a root reached through an 8.3 short name
    (C:\\Users\\RUNNER~1) or a link above the checkout is not taken for a link inside it.
    """
    return os.path.normcase(os.path.join(os.path.realpath(root), *[part for part in path.split("/") if part]))


def plain_file(root: Path, path: str) -> bool:
    """A regular file whose path below the checkout root passes through no symlink or junction."""
    p = root / path
    try:
        if not stat.S_ISREG(os.lstat(p).st_mode):
            return False
    except OSError:
        return False
    return os.path.normcase(os.path.realpath(p)) == _inside(root, path)


def converted(raw: bytes, blob: bytes) -> bool:
    """True if raw is exactly an LF-only text blob with every LF written as CRLF."""
    return b"\0" not in blob and b"\r" not in blob and b"\n" in blob and raw == blob.replace(b"\n", b"\r\n")


def classify(root: Path, entries: dict):
    """Return (repairs, other): repairs maps a path to (current bytes, blob bytes, executable bit)."""
    filemode = _git(root, "config", "--bool", "--default", "true", "core.filemode").strip() == b"true"
    stale = modified(root)
    wanted, other = {}, []
    for path, (mode, blob) in entries.items():
        if plain_file(root, path):
            wanted.setdefault(blob, []).append((path, mode))
        else:
            other.append(path)
    repairs = {}
    for blob, data in read_blobs(root, wanted):
        for path, mode in wanted[blob]:
            raw = (root / path).read_bytes()
            executable = _executable(root / path)
            if filemode and executable != (mode == "100755"):
                if path in stale:
                    other.append(path)  # a mode change is an edit
            elif raw == data:
                if path in stale:
                    repairs[path] = (raw, data, executable)
            elif converted(raw, data):
                repairs[path] = (raw, data, executable)
            else:
                other.append(path)
    return repairs, sorted(other)


def _executable(path: Path) -> bool:
    return bool(os.lstat(path).st_mode & 0o111)


def backup_root(root: Path) -> Path:
    git_dir = Path(_git(root, "rev-parse", "--absolute-git-dir").decode("utf-8").strip())
    stamp = datetime.datetime.now(datetime.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    parent = git_dir / "checkout-bytes"
    parent.mkdir(exist_ok=True)
    return Path(tempfile.mkdtemp(prefix=stamp + "-", dir=parent))  # created exclusively: never shared by two runs


def _put_back(kept: Path, target: Path) -> bool:
    """Put the kept original back only if nothing is at the target; never replace a file."""
    try:
        if os.name == "nt":
            os.rename(kept, target)  # fails if the target exists
        else:
            os.link(kept, target)    # fails if the target exists; the backup name is kept as well
        return True
    except OSError:
        return False


def restore(root: Path, backups: Path, path: str, expected: bytes, blob: bytes, executable: bool) -> str | None:
    """Rewrite one file from the index; return None on success or the reason it was left alone.

    The original is moved into the backup directory and never deleted, so a write that reaches it
    by any route after the checks below is still kept there."""
    target = root / path
    kept = backups / path
    if not plain_file(root, path):
        return "no longer a regular file inside the checkout"  # rechecked just before the move
    kept.parent.mkdir(parents=True, exist_ok=True)
    try:
        os.rename(target, kept)  # the backup path is new for this run
    except OSError as exc:
        return f"could not move it to the backup directory ({type(exc).__name__})"
    try:
        if os.path.normcase(os.path.realpath(target.parent)) != _inside(root, posixpath.dirname(path)):
            reason = "its directory changed while the repair ran"
        elif kept.read_bytes() != expected or _executable(kept) != executable:
            reason = "changed while the repair ran"
        else:
            # Without --force, checkout-index refuses to replace a file that was saved in the meantime.
            r = subprocess.run(["git", "checkout-index", "-u", "--", path], cwd=root, capture_output=True, text=True)
            if r.returncode != 0:
                reason = "git checkout-index failed: " + (r.stderr.strip().splitlines() or ["no message"])[-1]
            else:
                # Git has written the target: from here on nothing is moved back, so the original
                # stays in the backup whatever happens to the target or its directory.
                try:
                    return None if target.read_bytes() == blob else "Git wrote different bytes; the original is kept in the backup directory"
                except OSError as exc:
                    return f"{type(exc).__name__} after checkout; the original is kept in the backup directory"
    except OSError as exc:
        reason = f"{type(exc).__name__}: {exc}"
    if _put_back(kept, target):
        return reason
    return f"{reason}; the original is kept in the backup directory"


def still_converted(root: Path, paths) -> list[str]:
    """Paths for which Git would still convert line endings on checkout."""
    if not paths:
        return []
    out = subprocess.run(["git", "check-attr", "-z", "--stdin", "text"], cwd=root, capture_output=True, check=True,
                         input=b"\0".join(p.encode("utf-8") for p in paths) + b"\0").stdout.split(b"\0")
    return [out[i].decode("utf-8") for i in range(0, len(out) - 2, 3) if out[i + 2] != b"unset"]


def main(argv=None, root: Path = ROOT) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--fix", action="store_true", help="restore files that differ from their blob only by line-ending conversion")
    args = parser.parse_args(argv)
    entries, unmerged = index_entries(root)
    if unmerged:
        for path in unmerged:
            print("unmerged: " + path)
        print("Resolve the merge first; nothing was changed.")
        return 2
    repairs, other = classify(root, entries)
    if args.fix:
        converting = still_converted(root, sorted(repairs))
        if converting:
            for path in converting:
                print("line-ending conversion still enabled: " + path)
            print("Add the repository's .gitattributes (`* -text`) first; nothing was changed.")
            return 2
    failed = 0
    backups = backup_root(root) if args.fix and repairs else None
    for path in sorted(repairs):
        if not args.fix:
            print("needs repair: " + path)
            continue
        reason = restore(root, backups, path, *repairs[path])
        if reason:
            failed += 1
            print(f"not repaired: {path} ({reason})")
        else:
            print("restored " + path)
    for path in other:
        print("left unchanged (other edits, links or missing): " + path)
    done = len(repairs) - failed
    print(f"{done if args.fix else len(repairs)} file(s) {'restored' if args.fix else 'need repair'}; "
          f"{failed} not repaired; {len(other)} other difference(s)")
    if backups is not None and backups.exists():
        print(f"Originals are kept in {backups}; delete that directory once the checkout looks right.")
    return 1 if failed or (repairs and not args.fix) else 0


if __name__ == "__main__":
    sys.exit(main())
