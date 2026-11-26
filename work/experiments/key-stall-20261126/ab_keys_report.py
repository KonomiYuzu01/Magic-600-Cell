"""The frame thread around each turn key in on-screen runs that ab_keys.py wrote: the key call, the frame calls and
frame intervals after it, and the key-down to the core-confirmed present of the frame that shows it.

    python -B work/experiments/key-stall-20261126/ab_keys_report.py RUN_DIR [RUN_DIR ...] --out RECORD.json

From each run's `events.jsonl` (the shell's own log, `magic600-c9-shell-events-v1`):
- `key_call_ms`: `handled_qpc - qpc` of each key row, from the key-down's arrival until `m6h_key_down` returned;
- `frame_call_ms`: `returned_qpc - qpc` of each frame row, the frame call on the frame thread (it includes the core's
  frame-latency wait), over all frames and over the first four frames submitted after each key;
- `interval_ms`: between consecutive frame rows' `qpc`; `long_intervals`: intervals above 1.5 refreshes (the median
  interval), in the whole run and in the key windows (from the last frame before the key-down to the frame that shows
  it, plus one);
- `shown_ms`: from the key-down to the `display_qpc` of the frame the key row names, when that frame has a presented
  row (key_to_frame.py's rule); `frames_to_shown`: frames submitted after the key-down up to and including that frame.
It reads only and writes only `--out`, which must not exist yet. The record holds run IDs and counts, no local path.
Evidence kind: what the shell's own logs show (PHS.64).
"""
from __future__ import annotations

import argparse
import bisect
import hashlib
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
FORMAT = "magic600-key-stall-20261126-ab-keys-v1"


def quantiles(values):
    if not values:
        return None
    values = sorted(values)
    pick = lambda q: round(values[min(len(values) - 1, int(q * len(values)))], 2)  # noqa: E731
    return {"n": len(values), "p50": pick(0.50), "p90": pick(0.90), "p99": pick(0.99), "max": round(values[-1], 2)}


def run(folder: Path) -> dict:
    path = folder / "events.jsonl"
    head, keys, frames, presented = None, [], [], {}
    with path.open(encoding="utf-8") as handle:
        for line in handle:
            row = json.loads(line)
            kind = row.get("t")
            if kind == "run":
                head = row
            elif kind == "key":
                keys.append(row)
            elif kind == "frame":
                frames.append(row)
            elif kind == "presented":
                presented[row["frame_id"]] = row
    tick = 1000.0 / head["qpc_frequency"]
    launch = json.loads((folder / "launch.json").read_text(encoding="utf-8"))
    posts = json.loads((folder / "posts.json").read_text(encoding="utf-8"))
    starts = [frame["qpc"] for frame in frames]
    index_of = {frame["frame_id"]: index for index, frame in enumerate(frames)}
    intervals = [(b - a) * tick for a, b in zip(starts, starts[1:])]
    refresh = sorted(intervals)[len(intervals) // 2]
    long = 1.5 * refresh
    calls = [(frame["returned_qpc"] - frame["qpc"]) * tick for frame in frames if "returned_qpc" in frame]
    after, shown, to_shown, window_long, unconfirmed = [], [], [], 0, 0
    for key in keys:
        first = bisect.bisect_right(starts, key["qpc"])
        for frame in frames[first:first + 4]:
            if "returned_qpc" in frame:
                after.append((frame["returned_qpc"] - frame["qpc"]) * tick)
        named = key.get("frame_id")
        if named in index_of:
            last = index_of[named]
            to_shown.append(last - first + 1)
            window = intervals[max(first - 1, 0):last + 1]
            window_long += sum(interval > long for interval in window)
            if named in presented:
                shown.append((presented[named]["display_qpc"] - key["qpc"]) * tick)
            else:
                unconfirmed += 1
        else:
            unconfirmed += 1
    return {
        "run_id": head.get("run_id"),
        "build_identity": head.get("build_identity"),
        "host": launch.get("host"),
        "puzzle": launch.get("puzzle"),
        "exit_code": launch.get("exit_code"),
        "owner_idle_s_at_start": posts.get("owner_idle_s_at_start"),
        "posts": len(posts["posts"]),
        "posts_with_foreground": sum(post["foreground"] for post in posts["posts"]),
        "keys": len(keys),
        "frames": len(frames),
        "refresh_ms": round(refresh, 3),
        "key_call_ms": quantiles([(key["handled_qpc"] - key["qpc"]) * tick for key in keys if "handled_qpc" in key]),
        "frame_call_ms": quantiles(calls),
        "frame_call_ms_four_after_key": quantiles(after),
        "interval_ms": quantiles(intervals),
        "long_intervals": sum(interval > long for interval in intervals),
        "long_intervals_in_key_windows": window_long,
        "shown_ms": quantiles(shown),
        "unconfirmed_keys": unconfirmed,
        "frames_to_shown": quantiles([float(count) for count in to_shown]),
        "inputs": {"events.jsonl": hashlib.sha256(path.read_bytes()).hexdigest()},
    }


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("folders", type=Path, nargs="+")
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args(argv)
    if args.out.exists():
        print(f"refused: {args.out.name} exists")
        return 2
    runs = [run(folder) for folder in args.folders]
    record = {"format": FORMAT, "evidence": "actual_windows_directx: what the shell's own logs show", "runs": runs}
    args.out.write_bytes((json.dumps(record, indent=2, sort_keys=True) + "\n").encode("utf-8"))
    for one in runs:
        print(f"{one['host']:5} {one['run_id']}: keys {one['keys']} ({one['posts_with_foreground']}/{one['posts']} "
              f"with the foreground), refresh {one['refresh_ms']} ms")
        for name in ("key_call_ms", "frame_call_ms", "frame_call_ms_four_after_key", "interval_ms", "shown_ms",
                     "frames_to_shown"):
            print(f"    {name}: {one[name]}")
        print(f"    long intervals {one['long_intervals']} (in key windows {one['long_intervals_in_key_windows']}), "
              f"unconfirmed keys {one['unconfirmed_keys']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
