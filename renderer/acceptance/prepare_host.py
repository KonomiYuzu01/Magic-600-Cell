"""Prepare an offline build of the C9 acceptance host; never launch Godot, PresentMon or a GPU device.

The host is S-A2 as tested: the official Godot 4.7.2 .NET editor binary running renderer/acceptance/project
(net8.0) on the product core renderer/core (ABI 3). Derived from work/experiments/renderer-sa2/prepare_l2.py and
run_smoke.build (see PROVENANCE.md). Everything is written under work/build/c9-acceptance/<UTC>/:
native/ (the core), project/ (a copy of the Godot project, built offline), dotnet/ (private NuGet and
.NET folders), logs/, c9-build-identity.json and launch.json. The core's CPU self-test runs after the native
build; no GPU mode runs.

  python -B renderer/acceptance/prepare_host.py [--godot <exe>]
  python -B renderer/acceptance/prepare_host.py --verify <build directory> [--godot <exe>]
"""
from __future__ import annotations

import sys

sys.dont_write_bytecode = True

import argparse
import datetime as dt
import hashlib
import json
import os
import shutil
import subprocess
from pathlib import Path, PurePosixPath

ROOT = Path(__file__).resolve().parents[2]
HERE = ROOT / "renderer" / "acceptance"
CORE = ROOT / "renderer" / "core"
sys.path.insert(0, str(CORE))
import check_core  # noqa: E402  (the core's own CPU self-test acceptance)

BUILD_ROOT = ROOT / "work" / "build" / "c9-acceptance"
# cl.exe is not long-path aware (run_smoke.py); the native folder path stays within this limit.
BUILD_PATH_LIMIT = 150
GODOT_EXE = "Godot_v4.7.2-stable_mono_win64.exe"
GODOT_CONSOLE = "Godot_v4.7.2-stable_mono_win64_console.exe"
# Checked by Level2._Ready at run time; this script never starts Godot.
GODOT_VERSION = "4.7.2.stable.mono.official.ed1daf0bf"
PACKAGES = {"godot.net.sdk", "godot.sourcegenerators", "godotsharp", "godotsharpeditor"}
INJECT = "M600_SA2_INJECT_UNCONFIRMED_DRAIN"
ENVIRONMENT_KEYS = ("M600_GODOT_NUPKGS", "NUGET_PACKAGES", "DOTNET_CLI_HOME", "NUGET_HTTP_CACHE_PATH",
                    "DOTNET_CLI_TELEMETRY_OPTOUT", "DOTNET_NOLOGO", "DOTNET_SKIP_FIRST_TIME_EXPERIENCE",
                    "DOTNET_CLI_WORKLOAD_UPDATE_NOTIFY_DISABLE", "DOTNET_ADD_GLOBAL_TOOLS_TO_PATH")
# The build's sources: the Godot project and the whole product core. Generated folders are excluded.
SOURCES = ("renderer/acceptance/project", "renderer/core")
EXCLUDED = {".godot", "bin", "obj", "__pycache__"}
IDENTITY = "c9-build-identity.json"


def sha256(path):
    with Path(path).open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def git(*args):
    env = os.environ.copy()
    env["GIT_OPTIONAL_LOCKS"] = "0"
    completed = subprocess.run(["git", "--no-optional-locks", *args], cwd=ROOT, env=env,
                               capture_output=True, text=True, check=True, timeout=30)
    return completed.stdout.strip()


def source_files():
    files = []
    for source in SOURCES:
        base = ROOT / source
        files += [p for p in base.rglob("*")
                  if p.is_file() and not any(part in EXCLUDED for part in p.relative_to(base).parts[:-1])]
    return sorted(files, key=lambda p: p.relative_to(ROOT).as_posix())


def source_identity():
    files = source_files()
    digest = hashlib.sha256()
    for path in files:
        digest.update(path.relative_to(ROOT).as_posix().encode("utf-8") + b"\0")
        digest.update(sha256(path).encode("ascii") + b"\n")
    tracked = {name for name in git("ls-files", "-z", "--", *SOURCES).split("\0")
               if name and not any(part in EXCLUDED for part in PurePosixPath(name).parts[:-1])}
    walked = {p.relative_to(ROOT).as_posix() for p in files}
    env = os.environ.copy()
    env["GIT_OPTIONAL_LOCKS"] = "0"
    changed = subprocess.run(["git", "--no-optional-locks", "diff", "--quiet", "HEAD", "--", *SOURCES],
                             cwd=ROOT, env=env, timeout=30).returncode
    if changed not in (0, 1):
        raise RuntimeError("cannot compare source with HEAD")
    return {"head": git("rev-parse", "HEAD"), "sha256": digest.hexdigest(),
            "matches_head": tracked == walked and changed == 0, "file_count": len(files)}


