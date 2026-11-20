"""Prepare an offline build of the C9 acceptance host; never launch Godot, PresentMon or a GPU device.

The host is S-A2 as tested: the official Godot 4.7.2 .NET editor binary running renderer/acceptance/project
(net8.0) on the product core renderer/core (ABI 5). Derived from work/experiments/renderer-sa2/prepare_l2.py and
run_smoke.build (see PROVENANCE.md). Everything is written under work/build/c9-acceptance/<UTC>/:
native/ (the core), project/ (a copy of the Godot project, built offline), dotnet/ (private NuGet and
.NET folders), logs/, c9-build-identity.json and launch.json. The core's CPU self-test runs after the native
build; no GPU mode runs.

--route surface (C9 U4) builds the route-B variant: the same sources plus renderer/acceptance/route-b/override.cfg
in project/ (a transparent window with vsync, as the product shell's), no --disable-vsync in launch.json, and
"route": "surface" in the build identity, which run_scene.ps1 reads to pass --sa2-route surface.

--stages DIR (C9 U5c; DIR is renderer/acceptance/stages, the renderer gate's stage pair) also builds the user
stages, on either route: it builds c600-stage-hlsl offline under work/build/c9-stage-tool, runs it for DIR's
vertex.6cs and fragment.6cs with DIR's limits.json into stages/, copies DIR's arrays.json there, and copies
dxcompiler.dll and dxil.dll from the DXC folder (--dxc, default the toolchain's tools/.dxc/v<version>/bin/x64)
beside the core in native/. The build identity's "stages" record holds the stage digests, the HLSL and record
SHA-256s, the overdraw allowance the harness attaches, the d4 value against the built-in projection's, the DLLs'
SHA-256s and file versions and the toolchain's DXC version; run_scene.ps1 reads it to pass --sa2-stages.

  python -B renderer/acceptance/prepare_host.py [--godot <exe>] [--route texture|surface] [--stages DIR [--dxc DIR]]
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
import struct
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
SOURCES = ("renderer/acceptance/project", "renderer/acceptance/route-b", "renderer/core")
EXCLUDED = {".godot", "bin", "obj", "__pycache__"}
IDENTITY = "c9-build-identity.json"
# C9 U4: the texture routes (Godot shows the core's ring) and route B (the core presents on its own surface).
ROUTES = ("texture", "surface")
OVERRIDE = HERE / "route-b" / "override.cfg"
# C9 U5c: the user-stage build (--stages). The stage sources, the generator and its own target folder, the files the
# harness reads from the build's stages/ folder, and the DXC libraries the core loads from its own folder.
STAGE_SOURCE = HERE / "stages"
STAGE_INPUTS = ("arrays.json", "fragment.6cs", "limits.json", "vertex.6cs")
STAGE_KINDS = ("vertex", "fragment")
STAGE_FILES = ("arrays.json", "fragment.hlsl", "fragment.json", "vertex.hlsl", "vertex.json")
STAGE_CONTRACT = "m600-stage-hlsl/1"
ARRAYS_FORMAT = "magic600-stage-arrays/1"
STAGE_TOOL_TARGET = ROOT / "work" / "build" / "c9-stage-tool"
STAGE_TOOL_SOURCES = ("rust", "renderer/acceptance/stages")
DXC_LIBRARIES = check_core.DXC_LIBRARIES
TOOLCHAIN_LOCK = ROOT / "tools" / "toolchain.lock.json"
# The built-in projection's d4 (renderer/core/src/scene.cpp constants(): 1.18f) as binary32 bits, 0x3f970a3d.
BUILTIN_D4_BITS = "0x" + struct.pack(">f", 1.18).hex()


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
    # C9 U5c: a stage build's stages/ folder (the harness reads every file in it).
    if (build / "stages").is_dir():
        paths.update({"stages:" + p.name: p for p in (build / "stages").iterdir() if p.is_file()})
    return {name: {"path": str(path.resolve()), "sha256": sha256(path)} for name, path in sorted(paths.items())}


def launch_value(build, executable, environment, route="texture"):
    """prepare_l2.launch_value for this build's folders, unchanged in form; route B keeps Godot's vsync."""
    project, native = (build / "project").resolve(), (build / "native").resolve()
    if Path(executable).name != GODOT_EXE:
        raise ValueError("level 2 must launch the process that presents, not the console wrapper")
    if route not in ROUTES:
        raise ValueError("unknown route")
    vsync = [] if route == "surface" else ["--disable-vsync"]
    return {"format": "magic600-l2-launch-v1", "candidate": "sa2", "executable": str(Path(executable).resolve()),
            "dll": str(native / "sa2_interop.dll"), "working_directory": str(project),
            "arguments": ["--path", str(project), "--rendering-driver", "d3d12", *vsync, "--render-thread", "safe"],
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


def git_clean(*paths):
    """Whether these repository paths have no change against HEAD, untracked files included."""
    return git("status", "--porcelain", "--untracked-files=all", "--", *paths) == ""


def stage_source(directory):
    """The stage sources: a folder inside the repository with exactly the four inputs, and their digests."""
    directory = Path(directory)
    directory = (directory if directory.is_absolute() else ROOT / directory).resolve()
    if not directory.is_relative_to(ROOT.resolve()) or not directory.is_dir():
        raise RuntimeError("--stages needs a stage folder inside the repository, such as renderer/acceptance/stages")
    names = sorted(p.name for p in directory.iterdir())
    if names != sorted(STAGE_INPUTS):
        raise RuntimeError(f"the stage folder must hold exactly {', '.join(STAGE_INPUTS)}")
    relative = directory.relative_to(ROOT.resolve()).as_posix()
    return {"directory": relative, "files": {name: sha256(directory / name) for name in STAGE_INPUTS},
            "matches_head": git_clean(relative)}


def dxc_lock():
    """The toolchain's DXC entry (tools/toolchain.lock.json, id dxc)."""
    entries = json.loads(TOOLCHAIN_LOCK.read_text(encoding="utf-8"))["tools"]
    found = [entry for entry in entries if isinstance(entry, dict) and entry.get("id") == "dxc"]
    if len(found) != 1 or not isinstance(found[0].get("version"), str) or not isinstance(found[0].get("dest"), str):
        raise RuntimeError("tools/toolchain.lock.json has no single dxc entry with a version and a dest")
    return found[0]


