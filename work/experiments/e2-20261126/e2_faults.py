"""Why LX-2's E2 missed in three faults of U7 on 26 November 2026, from one run's shell log, and a model of E2 after the
two changes of this folder's README.

    python -B work/experiments/e2-20261126/e2_faults.py RUN_DIR --out FILE

RUN_DIR is a product run folder (its `events.jsonl`). Each E2 key (a `key` row) belongs to the fault of the newest LX-2
row before it. For each key it reads:

- `submit`: from the key to the return of the frame call that first draws the key's revision (the key row's frame);
- `e2`: from the key to that frame's confirmed display (its `presented` row);
- `adopted`: the first frame from the key on whose engine revision reaches the key's revision, and `lag`, the frames
  from it to the drawing frame (0 when the twist's own publication is drawn: its adoption installs the bundle and
  publishes the publication in one step);
- `probe`: whether the first `recovered` row after the key, the control probe's publication (LX-2's module,
  renderer/shell/project/Lx2/Lx2Module.cs), lies within 20 ms of the drawing frame's call.

The model (not a measurement): (b) the twist's publication is drawn in its adoption frame, as for the keys with lag 0;
(a) a frame reaches the display one refresh sooner than measured (maximum frame latency 1 in place of 2).
Each is applied alone and both together, per fault. The refresh is the median interval of the presented rows.

Writes a JSON record with the input's SHA-256 and no path. Evidence: actual_windows_directx for the measured part, what
the shell's own log shows; the modelled part is a model.
"""
from __future__ import annotations

import argparse
import bisect
import hashlib
import json
import statistics
import sys
from pathlib import Path

sys.dont_write_bytecode = True
PROBE_WINDOW_MS = 20.0


def nearest_rank(values: list[float], p: float) -> float | None:
    if not values:
        return None
    ordered = sorted(values)
    rank = max(1, -(-int(p * 1000) * len(ordered) // 1000))
    return round(ordered[min(rank, len(ordered)) - 1], 2)


def summary(values: list[float]) -> dict:
    return {"n": len(values), "p50": nearest_rank(values, 0.5), "p95": nearest_rank(values, 0.95),
            "max": round(max(values), 2) if values else None, "over_100": sum(v > 100.0 for v in values)}


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("run", type=Path)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args(argv)
    source = args.run / "events.jsonl"
    data = source.read_bytes()
    frames, presented, keys, lx2 = {}, {}, [], []
    frequency = None
    for line in data.decode("utf-8").splitlines():
        try:
            row = json.loads(line)
        except json.JSONDecodeError:
            continue
        kind = row.get("t")
        if kind == "run":
            frequency = row["qpc_frequency"]
        elif kind == "frame":
            frames[row["frame_id"]] = row
        elif kind == "presented":
            presented[row["frame_id"]] = row
        elif kind == "key":
            keys.append(row)
        elif kind == "lx2":
            lx2.append(row)
    if not frequency:
        raise SystemExit("no run row with the QPC frequency")
    ms = 1000.0 / frequency
    lx2.sort(key=lambda r: r["qpc"])
    lx2_qpc = [r["qpc"] for r in lx2]
    ids = sorted(frames)
    frame_qpc = [frames[i]["qpc"] for i in ids]
    shown = sorted(presented)
    refresh = statistics.median((presented[shown[i]]["display_qpc"] - presented[shown[i - 1]]["display_qpc"]) * ms
                                for i in range(1, len(shown)) if shown[i] == shown[i - 1] + 1)
    recovered = [r for r in lx2 if r["event"] == "recovered"]
    recovered_qpc = [r["qpc"] for r in recovered]
    per: dict[str, dict] = {}
    for key in keys:
        at = bisect.bisect_right(lx2_qpc, key["qpc"]) - 1
        fault = lx2[at]["fault"] if at >= 0 else "none"
        frame = frames.get(key["frame_id"])
        if frame is None:
            continue
        revision = frame["publication_revision"]
        start = bisect.bisect_left(frame_qpc, key["qpc"])
        adopted = next((ids[i] for i in range(start, len(ids)) if frames[ids[i]]["engine_revision"] >= revision), None)
        lag = key["frame_id"] - adopted if adopted is not None else None
        after = bisect.bisect_left(recovered_qpc, key["qpc"])
        probe = after < len(recovered) and abs(recovered_qpc[after] - frame["qpc"]) * ms < PROBE_WINDOW_MS
        row = per.setdefault(fault, {"keys": 0, "submit": [], "e2": [], "lag": {}, "slow": 0, "slow_probe": 0,
                                     "model_b": [], "model_a": [], "model_ab": []})
        row["keys"] += 1
        row["submit"].append((frame["returned_qpc"] - key["qpc"]) * ms)
        row["lag"][str(lag)] = row["lag"].get(str(lag), 0) + 1
        shown_row = presented.get(key["frame_id"])
        if shown_row is None:
            continue
        e2 = (shown_row["display_qpc"] - key["qpc"]) * ms
        row["e2"].append(e2)
        if e2 > 100.0:
            row["slow"] += 1
            row["slow_probe"] += int(probe)
        model_frame = adopted if adopted is not None and lag is not None and lag > 0 else key["frame_id"]
        model_row = presented.get(model_frame)
        b = (model_row["display_qpc"] - key["qpc"]) * ms if model_row else e2
        row["model_b"].append(b)
        row["model_a"].append(e2 - refresh)
        row["model_ab"].append(b - refresh)
    faults = {}
    for fault, row in per.items():
        faults[fault] = {"keys": row["keys"], "submit_ms": summary(row["submit"]), "e2_ms": summary(row["e2"]),
                         "lag_frames": dict(sorted(row["lag"].items())),
                         "over_100_drawn_at_the_probe": f"{row['slow_probe']} of {row['slow']}",
                         "model": {"b_twist_publication_first": summary(row["model_b"]),
                                   "a_latency_1": summary(row["model_a"]), "a_and_b": summary(row["model_ab"])}}
    record = {"format": "magic600-e2-faults-20261126-v1",
              "evidence": "actual_windows_directx for the measured fields (the shell's own log); the model fields are "
                          "a model, not a measurement",
              "inputs": {"events.jsonl": hashlib.sha256(data).hexdigest()},
              "refresh_ms": round(refresh, 3), "keys": len(keys), "faults": faults}
    args.out.write_bytes((json.dumps(record, indent=1) + "\n").encode("utf-8"))
    for fault, row in faults.items():
        print(f"{fault:18s} keys {row['keys']:4d} e2 p50 {row['e2_ms']['p50']} p95 {row['e2_ms']['p95']} | lag "
              f"{row['lag_frames']} | slow at probe {row['over_100_drawn_at_the_probe']} | model a+b p95 "
              f"{row['model']['a_and_b']['p95']} (b {row['model']['b_twist_publication_first']['p95']}, a "
              f"{row['model']['a_latency_1']['p95']})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