def locate_without_launch(explicit=None):
    """The pinned Godot 4.7.2 .NET install (prepare_l2.py); its version is checked at run time, not here."""
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


def offline_environment(private, nupkgs):
    env = os.environ.copy()
    env.pop(INJECT, None)
    env.update({"M600_GODOT_NUPKGS": str(nupkgs), "NUGET_PACKAGES": str(private / "packages"),
                "DOTNET_CLI_HOME": str(private / "dotnet-home"), "NUGET_HTTP_CACHE_PATH": str(private / "nuget-http-cache"),
                "DOTNET_CLI_TELEMETRY_OPTOUT": "1", "DOTNET_NOLOGO": "1", "DOTNET_SKIP_FIRST_TIME_EXPERIENCE": "1",
                "DOTNET_CLI_WORKLOAD_UPDATE_NOTIFY_DISABLE": "1", "DOTNET_ADD_GLOBAL_TOOLS_TO_PATH": "0"})
    return env


def check_package_cache(cache):
    directories = {path.name.casefold(): path for path in Path(cache).iterdir() if path.is_dir()}
    if set(directories) != PACKAGES:
        raise RuntimeError("offline restore did not contain exactly the four Godot packages")
    for path in directories.values():
        if {p.name for p in path.iterdir() if p.is_dir()} != {"4.7.2"}:
            raise RuntimeError("unexpected Godot package version")


def run_logged(command, cwd, env, timeout, prefix):
    """Run one build step with its output in logs/; a timeout kills the process tree."""
    with Path(f"{prefix}.stdout.txt").open("wb") as stdout, Path(f"{prefix}.stderr.txt").open("wb") as stderr:
        process = subprocess.Popen(command, cwd=cwd, env=env, stdout=stdout, stderr=stderr)
        try:
            process.wait(timeout=timeout)
        except subprocess.TimeoutExpired:
            subprocess.run(["taskkill", "/PID", str(process.pid), "/T", "/F"], capture_output=True, timeout=15)
            if process.poll() is None:
                process.kill()
            process.wait(timeout=15)
            raise RuntimeError(f"{Path(prefix).name} timed out; see logs/")
    if process.returncode != 0:
        raise RuntimeError(f"{Path(prefix).name} exited {process.returncode}; see logs/")


def part_digests(build, executable):
    native, project = build / "native", build / "project"
    paths = {"godot:exe": Path(executable), "godot:assembly": project / ".godot/mono/temp/bin/Debug/SA2Smoke.dll"}
    paths.update({"native:" + p.name: p for p in native.iterdir()
                  if p.is_file() and p.suffix.lower() in (".dll", ".dxil", ".exe")})
    paths.update({"project:" + p.relative_to(project).as_posix(): p for p in project.rglob("*")
                  if p.is_file() and not any(part in EXCLUDED for part in p.relative_to(project).parts[:-1])})
    return {name: {"path": str(path.resolve()), "sha256": sha256(path)} for name, path in sorted(paths.items())}


def launch_value(build, executable, environment):
    """prepare_l2.launch_value for this build's folders, unchanged in form."""
    project, native = (build / "project").resolve(), (build / "native").resolve()
    if Path(executable).name != GODOT_EXE:
        raise ValueError("level 2 must launch the process that presents, not the console wrapper")
    return {"format": "magic600-l2-launch-v1", "candidate": "sa2", "executable": str(Path(executable).resolve()),
            "dll": str(native / "sa2_interop.dll"), "working_directory": str(project),
            "arguments": ["--path", str(project), "--rendering-driver", "d3d12", "--disable-vsync", "--render-thread", "safe"],
            "run_arguments": ["--log-file", "{out}\\godot.log"], "validation_arguments": ["--gpu-validation"],
            "separator": ["--"], "environment": dict(environment), "validation_environment": {}}


def write_new(path, value):
    with Path(path).open("x", encoding="utf-8", newline="\n") as stream:
        stream.write(json.dumps(value, indent=2) + "\n")


def code_vectors():
    layout = json.loads((CORE / "code_layout.json").read_text(encoding="utf-8"))
    vectors = ";".join(f'{v["sequence"]},{v["slot"]},{v["generation"]},{v["code"]}'
                       for v in layout["test_vectors"] if "code" in v)
    if len(vectors.split(";")) != 3:
        raise RuntimeError("expected three code vectors in renderer/core/code_layout.json")
    return vectors


