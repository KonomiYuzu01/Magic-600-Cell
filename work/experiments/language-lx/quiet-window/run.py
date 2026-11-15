"""One quiet-machine window for the decisive CPU timing of LX-5, the LX-5 crate follow-up and LX-6.

Experiment harness, NOT product code. It opens no window and does no GPU work. In order it runs LX-5's measure.py, the
crate follow-up's check.py --full and LX-6's measure.py at its frozen commit (through verify_frozen.py), all with one
public D6 conditions record. It waits until nobody has touched the keyboard or mouse for --settle seconds, refuses
battery power, a busy processor or a busy Codex process, holds off system sleep while it runs, and records per
run whether input happened during it. It changes no setting, priority or other program.

From the repository root; the three texts are the owner's own observations:

    python -B work/experiments/language-lx/quiet-window/run.py --vendor-mode <text> --vsync <text> --overlays <text>
    python -B work/experiments/language-lx/quiet-window/run.py --plan

--plan probes and prints the conditions and the commands and runs no measurement. Public records go to
work/experiments/language-lx/quiet-window/<UTC time>/; console logs, which can hold local paths, go to
work/loop-memory/quiet-window/ and are never published.
"""
import os
import sys

if os.name != "nt":
    raise SystemExit("Windows only: the Linux legs have their own commands")
import argparse  # noqa: E402
import ctypes  # noqa: E402
from ctypes import wintypes  # noqa: E402
from datetime import datetime, timezone  # noqa: E402
import hashlib  # noqa: E402
import json  # noqa: E402
from pathlib import Path  # noqa: E402
import shutil  # noqa: E402
import subprocess  # noqa: E402
import time  # noqa: E402
import winreg  # noqa: E402

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
LX = HERE.parent
REPO = HERE.parents[3]
LX6_CONDITIONS = LX / "lx6-engine-port/conditions.json"
STEPS = ("lx5", "crate", "lx6")
BUSY_LIMIT = 10.0  # percent of all logical processors over ten seconds
MODES = {"00000000-0000-0000-0000-000000000000": "Balanced",
         "961cc777-2547-4f9d-8174-7d86181b8a7a": "Best power efficiency",
         "ded574b5-45a0-4f42-8737-46345c09c238": "Best performance"}
PLANS = {"381b4222-f694-41f0-9685-ff5bb260df2e": "Balanced", "8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c": "High performance",
         "a1841308-3541-4fab-bc81-f71556f20b4a": "Power saver", "e9a42b02-d5df-448d-aa00-03f14749eb61": "Ultimate Performance"}
GPU_VENDORS = ("NVIDIA", "AMD", "Radeon", "Intel")
DISPLAY_CLASS = r"SYSTEM\CurrentControlSet\Control\Class\{4d36e968-e325-11ce-bfc1-08002be10318}"
ES_CONTINUOUS, ES_SYSTEM_REQUIRED = 0x80000000, 0x00000001
ESTIMATE_MINUTES = {"lx5": 15, "crate": 3, "lx6": 3}  # rough: the stored smoke samples scaled to full mode, plus builds
SECRET_WORDS = ("TOKEN", "SECRET", "PASSWORD", "API_KEY", "APIKEY", "CREDENTIAL")  # never passed to the measured runs

kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
user32 = ctypes.WinDLL("user32", use_last_error=True)
powrprof = ctypes.WinDLL("powrprof", use_last_error=True)


class GUID(ctypes.Structure):
    _fields_ = [("data1", wintypes.DWORD), ("data2", wintypes.WORD), ("data3", wintypes.WORD), ("data4", ctypes.c_ubyte * 8)]

    def text(self):
        tail = bytes(self.data4)
        return f"{self.data1:08x}-{self.data2:04x}-{self.data3:04x}-{tail[:2].hex()}-{tail[2:].hex()}"


class POWER_STATUS(ctypes.Structure):
    _fields_ = [("ac", ctypes.c_ubyte), ("battery_flag", ctypes.c_ubyte), ("battery_percent", ctypes.c_ubyte),
                ("saver", ctypes.c_ubyte), ("life", wintypes.DWORD), ("full_life", wintypes.DWORD)]


class FILETIME(ctypes.Structure):
    _fields_ = [("low", wintypes.DWORD), ("high", wintypes.DWORD)]


class LASTINPUTINFO(ctypes.Structure):
    _fields_ = [("size", wintypes.UINT), ("time", wintypes.DWORD)]


