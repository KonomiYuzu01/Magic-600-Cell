"""Run one ADR-D02 gate row in the product and print its D02_GATE line (build steps B19 and B24).

  python -B renderer/shell/gate_d02.py --row ID [--out NEW_FOLDER] [--seconds 60] [--warmup 10] [--state ID]
                                       [--build DIR] [--resolution WxH]
  python -B renderer/shell/gate_d02.py --occlusion [--out NEW_FOLDER] [--state ID] [--grid 24x14] [--region x0,y0,x1,y1]
  python -B renderer/shell/gate_d02.py --list

A row launches the product through d02_run.py's launcher and player (`--frame --puzzle jumble-on --d02`, the row's
state's session fixture, its script and the event log), with M600_D02_GATE=1 so that every frame row carries the core's
counters (CoreSurface.GateFields). The row's script (renderer/shell/d02_scripts/gate-<id>.json) sets its views and
holds one repeat block, which d02_run.expand_repeat unrolls for warm-up plus window seconds as quiet steps (no screen
dump in the timed frames). While the shell runs, the shell process's dedicated GPU memory is sampled every 0.25 s
from outside (the GPU Process Memory counters' Dedicated Usage, through PDH; the figure Task Manager shows), into
gate-vram.jsonl. Then tools/perf/d02/gate_row.py reads events.jsonl and the samples and prints

  D02_GATE row=<id> panes=<n> view_vertex_evals=<v> fps_avg=<x> p99_ms=<y> vram_gb=<z> pass=<bool>

and gate.json in the run folder holds the line, the record and the row's validity. A row is invalid (exit 3, never a
pass) when the window's pane count is not the row's, or a row whose script sweeps committed no twist in the window.
A failing row is recorded as it is and goes to the owner; it is never tuned away (docs/progress/1.0/adr-d02-gate-results.md).

`--occlusion` (B24) opens the state at the default camera under both chart presets (ring sheet, then Hopf rings) and
clicks a grid over Global's part of the window; each click is one pick pass, and the distinct pieces the picks select
give a sampled count of the pieces each preset shows, printed as one D02_OCCLUSION line per preset.

Windows only; a run opens the product window on the GPU (machine mark W) and keeps the owner's window rule
(d02_run.py). A deep state takes about four minutes to reopen. Exit 0 pass, 1 fail, 2 usage or refused, 3 invalid.
"""
from __future__ import annotations

import sys

sys.dont_write_bytecode = True

import argparse
import json
import threading
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import d02_run  # noqa: E402
import launch  # noqa: E402

ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / "tools" / "perf" / "d02"))
import gate_row  # noqa: E402

RECORD = "gate.json"
VRAM = "gate-vram.jsonl"
FORMAT = "magic600-d02-gate-v1"
# Row -> sitting, default state, panes the window must show, and whether its script sweeps (framework B19 and B24).
ROWS = {
    "G1": {"sitting": 1, "state": "deep-2", "panes": 1, "sweeps": True,
           "views": "Local hidden, Global with the contact-class tint, twist sweeps repeating: the one-view baseline"},
    "G9": {"sitting": 1, "state": "deep-2", "panes": 2, "sweeps": True, "views": "G1's script with Local shown"},
    "G2": {"sitting": 1, "state": "wj-ia-end", "panes": 2, "sweeps": False,
           "views": "two views, a blocked cap named (cap 0), with all line layers"},
    "G3": {"sitting": 1, "state": "wj-ia-300", "panes": 2, "sweeps": False,
           "views": "two views, the blocked grip with the most recorded certificate points (cap 0, 462), its glyphs"},
    "G4": {"sitting": 2, "state": "deep-2", "panes": 2, "sweeps": True,
           "views": "Global chain, Local slice, twists of the named cap"},
    "G5": {"sitting": 2, "state": "deep-2", "panes": 2, "sweeps": False, "views": "Global chain, Local raw"},
    "G6": {"sitting": 2, "state": "deep-2", "panes": 2, "sweeps": True,
           "views": "Global ring sheet, Local chain carrying the sweeps"},
    "G7": {"sitting": 2, "state": "deep-2", "panes": 2, "sweeps": False, "views": "Global Hopf rings, Local chain"},
    "G10": {"sitting": 2, "state": "deep-2", "panes": 3, "sweeps": False,
            "views": "Global chain, Local slice with the split held: three panes"},
    "G11": {"sitting": 2, "state": "deep-2", "panes": 3, "sweeps": False,
            "views": "Global ring sheet, Local raw with the antipode: three panes, the heaviest case"},
    "G8": {"sitting": 2, "state": "deep-2", "panes": 2, "sweeps": True,
           "views": "the simulation matrix held (B14's view style), on G9's two views"},
}
ORDER = ["G1", "G9", "G2", "G3", "G4", "G5", "G6", "G7", "G10", "G11", "G8"]
# A deep state reopens in about four minutes (B5's index); the run's limit leaves room for it.
REOPEN_S = {"deep-1": 420.0, "deep-2": 420.0}
OCCLUSION_PRESETS = ("ring_sheet", "hopf_rings")


