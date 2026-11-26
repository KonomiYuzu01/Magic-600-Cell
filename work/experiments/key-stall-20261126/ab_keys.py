"""Interleaved on-screen runs of one product build with its own host (`old-N`) and another host DLL (`new-N`), with
turn keys posted to the shell's window.

    python -B work/experiments/key-stall-20261126/ab_keys.py --build DIR --host DLL --out-root DIR [--core DLL]
        [--seconds 120] [--pace 0.5] [--plan old-1,new-1,old-2,new-2] [--puzzle jumble-off|jumble-on]

Each run starts the build's shell with the command and environment that `renderer/shell/launch.py` gives it (after
`build_product.verify`, a fresh copy of the puzzle's session fixture, no LX-4, LX-2 or editor), with only
`M600_HOST_DLL` replaced in a `new` run (and `+host-key-stall` appended to `M600_BUILD_IDENTITY`, so its run row says
so); with `--core`, a `new` run also replaces `M600_CORE_DLL` (a `renderer/core/build.cmd` output folder's
`sa2_interop.dll`, which loads the shaders beside it) and appends `+core`. It finds the shell's window, asks once for
the foreground (SetForegroundWindow, no foreground-lock workaround), waits 10 s, then posts one turn key every `--pace` seconds for `--seconds`, cycling J, K and L (WM_KEYDOWN and WM_KEYUP
with the key's scan code, as tools/perf/c9/lx4.py posts a turn). Then it posts WM_CLOSE and writes `launch.json`
(`keys.py` in work/experiments/presented-gap-20261126 reads the run folder) and `posts.json` (each post's QPC time and
whether the shell had the foreground then; the owner's idle seconds at the start). Keep `--out-root` under the
ignored `work/`.

An experiment, not a measurement of record: the window starts from this process, so it may not get the foreground,
and nothing here controls what else is on the desktop.
"""
from __future__ import annotations

import argparse
import ctypes
import datetime as dt
import json
import shutil
import subprocess
import sys
import time
from ctypes import wintypes
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "renderer" / "shell"))
sys.path.insert(0, str(ROOT / "tools" / "perf" / "c9"))
import build_product  # noqa: E402
import launch  # noqa: E402
import lx4  # noqa: E402

USER32 = ctypes.WinDLL("user32")
KERNEL32 = ctypes.WinDLL("kernel32")
WM_CLOSE = 0x0010
ENUM = ctypes.WINFUNCTYPE(wintypes.BOOL, wintypes.HWND, wintypes.LPARAM)
USER32.PostMessageW.argtypes = (wintypes.HWND, wintypes.UINT, wintypes.WPARAM, wintypes.LPARAM)
USER32.GetForegroundWindow.restype = wintypes.HWND
USER32.SetForegroundWindow.argtypes = (wintypes.HWND,)
USER32.GetWindowThreadProcessId.argtypes = (wintypes.HWND, ctypes.POINTER(wintypes.DWORD))
USER32.IsWindowVisible.argtypes = (wintypes.HWND,)
USER32.GetWindow.argtypes = (wintypes.HWND, wintypes.UINT)
USER32.GetWindow.restype = wintypes.HWND


class LastInput(ctypes.Structure):
    _fields_ = [("cbSize", wintypes.UINT), ("dwTime", wintypes.DWORD)]


def idle_seconds() -> float:
    info = LastInput(ctypes.sizeof(LastInput), 0)
    USER32.GetLastInputInfo(ctypes.byref(info))
    return (KERNEL32.GetTickCount() - info.dwTime) / 1000.0


def qpc() -> int:
    value = ctypes.c_int64()
    KERNEL32.QueryPerformanceCounter(ctypes.byref(value))
    return value.value


