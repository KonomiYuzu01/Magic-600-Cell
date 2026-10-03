"""Read the Windows process list and identify only the batch's heavy work."""
from __future__ import annotations

import json
import subprocess

PROCESS_COMMAND = (
    "Get-CimInstance Win32_Process | "
    "Select-Object ProcessId,ParentProcessId,Name,CommandLine | ConvertTo-Json -Compress"
)
BUILD = {"cl", "link", "lib", "ninja", "cmake", "msbuild", "devenv", "csc",
         "vbcscompiler", "dxc", "fxc", "cargo", "rustc"}
GPU = {"sb_probe", "sb_handoff", "nativehostregression", "c600native", "mpult"}


def list_processes(timeout=60) -> list[dict]:
    try:
        result = subprocess.run(
            ["powershell.exe", "-NoProfile", "-NonInteractive", "-Command", PROCESS_COMMAND],
            stdin=subprocess.DEVNULL, capture_output=True, encoding="utf-8", errors="replace",
            timeout=timeout)
    except subprocess.TimeoutExpired:
        raise RuntimeError("process list timed out") from None
    except OSError as exc:
        raise RuntimeError("process list unavailable: " + type(exc).__name__) from None
    if result.returncode:
        raise RuntimeError(f"process list failed (exit {result.returncode})")
    try:
        records = json.loads(result.stdout.lstrip("\ufeff"))
        if isinstance(records, dict):
            records = [records]
        if not isinstance(records, list):
            raise ValueError("expected process objects")
        return [{"pid": int(p["ProcessId"]), "ppid": int(p["ParentProcessId"]),
                 "name": str(p["Name"]), "command": p.get("CommandLine")} for p in records]
    except (ValueError, TypeError, KeyError):
        raise RuntimeError("process list returned invalid data") from None


def heavy(processes, own_pid) -> list[tuple[int, str, str]]:
    excluded = {own_pid}
    while True:
        descendants = {p["pid"] for p in processes if p["ppid"] in excluded}
        added = descendants - excluded
        if not added:
            break
        excluded.update(added)
    found = []
    for process in processes:
        if process["pid"] in excluded:
            continue
        name = process["name"]
        executable = name.casefold().removesuffix(".exe")
        command = (process["command"] or "").casefold()
        if executable in BUILD:
            why = "build"
        elif executable in {"blender", "ffmpeg"}:
            why = "heavy tool"
        elif executable.startswith("godot"):
            why = "Godot"
        elif executable in GPU:
            why = "project GPU program"
        elif executable.startswith("presentmon") and executable != "presentmonservice":
            why = "PresentMon capture"
        elif "codex_review.py" in command and "implement" in command:
            why = "Codex implementation call"
        else:
            continue
        found.append((process["pid"], name, why))
    return found
