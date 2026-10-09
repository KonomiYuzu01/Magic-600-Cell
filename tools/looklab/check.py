"""Offline core checks and isolated Godot lanes (standard library only)."""
from pathlib import Path
import hashlib
import json
import os
import queue
import re
import shutil
import subprocess
import sys
import tempfile
import threading
import time
from uuid import uuid4
from xml.sax.saxutils import escape

ROOT = Path(__file__).resolve().parents[2]
LAB = Path("tools/looklab")
GODOT_EXE = "Godot_v4.7.2-stable_mono_win64_console.exe"
GODOT_VERSION = "4.7.2.stable.mono"
BUILD_TIMEOUT = 180
GODOT_TIMEOUT = 120
DIRECTORY_TIMEOUT = 20
DIRECTORIES = {"user", "config", "data", "cache"}


class CheckFailure(Exception):
    pass


class GraphicsUnavailable(CheckFailure):
    pass


APP_OUTPUT = "work/loop-memory/looklab/"


def app_output(key):
    """A snapshot key inside the app's output folder, or a folder above it."""
    return key.startswith(APP_OUTPUT) or (key.endswith("/") and APP_OUTPUT.startswith(key))


def snapshot(root):
    """Include ignored files as well as tracked files; do not traverse links."""
    result = {}
    for folder, dirs, files in os.walk(root, followlinks=False):
        dirs[:] = sorted(d for d in dirs if d != ".git")
        result[Path(folder).relative_to(root).as_posix() + "/"] = "directory"
        for name in sorted(files):
            path = Path(folder) / name
            key = path.relative_to(root).as_posix()
            if path.is_symlink():
                result[key] = ("link", os.readlink(path))
            else:
                with path.open("rb") as stream:
                    result[key] = hashlib.file_digest(stream, "sha256").hexdigest()
    return result


def scoped_env(parent=None):
    return {key: value for key, value in (os.environ if parent is None else parent).items()
            if not re.search("KEY|SECRET|TOKEN", key, re.IGNORECASE)}


def child_env(temporary):
    env = scoped_env()
    for key in ("DOTNET_CLI_HOME", "NUGET_PACKAGES", "NUGET_HTTP_CACHE_PATH",
                "NUGET_PLUGINS_CACHE_PATH", "APPDATA", "LOCALAPPDATA", "TEMP", "TMP"):
        folder = temporary / key.lower()
        folder.mkdir()
        env[key] = str(folder)
    for key in ("DOTNET_CLI_TELEMETRY_OPTOUT", "DOTNET_NOLOGO",
                "DOTNET_SKIP_FIRST_TIME_EXPERIENCE", "MSBUILDDISABLENODEREUSE",
                "DOTNET_CLI_DO_NOT_USE_MSBUILD_SERVER", "PYTHONDONTWRITEBYTECODE"):
        env[key] = "1"
    env["NUGET_FALLBACK_PACKAGES"] = ""
    env["LOOKLAB_TEMP_ROOT"] = str(temporary)
    # The Windows Python install-manager alias discovers runtimes through
    # AppData. Use the already-running interpreter after redirecting AppData.
    env["LOOKLAB_PYTHON"] = sys.executable
    return env


def inside(path, root):
    return Path(path).resolve().is_relative_to(Path(root).resolve())


class DirectoryGuard:
    def __init__(self, temporary):
        self.temporary = temporary
        self.seen = set()
        self.complete = False

    def line(self, line):
        if line.startswith("LOOKLAB_DIR "):
            try:
                _, kind, encoded = line.strip().split(" ", 2)
                path = json.loads(encoded)
            except (ValueError, json.JSONDecodeError) as exc:
                raise CheckFailure("malformed Godot directory report") from exc
            if kind not in DIRECTORIES or kind in self.seen or not isinstance(path, str):
                raise CheckFailure("invalid or repeated Godot directory: " + kind)
            if not Path(path).is_absolute() or not inside(path, self.temporary):
                raise CheckFailure(f"Godot {kind} directory is outside the temporary folder: {path}")
            self.seen.add(kind)
        elif line.strip() == "LOOKLAB_DIRS_END":
            self.complete = True
            self.finish()

    def finish(self):
        if self.seen != DIRECTORIES or not self.complete:
            raise CheckFailure("missing Godot directory report: " + ", ".join(sorted(DIRECTORIES - self.seen)) + (" (end marker)" if not self.complete else ""))


