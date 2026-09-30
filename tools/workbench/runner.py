"""Detached executor for fixed workbench plans; no shell or Qt imports."""
from __future__ import annotations

import sys

sys.dont_write_bytecode = True

import argparse
import ctypes
import hashlib
import json
import os
import re
import signal
import subprocess
import threading
import time
from pathlib import Path

try:
    from . import paths, runs
except ImportError:
    import paths
    import runs

LOG_LIMIT = 32 * 1024 * 1024
INPUT_FILES = 500
INPUT_BYTES = 256 * 1024 * 1024
CALL_RE = re.compile(rb"^(plan|review|implement) (\d{8}T\d{6}Z-[0-9a-f]{8}):")


class _Windows:
    """Non-inheritable nested jobs and documented suspended-thread startup."""

    def __init__(self):
        from ctypes import wintypes as w

        class Basic(ctypes.Structure):
            _fields_ = [("PerProcessUserTimeLimit", ctypes.c_longlong), ("PerJobUserTimeLimit", ctypes.c_longlong),
                        ("LimitFlags", w.DWORD), ("MinimumWorkingSetSize", ctypes.c_size_t),
                        ("MaximumWorkingSetSize", ctypes.c_size_t), ("ActiveProcessLimit", w.DWORD),
                        ("Affinity", ctypes.c_size_t), ("PriorityClass", w.DWORD), ("SchedulingClass", w.DWORD)]

        class IO(ctypes.Structure):
            _fields_ = [(name, ctypes.c_ulonglong) for name in ("ReadOperationCount", "WriteOperationCount",
                        "OtherOperationCount", "ReadTransferCount", "WriteTransferCount", "OtherTransferCount")]

        class Extended(ctypes.Structure):
            _fields_ = [("BasicLimitInformation", Basic), ("IoInfo", IO), ("ProcessMemoryLimit", ctypes.c_size_t),
                        ("JobMemoryLimit", ctypes.c_size_t), ("PeakProcessMemoryUsed", ctypes.c_size_t),
                        ("PeakJobMemoryUsed", ctypes.c_size_t)]

        class Thread(ctypes.Structure):
            _fields_ = [("dwSize", w.DWORD), ("cntUsage", w.DWORD), ("th32ThreadID", w.DWORD),
                        ("th32OwnerProcessID", w.DWORD), ("tpBasePri", w.LONG), ("tpDeltaPri", w.LONG), ("dwFlags", w.DWORD)]

        self.extended, self.thread = Extended, Thread
        self.k = ctypes.WinDLL("kernel32", use_last_error=True)
        signatures = {
            "CreateJobObjectW": ([w.LPVOID, w.LPCWSTR], w.HANDLE),
            "SetInformationJobObject": ([w.HANDLE, ctypes.c_int, w.LPVOID, w.DWORD], w.BOOL),
            "AssignProcessToJobObject": ([w.HANDLE, w.HANDLE], w.BOOL),
            "GetCurrentProcess": ([], w.HANDLE),
            "CloseHandle": ([w.HANDLE], w.BOOL),
            "TerminateJobObject": ([w.HANDLE, w.UINT], w.BOOL),
            "TerminateProcess": ([w.HANDLE, w.UINT], w.BOOL),
            "CreateToolhelp32Snapshot": ([w.DWORD, w.DWORD], w.HANDLE),
            "Thread32First": ([w.HANDLE, ctypes.POINTER(Thread)], w.BOOL),
            "Thread32Next": ([w.HANDLE, ctypes.POINTER(Thread)], w.BOOL),
            "OpenThread": ([w.DWORD, w.BOOL, w.DWORD], w.HANDLE),
            "ResumeThread": ([w.HANDLE], w.DWORD),
        }
        for name, (args, result) in signatures.items():
            fn = getattr(self.k, name)
            fn.argtypes, fn.restype = args, result

    def job(self):
        handle = self.k.CreateJobObjectW(None, None)
        if not handle:
            raise ctypes.WinError(ctypes.get_last_error())
        info = self.extended()
        info.BasicLimitInformation.LimitFlags = 0x2000  # JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE only
        if not self.k.SetInformationJobObject(handle, 9, ctypes.byref(info), ctypes.sizeof(info)):
            error = ctypes.WinError(ctypes.get_last_error())
            self.k.CloseHandle(handle)
            raise error
        return handle

    def assign(self, job, process):
        if not self.k.AssignProcessToJobObject(job, process):
            raise ctypes.WinError(ctypes.get_last_error())

    def own_runner(self):
        job = self.job()
        try:
            self.assign(job, self.k.GetCurrentProcess())
        except OSError:
            self.k.CloseHandle(job)
            raise
        # Do not close J0 in Python: process exit closes it and ends all descendants.
        self.runner_job = job

    def resume(self, pid):
        snapshot = self.k.CreateToolhelp32Snapshot(0x4, 0)  # TH32CS_SNAPTHREAD
        if snapshot == ctypes.c_void_p(-1).value:
            raise ctypes.WinError(ctypes.get_last_error())
        resumed = 0
        try:
            item = self.thread()
            item.dwSize = ctypes.sizeof(item)
            ok = self.k.Thread32First(snapshot, ctypes.byref(item))
            if not ok:
                raise ctypes.WinError(ctypes.get_last_error())
            while ok:
                if item.th32OwnerProcessID == pid:
                    thread = self.k.OpenThread(0x2, False, item.th32ThreadID)  # THREAD_SUSPEND_RESUME
                    if not thread:
                        raise ctypes.WinError(ctypes.get_last_error())
                    try:
                        if self.k.ResumeThread(thread) == 0xFFFFFFFF:
                            raise ctypes.WinError(ctypes.get_last_error())
                        resumed += 1
                    finally:
                        self.k.CloseHandle(thread)
                ok = self.k.Thread32Next(snapshot, ctypes.byref(item))
            if ctypes.get_last_error() != 18:  # ERROR_NO_MORE_FILES
                raise ctypes.WinError(ctypes.get_last_error())
            if not resumed:
                raise OSError("no suspended step thread was found")
        finally:
            self.k.CloseHandle(snapshot)

    def terminate_job(self, job):
        if not self.k.TerminateJobObject(job, 1):
            raise ctypes.WinError(ctypes.get_last_error())

    def terminate_process(self, proc):
        if proc.poll() is None and not self.k.TerminateProcess(int(proc._handle), 1):
            raise ctypes.WinError(ctypes.get_last_error())


