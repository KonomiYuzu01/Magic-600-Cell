"""One ADR-D02 gate row's numbers from a product run's event log (build steps B19 and B24).

  python -B tools/perf/d02/gate_row.py --events RUN/events.jsonl --row G1 [--seconds 60] [--warmup 10]
                                       [--vram RUN/gate-vram.jsonl] [--json OUT]

It prints exactly one line:

  D02_GATE row=<id> panes=<n> view_vertex_evals=<v> fps_avg=<x> p99_ms=<y> vram_gb=<z> pass=<bool>

against the renderer selection gate (docs/wiki/decisions/renderer-candidates.md): an average of at least 30 fps, a
99th-percentile frame time of at most 33.3 ms and a peak VRAM of at most about 7 GB (taken here as 7.0 GiB).

- Frames: the `presented` rows, the presentations the core confirmed, ordered by `display_qpc` (the confirming
  statistics row's SyncQPCTime), with the run row's `qpc_frequency`. A frame interval is the gap between two
  consecutive confirmed presentations; a presentation the core could not confirm shows as a longer gap, so the
  reading errs towards failing.
- Warm-up: the window is the last `--seconds` seconds of confirmed presentations before the shell quit. Everything
  before it (the launch, the state's open, the views' setup and the first `--warmup` seconds of the row's motion) is
  dropped. The row scripts run their motion for warm-up plus window seconds (renderer/shell/gate_d02.py), so the
  window lies in the motion; a run whose presentations span less than warm-up plus window is refused.
- Panes: the frame rows submitted in the window (a composite frame names `panes`; a one-view frame names none and
  counts as 1). The line prints the most panes seen; the record lists every count.
- view_vertex_evals: the core's own pipeline statistics, from the gate fields of the frame rows (`core_frames_reported`,
  `core_view_vertex_evals_total`, written with M600_D02_GATE=1 through `sa2_counters`): the total's growth over the
  window divided by the reported frames' growth, the mean per frame. Without those fields it prints `not_measured`.
- Surface resizes (SF5): the shell's `surface` rows in the window, one per resize of the core's surface (each costs
  one long presented interval); the record gives the count, the line does not.
- VRAM: the peak of `--vram`'s samples in the window, the shell process's dedicated GPU memory (the GPU Process Memory
  counters' Dedicated Usage, the figure Task Manager shows), sampled from outside by gate_d02.py. Without samples it
  prints `vram_gb=not_measured` and `pass` is computed on the rate and the 99th percentile only (the record says so).

A missing required field (no run row, no `qpc_frequency`, a presented row without `display_qpc`) refuses the row:
exit 2 and no D02_GATE line. Exit 0 when the row passes, 1 when it fails.
"""
from __future__ import annotations

import sys

sys.dont_write_bytecode = True

import argparse
import json
import math
from pathlib import Path

FPS_MIN = 30.0
P99_MAX_MS = 33.3
VRAM_MAX_GB = 7.0
GIB = 1 << 30
DEFAULT_SECONDS = 60.0
DEFAULT_WARMUP = 10.0
FORMAT = "magic600-d02-gate-row-v1"


class Refused(ValueError):
    """The run cannot give this row's numbers; never a pass."""


def percentile(values: list[float], q: float) -> float:
    """The nearest-rank percentile: the smallest value with at least q percent of the values at or below it."""
    if not values:
        raise Refused("no frame intervals in the window")
    ordered = sorted(values)
    rank = max(1, math.ceil(q / 100.0 * len(ordered)))
    return ordered[rank - 1]


def read_rows(path: Path) -> list[dict]:
    if not path.is_file():
        raise Refused(f"no event log at {path.name}")
    rows = []
    for number, text in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        if text.strip():
            try:
                rows.append(json.loads(text))
            except json.JSONDecodeError as error:
                raise Refused(f"line {number} of {path.name} is not JSON: {error.msg}") from None
    return rows


def frequency(rows: list[dict]) -> int:
    runs = [row for row in rows if row.get("t") == "run"]
    if not runs:
        raise Refused("the event log has no run row (the shell did not close)")
    value = runs[0].get("qpc_frequency")
    if not isinstance(value, int) or value <= 0:
        raise Refused("the run row has no qpc_frequency")
    return value


def presentations(rows: list[dict]) -> list[int]:
    times = []
    for index, row in enumerate(rows):
        if row.get("t") != "presented":
            continue
        value = row.get("display_qpc")
        if not isinstance(value, int) or value <= 0:
            raise Refused(f"presented row {index} has no display_qpc")
        times.append(value)
    if len(times) < 2:
        raise Refused("the event log has fewer than two presented rows")
    return sorted(times)


def vertex_evals(frames: list[dict]):
    """The mean view vertex evaluations per reported frame over the window, or None without the gate fields."""
    counted = [row for row in frames
               if isinstance(row.get("core_frames_reported"), int) and isinstance(row.get("core_view_vertex_evals_total"), int)]
    if len(counted) < 2:
        return None
    first, last = counted[0], counted[-1]
    reported = last["core_frames_reported"] - first["core_frames_reported"]
    if reported <= 0:
        return None
    return round((last["core_view_vertex_evals_total"] - first["core_view_vertex_evals_total"]) / reported)


def surface_resizes(rows: list[dict], start: int, end: int) -> int:
    """SF5: the resizes of the core's surface in the window, the shell's `surface` rows (each costs one long presented
    interval: DXGI's statistics go disjoint after ResizeBuffers)."""
    return sum(1 for row in rows if row.get("t") == "surface" and isinstance(row.get("qpc"), int) and start <= row["qpc"] <= end)