def listing(folder):
    """Observe names and mtimes only; never read personal Godot files."""
    return None if not folder.exists() else sorted((p.name, p.stat().st_mtime_ns) for p in folder.iterdir())


def real_godot_locations(parent):
    return [Path(parent[key]) / "Godot" for key in ("APPDATA", "LOCALAPPDATA") if parent.get(key)]


def verify_real_data(folders, before, project_name):
    for folder, previous in zip(folders, before):
        if (folder / "app_userdata" / project_name).exists():
            raise CheckFailure("Godot created the run's user directory in real AppData")
        if listing(folder) != previous:
            raise CheckFailure("real Godot AppData listing changed: " + str(folder))


def end_tree(process, env):
    if process.poll() is None:
        if os.name == "nt":
            subprocess.run(["taskkill", "/PID", str(process.pid), "/T", "/F"], env=scoped_env(env),
                           stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=15, check=False)
        else:
            process.kill()
    process.wait(timeout=15)


def run_process(command, *, cwd, env, timeout, guard=None):
    """Read live output so an unsafe directory report stops the process immediately."""
    print("looklab command: " + json.dumps([str(x) for x in command]), flush=True)
    try:
        process = subprocess.Popen(command, cwd=cwd, env=scoped_env(env), stdin=subprocess.PIPE,
                                   stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                                   text=True, errors="replace")
    except OSError as exc:
        raise CheckFailure(f"{command[0]} could not run: {exc}") from exc
    lines = queue.Queue()

    def read():
        try:
            for line in process.stdout:
                lines.put(line)
        finally:
            lines.put(None)

    reader = threading.Thread(target=read, daemon=True)
    reader.start()
    output = []
    started = time.monotonic()
    try:
        while True:
            elapsed = time.monotonic() - started
            if elapsed > timeout:
                raise CheckFailure(f"process timed out after {timeout}s")
            if guard and not guard.complete and elapsed > DIRECTORY_TIMEOUT:
                raise CheckFailure("Godot did not complete its directory report within 20s")
            try:
                line = lines.get(timeout=min(0.5, timeout - elapsed))
            except queue.Empty:
                continue
            if line is None:
                break
            output.append(line)
            if guard:
                was_complete = guard.complete
                guard.line(line)
                if guard.complete and not was_complete:
                    process.stdin.write("LOOKLAB_DIRECTORY_GUARD_OK\n")
                    process.stdin.flush()
        process.wait(timeout=max(0.01, timeout - (time.monotonic() - started)))
        return subprocess.CompletedProcess(command, process.returncode, "".join(output))
    except (CheckFailure, subprocess.TimeoutExpired) as exc:
        end_tree(process, env)
        print("".join(output), end="", flush=True)
        raise CheckFailure(str(exc)) from exc
    finally:
        process.stdin.close()
        reader.join(timeout=5)
        process.stdout.close()


def require_success(run):
    if run.returncode:
        print(run.stdout, end="" if run.stdout.endswith("\n") else "\n")
        raise CheckFailure(f"process exited {run.returncode}")


