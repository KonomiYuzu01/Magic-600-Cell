"""Play a D02 script in the product shell and compare every screen with its expectation (ADR-D02 build step B4).

  python -B renderer/shell/d02_run.py --script NAME --out NEW_FOLDER [--build DIR] [--state ID]

It starts the newest product build (or --build) through launch.py with `--frame --puzzle jumble-on --d02` and the
script renderer/shell/d02_scripts/NAME.json. The shell plays the script inside: each step's chord goes through the
shell's own key entry, and after each step the shell writes the screen's text, pane by pane, to d02-screen.jsonl in
the run folder, then quits after the last step. This script computes each step's expectation from the design and
sample files and prints `M600_D02_RUN_PASS script=NAME steps=N mismatches=0`, or `M600_D02_RUN_FAIL` with the
mismatches, and writes d02-run.json beside the shell's logs.

The owner's window rule (26 November 2026) is checked on the way: the window that has the foreground before the
launch must still have it five seconds after it, the shell's window never carries WS_EX_TOPMOST, and no screen dump
finds the shell in the foreground. A click into the window during the run fails that check; leave it alone. A run
opens a window on the GPU for about half a minute (machine mark W): never during a quiet-machine hold. Exit 0 on a
pass, 1 on a failure, 2 for a usage error.
"""
from __future__ import annotations

import sys

sys.dont_write_bytecode = True

import argparse
import json
import subprocess
import threading
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import launch  # noqa: E402

ROOT = HERE.parents[1]
SCRIPTS = HERE / "d02_scripts"
DESIGN = ROOT / "tools" / "looklab" / "data" / "design"
SAMPLES = ROOT / "tools" / "looklab" / "data" / "samples" / "jumble"
TABLE = ROOT / "docs" / "progress" / "1.0" / "command-table.json"
SCREEN = "d02-screen.jsonl"
RECORD = "d02-run.json"
FORMAT = "magic600-d02-run-v1"
FOREGROUND_AFTER_S = 5.0
RUN_TIMEOUT_S = 240.0


def load(path: Path):
    return json.loads(path.read_text(encoding="utf-8"))


def data() -> dict:
    """What the expectations are computed from: the draft rows, the table's rows and the sample states."""
    return {"rows": load(DESIGN / "command-rows-d02.json")["commands"], "table": load(TABLE)["commands"],
            "states": [state["state"] for state in load(SAMPLES / "states.json")["states"]]}


def palette_matches(query: str, rows: list[dict], table: list[dict]) -> list[str]:
    """The palette's rows for a query, in registry order (a fresh session has no pins or uses): every term in the
    label or the command id (PaletteModel.Rows). The labels shown are the D02 rows' own; table rows keep theirs."""
    terms = query.lower().split()
    found = []
    for row in table + rows:
        if all(term in row["label"].lower() or term in row["id"].lower() for term in terms):
            found.append(row["label"])
    return found


def overlay(dump: dict) -> list[str]:
    return [text for group, text in dump["lines"] if group.startswith("overlay:")]


def expect_smoke(dumps: list[dict], source: dict, state: str | None) -> list[str]:
    """B4's smoke script: D02 is on; the palette lists the CR-16 draft rows, each marked; Escape closes it."""
    problems = []
    if [dump["do"] for dump in dumps] != ["start", "chord F1", "text tuning.", "chord Escape", "wait 6000 ms"]:
        return [f"the steps written were {[dump['do'] for dump in dumps]}"]
    chip = "D02 · " + (state or "no state")
    if (("frame", chip) not in {(group, text) for group, text in map(tuple, dumps[0]["lines"])}):
        problems.append(f"step 0: the frame does not show {chip!r}")
    if overlay(dumps[0]):
        problems.append("step 0: an overlay is open at the start")
    if not overlay(dumps[1]):
        problems.append("step 1: F1 did not open the palette")
    drafts = {row["label"] for row in source["rows"]}
    wanted = palette_matches("tuning.", source["rows"], source["table"])[:12]
    if len(wanted) != 8 or not set(wanted) <= drafts:
        problems.append(f"the data: tuning. matches {wanted}, not the eight CR-16 draft rows")
    shown = overlay(dumps[2])
    labels = [text for text in shown if text in drafts]
    if labels != wanted:
        problems.append(f"step 2: the palette lists {labels}, expected {wanted}")
    if shown.count("draft row") != len(wanted):
        problems.append(f"step 2: {shown.count('draft row')} draft-row marks for {len(wanted)} draft rows")
    if "tuning." not in shown:
        problems.append("step 2: the query field does not show tuning.")
    for step in (3, 4):
        if overlay(dumps[step]):
            problems.append(f"step {step}: the palette is still open")
    return problems


EXPECT = {"smoke": expect_smoke}


def window_problems(dumps: list[dict]) -> list[str]:
    """The owner's window rule as the shell itself reports it in every dump."""
    problems = []
    for dump in dumps:
        window = dump.get("window", {})
        if window.get("topmost") is not False:
            problems.append(f"step {dump['step']}: the shell's window is topmost")
        if window.get("foreground") is not False:
            problems.append(f"step {dump['step']}: the shell's window has the foreground")
    return problems


