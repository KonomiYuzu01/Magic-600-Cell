"""Record checklist progress in docs/progress/status.json (schema 2) with evidence.

  python tools/workbench/progress.py done <step> <item> --evidence <ref>
  python tools/workbench/progress.py status <step> not_started|in_progress|blocked|done
  python tools/workbench/progress.py check
  python tools/workbench/progress.py track <track> --title <text>
  python tools/workbench/progress.py step <step> --track <track> --title <text> --item <id>:<weight>:<title> [--item ...]
                                         [--acceptance <text> --acceptance-source <path>] [--weight <n>]
  python tools/workbench/progress.py item <step> <item> --title <text> [--weight <n>]
  python tools/workbench/progress.py current <step>

Sessions run this in their own checkout and commit the change normally; the
workbench never runs it. It edits the status.json of the checkout that contains
the current directory (a worktree, or the main checkout).

Evidence is a commit sha that resolves in that checkout, a pull request URL of
this repository, or a repository-relative path to an existing test or result
file (not under work/). `done` also moves a not_started step to in_progress.
`status done` needs every item done; `status not_started` needs none done. Both
set `updated` to today's UTC date, validate the whole resulting document, and
keep the file layout (tools/workbench/checklist.py `dump`) and its line endings;
a refused command leaves the file unchanged.

Parallel work gets its own track. `track` adds one (the first `track` writes
the default tracks into the file before it). `step` appends a not_started step
with at least one item; acceptance text needs a repository document as its
source. `item` appends an item; it moves a done step back to in_progress.
`current` names the step the status line shows. Titles and acceptance text are
one line of public English text: no CJK characters and no local paths.

Exit codes: 0 ok, 2 invalid input or an invalid file.
"""
from __future__ import annotations

import argparse
import functools
import json
import os
import re
import subprocess
import sys
import tempfile
import time
from contextlib import contextmanager
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import checklist  # noqa: E402
import paths  # noqa: E402

STATUS = Path("docs") / "progress" / "status.json"
STATUS_MAX = 256 * 1024


ACCEPTANCE_MAX = 600
# Local paths that must never reach the public status.json: a drive path (C:\ or C:/; a URL scheme
# is no drive), a UNC path (\\server\share), a home-relative path (~/ or ~\), an environment
# reference to the home directory, and any absolute POSIX path (a "/" that starts a word). Relative
# repository references ("docs/x.md", "0.4/0.4.1") and URLs ("https://...") stay allowed.
PRIVATE_RE = re.compile(r"(?<![A-Za-z])[A-Za-z]:[\\/]|\\\\[^\\\s]|~[\\/]|%USERPROFILE%|\$HOME\b|/Users/|/home/|\\\\Users"
                        r"|(?:^|(?<=[\s(\[\"'=,;]))/(?=[^\s/])")
LOCK_NAME = "magic600-progress.lock"
LOCK_WAIT = 10.0
_CJK_RE = re.compile(r"[\u3040-\u30ff\u3400-\u9fff\uac00-\ud7af]")


class ProgressRejected(Exception):
    pass


class _Parser(argparse.ArgumentParser):
    def error(self, message):
        raise ProgressRejected(message)


def checkout_root(start: Path) -> Path | None:
    """The checkout (main or worktree) that contains `start`: the deepest of paths.checkouts(main)."""
    main_dir = paths.main_checkout(start)
    if main_dir is None:
        return None
    roots = [root for root in paths.checkouts(main_dir) if paths.inside(start, [root])]
    return max(roots, key=lambda root: len(root.resolve().parts)).resolve() if roots else None


def evidence_problem(root: Path, ref: str) -> str | None:
    """None when `ref` is accepted evidence in checkout `root`, else the reason: the form
    (checklist.evidence_kind), a sha that `git rev-parse --verify --quiet <sha>^{commit}` resolves
    in `root`, or a path that is an existing regular file inside `root` (paths.inside)."""
    kind = checklist.evidence_kind(ref)
    if kind is None:
        return "unaccepted evidence form"
    if kind == "sha":
        try:
            result = subprocess.run(["git", "-C", str(root), "rev-parse", "--verify", "--quiet", f"{ref}^{{commit}}"],
                                    timeout=30, capture_output=True)
        except (OSError, subprocess.TimeoutExpired):
            return f"cannot resolve commit evidence: {ref}"
        if result.returncode != 0:
            return f"commit evidence does not resolve: {ref}"
    if kind == "path" and (not paths.inside(root / ref, [root]) or not (root / ref).is_file()):
        return f"evidence file is missing or outside the checkout: {ref}"
    return None


def _confined(root: Path) -> None:
    # Both the file and its directory, where the temporary file is created and replaced.
    if not (paths.within(root / STATUS, root) and paths.within((root / STATUS).parent, root)):
        raise ProgressRejected("status.json leads outside the checkout")


