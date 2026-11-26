"""Start C9's product shell from a product build (route B): the one way the owner-machine steps start it.

  python -B renderer/shell/launch.py --out NEW_FOLDER [--build DIR] [--run-id ID] [--echo] [--resolution WxH]
                                     [--puzzle jumble-off|jumble-on] [--lx4 --lx4-fixtures DIR]
                                     [--lx2-injection CHILD] [--editor] [--cold-start] [--dry-run]
                                     [--position X,Y] [--d02 [--d02-state ID] [--d02-fixture DIR] [--d02-script FILE]
                                     [--d02-data DIR]]

The launcher recomputes every part of the product build (`build_product.verify`; default: the newest build under
work/build/c9-product), creates the run folder `--out`, which must not exist yet, and starts the pinned Godot (the
process that presents, not the console wrapper) on the build's shell with the environment the shell reads:
`M600_CORE_DLL`, `M600_HOST_DLL`, `M600_ASSETS` (the build's copy of the model), `M600_EVENTS_DIR` (the run folder),
`M600_BUILD_IDENTITY` (the build's identity), `M600_RUN_ID`, and for charter E2's turns `M600_SESSION_DIR` (a new
session folder, `session` in the run folder) and `M600_TWIST_MENU` (the build's copy of the twist menu); `--echo`
adds `M600_INPUT_ECHO=1`, the input echo of route B's check B1. `M600_PUZZLE` names the puzzle the shell runs and its
run row records (`--puzzle`, default jumble-off). With Jumble on the shell's host opens the jumbled puzzle and the
core draws its posed engine images: each sticker keeps its home colour and moves with its piece's pose, and no slot
is pickable (the core's ID pass stamps no posed instance).

LX-4 (C9 U8): `--lx4 --lx4-fixtures DIR` adds `M600_LX4=1`, which makes the shell create LX-4's module,
`M600_LX4_DLL` (the build's `lx4/c600_shell_lx4.dll`) and `M600_LX4_FIXTURES` (DIR, a folder that `c600-lx4 --out`
wrote). The module refuses to start without both, so either flag without the other, or a folder without
`MANIFEST.json`, is refused here. The fixtures are not part of the build: `launch.json` records the SHA-256 of their
`MANIFEST.json` (never their path).

LX-2 (RTE.42 conditions 3 and 4, a draft): `--lx2-injection CHILD` names a release build of c600-lx2's injection
child (`c600-lx2-child`, never shipped), which then replaces the build's runtime as `M600_RUNTIME`, and adds
`M600_LX2=1`, which makes the shell create LX-2's module. The product build does not contain the child, so
`build_product.verify` cannot vouch for it: `launch.json` records its file name and SHA-256 (never its path). It
combines with `--lx4`, so that one run measures the echo bar during the fault windows, and is refused with
`--cold-start`, which measures the product's own runtime. The launcher drops `C600_LX2_CEILING` and `C600_LX2_FAULT`
from the environment it passes on: LX-2's module sets them in the shell's process before a respawn of the child.

LP4-3 (a draft): `--editor` adds `M600_EDITOR=1`, which makes the shell create the editor panel of the write-check-run
flow, and `M600_EDITOR_DLL` (the build's `editor/c600_shell_editor.dll`); the panel runs the solver's macro in the
language runtime that `M600_RUNTIME` names. `launch.json` records the flag.

Charter E3 (C9 U6, a draft): every launch copies the build's session fixture of its puzzle (`session-fixture/session`,
with Jumble on `session-fixture-jumble-on/session`) fresh to the run's `session` folder and sets `M600_SESSION_FIXTURE`
(that fixture's record), so that the host reopens that existing session, and `M600_RUNTIME` (the build's
c600-runtime), so that the host starts the language runtime. `--cold-start` waits until 20 s have passed since the
previous product exit and adds `M600_EXIT_AT_FIRST_FRAME=1`, so that the shell quits once the first correct frame is
shown. The launcher reads the performance counter right before it creates the
process, and after the shell exits it writes `startup.jsonl` (`magic600-c9-startup-v1`, which
tools/perf/c9/cold_start.py reads): its run row with the cold flags, its process_start row, then the shell's rows from
`startup.body.jsonl`, which it removes. Cold flags: a new process; no process of the pinned Godot or the build's
runtime running at launch (read with EnumProcesses, never tasklist; a process this user may not query is not seen); a
fresh session copy; and the seconds since the previous product exit, from `work/build/c9-product/last-exit.json`, which
every launch rewrites when the shell exits, or null when there is none or it is from another boot (DRAFT: "this sign-in"
read as "this boot").

ADR-D02's build (B4): `--d02` (with `--frame` only) adds `M600_D02=1`, which turns the frame's D02 surfaces on, and
starts the shell for watching: never on top and never taking the foreground (the owner, 26 November 2026). The process
gets STARTUPINFO with SW_SHOWNOACTIVATE, and `M600_D02_FOREGROUND` names the window that had the foreground just before
the launch, so that the shell hands the foreground straight back to it if Godot activates its own window at creation
(Godot 4.7's show_window does so whenever Windows lets it, for example after 200 s without owner input). Without
`--d02` nothing of this applies, so the measured C9 runs are unchanged. `--d02-state ID` names the sample state the
surfaces read (`M600_D02_STATE`). `--d02-fixture DIR` runs on a D02 session fixture (B5: a folder with `fixture.json`
and `session/`) instead of the build's own; `--d02-script FILE` makes the shell play a D02 script and quit
(`M600_D02_SCRIPT`). Neither is part of the build: `launch.json` records only the SHA-256 of the fixture's
`fixture.json` and the script's name and SHA-256, never a path. `--d02-data DIR` (the watch command's) names the
checkout's tools/looklab/data, which the shell reads again when the watch command asks for a data-only reload
(`M600_D02_DATA`; recorded as a flag only). `--position X,Y` places the window (a watched restart opens where the old
window was).

It waits for the shell to exit and writes `launch.json` (`magic600-c9-launch-v1`) beside the shell's logs, with the
build's own folder and the run folder written as `<build>` and `<out>`.

`--dry-run` checks the build and the run folder and prints the command and the environment; it creates nothing and
starts nothing. Start the launcher from a PowerShell on the desktop: a window started from a background process never
gets the foreground. Exit 0 when the shell exited 0, 1 when it exited otherwise, 2 for a usage or build error.
"""
from __future__ import annotations