def dxc_directory(explicit=None):
    if explicit:
        directory = Path(explicit).resolve()
    else:
        directory = (ROOT / dxc_lock()["dest"] / "bin" / "x64").resolve()
    missing = [name for name in DXC_LIBRARIES if not (directory / name).is_file()]
    if missing:
        raise RuntimeError(f"the DXC folder {directory} lacks {', '.join(missing)}; install dxc or pass --dxc")
    return directory


def file_version(path):
    """The fixed file version of a Windows module's version resource, read as data; the module is never run."""
    import ctypes
    from ctypes import wintypes as wt
    library = ctypes.WinDLL("version", use_last_error=True)
    library.GetFileVersionInfoSizeW.argtypes, library.GetFileVersionInfoSizeW.restype = [wt.LPCWSTR, wt.LPDWORD], wt.DWORD
    library.GetFileVersionInfoW.argtypes = [wt.LPCWSTR, wt.DWORD, wt.DWORD, ctypes.c_void_p]
    library.GetFileVersionInfoW.restype = wt.BOOL
    library.VerQueryValueW.argtypes = [ctypes.c_void_p, wt.LPCWSTR, ctypes.POINTER(ctypes.c_void_p), ctypes.POINTER(wt.UINT)]
    library.VerQueryValueW.restype = wt.BOOL
    size = library.GetFileVersionInfoSizeW(str(path), None)
    if not size:
        raise RuntimeError(f"{Path(path).name} has no version resource")
    data = ctypes.create_string_buffer(size)
    if not library.GetFileVersionInfoW(str(path), 0, size, data):
        raise RuntimeError(f"cannot read the version resource of {Path(path).name}")
    pointer, length = ctypes.c_void_p(), wt.UINT()
    if not library.VerQueryValueW(data, "\\", ctypes.byref(pointer), ctypes.byref(length)) or length.value < 52:
        raise RuntimeError(f"{Path(path).name} has no fixed file version")
    # VS_FIXEDFILEINFO: dwSignature, dwStrucVersion, dwFileVersionMS, dwFileVersionLS, ...
    fixed = ctypes.cast(pointer, ctypes.POINTER(ctypes.c_uint32 * 4)).contents
    if fixed[0] != 0xFEEF04BD:
        raise RuntimeError(f"{Path(path).name} has a malformed fixed file version")
    return f"{fixed[2] >> 16}.{fixed[2] & 0xFFFF}.{fixed[3] >> 16}.{fixed[3] & 0xFFFF}"