def vram_peak(samples: list[dict], start: int, end: int):
    inside = [s["dedicated_bytes"] for s in samples
              if isinstance(s.get("qpc"), int) and isinstance(s.get("dedicated_bytes"), int) and start <= s["qpc"] <= end]
    return max(inside) if inside else None


def measure(rows: list[dict], seconds: float = DEFAULT_SECONDS, warmup: float = DEFAULT_WARMUP,
            vram: list[dict] | None = None) -> dict:
    """The row's numbers from the event rows (and the VRAM samples), as the record writes them."""
    if seconds <= 0 or warmup < 0:
        raise Refused("the window must be positive and the warm-up not negative")
    freq = frequency(rows)
    times = presentations(rows)
    span = (times[-1] - times[0]) / freq
    if span + 1e-9 < seconds + warmup:
        raise Refused(f"the run presented {span:.1f} s, less than the warm-up {warmup:g} s plus the window {seconds:g} s")
    end = times[-1]
    start = end - round(seconds * freq)
    window = [t for t in times if t >= start]
    intervals = [(b - a) * 1000.0 / freq for a, b in zip(window, window[1:])]
    if not intervals:
        raise Refused("no frame intervals in the window")
    covered = (window[-1] - window[0]) / freq
    fps = (len(window) - 1) / covered if covered > 0 else 0.0
    p99 = percentile(intervals, 99.0)
    frames = [row for row in rows if row.get("t") == "frame" and isinstance(row.get("qpc"), int) and start <= row["qpc"] <= end]
    if not frames:
        raise Refused("no frame rows were submitted in the window")
    counts: dict[str, int] = {}
    for row in frames:
        key = str(row.get("panes") or 1)
        counts[key] = counts.get(key, 0) + 1
    panes = max(int(key) for key in counts)
    held = sum(1 for row in frames if row.get("held"))
    evals = vertex_evals(frames)
    peak = vram_peak(vram or [], start, end)
    vram_gb = None if peak is None else peak / GIB
    passed = fps >= FPS_MIN and p99 <= P99_MAX_MS and (vram_gb is None or vram_gb <= VRAM_MAX_GB)
    whole = vram_peak(vram or [], times[0], end)
    return {"format": FORMAT, "window_s": seconds, "warmup_s": warmup, "presented_span_s": round(span, 3),
            "window_frames": len(window), "fps_avg": fps, "p99_ms": p99, "max_interval_ms": max(intervals),
            "panes": panes, "pane_counts": counts, "held_frames": held, "view_vertex_evals": evals,
            "surface_resizes": surface_resizes(rows, start, end), "vram_gb": vram_gb, "vram_gb_whole_run": None if whole is None else whole / GIB,
            "vram_measured": peak is not None, "pass": passed,
            "pass_basis": "fps, p99 and VRAM" if peak is not None else "fps and p99 only: VRAM not measured",
            "limits": {"fps_min": FPS_MIN, "p99_max_ms": P99_MAX_MS, "vram_max_gib": VRAM_MAX_GB}}


def take_foreground(window: int) -> bool:
    """gate_d02.py --foreground, for the owner's sitting (owner, 26 November 2026: measurement runs that need the
    foreground keep their own rules): one SetForegroundWindow of the shell's window at the start, never a topmost
    style. True when the window then has the foreground; Windows refuses it to a run not started from the desktop."""
    import ctypes
    from ctypes import wintypes
    user32 = ctypes.WinDLL("user32")
    user32.SetForegroundWindow.argtypes = (wintypes.HWND,)
    user32.GetForegroundWindow.restype = wintypes.HWND
    user32.SetForegroundWindow(window)
    return int(user32.GetForegroundWindow() or 0) == window


def line(row_id: str, result: dict) -> str:
    evals = "not_measured" if result["view_vertex_evals"] is None else str(result["view_vertex_evals"])
    vram = "not_measured" if result["vram_gb"] is None else f"{result['vram_gb']:.2f}"
    return (f"D02_GATE row={row_id} panes={result['panes']} view_vertex_evals={evals} fps_avg={result['fps_avg']:.2f} "
            f"p99_ms={result['p99_ms']:.2f} vram_gb={vram} pass={'true' if result['pass'] else 'false'}")


def main(argv: list[str] | None = None, out=print) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--events", type=Path, required=True, help="the run's events.jsonl")
    parser.add_argument("--row", required=True, help="the gate row's id (G1 ... G11)")
    parser.add_argument("--seconds", type=float, default=DEFAULT_SECONDS, help="the measured window (default 60)")
    parser.add_argument("--warmup", type=float, default=DEFAULT_WARMUP, help="the motion dropped before it (default 10)")
    parser.add_argument("--vram", type=Path, help="gate-vram.jsonl, the dedicated GPU memory samples")
    parser.add_argument("--json", type=Path, help="write the record here")
    args = parser.parse_args(argv)
    try:
        samples = read_rows(args.vram) if args.vram is not None and args.vram.is_file() else None
        result = measure(read_rows(args.events), args.seconds, args.warmup, samples)
    except Refused as error:
        out(f"D02_GATE_REFUSED row={args.row}: {error}")
        return 2
    result["row"] = args.row
    result["line"] = line(args.row, result)
    if args.json is not None:
        args.json.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8", newline="\n")
    out(result["line"])
    return 0 if result["pass"] else 1


if __name__ == "__main__":
    sys.exit(main())
