"""Record checklist progress in docs/progress/status.json (schema 2) with evidence.

  python tools/workbench/progress.py done <step> <item> --evidence <ref>
  python tools/workbench/progress.py status <step> not_started|in_progress|blocked|done
  python tools/workbench/progress.py check

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

Exit codes: 0 ok, 2 invalid input or an invalid file.
"""
from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import tempfile
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import checklist  # noqa: E402
import paths  # noqa: E402

STATUS = Path("docs") / "progress" / "status.json"
STATUS_MAX = 256 * 1024


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
    target, tmp = root / STATUS, None
    try:
        with tempfile.NamedTemporaryFile(dir=target.parent, prefix=".status-", suffix=".tmp", delete=False) as f:
            tmp = Path(f.name)
            f.write(raw)
        os.replace(tmp, target)
    finally:
        if tmp is not None:
            tmp.unlink(missing_ok=True)


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
        else:
            set_status(root, args.step, args.status, today)
            print(f"status {args.step} {args.status}")
    except (ProgressRejected, OSError, ValueError) as e:
        print(" ".join(str(e).splitlines()), file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())