class PROCESSENTRY32W(ctypes.Structure):
    _fields_ = [("size", wintypes.DWORD), ("usage", wintypes.DWORD), ("pid", wintypes.DWORD),
                ("heap", ctypes.c_size_t), ("module", wintypes.DWORD), ("threads", wintypes.DWORD),
                ("parent", wintypes.DWORD), ("priority", ctypes.c_long), ("flags", wintypes.DWORD),
                ("exe", wintypes.WCHAR * 260)]


kernel32.CreateToolhelp32Snapshot.restype = wintypes.HANDLE
kernel32.CreateToolhelp32Snapshot.argtypes = [wintypes.DWORD, wintypes.DWORD]
kernel32.Process32FirstW.argtypes = [wintypes.HANDLE, ctypes.POINTER(PROCESSENTRY32W)]
kernel32.Process32NextW.argtypes = [wintypes.HANDLE, ctypes.POINTER(PROCESSENTRY32W)]
kernel32.CloseHandle.argtypes = [wintypes.HANDLE]
kernel32.LocalFree.argtypes = [ctypes.c_void_p]
kernel32.GetTickCount.restype = wintypes.DWORD
kernel32.SetThreadExecutionState.restype = wintypes.DWORD
kernel32.SetThreadExecutionState.argtypes = [wintypes.DWORD]
kernel32.OpenProcess.restype = wintypes.HANDLE
kernel32.OpenProcess.argtypes = [wintypes.DWORD, wintypes.BOOL, wintypes.DWORD]
kernel32.GetProcessTimes.argtypes = [wintypes.HANDLE] + [ctypes.POINTER(FILETIME)] * 4


def sha(data):
    return hashlib.sha256(data).hexdigest()


def power_source():
    status = POWER_STATUS()
    if not kernel32.GetSystemPowerStatus(ctypes.byref(status)):
        return "unknown"
    source = {0: "battery", 1: "mains"}.get(status.ac, "unknown")
    return source + ("; battery saver on" if status.saver else "")


def windows_power_mode():
    scheme = ctypes.POINTER(GUID)()
    if powrprof.PowerGetActiveScheme(None, ctypes.byref(scheme)) != 0:
        return "unknown"
    try:
        plan = scheme.contents.text()
    finally:
        kernel32.LocalFree(ctypes.cast(scheme, ctypes.c_void_p))
    overlay = GUID()
    try:  # not in the SDK headers, but exported by powrprof.dll since Windows 10
        mode = MODES.get(overlay.text(), "overlay " + overlay.text()) \
            if powrprof.PowerGetEffectiveOverlayScheme(ctypes.byref(overlay)) == 0 else "unknown"
    except AttributeError:
        mode = "unknown"
    return f"power mode {mode}; power plan {PLANS.get(plan, 'custom ' + plan)}"


def cpu():
    with winreg.OpenKey(winreg.HKEY_LOCAL_MACHINE, r"HARDWARE\DESCRIPTION\System\CentralProcessor\0") as key:
        name = winreg.QueryValueEx(key, "ProcessorNameString")[0].strip()
    return f"{name}; {os.cpu_count()} logical processors"


def gpu():
    names = set()
    with winreg.OpenKey(winreg.HKEY_LOCAL_MACHINE, DISPLAY_CLASS) as root:
        for index in range(winreg.QueryInfoKey(root)[0]):
            sub = winreg.EnumKey(root, index)
            if not sub.isdigit():
                continue
            try:
                with winreg.OpenKey(root, sub) as key:
                    name = winreg.QueryValueEx(key, "DriverDesc")[0]
                    version = winreg.QueryValueEx(key, "DriverVersion")[0]
            except OSError:
                continue
            if any(vendor in name for vendor in GPU_VENDORS):  # physical adapters only, no virtual displays
                names.add(f"{name} (driver {version})")
    return "; ".join(sorted(names)) or "unknown"


def cpu_times():
    idle, kernel, user = FILETIME(), FILETIME(), FILETIME()
    if not kernel32.GetSystemTimes(ctypes.byref(idle), ctypes.byref(kernel), ctypes.byref(user)):
        raise OSError("GetSystemTimes failed")
    return [(t.high << 32) | t.low for t in (idle, kernel, user)]


def cpu_busy(seconds=10.0):
    idle0, kernel0, user0 = cpu_times()
    time.sleep(seconds)
    idle1, kernel1, user1 = cpu_times()
    total = (kernel1 - kernel0) + (user1 - user0)  # kernel time includes idle time
    return 100.0 * (total - (idle1 - idle0)) / total if total else 0.0


def processes():
    snapshot = kernel32.CreateToolhelp32Snapshot(0x2, 0)
    if snapshot in (None, wintypes.HANDLE(-1).value):
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


