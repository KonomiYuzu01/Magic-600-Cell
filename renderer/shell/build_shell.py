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
# The frame (A5): the chrome library and the design data are Look Lab's own sources, staged into the build's project.
# The repository holds no second copy of them.
CHROME = ROOT / "tools" / "looklab" / "app" / "Chrome"
DESIGN = ROOT / "tools" / "looklab" / "data" / "design"
# ADR-D02's sample data (B1), which the D02 surfaces read behind M600_D02=1; staged as samples/.
SAMPLES = ROOT / "tools" / "looklab" / "data" / "samples"
# The owner's opening look values, which the D02 view shape reads behind M600_D02=1; staged as look/opening.json.
OPENING = ROOT / "tools" / "looklab" / "data" / "opening.json"
# Local's own shape (B18c), which the D02 Local view reads behind M600_D02=1; staged as look/local-view.json.
LOCAL_VIEW = ROOT / "tools" / "looklab" / "data" / "local-view.json"
# Look Lab's Hopf rings (B23), which the D02 Global view opens on behind M600_D02=1; staged as look/hopf-rings.json.
HOPF_RINGS = ROOT / "tools" / "looklab" / "fixtures" / "hopf-rings.json"
# Sample mode (B4a, launch.py --sample): ADR-D01's frame on sample data. Look Lab's sample wiring is staged as Live/,
# one copy shared with Look Lab, and the files it reads as sample-root/ at their repository paths.
LIVE = ROOT / "tools" / "looklab" / "app" / "Live"
LIVE_FILES = ("LabDemo.cs", "LabSolve.cs", "LabScripts.cs", "SampleInspect.cs", "SampleMount.cs")
SAMPLE_ROOT_FILES = ("assets/seed_atlas.json",)
SAMPLE_ROOT_GLOBS = ("tools/looklab/data/samples/*.6cs", "tools/looklab/data/samples/packages/*.json",
                     "tools/looklab/data/design/keymaps/*.json")
COMMANDS = ROOT / "docs" / "progress" / "1.0" / "command-table.json"
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


def stage(project: Path) -> None:
    """Copy the shell project and the frame's sources into `project` (a new folder): the chrome library as Chrome/,
    the design data with the command table as design/, ADR-D02's samples as samples/, the opening look values as
    look/opening.json, and sample mode's wiring as Live/ with the files it reads under sample-root/."""
    shutil.copytree(PROJECT, project, ignore=shutil.ignore_patterns(*prepare_host.EXCLUDED))
    shutil.copytree(CHROME, project / "Chrome", ignore=shutil.ignore_patterns(*prepare_host.EXCLUDED))
    shutil.copytree(DESIGN, project / "design")
    shutil.copy2(COMMANDS, project / "design" / "command-table.json")
    shutil.copy2(COMMANDS.parent / "glossary" / "terms.json", project / "design" / "glossary-terms.json")
    shutil.copytree(SAMPLES, project / "samples")
    (project / "look").mkdir()
    shutil.copy2(OPENING, project / "look" / "opening.json")
    shutil.copy2(LOCAL_VIEW, project / "look" / "local-view.json")
    shutil.copy2(HOPF_RINGS, project / "look" / "hopf-rings.json")
    (project / "Live").mkdir()
    for name in LIVE_FILES:
        shutil.copy2(LIVE / name, project / "Live" / name)
    sample_root = project / "sample-root"
    files = [ROOT / name for name in SAMPLE_ROOT_FILES]
    for pattern in SAMPLE_ROOT_GLOBS:
        files += sorted(ROOT.glob(pattern))
    for path in files:
        target = sample_root / path.relative_to(ROOT)
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(path, target)


def build(executable: str | None, out: Path | None = None) -> Path:
    """Build into `out` (a new folder; default work/build/c9-shell/<stamp>/) and return it."""
    executable, nupkgs = prepare_host.locate_without_launch(executable)
    head = prepare_host.git("rev-parse", "HEAD").strip()
    dirty = prepare_host.git("status", "--porcelain", "--", "renderer/shell").strip() != ""
    if out is None:
        out = BUILD_ROOT / dt.datetime.now(dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    project, private, logs = out / "project", out / "dotnet", out / "logs"
    # Plain mkdir under the ignored work tree, never %TEMP% (Application Control).
    out.mkdir(parents=True)
    logs.mkdir()
    private.mkdir()
    stage(project)
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
