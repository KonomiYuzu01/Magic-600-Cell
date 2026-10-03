"""Shared interface of the Windows verification batch (run_all.py).

The batch runs, in order, whichever Windows-only checks exist in this checkout:
the native host self-test, the migration probes and the S-B renderer captures.
run_all.py owns the console, the heavy-work guard, the directories and the
sanitised report. Each step module owns one check and reaches the owner and
subprocesses only through the Context it is given, so a test can drive a step
with a fake Context: no console, real check, GPU or administrator rights.

A step module (STEP_MODULES, imported from this directory) exposes STEP, an
object with the Step attributes and methods below. A step writes only below
ctx.private and ctx.scratch; the checks it starts write where they always do
(the migration probes under work/migration-probes/runs/, the captures under
work/loop-memory/perf/renderer/sb/). Every check runs on fresh synthetic
data: a step never opens, copies or names a personal session.
"""
from __future__ import annotations

import dataclasses
import hashlib
from pathlib import Path
from typing import Mapping, Optional, Protocol, Sequence

FORMAT = "magic600-windows-batch-v1"

# Step modules in this directory, in run order.
STEP_MODULES = ("step_native", "step_migration", "step_renderer")

PASS = "pass"                # the check ran and met its expectation
FAIL = "fail"                # the check ran and did not meet it
SKIPPED = "skipped"          # not run: missing here, needs elevation, or the owner skipped it
ERROR = "error"              # the check could not complete: a tool failed, a time limit, missing output
INTERRUPTED = "interrupted"  # Ctrl+C during the step
STATUSES = (PASS, FAIL, SKIPPED, ERROR, INTERRUPTED)

REASON_LIMIT = 300  # characters of minimal error text kept in a result
MAX_LINES = 8       # summary lines per step

# Exit codes of run_all.py.
EXIT_PASS = 0         # every step that ran passed; skipped steps are allowed
EXIT_FAIL = 1         # a step failed or ended in error
EXIT_USAGE = 2        # bad arguments, not Windows, or the results would still hold private text
EXIT_INTERRUPTED = 130

# ctx.options keys and their defaults; the command line overrides them.
DEFAULT_OPTIONS = {
    "scenes": ("w1", "w2", "w4"),  # renderer scenes to capture (W3 met on 3 October 2026)
    "runs": 3,                     # capture runs per scene
    "faults": True,                # also run the four injected-fault W3 runs
    "rebuild_probe": False,        # rebuild sb_probe.exe even when it exists
}


@dataclasses.dataclass
class StepResult:
    status: str                    # one of STATUSES
    reason: str = ""               # minimal error text or why it was skipped: one line, clip()ped
    counts: dict = dataclasses.field(default_factory=dict)   # name -> int
    digests: dict = dataclasses.field(default_factory=dict)  # name -> lowercase SHA-256 hex
    details: dict = dataclasses.field(default_factory=dict)  # JSON-safe, for batch.json only
    lines: list = dataclasses.field(default_factory=list)    # at most MAX_LINES short lines for summary.md


@dataclasses.dataclass
class Completed:
    returncode: Optional[int]  # None when the time limit ended the program
    output: str                # merged stdout and stderr; "" for an interactive run
    seconds: float
    timed_out: bool = False


class Context(Protocol):
    repo: Path        # repository root: the checkout that holds run_all.py
    private: Path     # this step's directory work/loop-memory/windows-batch/<stamp>/<step name>/, created
    scratch: Path     # this step's empty directory below the system temporary directory, created with a
                      # short path (native builds fail near 260 characters); removed after the batch
    elevated: bool    # the batch runs with administrator rights
    options: Mapping[str, object]  # DEFAULT_OPTIONS as overridden on the command line

    def say(self, line: str) -> None:
        """Print one line of progress."""

    def manual(self, line: str) -> None:
        """A manual step: print `line` as one clear line for the owner, then wait for Enter.
        `line` says what the owner does or watches next."""

    def ask(self, question: str, default: str = "") -> str:
        """Print the question with its default; return the stripped answer, or the default if empty."""

    def run(self, argv: Sequence[str], *, timeout: float, cwd: Optional[Path] = None,
            env: Optional[Mapping[str, str]] = None, interactive: bool = False) -> Completed:
        """Run a program without a shell; cwd defaults to repo, env replaces the environment when given.

        Not interactive: stdin is empty, stdout and stderr are merged, echoed to the console as they
        arrive, saved below ctx.private and returned. Interactive: the program shares the console,
        so its own prompts reach the owner, and output is "". After the time limit the program and
        its child processes are ended (returncode None, timed_out True). Ctrl+C ends them too and
        then raises KeyboardInterrupt."""

    def require_quiet(self) -> Optional[str]:
        """The heavy-work guard, called right before each capture. Returns None once no build,
        Codex implementation call, capture or other heavy process runs (the owner stops them and
        presses Enter to check again), or the reason to skip when the owner types skip."""


class Step(Protocol):
    name: str   # short id: the --only value and the name of its private and scratch directories
    title: str  # one line for the plan

    def unavailable(self, ctx: Context) -> Optional[str]:
        """None when the step can run here, otherwise why it is skipped: its files are not in this
        checkout, a tool or environment is missing, or it needs administrator rights. It only reads:
        no prompt, process or write."""

    def run(self, ctx: Context) -> StepResult:
        """Run the check. An expected failure is a FAIL result and a check that could not complete
        is an ERROR result, not an exception; KeyboardInterrupt propagates."""


def clip(text: object, limit: int = REASON_LIMIT) -> str:
    """One line of at most `limit` characters: the minimal error text kept in a result."""
    line = " ".join(str(text).split())
    return line if len(line) <= limit else line[: limit - 3] + "..."


def sha256_file(path: Path) -> str:
    """SHA-256 of a file's raw bytes."""
    digest = hashlib.sha256()
    with open(path, "rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()
