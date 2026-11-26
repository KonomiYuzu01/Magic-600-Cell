"""Why a shell run's frames have no `presented` row: single skips of frames that were on screen, and the race behind them.

    python -B work/experiments/presented-gap-20261126/race.py <run folder> [<run folder> ...] --out <file>

Each run folder is one that `renderer/shell/launch.py --out` wrote (or this folder's `ab_fix.py`). From
`events.jsonl` (`magic600-c9-shell-events-v1`) the record gives per run:

- `frames`, `without_presented_row`: as `presented_share.py` (owner-sitting-20261125) counts them.
- `missed_between_confirmed`: the frames without a row whose two neighbours both have one, by how many refreshes apart
  the neighbours' `display_qpc` (SyncQPCTime) lie. Two refreshes apart means the missed frame's own refresh lay between
  them: with sync interval 1 the frame was on screen, and no statistics sample met its present count.
- `steps`: the steps between consecutive confirmed frame IDs (a step of 2 is one frame without a row), and the frames
  inside them: `missed_in_steps_2_to_4` (one to three frames in a row) and `missed_in_steps_5_up` (stalls of four or
  more, where no statistics row advanced for several refreshes).
- `sample_offset_us`: when the shell's one statistics sample per frame ran, relative to the vblank it could first show
  a frame at. For each confirmed frame, the first frame whose sample ran after that frame's `display_qpc` is found,
  and the record counts by the sample's offset after that vblank (100 microsecond buckets, `2000+` beyond) whether that
  sample already confirmed the frame (`posted`) or a later sample did (`not_posted`). The sample time is the frame
  row's `returned_qpc` (when m6h_frame returned; the shell reads the statistics right after it) or, where the build
  logs no `returned_qpc`, the frame's `ui_frame` pre-draw QPC, which follows the read.
- `frame_call_ms`: the median and 95th percentile of `returned_qpc - qpc` (how long m6h_frame blocked), when logged.

A single sample per frame that runs within a millisecond after the vblank, where DXGI has posted that vblank's
statistics for some frames and not yet for others, passes a present count whenever the answer changes from "not yet"
to "posted" between two frames. The record shows where the samples ran and how often the statistics were posted by
then. It reads only and writes only `--out`, which must not exist yet. The record holds run IDs and counts, no local
path. Evidence kind: what the shell's own logs show (PHS.64); nothing about what the screen showed.
"""
from __future__ import annotations

import argparse
import bisect
import collections
import hashlib
import json
import statistics
import sys
from pathlib import Path

sys.dont_write_bytecode = True

FORMAT = "magic600-presented-gap-20261126-race-v1"
REFRESH_QPC = 10_000_000 / 60  # the owner's display: 60 Hz; QPC frequency 10 MHz (checked per run)


def race(folder: Path) -> dict:
    events_path = folder / "events.jsonl"
    frames: dict[int, tuple[int, int | None]] = {}
    shown: dict[int, int] = {}
    predraw: dict[int, int] = {}
    head = None
    with events_path.open(encoding="utf-8") as handle:
        for line in handle:
            if '"t":"frame"' in line:
                row = json.loads(line)
                frames[row["frame_id"]] = (row["qpc"], row.get("returned_qpc"))
            elif '"t":"presented"' in line:
                row = json.loads(line)
                shown[row["frame_id"]] = row["display_qpc"]
            elif '"t":"ui_frame"' in line:
                row = json.loads(line)
                predraw[row["frame_id"]] = row["qpc"]
            elif '"t":"run"' in line:
                head = json.loads(line)
    if head is None or not frames or head.get("qpc_frequency") != 10_000_000:
        raise SystemExit(f"refused: {folder.name} has no run row, no frames or another QPC frequency")
    ids = sorted(frames)
    missed = collections.Counter()
    for frame_id in ids[1:-1]:
        if frame_id in shown:
            continue
        before, after = shown.get(frame_id - 1), shown.get(frame_id + 1)
        missed[str(round((after - before) / REFRESH_QPC)) if before and after else "not between confirmed"] += 1
    confirmed = sorted(shown)
    steps = collections.Counter(b - a for a, b in zip(confirmed, confirmed[1:]))
    # Which frame's sample confirmed each frame: the presented rows follow the frame row of the frame whose call read
    # them, so the confirming frame is the last frame row before the presented row.
    confirming: dict[int, int] = {}
    last = None
    with events_path.open(encoding="utf-8") as handle:
        for line in handle:
            if '"t":"frame"' in line:
                last = json.loads(line)["frame_id"]
            elif '"t":"presented"' in line and last is not None:
                confirming[json.loads(line)["frame_id"]] = last
    samples = sorted((returned if returned is not None else predraw[frame_id], frame_id)
                     for frame_id, (_, returned) in frames.items() if returned is not None or frame_id in predraw)
    times = [time for time, _ in samples]
    offsets: dict[str, list[int]] = {}
    for frame_id, display in shown.items():
        index = bisect.bisect_right(times, display)
        if index >= len(samples) or frame_id not in confirming:
            continue
        time, sampler = samples[index]
        offset = (time - display) / 10
        bucket = f"{int(offset // 100) * 100}" if offset < 2000 else "2000+"
        pair = offsets.setdefault(bucket, [0, 0])
        pair[0 if sampler == confirming[frame_id] else 1] += 1
    calls = sorted((returned - qpc) / 10_000 for qpc, returned in frames.values() if returned is not None)
    return {
        "run_id": head.get("run_id"),
        "build_identity": head.get("build_identity"),
        "frames": len(ids),
        "without_presented_row": len(ids) - len(shown.keys() & frames.keys()),
        "missed_between_confirmed": dict(sorted(missed.items())),
        "steps": {str(step): count for step, count in sorted(steps.items())},
        # Frames inside a step: in short runs of one to three (steps 2 to 4), and in stalls of four or more (step 5 up).
        "missed_in_steps_2_to_4": sum((step - 1) * count for step, count in steps.items() if 2 <= step <= 4),
        "missed_in_steps_5_up": sum((step - 1) * count for step, count in steps.items() if step >= 5),
        "sample_offset_us": {bucket: {"posted": pair[0], "not_posted": pair[1]}
                             for bucket, pair in sorted(offsets.items(), key=lambda kv: (kv[0] == "2000+", int(kv[0].rstrip("+"))))},
        "sample_time": "returned_qpc" if calls else "ui_frame pre-draw qpc",
        "frame_call_ms": {"median": round(statistics.median(calls), 3), "p95": round(calls[int(0.95 * (len(calls) - 1))], 3)}
        if calls else None,
        "inputs": {"events.jsonl": hashlib.sha256(events_path.read_bytes()).hexdigest()},
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("folders", type=Path, nargs="+")
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args(argv)
    if args.out.exists():
        print(f"refused: {args.out.name} exists")
        return 2
    runs = [race(folder) for folder in args.folders]
    record = {"format": FORMAT,
              "evidence": "actual_windows_directx: what the shell's own logs show; nothing about what the screen showed",
              "runs": runs}
    args.out.write_bytes((json.dumps(record, indent=2, sort_keys=True) + "\n").encode("utf-8"))
    for run in runs:
        between = run["missed_between_confirmed"].get("2", 0)
        print(f"{run['run_id']} {run['build_identity']}: {run['without_presented_row']} of {run['frames']} frames "
              f"without a presented row; {between} of them single frames between confirmed neighbours two refreshes apart")
    return 0


if __name__ == "__main__":
    sys.exit(main())