def read_arrays(path):
    """arrays.json (magic600-stage-arrays/1): per stage kind, each array's values as binary32 bit patterns."""
    value = json.loads(Path(path).read_text(encoding="utf-8"))
    if not isinstance(value, dict) or value.get("format") != ARRAYS_FORMAT or set(value) - {"format", "note", *STAGE_KINDS}:
        raise RuntimeError(f"arrays.json is not {ARRAYS_FORMAT}")
    arrays = {}
    for kind in STAGE_KINDS:
        given = value.get(kind)
        if not isinstance(given, dict):
            raise RuntimeError(f"arrays.json lacks its {kind} object")
        for name, bits in given.items():
            if not isinstance(bits, list) or not bits or not all(
                    isinstance(bit, str) and len(bit) == 10 and bit.startswith("0x")
                    and all(c in "0123456789abcdef" for c in bit[2:]) for bit in bits):
                raise RuntimeError(f"arrays.json {kind}.{name} must list lowercase 0x%08x bit patterns")
        arrays[kind] = given
    return arrays


def stage_record(stages, kind, given):
    """Check one generated stage against its HLSL and the given arrays; return its identity entry."""
    hlsl, record_path = stages / f"{kind}.hlsl", stages / f"{kind}.json"
    record = json.loads(record_path.read_text(encoding="utf-8"))
    if (record.get("kind") != kind or record.get("contract") != STAGE_CONTRACT
            or record.get("hlsl_sha256") != sha256(hlsl) or type(record.get("overdraw")) is not int
            or record["overdraw"] <= 0 or record["overdraw"] >= 2**32):
        raise RuntimeError(f"the generated {kind} record does not match its HLSL, kind or contract")
    layout = record.get("arrays")
    if not isinstance(layout, list) or sorted(entry.get("name") for entry in layout) != sorted(given):
        raise RuntimeError(f"arrays.json's {kind} arrays differ from the stage's declared arrays")
    covered = set()
    for entry in layout:
        if len(given[entry["name"]]) != entry.get("length"):
            raise RuntimeError(f"arrays.json {kind}.{entry['name']} does not have the declared length")
        span = set(range(entry["offset"], entry["offset"] + entry["length"]))
        if span & covered:
            raise RuntimeError(f"the {kind} arrays overlap")
        covered |= span
    if covered != set(range(len(covered))):
        raise RuntimeError(f"the {kind} arrays leave a gap")
    return {"digest": record["digest"], "hlsl_sha256": record["hlsl_sha256"], "json_sha256": sha256(record_path),
            "overdraw": record["overdraw"], "ops": record.get("ops"), "module": record.get("module"),
            "entry": record.get("entry"), "generator": record.get("generator"), "arrays": layout}


