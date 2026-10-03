"""Offline owner-machine build, deployment and ordered Qt interop run matrix."""
import sys
sys.dont_write_bytecode = True

import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import shutil
import subprocess

from smoke_summary import PHRASES, selftest_status, write_summary

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
QT = ROOT / "tools/qt/6.10.3/msvc2022_64"
NATIVE = ROOT / "work/experiments/renderer-sa2/native"
SOURCE_ROOTS = (HERE, NATIVE)
EXCLUDED = {"results", "build", "bin", "obj", "deploy", "CMakeFiles", "__pycache__"}
DRAIN_LINE = "sa2: drain not confirmed; ending the process with exit code 3"
# Observations a recorded probe row may report. Any other reason means the run
# did not exercise its combination (setup, identity, teardown or harness failure).
RECORDED_REASONS = frozenset({"debug-errors", "decode-mismatch", "verify-mismatch", "unverified-frames",
                              "eligible-coverage", "readback-missing"})


def matrix(device_loss=False):
    rows = [dict(id="Q0", expected="pass", timeout_s=300),
            dict(id="Q0b", expected="pass", timeout_s=300)]
    specs = [
        ("Q1", "qt", "rhi-upload", "same", "tracked", "legacy", True, "threaded", 1200, 200, "pass"),
        ("Q2", "qt", "import-copy", "same", "tracked", "legacy", True, "threaded", 1200, 200, "pass"),
        ("Q3", "qt", "import-copy", "own", "tracked", "legacy", True, "threaded", 1200, 200, "pass"),
        ("Q4", "qt", "export-copy", "same", "tracked", "legacy", True, "threaded", 1200, 200, "pass"),
        ("Q5", "qt", "import-direct", "same", "tracked", "legacy", True, "threaded", 1200, 200, "pass"),
        ("Q6", "from-rhi", "import-copy", "same", "tracked", "legacy", True, "threaded", 1200, 200, "pass"),
        ("Q7", "from-rhi", "import-copy", "own", "tracked", "legacy", True, "threaded", 1200, 200, "pass"),
        ("Q8", "from-rhi", "import-direct", "same", "tracked", "legacy", True, "basic", 1200, 200, "pass"),
        ("Q9", "from-device", "import-copy", "same", "tracked", "legacy", True, "threaded", 1200, 200, "pass"),
        ("Q10", "qt", "import-copy", "same", "declared", "legacy", True, "threaded", 300, 0, "recorded"),
        ("Q11", "from-rhi", "import-copy", "own", "tracked", "legacy", False, "threaded", 3000, 500, "pass"),
        ("Q12", "qt", "import-copy", "own", "tracked", "match", True, "threaded", 1200, 0, "recorded"),
        ("Q13", "from-rhi", "import-copy", "own", "tracked", "legacy", True, "threaded", 120, 0, "pending"),
    ]
    if device_loss:
        specs += [("Q14", "qt", "import-copy", "own", "tracked", "legacy", True, "threaded", 400, 0, "bounded-loss"),
                  ("Q15", "from-rhi", "import-copy", "own", "tracked", "legacy", True, "threaded", 400, 0, "bounded-loss")]
    keys = ("id", "device", "route", "queue", "handover", "barriers", "debug_layer",
            "render_loop", "frames", "resize_every", "expected")
    for spec in specs:
        row = dict(zip(keys, spec))
        row.update(timeout_s=120 if row["expected"] == "bounded-loss" else 300,
                   device_loss_at=200 if row["expected"] == "bounded-loss" else 0,
                   inject_drain=row["id"] == "Q13")
        rows.append(row)
    return rows


def commands(row, build, private):
    build, private = Path(build), Path(private)
    if row["id"] == "Q0":
        return [str(build / "native/sa2_selftest.exe"), "--hardware", "--debug", "--out", str(private / "Q0.json")]
    if row["id"] == "Q0b":
        return [str(build / "app/sd_code_layout_test.exe")]
    cmd = [str(build / "deploy/sd_smoke.exe"), "--sd-out", str(private / (row["id"] + ".json")),
           "--sd-dll", str(build / "native/sa2_interop.dll")]
    for key in ("device", "route", "queue", "handover", "barriers", "frames", "resize_every", "device_loss_at"):
        cmd += ["--sd-" + key.replace("_", "-"), str(row[key])]
    cmd += ["--sd-verify-every", "50", "--sd-timeout-ms", "5000"]
    if row["debug_layer"]:
        cmd += ["--sd-debug-layer"]
    return cmd


