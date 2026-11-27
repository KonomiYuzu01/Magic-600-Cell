"""Run one ADR-D02 gate row in the product and print its D02_GATE line (build steps B19 and B24).

  python -B renderer/shell/gate_d02.py --row ID [--out NEW_FOLDER] [--seconds 60] [--warmup 10] [--state ID]
                                       [--build DIR] [--resolution WxH] [--foreground]
  python -B renderer/shell/gate_d02.py --occlusion [--out NEW_FOLDER] [--state ID] [--grid 24x14] [--region x0,y0,x1,y1]
                                       [--resolution WxH] [--foreground]
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

A sweeping row's script selects its grip cap (grip.select) before the repeat block; the run moves that selection and
the cap naming to an admissible cap of the run's state (its sample rim.json) when the script's cap is not one, and
gate.json records the cap. The window is the display's native size, read at run time (`--resolution` overrides), and
gate.json records it with its source. `--foreground` (the owner's sitting only) lets the window take the foreground
once at the start, never topmost; without it the owner's window rule holds as in every run (d02_run.py), and gate.json
records which.

Windows only; a run opens the product window on the GPU (machine mark W). A deep state takes about four minutes to
reopen. Exit 0 pass, 1 fail, 2 usage or refused, 3 invalid.
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
# "local" (LF1): the jumble.slice.shell argument the row's script sets before its repeat block, the Local mode the
# row names: `default` for Local's slice, the shell's limit 3.14 for Local's whole chain (G6's carries the sweeps,
# G7's does not; LF2 set G7's).
ROWS = {
    "G1": {"sitting": 1, "state": "deep-2", "panes": 1, "sweeps": True,
           "views": "Local hidden, Global with the contact-class tint, twist sweeps repeating: the one-view baseline"},
    "G9": {"sitting": 1, "state": "deep-2", "panes": 2, "sweeps": True, "views": "G1's script with Local shown"},
    "G2": {"sitting": 1, "state": "wj-ia-end", "panes": 2, "sweeps": False,
           "views": "two views, a blocked cap named (cap 0), with all line layers"},
    "G3": {"sitting": 1, "state": "wj-ia-300", "panes": 2, "sweeps": False,
           "views": "two views, the blocked grip with the most recorded certificate points (cap 0, 462), its glyphs"},
    "G4": {"sitting": 2, "state": "deep-2", "panes": 2, "sweeps": True, "local": "default",
           "views": "Global chain, Local slice, twists of the named cap"},
    "G5": {"sitting": 2, "state": "deep-2", "panes": 2, "sweeps": False, "views": "Global chain, Local raw"},
    "G6": {"sitting": 2, "state": "deep-2", "panes": 2, "sweeps": True, "local": "3.14",
           "views": "Global ring sheet, Local chain carrying the sweeps"},
    "G7": {"sitting": 2, "state": "deep-2", "panes": 2, "sweeps": False, "local": "3.14",
           "views": "Global Hopf rings, Local chain"},
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
SAMPLES = ROOT / "tools" / "looklab" / "data" / "samples" / "jumble"


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


def native_resolution() -> str:
    """The primary display's size in physical pixels at run time (DESKTOPHORZRES and DESKTOPVERTRES, which ignore
    the process's DPI scaling), as WxH."""
    import ctypes
    user32, gdi32 = ctypes.WinDLL("user32"), ctypes.WinDLL("gdi32")
    user32.GetDC.restype = ctypes.c_void_p
    user32.GetDC.argtypes = (ctypes.c_void_p,)
    user32.ReleaseDC.argtypes = (ctypes.c_void_p, ctypes.c_void_p)
    gdi32.GetDeviceCaps.argtypes = (ctypes.c_void_p, ctypes.c_int)
    dc = user32.GetDC(None)
    if not dc:
        raise OSError("the display's device context is not available")
    try:
        width, height = gdi32.GetDeviceCaps(dc, 118), gdi32.GetDeviceCaps(dc, 117)  # DESKTOPHORZRES, DESKTOPVERTRES
    finally:
        user32.ReleaseDC(None, dc)
    if width <= 0 or height <= 0:
        raise OSError("the display reported no size")
    return f"{width}x{height}"