def _read(root: Path) -> tuple[dict, bytes]:
    _confined(root)
    try:
        with (root / STATUS).open("rb") as f:
            raw = f.read(STATUS_MAX + 1)
    except OSError as e:
        raise ProgressRejected("cannot read status.json") from e
    if len(raw) > STATUS_MAX:
        raise ProgressRejected(f"status.json is larger than {STATUS_MAX} bytes")
    try:
        doc = json.loads(raw.decode("utf-8"))
    except (ValueError, RecursionError) as e:
        raise ProgressRejected("status.json is not UTF-8 JSON") from e
    return doc, raw


def _problems(root: Path, doc) -> list[str]:
    try:
        problems = checklist.validate(doc)
    except TypeError:
        return ["malformed checklist"]
    steps = doc.get("steps") if isinstance(doc, dict) else None
    for step in steps if isinstance(steps, list) else []:
        if not isinstance(step, dict) or not isinstance(step.get("items"), list):
            continue
        for item in step["items"]:
            if isinstance(item, dict) and item.get("done") is True:
                problem = evidence_problem(root, item.get("evidence"))
                if problem:
                    problems.append(f"{step.get('id', '?')}/{item.get('id', '?')}: {problem}")
    return problems


def check(root: Path) -> list[str]:
    """Problems of <root>/docs/progress/status.json: unreadable, over STATUS_MAX or not JSON,
    checklist.validate, and evidence_problem for every done item. (The JSON schema file is
    enforced by tests/test_workbench.py; this CLI needs no repository code outside tools/workbench.)"""
    try:
        doc, _ = _read(root)
    except ProgressRejected as e:
        return [str(e)]
    return _problems(root, doc)


def _checked(root: Path) -> tuple[dict, bytes]:
    doc, raw = _read(root)
    problems = _problems(root, doc)
    if problems:
        raise ProgressRejected(f"status.json is invalid: {problems[0]}")
    return doc, raw


def _step(doc: dict, step: str) -> dict:
    found = next((s for s in doc["steps"] if s["id"] == step), None)
    if found is None:
        raise ProgressRejected(f"unknown step: {step}")
    return found


def _git_dir(root: Path) -> Path:
    """The Git directory of checkout `root` (`.git`, or the directory a worktree's `.git` file names)."""
    git = root / ".git"
    if git.is_dir():
        return git
    try:
        text = git.read_text(encoding="utf-8", errors="replace").strip()
    except OSError as e:
        raise ProgressRejected("cannot find the Git directory for the progress lock") from e
    if not text.startswith("gitdir:"):
        raise ProgressRejected("cannot find the Git directory for the progress lock")
    gitdir = Path(text[len("gitdir:"):].strip())
    return gitdir if gitdir.is_absolute() else root / gitdir


@contextmanager
def _locked(root: Path):
    """Hold the checkout's progress lock for one whole read-modify-replace transaction, so that
    concurrent commands never overwrite each other's accepted changes. The lock is a file created
    exclusively in the Git directory (never in the working tree); a command that cannot take it
    within LOCK_WAIT seconds is refused and changes nothing."""
    lock = _git_dir(root) / LOCK_NAME
    deadline = time.monotonic() + LOCK_WAIT
    while True:
        try:
            fd = os.open(lock, os.O_CREAT | os.O_EXCL | os.O_WRONLY)
            break
        except FileExistsError:
            if time.monotonic() >= deadline:
                raise ProgressRejected(f"another progress command holds {LOCK_NAME}; run this one again "
                                       "(delete the lock file only if no progress command is running)") from None
            time.sleep(0.05)
        except OSError as e:
            raise ProgressRejected("cannot create the progress lock") from e
    try:
        os.write(fd, f"{os.getpid()}\n".encode("ascii"))
        os.close(fd)
        yield
    finally:
        try:
            lock.unlink()
        except OSError:
            pass


def _transaction(fn):
    """Run a command that reads, changes and replaces status.json under the progress lock."""
    @functools.wraps(fn)
    def run(root: Path, *args, **kwargs):
        with _locked(root):
            return fn(root, *args, **kwargs)
    return run


