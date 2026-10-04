"""Build and test LookLab.Core offline, without checkout build products."""
from pathlib import Path
import hashlib
import os
import shutil
import subprocess
import sys
import tempfile
from uuid import uuid4

ROOT = Path(__file__).resolve().parents[2]


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


def main(argv=None):
    if (sys.argv[1:] if argv is None else argv):
        print("looklab check: no arguments are accepted", file=sys.stderr)
        return 2
    temporary = Path(tempfile.gettempdir()) / f"looklab-check-{uuid4().hex}"
    temporary.mkdir()
    result = 0
    before = None
    try:
        before = snapshot(ROOT)
        env = os.environ.copy()
        for key in ("DOTNET_CLI_HOME", "NUGET_PACKAGES", "NUGET_HTTP_CACHE_PATH",
                    "NUGET_PLUGINS_CACHE_PATH"):
            folder = temporary / key.lower()
            folder.mkdir()
            env[key] = str(folder)
        for key in ("DOTNET_CLI_TELEMETRY_OPTOUT", "DOTNET_NOLOGO",
                    "DOTNET_SKIP_FIRST_TIME_EXPERIENCE", "MSBUILDDISABLENODEREUSE",
                    "DOTNET_CLI_DO_NOT_USE_MSBUILD_SERVER"):
            env[key] = "1"
        art = temporary / "art"
        project = "tools/looklab/tests/LookLab.Tests.csproj"
        steps = [
            ["dotnet", "restore", project, "--configfile", "tools/looklab/nuget.config",
             "--artifacts-path", str(art), "-nodeReuse:false"],
            ["dotnet", "build", project, "--no-restore", "--artifacts-path", str(art),
             "-c", "Debug", "-nologo", "-nodeReuse:false", "-p:UseSharedCompilation=false",
             "--disable-build-servers"],
            ["dotnet", str(art / "bin/LookLab.Tests/debug/LookLab.Tests.dll"), str(ROOT)],
        ]
        for number, command in enumerate(steps, 1):
            print(f"looklab check: step {number}/3", flush=True)
            try:
                run = subprocess.run(command, cwd=ROOT, env=env, stdout=subprocess.PIPE,
                                     stderr=subprocess.STDOUT, text=True, errors="replace")
            except OSError as exc:
                print(f"looklab check: {command[0]} could not run: {exc}")
                result = 1
                break
            if run.returncode:
                print("\n".join(run.stdout.splitlines()[-60:]))
                result = run.returncode
                break
            if number == 3:
                print(run.stdout, end="")
    finally:
        try:
            shutil.rmtree(temporary)
        except OSError:
            print(f"looklab check: could not remove {temporary}")
        after = snapshot(ROOT) if before is not None else None
        if before is not None and after != before:
            changed = sorted(key for key in before.keys() | after.keys()
                             if before.get(key) != after.get(key))
            print("looklab check: checkout changed: " + ", ".join(changed))
            if result == 0:
                result = 1
        elif before is not None:
            print("looklab check: checkout unchanged (including ignored files)")
    return result


if __name__ == "__main__":
    sys.exit(main())
