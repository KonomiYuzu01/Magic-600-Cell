"""Shell-run watcher for the unattended C9 runs (unattended.ps1). Standard library only; the live mode is Windows only.

    python -B tools/perf/c9/foreground.py --window-of EXE [--hold] [--wait S]

Started before the shell, it takes the session's last-input time as its baseline and checks once a second:

- owner input: the session's last input (GetLastInputInfo) is newer than the baseline. Posted window messages do not
  count as input, so the posters never trigger it. Exit 10, `owner-input`.
- the shell's window: the first visible, unowned top-level window of a process whose image is EXE. Without one within
  `--wait` seconds (default 180) it exits 12, `no-window`, and the runner lets the run go on.
- with `--hold` (U3, U7, U8): once the window is there, it is made topmost (SWP_NOACTIVATE) and the foreground is asked
  for once with SetForegroundWindow, as the acceptance harness does (Level2Windows.cs: "Exactly one request; no
  foreground-lock workaround"): no AttachThreadInput, no synthesized input, no change to ForegroundLockTimeout. The
  quiet-machine gate has the owner idle for 300 s, longer than the 200 s foreground lock, so the request is allowed.
  When the shell does not have the foreground right after it, exit 11, `not-taken`; when another process holds the
  foreground at two checks in a row afterwards, exit 11, `foreground-lost`.
- the window closed or hidden: exit 0, `window-closed`.

Each event is one JSON line on stdout (`start`, `window`, `take`, `end`). The runner stops the run on exit 10 or 11
while the shell is measuring, sets its folder aside and runs it again (at most twice). The tests use a fake system;
none of them looks for a window or reads the input time.
"""
from __future__ import annotations

import argparse
import json
import sys
import time

OWNER_INPUT, FOREGROUND, NO_WINDOW = 10, 11, 12
WAIT_S = 180
POLL_S = 1.0
LOST_AFTER = 2  # consecutive checks with another process in the foreground


def newer(tick, baseline):
    """True when a 32-bit tick count is later than the baseline (GetTickCount wraps after 49.7 days)."""
    return 0 < (tick - baseline) % (1 << 32) < (1 << 31)


def watch(system, exe, hold, wait_s=WAIT_S, clock=time.monotonic, sleep=time.sleep, out=print, poll=POLL_S):
    def event(row):
        out(json.dumps(row, sort_keys=True))

    baseline = system.last_input()
    started = clock()
    event({'t': 'start', 'window_of': exe, 'hold': hold, 'input_tick': baseline})
    window = pid = None
    others = 0
    while True:
        tick = system.last_input()
        if newer(tick, baseline):
            event({'t': 'end', 'reason': 'owner-input', 'input_tick': tick, 'seconds': round(clock() - started, 1)})
            return OWNER_INPUT
        if window is None:
            found = system.find_window(exe)
            if found is not None:
                window, pid = found
                if hold:
                    system.topmost(window)
                    system.set_foreground(window)  # Exactly one request; no foreground-lock workaround.
                    taken = system.foreground_pid() == pid
                    event({'t': 'take', 'pid': pid, 'foreground': taken, 'topmost': system.is_topmost(window)})
                    if not taken:
                        event({'t': 'end', 'reason': 'not-taken'})
                        return FOREGROUND
                else:
                    event({'t': 'window', 'pid': pid})
            elif clock() - started >= wait_s:
                event({'t': 'end', 'reason': 'no-window'})
                return NO_WINDOW
        elif not system.visible(window):
            event({'t': 'end', 'reason': 'window-closed', 'seconds': round(clock() - started, 1)})
            return 0
        elif hold:
            holder = system.foreground_pid()
            others = others + 1 if holder != pid else 0
            if others >= LOST_AFTER:
                event({'t': 'end', 'reason': 'foreground-lost', 'holder': holder,
                       'seconds': round(clock() - started, 1)})
                return FOREGROUND
        sleep(poll)


