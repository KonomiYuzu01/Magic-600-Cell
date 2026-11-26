"""Interleaved short runs of one product build with its own core (`ctl-N`) and with another core DLL (`fix-N`).

    python -B work/experiments/presented-gap-20261126/ab_core.py --build DIR --core DLL --out-root DIR
                                                                 [--seconds 60] [--plan ctl-1,fix-1,ctl-2,fix-2]

Each run starts the build's shell with the command and environment that `renderer/shell/launch.py` gives it (after
`build_product.verify`, a fresh copy of the Jumble-off session fixture, no LX-4, LX-2 or editor), with only
`M600_CORE_DLL` replaced in a `fix` run (and `+core-fix` appended to `M600_BUILD_IDENTITY`, so its run row says so).
It lets the shell run for `--seconds`, closes its window with WM_CLOSE, and writes a `launch.json` that
`presented_share.py` (owner-sitting-20261125) and `race.py` read. The core DLL loads the shaders beside it, so `--core`
names a `renderer/core/build.cmd` output folder's `sa2_interop.dll`. Keep `--out-root` under the ignored `work/`.

An experiment, not a measurement of record: the window starts from this process, so it may not get the foreground,
and nothing here controls what else is on the desktop.
"""
from __future__ import annotations

import argparse
import datetime as dt
import json
import shutil
import subprocess
import sys
import time
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "renderer" / "shell"))
import build_product  # noqa: E402
import launch  # noqa: E402


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--build", type=Path, required=True)
    parser.add_argument("--core", type=Path, required=True)
    parser.add_argument("--out-root", type=Path, required=True)
    parser.add_argument("--seconds", type=float, default=60.0)
    parser.add_argument("--plan", default="ctl-1,fix-1,ctl-2,fix-2")
    args = parser.parse_args(argv)
    product = build_product.verify(args.build)
    for name in args.plan.split(","):
        out = args.out_root / f"run-{name}"
        out.mkdir(parents=True)
        shutil.copytree(args.build / build_product.fixture_session("jumble-off"), out / launch.SESSION)
        run_id = dt.datetime.now(dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
        arguments = launch.command(product, args.build, out, None)
        env = launch.environment(product, args.build, out, run_id, False)
        fix = name.startswith("fix")
        if fix:
            env["M600_CORE_DLL"] = str(args.core)
            env["M600_BUILD_IDENTITY"] = product["identity"] + "+core-fix"
        started = dt.datetime.now(dt.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
        print(name, "start", started, flush=True)
        process = subprocess.Popen(arguments, env=env, cwd=ROOT)
        time.sleep(args.seconds)
        subprocess.run(["powershell", "-NoProfile", "-Command",
                        f"(Get-Process -Id {process.pid}).CloseMainWindow() | Out-Null"], check=False)
        try:
            code = process.wait(timeout=60)
        except subprocess.TimeoutExpired:
            process.kill()
            code = process.wait(timeout=30)
        record = {"format": "magic600-presented-gap-20261126-ab-launch-v1", "run_id": run_id, "started_utc": started,
                  "command": [Path(arguments[0]).name] + arguments[1:], "lx4": False, "editor": False,
                  "lx2_injection": None, "core": "fix" if fix else "build", "exit_code": code}
        (out / "launch.json").write_text(json.dumps(record, indent=2) + "\n", encoding="utf-8")
        print(name, "exit", code, flush=True)
        time.sleep(10)
    return 0


if __name__ == "__main__":
    sys.exit(main())
