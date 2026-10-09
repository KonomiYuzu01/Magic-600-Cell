"""Offline owner-machine runner for the SA2 level-1 interop matrix."""

import argparse
import datetime as dt
import hashlib
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path, PurePosixPath

sys.dont_write_bytecode = True
from smoke_summary import GODOT_VERSION, selftest_status, write_summary

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
EXPERIMENT = "work/experiments/renderer-sa2"
PRIVATE_ROOT = ROOT / "work/loop-memory/perf/renderer/sa2-smoke"
# cl.exe is not long-path aware: CMake's try-compile objects under PRIVATE_ROOT
# pass 260 characters in a long checkout path. Binaries are built in a short
# directory of the ignored work tree; logs and results stay in PRIVATE_ROOT.
BUILD_ROOT = ROOT / "work/sa2b"
BUILD_PATH_LIMIT = 150
PACKAGES = {"godot.net.sdk", "godot.sourcegenerators", "godotsharp", "godotsharpeditor"}
DRAIN_LINE = "sa2: drain not confirmed; ending the process with exit code 3"
INJECT = "M600_SA2_INJECT_UNCONFIRMED_DRAIN"
# Directories excluded from the source identity at any depth; native/build*/
# (the manual build folders of native/README.md) are excluded as well.
EXCLUDED = {"results", ".godot", "bin", "obj", "__pycache__", ".sandbox-build"}


def matrix(device_loss=False):
    rows = [{"id": "R0", "route": "native-selftest", "expected": "pass"}]

    def add(number, route="export", queue="same", handover="tracked", barriers="match",
            validation=True, render_thread="safe", frames=1200, resize_every=200, expected="pass", **extra):
        rows.append(dict(id=f"R{number}", route=route, queue=queue, handover=handover, barriers=barriers,
                         validation=validation, render_thread=render_thread, frames=frames,
                         resize_every=resize_every, expected=expected, **extra))
    add(1, "rd-compute")
    add(2)
    add(3, queue="own")
    add(4, "import-copy")
    add(5, "import-copy", queue="own")
    add(6, "import-texture2drd", frames=300, resize_every=0, expected="unsupported")
    add(7, handover="render-target", frames=300, resize_every=0, expected="recorded")
    add(8, "import-copy", handover="render-target", frames=300, resize_every=0, expected="recorded")
    add(9, queue="own", validation=False, frames=3000, resize_every=500)
    add(10, queue="own", render_thread="separate")
    add(11, queue="own", barriers="legacy", resize_every=0, expected="recorded")
    add(12, queue="own", frames=120, resize_every=0, expected="pending-drain")
    if device_loss:
        add(13, queue="own", frames=400, resize_every=0, expected="device-loss", device_loss_at=200)
    return rows


def commands(row, godot, project, native, result, log):
    if row["id"] == "R0":
        return [str(Path(native) / "sa2_selftest.exe"), "--hardware", "--debug", "--out", str(result)]
    command = [str(godot), "--path", str(project), "--rendering-driver", "d3d12", "--windowed",
               "--resolution", "1280x720", "--position", "40,40", "--disable-vsync", "--log-file", str(log),
               "--render-thread", row["render_thread"]]
    if row["validation"]:
        command.append("--gpu-validation")
    command += ["--", "--sa2-out", str(result), "--sa2-dll", str(Path(native) / "sa2_interop.dll"),
                "--sa2-route", row["route"], "--sa2-queue", row["queue"], "--sa2-handover", row["handover"],
                "--sa2-barriers", row["barriers"], "--sa2-frames", str(row["frames"]),
                "--sa2-warmup", "0" if row["handover"] == "render-target" or row["route"] == "rd-compute" else "3",
                "--sa2-resize-every", str(row["resize_every"]), "--sa2-verify-every", "50",
                "--sa2-device-loss-at", str(row.get("device_loss_at", 0)), "--sa2-timeout-ms", "5000",
                # Godot consumes --gpu-validation and --render-thread before the harness
                # can see them, so the runner declares both again as user arguments.
                "--sa2-gpu-validation", "1" if row["validation"] else "0",
                "--sa2-render-thread", row["render_thread"]]
    return command


