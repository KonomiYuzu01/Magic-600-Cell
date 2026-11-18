"""Runner for the C9 confirmed-presentation probe (route B). Windows only; standard library only.

    python work/experiments/c9-present-probe/run_probe.py --check-only      print the quiet-machine gate JSON
    python work/experiments/c9-present-probe/run_probe.py --build-only      native build plus offline dotnet build
    python work/experiments/c9-present-probe/run_probe.py [--topmost] [--scale S] [--wait-quiet SECONDS]
                                                          [--min-idle SECONDS] [--skip-build] [--godot EXE]

A run checks the quiet-machine gate (exit 5 when it is not quiet), builds, records the build identity,
runs c9pp_control.exe and then the Godot harness with timeouts, and analyzes the logs. It stops after the
first harness that exits 3 (suspected TDR) or times out. Output: <repo>/work/loop-memory/c9-present-probe/
<UTC timestamp>/ (private; never publish). Probe evidence for a PHS.52 design proposal only.

Exit codes: 0 both harnesses completed, 1 build failure, 2 harness error, 3 suspected TDR or timeout,
5 the machine is not quiet.
"""
import os
import sys

if os.name != "nt":
    raise SystemExit("Windows only")
import argparse  # noqa: E402
import ctypes  # noqa: E402
import hashlib  # noqa: E402
import json  # noqa: E402
import shutil  # noqa: E402
import subprocess  # noqa: E402
import time  # noqa: E402
from ctypes import wintypes  # noqa: E402
from datetime import datetime, timezone  # noqa: E402
from pathlib import Path  # noqa: E402

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]
DEFAULT_BUILD = REPO / "work/build/c9-present-probe"
PRIVATE_ROOT = REPO / "work/loop-memory/c9-present-probe"
GODOT_VERSION = "4.7.2.stable.mono.official.ed1daf0bf"
GODOT_GLOB = ("GodotEngine.GodotEngine.Mono_*/Godot_v4.7.2-stable_mono_win64/"
              "Godot_v4.7.2-stable_mono_win64_console.exe")
SOURCE_EXCLUDED = {"__pycache__", ".godot", "bin", "obj"}
# Processes that make a GPU timing run meaningless or that another session may be driving.
BUSY_EXACT = {"cargo.exe", "rustc.exe", "cl.exe", "link.exe", "msbuild.exe", "ninja.exe", "cmake.exe", "blender.exe",
              "ffmpeg.exe", "presentmon.exe", "obs64.exe", "c9pp_control.exe", "sa2_selftest.exe"}
BUSY_PREFIX = ("godot", "presentmon-")
# Resident services that count only while they use CPU (more than 0.1 s in the window).
CPU_GATED = {"presentmonservice.exe"}
CPU_BUSY_LIMIT = 15.0  # percent of all logical processors over the sampling window
MIN_IDLE_DEFAULT = 300

kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
user32 = ctypes.WinDLL("user32", use_last_error=True)
kernel32.CreateToolhelp32Snapshot.restype = wintypes.HANDLE
kernel32.OpenProcess.restype = wintypes.HANDLE
user32.OpenInputDesktop.restype = wintypes.HANDLE


class PROCESSENTRY32W(ctypes.Structure):
    _fields_ = [("size", wintypes.DWORD), ("usage", wintypes.DWORD), ("pid", wintypes.DWORD),
                ("heap", ctypes.c_size_t), ("module", wintypes.DWORD), ("threads", wintypes.DWORD),
                ("parent", wintypes.DWORD), ("priority", ctypes.c_long), ("flags", wintypes.DWORD),
                ("exe", ctypes.c_wchar * 260)]


class FILETIME(ctypes.Structure):
    _fields_ = [("low", wintypes.DWORD), ("high", wintypes.DWORD)]


class LASTINPUTINFO(ctypes.Structure):
    _fields_ = [("size", wintypes.UINT), ("time", wintypes.DWORD)]


class POWER_STATUS(ctypes.Structure):
    _fields_ = [("ac", ctypes.c_ubyte), ("flag", ctypes.c_ubyte), ("percent", ctypes.c_ubyte),
                ("saver", ctypes.c_ubyte), ("life", wintypes.DWORD), ("full", wintypes.DWORD)]