def dedicated_peak(items: list[tuple[str, int]], pid: int):
    """(luid, bytes): the adapter with the most dedicated memory of process `pid`, from the counter array's
    instances ("pid_<pid>_luid_<hi>_<lo>_phys_<n>"), summed per adapter; None when the process has none yet."""
    prefix = f"pid_{pid}_luid_"
    per: dict[str, int] = {}
    for name, value in items:
        if name.startswith(prefix):
            luid = name[len(prefix):].split("_phys_")[0]
            per[luid] = per.get(luid, 0) + int(value)
    if not per:
        return None
    luid = max(per, key=per.get)
    return luid, per[luid]


class GpuMemory:
    """Samples a process's dedicated GPU memory from outside through PDH (Windows), with the QPC of each sample."""

    COUNTER = "\\GPU Process Memory(*)\\Dedicated Usage"

    def __init__(self, period: float = 0.25):
        self.period, self.samples, self.error = period, [], None
        self._stop = threading.Event()
        self._thread: threading.Thread | None = None

    def start(self, pid: int):
        self._thread = threading.Thread(target=self._run, args=(pid,), daemon=True)
        self._thread.start()

    def stop(self):
        self._stop.set()
        if self._thread is not None:
            self._thread.join(5)

    def _run(self, pid: int):
        try:
            import ctypes
            from ctypes import wintypes

            class Value(ctypes.Structure):
                _fields_ = [("status", wintypes.DWORD), ("large", ctypes.c_longlong)]

            class Item(ctypes.Structure):
                _fields_ = [("name", wintypes.LPWSTR), ("value", Value)]

            pdh, kernel32 = ctypes.WinDLL("pdh"), ctypes.WinDLL("kernel32")
            handle = ctypes.c_void_p
            pdh.PdhOpenQueryW.argtypes = (wintypes.LPCWSTR, ctypes.c_size_t, ctypes.POINTER(handle))
            pdh.PdhAddEnglishCounterW.argtypes = (handle, wintypes.LPCWSTR, ctypes.c_size_t, ctypes.POINTER(handle))
            pdh.PdhCollectQueryData.argtypes = (handle,)
            pdh.PdhGetFormattedCounterArrayW.argtypes = (handle, wintypes.DWORD, ctypes.POINTER(wintypes.DWORD),
                                                         ctypes.POINTER(wintypes.DWORD), ctypes.c_void_p)
            pdh.PdhCloseQuery.argtypes = (handle,)
            query, counter = handle(), handle()
            for name, status in (("PdhOpenQueryW", pdh.PdhOpenQueryW(None, 0, ctypes.byref(query))),
                                 ("PdhAddEnglishCounterW", pdh.PdhAddEnglishCounterW(query, self.COUNTER, 0, ctypes.byref(counter)))):
                if status != 0:
                    raise OSError(f"{name} answered 0x{status & 0xFFFFFFFF:08x}")
            while not self._stop.is_set():
                if pdh.PdhCollectQueryData(query) == 0:
                    size, count = wintypes.DWORD(0), wintypes.DWORD(0)
                    pdh.PdhGetFormattedCounterArrayW(counter, 0x400, ctypes.byref(size), ctypes.byref(count), None)
                    buffer = (ctypes.c_byte * max(size.value, 1))()
                    if size.value and pdh.PdhGetFormattedCounterArrayW(counter, 0x400, ctypes.byref(size),
                                                                       ctypes.byref(count), buffer) == 0:
                        rows = ctypes.cast(buffer, ctypes.POINTER(Item))
                        items = [(rows[i].name or "", rows[i].value.large) for i in range(count.value) if rows[i].value.status in (0, 1)]
                        qpc = ctypes.c_int64()
                        kernel32.QueryPerformanceCounter(ctypes.byref(qpc))
                        peak = dedicated_peak(items, pid)
                        if peak is not None:
                            self.samples.append({"qpc": qpc.value, "dedicated_bytes": peak[1], "luid": peak[0]})
                self._stop.wait(self.period)
            pdh.PdhCloseQuery(query)
        except (OSError, AttributeError, ValueError) as error:
            self.error = str(error)