def core_check(temporary, env):
    art = temporary / "art"
    project = str(LAB / "tests/LookLab.Tests.csproj")
    steps = [
        ["dotnet", "restore", project, "--configfile", str(LAB / "nuget.config"),
         "--artifacts-path", str(art), "-nodeReuse:false", "-p:UseSharedCompilation=false",
         "--disable-build-servers"],
        ["dotnet", "build", project, "--no-restore", "--artifacts-path", str(art),
         "-c", "Debug", "-nologo", "-nodeReuse:false", "-p:UseSharedCompilation=false",
         "--disable-build-servers"],
        ["dotnet", str(art / "bin/LookLab.Tests/debug/LookLab.Tests.dll"), str(ROOT)],
    ]
    for number, command in enumerate(steps, 1):
        print(f"looklab core: step {number}/3", flush=True)
        run = run_process(command, cwd=ROOT, env=env, timeout=BUILD_TIMEOUT)
        require_success(run)
        if number == 3:
            print(run.stdout, end="")


def find_godot(parent):
    if parent.get("LOOKLAB_GODOT"):
        path = Path(parent["LOOKLAB_GODOT"])
        if not path.is_file():
            raise CheckFailure("LOOKLAB_GODOT does not name a console executable")
        return path.resolve()
    packages = Path(parent.get("LOCALAPPDATA", "")) / "Microsoft/WinGet/Packages"
    matches = sorted(p for folder in packages.glob("GodotEngine.GodotEngine.Mono_*")
                     for p in folder.rglob(GODOT_EXE) if p.is_file())
    if len(matches) != 1:
        raise CheckFailure(f"expected exactly one {GODOT_EXE} under the WinGet Mono packages; found {len(matches)}. Set LOOKLAB_GODOT explicitly.")
    return matches[0].resolve()


def check_version(output):
    if not any(line.strip() == GODOT_VERSION or line.strip().startswith(GODOT_VERSION + ".")
               for line in output.splitlines()):
        raise CheckFailure("Godot version must be " + GODOT_VERSION + "; got " + output.strip())


def stage_project(temporary, packages, project_name):
    stage = temporary / "stage"
    stage.mkdir()
    for name in ("app", "core"):
        source = ROOT / LAB / name
        for folder, dirs, files in os.walk(source, followlinks=False):
            dirs[:] = [d for d in dirs if d not in ("bin", "obj", ".godot")]
            target = stage / name / Path(folder).relative_to(source)
            target.mkdir(parents=True, exist_ok=True)
            for item in dirs + files:
                if (Path(folder) / item).is_symlink():
                    raise CheckFailure("staged sources must not contain links")
            for file in files:
                shutil.copyfile(Path(folder) / file, target / file)
    shutil.copyfile(ROOT / LAB / "Directory.Build.props", stage / "Directory.Build.props")
    config = '<?xml version="1.0" encoding="utf-8"?>\n<configuration>\n  <packageSources><clear /><add key="GodotLocal" value="' + escape(str(packages), {'"': '&quot;'}) + '" /></packageSources>\n  <fallbackPackageFolders><clear /></fallbackPackageFolders>\n</configuration>\n'
    (stage / "nuget.config").write_text(config, encoding="utf-8")
    project = stage / "app/project.godot"
    text = project.read_text(encoding="utf-8")
    text, count = re.subn(r'^config/name="[^"]*"$', f'config/name="{project_name}"', text, flags=re.MULTILINE)
    if count != 1 or "config/use_custom_user_dir" in text:
        raise CheckFailure("staged project needs one name and no custom user directory")
    project.write_text(text, encoding="utf-8")
    return stage


def audit_packages(cache, source):
    required = {"godot.net.sdk", "godot.sourcegenerators", "godotsharp"}
    allowed = required | {"godotsharpeditor"}
    present = set()
    for package in cache.iterdir():
        if not package.is_dir() or package.name not in allowed:
            raise CheckFailure("unexpected cached package: " + package.name)
        versions = list(package.iterdir())
        if len(versions) != 1 or versions[0].name != "4.7.2":
            raise CheckFailure("cached package is not exclusively 4.7.2: " + package.name)
        version = versions[0]
        metadata = json.loads((version / ".nupkg.metadata").read_text(encoding="utf-8"))
        if Path(metadata.get("source", "")).resolve() != source.resolve():
            raise CheckFailure("cached package came from another source: " + package.name)
        archive = version / f"{package.name}.4.7.2.nupkg"
        originals = [p for p in source.iterdir() if p.name.lower() == archive.name]
        if len(originals) != 1 or hashlib.sha256(archive.read_bytes()).digest() != hashlib.sha256(originals[0].read_bytes()).digest():
            raise CheckFailure("cached package differs from the local Godot archive: " + package.name)
        present.add(package.name)
    if not required <= present:
        raise CheckFailure("missing cached Godot packages: " + ", ".join(sorted(required - present)))