def _write(root: Path, doc: dict, original: bytes) -> None:
    problems = _problems(root, doc)
    if problems:
        raise ProgressRejected(f"status.json would be invalid: {problems[0]}")
    text = checklist.dump(doc)
    if b"\r\n" in original:
        text = text.replace("\n", "\r\n")
    raw = text.encode("utf-8")
    if len(raw) > STATUS_MAX:
        raise ProgressRejected(f"status.json would be larger than {STATUS_MAX} bytes")
    _confined(root)
    try:
        with (root / STATUS).open("rb") as f:
            current = f.read(STATUS_MAX + 1)
    except OSError as e:
        raise ProgressRejected("cannot read status.json") from e
    if current != original:  # changed by a writer that bypassed the lock: never overwrite its change
        raise ProgressRejected("status.json changed during this command; run it again")
    target, tmp = root / STATUS, None
    try:
        with tempfile.NamedTemporaryFile(dir=target.parent, prefix=".status-", suffix=".tmp", delete=False) as f:
            tmp = Path(f.name)
            f.write(raw)
        os.replace(tmp, target)
    finally:
        if tmp is not None:
            tmp.unlink(missing_ok=True)


def _text(value: str, what: str, limit: int) -> str:
    """`value` stripped, or ProgressRejected: empty, over `limit`, several lines, CJK text or a local path."""
    value = value.strip() if isinstance(value, str) else ""
    if not value or len(value) > limit:
        raise ProgressRejected(f"{what} must be 1 to {limit} characters")
    if any(ord(c) < 32 or ord(c) == 127 for c in value):
        raise ProgressRejected(f"{what} must be one line without control characters")
    if _CJK_RE.search(value) or PRIVATE_RE.search(value):
        raise ProgressRejected(f"{what} must be public English text without local paths")
    return value


def _weight(value) -> int:
    if not checklist._positive(value):
        raise ProgressRejected(f"weight must be an integer from 1 to {checklist.WEIGHT_MAX}")
    return value


def _new_item(item: str, title: str, weight: int) -> dict:
    if not checklist.ITEM_ID_RE.fullmatch(item):
        raise ProgressRejected(f"invalid item id: {item}")
    return {"id": item, "title": _text(title, "item title", checklist.TITLE_MAX), "weight": _weight(weight),
            "done": False, "evidence": None}


def parse_item(spec: str) -> dict:
    """`<id>:<weight>:<title>` as a new open item; the title may contain colons."""
    parts = spec.split(":", 2)
    if len(parts) != 3 or not parts[1].isdigit():
        raise ProgressRejected(f"--item needs <id>:<weight>:<title>: {spec}")
    return _new_item(parts[0], parts[2], int(parts[1]))


@_transaction
def add_track(root: Path, track: str, title: str, today: str) -> None:
    """Append a track; a document without `tracks` first gets checklist.DEFAULT_TRACKS."""
    doc, raw = _checked(root)
    if not checklist.TRACK_ID_RE.fullmatch(track):
        raise ProgressRejected(f"invalid track id: {track}")
    listed = [{"id": tid, "title": t} for tid, t in checklist.tracks(doc)]
    if any(t["id"] == track for t in listed):
        raise ProgressRejected(f"track already exists: {track}")
    listed.append({"id": track, "title": _text(title, "track title", checklist.TITLE_MAX)})
    doc = _with_tracks(doc, listed)
    doc["updated"] = today
    _write(root, doc, raw)


def _with_tracks(doc: dict, listed: list) -> dict:
    # `tracks` sits before `steps`, after the other top-level fields (checklist.dump writes steps last).
    out = {k: v for k, v in doc.items() if k not in ("tracks", "steps")}
    out["tracks"] = listed
    out["steps"] = doc["steps"]
    return out


@_transaction
def add_step(root: Path, step: str, track: str, title: str, items: list[dict], today: str,
             acceptance: str | None = None, acceptance_source: str | None = None, weight: int | None = None) -> None:
    """Append a not_started step with `items` (parse_item results) to a known track."""
    doc, raw = _checked(root)
    if not checklist.TRACK_ID_RE.fullmatch(step):
        raise ProgressRejected(f"invalid step id: {step}")
    if any(s["id"] == step for s in doc["steps"]):
        raise ProgressRejected(f"step already exists: {step}")
    if track not in {tid for tid, _ in checklist.tracks(doc)}:
        raise ProgressRejected(f"unknown track: {track}")
    if not items:
        raise ProgressRejected("a step needs at least one --item")
    if (acceptance is None) != (acceptance_source is None):
        raise ProgressRejected("--acceptance and --acceptance-source go together")
    if acceptance is not None:
        acceptance = _text(acceptance, "acceptance", ACCEPTANCE_MAX)
        if checklist.evidence_kind(acceptance_source) != "path" or evidence_problem(root, acceptance_source):
            raise ProgressRejected(f"acceptance source is not a repository file: {acceptance_source}")
    entry = {"id": step, "track": track, "title": _text(title, "step title", checklist.TITLE_MAX),
             "status": "not_started", "acceptance": acceptance, "acceptance_source": acceptance_source, "blocker": None}
    if weight is not None:
        entry["weight"] = _weight(weight)
    entry["items"] = items
    doc["steps"].append(entry)
    doc["updated"] = today
    _write(root, doc, raw)