def window_size(resolution: str | None, read=native_resolution) -> dict:
    """The window's size and where it came from: `--resolution`, else the display's native size read now."""
    if resolution:
        return {"resolution": resolution, "source": "--resolution"}
    return {"resolution": read(), "source": "native display size (DESKTOPHORZRES x DESKTOPVERTRES at run time)"}


def admissible_caps(state: str) -> list[int]:
    """The state's admissible caps, lowest first, from its sample rim (tools/looklab/data/samples/jumble/<state>/rim.json)."""
    rim = SAMPLES / state / "rim.json"
    if not rim.is_file():
        raise ValueError(f"state {state} has no sample rim.json: its admissible caps are unknown")
    rows = json.loads(rim.read_text(encoding="utf-8"))["rows"]
    return sorted(row["cap"] for row in rows if row.get("status") == "admissible" and not row.get("blocked"))


def sweep_cap(state: str, written: int) -> int:
    """The cap a sweeping row selects at `state`: the script's own when it is admissible there, else the lowest
    admissible cap with its own sample file (cap-<n>.json), else the lowest admissible cap."""
    caps = admissible_caps(state)
    if not caps:
        raise ValueError(f"state {state} has no admissible cap to sweep")
    if written in caps:
        return written
    sampled = [cap for cap in caps if (SAMPLES / state / f"cap-{cap}.json").is_file()]
    return (sampled or caps)[0]


def at_cap(script: dict, cap: int) -> dict:
    """The script with its grip selection and its cap naming (a command step or a palette line) at `cap`."""
    steps = []
    for step in script["steps"]:
        if step.get("command") in ("grip.select", "jumble.cap.name") and "argument" in step:
            step = {**step, "argument": str(cap)}
        elif str(step.get("text", "")).startswith("> jumble.cap.name "):
            step = {**step, "text": f"> jumble.cap.name {cap}"}
        steps.append(step)
    return {**script, "steps": steps}


def prepare(name: str, out: Path, seconds: float, resolution: str, state: str | None = None,
            sweeps: bool = False) -> tuple[dict, Path]:
    written = json.loads((d02_run.SCRIPTS / f"{name}.json").read_text(encoding="utf-8"))
    if sweeps:
        selected = [step for step in written["steps"] if step.get("command") == "grip.select" and "argument" in step]
        if not selected:
            raise ValueError(f"{name} sweeps but selects no grip cap before its repeat block")
        cap = sweep_cap(state or written["state"], int(selected[0]["argument"]))
        written = {**at_cap(written, cap), "grip_cap": cap}
    script = d02_run.expand_repeat(written, seconds)
    script["resolution"] = resolution
    path = out.parent / f"{out.name}.{name}.json"
    path.write_text(json.dumps(script, indent=1) + "\n", encoding="utf-8", newline="\n")
    return script, path


def launch_run(name: str, path: Path, out: Path, state: str, build: Path | None, limit: float, foreground: bool = False):
    memory = GpuMemory()
    d02_run.RUN_TIMEOUT_S = max(d02_run.RUN_TIMEOUT_S, limit)
    try:
        code, record = d02_run.run(name, out, build, state, launcher=gated_launcher(lambda p: memory.start(p.pid)), path=path,
                                   foreground=gate_row.take_foreground if foreground else None)
    finally:
        memory.stop()
    if out.is_dir():
        (out / VRAM).write_text("".join(json.dumps(s) + "\n" for s in memory.samples), encoding="utf-8", newline="\n")
        path.replace(out / path.name)
    return code, record, memory