def build_stages(build, logs, source_directory, dxc):
    """C9 U5c: generate the stage pair into stages/, copy DXC beside the core; return the identity's stages record."""
    source = stage_source(source_directory)
    inputs = ROOT / source["directory"]
    tool_matches_head = git_clean(*STAGE_TOOL_SOURCES)
    stages, native = build / "stages", build / "native"
    stages.mkdir()
    env = os.environ.copy()
    env.update({"CARGO_TARGET_DIR": str(STAGE_TOOL_TARGET), "CARGO_TERM_COLOR": "never"})
    print(f"stage generator build: {STAGE_TOOL_TARGET}", flush=True)
    run_logged(["cargo", "build", "--offline", "--locked", "--manifest-path", str(ROOT / "rust" / "Cargo.toml"),
                "-p", "c600-render", "--bin", "c600-stage-hlsl"], ROOT, env, 1800, logs / "stage-tool-build")
    tool = STAGE_TOOL_TARGET / "debug" / "c600-stage-hlsl.exe"
    if not tool.is_file():
        raise RuntimeError("c600-stage-hlsl was not built")
    tool_sha256 = sha256(tool)
    for kind in STAGE_KINDS:
        run_logged([str(tool), "--source", str(inputs / f"{kind}.6cs"), "--limits", str(inputs / "limits.json"),
                    "--out", str(stages)], ROOT, env, 300, logs / f"stage-hlsl-{kind}")
    shutil.copyfile(inputs / "arrays.json", stages / "arrays.json")
    if sorted(p.name for p in stages.iterdir()) != sorted(STAGE_FILES):
        raise RuntimeError("the stage folder does not hold exactly the generated pair and arrays.json")
    if sha256(tool) != tool_sha256 or stage_source(source_directory) != source:
        raise RuntimeError("the stage sources or the generator changed during the build; discard this build")
    arrays = read_arrays(stages / "arrays.json")
    records = {kind: stage_record(stages, kind, arrays[kind]) for kind in STAGE_KINDS}
    d4 = arrays["vertex"].get("d4")
    if d4 != [BUILTIN_D4_BITS]:
        raise RuntimeError(f"arrays.json's vertex d4 must be the built-in projection's {BUILTIN_D4_BITS} (1.18f)")
    for name in DXC_LIBRARIES:
        shutil.copyfile(dxc / name, native / name)
    lock = dxc_lock()
    return {"directory": str(stages.resolve()), "source": source,
            "generator": {"name": "c600-stage-hlsl", "executable": str(tool.resolve()), "sha256": tool_sha256,
                          "sources_match_head": tool_matches_head},
            **records,
            # The allowance the harness attaches with sa2_scene_stages: the smaller of the two declared values.
            "overdraw": min(records[kind]["overdraw"] for kind in STAGE_KINDS),
            "arrays": {"sha256": sha256(stages / "arrays.json"), **arrays},
            "d4": {"bits": d4[0], "builtin_bits": BUILTIN_D4_BITS,
                   "builtin": "1.18f, renderer/core/src/scene.cpp constants()", "equal": d4[0] == BUILTIN_D4_BITS},
            "dxc": {"version": lock["version"], "lock_dest": lock["dest"], "source": str(dxc),
                    "files": {name: {"sha256": sha256(native / name), "file_version": file_version(native / name)}
                              for name in DXC_LIBRARIES}}}


def verify_stages(build, identity):
    """A stage build's record against its parts and the current stage sources; a build without one has no stages."""
    stages, parts = identity.get("stages"), identity["parts"]
    staged = sorted(name[len("stages:"):] for name in parts if name.startswith("stages:"))
    dxc = sorted(name[len("native:"):] for name in parts if name[len("native:"):] in DXC_LIBRARIES)
    if stages is None:
        if staged or dxc or (build / "stages").exists():
            raise RuntimeError("a build without a stages record holds stage files; rebuild")
        return None
    if staged != sorted(STAGE_FILES) or dxc != sorted(DXC_LIBRARIES) or Path(stages["directory"]) != build / "stages":
        raise RuntimeError("the stage build's files do not match its stages record; rebuild")
    for kind in STAGE_KINDS:
        if (parts[f"stages:{kind}.hlsl"]["sha256"] != stages[kind]["hlsl_sha256"]
                or parts[f"stages:{kind}.json"]["sha256"] != stages[kind]["json_sha256"]):
            raise RuntimeError(f"the {kind} stage does not match its stages record; rebuild")
    if parts["stages:arrays.json"]["sha256"] != stages["arrays"]["sha256"]:
        raise RuntimeError("arrays.json does not match its stages record; rebuild")
    if any(parts[f"native:{name}"]["sha256"] != stages["dxc"]["files"][name]["sha256"] for name in DXC_LIBRARIES):
        raise RuntimeError("the DXC libraries beside the core do not match the stages record; rebuild")
    if stages["d4"]["bits"] != BUILTIN_D4_BITS or stages["d4"]["builtin_bits"] != BUILTIN_D4_BITS:
        raise RuntimeError("the build's d4 is not the built-in projection's; rebuild")
    if stage_source(stages["source"]["directory"])["files"] != stages["source"]["files"]:
        raise RuntimeError(f"{stages['source']['directory']} changed since this build; rebuild")
    return stages