# ---------------------------------------------------------------------------------------------
# Quiet-machine gate.
def processes():
    snapshot = kernel32.CreateToolhelp32Snapshot(0x2, 0)  # TH32CS_SNAPPROCESS
    if not snapshot or snapshot == wintypes.HANDLE(-1).value:
        raise OSError("process snapshot failed")
    found = {}
    try:
        entry = PROCESSENTRY32W()
        entry.size = ctypes.sizeof(entry)
        ok = kernel32.Process32FirstW(snapshot, ctypes.byref(entry))
        while ok:
            found[entry.pid] = (entry.exe.lower(), entry.parent)
            ok = kernel32.Process32NextW(snapshot, ctypes.byref(entry))
    finally:
        kernel32.CloseHandle(snapshot)
    return found


def process_cpu_seconds(pid):
    handle = kernel32.OpenProcess(0x1000, False, pid)  # PROCESS_QUERY_LIMITED_INFORMATION
    if not handle:
        return None
    try:
        times = [FILETIME() for _ in range(4)]
        if not kernel32.GetProcessTimes(handle, *(ctypes.byref(t) for t in times)):
            return None
        return sum((t.high << 32) | t.low for t in times[2:]) / 1e7
    finally:
        kernel32.CloseHandle(handle)


def system_times():
    idle, kernel, user = FILETIME(), FILETIME(), FILETIME()
    if not kernel32.GetSystemTimes(ctypes.byref(idle), ctypes.byref(kernel), ctypes.byref(user)):
        raise OSError("GetSystemTimes failed")
    return [(t.high << 32) | t.low for t in (idle, kernel, user)]


def idle_seconds():
    info = LASTINPUTINFO()
    info.size = ctypes.sizeof(info)
    if not user32.GetLastInputInfo(ctypes.byref(info)):
        raise OSError("GetLastInputInfo failed")
    return ((kernel32.GetTickCount() - info.time) & 0xFFFFFFFF) / 1000.0


def power_source():
    status = POWER_STATUS()
    if not kernel32.GetSystemPowerStatus(ctypes.byref(status)):
        return "unknown"
    return {0: "battery", 1: "mains"}.get(status.ac, "unknown")


def input_desktop():
    """Name of the input desktop when this session can open it ("Default" when unlocked), else None."""
    desktop = user32.OpenInputDesktop(0, False, 0x0001)  # DESKTOP_READOBJECTS
    if not desktop:
        return None
    try:
        name = ctypes.create_unicode_buffer(256)
        needed = wintypes.DWORD()
        if not user32.GetUserObjectInformationW(desktop, 2, name, ctypes.sizeof(name), ctypes.byref(needed)):  # UOI_NAME
            return None
        return name.value
    finally:
        user32.CloseDesktop(desktop)


def gate(min_idle=MIN_IDLE_DEFAULT, window=3.0):
    """One gate check. Busy Codex rule: a codex.exe started by the ChatGPT app counts only when it used
    more than 0.1 s of CPU in the window; any other codex.exe always counts; an unreadable helper counts."""
    found = processes()
    names = sorted({exe for exe, _ in found.values()
                    if exe in BUSY_EXACT or exe.startswith(BUSY_PREFIX)})
    codex = [(pid, found.get(parent, ("",))[0]) for pid, (exe, parent) in found.items() if exe == "codex.exe"]
    helpers = [pid for pid, parent in codex if parent == "chatgpt.exe"]
    gated = [(pid, exe) for pid, (exe, _) in found.items() if exe in CPU_GATED]
    before = {pid: process_cpu_seconds(pid) for pid in helpers + [pid for pid, _ in gated]}
    t0 = system_times()
    time.sleep(window)
    t1 = system_times()
    total = (t1[1] - t0[1]) + (t1[2] - t0[2])  # kernel time includes idle time
    cpu = round(100.0 * (total - (t1[0] - t0[0])) / total, 1) if total else 0.0
    codex_busy = len(codex) - len(helpers)
    for pid in helpers:
        after = process_cpu_seconds(pid)
        if before[pid] is None or (after is not None and after - before[pid] > 0.1):
            codex_busy += 1
    gated_cpu = {}
    for pid, exe in gated:
        after = process_cpu_seconds(pid)
        used = None if before[pid] is None or after is None else round(after - before[pid], 3)
        gated_cpu[exe] = used
        if used is not None and used > 0.1:
            names = sorted(set(names) | {exe})
    idle = round(idle_seconds(), 1)
    power = power_source()
    desktop = input_desktop()
    reasons = []
    if names:
        reasons.append("busy processes: " + ", ".join(names))
    if codex_busy:
        reasons.append(f"{codex_busy} busy Codex process(es)")
    if cpu > CPU_BUSY_LIMIT:
        reasons.append(f"CPU busy {cpu}% over {window:.0f} s (limit {CPU_BUSY_LIMIT:.0f}%)")
    if idle < min_idle:
        reasons.append(f"keyboard or mouse input {idle:.0f} s ago (need {min_idle} s)")
    if power != "mains":
        reasons.append(f"power source {power}")
    if desktop != "Default":
        reasons.append(f"input desktop {desktop!r} (locked or not accessible)")
    return {"quiet": not reasons, "reasons": reasons, "checked_utc": utc_now(), "busy_processes": names,
            "codex_busy": codex_busy, "cpu_gated_seconds": gated_cpu, "cpu_busy_percent": cpu, "cpu_window_s": window, "idle_seconds": idle,
            "min_idle_seconds": min_idle, "power_source": power, "input_desktop": desktop}