class Desktop:
    """The foreground window and a process's windows, through user32 (Windows only)."""

    def __init__(self):
        import ctypes
        from ctypes import wintypes
        self.ctypes, self.wintypes = ctypes, wintypes
        self.user32 = ctypes.WinDLL("user32")
        self.callback = ctypes.WINFUNCTYPE(wintypes.BOOL, wintypes.HWND, wintypes.LPARAM)
        self.user32.GetForegroundWindow.restype = wintypes.HWND
        self.user32.EnumWindows.argtypes = (self.callback, wintypes.LPARAM)
        self.user32.GetWindowThreadProcessId.argtypes = (wintypes.HWND, ctypes.POINTER(wintypes.DWORD))
        self.user32.GetWindowLongPtrW.argtypes = (wintypes.HWND, ctypes.c_int)
        self.user32.GetWindowLongPtrW.restype = ctypes.c_ssize_t
        self.user32.IsWindowVisible.argtypes = (wintypes.HWND,)

    def foreground(self) -> int:
        return int(self.user32.GetForegroundWindow() or 0)

    def windows_of(self, pid: int) -> list[int]:
        found = []

        def visit(window, _):
            owner = self.wintypes.DWORD()
            self.user32.GetWindowThreadProcessId(window, self.ctypes.byref(owner))
            if owner.value == pid and self.user32.IsWindowVisible(window):
                found.append(int(window))
            return True

        self.user32.EnumWindows(self.callback(visit), 0)
        return found

    def topmost(self, window: int) -> bool:
        return bool(self.user32.GetWindowLongPtrW(window, -20) & 0x8)  # GWL_EXSTYLE, WS_EX_TOPMOST


def run(script: str, out: Path, build: Path | None, state: str | None, desktop=None, launcher=launch.main,
        clock=time.monotonic, sleep=time.sleep) -> tuple[int, dict]:
    """Launches the run, watches the foreground, and compares the screens. Returns the exit code and the record."""
    path = SCRIPTS / f"{script}.json"
    source = data()
    if script not in EXPECT or not path.is_file():
        raise ValueError(f"no D02 script named {script} (renderer/shell/d02_scripts/)")
    if state is not None and state not in source["states"]:
        raise ValueError(f"no sample state named {state}")
    desktop = desktop or Desktop()
    started: dict = {}
    lines: list[str] = []

    def popen(arguments, cwd, env, **options):
        process = subprocess.Popen(arguments, cwd=cwd, env=env, **options)
        started["process"], started["at"] = process, clock()
        return process

    arguments = ["--out", str(out), "--frame", "--puzzle", "jumble-on", "--d02", "--d02-script", str(path)]
    arguments += ["--build", str(build)] if build else []
    arguments += ["--d02-state", state] if state else []
    result: dict = {}
    before = desktop.foreground()
    thread = threading.Thread(target=lambda: result.update(code=launcher(arguments, popen=popen, out_stream=lines.append)),
                              daemon=True)
    thread.start()
    while "at" not in started and thread.is_alive():
        sleep(0.1)
    after, topmost = None, None
    if "at" in started:
        while clock() - started["at"] < FOREGROUND_AFTER_S and thread.is_alive():
            sleep(0.1)
        after = desktop.foreground()
        topmost = any(desktop.topmost(window) for window in desktop.windows_of(started["process"].pid))
    thread.join(RUN_TIMEOUT_S)
    problems = []
    if thread.is_alive():
        subprocess.run(["taskkill", "/T", "/F", "/PID", str(started["process"].pid)], capture_output=True)
        problems.append(f"the shell ran longer than {RUN_TIMEOUT_S:.0f} s and was stopped")
        thread.join(30)
    if result.get("code") != 0:
        problems.append(f"the launch ended with {result.get('code')}: {' | '.join(lines[-3:])}")
    if after is None or after != before:
        problems.append("the foreground changed during the first five seconds" if after is not None
                        else "the shell did not start")
    if topmost:
        problems.append("the shell's window carries WS_EX_TOPMOST")
    dumps = []
    screen = out / SCREEN
    if screen.is_file():
        dumps = [json.loads(line) for line in screen.read_text(encoding="utf-8").splitlines() if line.strip()]
    else:
        problems.append("the shell wrote no d02-screen.jsonl")
    if dumps:
        problems += window_problems(dumps)
        problems += EXPECT[script](dumps, source, state)
    record = {"format": FORMAT, "script": script, "state": state, "steps": len(dumps), "mismatches": problems,
              "foreground_kept": after is not None and after == before, "topmost": bool(topmost),
              "gave_back": max((dump.get("window", {}).get("gave_back", 0) for dump in dumps), default=0)}
    if out.is_dir():
        (out / RECORD).write_text(json.dumps(record, indent=2) + "\n", encoding="utf-8", newline="\n")
    return (0 if not problems else 1), record


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--script", required=True, help="a script of renderer/shell/d02_scripts/, without .json")
    parser.add_argument("--out", type=Path, required=True, help="the run folder, which must not exist yet")
    parser.add_argument("--build", type=Path, help="the product build (default: the newest)")
    parser.add_argument("--state", help="the sample state the D02 surfaces read")
    args = parser.parse_args(argv)
    try:
        code, record = run(args.script, args.out.resolve(), args.build, args.state)
    except (ValueError, OSError) as error:
        print(f"d02_run: {error}")
        return 2
    if code == 0:
        print(f"M600_D02_RUN_PASS script={args.script} steps={record['steps']} mismatches=0")
    else:
        print(f"M600_D02_RUN_FAIL script={args.script} steps={record['steps']} mismatches={len(record['mismatches'])}")
        for problem in record["mismatches"]:
            print("  " + problem)
    return code


if __name__ == "__main__":
    sys.exit(main())