def window_of(pid: int) -> int | None:
    found = []

    def one(hwnd, _):
        owner = wintypes.DWORD()
        USER32.GetWindowThreadProcessId(hwnd, ctypes.byref(owner))
        if owner.value == pid and USER32.IsWindowVisible(hwnd) and not USER32.GetWindow(hwnd, 4):
            found.append(hwnd)
        return True

    USER32.EnumWindows(ENUM(one), 0)
    return found[0] if found else None


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--build", type=Path, required=True)
    parser.add_argument("--host", type=Path, required=True)
    parser.add_argument("--core", type=Path)
    parser.add_argument("--out-root", type=Path, required=True)
    parser.add_argument("--seconds", type=float, default=120.0)
    parser.add_argument("--pace", type=float, default=0.5)
    parser.add_argument("--plan", default="old-1,new-1,old-2,new-2")
    parser.add_argument("--puzzle", choices=launch.PUZZLES, default=launch.PUZZLES[0])
    args = parser.parse_args(argv)
    product = build_product.verify(args.build)
    keys = ["KeyJ", "KeyK", "KeyL"]
    for name in args.plan.split(","):
        out = args.out_root / f"run-{name}"
        out.mkdir(parents=True)
        shutil.copytree(args.build / build_product.fixture_session(args.puzzle), out / launch.SESSION)
        run_id = dt.datetime.now(dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
        arguments = launch.command(product, args.build, out, None)
        env = launch.environment(product, args.build, out, run_id, False, puzzle=args.puzzle)
        new = name.startswith("new")
        if new:
            env["M600_HOST_DLL"] = str(args.host)
            env["M600_BUILD_IDENTITY"] = product["identity"] + "+host-key-stall"
            if args.core:
                env["M600_CORE_DLL"] = str(args.core)
                env["M600_BUILD_IDENTITY"] += "+core"
        started = dt.datetime.now(dt.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
        idle = idle_seconds()
        print(name, "start", started, f"owner idle {idle:.0f} s", flush=True)
        process = subprocess.Popen(arguments, env=env, cwd=ROOT)
        hwnd, deadline = None, time.monotonic() + 60
        while hwnd is None and time.monotonic() < deadline and process.poll() is None:
            time.sleep(0.2)
            hwnd = window_of(process.pid)
        posts = []
        if hwnd is not None:
            USER32.SetForegroundWindow(hwnd)
            time.sleep(10)
            end, index = time.monotonic() + args.seconds, 0
            while time.monotonic() < end and process.poll() is None:
                vk, scan = lx4.TURN_KEYS[keys[index % len(keys)]]
                down, up = lx4.key_lparams(scan)
                at = qpc()
                ok = bool(USER32.PostMessageW(hwnd, lx4.WM_KEYDOWN, vk, down))
                ok = bool(USER32.PostMessageW(hwnd, lx4.WM_KEYUP, vk, up)) and ok
                posts.append({"key": keys[index % len(keys)], "qpc": at, "ok": ok,
                              "foreground": (USER32.GetForegroundWindow() or 0) == hwnd})
                index += 1
                time.sleep(args.pace)
            time.sleep(2)
            USER32.PostMessageW(hwnd, WM_CLOSE, 0, 0)
        else:
            print(name, "no window", flush=True)
            process.kill()
        try:
            code = process.wait(timeout=60)
        except subprocess.TimeoutExpired:
            process.kill()
            code = process.wait(timeout=30)
        record = {"format": "magic600-key-stall-20261126-ab-launch-v1", "run_id": run_id, "started_utc": started,
                  "command": [Path(arguments[0]).name] + arguments[1:], "lx4": False, "editor": False,
                  "lx2_injection": None, "host": "new" if new else "build",
                  "core": "new" if new and args.core else "build", "puzzle": args.puzzle, "exit_code": code}
        (out / "launch.json").write_text(json.dumps(record, indent=2) + "\n", encoding="utf-8")
        (out / "posts.json").write_text(json.dumps({"owner_idle_s_at_start": round(idle, 1), "pace_s": args.pace,
                                                    "posts": posts}, indent=1) + "\n", encoding="utf-8")
        foreground = sum(post["foreground"] for post in posts)
        print(name, "exit", code, f"{len(posts)} keys posted, {foreground} with the foreground", flush=True)
        time.sleep(10)
    return 0


if __name__ == "__main__":
    sys.exit(main())
