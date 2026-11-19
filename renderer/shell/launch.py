"""Start C9's product shell from a product build (route B): the one way the owner-machine steps start it.

  python -B renderer/shell/launch.py --out NEW_FOLDER [--build DIR] [--run-id ID] [--echo] [--resolution WxH]
                                     [--puzzle jumble-off|jumble-on] [--lx4] [--dry-run]

The launcher recomputes every part of the product build (`build_product.verify`; default: the newest build under
work/build/c9-product), creates the run folder `--out`, which must not exist yet, and starts the pinned Godot (the
process that presents, not the console wrapper) on the build's shell with the environment the shell reads:
`M600_CORE_DLL`, `M600_HOST_DLL`, `M600_ASSETS` (the build's copy of the model), `M600_EVENTS_DIR` (the run folder),
`M600_BUILD_IDENTITY` (the build's identity), `M600_RUN_ID`, and for charter E2's turns `M600_SESSION_DIR` (a new
session folder, `session` in the run folder) and `M600_TWIST_MENU` (the build's copy of the twist menu); `--echo`
adds `M600_INPUT_ECHO=1`, the input echo of route B's check B1. `M600_PUZZLE` names the puzzle the shell runs and its
run row records (`--puzzle`, default jumble-off). The shell's host opens only Jumble off for now, so the launcher
refuses jumble-on before it starts anything. `--lx4` adds `M600_LX4=1`, which makes the shell create LX-4's module
(C9 U8). It waits for the shell to exit and writes
`launch.json` (`magic600-c9-launch-v1`) beside the shell's logs, with the build's own folder and the run folder
written as `<build>` and `<out>`.

`--dry-run` checks the build and the run folder and prints the command and the environment; it creates nothing and
starts nothing. Start the launcher from a PowerShell on the desktop: a window started from a background process never
gets the foreground. Exit 0 when the shell exited 0, 1 when it exited otherwise, 2 for a usage or build error.
"""
from __future__ import annotations

import sys

sys.dont_write_bytecode = True

import argparse
import datetime as dt
import json
import os
import subprocess
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import build_product  # noqa: E402

FORMAT = "magic600-c9-launch-v1"
RECORD = "launch.json"
SHELL_VARIABLES = ("M600_CORE_DLL", "M600_HOST_DLL", "M600_ASSETS", "M600_EVENTS_DIR", "M600_BUILD_IDENTITY",
                   "M600_RUN_ID", "M600_INPUT_ECHO", "M600_SESSION_DIR", "M600_TWIST_MENU", "M600_PUZZLE", "M600_LX4")
SESSION = "session"
# The puzzles the shell's run row may name (EventLog.Puzzles) and the ones its host runs now.
PUZZLES = ("jumble-off", "jumble-on")
RUNNABLE_PUZZLES = ("jumble-off",)


class LaunchError(Exception):
    """A launch that must not start."""


def run_id_now() -> str:
    return dt.datetime.now(dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")


def command(product: dict, build: Path, out: Path, resolution: str | None = None) -> list[str]:
    arguments = [product["godot"]["executable"], "--path", str(build / build_product.PROJECT),
                 "--rendering-driver", "d3d12", "--render-thread", "safe", "--windowed",
                 "--log-file", str(out / "godot.log")]
    if resolution:
        arguments += ["--resolution", resolution]
    return arguments


def environment(product: dict, build: Path, out: Path, run_id: str, echo: bool, base: dict | None = None,
                puzzle: str = PUZZLES[0], lx4: bool = False) -> dict:
    """The process environment: `base` (default: this process's) without any shell variable, then the launch's."""
    env = {key: value for key, value in (os.environ if base is None else base).items()
           if key.upper() not in SHELL_VARIABLES}
    env.update({"M600_CORE_DLL": str(build / build_product.CORE_DLL), "M600_HOST_DLL": str(build / build_product.HOST_DLL),
                "M600_ASSETS": str(build / build_product.ASSETS), "M600_EVENTS_DIR": str(out),
                "M600_BUILD_IDENTITY": product["identity"], "M600_RUN_ID": run_id,
                "M600_SESSION_DIR": str(out / SESSION), "M600_TWIST_MENU": str(build / build_product.MENU),
                "M600_PUZZLE": puzzle})
    if echo:
        env["M600_INPUT_ECHO"] = "1"
    if lx4:
        env["M600_LX4"] = "1"
    return env


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


def main(argv: list[str] | None = None, popen=subprocess.Popen, out_stream=print) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--out", type=Path, required=True, help="the run folder, which must not exist yet")
    parser.add_argument("--build", type=Path, help="the product build (default: the newest under work/build/c9-product)")
    parser.add_argument("--run-id", help="the run's id (default: the UTC start time)")
    parser.add_argument("--echo", action="store_true", help="show and log the input echo (route B check B1)")
    parser.add_argument("--resolution", help="the window's size as WxH (default: the project's)")
    parser.add_argument("--puzzle", choices=PUZZLES, default=PUZZLES[0], help="the puzzle the shell runs")
    parser.add_argument("--lx4", action="store_true", help="create LX-4's module in the shell (C9 U8)")
    parser.add_argument("--dry-run", action="store_true", help="check and print; create and start nothing")
    args = parser.parse_args(argv)
    try:
        build = args.build.resolve() if args.build else build_product.newest()
        if build is None:
            raise LaunchError(f"no product build under {build_product.BUILD_ROOT}; run build_product.py")
        product = build_product.verify(build)
        if args.resolution and not all(part.isdigit() for part in args.resolution.split("x", 1)):
            raise LaunchError("--resolution must be WxH")
        if args.puzzle not in RUNNABLE_PUZZLES:
            raise LaunchError(f"the shell's host does not run {args.puzzle} yet")
        out = check_out(args.out)
        run_id = args.run_id or run_id_now()
        arguments = command(product, build, out, args.resolution)
        env = environment(product, build, out, run_id, args.echo, puzzle=args.puzzle, lx4=args.lx4)
    except (LaunchError, build_product.ProductError, OSError) as error:
        out_stream(f"launch: {error}")
        return 2
    if args.dry_run:
        out_stream(f"launch: ready: build {product['identity']} ({build.name}), run {run_id}")
        out_stream("command: " + subprocess.list2cmdline(arguments))
        for key in SHELL_VARIABLES:
            if key in env:
                out_stream(f"env: {key}={env[key]}")
        return 0
    out.mkdir(parents=True)
    started, clock = dt.datetime.now(dt.timezone.utc), time.monotonic()
    process = popen(arguments, cwd=str(build / build_product.PROJECT), env=env)
    code = process.wait()
    seconds = round(time.monotonic() - clock, 3)
    with (out / RECORD).open("x", encoding="utf-8", newline="\n") as stream:
        stream.write(json.dumps({"format": FORMAT, "run_id": run_id, "build_identity": product["identity"],
                                 "echo": args.echo, "puzzle": args.puzzle, "lx4": args.lx4,
                                 "command": sanitized(arguments, build, out),
                                 "started_utc": started.strftime("%Y-%m-%dT%H:%M:%SZ"), "seconds": seconds,
                                 "exit_code": code}, indent=2) + "\n")
    out_stream(f"launch: the shell exited {code} after {seconds} s; logs in {out}")
    return 0 if code == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