def gated_launcher(on_start):
    """d02_run's launcher with M600_D02_GATE=1 in the shell's environment; `on_start` gets the shell's process."""
    def launcher(arguments, popen, out_stream):
        def gated(args, cwd, env, **options):
            process = popen(args, cwd, {**env, "M600_D02_GATE": "1"}, **options)
            on_start(process)
            return process
        return launch.main(arguments, popen=gated, out_stream=out_stream)
    return launcher


def window_checks(row: str, events: Path, result: dict) -> list[str]:
    """Why the run is not a valid run of this row (empty when it is)."""
    problems = []
    want = ROWS[row]["panes"]
    if result["panes"] != want or set(result["pane_counts"]) != {str(want)}:
        problems.append(f"row {row} draws {want} pane(s); the window's frames drew {result['pane_counts']}")
    if ROWS[row]["sweeps"]:
        rows = gate_row.read_rows(events)
        freq = gate_row.frequency(rows)
        end = max(r["display_qpc"] for r in rows if r.get("t") == "presented")
        start = end - round(result["window_s"] * freq)
        committed = [r for r in rows if r.get("t") == "key" and r.get("frame_id") is not None
                     and isinstance(r.get("qpc"), int) and start <= r["qpc"] <= end]
        if not committed:
            problems.append("no twist committed in the window: the row's sweeps did not run (a blocked grip refuses them)")
        result["committed_twists"] = len(committed)
    return problems


def prepare(name: str, out: Path, seconds: float, resolution: str | None) -> tuple[dict, Path]:
    written = json.loads((d02_run.SCRIPTS / f"{name}.json").read_text(encoding="utf-8"))
    script = d02_run.expand_repeat(written, seconds)
    if resolution:
        script["resolution"] = resolution
    path = out.parent / f"{out.name}.{name}.json"
    path.write_text(json.dumps(script, indent=1) + "\n", encoding="utf-8", newline="\n")
    return script, path


def launch_run(name: str, path: Path, out: Path, state: str, build: Path | None, limit: float):
    memory = GpuMemory()
    d02_run.RUN_TIMEOUT_S = max(d02_run.RUN_TIMEOUT_S, limit)
    try:
        code, record = d02_run.run(name, out, build, state, launcher=gated_launcher(lambda p: memory.start(p.pid)), path=path)
    finally:
        memory.stop()
    if out.is_dir():
        (out / VRAM).write_text("".join(json.dumps(s) + "\n" for s in memory.samples), encoding="utf-8", newline="\n")
        path.replace(out / path.name)
    return code, record, memory