def build_host(executable, nupkgs):
    source = source_identity()
    stamp = dt.datetime.now(dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    build = BUILD_ROOT / stamp
    native, project, private, logs = build / "native", build / "project", build / "dotnet", build / "logs"
    if len(str(native)) > BUILD_PATH_LIMIT:
        raise RuntimeError("checkout path too long for the native build; use a shorter checkout path")
    script = CORE / "build.cmd"
    if any(c in str(script) + str(native) for c in ('"', "\r", "\n")):
        raise RuntimeError("unsupported build path")
    # Plain mkdir under the ignored work tree, never %TEMP% (Application Control).
    build.mkdir(parents=True)
    native.mkdir()
    logs.mkdir()
    private.mkdir()
    env = offline_environment(private, nupkgs)
    print(f"native build: {native}", flush=True)
    run_logged(f'cmd.exe /d /c call "{script}" "{native}"', CORE, env, 1800, logs / "native-build")
    missing = [name for name in ("sa2_interop.dll", "sa2_selftest.exe", *check_core.SHADERS) if not (native / name).is_file()]
    if missing:
        raise RuntimeError("native build outputs are missing: " + ", ".join(missing))
    check_core.check_sources(native)
    check_core.check_result(native / "sa2_selftest.exe", native, "cpu", code_vectors())
    shutil.copytree(HERE / "project", project, ignore=shutil.ignore_patterns(*EXCLUDED))
    print(f"dotnet build: {project}", flush=True)
    command = ["dotnet", "build", "SA2Smoke.csproj", "-c", "Debug", "-nologo", "-nodeReuse:false", "-p:UseSharedCompilation=false"]
    run_logged(command, project, env, 900, logs / "dotnet-build")
    check_package_cache(private / "packages")
    if not (project / ".godot/mono/temp/bin/Debug/SA2Smoke.dll").is_file():
        raise RuntimeError("the project assembly was not built")
    if source_identity() != source:
        raise RuntimeError("source changed during the build; discard this build")
    environment = {key: env[key] for key in ENVIRONMENT_KEYS}
    environment[INJECT] = None
    identity = {"format": "magic600-c9-acceptance-build-v1", "source": source, "godot_version": GODOT_VERSION,
                "abi_version": 3, "project": str(project.resolve()), "native": str(native.resolve()),
                "cpu_selftest": "pass", "environment": environment, "parts": part_digests(build, executable)}
    write_new(build / IDENTITY, identity)
    write_new(build / "launch.json", launch_value(build, executable, environment))
    return build


def verify(build, executable):
    """Check a prepared build without starting anything: its parts, its launch file and the current source."""
    build = Path(build).resolve()
    if not build.is_relative_to(BUILD_ROOT.resolve()):
        raise RuntimeError("the build is not under work/build/c9-acceptance")
    identity = json.loads((build / IDENTITY).read_text(encoding="utf-8"))
    if identity.get("format") != "magic600-c9-acceptance-build-v1" or identity.get("godot_version") != GODOT_VERSION:
        raise RuntimeError("not a C9 acceptance build identity")
    if Path(identity["native"]) != build / "native" or Path(identity["project"]) != build / "project":
        raise RuntimeError("prepared build paths do not match its directory")
    if part_digests(build, executable) != identity["parts"]:
        raise RuntimeError("prepared source or binary digests changed; rebuild")
    launch = json.loads((build / "launch.json").read_text(encoding="utf-8"))
    if launch != launch_value(build, executable, identity["environment"]):
        raise RuntimeError("launch.json does not match the build; rebuild")
    if source_identity()["sha256"] != identity["source"]["sha256"]:
        raise RuntimeError("renderer/core or renderer/acceptance/project changed since this build; rebuild")
    return identity


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--godot", type=Path, help="the pinned non-console or console exe; neither is started")
    parser.add_argument("--verify", type=Path, metavar="BUILD", help="check a prepared build; build nothing")
    args = parser.parse_args(argv)
    if os.name != "nt":
        parser.exit(1, "the C9 acceptance host requires the owner's Windows build tools\n")
    try:
        executable, nupkgs = locate_without_launch(args.godot)
        if args.verify:
            identity = verify(args.verify, executable)
            print(f"build verified: source {identity['source']['sha256'][:12]} at {identity['source']['head'][:12]}"
                  f"{'' if identity['source']['matches_head'] else ' (uncommitted source)'}; "
                  f"{len(identity['parts'])} parts unchanged")
        else:
            print(build_host(executable, nupkgs))
        return 0
    except (OSError, ValueError, KeyError, RuntimeError, subprocess.SubprocessError) as error:
        print(f"C9 acceptance host preparation failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
