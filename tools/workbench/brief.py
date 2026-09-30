"""Write this session's plain-language brief for the development workbench.

  python tools/workbench/brief.py --goal "..." --step "..." --doing "..." --why "..." --waiting-for "..."

Claude runs this in the main session at each plan step and milestone (CLAUDE.md
working loop); subagents never run it. Omitted fields keep their previous value.
The session defaults to CLAUDE_CODE_SESSION_ID. The brief is private: it is
written under work/loop-memory/workbench/briefs/ of the main checkout.

Exit codes: 0 written, 2 invalid input.
"""
from __future__ import annotations

import argparse
import json
import os
import sys
from datetime import datetime, timezone
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import paths  # noqa: E402

FIELDS = ("goal", "step", "doing", "why", "waiting_for")
FIELD_MAX_BYTES = 500


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--session", default=os.environ.get("CLAUDE_CODE_SESSION_ID"))
    for name in FIELDS:
        parser.add_argument("--" + name.replace("_", "-"), dest=name)
    args = parser.parse_args(argv)
    if not paths.valid_sid(args.session):
        print("brief: no valid session id (pass --session or run inside a Claude Code session)", file=sys.stderr)
        return 2
    given = {k: getattr(args, k) for k in FIELDS if getattr(args, k) is not None}
    if not given:
        print("brief: give at least one field", file=sys.stderr)
        return 2
    for k, v in given.items():
        if len(v.encode("utf-8")) > FIELD_MAX_BYTES:
            print(f"brief: --{k.replace('_', '-')} is longer than {FIELD_MAX_BYTES} bytes", file=sys.stderr)
            return 2
    main_dir = paths.main_checkout(Path.cwd()) or paths.main_checkout(Path(__file__).resolve().parent)
    if main_dir is None:
        print("brief: not inside a checkout of this repository", file=sys.stderr)
        return 2
    target = paths.data_root(main_dir) / "briefs" / f"{args.session}.json"
    try:
        previous = json.loads(target.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        previous = {}
    brief = {k: previous.get(k, "") for k in FIELDS} if isinstance(previous, dict) else dict.fromkeys(FIELDS, "")
    brief.update(given)
    brief.update(schema=1, session_id=args.session, updated=datetime.now(timezone.utc).isoformat(timespec="seconds"))
    target.parent.mkdir(parents=True, exist_ok=True)
    tmp = target.with_name(f"{target.name}.{os.getpid()}.tmp")
    tmp.write_text(json.dumps(brief, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    os.replace(tmp, target)
    print(f"brief updated: {args.session}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
