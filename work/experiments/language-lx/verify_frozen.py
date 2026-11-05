"""Run an experiment's check against the commit it was frozen on.

Some experiments pin chapters, cards or decision pages by digest. Those documents keep moving after the freeze, so the
folder's own check rightly refuses to run at a later commit: an old result cannot be relabelled. This script reproduces the
run instead. It makes a temporary local clone at the frozen commit, lays the committed experiment folder over it, runs the
check there and removes the clone. It fails if the check changed any file of the folder.

Usage, from the repository root:

    python work/experiments/language-lx/verify_frozen.py <folder> [<script inside the folder> [args...]]

The default script is check.py. Set LX6_ENGINE_PYTHON to the pinned engine interpreter for lx6-engine-port. The frozen commit
must stay reachable, so the branch that carries these experiments is merged with a merge commit, never squashed.
"""
import hashlib
import os
from pathlib import Path
import shutil
import stat
import subprocess
import sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
BASES = {
    "envelope-facts": "1e7d0074df96cbeb05f55282cf8f6c03fbf4a4ff",
    "lx6-engine-port": "1e7d0074df96cbeb05f55282cf8f6c03fbf4a4ff",
    "lx7-charts": "1e7d0074df96cbeb05f55282cf8f6c03fbf4a4ff",
}
SCRATCH = ROOT / "work" / "lxv"


def git(*args, cwd=ROOT):
    subprocess.run(["git", *args], cwd=cwd, check=True, capture_output=True)


def force_remove(function, path, _):
    os.chmod(path, stat.S_IWRITE)
    function(path)


def snapshot(folder):
    return {path.relative_to(folder).as_posix(): hashlib.sha256(path.read_bytes()).hexdigest()
            for path in sorted(folder.rglob("*")) if path.is_file()}


def main():
    if not 2 <= len(sys.argv) or sys.argv[1] not in BASES:
        sys.exit(__doc__)
    name, script = sys.argv[1], (sys.argv[2:] or ["check.py"])
    if SCRATCH.exists():
        sys.exit(f"{SCRATCH} exists: remove it first")
    clone = SCRATCH / name
    try:
        clone.parent.mkdir(parents=True)
        git("clone", "--quiet", "--no-checkout", "-c", "core.longpaths=true", "-c", "core.autocrlf=false", str(ROOT), str(clone))
        git("checkout", "--quiet", "--detach", BASES[name], cwd=clone)
        relative = (HERE / name).relative_to(ROOT)
        shutil.copytree(HERE / name, clone / relative, dirs_exist_ok=True)
        before = snapshot(clone / relative)
        command = [sys.executable, "-B", (relative / script[0]).as_posix(), *script[1:]]
        print("Frozen commit", BASES[name], "running", " ".join(command), flush=True)
        result = subprocess.run(command, cwd=clone)
        after = snapshot(clone / relative)
        changed = sorted(k for k in before.keys() | after.keys() if before.get(k) != after.get(k))
        if changed:
            print("Files changed by the check:", changed[:20], len(changed))
        print("Exit", result.returncode, "- folder unchanged" if not changed else "- folder CHANGED")
        sys.exit(result.returncode or (1 if changed else 0))
    finally:
        shutil.rmtree(SCRATCH, onexc=force_remove)


if __name__ == "__main__":
    main()