def codex_busy(window=2.0):
    """Busy Codex processes under the recorded rule: one the ChatGPT app started counts only when it used more than
    0.1 s of CPU in 2 s; any other one, the project wrapper's included, always counts. An unreadable helper counts."""
    found = processes()
    codex = [(pid, found.get(parent, ("",))[0]) for pid, (exe, parent) in found.items() if exe == "codex.exe"]
    helpers = [pid for pid, parent in codex if parent == "chatgpt.exe"]
    before = {pid: process_cpu_seconds(pid) for pid in helpers}
    time.sleep(window)
    busy = len(codex) - len(helpers)
    for pid in helpers:
        after = process_cpu_seconds(pid)
        if before[pid] is None or (after is not None and after - before[pid] > 0.1):
            busy += 1
    return busy


def idle_seconds():
    info = LASTINPUTINFO()
    info.size = ctypes.sizeof(info)
    if not user32.GetLastInputInfo(ctypes.byref(info)):
        raise OSError("GetLastInputInfo failed")
    return ((kernel32.GetTickCount() - info.time) & 0xFFFFFFFF) / 1000.0


def settle(seconds, limit=1800):
    start = time.monotonic()
    while idle_seconds() < seconds:
        if time.monotonic() - start > limit:
            raise SystemExit(f"no {seconds} s without keyboard or mouse input within {limit // 60} minutes; nothing ran")
        time.sleep(5)
    return round(time.monotonic() - start)


def engine_python(given):
    if given:
        return Path(given)
    common = subprocess.run(["git", "rev-parse", "--path-format=absolute", "--git-common-dir"], cwd=REPO,
                            capture_output=True, text=True, check=True).stdout.strip()
    for root in (REPO, Path(common).parent):
        candidate = root / "tools/.venv/engine/Scripts/python.exe"
        if candidate.exists():
            return candidate
    raise SystemExit("the pinned engine interpreter tools/.venv/engine was not found; pass --engine-python")


def commands(out, conditions, engine):
    lx6_conditions = LX6_CONDITIONS.relative_to(REPO).as_posix()
    return {
        "lx5": ([sys.executable, "-B", "work/experiments/language-lx/lx5-arithmetic/measure.py",
                 "--conditions", conditions, "--output", out / "lx5-full.json"], {}),
        "crate": ([sys.executable, "-I", "-B", "work/experiments/language-lx/lx5-crate/check.py", "--full",
                   "--conditions", conditions, "--output", out / "crate-full.json"], {}),
        "lx6": ([sys.executable, "-B", "work/experiments/language-lx/verify_frozen.py", "lx6-engine-port", "measure.py",
                 "--conditions", lx6_conditions, "--output", out / "lx6-full.json"], {"LX6_ENGINE_PYTHON": str(engine)}),
    }


