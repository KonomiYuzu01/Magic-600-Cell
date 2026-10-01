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

import argparse
import itertools
import json
import os
import secrets
import sys
import tempfile
import time
from datetime import datetime, timezone
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import home  # noqa: E402
import notes  # noqa: E402
import paths  # noqa: E402

PICK_EXT = (".png", ".jpg", ".jpeg", ".gif", ".svg", ".webp")
WAIT_DEFAULT = 540
WAIT_MAX = 590
WAIT_POLL = 2.0


class AskRejected(Exception):
    pass


class _Parser(argparse.ArgumentParser):
    def error(self, message):
        raise AskRejected(message)


def build_record(sid: str, question: str, context: str, options: list[str], attachments: list[str],
                 kind: str | None, blocking: bool, roots: list, now: float) -> dict:
    """The validated ask record (see home.asks for the reader). Raises AskRejected.
    {"schema": 1, "id": "YYYYmmddTHHMMSSZ-<8 hex>", "session_id": sid, "created": ISO UTC,
     "kind": ..., "blocking": bool, "question": str, "context": str, "options": [str],
     "attachments": [resolved absolute path strings]}"""
    if not paths.valid_sid(sid):
        raise AskRejected("invalid session id")
    kind = kind if kind is not None else ("decision" if len(options) >= 2 else "question")
    if kind not in home.ASK_KINDS:
        raise AskRejected("unknown ask kind")
    if kind == "approve" and not options:
        options = ["Approve", "Reject"]
    if "\r" in question or "\n" in question:
        raise AskRejected("the question must be one line")
    question, context = question.strip(), context.strip()
    if not question:
        raise AskRejected("the question is empty")
    for label, text, limit in (("question", question, home.QUESTION_MAX_BYTES),
                               ("context", context, home.CONTEXT_MAX_BYTES)):
        if len(text.encode("utf-8")) > limit:
            raise AskRejected(f"the {label} is longer than {limit} bytes")
    if len(options) > home.OPTIONS_MAX:
        raise AskRejected(f"at most {home.OPTIONS_MAX} options are allowed")
    labels = []
    for option in options:
        if "\r" in option or "\n" in option:
            raise AskRejected("an option must be one line")
        option = option.strip()
        if not option:
            raise AskRejected("an option is empty")
        if len(option.encode("utf-8")) > home.OPTION_MAX_BYTES:
            raise AskRejected(f"an option is longer than {home.OPTION_MAX_BYTES} bytes")
        if option in labels:
            raise AskRejected("duplicate options")
        labels.append(option)
    if kind == "decision" and len(labels) < 2:
        raise AskRejected("a decision needs at least two options")
    if kind == "pick" and (labels or len(attachments) < 2 or
                           any(Path(a).suffix.lower() not in PICK_EXT for a in attachments)):
        raise AskRejected("a pick needs at least two image attachments and no options")
    if len(attachments) > home.ATTACHMENTS_MAX:
        raise AskRejected(f"at most {home.ATTACHMENTS_MAX} attachments are allowed")
    files, seen = [], set()
    for attachment in attachments:
        file = Path(attachment).resolve()
        key = os.path.normcase(str(file))
        if key in seen:
            raise AskRejected("duplicate attachments")
        if not file.is_file() or not paths.inside(file, roots):
            raise AskRejected("an attachment must be a regular file inside a checkout")
        seen.add(key)
        files.append(str(file))
    return {"schema": 1, "id": time.strftime("%Y%m%dT%H%M%SZ", time.gmtime(now)) + "-" + secrets.token_hex(4),
            "session_id": sid, "created": datetime.fromtimestamp(now, timezone.utc).isoformat(timespec="seconds"),
            "kind": kind, "blocking": blocking, "question": question, "context": context,
            "options": labels, "attachments": files}


