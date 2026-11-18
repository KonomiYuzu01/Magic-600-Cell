"""Build C9's product shell offline; never launch Godot, PresentMon or a GPU device.

The shell (renderer/shell/project) is a Godot 4.7.2 .NET project on net10.0, the supported .NET release that ADR-R01.5
requires in place of the acceptance harness's net8.0. This script copies the project to work/build/c9-shell/<stamp>/,
restores the four Godot packages from the pinned Godot install's own package folder only (the harness's offline
environment: renderer/acceptance/prepare_host.py), builds the C# assembly and writes build.json with the inputs and
the assembly's digest. A pass shows that the project builds offline; it is no Windows/DirectX, input or
performance evidence, and it builds no core and no bridge.

  python -B renderer/shell/build_shell.py [--godot <exe>]
"""
from __future__ import annotations

import sys

sys.dont_write_bytecode = True

import argparse
import datetime as dt
import json
import shutil
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PROJECT = ROOT / "renderer" / "shell" / "project"
BUILD_ROOT = ROOT / "work" / "build" / "c9-shell"
CSPROJ = "Magic600Shell.csproj"
ASSEMBLY = ".godot/mono/temp/bin/Debug/Magic600Shell.dll"
sys.path.insert(0, str(ROOT / "renderer" / "acceptance"))
import prepare_host  # noqa: E402  (the harness's Godot locator and offline restore)


def build_command() -> list[str]:
    """The dotnet build of the shell, as the harness builds its own project."""
    return ["dotnet", "build", CSPROJ, "-c", "Debug", "-nologo", "-nodeReuse:false", "-p:UseSharedCompilation=false"]


def dotnet_version() -> str:
    completed = subprocess.run(["dotnet", "--version"], capture_output=True, text=True, timeout=60)
    return completed.stdout.strip() if completed.returncode == 0 else f"unavailable (exit {completed.returncode})"


def build(executable: str | None) -> Path:
    executable, nupkgs = prepare_host.locate_without_launch(executable)
    head = prepare_host.git("rev-parse", "HEAD").strip()
    dirty = prepare_host.git("status", "--porcelain", "--", "renderer/shell").strip() != ""
    stamp = dt.datetime.now(dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    out = BUILD_ROOT / stamp
    project, private, logs = out / "project", out / "dotnet", out / "logs"
    # Plain mkdir under the ignored work tree, never %TEMP% (Application Control).
    out.mkdir(parents=True)
    logs.mkdir()
    private.mkdir()
    shutil.copytree(PROJECT, project, ignore=shutil.ignore_patterns(*prepare_host.EXCLUDED))
    env = prepare_host.offline_environment(private, nupkgs)
    print(f"dotnet build: {project}", flush=True)
    prepare_host.run_logged(build_command(), project, env, 900, logs / "dotnet-build")
    prepare_host.check_package_cache(private / "packages")
    assembly = project / ASSEMBLY
    if not assembly.is_file():
        raise RuntimeError("the shell assembly was not built")
    record = {
        "format": "magic600-c9-shell-build-v1",
        "evidence": "source build only: no launch, no core, no bridge",
        "head": head,
        "renderer_shell_dirty": dirty,
        "godot_executable": str(Path(executable).resolve()),
        "godot_version_expected": prepare_host.GODOT_VERSION,
        "dotnet": dotnet_version(),
        "command": build_command(),
        "assembly": {"path": str(assembly.resolve()), "sha256": prepare_host.sha256(assembly)},
    }
    prepare_host.write_new(out / "build.json", record)
    return out


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--godot", help="the pinned Godot 4.7.2 .NET executable, if not the winget install")
    args = parser.parse_args(argv)
    try:
        out = build(args.godot)
    except (OSError, RuntimeError, subprocess.SubprocessError) as error:
        print(f"FAIL: {error}")
        return 1
    print(f"PASS: the shell built offline in {out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