def run_environment(base_env, run_id):
    """One run's environment: only R12 gets the drain injection, even if the base inherited it."""
    env = {key: value for key, value in base_env.items() if key.upper() != INJECT}
    if run_id == "R12":
        env[INJECT] = "1"
    return env


def sha256(path):
    with Path(path).open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def excluded(parts):
    """True for an experiment-relative file path under an excluded directory."""
    directories = parts[:-1]
    return (any(part in EXCLUDED for part in directories)
            or (len(directories) > 1 and directories[0] == "native" and directories[1].startswith("build")))


def source_files():
    return sorted((p for p in HERE.rglob("*") if p.is_file() and not excluded(p.relative_to(HERE).parts)),
                  key=lambda p: p.relative_to(HERE).as_posix())


def diff_pathspecs():
    """The experiment for `git diff`, minus every directory the source walk excludes."""
    specs = [EXPERIMENT]
    for name in sorted(EXCLUDED):
        specs += [f":(exclude){EXPERIMENT}/{name}", f":(exclude,glob){EXPERIMENT}/**/{name}/**"]
    return specs + [f":(exclude,glob){EXPERIMENT}/native/build*/**"]


def git(*args):
    env = os.environ.copy()
    env["GIT_OPTIONAL_LOCKS"] = "0"
    completed = subprocess.run(["git", "--no-optional-locks", *args], cwd=ROOT, env=env,
                               capture_output=True, text=True, check=True, timeout=30)
    return completed.stdout.strip()


def source_identity():
    files = source_files()
    digest = hashlib.sha256()
    for path in files:
        digest.update(path.relative_to(HERE).as_posix().encode("utf-8") + b"\0")
        digest.update(sha256(path).encode("ascii") + b"\n")
    tracked = {name for name in git("ls-files", "-z", "--", EXPERIMENT).split("\0")
               if name and not excluded(PurePosixPath(name).relative_to(EXPERIMENT).parts)}
    walked = {p.relative_to(ROOT).as_posix() for p in files}
    env = os.environ.copy()
    env["GIT_OPTIONAL_LOCKS"] = "0"
    changed = subprocess.run(["git", "--no-optional-locks", "diff", "--quiet", "HEAD", "--", *diff_pathspecs()],
                             cwd=ROOT, env=env, timeout=30).returncode
    if changed not in (0, 1):
        raise RuntimeError("cannot compare source with HEAD")
    return {"head": git("rev-parse", "HEAD"), "sha256": digest.hexdigest(),
            "matches_head": tracked == walked and changed == 0, "file_count": len(files)}


def locate_godot(explicit=None):
    if explicit:
        path = Path(explicit).resolve()
    else:
        base = os.environ.get("LOCALAPPDATA")
        if not base:
            raise RuntimeError("LOCALAPPDATA is unavailable; pass --godot")
        matches = list((Path(base) / "Microsoft/WinGet/Packages").glob(
            "GodotEngine.GodotEngine.Mono_*/Godot_v4.7.2-stable_mono_win64/Godot_v4.7.2-stable_mono_win64_console.exe"))
        if len(matches) != 1:
            raise RuntimeError("expected exactly one Godot installation; pass --godot")
        path = matches[0].resolve()
    version = subprocess.run([str(path), "--version"], capture_output=True, text=True, check=True, timeout=30).stdout.strip()
    if version != GODOT_VERSION:
        raise RuntimeError(f"Godot version is {version!r}, expected {GODOT_VERSION!r}")
    nupkgs = path.parent / "GodotSharp/Tools/nupkgs"
    if not nupkgs.is_dir():
        raise RuntimeError("the installation's offline NuGet folder is missing")
    return path, nupkgs, version


def write_json(path, value):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(path.name + ".tmp")
    temporary.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")
    temporary.replace(path)