def run_row(row: str, out: Path, seconds: float, warmup: float, state: str | None, build: Path | None,
            resolution: str | None, show=print) -> int:
    state = state or ROWS[row]["state"]
    name = f"gate-{row.lower()}"
    _, path = prepare(name, out, seconds + warmup, resolution)
    code, record, memory = launch_run(name, path, out, state, build, REOPEN_S.get(state, 120.0) + seconds + warmup + 90)
    gate = {"format": FORMAT, "row": row, "state": state, "script": name, "seconds": seconds, "warmup": warmup,
            "views": ROWS[row]["views"], "run": {"code": code, "mismatches": record["mismatches"]},
            "vram_samples": len(memory.samples), "vram_error": memory.error,
            "evidence": "actual Windows/DirectX on this machine; gate evidence only from the owner's sitting"}
    try:
        result = gate_row.measure(gate_row.read_rows(out / "events.jsonl"), seconds, warmup, memory.samples or None)
        gate["problems"] = window_checks(row, out / "events.jsonl", result)
        gate["line"] = gate_row.line(row, result)
        gate["result"] = result
    except gate_row.Refused as error:
        gate["refused"] = str(error)
    gate["valid"] = "line" in gate and not gate.get("problems") and code == 0
    if out.is_dir():
        (out / RECORD).write_text(json.dumps(gate, indent=2) + "\n", encoding="utf-8", newline="\n")
    if "line" not in gate:
        show(f"D02_GATE_REFUSED row={row}: {gate.get('refused')}")
        return 2
    show(gate["line"])
    for problem in record["mismatches"] + gate["problems"]:
        show("  not a valid run: " + problem)
    if not gate["valid"]:
        return 3
    return 0 if gate["result"]["pass"] else 1


def occlusion_script(state: str, grid: tuple[int, int], region: tuple[float, float, float, float]) -> dict:
    steps: list[dict] = [{"wait_ms": 4000}]
    columns, rows = grid
    x0, y0, x1, y1 = region
    for preset in OCCLUSION_PRESETS:
        steps += [{"command": f"view.preset.global.{preset}"}, {"wait_ms": 3000}]
        for j in range(rows):
            for i in range(columns):
                x = x0 + (x1 - x0) * (i + 0.5) / columns
                y = y0 + (y1 - y0) * (j + 0.5) / rows
                steps += [{"click": [round(x, 4), round(y, 4)]}, {"wait_ms": 250}]
    return {"format": "magic600-d02-script/1", "name": d02_run.GATE_OCCLUSION, "state": state, "resolution": "2000x1200",
            "note": "B24's occlusion measure, written by gate_d02.py: both chart presets at the default camera, a grid "
                    "of clicks over Global, one pick pass each; the distinct selected pieces are counted per preset.",
            "steps": steps}


def count_picks(dumps: list[dict]) -> dict:
    """Per preset: clicks, hits and distinct pieces, from the dump after each click's wait."""
    found: dict[str, dict] = {}
    preset = None
    for index, dump in enumerate(dumps):
        do = dump.get("do", "")
        if do.startswith("command view.preset.global."):
            preset = do.split("view.preset.global.")[1].split()[0]
            found[preset] = {"clicks": 0, "hits": 0, "pieces": set()}
        elif do.startswith("click") and preset is not None:
            found[preset]["clicks"] += 1
            after = dumps[index + 1] if index + 1 < len(dumps) else dump
            selection = (after.get("pick") or {}).get("selection")
            if selection is not None:
                found[preset]["hits"] += 1
                found[preset]["pieces"].add(selection["piece"])
    return found