import sys

sys.dont_write_bytecode = True

import argparse
import datetime as dt
import hashlib
import json
import os
import shutil
import subprocess
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import build_product  # noqa: E402

FORMAT = "magic600-c9-launch-v1"
RECORD = "launch.json"
SHELL_VARIABLES = ("M600_CORE_DLL", "M600_HOST_DLL", "M600_ASSETS", "M600_EVENTS_DIR", "M600_BUILD_IDENTITY",
                   "M600_RUN_ID", "M600_INPUT_ECHO", "M600_SESSION_DIR", "M600_TWIST_MENU", "M600_PUZZLE", "M600_LX4",
                   "M600_LX4_DLL", "M600_LX4_FIXTURES", "M600_SESSION_FIXTURE", "M600_RUNTIME",
                   "M600_EXIT_AT_FIRST_FRAME", "M600_LX2", "M600_CHROME", "M600_EDITOR", "M600_EDITOR_DLL",
                   "M600_D02", "M600_D02_STATE", "M600_D02_FIXTURE", "M600_D02_SCRIPT", "M600_D02_FOREGROUND",
                   "M600_D02_DATA")
# The file of an LX-4 fixture folder that the launcher checks and records (renderer/bridge/lx4's fixtures.rs).
LX4_MANIFEST = "MANIFEST.json"
# LX-2's fault variables, which the injection child reads at its start: a run starts with them unset, and only LX-2's
# module sets them, in the shell's process.
LX2_FAULT_VARIABLES = ("C600_LX2_CEILING", "C600_LX2_FAULT")
SESSION = "session"
# Charter E3's startup log (tools/perf/c9/cold_start.py) and the launcher's record of the previous exit.
STARTUP_FORMAT = "magic600-c9-startup-v1"
STARTUP = "startup.jsonl"
STARTUP_BODY = "startup.body.jsonl"
LAST_EXIT = build_product.BUILD_ROOT / "last-exit.json"
LAST_EXIT_FORMAT = "magic600-c9-last-exit-v1"
PAUSE_S = 20
# The wall clock and the boot's tick count may drift apart by this much before the previous exit counts as another
# boot's.
CLOCK_SLACK_S = 60
# The puzzles the shell's run row may name (EventLog.Puzzles); its host runs both.
PUZZLES = ("jumble-off", "jumble-on")
# A D02 session fixture's record (B5), the only file of the folder the launcher reads.
D02_FIXTURE = "fixture.json"
# A watched start (the owner's window rule): STARTF_USESHOWWINDOW with SW_SHOWNOACTIVATE.
STARTF_USESHOWWINDOW = 0x00000001
SW_SHOWNOACTIVATE = 4