def run_process(command, cwd, env, timeout, prefix):
    prefix = Path(prefix)
    stdout_path = prefix.with_suffix(".stdout.txt")
    stderr_path = prefix.with_suffix(".stderr.txt")
    timed_out = False
    with stdout_path.open("wb") as stdout, stderr_path.open("wb") as stderr:
        process = subprocess.Popen(command, cwd=cwd, env=env, stdout=stdout, stderr=stderr)
        try:
            process.wait(timeout=timeout)
        except subprocess.TimeoutExpired:
            timed_out = True
            if os.name == "nt":
                killed = subprocess.run(["taskkill", "/PID", str(process.pid), "/T", "/F"],
                                        capture_output=True, timeout=15)
                if killed.returncode != 0 and process.poll() is None:
                    process.kill()
            else:
                process.kill()
            process.wait(timeout=15)
    output = stdout_path.read_text(encoding="utf-8", errors="replace")
    errors = stderr_path.read_text(encoding="utf-8", errors="replace")
    return {"exit_code": process.returncode, "timed_out": timed_out,
            "stdout": str(stdout_path), "stderr": str(stderr_path),
            "output_line_counts": {"error": sum("ERROR:" in line for line in (output + "\n" + errors).splitlines()),
                                   "warning": sum("WARNING:" in line for line in (output + "\n" + errors).splitlines())}}, output, errors


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


def build(private, nupkgs, source, godot_version):
    native = BUILD_ROOT / private.name / "native"
    if len(str(native)) > BUILD_PATH_LIMIT:
        raise RuntimeError("checkout path too long for the native build; use a shorter checkout path")
    native.mkdir(parents=True)
    script = HERE / "native/build.cmd"
    if not script.is_file():
        raise RuntimeError("native/build.cmd is absent; integrate packet SA2-N first")
    # Always quote both cmd paths; neither path is composed from user text.
    if any(c in str(script) + str(native) for c in ('"', '\r', '\n')):
        raise RuntimeError("unsupported build path")
    native_command = f'cmd.exe /d /c call "{script}" "{native}"'
    env = offline_environment(private, nupkgs)
    native_run, _, _ = run_process(native_command, HERE, env, 300, private / "native-build-log")
    if native_run["exit_code"] != 0 or native_run["timed_out"]:
        raise RuntimeError("native build failed; see private native-build-log files")
    project = native.parent / "project"
    shutil.copytree(HERE / "project", project, ignore=shutil.ignore_patterns(".godot", "bin", "obj", ".sandbox-build"))
    command = ["dotnet", "build", "SA2Smoke.csproj", "-c", "Debug", "-nologo", "-nodeReuse:false", "-p:UseSharedCompilation=false"]
    managed_run, _, _ = run_process(command, project, env, 300, private / "dotnet-build-log")
    if managed_run["exit_code"] != 0 or managed_run["timed_out"]:
        raise RuntimeError("offline .NET build failed; see private dotnet-build-log files")
    check_package_cache(private / "packages")
    assembly = project / ".godot/mono/temp/bin/Debug/SA2Smoke.dll"
    dll = native / "sa2_interop.dll"
    if not (native / "sa2_selftest.exe").is_file():
        raise RuntimeError("native self-test was not built in the native build root")
    after = source_identity()
    if source != after:
        raise RuntimeError("source changed during the build; discard this build")
    identity = {"source": source, "dll_sha256": sha256(dll), "assembly_sha256": sha256(assembly),
                "godot_version": godot_version, "project": str(project), "native": str(native)}
    write_json(private / "build-identity.json", identity)
    return identity


def reuse_build(source, version):
    identities = sorted(PRIVATE_ROOT.glob("*/build-identity.json"), reverse=True)
    if not identities:
        raise RuntimeError("no private build exists for --skip-build")
    identity = json.loads(identities[0].read_text(encoding="utf-8"))
    if identity["source"] != source or identity["godot_version"] != version:
        raise RuntimeError("newest private build does not match current source/Godot; rebuild")
    native, project = Path(identity["native"]).resolve(), Path(identity["project"]).resolve()
    if not native.is_relative_to(BUILD_ROOT.resolve()) or not project.is_relative_to(BUILD_ROOT.resolve()):
        raise RuntimeError("reuse build is outside the build root")
    if sha256(native / "sa2_interop.dll") != identity["dll_sha256"] or sha256(project / ".godot/mono/temp/bin/Debug/SA2Smoke.dll") != identity["assembly_sha256"]:
        raise RuntimeError("private build binary digest changed; rebuild")
    return identity