def run_row(row: str, out: Path, seconds: float, warmup: float, state: str | None, build: Path | None,
            resolution: str | None, show=print, foreground: bool = False) -> int:
    state = state or ROWS[row]["state"]
    name = f"gate-{row.lower()}"
    window = window_size(resolution)
    script, path = prepare(name, out, seconds + warmup, window["resolution"], state, ROWS[row]["sweeps"])
    code, record, memory = launch_run(name, path, out, state, build, REOPEN_S.get(state, 120.0) + seconds + warmup + 90,
                                      foreground)
    gate = {"format": FORMAT, "row": row, "state": state, "script": name, "seconds": seconds, "warmup": warmup,
            "window": window, "foreground": foreground, "grip_cap": script.get("grip_cap"),
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


def opening_clicks(dumps: list[dict], events: Path) -> dict:
    """Where Global's opening sat, as the shell reports it (the dump's frame.global_opening, window pixels), and how
    many of the run's clicks (events.jsonl) landed inside it."""
    rects = [tuple(dump["frame"]["global_opening"]) for dump in dumps
             if isinstance(dump.get("frame"), dict) and "global_opening" in dump["frame"]]
    rows = gate_row.read_rows(events) if events.is_file() else []
    clicks = [(row["x"], row["y"]) for row in rows if row.get("t") == "click" and "x" in row]
    if not rects:
        return {"global_opening": None, "clicks": len(clicks), "inside": 0}
    x, y, w, h = rects[-1]
    inside = sum(1 for cx, cy in clicks if x - 0.5 <= cx <= x + w + 0.5 and y - 0.5 <= cy <= y + h + 0.5)
    return {"global_opening": list(rects[-1]), "moved": len(set(rects)) > 1, "clicks": len(clicks), "inside": inside}


def occlusion_script(state: str, grid: tuple[int, int], region: tuple[float, float, float, float],
                     resolution: str = "2000x1200") -> dict:
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
    return {"format": "magic600-d02-script/1", "name": d02_run.GATE_OCCLUSION, "state": state, "resolution": resolution,
            "note": "B24's occlusion measure, written by gate_d02.py: both chart presets at the default camera, a grid "
                    "of clicks over Global, one pick pass each; the distinct selected pieces are counted per preset. "
                    "A click step's fractions are of Global's opening as the shell lays it out at run time "
                    "(ShellPick.ScriptClick), so the grid follows Global's pane; the run checks each click against the "
                    "opening the dump reports.",
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


def run_occlusion(out: Path, state: str, build: Path | None, grid: tuple[int, int], region, show=print,
                  resolution: str | None = None, foreground: bool = False) -> int:
    window = window_size(resolution)
    script = occlusion_script(state, grid, region, window["resolution"])
    path = out.parent / f"{out.name}.{d02_run.GATE_OCCLUSION}.json"
    path.write_text(json.dumps(script, indent=1) + "\n", encoding="utf-8", newline="\n")
    clicks = grid[0] * grid[1] * len(OCCLUSION_PRESETS)
    code, record, _ = launch_run(d02_run.GATE_OCCLUSION, path, out, state, build, REOPEN_S.get(state, 120.0) + clicks + 120,
                                 foreground)
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
    placed = opening_clicks(dumps, out / "events.jsonl")
    problems = []
    if placed["global_opening"] is None:
        problems.append("the dumps report no Global opening")
    elif placed["inside"] != placed["clicks"] or placed["clicks"] < clicks:
        problems.append(f"{placed['inside']} of {placed['clicks']} clicks (of {clicks}) landed in Global's opening")
    record_out = {"format": FORMAT, "occlusion": lines, "state": state, "region": list(region),
                  "region_of": "fractions of Global's opening as the shell lays it out", "placement": placed,
                  "window": window, "foreground": foreground, "problems": problems,
                  "run": {"code": code, "mismatches": record["mismatches"]},
                  "reading": "a sampled count: pieces no grid pick reached may still show between the grid points"}
    if out.is_dir():
        (out / RECORD).write_text(json.dumps(record_out, indent=2) + "\n", encoding="utf-8", newline="\n")
    for text in lines:
        show(text)
    for problem in problems:
        show("  not a valid run: " + problem)
    return 0 if code == 0 and not problems else 3


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
    parser.add_argument("--resolution", help="the window size, WxH (default: the display's native size, read at run time)")
    parser.add_argument("--foreground", action="store_true",
                        help="the window takes the foreground once at the start, never topmost (the owner's sitting only)")
    parser.add_argument("--grid", default="24x14", help="the occlusion grid, columns x rows")
    parser.add_argument("--region", default="0,0,1,1",
                        help="the occlusion grid's fractions of Global's opening as the shell lays it out (default: all of it)")
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
            return run_occlusion(out, args.state or "deep-2", args.build, (columns, rows), region,
                                 resolution=args.resolution, foreground=args.foreground)
        return run_row(args.row, out, args.seconds, args.warmup, args.state, args.build, args.resolution,
                       foreground=args.foreground)
    except (ValueError, OSError) as error:
        print(f"gate_d02: {error}")
        return 2


if __name__ == "__main__":
    sys.exit(main())