class Windows:
    """The user32 and kernel32 calls the watcher makes. Created only by a live run."""

    def __init__(self):
        import ctypes
        from ctypes import wintypes
        self.ctypes, self.wintypes = ctypes, wintypes
        self.user32, self.kernel32 = ctypes.WinDLL('user32'), ctypes.WinDLL('kernel32')

        class LastInput(ctypes.Structure):
            _fields_ = (('cbSize', wintypes.UINT), ('dwTime', wintypes.DWORD))

        self.LastInput = LastInput
        self.callback = ctypes.WINFUNCTYPE(wintypes.BOOL, wintypes.HWND, wintypes.LPARAM)
        user32, kernel32 = self.user32, self.kernel32
        user32.GetLastInputInfo.argtypes = (ctypes.POINTER(LastInput),)
        user32.GetLastInputInfo.restype = wintypes.BOOL
        user32.EnumWindows.argtypes = (self.callback, wintypes.LPARAM)
        user32.EnumWindows.restype = wintypes.BOOL
        user32.IsWindowVisible.argtypes = (wintypes.HWND,)
        user32.IsWindowVisible.restype = wintypes.BOOL
        user32.IsWindow.argtypes = (wintypes.HWND,)
        user32.IsWindow.restype = wintypes.BOOL
        user32.GetWindow.argtypes = (wintypes.HWND, wintypes.UINT)
        user32.GetWindow.restype = wintypes.HWND
        user32.GetWindowThreadProcessId.argtypes = (wintypes.HWND, ctypes.POINTER(wintypes.DWORD))
        user32.GetWindowThreadProcessId.restype = wintypes.DWORD
        user32.GetForegroundWindow.restype = wintypes.HWND
        user32.SetForegroundWindow.argtypes = (wintypes.HWND,)
        user32.SetForegroundWindow.restype = wintypes.BOOL
        user32.SetWindowPos.argtypes = (wintypes.HWND, wintypes.HWND, ctypes.c_int, ctypes.c_int, ctypes.c_int,
                                        ctypes.c_int, wintypes.UINT)
        user32.SetWindowPos.restype = wintypes.BOOL
        user32.GetWindowLongPtrW.argtypes = (wintypes.HWND, ctypes.c_int)
        user32.GetWindowLongPtrW.restype = ctypes.c_ssize_t
        kernel32.OpenProcess.argtypes = (wintypes.DWORD, wintypes.BOOL, wintypes.DWORD)
        kernel32.OpenProcess.restype = wintypes.HANDLE
        kernel32.QueryFullProcessImageNameW.argtypes = (wintypes.HANDLE, wintypes.DWORD, wintypes.LPWSTR,
                                                        ctypes.POINTER(wintypes.DWORD))
        kernel32.QueryFullProcessImageNameW.restype = wintypes.BOOL
        kernel32.CloseHandle.argtypes = (wintypes.HANDLE,)

    def last_input(self):
        info = self.LastInput(self.ctypes.sizeof(self.LastInput), 0)
        if not self.user32.GetLastInputInfo(self.ctypes.byref(info)):
            raise OSError('GetLastInputInfo failed')
        return info.dwTime

    def pid(self, window):
        value = self.wintypes.DWORD()
        self.user32.GetWindowThreadProcessId(window, self.ctypes.byref(value))
        return value.value

    def image(self, pid):
        handle = self.kernel32.OpenProcess(0x1000, False, pid)  # PROCESS_QUERY_LIMITED_INFORMATION
        if not handle:
            return None
        try:
            size = self.wintypes.DWORD(1024)
            buffer = self.ctypes.create_unicode_buffer(size.value)
            ok = self.kernel32.QueryFullProcessImageNameW(handle, 0, buffer, self.ctypes.byref(size))
            return buffer.value.replace('/', '\\').rsplit('\\', 1)[-1] if ok else None
        finally:
            self.kernel32.CloseHandle(handle)

    def find_window(self, exe):
        found = []

        def visit(window, _):
            if self.user32.IsWindowVisible(window) and not self.user32.GetWindow(window, 4):  # GW_OWNER
                pid = self.pid(window)
                if (self.image(pid) or '').lower() == exe.lower():
                    found.append((window, pid))
                    return False
            return True

        self.user32.EnumWindows(self.callback(visit), 0)
        return found[0] if found else None

    def topmost(self, window):
        # HWND_TOPMOST; SWP_NOSIZE | SWP_NOMOVE | SWP_NOACTIVATE: the foreground is asked for once, apart.
        self.user32.SetWindowPos(window, self.wintypes.HWND(-1), 0, 0, 0, 0, 0x0001 | 0x0002 | 0x0010)

    def is_topmost(self, window):
        return bool(self.user32.GetWindowLongPtrW(window, -20) & 0x8)  # GWL_EXSTYLE, WS_EX_TOPMOST

    def set_foreground(self, window):
        return bool(self.user32.SetForegroundWindow(window))

    def foreground_pid(self):
        window = self.user32.GetForegroundWindow()
        return self.pid(window) if window else None

    def visible(self, window):
        return bool(self.user32.IsWindow(window) and self.user32.IsWindowVisible(window))


def main(argv=None, system_factory=Windows):
    parser = argparse.ArgumentParser(description='Watch a shell run: owner input, the window, the foreground.')
    parser.add_argument('--window-of', required=True, metavar='EXE', help="the shell's executable name")
    parser.add_argument('--hold', action='store_true', help='make the window topmost, ask once for the foreground '
                        'and report when it is lost')
    parser.add_argument('--wait', type=float, default=WAIT_S, help='seconds to wait for the window')
    args = parser.parse_args(sys.argv[1:] if argv is None else argv)
    if sys.platform != 'win32':
        print('foreground: Windows only', file=sys.stderr)
        return 2
    return watch(system_factory(), args.window_of, args.hold, args.wait)


if __name__ == '__main__':
    sys.exit(main())