class LaunchError(Exception):
    """A launch that must not start."""


def run_id_now() -> str:
    return dt.datetime.now(dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")


def command(product: dict, build: Path, out: Path, resolution: str | None = None,
            position: str | None = None) -> list[str]:
    arguments = [product["godot"]["executable"], "--path", str(build / build_product.PROJECT),
                 "--rendering-driver", "d3d12", "--render-thread", "safe", "--windowed",
                 "--log-file", str(out / "godot.log")]
    if resolution:
        arguments += ["--resolution", resolution]
    if position:
        arguments += ["--position", position]
    return arguments


def no_activation():
    """The STARTUPINFO of a watched start: the window shows without activation (the owner's window rule)."""
    info = subprocess.STARTUPINFO()
    info.dwFlags |= STARTF_USESHOWWINDOW
    info.wShowWindow = SW_SHOWNOACTIVATE
    return info


def d02_folder(d02: bool, fixture: Path | None) -> Path | None:
    """A D02 session fixture folder, resolved; LaunchError without --d02 or without its record and session."""
    if fixture is None:
        return None
    if not d02:
        raise LaunchError("--d02-fixture needs --d02")
    folder = fixture.resolve()
    if not (folder / D02_FIXTURE).is_file() or not (folder / SESSION).is_dir():
        raise LaunchError(f"--d02-fixture: the folder needs {D02_FIXTURE} and {SESSION}/ (B5's fixture writer)")
    return folder


def environment(product: dict, build: Path, out: Path, run_id: str, echo: bool, base: dict | None = None,
                puzzle: str = PUZZLES[0], lx4: Path | None = None, cold_start: bool = False,
                lx2_injection: Path | None = None, frame: bool = False, editor: bool = False,
                d02: dict | None = None) -> dict:
    """The process environment: `base` (default: this process's) without any shell variable or LX-2 fault variable,
    then the launch's. `lx4` is LX-4's fixture folder, which turns its module on with the build's library.
    `lx2_injection` replaces the build's runtime with LX-2's injection child. `editor` turns the editor panel on
    with the build's editor library. `d02` holds the D02 variables of a watched start (B4)."""
    env = {key: value for key, value in (os.environ if base is None else base).items()
           if key.upper() not in SHELL_VARIABLES + LX2_FAULT_VARIABLES}
    env.update({"M600_CORE_DLL": str(build / build_product.CORE_DLL), "M600_HOST_DLL": str(build / build_product.HOST_DLL),
                "M600_ASSETS": str(build / build_product.ASSETS), "M600_EVENTS_DIR": str(out),
                "M600_BUILD_IDENTITY": product["identity"], "M600_RUN_ID": run_id,
                "M600_SESSION_DIR": str(out / SESSION), "M600_TWIST_MENU": str(build / build_product.MENU),
                "M600_SESSION_FIXTURE": str(build / build_product.fixture_record(puzzle)),
                "M600_RUNTIME": str(build / build_product.RUNTIME), "M600_PUZZLE": puzzle})
    if echo:
        env["M600_INPUT_ECHO"] = "1"
    if lx4 is not None:
        env.update({"M600_LX4": "1", "M600_LX4_DLL": str(build / build_product.LX4_DLL),
                    "M600_LX4_FIXTURES": str(lx4)})
    if cold_start:
        env["M600_EXIT_AT_FIRST_FRAME"] = "1"
    if frame:
        env["M600_CHROME"] = "1"
    if editor:
        env.update({"M600_EDITOR": "1", "M600_EDITOR_DLL": str(build / build_product.EDITOR_DLL)})
    if lx2_injection is not None:
        env["M600_RUNTIME"] = str(lx2_injection)
        env["M600_LX2"] = "1"
    if d02 is not None:
        env.update({"M600_D02": "1", **d02})
        if "M600_D02_FIXTURE" in d02:
            env["M600_SESSION_FIXTURE"] = d02["M600_D02_FIXTURE"]
    return env


def injection_record(child: Path) -> dict:
    """The injection child's file name and SHA-256: `launch.json` records what verify cannot vouch for."""
    digest = hashlib.sha256()
    with child.open("rb") as stream:
        for block in iter(lambda: stream.read(1 << 20), b""):
            digest.update(block)
    return {"name": child.name, "sha256": digest.hexdigest()}


def lx4_folder(lx4: bool, fixtures: Path | None) -> Path | None:
    """LX-4's resolved fixture folder, None without `--lx4`; LaunchError for one flag without the other or a folder
    without its manifest."""
    if lx4 != (fixtures is not None):
        raise LaunchError("--lx4 and --lx4-fixtures go together: LX-4's module refuses to start without its fixtures")
    if fixtures is None:
        return None
    folder = fixtures.resolve()
    if not (folder / LX4_MANIFEST).is_file():
        raise LaunchError(f"--lx4-fixtures: no {LX4_MANIFEST} in the folder (write it with c600-lx4 --out)")
    return folder


class Machine:
    """The launcher's clocks and process list on Windows, through ctypes; tests pass a machine of their own."""

    def __init__(self):
        import ctypes
        from ctypes import wintypes
        self._ctypes, self._wintypes = ctypes, wintypes
        kernel = self._kernel = ctypes.WinDLL("kernel32", use_last_error=True)
        kernel.GetTickCount64.restype = ctypes.c_uint64
        kernel.OpenProcess.restype = wintypes.HANDLE
        kernel.OpenProcess.argtypes = [wintypes.DWORD, wintypes.BOOL, wintypes.DWORD]
        kernel.QueryFullProcessImageNameW.argtypes = [wintypes.HANDLE, wintypes.DWORD, wintypes.LPWSTR,
                                                      ctypes.POINTER(wintypes.DWORD)]
        kernel.CloseHandle.argtypes = [wintypes.HANDLE]
        kernel.K32EnumProcesses.argtypes = [ctypes.POINTER(wintypes.DWORD), wintypes.DWORD,
                                            ctypes.POINTER(wintypes.DWORD)]

    def _counter(self, call) -> int:
        value = self._ctypes.c_int64()
        if not call(self._ctypes.byref(value)):
            raise OSError("the performance counter did not answer")
        return value.value

    def qpc(self) -> int:
        return self._counter(self._kernel.QueryPerformanceCounter)

    def qpc_frequency(self) -> int:
        return self._counter(self._kernel.QueryPerformanceFrequency)

    def ticks_ms(self) -> int:
        """Milliseconds since this boot (GetTickCount64)."""
        return int(self._kernel.GetTickCount64())

    def wall(self) -> float:
        return time.time()

    def sleep(self, seconds: float) -> None:
        time.sleep(seconds)

    def foreground(self) -> int:
        """The window that has the foreground now, as a number; 0 when there is none."""
        user32 = self._ctypes.WinDLL("user32")
        user32.GetForegroundWindow.restype = self._wintypes.HWND
        return int(user32.GetForegroundWindow() or 0)

    def process_images(self) -> list[str]:
        """The image path of every running process this user may query."""
        ctypes, wintypes = self._ctypes, self._wintypes
        count = 4096
        while True:
            pids = (wintypes.DWORD * count)()
            needed = wintypes.DWORD()
            if not self._kernel.K32EnumProcesses(pids, ctypes.sizeof(pids), ctypes.byref(needed)):
                raise OSError("EnumProcesses did not answer")
            if needed.value < ctypes.sizeof(pids):
                break
            count *= 2
        images = []
        for pid in pids[:needed.value // ctypes.sizeof(wintypes.DWORD)]:
            handle = self._kernel.OpenProcess(0x1000, False, pid)  # PROCESS_QUERY_LIMITED_INFORMATION
            if not handle:
                continue
            try:
                size = wintypes.DWORD(32768)
                buffer = ctypes.create_unicode_buffer(size.value)
                if self._kernel.QueryFullProcessImageNameW(handle, 0, buffer, ctypes.byref(size)):
                    images.append(buffer.value)
            finally:
                self._kernel.CloseHandle(handle)
        return images


def seconds_since_previous_exit(machine, state: Path) -> float | None:
    """Seconds since the previous product exit by this boot's tick count, or None when there is no record or it is
    from another boot (the wall clock and the tick count disagree by more than CLOCK_SLACK_S)."""
    try:
        last = json.loads(state.read_text(encoding="utf-8"))
        if last.get("format") != LAST_EXIT_FORMAT:
            return None
        wall, ticks = float(last["wall"]), int(last["ticks_ms"])
    except (OSError, ValueError, KeyError, TypeError, AttributeError):
        return None
    by_ticks = (machine.ticks_ms() - ticks) / 1000
    if by_ticks < 0 or abs((machine.wall() - wall) - by_ticks) > CLOCK_SLACK_S:
        return None
    return round(by_ticks, 3)


def record_exit(machine, state: Path) -> None:
    """Rewrite the record of the previous exit with this moment."""
    state.parent.mkdir(parents=True, exist_ok=True)
    partial = state.with_name(state.name + ".partial")
    partial.write_text(json.dumps({"format": LAST_EXIT_FORMAT, "wall": machine.wall(), "ticks_ms": machine.ticks_ms()})
                       + "\n", encoding="utf-8", newline="\n")
    os.replace(partial, state)


def product_running(machine, images: set[str]) -> bool:
    """Whether a process of one of `images` runs; True when the process list cannot be read."""
    wanted = {os.path.normcase(os.path.normpath(image)) for image in images}
    try:
        running = machine.process_images()
    except OSError:
        return True
    return any(os.path.normcase(os.path.normpath(image)) in wanted for image in running)


def seal(out: Path, run: dict, process_start: int) -> None:
    """startup.jsonl from the launcher's run and process_start rows and the shell's body rows; the body is removed."""
    body = out / STARTUP_BODY
    rows = "".join(json.dumps(row, separators=(",", ":")) + "\n"
                   for row in (run, {"t": "process_start", "qpc": process_start}))
    try:
        shell_rows = body.read_text(encoding="utf-8")
    except FileNotFoundError:
        shell_rows = ""
    if shell_rows and not shell_rows.endswith("\n"):
        shell_rows += "\n"
    with (out / STARTUP).open("x", encoding="utf-8", newline="\n") as stream:
        stream.write(rows + shell_rows)
    body.unlink(missing_ok=True)


def position_ok(position: str) -> bool:
    parts = position.split(",")
    return len(parts) == 2 and all(part.lstrip("-").isdigit() for part in parts)


def check_out(out: Path) -> Path:
    out = out.resolve()
    if out.exists():
        raise LaunchError(f"the run folder already exists; a run never reuses one: {out}")
    return out


def sanitized(arguments: list[str], build: Path, out: Path) -> list[str]:
    """The command with the build folder and the run folder written as <build> and <out>."""
    def one(argument: str) -> str:
        for folder, name in ((out, "<out>"), (build, "<build>")):
            text = str(folder)
            if argument == text or argument.startswith(text + os.sep):
                return name + argument[len(text):].replace(os.sep, "/")
        return argument
    return [Path(arguments[0]).name] + [one(argument) for argument in arguments[1:]]


def main(argv: list[str] | None = None, popen=subprocess.Popen, out_stream=print, machine=None,
         state: Path = LAST_EXIT) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--out", type=Path, required=True, help="the run folder, which must not exist yet")
    parser.add_argument("--build", type=Path, help="the product build (default: the newest under work/build/c9-product)")
    parser.add_argument("--run-id", help="the run's id (default: the UTC start time)")
    parser.add_argument("--echo", action="store_true", help="show and log the input echo (route B check B1)")
    parser.add_argument("--resolution", help="the window's size as WxH (default: the project's)")
    parser.add_argument("--puzzle", choices=PUZZLES, default=PUZZLES[0], help="the puzzle the shell runs")
    parser.add_argument("--lx4", action="store_true", help="create LX-4's module in the shell (C9 U8)")
    parser.add_argument("--lx4-fixtures", type=Path, metavar="DIR",
                        help="LX-4's fixture folder (c600-lx4 --out); needed with --lx4")
    parser.add_argument("--lx2-injection", type=Path, metavar="CHILD",
                        help="LX-2: run on this c600-lx2-child build instead of the build's runtime, with LX-2's module")
    parser.add_argument("--cold-start", action="store_true",
                        help="charter E3: wait 20 s since the previous exit; the shell quits at its first correct frame")
    parser.add_argument("--frame", action="store_true",
                        help="open the frame (A5): the chrome library over the window, the core under the Global view")
    parser.add_argument("--editor", action="store_true",
                        help="LP4-3: create the editor panel of the write-check-run flow in the shell")
    parser.add_argument("--position", help="the window's top left corner as X,Y (a watched restart keeps its place)")
    parser.add_argument("--d02", action="store_true",
                        help="ADR-D02's build: the frame's D02 surfaces, a window that never takes the foreground")
    parser.add_argument("--d02-state", metavar="ID", help="the sample state the D02 surfaces read; needs --d02")
    parser.add_argument("--d02-fixture", type=Path, metavar="DIR",
                        help="run on this D02 session fixture (B5) instead of the build's; needs --d02")
    parser.add_argument("--d02-script", type=Path, metavar="FILE",
                        help="play this D02 script in the shell, which then quits; needs --d02")
    parser.add_argument("--d02-data", type=Path, metavar="DIR",
                        help="the data folder a data-only reload reads (the watch command's); needs --d02")
    parser.add_argument("--dry-run", action="store_true", help="check and print; create and start nothing")
    args = parser.parse_args(argv)
    try:
        build = args.build.resolve() if args.build else build_product.newest()
        if build is None:
            raise LaunchError(f"no product build under {build_product.BUILD_ROOT}; run build_product.py")
        product = build_product.verify(build)
        if args.resolution and not all(part.isdigit() for part in args.resolution.split("x", 1)):
            raise LaunchError("--resolution must be WxH")
        injection, child = None, None
        if args.lx2_injection is not None:
            if args.cold_start:
                raise LaunchError("--cold-start measures the build's own runtime; --lx2-injection replaces it")
            child = args.lx2_injection.resolve()
            if not child.is_file():
                raise LaunchError(f"--lx2-injection: no file {child.name}")
            injection = injection_record(child)
        if args.position and not position_ok(args.position):
            raise LaunchError("--position must be X,Y")
        fixtures = lx4_folder(args.lx4, args.lx4_fixtures)
        manifest = None if fixtures is None else injection_record(fixtures / LX4_MANIFEST)["sha256"]
        if args.d02 and not args.frame:
            raise LaunchError("--d02 needs --frame: the D02 surfaces live in the frame")
        if not args.d02 and (args.d02_state or args.d02_script is not None or args.d02_data is not None):
            raise LaunchError("--d02-state, --d02-script and --d02-data need --d02")
        if args.d02_data is not None and not (args.d02_data / "samples").is_dir():
            raise LaunchError("--d02-data: the folder needs samples/ (tools/looklab/data)")
        if args.d02 and args.cold_start:
            raise LaunchError("--cold-start measures the product; a watched D02 start is not measured")
        d02_fixture = d02_folder(args.d02, args.d02_fixture)
        script = None
        if args.d02_script is not None:
            script = args.d02_script.resolve()
            if not script.is_file():
                raise LaunchError(f"--d02-script: no file {script.name}")
        out = check_out(args.out)
        run_id = args.run_id or run_id_now()
        arguments = command(product, build, out, args.resolution, args.position)
        d02 = None
        if args.d02:
            fixture_record = None if d02_fixture is None else str(d02_fixture / D02_FIXTURE)
            d02 = {key: value for key, value in (("M600_D02_STATE", args.d02_state),
                                                  ("M600_D02_FIXTURE", fixture_record),
                                                  ("M600_D02_SCRIPT", None if script is None else str(script)),
                                                  ("M600_D02_DATA", None if args.d02_data is None
                                                   else str(args.d02_data.resolve())))
                   if value}
        env = environment(product, build, out, run_id, args.echo, puzzle=args.puzzle, lx4=fixtures,
                          cold_start=args.cold_start, lx2_injection=child, frame=args.frame, editor=args.editor,
                          d02=d02)
    except (LaunchError, build_product.ProductError, OSError) as error:
        out_stream(f"launch: {error}")
        return 2
    if args.dry_run:
        out_stream(f"launch: ready: build {product['identity']} ({build.name}), run {run_id}")
        if injection is not None:
            out_stream(f"launch: LX-2 injection child {injection['name']} sha256 {injection['sha256']} "
                       "(not part of the verified build)")
        out_stream("command: " + subprocess.list2cmdline(arguments))
        for key in SHELL_VARIABLES:
            if key in env:
                out_stream(f"env: {key}={env[key]}")
        return 0
    machine = machine or Machine()
    out.mkdir(parents=True)
    try:
        shutil.copytree(build / build_product.fixture_session(args.puzzle) if d02_fixture is None
                        else d02_fixture / SESSION, out / SESSION)
    except OSError as error:
        out_stream(f"launch: the session fixture did not copy: {error}")
        return 2
    gap = seconds_since_previous_exit(machine, state)
    if args.cold_start and gap is not None and gap < PAUSE_S:
        out_stream(f"launch: waiting {PAUSE_S - gap:.1f} s since the previous exit (cold start)")
        machine.sleep(PAUSE_S - gap)
        gap = seconds_since_previous_exit(machine, state)
    running = product_running(machine, {product["godot"]["executable"], str(build / build_product.RUNTIME)}
                              | ({str(child)} if child is not None else set()))
    frequency = machine.qpc_frequency()
    started, clock = dt.datetime.now(dt.timezone.utc), time.monotonic()
    options = {}
    if d02 is not None:
        # The window that has the foreground now gets it back if Godot activates the shell's window (B4).
        env["M600_D02_FOREGROUND"] = str(machine.foreground())
        options["startupinfo"] = no_activation()
    process_start = machine.qpc()
    process = popen(arguments, cwd=str(build / build_product.PROJECT), env=env, **options)
    code = process.wait()
    record_exit(machine, state)
    seconds = round(time.monotonic() - clock, 3)
    cold = {"new_process": True, "no_product_process_at_launch": not running, "fresh_session_copy": True,
            "seconds_since_previous_exit": gap}
    seal(out, {"t": "run", "format": STARTUP_FORMAT, "run_id": run_id, "build_identity": product["identity"],
               "qpc_frequency": frequency, "synthetic": False, "cold": cold}, process_start)
    with (out / RECORD).open("x", encoding="utf-8", newline="\n") as stream:
        stream.write(json.dumps({"format": FORMAT, "run_id": run_id, "build_identity": product["identity"],
                                 "echo": args.echo, "puzzle": args.puzzle, "lx4": args.lx4,
                                 "lx4_fixtures": None if manifest is None else {"manifest_sha256": manifest},
                                 "lx2_injection": injection, "editor": args.editor,
                                 "d02": None if d02 is None else {
                                     "state": args.d02_state,
                                     "fixture": None if d02_fixture is None else {
                                         "sha256": injection_record(d02_fixture / D02_FIXTURE)["sha256"]},
                                     "script": None if script is None else injection_record(script),
                                     "data_reload": args.d02_data is not None},
                                 "cold_start": args.cold_start, "command": sanitized(arguments, build, out),
                                 "started_utc": started.strftime("%Y-%m-%dT%H:%M:%SZ"), "seconds": seconds,
                                 "exit_code": code}, indent=2) + "\n")
    out_stream(f"launch: the shell exited {code} after {seconds} s; logs in {out}")
    return 0 if code == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
