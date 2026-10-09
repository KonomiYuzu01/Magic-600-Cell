"""Prepare the offline Qt level 2 build; never start the app or a capture."""
import sys
sys.dont_write_bytecode = True

import argparse
from datetime import datetime, timezone
import os
from pathlib import Path
import subprocess

import run_smoke


def launch_record(build):
    build = Path(build).resolve()
    env = run_smoke.clean_environment()
    # Keep level 1's startup diagnostics, but remove inherited per-frame logging switches.
    removed = ("QSG_RHI_DEBUG_LAYER", "M600_SA2_INJECT_UNCONFIRMED_DRAIN", "QT_PLUGIN_PATH",
               "QT_QPA_PLATFORM_PLUGIN_PATH", "QML_IMPORT_PATH", "QML2_IMPORT_PATH", "QSG_RHI_BACKEND",
               "QSG_INFO", "QSG_RENDER_TIMING", "QSG_RHI_PROFILE", "QSG_VISUALIZE", "QSG_RENDERER_DEBUG", "QT_DEBUG_PLUGINS")
    environment = {name: None for name in removed}
    environment.update(PATH=str(build / "deploy") + os.pathsep + env.get("PATH", ""),
                       QT_ENABLE_HIGHDPI_SCALING=env["QT_ENABLE_HIGHDPI_SCALING"],
                       QT_LOGGING_RULES=env["QT_LOGGING_RULES"], QT_FORCE_STDERR_LOGGING=env["QT_FORCE_STDERR_LOGGING"],
                       QSG_RENDER_LOOP="threaded")
    return dict(format="magic600-l2-launch-v1", candidate="sd", executable=str(build / "deploy/sd_smoke.exe"),
                dll=str(build / "native/sa2_interop.dll"), working_directory=str(build / "deploy"),
                arguments=[], run_arguments=[], validation_arguments=[], separator=[],
                environment=environment, validation_environment={})


def write_launch(build):
    record = launch_record(build)
    run_smoke.atomic_json(Path(build) / "launch.json", record)
    return record


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--skip-build", action="store_true", help="reuse only the newest manifest-matching build")
    args = parser.parse_args(argv)
    if os.name != "nt":
        raise ValueError("preparation requires the owner's Windows build tools")
    env = run_smoke.clean_environment()
    env["PATH"] = str(run_smoke.QT / "bin") + os.pathsep + env.get("PATH", "")
    version = subprocess.check_output([str(run_smoke.QT / "bin/qmake.exe"), "-query", "QT_VERSION"],
                                      env=env, timeout=30, text=True).strip()
    if version != "6.10.3":
        raise ValueError("Qt version must be exactly 6.10.3")
    source = run_smoke.source_identity()
    if args.skip_build:
        build, _ = run_smoke.reuse_build(source, version)
    else:
        stamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%S%fZ")
        build = run_smoke.ROOT / "work/sdb" / stamp
        if len(str(build / "native")) > 150:
            raise ValueError("native build path exceeds 150 characters; run from a shorter checkout")
        # Same build/deploy commands and artifact digest manifest as level 1; logs stay in the build.
        run_smoke.build_all(build, build, env, source)
    write_launch(build)
    print(build)
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        print(f"sd l2 prepare: {error}", file=sys.stderr)
        sys.exit(1)