def write_record(data: Path, record: dict) -> Path:
    """Write <data>/asks/<sid>/<id>.json atomically (tmp file then os.replace). Raises AskRejected
    when the session already has home.ASKS_PER_SESSION asks or the record is over home.ASK_MAX_BYTES."""
    raw = (json.dumps(record, ensure_ascii=False, indent=2) + "\n").encode("utf-8")
    if len(raw) > home.ASK_MAX_BYTES:
        raise AskRejected(f"the ask is larger than {home.ASK_MAX_BYTES} bytes")
    directory = data / "asks" / record["session_id"]
    main_dir = data.parents[2]   # data is <main>/work/loop-memory/workbench (paths.data_root)
    if not paths.within(directory, main_dir):
        raise AskRejected("the ask directory leads outside the checkout")
    directory.mkdir(parents=True, exist_ok=True)
    if not paths.inside(directory, [main_dir]):
        raise AskRejected("the ask directory leads outside the checkout")
    with os.scandir(directory) as entries:
        names = [e.name for e in itertools.islice(entries, home.ASK_SCAN_MAX)]
    if len(names) >= home.ASK_SCAN_MAX or sum(n.endswith(".json") for n in names) >= home.ASKS_PER_SESSION:
        raise AskRejected("this session's ask inbox is full")
    target = directory / f"{record['id']}.json"
    tmp = None
    try:
        with tempfile.NamedTemporaryFile(dir=directory, prefix=f".{record['id']}-", suffix=".tmp", delete=False) as f:
            tmp = Path(f.name)
            f.write(raw)
        os.replace(tmp, target)
    finally:
        if tmp is not None:
            tmp.unlink(missing_ok=True)
    return target


def answered(data: Path, sid: str, ask_id: str) -> bool:
    """True when the session's owner-note inbox holds a note whose text starts with
    `answer to ask <ask_id>: ` (any delivery state). Bounded read; never writes."""
    try:
        with (data / "inbox" / f"{sid}.jsonl").open("rb") as f:
            raw = f.read(notes.INBOX_MAX_BYTES)
    except OSError:
        return False
    prefix = f"answer to ask {ask_id}: "
    for line in raw.splitlines():
        try:
            note = json.loads(line)
        except (ValueError, RecursionError):
            continue
        if isinstance(note, dict) and isinstance(note.get("text"), str) and note["text"].startswith(prefix):
            return True
    return False


def main(argv=None) -> int:
    argv = sys.argv[1:] if argv is None else list(argv)
    waiting = bool(argv and argv[0] == "wait")
    parser = _Parser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--session", default=os.environ.get("CLAUDE_CODE_SESSION_ID"))
    if waiting:
        parser.add_argument("ask_id")
        parser.add_argument("--timeout", type=int, default=WAIT_DEFAULT)
    else:
        parser.add_argument("--question", required=True)
        parser.add_argument("--context", default="")
        parser.add_argument("--option", action="append", default=[])
        parser.add_argument("--attach", action="append", default=[])
        parser.add_argument("--kind", choices=home.ASK_KINDS)
        parser.add_argument("--blocking", action="store_true")
    try:
        args = parser.parse_args(argv[1:] if waiting else argv)
        if args.session is None:
            raise AskRejected("ask.py runs in a Claude Code session (CLAUDE_CODE_SESSION_ID is not set)")
        if not paths.valid_sid(args.session):
            raise AskRejected("invalid session id")
        main_dir = paths.main_checkout(Path.cwd())
        if main_dir is None:
            raise AskRejected("not inside a checkout of this repository")
        data = paths.data_root(main_dir)
        if waiting:
            if not home.ID_RE.fullmatch(args.ask_id):
                raise AskRejected("invalid ask id")
            if not 1 <= args.timeout <= WAIT_MAX:
                raise AskRejected(f"timeout must be from 1 to {WAIT_MAX} seconds")
            deadline = time.monotonic() + args.timeout
            while time.monotonic() < deadline:
                if answered(data, args.session, args.ask_id):
                    print(f"answer to ask {args.ask_id} is queued; it arrives with this tool result")
                    return 0
                time.sleep(min(WAIT_POLL, max(0, deadline - time.monotonic())))
            print("no answer yet")
            return 3
        record = build_record(args.session, args.question, args.context, args.option, args.attach,
                              args.kind, args.blocking, paths.checkouts(main_dir), time.time())
        write_record(data, record)
    except (AskRejected, OSError, ValueError) as e:
        print(" ".join(str(e).splitlines()), file=sys.stderr)
        return 2
    print(f"ask {record['id']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