def wait_quiet(min_idle, limit):
    start = time.monotonic()
    while True:
        result = gate(min_idle)
        if result["quiet"] or time.monotonic() - start >= limit:
            return result
        time.sleep(15)


# ---------------------------------------------------------------------------------------------
# Build and identity.
def utc_now():
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def sha256(path):
    digest = hashlib.sha256()
    with open(path, "rb") as handle:
        for block in iter(lambda: handle.read(1 << 20), b""):
            digest.update(block)
    return digest.hexdigest()


def source_identity():
    digest, files = hashlib.sha256(), []
    for path in sorted(HERE.rglob("*")):
        relative = path.relative_to(HERE)
        if not path.is_file() or any(part in SOURCE_EXCLUDED for part in relative.parts):
            continue
        files.append(relative.as_posix())
        digest.update(relative.as_posix().encode() + b"\0" + sha256(path).encode() + b"\n")
    return {"sha256": digest.hexdigest(), "file_count": len(files)}


def git_head():
    env = dict(os.environ, GIT_OPTIONAL_LOCKS="0")
    try:
        return subprocess.run(["git", "--no-optional-locks", "rev-parse", "HEAD"], cwd=REPO, env=env, capture_output=True,
                              text=True, timeout=30, check=True).stdout.strip()
    except (OSError, subprocess.SubprocessError):
        return None


def locate_godot(explicit=None):
    if explicit:
        path = Path(explicit).resolve()
    else:
        base = os.environ.get("LOCALAPPDATA")
        if not base:
            raise RuntimeError("LOCALAPPDATA is unavailable; pass --godot")
        matches = list((Path(base) / "Microsoft/WinGet/Packages").glob(GODOT_GLOB))
        if len(matches) != 1:
            raise RuntimeError("expected exactly one Godot 4.7.2 .NET installation; pass --godot")
        path = matches[0].resolve()
    nupkgs = path.parent / "GodotSharp/Tools/nupkgs"
    if not path.is_file() or not nupkgs.is_dir():
        raise RuntimeError("Godot executable or its offline NuGet folder is missing")
    return path, nupkgs


def run_process(command, cwd, env, timeout, prefix):
    prefix = Path(prefix)
    prefix.parent.mkdir(parents=True, exist_ok=True)
    timed_out, start = False, time.monotonic()
    with open(str(prefix) + ".stdout.txt", "wb") as out, open(str(prefix) + ".stderr.txt", "wb") as err:
        process = subprocess.Popen(command, cwd=cwd, env=env, stdout=out, stderr=err)
        try:
            process.wait(timeout=timeout)
        except subprocess.TimeoutExpired:
            timed_out = True
            subprocess.run(["taskkill", "/PID", str(process.pid), "/T", "/F"], capture_output=True, timeout=30)
            try:
                process.wait(timeout=30)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(timeout=30)
    return {"exit_code": process.returncode, "timed_out": timed_out, "seconds": round(time.monotonic() - start, 1)}


def offline_environment(build, nupkgs):
    env = os.environ.copy()
    private = build / "nuget"
    env.update({"M600_GODOT_NUPKGS": str(nupkgs), "NUGET_PACKAGES": str(private / "packages"),
                "DOTNET_CLI_HOME": str(private / "dotnet-home"), "NUGET_HTTP_CACHE_PATH": str(private / "http-cache"),
                "DOTNET_CLI_TELEMETRY_OPTOUT": "1", "DOTNET_NOLOGO": "1", "DOTNET_SKIP_FIRST_TIME_EXPERIENCE": "1",
                "DOTNET_CLI_WORKLOAD_UPDATE_NOTIFY_DISABLE": "1", "DOTNET_ADD_GLOBAL_TOOLS_TO_PATH": "0",
                "DOTNET_GENERATE_ASPNET_CERTIFICATE": "0"})
    return env