@_transaction
def add_item(root: Path, step: str, item: str, title: str, weight: int, today: str) -> None:
    """Append an open item to `step`; a done step goes back to in_progress."""
    doc, raw = _checked(root)
    found = _step(doc, step)
    if any(i["id"] == item for i in found["items"]):
        raise ProgressRejected(f"item already exists: {step}/{item}")
    found["items"].append(_new_item(item, title, weight))
    if found["status"] == "done":
        found["status"] = "in_progress"
    doc["updated"] = today
    _write(root, doc, raw)


@_transaction
def set_current(root: Path, step: str, today: str) -> None:
    doc, raw = _checked(root)
    _step(doc, step)
    if doc["current"] == step:
        raise ProgressRejected(f"{step} is already current")
    doc["current"] = step
    doc["updated"] = today
    _write(root, doc, raw)


@_transaction
def mark_done(root: Path, step: str, item: str, evidence: str, today: str) -> None:
    """Set the item done with `evidence`, a not_started step to in_progress, and `updated` to
    `today` (YYYY-MM-DD); write atomically in checklist.dump layout with the original line
    endings. Raises ProgressRejected (unknown step or item, already done, bad evidence, or a
    document that is invalid before or after)."""
    doc, raw = _checked(root)
    found = _step(doc, step)
    entry = next((i for i in found["items"] if i["id"] == item), None)
    if entry is None:
        raise ProgressRejected(f"unknown item: {step}/{item}")
    if entry["done"]:
        raise ProgressRejected(f"already done: {step}/{item}")
    problem = evidence_problem(root, evidence)
    if problem:
        raise ProgressRejected(problem)
    entry.update(done=True, evidence=evidence)
    if found["status"] == "not_started":
        found["status"] = "in_progress"
    doc["updated"] = today
    _write(root, doc, raw)


@_transaction
def set_status(root: Path, step: str, status: str, today: str) -> None:
    """Set the step's status (checklist.STATUSES) and `updated`, with the same validation and
    write as mark_done. Raises ProgressRejected."""
    doc, raw = _checked(root)
    if status not in checklist.STATUSES:
        raise ProgressRejected("unknown status")
    found = _step(doc, step)
    if found["status"] == status:
        raise ProgressRejected(f"{step} already has status {status}")
    found["status"] = status
    doc["updated"] = today
    _write(root, doc, raw)


def main(argv=None) -> int:
    parser = _Parser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    commands = parser.add_subparsers(dest="command", required=True)
    done = commands.add_parser("done")
    done.add_argument("step")
    done.add_argument("item")
    done.add_argument("--evidence", required=True)
    status = commands.add_parser("status")
    status.add_argument("step")
    status.add_argument("status", choices=checklist.STATUSES)
    commands.add_parser("check")
    track = commands.add_parser("track")
    track.add_argument("track")
    track.add_argument("--title", required=True)
    step = commands.add_parser("step")
    step.add_argument("step")
    step.add_argument("--track", required=True)
    step.add_argument("--title", required=True)
    step.add_argument("--item", action="append", default=[])
    step.add_argument("--acceptance")
    step.add_argument("--acceptance-source")
    step.add_argument("--weight", type=int)
    item = commands.add_parser("item")
    item.add_argument("step")
    item.add_argument("item")
    item.add_argument("--title", required=True)
    item.add_argument("--weight", type=int, default=1)
    current = commands.add_parser("current")
    current.add_argument("step")
    try:
        args = parser.parse_args(argv)
        root = checkout_root(Path.cwd())
        if root is None:
            raise ProgressRejected("not inside a checkout of this repository")
        if args.command == "check":
            problems = check(root)
            if problems:
                print("\n".join(problems))
                return 2
            print("status.json ok")
            return 0
        today = time.strftime("%Y-%m-%d", time.gmtime())
        if args.command == "done":
            mark_done(root, args.step, args.item, args.evidence, today)
            print(f"done {args.step}/{args.item} ({checklist.evidence_kind(args.evidence)})")
        elif args.command == "track":
            add_track(root, args.track, args.title, today)
            print(f"track {args.track} added")
        elif args.command == "step":
            add_step(root, args.step, args.track, args.title, [parse_item(i) for i in args.item], today,
                     args.acceptance, args.acceptance_source, args.weight)
            print(f"step {args.step} added to {args.track}")
        elif args.command == "item":
            add_item(root, args.step, args.item, args.title, args.weight, today)
            print(f"item {args.step}/{args.item} added")
        elif args.command == "current":
            set_current(root, args.step, today)
            print(f"current {args.step}")
        else:
            set_status(root, args.step, args.status, today)
            print(f"status {args.step} {args.status}")
    except (ProgressRejected, OSError, ValueError) as e:
        print(" ".join(str(e).splitlines()), file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())
