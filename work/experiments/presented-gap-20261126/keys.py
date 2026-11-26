"""How long the shell's frame loop stalls on a turn key: the key's handling time and the frame interval around it.

    python -B work/experiments/presented-gap-20261126/keys.py <run folder> [<run folder> ...] --out <file>

Each run folder is one that `renderer/shell/launch.py --out` wrote. From `events.jsonl` the record gives per run the
`key` rows (the shell's measured presses), `handled_ms` (`handled_qpc - qpc`: from the key-down's arrival until
`m6h_key_down` returned on the frame thread, when the build logs `handled_qpc`), the intervals between consecutive
`frame` rows (`qpc`, when the shell handed the frame to the core) above 40 ms, and the interval that contains each key.
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

FORMAT = "magic600-presented-gap-20261126-keys-v1"


def quantiles(values: list[float]) -> dict | None:
    if not values:
        return None
    values = sorted(values)
    pick = lambda q: round(values[min(len(values) - 1, int(q * len(values)))], 3)  # noqa: E731
    return {"n": len(values), "p10": pick(0.10), "p50": pick(0.50), "p90": pick(0.90), "p99": pick(0.99),
            "max": round(values[-1], 3)}


def keys(folder: Path) -> dict:
    path = folder / "events.jsonl"
    presses, frames, head = [], [], None
    with path.open(encoding="utf-8") as handle:
        for line in handle:
            if '"t":"key"' in line:
                presses.append(json.loads(line))
            elif '"t":"frame"' in line:
                frames.append(json.loads(line)["qpc"])
            elif '"t":"run"' in line:
                head = json.loads(line)
    if head is None or head.get("qpc_frequency") != 10_000_000 or len(frames) < 2:
        raise SystemExit(f"refused: {folder.name} has no run row, another QPC frequency or no frames")
    intervals = [(b - a) / 10_000 for a, b in zip(frames, frames[1:])]
    around = []
    for press in presses:
        index = bisect.bisect_right(frames, press["qpc"])
        if 0 < index < len(frames):
            around.append(intervals[index - 1])
    handled = [(press["handled_qpc"] - press["qpc"]) / 10_000 for press in presses if "handled_qpc" in press]
    return {
        "run_id": head.get("run_id"),
        "build_identity": head.get("build_identity"),
        "keys": len(presses),
        "frames": len(frames),
        "handled_ms": quantiles(handled),
        "frame_intervals_above_40_ms": sum(interval > 40 for interval in intervals),
        "interval_around_key_ms": quantiles(around),
        "inputs": {"events.jsonl": hashlib.sha256(path.read_bytes()).hexdigest()},
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("folders", type=Path, nargs="+")
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args(argv)
    if args.out.exists():
        print(f"refused: {args.out.name} exists")
        return 2
    runs = [keys(folder) for folder in args.folders]
    record = {"format": FORMAT, "evidence": "actual_windows_directx: what the shell's own logs show", "runs": runs}
    args.out.write_bytes((json.dumps(record, indent=2, sort_keys=True) + "\n").encode("utf-8"))
    for run in runs:
        print(f"{run['run_id']} {run['build_identity']}: {run['keys']} keys, {run['frame_intervals_above_40_ms']} frame "
              f"intervals above 40 ms, handled {run['handled_ms']}, around a key {run['interval_around_key_ms']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