def run_occlusion(out: Path, state: str, build: Path | None, grid: tuple[int, int], region, show=print) -> int:
    script = occlusion_script(state, grid, region)
    path = out.parent / f"{out.name}.{d02_run.GATE_OCCLUSION}.json"
    path.write_text(json.dumps(script, indent=1) + "\n", encoding="utf-8", newline="\n")
    clicks = grid[0] * grid[1] * len(OCCLUSION_PRESETS)
    code, record, _ = launch_run(d02_run.GATE_OCCLUSION, path, out, state, build, REOPEN_S.get(state, 120.0) + clicks + 120)
    screen = out / d02_run.SCREEN
    dumps = [json.loads(t) for t in screen.read_text(encoding="utf-8").splitlines() if t.strip()] if screen.is_file() else []
    pieces = len(set(d02_run.slot_pieces()))
    found = count_picks(dumps)
    lines = []
    for preset in OCCLUSION_PRESETS:
        f = found.get(preset, {"clicks": 0, "hits": 0, "pieces": set()})
        seen = len(f["pieces"])
        lines.append(f"D02_OCCLUSION preset={preset} grid={grid[0]}x{grid[1]} clicks={f['clicks']} hits={f['hits']} "
                     f"distinct_pieces={seen} pieces={pieces} seen_fraction={seen / pieces:.4f}")
    record_out = {"format": FORMAT, "occlusion": lines, "state": state, "region": list(region),
                  "run": {"code": code, "mismatches": record["mismatches"]},
                  "reading": "a sampled count: pieces no grid pick reached may still show between the grid points"}
    if out.is_dir():
        (out / RECORD).write_text(json.dumps(record_out, indent=2) + "\n", encoding="utf-8", newline="\n")
    for text in lines:
        show(text)
    return 0 if code == 0 else 3


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--row", choices=ORDER, help="the gate row")
    mode.add_argument("--occlusion", action="store_true", help="B24's occlusion measure of both chart presets")
    mode.add_argument("--list", action="store_true", help="print the rows and their commands")
    parser.add_argument("--out", type=Path, help="the run folder, which must not exist yet (default under work/gate-d02/)")
    parser.add_argument("--seconds", type=float, default=gate_row.DEFAULT_SECONDS, help="the measured window (60)")
    parser.add_argument("--warmup", type=float, default=gate_row.DEFAULT_WARMUP, help="motion dropped before it (10)")
    parser.add_argument("--state", help="override the row's sample state (a dry run; never gate evidence)")
    parser.add_argument("--build", type=Path, help="the product build (default: the newest)")
    parser.add_argument("--resolution", help="the window size, WxH (default: the script's 2000x1200)")
    parser.add_argument("--grid", default="24x14", help="the occlusion grid, columns x rows")
    parser.add_argument("--region", default="0.30,0.25,0.70,0.75", help="the occlusion grid's window fractions")
    args = parser.parse_args(argv)
    if args.list:
        for row in ORDER:
            spec = ROWS[row]
            print(f"{row} (sitting {spec['sitting']}, {spec['state']}, {spec['panes']} pane(s)): {spec['views']}\n"
                  f"  python -B renderer/shell/gate_d02.py --row {row} --out work/gate-d02/{row.lower()}")
        return 0
    import time
    out = (args.out or ROOT / "work" / "gate-d02" / f"{(args.row or 'occlusion').lower()}-{time.strftime('%Y%m%dT%H%M%S')}").resolve()
    if out.exists():
        print(f"gate_d02: {out.name} exists; a run folder must be new")
        return 2
    out.parent.mkdir(parents=True, exist_ok=True)
    try:
        if args.occlusion:
            columns, rows = (int(v) for v in args.grid.lower().split("x"))
            region = tuple(float(v) for v in args.region.split(","))
            if len(region) != 4:
                raise ValueError("--region takes x0,y0,x1,y1")
            return run_occlusion(out, args.state or "deep-2", args.build, (columns, rows), region)
        return run_row(args.row, out, args.seconds, args.warmup, args.state, args.build, args.resolution)
    except (ValueError, OSError) as error:
        print(f"gate_d02: {error}")
        return 2


if __name__ == "__main__":
    sys.exit(main())
