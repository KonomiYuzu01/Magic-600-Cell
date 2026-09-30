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

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import checklist  # noqa: E402
import paths  # noqa: E402

STATUS = Path("docs") / "progress" / "status.json"
STATUS_MAX = 256 * 1024


class ProgressRejected(Exception):
    pass


def checkout_root(start: Path) -> Path | None:
    """The checkout (main or worktree) that contains `start`: the deepest of paths.checkouts(main)."""
    raise NotImplementedError


def evidence_problem(root: Path, ref: str) -> str | None:
    """None when `ref` is accepted evidence in checkout `root`, else the reason: the form
    (checklist.evidence_kind), a sha that `git rev-parse --verify --quiet <sha>^{commit}` resolves
    in `root`, or a path that is an existing regular file inside `root` (paths.inside)."""
    raise NotImplementedError


def check(root: Path) -> list[str]:
    """Problems of <root>/docs/progress/status.json: unreadable, over STATUS_MAX or not JSON,
    checklist.validate, and evidence_problem for every done item. (The JSON schema file is
    enforced by tests/test_workbench.py; this CLI needs no repository code outside tools/workbench.)"""
    raise NotImplementedError


def mark_done(root: Path, step: str, item: str, evidence: str, today: str) -> None:
    """Set the item done with `evidence`, a not_started step to in_progress, and `updated` to
    `today` (YYYY-MM-DD); write atomically in checklist.dump layout with the original line
    endings. Raises ProgressRejected (unknown step or item, already done, bad evidence, or a
    document that is invalid before or after)."""
    raise NotImplementedError


def set_status(root: Path, step: str, status: str, today: str) -> None:
    """Set the step's status (checklist.STATUSES) and `updated`, with the same validation and
    write as mark_done. Raises ProgressRejected."""
    raise NotImplementedError


def main(argv=None) -> int:
    raise NotImplementedError


if __name__ == "__main__":
    sys.exit(main())