def graphics_report(output):
    lines = [line[len("LOOKLAB_GRAPHICS "):] for line in output.splitlines() if line.startswith("LOOKLAB_GRAPHICS ")]
    if len(lines) != 1:
        raise CheckFailure("missing or repeated graphics capability report")
    report = json.loads(lines[0])
    if report.get("display", "").lower() != "windows" or report.get("driver", "").lower() not in ("vulkan", "d3d12") or not report.get("adapter", "").strip():
        raise CheckFailure("graphics lane requires Windows display, Vulkan or Direct3D 12, and a non-empty adapter: " + lines[0])


def graphics_startup_unavailable(output):
    return any(message in output.lower() for message in (
        "unable to create window", "failed to create window", "cannot create vulkan device",
        "failed to create vulkan device", "unable to initialize vulkan", "failed to initialize direct3d 12",
        "cannot create vulkan instance", "can't create a vulkan device", "vkcreateinstance failed",
        "vkcreatedevice failed", "failed to create displayserver", "your gpu does not support vulkan"))


def godot_run(godot, stage, temporary, env, arguments, *, graphics=False, expected_failure=None, timeout=GODOT_TIMEOUT):
    command = [str(godot), "--path", str(stage / "app"), *arguments]
    guard = DirectoryGuard(temporary)
    run = run_process(command, cwd=stage, env=env, timeout=timeout, guard=guard)
    print(run.stdout, end="")
    if graphics:
        # A reported dummy/headless/empty adapter is always a failure, even if
        # another line also mentions a window or driver startup error.
        if any(line.startswith("LOOKLAB_GRAPHICS ") for line in run.stdout.splitlines()):
            graphics_report(run.stdout)
        elif run.returncode and graphics_startup_unavailable(run.stdout):
            raise GraphicsUnavailable("graphics lane not run: window or real rendering driver unavailable")
        else:
            graphics_report(run.stdout)
    if expected_failure is not None:
        if run.returncode != 1 or f"LOOKLAB_FAIL {expected_failure}" not in run.stdout:
            raise CheckFailure("injected fault did not fail its own check: " + expected_failure)
    else:
        if run.returncode:
            raise CheckFailure(f"Godot exited {run.returncode}")
    guard.finish()
    return run