def build_host(executable, nupkgs, route="texture", stages_source=None, dxc=None):
    if route not in ROUTES:
        raise ValueError("unknown route")
    source = source_identity()
    stamp = dt.datetime.now(dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    build = BUILD_ROOT / stamp
    native, project, private, logs = build / "native", build / "project", build / "dotnet", build / "logs"
    if len(str(native)) > BUILD_PATH_LIMIT:
        raise RuntimeError("checkout path too long for the native build; use a shorter checkout path")
    script = CORE / "build.cmd"
    if any(c in str(script) + str(native) for c in ('"', "\r", "\n")):
        raise RuntimeError("unsupported build path")
    if stages_source is not None:
        # Before anything is built: the stage sources and the DXC libraries must exist (C9 U5c).
        stage_source(stages_source)
        dxc = dxc_directory(dxc)
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
    stages = None if stages_source is None else build_stages(build, logs, stages_source, dxc)
    shutil.copytree(HERE / "project", project, ignore=shutil.ignore_patterns(*EXCLUDED))
    if route == "surface":
        # Godot reads res://override.cfg over the verbatim project.godot (C9 U4).
        shutil.copyfile(OVERRIDE, project / "override.cfg")
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
                "abi_version": 5, "route": route, "project": str(project.resolve()), "native": str(native.resolve()),
                "cpu_selftest": "pass", "environment": environment, "parts": part_digests(build, executable)}
    if stages is not None:
        identity["stages"] = stages
    write_new(build / IDENTITY, identity)
    write_new(build / "launch.json", launch_value(build, executable, environment, route))
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
    # Builds from before C9 U4 carry no route: they are texture builds.
    route = identity.get("route", "texture")
    if route not in ROUTES or ("project:override.cfg" in identity["parts"]) != (route == "surface"):
        raise RuntimeError("the build's route and its override.cfg disagree; rebuild")
    if route == "surface" and identity["parts"]["project:override.cfg"]["sha256"] != sha256(OVERRIDE):
        raise RuntimeError("renderer/acceptance/route-b changed since this build; rebuild")
    launch = json.loads((build / "launch.json").read_text(encoding="utf-8"))
    if launch != launch_value(build, executable, identity["environment"], route):
        raise RuntimeError("launch.json does not match the build; rebuild")
    if source_identity()["sha256"] != identity["source"]["sha256"]:
        raise RuntimeError("renderer/core or renderer/acceptance/project changed since this build; rebuild")
    verify_stages(build, identity)
    return identity


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--godot", type=Path, help="the pinned non-console or console exe; neither is started")
    parser.add_argument("--verify", type=Path, metavar="BUILD", help="check a prepared build; build nothing")
    parser.add_argument("--route", choices=ROUTES, help="build the texture host (default) or route B's (C9 U4)")
    parser.add_argument("--stages", type=Path, metavar="DIR",
                        help="also build the user-stage pair in DIR, renderer/acceptance/stages (C9 U5c)")
    parser.add_argument("--dxc", type=Path, metavar="DIR",
                        help="with --stages: the folder of dxcompiler.dll and dxil.dll (default: the toolchain's dxc)")
    args = parser.parse_args(argv)
    if args.verify and (args.route or args.stages or args.dxc):
        parser.error("--route, --stages and --dxc apply to a build; --verify reads them from the build identity")
    if args.dxc and not args.stages:
        parser.error("--dxc applies with --stages")
    if os.name != "nt":
        parser.exit(1, "the C9 acceptance host requires the owner's Windows build tools\n")
    try:
        executable, nupkgs = locate_without_launch(args.godot)
        if args.verify:
            identity = verify(args.verify, executable)
            stages = identity.get("stages")
            staged = ("no user stages" if stages is None else
                      f"user stages {stages['vertex']['digest'][:12]}/{stages['fragment']['digest'][:12]}"
                      f" overdraw {stages['overdraw']} DXC {stages['dxc']['version']}")
            print(f"build verified: source {identity['source']['sha256'][:12]} at {identity['source']['head'][:12]}"
                  f"{'' if identity['source']['matches_head'] else ' (uncommitted source)'}; "
                  f"route {identity.get('route', 'texture')}; {staged}; {len(identity['parts'])} parts unchanged")
        else:
            print(build_host(executable, nupkgs, args.route or "texture", args.stages, args.dxc))
        return 0
    except (OSError, ValueError, KeyError, RuntimeError, subprocess.SubprocessError) as error:
        print(f"C9 acceptance host preparation failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