def public(command):
    """The command as it may be published: no interpreter path, repository paths relative."""
    words = []
    for word in command:
        if word == sys.executable:
            words.append("python")
        elif isinstance(word, Path):
            words.append(word.resolve().relative_to(REPO).as_posix())
        else:
            words.append(str(word))
    return " ".join(words)


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--vendor-mode", help="the laptop vendor's performance mode, as the owner sees it")
    parser.add_argument("--vsync", help="the driver's V-Sync setting, as the owner sees it")
    parser.add_argument("--overlays", help="overlays running, as the owner sees them")
    parser.add_argument("--frame-generation", default="off", help="the renderer gate requires off")
    parser.add_argument("--only", default=",".join(STEPS), help="comma list out of " + ", ".join(STEPS))
    parser.add_argument("--settle", type=int, default=120, help="seconds without input before the first run")
    parser.add_argument("--engine-python", help="the pinned engine interpreter for LX-6 (default: tools/.venv/engine)")
    parser.add_argument("--plan", action="store_true", help="probe and print; run no measurement")
    args = parser.parse_args()
    chosen = [name for name in STEPS if name in args.only.split(",")]
    if not chosen or set(args.only.split(",")) - set(STEPS):
        parser.error("--only takes a comma list out of " + ", ".join(STEPS))
    if not args.plan and not (args.vendor_mode and args.vsync and args.overlays):
        parser.error("--vendor-mode, --vsync and --overlays record the owner's own observations")
    for path in (LX / "lx5-arithmetic/build", LX / "lx5-crate/build", REPO / "work/lxv", LX6_CONDITIONS):
        if path.exists():
            raise SystemExit(path.relative_to(REPO).as_posix() + " exists; another run may own it")
    engine = engine_python(args.engine_python)
    conditions = {"power_source": power_source(), "windows_power_mode": windows_power_mode(),
                  "vendor_performance_mode": args.vendor_mode or "(owner's observation)",
                  "driver_vsync": args.vsync or "(owner's observation)", "overlays": args.overlays or "(owner's observation)",
                  "frame_generation": args.frame_generation, "cpu": cpu(), "gpu": gpu(), "quiet_machine": "(checked at start)",
                  "evidence_class": "performance on the owner's machine: Windows, CPU only, no GPU or display work"}
    stamp = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H%MZ")
    out, logs = HERE / stamp, REPO / "work/loop-memory/quiet-window" / stamp
    planned = commands(out, out / "conditions.json", engine)
    if args.plan:
        busy = cpu_busy()
        print(json.dumps(conditions, indent=2, ensure_ascii=True))
        print(f"Now: CPU busy {busy:.1f}% over 10 s (limit {BUSY_LIMIT:.0f}%); busy Codex processes {codex_busy()}; "
              f"no input for {idle_seconds():.0f} s")
        for name in chosen:
            print(name + ": " + public(planned[name][0]))
        print(f"Roughly {sum(ESTIMATE_MINUTES[name] for name in chosen)} minutes plus the settle wait, from the stored "
              "smoke samples scaled to full mode. Nothing ran.")
        return 0
    if conditions["power_source"] != "mains":
        raise SystemExit("power source is " + conditions["power_source"] + "; plug in first. Nothing ran.")
    print(f"Waiting for {args.settle} s without keyboard or mouse input ...", flush=True)
    waited = settle(args.settle)
    busy, codex = cpu_busy(), codex_busy()
    if busy > BUSY_LIMIT or codex:
        raise SystemExit(f"machine not quiet: CPU busy {busy:.1f}% (limit {BUSY_LIMIT:.0f}%), busy Codex processes "
                         f"{codex}. Nothing ran.")
    conditions["quiet_machine"] = (f"checked at start: no keyboard or mouse input for at least {args.settle} s; "
                                   f"system CPU busy {busy:.1f}% over 10 s (limit {BUSY_LIMIT:.0f}%); no busy Codex "
                                   "process; input during each run is recorded in window.json")
    out.mkdir(parents=True)
    logs.mkdir(parents=True)
    record = (json.dumps(conditions, indent=2, sort_keys=True, ensure_ascii=True) + "\n").encode("ascii")
    (out / "conditions.json").write_bytes(record)
    started = datetime.now(timezone.utc).isoformat()
    runs = []
    child = {key: value for key, value in os.environ.items() if not any(word in key.upper() for word in SECRET_WORDS)}
    kernel32.SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED)
    try:
        shutil.copyfile(out / "conditions.json", LX6_CONDITIONS)
        for name in chosen:
            command, extra = planned[name]
            print(f"{name}: running", flush=True)
            begin, clock = datetime.now(timezone.utc).isoformat(), time.monotonic()
            with open(logs / (name + ".log"), "wb") as log:
                exit_code = subprocess.run([str(word) for word in command], cwd=REPO, env=dict(child, **extra),
                                           stdout=log, stderr=subprocess.STDOUT).returncode
            seconds = time.monotonic() - clock
            output = out / (name + "-full.json")
            runs.append({"name": name, "command": public(command), "started_utc": begin, "seconds": round(seconds, 1),
                         "exit": exit_code, "owner_input_during_run": idle_seconds() < seconds,
                         "output": output.name, "output_sha256": sha(output.read_bytes()) if output.exists() else None})
            print(f"{name}: exit {exit_code} after {seconds:.0f} s; input during the run: "
                  f"{runs[-1]['owner_input_during_run']}", flush=True)
    finally:
        kernel32.SetThreadExecutionState(ES_CONTINUOUS)
        LX6_CONDITIONS.unlink(missing_ok=True)
        window = {"format": "lx-quiet-window/1", "started_utc": started, "finished_utc": datetime.now(timezone.utc).isoformat(),
                  "conditions_sha256": sha(record), "settle_seconds": args.settle, "settle_wait_seconds": waited,
                  "sleep": "system sleep held off for the window by an execution-state request; display left to the owner's settings",
                  "runs": runs, "decision": "open", "decider": "owner",
                  "reading": "A run with input during it, a non-zero exit or changed conditions is a lead, not decisive evidence."}
        (out / "window.json").write_bytes((json.dumps(window, indent=2, sort_keys=True, ensure_ascii=True) + "\n").encode("ascii"))
    print("Records: " + out.relative_to(REPO).as_posix(), flush=True)
    return 0 if len(runs) == len(chosen) and all(run["exit"] == 0 for run in runs) else 1


if __name__ == "__main__":
    raise SystemExit(main())