def godot_lane(mode, temporary, env, parent):
    godot = find_godot(parent)
    packages = godot.parent / "GodotSharp/Tools/nupkgs"
    if not packages.is_dir():
        raise CheckFailure("Godot install has no GodotSharp/Tools/nupkgs folder")
    project_name = "LookLabCheck-" + uuid4().hex
    folders = real_godot_locations(parent)
    before = [listing(p) for p in folders]
    try:
        version = run_process([str(godot), "--version"], cwd=temporary, env=env, timeout=15)
        require_success(version)
        check_version(version.stdout)
        stage = stage_project(temporary, packages, project_name)
        project = str(stage / "app/LookLab.csproj")
        for operation in ("restore", "build"):
            command = ["dotnet", operation, project, "-nodeReuse:false", "-p:UseSharedCompilation=false", "--disable-build-servers"]
            command += ["--configfile", str(stage / "nuget.config")] if operation == "restore" else ["--no-restore", "-c", "Debug", "-nologo"]
            require_success(run_process(command, cwd=stage, env=env, timeout=BUILD_TIMEOUT))
        audit_packages(Path(env["NUGET_PACKAGES"]), packages)
        print("looklab Godot: staged build and local package audit passed", flush=True)
        godot_run(godot, stage, temporary, env, ["--headless", "--import"])
        common = ["--", str(ROOT), str(temporary)]
        if mode == "--godot":
            run = godot_run(godot, stage, temporary, env, ["--headless", *common, "stage0-cpu"])
            if "LOOKLAB_STAGE0_CPU_PASS" not in run.stdout:
                raise CheckFailure("stage 0 CPU result missing")
            run = godot_run(godot, stage, temporary, env, ["--headless", *common, "smoke"])
            if not re.search(r"^LOOKLAB_SMOKE_PASS instances=600 vertices=30480 slots=259800 parameters=42 bytes=[1-9][0-9]*$", run.stdout, re.MULTILINE):
                raise CheckFailure("app smoke result missing or counts wrong")
            for fault in ("skip-parameter", "wrong-instance-count", "changed-preset-byte"):
                godot_run(godot, stage, temporary, env, ["--headless", *common, "smoke", fault], expected_failure=fault)
        elif mode == "--godot-gpu":
            run = godot_run(godot, stage, temporary, env, [*common, "stage0-gpu"], graphics=True)
            if "LOOKLAB_STAGE0_GPU_PASS" not in run.stdout:
                raise CheckFailure("stage 0 graphics result missing")
            godot_run(godot, stage, temporary, env, [*common, "stage0-gpu", "float-offset"], graphics=True, expected_failure="float-offset")
            run = godot_run(godot, stage, temporary, env, [*common, "geometry"], graphics=True)
            if "LOOKLAB_GEOMETRY_PASS cases=9 samples=2700 target=RGBA32F" not in run.stdout or "LOOKLAB_DRAW_COUNT_PASS instances=600 visible=600 primitives=6096000" not in run.stdout:
                raise CheckFailure("shader geometry or full draw-count result missing")
            for fault in ("transpose-q", "missing-instance"):
                godot_run(godot, stage, temporary, env, [*common, "geometry", fault], graphics=True, expected_failure=fault)
        else:
            godot_run(godot, stage, temporary, env, common, timeout=7200)
    finally:
        verify_real_data(folders, before, project_name)
        print("looklab Godot: real AppData unchanged", flush=True)


def main(argv=None):
    args = sys.argv[1:] if argv is None else argv
    if args not in ([], ["--godot"], ["--godot-gpu"], ["--run"]):
        print("looklab usage: check.py [--godot | --godot-gpu | --run]", file=sys.stderr)
        return 2
    temporary = Path(tempfile.gettempdir()) / f"looklab-check-{uuid4().hex}"
    result, before = 0, None
    try:
        temporary.mkdir()
        before = snapshot(ROOT)
        env = child_env(temporary)
        if args != ["--run"]:
            core_check(temporary, env)
        if args:
            godot_lane(args[0], temporary, env, os.environ)
    except GraphicsUnavailable as exc:
        print("looklab check: " + str(exc))
        result = 3
    except (CheckFailure, OSError, ValueError) as exc:
        print("looklab check: " + str(exc))
        result = 1
    finally:
        try:
            if temporary.exists():
                shutil.rmtree(temporary)
        except OSError as exc:
            print(f"looklab check: could not remove {temporary}: {exc}")
            result = 1
        after = snapshot(ROOT) if before is not None else None
        if before is not None and args == ["--run"]:
            # The interactive app may write its own outputs, such as saved presets.
            before, after = ({key: value for key, value in state.items() if not app_output(key)}
                             for state in (before, after))
        if before is not None and after != before:
            changed = sorted(key for key in before.keys() | after.keys() if before.get(key) != after.get(key))
            print("looklab check: checkout changed: " + ", ".join(changed))
            result = 1
        elif before is not None:
            outside = " outside " + APP_OUTPUT if args == ["--run"] else ""
            print(f"looklab check: checkout unchanged{outside} (including ignored files)")
    return result


if __name__ == "__main__":
    sys.exit(main())