def build(build_dir, godot):
    build_dir = Path(build_dir).resolve()
    if not build_dir.is_relative_to((REPO / "work").resolve()):
        raise RuntimeError("the build directory must be under <repo>/work (never %TEMP%)")
    if any(c in str(build_dir) for c in '"\r\n%^&|<>'):
        raise RuntimeError("unsupported build path")
    _, nupkgs = locate_godot(godot)
    logs = build_dir / "logs"
    logs.mkdir(parents=True, exist_ok=True)
    script = HERE / "native/build.cmd"
    native = run_process(f'cmd.exe /d /c call "{script}" "{build_dir}"', HERE, os.environ.copy(), 600, logs / "native-build")
    if native["exit_code"] != 0 or native["timed_out"]:
        raise RuntimeError(f"native build failed; see {logs / 'native-build.stdout.txt'}")
    project = build_dir / "godot-project"
    if project.exists():
        if not project.resolve().is_relative_to(build_dir):
            raise RuntimeError("unexpected project copy location")
        shutil.rmtree(project)
    shutil.copytree(HERE / "project", project, ignore=shutil.ignore_patterns(*SOURCE_EXCLUDED))
    command = ["dotnet", "build", "C9Probe.csproj", "-c", "Debug", "-nologo", "-nodeReuse:false",
               "-p:UseSharedCompilation=false"]
    managed = run_process(command, project, offline_environment(build_dir, nupkgs), 600, logs / "dotnet-build")
    if managed["exit_code"] != 0 or managed["timed_out"]:
        raise RuntimeError(f"offline dotnet build failed; see {logs / 'dotnet-build.stdout.txt'}")
    assembly = project / ".godot/mono/temp/bin/Debug/C9Probe.dll"
    for path in (build_dir / "c9pp.dll", build_dir / "c9pp_control.exe", assembly):
        if not path.is_file():
            raise RuntimeError(f"build output missing: {path.name}")
    return {"build_dir": str(build_dir), "project": str(project), "native_seconds": native["seconds"],
            "dotnet_seconds": managed["seconds"], "c9pp_dll_sha256": sha256(build_dir / "c9pp.dll"),
            "control_exe_sha256": sha256(build_dir / "c9pp_control.exe"), "assembly_sha256": sha256(assembly)}


def godot_version(godot):
    # --version prints and exits; it opens no window and creates no device.
    return subprocess.run([str(godot), "--version"], capture_output=True, text=True, timeout=60).stdout.strip()


def write_json(path, value):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    with open(path, "w", encoding="utf-8", newline="\n") as handle:
        json.dump(value, handle, indent=2)
        handle.write("\n")


