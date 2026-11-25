"""How many of a shell run's frames the core confirmed as presented, overall and per five minutes.

    python -B work/experiments/owner-sitting-20261125/presented_share.py <run folder> [<run folder> ...] --out <file>

Each run folder is one that `renderer/shell/launch.py --out` wrote. From `events.jsonl`
(`magic600-c9-shell-events-v1`) the record counts the `frame` rows (frames the shell handed to the core) and the
`presented` rows (frames the core confirmed as presented through its own swap chain's frame statistics). A frame
without its own `presented` row is the confirmation gap's unit: a key or sample whose frame is one of them has no
confirmed display (owner, 22 November 2026, item 10). The record also counts the steps between consecutive
presented sequences (a step of 2 is one frame skipped) and gives the window arguments from `launch.json`.

It reads only; it starts nothing and writes only `--out`, which must not exist yet. The record holds run IDs and
counts, no local path. Evidence kind: what the shell's logs show (PHS.64); it says nothing about what the screen
showed, and it does not say why a frame has no row.
"""
from __future__ import annotations

import argparse
import collections
import hashlib
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True

FORMAT = "magic600-owner-sitting-20261125-presented-share-v1"
BUCKET_SECONDS = 300


def share(folder: Path) -> dict:
    launch_path = folder / "launch.json"
    events_path = folder / "events.jsonl"
    launch = json.loads(launch_path.read_text(encoding="utf-8"))
    frames: dict[int, int] = {}
    presented: set[int] = set()
    sequences: list[int] = []
    head = None
    with events_path.open(encoding="utf-8") as handle:
        for line in handle:
            if '"t":"frame"' in line:
                row = json.loads(line)
                frames[row["frame_id"]] = row["qpc"]
            elif '"t":"presented"' in line:
                row = json.loads(line)
                presented.add(row["frame_id"])
                sequences.append(row["sequence"])
            elif '"t":"run"' in line:
                head = json.loads(line)
    if head is None or not frames:
        raise SystemExit(f"refused: {folder.name} has no run row or no frames")
    ids = sorted(frames)
    first = frames[ids[0]]
    frequency = head["qpc_frequency"]
    buckets: dict[int, list[int]] = {}
    for frame_id in ids:
        bucket = buckets.setdefault(int((frames[frame_id] - first) / frequency // BUCKET_SECONDS), [0, 0])
        bucket[0] += 1
        bucket[1] += frame_id not in presented
    missing = sum(frame_id not in presented for frame_id in ids)
    command = launch.get("command", [])
    window = [arg for index, arg in enumerate(command)
              if arg in ("--windowed", "--fullscreen") or (index and command[index - 1] == "--resolution")
              or arg == "--resolution"]
    return {
        "run_id": head.get("run_id"),
        "build_identity": head.get("build_identity"),
        "started_utc": launch.get("started_utc"),
        "lx4": launch.get("lx4"),
        "editor": launch.get("editor"),
        "lx2_injection": launch.get("lx2_injection") is not None,
        "window_arguments": window,
        "frames": len(ids),
        "presented_rows": len(presented),
        "frames_without_presented_row": missing,
        "share_without_presented_row": round(missing / len(ids), 4),
        "per_five_minutes": [round(bad / count, 3) for _, (count, bad) in sorted(buckets.items())],
        "presented_sequence_steps": {str(step): count for step, count in
                                     sorted(collections.Counter(b - a for a, b in zip(sequences, sequences[1:])).items())},
        "inputs": {"launch.json": hashlib.sha256(launch_path.read_bytes()).hexdigest(),
                   "events.jsonl": hashlib.sha256(events_path.read_bytes()).hexdigest()},
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("folders", type=Path, nargs="+")
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args(argv)
    if args.out.exists():
        print(f"refused: {args.out.name} exists")
        return 2
    runs = [share(folder) for folder in args.folders]
    record = {"format": FORMAT,
              "evidence": "actual_windows_directx: what the shell's own logs show; nothing about what the screen showed",
              "bucket_seconds": BUCKET_SECONDS, "runs": runs}
    args.out.write_bytes((json.dumps(record, indent=2, sort_keys=True) + "\n").encode("utf-8"))
    for run in runs:
        print(f"{run['run_id']} {run['build_identity']}: {run['frames_without_presented_row']} of {run['frames']} frames "
              f"without a presented row ({100 * run['share_without_presented_row']:.2f}%)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
