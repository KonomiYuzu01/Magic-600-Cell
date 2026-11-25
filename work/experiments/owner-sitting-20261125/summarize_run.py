"""Summarise one shell run of the 25 November 2026 owner-machine sitting into a path-free record.

    python -B work/experiments/owner-sitting-20261125/summarize_run.py <run folder> --label <name> --out <file>

The run folder is one that `renderer/shell/launch.py --out` wrote. The record carries:
- `launch.json`'s fields as they are (launch.py already writes the build and run folders as `<build>` and `<out>`);
- from `events.jsonl` (`magic600-c9-shell-events-v1`): the `run` row's foreground flag and correctness record, the
  row counts by kind, each `key` row's time in seconds after the first frame, and the engine and publication
  revisions the frames drew;
- from the session journal (`session/journal`): how often each command row appears;
- the SHA-256 of each input file.

It reads only; it starts nothing and writes only `--out`, which must not exist yet. Evidence kind: what the logs
show (PHS.64); it establishes neither the machine nor what the screen showed.
"""
from __future__ import annotations

import argparse
import collections
import hashlib
import json
import re
import sys
from pathlib import Path

sys.dont_write_bytecode = True

FORMAT = "magic600-owner-sitting-20261125-run-v1"
ROW = re.compile(rb'"row":"([a-z0-9_.:-]+)"')


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def summarise(folder: Path, label: str) -> dict:
    launch_path = folder / "launch.json"
    events_path = folder / "events.jsonl"
    journal_path = folder / "session" / "journal"
    launch = json.loads(launch_path.read_text(encoding="utf-8"))
    rows = [json.loads(line) for line in events_path.read_text(encoding="utf-8").splitlines() if line.strip()]
    run = next(row for row in rows if row.get("t") == "run")
    frames = [row for row in rows if row.get("t") == "frame"]
    first = frames[0]["qpc"] if frames else 0
    frequency = run["qpc_frequency"]
    keys = [{"seq": row["seq"], "seconds": round((row["qpc"] - first) / frequency, 2), "frame_id": row["frame_id"]}
            for row in rows if row.get("t") == "key"]
    revisions = sorted({(row.get("engine_revision"), row.get("publication_revision")) for row in frames},
                       key=lambda pair: (pair[0] is None, pair))
    journal = journal_path.read_bytes()
    commands = collections.Counter(name.decode() for name in ROW.findall(journal))
    return {
        "format": FORMAT,
        "label": label,
        "evidence": "actual_windows_directx: what the shell's own logs show; nothing about what the screen showed",
        "launch": launch,
        "run": {key: run.get(key) for key in ("run_id", "build_identity", "synthetic", "foreground_throughout",
                                              "correctness", "correctness_control", "correctness_detail")},
        "row_counts": dict(collections.Counter(row.get("t") for row in rows)),
        "keys": keys,
        "revisions_drawn": [list(pair) for pair in revisions],
        "journal_rows": dict(sorted(commands.items())),
        "inputs": {name: digest(path) for name, path in (("launch.json", launch_path), ("events.jsonl", events_path),
                                                          ("session/journal", journal_path))},
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("folder", type=Path)
    parser.add_argument("--label", required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args(argv)
    if args.out.exists():
        print(f"refused: {args.out.name} exists")
        return 2
    record = summarise(args.folder, args.label)
    args.out.write_bytes((json.dumps(record, indent=2, sort_keys=True) + "\n").encode("utf-8"))
    print(f"{args.label}: {len(record['keys'])} key rows, correctness {record['run']['correctness']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
