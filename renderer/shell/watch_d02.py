"""The owner's one command to watch ADR-D02's build in the product shell.

  python renderer/shell/watch_d02.py [--state ID] [--build DIR] [--follow]

Run it from a PowerShell on the desktop. It starts the newest product build (or --build) with the frame, Jumble on and
D02 on, in a fresh run folder under work/d02-watch/. The window never sits on top and never takes the foreground (the
owner, 26 November 2026): it opens behind the window you are using, and you switch to it when you want to look. Close
it as any window; the command ends with it.

--follow keeps one window across build steps. Every few seconds it checks:
- a data-only change (tools/looklab/data/samples or tools/looklab/data/design): the running shell reloads the data in
  place;
- a newer product build: it starts without activation at the old window's place and size, and the old window closes
  once the new one is up.
Ctrl+C ends --follow and closes the window. Closing the window ends it too.
"""
from __future__ import annotations

import sys

sys.dont_write_bytecode = True

import argparse
import datetime as dt
import hashlib
import json
import subprocess
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import build_product  # noqa: E402

ROOT = HERE.parents[1]
DATA = ROOT / "tools" / "looklab" / "data"
WATCH_ROOT = ROOT / "work" / "d02-watch"
STATES = DATA / "samples" / "jumble" / "states.json"
POLL_S = 3.0
UP_TIMEOUT_S = 90.0
CLOSE_TIMEOUT_S = 30.0


def run_folder(now: dt.datetime | None = None) -> Path:
    stamp = (now or dt.datetime.now(dt.timezone.utc)).strftime("%Y%m%dT%H%M%S%fZ")
    return WATCH_ROOT / stamp


def launch_arguments(build: Path, out: Path, state: str | None, place: dict | None = None) -> list[str]:
    """launch.py's arguments for a watched start: the frame, Jumble on, D02 on with a data-only reload, and the old
    window's place and size on a restart. Nothing asks for the foreground or for a topmost window."""
    arguments = [sys.executable, "-B", str(HERE / "launch.py"), "--out", str(out), "--build", str(build), "--frame",
                 "--puzzle", "jumble-on", "--d02", "--d02-data", str(DATA)]
    if state:
        arguments += ["--d02-state", state]
    if place:
        x, y = place["position"]
        width, height = place["size"]
        arguments += ["--position", f"{x},{y}", "--resolution", f"{width}x{height}"]
    return arguments


def data_digest(folder: Path = DATA) -> str:
    """A digest of the names, sizes and times of the sample and design files: it changes with any data-only edit."""
    digest = hashlib.sha256()
    for part in ("samples", "design"):
        for path in sorted((folder / part).rglob("*")):
            if path.is_file():
                info = path.stat()
                digest.update(f"{path.relative_to(folder).as_posix()}|{info.st_size}|{info.st_mtime_ns}\n".encode())
    return digest.hexdigest()


def place_of(out: Path) -> dict | None:
    """The window's place and size that the running shell wrote, or None before it wrote one."""
    try:
        return json.loads((out / "d02-window.json").read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return None


def start(build: Path, out: Path, state: str | None, place: dict | None = None) -> subprocess.Popen:
    out.parent.mkdir(parents=True, exist_ok=True)
    return subprocess.Popen(launch_arguments(build, out, state, place), cwd=str(ROOT))


def close(process: subprocess.Popen, out: Path) -> None:
    """Asks the shell to quit (d02-quit in its run folder), then waits; stops the launcher's tree only if it hangs."""
    (out / "d02-quit").write_text("quit\n", encoding="utf-8")
    try:
        process.wait(CLOSE_TIMEOUT_S)
    except subprocess.TimeoutExpired:
        subprocess.run(["taskkill", "/T", "/F", "/PID", str(process.pid)], capture_output=True)


def follow(build: Path, state: str | None, newest=build_product.newest, sleep=time.sleep, out_stream=print) -> int:
    out = run_folder()
    process = start(build, out, state)
    out_stream(f"watch: build {build.name}, run folder {out.name}; Ctrl+C ends")
    seen, reloads = data_digest(), 0
    try:
        while True:
            sleep(POLL_S)
            if process.poll() is not None:
                out_stream("watch: the window was closed")
                return 0
            latest = newest()
            if latest is not None and latest != build and (latest / build_product.RECORD).is_file():
                place = place_of(out)
                new_out = run_folder()
                new_process = start(latest, new_out, state, place)
                waited = 0.0
                while place_of(new_out) is None and new_process.poll() is None and waited < UP_TIMEOUT_S:
                    sleep(1.0)
                    waited += 1.0
                if place_of(new_out) is None:
                    out_stream(f"watch: build {latest.name} did not come up; the old window stays")
                    if new_process.poll() is None:
                        close(new_process, new_out)
                    build = latest
                    continue
                close(process, out)
                process, out, build = new_process, new_out, latest
                seen = data_digest()
                out_stream(f"watch: now on build {build.name}")
                continue
            digest = data_digest()
            if digest != seen:
                seen, reloads = digest, reloads + 1
                (out / "d02-reload").write_text(f"{reloads}\n", encoding="utf-8")
                out_stream(f"watch: data changed; reload {reloads} asked")
    except KeyboardInterrupt:
        close(process, out)
        return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--state", help="the sample state the D02 surfaces read (tools/looklab/data/samples/jumble)")
    parser.add_argument("--build", type=Path, help="the product build (default: the newest)")
    parser.add_argument("--follow", action="store_true", help="keep one window across build steps")
    args = parser.parse_args(argv)
    if args.state is not None:
        states = [state["state"] for state in json.loads(STATES.read_text(encoding="utf-8"))["states"]]
        if args.state not in states:
            print(f"watch: no sample state {args.state}; one of {', '.join(states)}")
            return 2
    build = args.build.resolve() if args.build else build_product.newest()
    if build is None:
        print(f"watch: no product build under {build_product.BUILD_ROOT}; run renderer/shell/build_product.py")
        return 2
    if args.follow:
        return follow(build, args.state)
    out = run_folder()
    process = start(build, out, args.state)
    print(f"watch: build {build.name}, run folder {out.name}; the window opens behind the one you use")
    try:
        return process.wait()
    except KeyboardInterrupt:
        close(process, out)
        return 0


if __name__ == "__main__":
    sys.exit(main())