class _Log:
    def __init__(self, path: Path, record: dict):
        self.file = path.open("ab")
        self.size = self.file.tell()
        self.record = record
        self.lock = threading.Lock()
        self.partial = b""
        self.error = None

    def write(self, data: bytes):
        with self.lock:
            kept = data[:max(0, LOG_LIMIT - self.size)]
            self.record["log_dropped"] += len(data) - len(kept)
            self.file.write(kept)
            self.file.flush()
            self.size += len(kept)
            if self.record["kind"] == "codex" and self.record["call_id"] is None:
                lines = (self.partial + kept).split(b"\n")
                self.partial = lines.pop()[:128]
                for line in lines:
                    match = CALL_RE.match(line)
                    if match:
                        self.record["call_id"] = match[2].decode("ascii")
                        break

    def line(self, text: str):
        self.write((text + "\n").encode("utf-8", errors="replace"))

    def pump(self, pipe):
        try:
            # A buffered read(size) can wait for size bytes, hiding live output.
            while chunk := os.read(pipe.fileno(), 65536):
                self.write(chunk)
        except (OSError, ValueError) as exc:
            self.error = exc

    def close(self):
        self.file.close()


def _inputs(checkout: Path, patterns: list[str]) -> dict:
    files, total, incomplete = {}, 0, False
    for pattern in patterns:
        try:
            for file in checkout.glob(pattern.replace("\\", "/")):
                if not file.is_file():
                    continue
                rel = file.relative_to(checkout).as_posix()
                if rel in files:
                    continue
                if not paths.inside(file, [checkout]):
                    incomplete = True
                    continue
                if len(files) >= INPUT_FILES or file.stat().st_size > INPUT_BYTES - total:
                    return {"files": files, "incomplete": True}
                digest = hashlib.sha256()
                try:
                    with file.open("rb") as f:
                        while chunk := f.read(1 << 20):
                            total += len(chunk)
                            if total > INPUT_BYTES:
                                return {"files": files, "incomplete": True}
                            digest.update(chunk)
                    files[rel] = digest.hexdigest()
                except OSError:
                    incomplete = True
        except (OSError, ValueError):
            incomplete = True
    return {"files": files, "incomplete": incomplete}


