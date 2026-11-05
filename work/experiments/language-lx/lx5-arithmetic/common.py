"""Local-only compilation and cleanup for this experiment."""
import hashlib
import gzip
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parent
REPO = ROOT.parents[3]
BUILD = ROOT / "build"
CANDIDATES = ("node", "dotnet", "rust")


def write(path, data):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(data.encode("utf-8") if isinstance(data, str) else data)


def json_bytes(value):
    return (json.dumps(value, indent=2, sort_keys=True, ensure_ascii=True) + "\n").encode("ascii")


def sha(data):
    return hashlib.sha256(data).hexdigest()


def reference_bytes():
    return gzip.decompress((ROOT / "fixtures/reference.tsv.gz").read_bytes())


def cleanup():
    # Resolve and check the one owned deletion target before recursive removal.
    if BUILD.resolve().parent != ROOT or BUILD.is_symlink():
        raise RuntimeError("build directory escaped the experiment")
    if BUILD.exists():
        shutil.rmtree(BUILD)


def environment():
    tmp = BUILD / "tmp"
    tmp.mkdir(parents=True, exist_ok=True)
    env = os.environ.copy()
    env.update(TMP=str(tmp), TEMP=str(tmp), DOTNET_CLI_HOME=str(tmp),
               DOTNET_NOLOGO="1", DOTNET_CLI_TELEMETRY_OPTOUT="1",
               DOTNET_SKIP_FIRST_TIME_EXPERIENCE="1", PYTHONDONTWRITEBYTECODE="1")
    return env


def run(args, timeout=300, env=None):
    return subprocess.run([str(a) for a in args], cwd=ROOT, env=env or environment(),
                          capture_output=True, timeout=timeout, check=True)


def versions():
    runtimes = run(["dotnet", "--list-runtimes"]).stdout.decode().splitlines()
    return {"python": sys.version.split()[0], "platform": sys.platform,
            "node": run(["node", "--version"]).stdout.decode().strip(),
            "dotnet_sdk": run(["dotnet", "--version"]).stdout.decode().strip(),
            "dotnet_runtime": max((line.split()[1] for line in runtimes if line.startswith("Microsoft.NETCore.App ")),
                                  key=lambda s: tuple(map(int, s.split(".")))),
            "rustc": run(["rustc", "--version"]).stdout.decode().strip()}


def compile_candidates():
    env = environment()
    # Call Roslyn directly: no project restore, NuGet, MSBuild or package source.
    sdk_line = run(["dotnet", "--list-sdks"], env=env).stdout.decode().strip().splitlines()[-1]
    sdk_version, sdk_base = sdk_line.split(" [", 1)
    sdk = Path(sdk_base.rstrip("]")) / sdk_version
    dotnet_root = sdk.parent.parent
    refs_root = dotnet_root / "packs" / "Microsoft.NETCore.App.Ref"
    ref_version = sorted(refs_root.iterdir(), key=lambda p: tuple(map(int, p.name.split("."))))[-1]
    framework = next((ref_version / "ref").iterdir()).name
    refs = sorted((ref_version / "ref" / framework).glob("*.dll"))
    args = ["-nologo", "-target:exe", "-optimize+", "-deterministic+", "-debug-",
            '-out:"' + str(BUILD / "candidate.dll") + '"']
    args += ['-r:"' + str(p) + '"' for p in refs]
    args += ['"' + str(ROOT / "candidate.cs") + '"']
    write(BUILD / "compile.rsp", "\n".join(args) + "\n")
    run(["dotnet", sdk / "Roslyn/bincore/csc.dll", "@" + str(BUILD / "compile.rsp")], env=env)
    runtime = sorted((dotnet_root / "shared/Microsoft.NETCore.App").iterdir(),
                     key=lambda p: tuple(map(int, p.name.split("."))))[-1].name
    write(BUILD / "candidate.runtimeconfig.json", json_bytes({"runtimeOptions": {
        "tfm": framework, "framework": {"name": "Microsoft.NETCore.App", "version": runtime}}}))
    rust_args = ["rustc", ROOT / "candidate.rs", "--edition=2021", "-O", "-C", "debuginfo=0",
                 "-o", BUILD / ("candidate.exe" if os.name == "nt" else "candidate")]
    if os.name == "nt":
        vswhere = Path(os.environ["ProgramFiles(x86)"]) / "Microsoft Visual Studio/Installer/vswhere.exe"
        vs = Path(run([vswhere, "-latest", "-products", "*", "-requires",
                       "Microsoft.VisualStudio.Component.VC.Tools.x86.x64", "-property", "installationPath"], env=env).stdout.decode().strip())
        msvc = sorted((vs / "VC/Tools/MSVC").iterdir())[-1]
        kits = Path(os.environ["ProgramFiles(x86)"]) / "Windows Kits/10/Lib"
        kit = sorted(kits.iterdir())[-1]
        env["LIB"] = os.pathsep.join(str(p) for p in (msvc / "lib/x64", kit / "ucrt/x64", kit / "um/x64"))
        rust_args += ["-C", "linker=" + str(msvc / "bin/Hostx64/x64/link.exe")]
    run(rust_args, env=env)
    return {"node": ["node", ROOT / "candidate.mjs"],
            "dotnet": ["dotnet", BUILD / "candidate.dll"],
            "rust": [BUILD / ("candidate.exe" if os.name == "nt" else "candidate")]}
