"""Ask the owner a question through the development workbench's "For you" inbox.

  python tools/workbench/ask.py --question "..." [--context "..."] [--option "label"]...
                                [--attach PATH]... [--kind decision|pick|approve|question] [--blocking]
  python tools/workbench/ask.py wait <ask-id> [--timeout SECONDS]

Claude runs this in the main session; subagents never run it. The session defaults
to CLAUDE_CODE_SESSION_ID. The ask is private: one JSON record under
work/loop-memory/workbench/asks/<session>/ of the main checkout. The owner's answer
arrives as an ordinary owner note `answer to ask <id>: <option or text>` at the
session's next tool step.

Kinds:
  decision  two or more --option labels (the default when two or more are given)
  approve   options default to Approve and Reject
  pick      two or more image attachments (png, jpg, jpeg, gif, svg, webp) and no
            --option; the owner picks one image and the answer is
            `picked <n>: <checkout>/<path>` (n is the attachment's 1-based position)
  question  free text, options allowed (the default otherwise)

Attachments must be regular files inside a checkout of this repository (work/ included).

`wait` keeps the turn open after a blocking ask: it only reads this session's owner-note
inbox, every 2 seconds, until a note answering the ask exists (exit 0; the note itself is
delivered with this tool step's result) or the timeout passes (exit 3; default 540 s, at
most 590 s). It never prints the answer.

Exit codes: 0 written or answered, 2 invalid input, 3 no answer before the timeout.
"""
from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import home  # noqa: E402
import paths  # noqa: E402

PICK_EXT = (".png", ".jpg", ".jpeg", ".gif", ".svg", ".webp")
WAIT_DEFAULT = 540
WAIT_MAX = 590
WAIT_POLL = 2.0


class AskRejected(Exception):
    pass


def build_record(sid: str, question: str, context: str, options: list[str], attachments: list[str],
                 kind: str | None, blocking: bool, roots: list, now: float) -> dict:
    """The validated ask record (see home.asks for the reader). Raises AskRejected.
    {"schema": 1, "id": "YYYYmmddTHHMMSSZ-<8 hex>", "session_id": sid, "created": ISO UTC,
     "kind": ..., "blocking": bool, "question": str, "context": str, "options": [str],
     "attachments": [resolved absolute path strings]}"""
    raise NotImplementedError


def write_record(data: Path, record: dict) -> Path:
    """Write <data>/asks/<sid>/<id>.json atomically (tmp file then os.replace). Raises AskRejected
    when the session already has home.ASKS_PER_SESSION asks or the record is over home.ASK_MAX_BYTES."""
    raise NotImplementedError


def answered(data: Path, sid: str, ask_id: str) -> bool:
    """True when the session's owner-note inbox holds a note whose text starts with
    `answer to ask <ask_id>: ` (any delivery state). Bounded read; never writes."""
    raise NotImplementedError


def main(argv=None) -> int:
    raise NotImplementedError


if __name__ == "__main__":
    sys.exit(main())