def _outputs(checkout: Path, patterns: list[str], started: float) -> list[str]:
    found = set()
    for pattern in patterns:
        try:
            for file in checkout.glob(pattern.replace("\\", "/")):
                if file.is_file() and paths.inside(file, [checkout]) and file.stat().st_mtime >= started:
                    found.add(file.relative_to(checkout).as_posix())
        except (OSError, ValueError):
            continue
    return sorted(found)


def _kill_group(proc):
    try:
        os.killpg(proc.pid, signal.SIGKILL)
    except ProcessLookupError:
        pass


def _step(argv: list[str], shown: list[str], record: dict, log: _Log, control, windows) -> tuple[str | None, str | None]:
    log.line("$ " + " ".join(shown))
    env = dict(os.environ, PYTHONUNBUFFERED="1", PATH=runs.step_path())
    flags = {"creationflags": 0x4 | subprocess.CREATE_NO_WINDOW} if windows else {"start_new_session": True}  # CREATE_SUSPENDED
    proc = subprocess.Popen(argv, cwd=record["cwd"], env=env, stdin=subprocess.DEVNULL,
                            stdout=subprocess.PIPE, stderr=subprocess.STDOUT, bufsize=0, **flags)
    record["exits"].append(None)
    job, pump, status, reason = None, None, None, None
    try:
        if windows:
            try:
                job = windows.job()
                barrier = os.environ.get("WORKBENCH_RUNNER_TEST_BARRIER")
                if barrier:
                    directory = Path(barrier)
                    (directory / "before-assign.reached").write_text(str(proc.pid), encoding="ascii")
                    deadline = time.monotonic() + 60
                    while not (directory / "go").exists():
                        status = control()
                        if status:
                            break
                        if time.monotonic() >= deadline:
                            raise OSError("test barrier timed out before job assignment")
                        time.sleep(0.1)
                if status is None:
                    if os.environ.get("WORKBENCH_RUNNER_TEST_FAIL") == "assign":
                        raise OSError("test hook refused the step job assignment")
                    windows.assign(job, int(proc._handle))
                    windows.resume(proc.pid)
            except Exception as exc:
                status, reason = "failed", f"cannot assign or resume the step: {exc}"
            if status is not None:
                windows.terminate_process(proc)
                proc.wait()
                return status, reason
        pump = threading.Thread(target=log.pump, args=(proc.stdout,), daemon=True)
        pump.start()
        while proc.poll() is None:
            status = control()
            if status is not None:
                if windows:
                    windows.terminate_job(job)
                else:
                    _kill_group(proc)
                break
            time.sleep(0.1)
        record["exits"][-1] = proc.wait()
        # Js also closes pipes retained by descendants after the step exits.
        if job is not None:
            windows.k.CloseHandle(job)
            job = None
        pump.join(timeout=2)
        if pump.is_alive() and not windows:
            _kill_group(proc)
            pump.join(timeout=2)
        if pump.is_alive():
            return "failed", "step output did not close"
        if log.error:
            return "failed", f"cannot read step output: {log.error}"
        return status, reason
    finally:
        if proc.poll() is None:
            if windows:
                windows.terminate_process(proc)
            else:
                _kill_group(proc)
            proc.wait()
        if job is not None:
            windows.k.CloseHandle(job)
        if pump is not None:
            pump.join(timeout=2)
        proc.stdout.close()
        code = record["exits"][-1]
        log.line(f"[exit {code if code is not None else 'null'}]")


def _read_record(directory: Path, rid: str) -> dict | None:
    try:
        with (directory / (rid + ".json")).open("rb") as f:
            raw = f.read(runs.JSON_LIMIT + 1)
        record = json.loads(raw) if len(raw) <= runs.JSON_LIMIT else None
        return record if isinstance(record, dict) and record.get("rid") == rid else None
    except (OSError, ValueError, UnicodeError, RecursionError):
        return None