# ---------------------------------------------------------------------------------------------
def main(argv=None):
    parser = argparse.ArgumentParser(description="C9 confirmed-presentation probe runner (route B)")
    parser.add_argument("--check-only", action="store_true", help="print the gate JSON and exit (0 quiet, 5 not quiet)")
    parser.add_argument("--build-only", action="store_true", help="build the native code and the Godot project, run nothing")
    parser.add_argument("--skip-build", action="store_true", help="use the existing build in --build-dir")
    parser.add_argument("--build-dir", type=Path, default=DEFAULT_BUILD)
    parser.add_argument("--godot", type=Path, help="Godot 4.7.2 .NET console executable")
    parser.add_argument("--topmost", action="store_true", help="always-on-top windows (default off)")
    parser.add_argument("--scale", type=float, default=1.0, help="duration scale, 0.25 to 10")
    parser.add_argument("--min-idle", type=int, default=MIN_IDLE_DEFAULT, help="required seconds without input")
    parser.add_argument("--wait-quiet", type=int, default=0, help="wait up to this many seconds for the gate")
    args = parser.parse_args(argv)
    if not 0.25 <= args.scale <= 10.0:
        parser.error("--scale must be in [0.25, 10]")
    if args.min_idle < 0 or args.wait_quiet < 0:
        parser.error("--min-idle and --wait-quiet must not be negative")

    if args.check_only:
        result = gate(args.min_idle)
        print(json.dumps(result, indent=2))
        return 0 if result["quiet"] else 5
    if args.build_only:
        try:
            identity = build(args.build_dir, args.godot)
        except RuntimeError as error:
            print(f"run_probe: {error}", file=sys.stderr)
            return 1
        print(json.dumps(identity, indent=2))
        return 0

    checked = wait_quiet(args.min_idle, args.wait_quiet)
    if not checked["quiet"]:
        print(json.dumps(checked, indent=2))
        print("run_probe: the machine is not quiet; nothing ran", file=sys.stderr)
        return 5
    try:
        godot, _ = locate_godot(args.godot)
        identity = None if args.skip_build else build(args.build_dir, args.godot)
    except RuntimeError as error:
        print(f"run_probe: {error}", file=sys.stderr)
        return 1
    build_dir = Path(args.build_dir).resolve()
    project = build_dir / "godot-project"
    if identity is None:
        identity = {"build_dir": str(build_dir), "project": str(project), "reused": True,
                    "c9pp_dll_sha256": sha256(build_dir / "c9pp.dll"),
                    "control_exe_sha256": sha256(build_dir / "c9pp_control.exe"),
                    "assembly_sha256": sha256(project / ".godot/mono/temp/bin/Debug/C9Probe.dll")}
    version = godot_version(godot)
    if version != GODOT_VERSION:
        print(f"run_probe: Godot version is {version!r}, expected {GODOT_VERSION!r}", file=sys.stderr)
        return 1
    run = PRIVATE_ROOT / datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    run.mkdir(parents=True)
    identity.update({"git_head": git_head(), "source": source_identity(), "godot_version": version,
                     "godot_sha256": sha256(godot), "scale": args.scale, "topmost": args.topmost,
                     "started_utc": utc_now()})
    write_json(run / "identity.json", identity)
    write_json(run / "gate.json", checked)

    dll = build_dir / "c9pp.dll"
    scale = f"{args.scale:g}"
    topmost = ["--topmost"] if args.topmost else []
    harnesses = [
        ("control", [str(build_dir / "c9pp_control.exe"), "--out", str(run / "control"), "--scale", scale] + topmost,
         60 * args.scale + 60),
        ("godot", [str(godot), "--path", str(project), "--rendering-driver", "d3d12", "--render-thread", "safe",
                   "--windowed", "--resolution", "1280x720", "--position", "40,40",
                   "--log-file", str(run / "godot" / "godot.log"), "--",
                   "--out", str(run / "godot"), "--dll", str(dll), "--scale", scale] + topmost,
         60 * args.scale + 120),
    ]
    runner = {"harnesses": {}, "stopped_early": False}
    exit_code = 0
    for name, command, timeout in harnesses:
        # The gate ran before the build: re-check owner idle time before each window, so a returning owner never
        # gets a (possibly topmost) probe window.
        if idle_seconds() < args.min_idle:
            print(f"run_probe: owner input within {args.min_idle} s before {name}; stopping", file=sys.stderr)
            runner["stopped_early"] = True
            runner["stopped_reason"] = f"owner input before {name}"
            exit_code = 5
            break
        (run / name).mkdir(parents=True, exist_ok=True)
        print(f"run_probe: {name} (timeout {timeout:.0f} s)")
        outcome = run_process(command, run / name, os.environ.copy(), timeout, run / name / "console")
        outcome["owner_input_during_run"] = idle_seconds() < outcome["seconds"]
        runner["harnesses"][name] = outcome
        print(f"run_probe: {name} exit {outcome['exit_code']} after {outcome['seconds']} s"
              + (" (timed out)" if outcome["timed_out"] else ""))
        if outcome["timed_out"] or outcome["exit_code"] == 3:
            runner["stopped_early"] = True
            exit_code = 3
            break
        if outcome["exit_code"] != 0:
            exit_code = 2
    runner["exit_code"] = exit_code
    runner["finished_utc"] = utc_now()
    write_json(run / "runner.json", runner)
    analyzer = subprocess.run([sys.executable, "-B", str(HERE / "analyze.py"), str(run)], capture_output=True, text=True,
                              timeout=300)
    if analyzer.returncode != 0:
        print(analyzer.stderr, file=sys.stderr)
    print(f"run_probe: results in {run} (summary.md); exit {exit_code}")
    return exit_code


if __name__ == "__main__":
    sys.exit(main())