def teardown_ok(result):
    teardown = result.get("teardown", {})
    return (teardown.get("phase") == "complete" and teardown.get("drain_result") == 0
            and teardown.get("detached") is True
            and all(entry.get("refcount_after") == 0 and entry.get("status") == 0
                    for entry in teardown.get("refcount_after", [])))


def _section(result, key):
    value = result.get(key)
    return value if isinstance(value, dict) else {}


def pass_evidence(row, result):
    f, v, d = (result.get(key, {}) for key in ("frames", "verify", "debug"))
    run = f.get("run", 0)
    # The harness resizes after every resize_every-th run frame except the last, and
    # each size differs from the one before, so every request rebuilds the ring once.
    every = row["resize_every"]
    return (run == row["frames"] and f.get("produced") == run and f.get("drawn") == run
            and f.get("not_drawn") == 0 and f.get("mismatched") == 0
            and f.get("eligible", 0) * 10 >= run * 9 and f.get("verified") == f.get("eligible")
            and f.get("readbacks_requested") == f.get("readbacks_completed") == f.get("eligible")
            and v.get("rd_checks") == run // 50
            and v.get("native_checks") == (0 if row["route"] == "rd-compute" else run // 50)
            and v.get("mismatched_texels") == 0 and not result.get("sa2_failures", [])
            and (not row["validation"] or d.get("error") == d.get("corruption") == 0)
            and (every <= 0 or _section(result, "resize").get("rebuilds") == (row["frames"] - 1) // every)
            and teardown_ok(result))


def config_mismatches(row, result):
    """Row fields that the result's `config` echo does not report exactly (type and value)."""
    config = _section(result, "config")
    expected = {key: row[key] for key in ("route", "queue", "handover", "barriers", "frames", "resize_every")}
    if row["route"] == "rd-compute":
        del expected["handover"]  # the packet defines no handover for the compute baseline (R1)
    expected.update(gpu_validation=row["validation"], render_thread=row["render_thread"],
                    render_thread_separate_observed=row["render_thread"] == "separate")
    return [key for key, value in expected.items()
            if type(config.get(key)) is not type(value) or config.get(key) != value]


def judge(row, run, expect_adapter, stdout="", stderr=""):
    result = run.get("result") or {}
    reasons = []
    # Godot's own ERROR:/WARNING: output lines are recorded data, not a pass
    # condition (packet SA2-G pass rule); a non-gating note flags ERROR lines.
    errors = (run.get("output_line_counts") or {}).get("error") or 0
    run["judge_notes"] = ["godot-output-errors"] if errors > 0 else []
    if run.get("timed_out"):
        return "unexpected", ["timeout"]
    adapter = result.get("engine", {}).get("adapter_name") or result.get("adapter_name")
    # An abort can leave only the startup log; do not invent an adapter name.
    if not adapter and expect_adapter in stdout + stderr:
        adapter = expect_adapter
    run["adapter_observed"] = adapter
    if adapter and adapter != expect_adapter:
        reasons.append("adapter-mismatch")
    if row["id"] == "R0":
        # Every check passes or is unsupported with a reason, and at least one passes.
        ok = run.get("exit_code") == 0 and selftest_status(result) == "pass"
        if not ok:
            reasons.append("native-selftest-failed")
    elif row["expected"] == "device-loss":
        # R13 explicitly accepts an engine abort, any exit, and a missing file.
        ok = True
    else:
        ok = result.get("format") == "magic600-sa2-smoke-run-v1"
        if not adapter:
            reasons.append("adapter-mismatch")
        run["config_mismatches"] = config_mismatches(row, result)
        if run["config_mismatches"]:
            reasons.append("config-mismatch")
        # Validation counts prove nothing unless the debug layer was really active.
        if row["validation"] and _section(result, "device").get("debug_layer") != 1:
            reasons.append("validation-not-active")
        if row["expected"] in ("recorded", "pending-drain"):
            ok = ok and _section(result, "frames").get("run") == row["frames"]
        if row["expected"] == "pending-drain":
            ok = ok and run.get("exit_code") == 3 and result.get("teardown", {}).get("phase") == "pending"
            if DRAIN_LINE not in stderr.splitlines():
                reasons.append("drain-message-missing")
                ok = False
        else:
            ok = ok and run.get("exit_code") == 0 and result.get("status") == row["expected"] and teardown_ok(result)
            if row["expected"] == "pass":
                ok = ok and pass_evidence(row, result)
            if row["expected"] == "unsupported":
                ok = ok and set(result.get("reasons", [])) == {"texture2drd-refused"}
        if not ok:
            reasons.append("expectation-mismatch")
    return ("as-expected" if ok and not reasons else "unexpected"), reasons


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--godot", type=Path)
    parser.add_argument("--expect-adapter", default="NVIDIA GeForce RTX 4070 Laptop GPU")
    parser.add_argument("--only", help="comma-separated run IDs, executed in matrix order")
    parser.add_argument("--device-loss", action="store_true")
    parser.add_argument("--skip-build", action="store_true")
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument("--write-summary", action="store_true")
    args = parser.parse_args(argv)
    rows = matrix(args.device_loss)
    if args.only:
        requested = args.only.split(",")
        known = {row["id"] for row in rows}
        if len(requested) != len(set(requested)) or not set(requested) <= known:
            parser.error("--only contains repeated/unknown IDs (R13 also requires --device-loss)")
        rows = [row for row in rows if row["id"] in requested]
    if args.dry_run:
        print("Offline build: cmd.exe /d /c call native\\build.cmd <build>\\native")
        print("Offline build: dotnet build SA2Smoke.csproj -c Debug -nologo -nodeReuse:false -p:UseSharedCompilation=false")
        for row in rows:
            print(f"{row['id']}: {json.dumps(row, sort_keys=True)}")
            print(subprocess.list2cmdline(commands(row, "<godot>", "<build>/project", "<build>/native",
                                                  f"<private>/{row['id']}.json", f"<private>/{row['id']}.godot.log")))
            if INJECT in run_environment({}, row["id"]):
                print(f"  environment: {INJECT}=1 (this run only)")
        return 0
    if os.name != "nt":
        parser.exit(1, "SA2 GPU/build runs require Windows; --dry-run is portable\n")
    private = None
    summary = {"format": "magic600-sa2-smoke-private-v1", "runs": []}
    try:
        godot, nupkgs, version = locate_godot(args.godot)
        summary["source"] = source_identity()
        stamp = dt.datetime.now(dt.timezone.utc).strftime("%Y%m%dT%H%M%S%fZ")
        private = PRIVATE_ROOT / stamp
        private.mkdir(parents=True)
        identity = reuse_build(summary["source"], version) if args.skip_build else build(private, nupkgs, summary["source"], version)
        summary["build"] = identity
        env = offline_environment(private, nupkgs)
        for row in rows:
            run_id = row["id"]
            result_path = private / f"{run_id}.json"
            command = commands(row, godot, identity["project"], identity["native"], result_path, private / f"{run_id}.godot.log")
            run, stdout, stderr = run_process(command, identity["project"], run_environment(env, run_id),
                                              120 if run_id == "R13" else 300, private / run_id)
            run.update(id=run_id, row=row, command=command, result_exists=result_path.is_file(), result=None)
            if result_path.is_file():
                try:
                    value = json.loads(result_path.read_text(encoding="utf-8-sig"))
                    if not isinstance(value, dict):
                        raise ValueError("result is not an object")
                    run["result"] = value
                except (ValueError, UnicodeError) as error:
                    run["result_error"] = str(error)
            run["judgement"], run["judge_reasons"] = judge(row, run, args.expect_adapter, stdout, stderr)
            summary["runs"].append(run)
            write_json(private / "private-summary.json", summary)
            print(f"{run_id}: {run['judgement']}; exit={run['exit_code']}; timeout={run['timed_out']}")
            if run_id == "R0" and run["judgement"] != "as-expected":
                break
        if args.write_summary:
            write_summary(summary, HERE / "results/sa2-smoke-summary.json")
        print(f"Private outputs: {private}")
        return 0 if all(run["judgement"] == "as-expected" for run in summary["runs"]) else 1
    except (OSError, ValueError, KeyError, RuntimeError, subprocess.SubprocessError) as error:
        if private is not None:
            summary["runner_error"] = str(error)
            write_json(private / "private-summary.json", summary)
        print(f"sa2 runner: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