def execute(args) -> int:
    if re.fullmatch(runs.RID_RE, args.rid) is None:
        return 2
    main, checkout = Path(args.main).resolve(), Path(args.checkout).resolve()
    directory = paths.data_root(main) / "runs"
    locks = directory / "locks"
    name = "codex-" + args.kind if args.codex else args.run
    record = runs._new_record(args.rid, {"kind": "codex" if args.codex else "registry", "run": name,
                             "title": name, "checkout": str(checkout), "cwd": str(checkout), "steps": [],
                             "display": [], "timeout_s": None, "digest": None, "windows_required": False}, None)
    fds, log, windows = [], None, None

    def finish(status, reason=None, code=1):
        record.update(status=status, reason=reason, ended=runs._iso(), alive_at=runs._iso())
        runs._write_record(directory, record)
        return code

    def refuse(reason):
        existing = _read_record(directory, args.rid)
        if existing is not None:
            record.update(existing)
        return finish("refused", reason, 2)

    try:
        locks.mkdir(parents=True, exist_ok=True)
        if os.name == "nt":
            try:
                windows = _Windows()
                windows.own_runner()
            except Exception:
                return refuse("cannot place the runner in a job object")
        deadline = time.monotonic() + 2
        while True:
            try:
                fds.append(runs._take_lock(locks / record["lock"]))
                break
            except OSError:
                if time.monotonic() >= deadline:
                    return refuse("cannot take the liveness lock")
                time.sleep(0.1)
        starting = _read_record(directory, args.rid)
        if starting is None or starting.get("status") != "starting":
            return refuse("no starting record")
        record.update(starting)
        created = runs._timestamp(record.get("created"))
        if created is None or time.time() - created >= 25:
            return finish("refused", "the runner started too late", 2)
        try:
            if args.codex:
                plan = runs.plan_codex(main, checkout, args.kind, args.packet, args.model, args.effort, args.speed)
            else:
                plan = runs.plan_run(main, checkout, args.run)
            if not args.codex and plan["digest"] != args.expect:
                return finish("refused", "the registry entry changed after it was shown", 2)
            entry, registry_sha = runs._entry_for_plan(main, plan)
        except runs.RunRefused as exc:
            return finish("refused", str(exc), 2)
        record.update(plan, registry_sha256=registry_sha)
        record["lock"] = "rid-" + args.rid + ".lock"
        if not args.codex:
            record["entry_lock"] = runs._entry_lock_name(checkout, args.run)
            try:
                fds.append(runs._take_lock(locks / record["entry_lock"]))
            except OSError:
                return finish("refused", "already running", 2)
        record["python"] = next((a[0] for a, d in zip(plan["steps"], plan["display"]) if runs._python(d[0])), None)
        record["inputs"] = _inputs(checkout, entry["inputs"])
        record.update(status="running", started=runs._iso(), alive_at=runs._iso(), reason=None)
        runs._write_record(directory, record)
        began = last_heartbeat = time.monotonic()

        def control():
            nonlocal last_heartbeat
            now = time.monotonic()
            if now - last_heartbeat >= 5:
                record["alive_at"] = runs._iso()
                runs._write_record(directory, record)
                last_heartbeat = now
            if (directory / (args.rid + ".cancel")).exists():
                return "cancelled"
            if plan["timeout_s"] is not None and now - began >= plan["timeout_s"]:
                return "timed_out"
            return None

        log = _Log(directory / (args.rid + ".log"), record)
        status, reason = None, None
        for argv, shown in zip(plan["steps"], plan["display"]):
            status = control()
            if status is not None:
                break
            status, reason = _step(argv, shown, record, log, control, windows)
            if status is not None:
                break
        if status is None:
            status = "passed" if all(code == 0 for code in record["exits"]) else "failed"
        record["outputs"] = _outputs(checkout, entry["outputs"], runs._timestamp(record["started"]))
        return finish(status, reason, 0 if status == "passed" else 1)
    except Exception as exc:
        try:
            return finish("failed", str(exc))
        except Exception:
            return 1
    finally:
        if log is not None:
            log.close()
        for fd in reversed(fds):
            runs._release_lock(fd)


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__, allow_abbrev=False)
    for name in ("main", "checkout", "rid"):
        parser.add_argument("--" + name, required=True)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--run")
    mode.add_argument("--codex", action="store_true")
    parser.add_argument("--expect")
    for name in ("kind", "packet", "model", "effort", "speed"):
        parser.add_argument("--" + name)
    args = parser.parse_args(argv)
    values = (args.kind, args.packet, args.model, args.effort, args.speed)
    if args.codex:
        if any(v is None for v in values) or args.expect is not None:
            parser.error("--codex requires --kind, --packet, --model, --effort and --speed only")
    elif args.expect is None or any(v is not None for v in values):
        parser.error("--run requires --expect and accepts no Codex options")
    return execute(args)


if __name__ == "__main__":
    raise SystemExit(main())
