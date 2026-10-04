"""Prepare an offline SA2 level 2 build; never launch Godot or an editor."""

import argparse
import datetime as dt
import json
import os
import sys
from pathlib import Path

sys.dont_write_bytecode = True
import run_smoke as smoke

HERE = Path(__file__).resolve().parent
GODOT_EXE = "Godot_v4.7.2-stable_mono_win64.exe"
GODOT_CONSOLE = "Godot_v4.7.2-stable_mono_win64_console.exe"
ENVIRONMENT_KEYS = ("M600_GODOT_NUPKGS", "NUGET_PACKAGES", "DOTNET_CLI_HOME", "NUGET_HTTP_CACHE_PATH",
                    "DOTNET_CLI_TELEMETRY_OPTOUT", "DOTNET_NOLOGO", "DOTNET_SKIP_FIRST_TIME_EXPERIENCE",
                    "DOTNET_CLI_WORKLOAD_UPDATE_NOTIFY_DISABLE", "DOTNET_ADD_GLOBAL_TOOLS_TO_PATH")


def locate_without_launch(explicit=None):
    # run_smoke.locate_godot also executes --version. Reuse its installed-package
    # location, but leave the exact runtime version check to Level2._Ready.
    if explicit:
        path = Path(explicit).resolve()
    else:
        base = os.environ.get("LOCALAPPDATA")
        if not base:
            raise RuntimeError("LOCALAPPDATA is unavailable; pass --godot")
        matches = list((Path(base) / "Microsoft/WinGet/Packages").glob(
            "GodotEngine.GodotEngine.Mono_*/Godot_v4.7.2-stable_mono_win64/" + GODOT_CONSOLE))
        if len(matches) != 1:
            raise RuntimeError("expected exactly one Godot installation; pass --godot")
        path = matches[0].resolve()
    if path.name not in (GODOT_EXE, GODOT_CONSOLE):
        raise RuntimeError("expected the pinned Godot 4.7.2 .NET executable")
    executable = path.with_name(GODOT_EXE)
    nupkgs = path.parent / "GodotSharp/Tools/nupkgs"
    if not executable.is_file() or not nupkgs.is_dir():
        raise RuntimeError("the non-console Godot executable or offline NuGet folder is missing")
    return executable, nupkgs


def launch_value(identity, executable, environment):
    project, native = Path(identity["project"]).resolve(), Path(identity["native"]).resolve()
    if Path(executable).name != GODOT_EXE:
        raise ValueError("level 2 must launch the process that presents, not the console wrapper")
    return {"format": "magic600-l2-launch-v1", "candidate": "sa2", "executable": str(Path(executable).resolve()),
            "dll": str(native / "sa2_interop.dll"), "working_directory": str(project),
            "arguments": ["--path", str(project), "--rendering-driver", "d3d12", "--disable-vsync", "--render-thread", "safe"],
            "run_arguments": ["--log-file", "{out}\\godot.log"], "validation_arguments": ["--gpu-validation"],
            "separator": ["--"], "environment": dict(environment), "validation_environment": {}}


def write_launch(build, identity, executable, environment):
    value = launch_value(identity, executable, environment)
    destination = Path(build) / "launch.json"
    if destination.exists():
        if json.loads(destination.read_text(encoding="utf-8")) != value:
            raise RuntimeError("prepared launch settings changed; rebuild")
        return destination
    temporary = destination.with_name("launch.json.tmp")
    created = False
    try:
        with temporary.open("x", encoding="utf-8", newline="\n") as stream:
            created = True
            stream.write(json.dumps(value, indent=2) + "\n")
            stream.flush()
            os.fsync(stream.fileno())
        # Windows rename refuses an existing destination.
        if destination.exists():
            raise FileExistsError(destination)
        temporary.rename(destination)
    finally:
        if created and temporary.exists():
            temporary.unlink()
    return destination


def part_digests(identity, executable):
    native, project = Path(identity["native"]), Path(identity["project"])
    paths = {"godot:exe": Path(executable), "godot:assembly": project / ".godot/mono/temp/bin/Debug/SA2Smoke.dll"}
    paths.update({"native:" + p.name: p for p in native.iterdir() if p.is_file() and p.suffix.lower() in (".dll", ".dxil", ".exe")})
    paths.update({"project:" + p.relative_to(project).as_posix(): p for p in project.rglob("*")
                  if p.is_file() and not any(part in smoke.EXCLUDED for part in p.relative_to(project).parts[:-1])})
    return {name: {"path": str(path.resolve()), "sha256": smoke.sha256(path)} for name, path in sorted(paths.items())}


def reuse_prepared(source, executable):
    paths = sorted(smoke.BUILD_ROOT.glob("*/l2-build-identity.json"), reverse=True)
    if not paths:
        raise RuntimeError("no prepared level 2 build exists for --skip-build")
    path = paths[0]
    value = json.loads(path.read_text(encoding="utf-8"))
    identity = value["build"]
    build = path.parent.resolve()
    if identity["source"] != source or identity["godot_version"] != smoke.GODOT_VERSION:
        raise RuntimeError("newest prepared build does not match source/Godot; rebuild")
    if Path(identity["native"]).resolve() != build / "native" or Path(identity["project"]).resolve() != build / "project":
        raise RuntimeError("prepared build paths do not match its directory")
    if part_digests(identity, executable) != value["parts"]:
        raise RuntimeError("prepared source/binary digests changed; rebuild")
    return build, identity, value["environment"]


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--godot", type=Path, help="the pinned non-console or console exe; neither is started")
    parser.add_argument("--skip-build", action="store_true")
    args = parser.parse_args(argv)
    if os.name != "nt":
        parser.exit(1, "SA2 preparation requires the owner's Windows build tools\n")
    try:
        executable, nupkgs = locate_without_launch(args.godot)
        source = smoke.source_identity()
        if args.skip_build:
            build, identity, environment = reuse_prepared(source, executable)
        else:
            stamp = dt.datetime.now(dt.timezone.utc).strftime("%Y%m%dT%H%M%S%fZ")
            # The level 1 build recipe enforces the same 150-character native path
            # limit and isolates .NET/NuGet. It starts no framework or GPU process.
            private = smoke.PRIVATE_ROOT / stamp
            if len(str(smoke.BUILD_ROOT / stamp / "native")) > smoke.BUILD_PATH_LIMIT:
                raise RuntimeError("checkout path too long for the native build; use a shorter checkout path")
            private.mkdir(parents=True)
            identity = smoke.build(private, nupkgs, source, smoke.GODOT_VERSION)
            build = Path(identity["native"]).parent
            isolated = smoke.offline_environment(private, nupkgs)
            environment = {key: isolated[key] for key in ENVIRONMENT_KEYS}
            environment[smoke.INJECT] = None
            smoke.write_json(build / "l2-build-identity.json", {
                "build": identity, "parts": part_digests(identity, executable), "environment": environment})
        write_launch(build, identity, executable, environment)
        print(build)
        return 0
    except (OSError, ValueError, KeyError, RuntimeError) as error:
        print(f"SA2 level 2 preparation failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