def sha256(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def excluded(root, parts):
    """True for a root-relative file path under an excluded directory or a native build folder."""
    directories = parts[:-1]
    return (any(part in EXCLUDED for part in directories)
            or (root == NATIVE and bool(directories) and directories[0].startswith("build")))


def source_files():
    return sorted((path for root in SOURCE_ROOTS for path in root.rglob("*")
                   if path.is_file() and not excluded(root, path.relative_to(root).parts)),
                  key=lambda path: path.relative_to(ROOT).as_posix())


def diff_pathspecs():
    """Both source roots for `git diff`, minus every directory the source walk excludes."""
    specs = []
    for root in SOURCE_ROOTS:
        base = root.relative_to(ROOT).as_posix()
        specs.append(base)
        for name in sorted(EXCLUDED):
            specs += [f":(exclude){base}/{name}", f":(exclude,glob){base}/**/{name}/**"]
    return specs + [f":(exclude,glob){NATIVE.relative_to(ROOT).as_posix()}/build*/**"]


def git_environment():
    env = os.environ.copy()
    env["GIT_OPTIONAL_LOCKS"] = "0"
    return env


def source_identity():
    paths = source_files()
    aggregate = hashlib.sha256()
    for path in paths:
        aggregate.update(path.relative_to(ROOT).as_posix().encode("utf-8") + b"\0")
        aggregate.update(sha256(path).encode("ascii") + b"\n")
    env = git_environment()
    head = subprocess.check_output(["git", "--no-optional-locks", "rev-parse", "HEAD"], cwd=ROOT, env=env, text=True).strip()
    tracked = set()
    for root in SOURCE_ROOTS:
        base = root.relative_to(ROOT).as_posix()
        names = subprocess.check_output(["git", "--no-optional-locks", "ls-files", "-z", "--", base],
                                        cwd=ROOT, env=env).decode("utf-8").split("\0")
        tracked |= {name for name in names if name and not excluded(root, PurePosixPath(name).relative_to(base).parts)}
    walked = {path.relative_to(ROOT).as_posix() for path in paths}
    diff = subprocess.run(["git", "--no-optional-locks", "diff", "--quiet", "HEAD", "--", *diff_pathspecs()],
                          cwd=ROOT, env=env, check=False)
    if diff.returncode not in (0, 1):
        raise ValueError("source identity diff failed")
    return dict(head=head, sha256=aggregate.hexdigest(), matches_head=walked == tracked and diff.returncode == 0)


def clean_environment():
    env = os.environ.copy()
    for key in ("QSG_RHI_DEBUG_LAYER", "M600_SA2_INJECT_UNCONFIRMED_DRAIN", "QT_PLUGIN_PATH",
                "QT_QPA_PLATFORM_PLUGIN_PATH", "QML_IMPORT_PATH", "QML2_IMPORT_PATH", "QSG_RHI_BACKEND"):
        env.pop(key, None)
    paths = []
    for entry in env.get("PATH", "").split(os.pathsep):
        path = Path(entry.strip('"'))
        if entry and not ((path / "qmake.exe").is_file() or (path / "Qt6Core.dll").is_file()
                          or "/qt/" in str(path).replace("\\", "/").lower()):
            paths.append(entry)
    env["PATH"] = os.pathsep.join(paths)
    env["QT_ENABLE_HIGHDPI_SCALING"] = "0"
    env["QT_LOGGING_RULES"] = "qt.rhi.general=true;qt.scenegraph.general=true"
    # Without a console window Qt logs to OutputDebugString, and the stderr counts would read 0.
    env["QT_FORCE_STDERR_LOGGING"] = "1"
    return env


def deploy_environment(env):
    """windeployqt adds the VC redistributable installer when VCINSTALLDIR is set (a developer prompt)."""
    env = dict(env)
    env.pop("VCINSTALLDIR", None)
    return env


def run_process(cmd, cwd, env, timeout, log_base):
    """File-backed capture avoids pipe blockage; kill only the owned PID tree."""
    with log_base.with_suffix(".stdout.log").open("wb") as stdout, log_base.with_suffix(".stderr.log").open("wb") as stderr:
        proc = subprocess.Popen(cmd, cwd=cwd, env=env, stdout=stdout, stderr=stderr)
        timed_out = False
        try:
            proc.wait(timeout=timeout)
        except subprocess.TimeoutExpired:
            timed_out = True
            subprocess.run(["taskkill", "/PID", str(proc.pid), "/T", "/F"],
                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=30, check=False)
            # The fallback targets the same owned process, never a process name.
            if proc.poll() is None:
                proc.kill()
            proc.wait(timeout=30)
    text = log_base.with_suffix(".stderr.log").read_text(encoding="utf-8", errors="replace")
    lines = text.splitlines()
    counts = {"total": len(lines), **{phrase: sum(phrase in line for line in lines) for phrase in PHRASES}}
    return proc.returncode, timed_out, counts, DRAIN_LINE in lines


def atomic_json(destination, value):
    temporary = destination.with_name(destination.name + ".tmp")
    temporary.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")
    os.replace(temporary, destination)


def build_command(script, destination):
    # A string, not a list: list2cmdline would escape the quotes as \", which cmd.exe does not understand.
    return f'cmd.exe /d /c call "{script}" "{destination}"'


def deployment_manifest(build):
    files = [build / "native/sa2_interop.dll", build / "native/sa2_selftest.exe",
             build / "app/sd_code_layout_test.exe", build / "app/sd_smoke.exe"]
    files += sorted(path for path in (build / "deploy").rglob("*") if path.is_file())
    return {path.relative_to(build).as_posix(): sha256(path) for path in files}


def reuse_build(source, qt_version):
    base = ROOT / "work/sdb"
    for build in sorted(base.iterdir(), reverse=True) if base.is_dir() else []:
        if not build.is_dir() or build.is_symlink():
            continue
        try:
            identity = json.loads((build / "build_identity.json").read_text(encoding="utf-8"))
            if identity["source"] != source or identity["qt_version"] != qt_version:
                continue
            artifacts = identity["artifacts"]
            if not artifacts or artifacts != deployment_manifest(build):
                continue
            if identity["dll_sha256"] != sha256(build / "native/sa2_interop.dll"):
                continue
            if identity["executable_sha256"] != sha256(build / "deploy/sd_smoke.exe"):
                continue
            return build, identity
        except (OSError, ValueError, KeyError):
            continue
    raise ValueError("no matching verified build under work/sdb")


def build_all(build, private, env, source):
    if len(str(build / "native")) > 150:
        raise ValueError("native build path exceeds 150 characters; run from a shorter checkout")
    build.mkdir(parents=True)
    for name, script, target in (("native-build", NATIVE / "build.cmd", build / "native"),
                                 ("app-build", HERE / "build.cmd", build / "app")):
        code, timeout, _, _ = run_process(build_command(script, target), ROOT, env, 900, private / name)
        if code != 0 or timeout:
            raise ValueError(name + " failed; see the private build log")
    deploy = build / "deploy"
    deploy.mkdir()
    shutil.copy2(build / "app/sd_smoke.exe", deploy / "sd_smoke.exe")
    command = [str(QT / "bin/windeployqt.exe"), "--release", "--no-translations", "--no-system-d3d-compiler",
               "--no-opengl-sw", "--no-quick-import", "--dir", str(deploy), str(deploy / "sd_smoke.exe")]
    code, timeout, _, _ = run_process(command, ROOT, deploy_environment(env), 300, private / "deploy")
    if code != 0 or timeout:
        raise ValueError("Qt deployment failed; see the private build log")
    if source_identity() != source:
        raise ValueError("source changed during build")
    paths = [p for p in deploy.rglob("*") if p.is_file()]
    identity = dict(source=source, qt_version="6.10.3", dll_sha256=sha256(build / "native/sa2_interop.dll"),
                    executable_sha256=sha256(deploy / "sd_smoke.exe"), artifacts=deployment_manifest(build),
                    deployment=dict(total_bytes=sum(p.stat().st_size for p in paths), file_count=len(paths)))
    atomic_json(build / "build_identity.json", identity)
    return identity


def teardown_complete(result):
    teardown = result.get("teardown", {})
    return (teardown.get("phase") == "complete" and teardown.get("drain_confirmed") is True
            and teardown.get("detached") is True)


def judge(row, result, exit_code, timed_out, drain_line, expect_adapter):
    if timed_out:
        return "unexpected"
    # Device loss is an observation: even an abort or a missing result is evidence.
    if row["expected"] == "bounded-loss":
        return "as-expected"
    if row["id"] in ("Q0", "Q0b"):
        # Q0: every check passes or is unsupported with a reason, and at least one passes.
        ok = exit_code == 0 and (row["id"] == "Q0b" or selftest_status(result) == "pass")
        return "as-expected" if ok else "unexpected"
    if not result or result.get("format") != "magic600-sd-smoke-run-v1":
        return "unexpected"
    if result.get("device", {}).get("adapter_name") != expect_adapter:
        return "unexpected"
    if row["expected"] == "pending":
        ok = (exit_code == 3 and drain_line and result.get("teardown", {}).get("phase") == "pending"
              and result.get("frames", {}).get("run") == row["frames"] and not result.get("sa2_failures"))
    else:
        ok = exit_code == 0 and result.get("status") == row["expected"]
        if row["expected"] == "recorded":
            # A probe row always reports recorded with exit 0, so it must also have run every
            # requested frame, torn down completely and failed only by observation.
            reasons = result.get("reasons", [])
            ok = (ok and result.get("frames", {}).get("run") == row["frames"] and teardown_complete(result)
                  and isinstance(reasons, list) and set(reasons) <= RECORDED_REASONS)
    return "as-expected" if ok else "unexpected"


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--only")
    parser.add_argument("--device-loss", action="store_true")
    parser.add_argument("--skip-build", action="store_true")
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument("--write-summary", action="store_true")
    parser.add_argument("--expect-adapter", default="NVIDIA GeForce RTX 4070 Laptop GPU")
    args = parser.parse_args(argv)
    rows = matrix(args.device_loss)
    if args.only:
        selected = args.only.split(",")
        if any(name not in {row["id"] for row in rows} for name in selected):
            parser.error("unknown run or device-loss run without --device-loss")
        rows = [row for row in rows if row["id"] in {"Q0", "Q0b", *selected}]
    if args.dry_run:
        print("BUILD: cmd.exe /d /c call <repository>/work/experiments/renderer-sa2/native/build.cmd <build>/native")
        print("BUILD: cmd.exe /d /c call <repository>/work/experiments/renderer-sd/build.cmd <build>/app")
        print("DEPLOY: windeployqt --release --no-translations --no-system-d3d-compiler --no-opengl-sw --no-quick-import --dir <build>/deploy <build>/deploy/sd_smoke.exe")
        for row in rows:
            print(row["id"], json.dumps(row, sort_keys=True))
            print("COMMAND:", subprocess.list2cmdline(commands(row, "<build>", "<private>")))
        return 0
    if os.name != "nt":
        raise ValueError("owner runtime requires Windows")
    env = clean_environment()
    env["PATH"] = str(QT / "bin") + os.pathsep + env["PATH"]
    version = subprocess.check_output([str(QT / "bin/qmake.exe"), "-query", "QT_VERSION"],
                                      env=env, timeout=30, text=True).strip()
    if version != "6.10.3":
        raise ValueError("Qt version must be exactly 6.10.3")
    source = source_identity()
    stamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%S%fZ")
    private = ROOT / "work/loop-memory/perf/renderer/sd-smoke" / stamp
    private.mkdir(parents=True)
    if args.skip_build:
        build, identity = reuse_build(source, version)
    else:
        build = ROOT / "work/sdb" / stamp
        identity = build_all(build, private, env, source)
    atomic_json(private / "build_identity.json", identity)
    record = {key: identity[key] for key in ("source", "qt_version", "dll_sha256", "executable_sha256", "deployment")}
    record["evidence_kind"] = "owner-runtime"
    record["runs"] = []
    for row in rows:
        if source_identity() != source or deployment_manifest(build) != identity["artifacts"]:
            raise ValueError("build inputs or artifacts changed before run")
        run_env = clean_environment()
        run_env["PATH"] = str(build / "deploy") + os.pathsep + run_env["PATH"]
        run_env["QSG_RENDER_LOOP"] = row.get("render_loop", "threaded")
        if row.get("device") == "qt" and row.get("debug_layer"):
            run_env["QSG_RHI_DEBUG_LAYER"] = "1"
        if row.get("inject_drain"):
            run_env["M600_SA2_INJECT_UNCONFIRMED_DRAIN"] = "1"
        code, timeout, counts, drain_line = run_process(commands(row, build, private), build / "deploy",
                                                       run_env, row["timeout_s"], private / row["id"])
        result_file = private / (row["id"] + ".json")
        exists = result_file.is_file()
        result = None
        if exists:
            try:
                result = json.loads(result_file.read_text(encoding="utf-8-sig"))
                if not isinstance(result, dict):
                    result = None
            except (ValueError, OSError):
                pass
        if row["id"] == "Q0b":
            result = dict(status="pass" if code == 0 else "fail", reasons=[])
        judgement = judge(row, result, code, timeout, drain_line, args.expect_adapter)
        record["runs"].append(dict(row=row, result=result, result_exists=exists, exit_code=code,
                                   timeout=timeout, stderr=counts, judgement=judgement))
        atomic_json(private / "private_summary.json", record)
        print(row["id"] + ": " + judgement + (" (timeout)" if timeout else f" (exit {code})"))
        if row["id"] in ("Q0", "Q0b") and judgement != "as-expected":
            break
    if args.write_summary:
        write_summary(record, HERE / "results/sd-smoke-summary.json")
    return 0 if all(run["judgement"] == "as-expected" for run in record["runs"]) else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        print(f"sd smoke: {error}", file=sys.stderr)
        sys.exit(1)
