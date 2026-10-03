"""Packet SD-Q acceptance: source/static evidence and Python fixtures only."""
import sys
sys.dont_write_bytecode = True

import copy
import json
import os
from pathlib import Path
import re
import shutil
import stat
import subprocess
import tempfile
import uuid
from unittest.mock import patch

import run_smoke
import sd_reference as reference
import smoke_summary

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
LAYOUT = ROOT / "work/experiments/renderer-sa2/code_layout.json"
HEADER = ROOT / "work/experiments/renderer-sa2/native/include/sa2_interop.h"


def require(condition, message):
    if not condition:
        raise AssertionError(message)


def static_checks():
    required = ("app/CMakeLists.txt", "app/src/main.cpp", "app/src/smoke.h", "app/src/smoke.cpp",
                "app/src/native_loader.h", "app/src/native_loader.cpp", "app/src/code_layout.h",
                "app/src/code_layout.cpp", "app/src/code_layout_test.cpp", "build.cmd", "run_smoke.py",
                "smoke_summary.py", "sd_reference.py", "check_project.py", "README.md")
    for name in required:
        require((HERE / name).is_file(), "missing " + name)
    cmake = (HERE / "app/CMakeLists.txt").read_text(encoding="utf-8")
    for text in ('cmake_minimum_required(VERSION 3.30)', 'set(CMAKE_CXX_STANDARD 20)',
                 'find_package(Qt6 REQUIRED COMPONENTS Core Gui GuiPrivate Quick)',
                 'NOT Qt6_VERSION STREQUAL "6.10.3"', 'qt_standard_project_setup()',
                 'add_executable(sd_smoke ', 'add_executable(sd_code_layout_test ',
                 'Qt6::Core Qt6::Gui Qt6::GuiPrivate Qt6::Quick d3d12 dxgi dxguid', '../../renderer-sa2/native/include'):
        require(text in cmake, "CMake setting: " + text)
    require(not re.search(r"add_executable\(sd_smoke\s+WIN32", cmake), "console executable required")
    build = (HERE / "build.cmd").read_bytes()
    require(b"\r\n" in build and b"\n" not in build.replace(b"\r\n", b"") and b"\r" not in build.replace(b"\r\n", b""), "build.cmd must be CRLF only")
    require(b"http" not in build.lower(), "offline build only")
    for text in (b"vswhere", b"vcvars64.bat", b"renderer-spike", b"-G Ninja", b"-DCMAKE_BUILD_TYPE=Release", b"-DCMAKE_PREFIX_PATH", b"6.10.3"):
        require(text in build, "build script setting missing")
    header = HEADER.read_text(encoding="utf-8")
    exports = set(re.findall(r"SA2_API\s+\w+\s+(sa2_\w+)\(", header))
    loader = (HERE / "app/src/native_loader.cpp").read_text(encoding="utf-8")
    require(exports == set(re.findall(r"SD_BIND\((sa2_\w+)\)", loader)), "bind every ABI export exactly by name")
    require("GetProcAddress(module, #name)" in loader and "reinterpret_cast<decltype(&name)>" in loader, "ABI-derived function types")
    require("LoadLibraryW(absolutePath)" in loader and "fn_sa2_abi_version() != SA2_ABI_VERSION" in loader, "ABI check before other calls")
    structs = (HERE / "app/src/native_loader.h").read_text(encoding="utf-8")
    for declaration in ("sizeof(sa2_device_info) == 48", "sizeof(sa2_config) == 28", "sizeof(struct sa2_debug_counts) == 128"):
        require(declaration in structs, "ABI size assertion missing")
    defines = dict(re.findall(r"^#define\s+(SA2_\w+)\s+(\d+)u?\b", header, re.M))
    cpp = "\n".join(path.read_text(encoding="utf-8") for path in (HERE / "app/src").glob("*") if path.suffix in (".h", ".cpp"))
    constants = set(re.findall(r"\bSA2_(?:OK|E_\w+|QUEUE_\w+|BARRIERS_\w+|STATE_\w+)\b", cpp))
    require(constants <= defines.keys(), "unknown status/queue/barrier/state constant")
    require(not re.search(r"(?:#define|constexpr\s+\w+)\s+SA2_", cpp), "use header constants, not copies")
    layout = json.loads(LAYOUT.read_text(encoding="utf-8"))
    code_header = (HERE / "app/src/code_layout.h").read_text(encoding="utf-8")
    for field, macro, attribute in (("block_px", "SA2_CODE_BLOCK_PX", "BLOCK_PX"),
                                   ("grid_cols", "SA2_CODE_GRID_COLS", "GRID_COLS"),
                                   ("grid_rows", "SA2_CODE_GRID_ROWS", "GRID_ROWS"),
                                   ("min_texture_px", "SA2_MIN_TEXTURE_PX", "MIN_TEXTURE_PX"),
                                   ("ring_slots", "SA2_RING_SLOTS", "RING_SLOTS")):
        require(int(defines[macro]) == layout[field] == getattr(reference, attribute), "layout constant " + field)
        require(f"{attribute} = {macro}" in code_header, "C++ layout constant " + field)
    for field in layout["fields"]:
        name = field["name"].upper() + "_BITS"
        require(int(defines["SA2_CODE_" + name]) == field["bits"] == getattr(reference, name), "field width")
        require(f"{name} = SA2_CODE_{name}" in code_header, "C++ field width")
    require([field["first_bit"] for field in layout["fields"]] == [0, 32, 36, 48], "immutable field offsets")
    code_test = (HERE / "app/src/code_layout_test.cpp").read_text(encoding="utf-8")
    for vector in layout["test_vectors"]:
        expected = vector.get("code", vector.get("crc16"))
        require(expected.lower() in code_test.lower(), "C++ immutable test vector missing")
    smoke = (HERE / "app/src/smoke.cpp").read_text(encoding="utf-8")
    main = (HERE / "app/src/main.cpp").read_text(encoding="utf-8")
    for token in ("fromNative", "createFrom", "setNativeLayout", "copyTexture", "uploadTexture", "readBackTexture",
                  "currentFrameCommandBuffer", "sa2_signal_godot_free", "sa2_godot_wait_ready", "sa2_mark_shown",
                  "sa2_drain", "sa2_remove_device", 's.phase = "pending"', "std::chrono::seconds(10)"):
        require(token in smoke, "render protocol source missing " + token)
    pending = smoke.find('s.phase = "pending"')
    write, release = smoke.find("writeResult(false)", pending), smoke.find("releaseRing();", pending)
    require(0 <= pending < write < release, "pending result written before drain")
    for token in ("QQuickGraphicsDevice::fromRhi", "QQuickGraphicsDevice::fromDeviceAndContext",
                  "QQuickWindow::setGraphicsApi(QSGRendererInterface::Direct3D12)", "Qt::QueuedConnection",
                  "Qt::DirectConnection", "owned.release(harness)"):
        require(token in main, "window/lifetime route missing")
    forbidden = re.compile(r"[a-z]:[\\/][^\r\n]*[\\/]users[\\/]|[\\/]users[\\/]|app" + r"data\\", re.I)
    for path in HERE.rglob("*"):
        if path.is_file() and path.suffix in (".py", ".cpp", ".h", ".txt", ".cmd", ".md"):
            text = path.read_text(encoding="utf-8")
            require(not forbidden.search(text), "private path literal in " + path.name)
            if path.suffix == ".py":
                require("sys.dont_write_bytecode = True" in text, "bytecode side effect in " + path.name)
                compile(text, str(path), "exec")
    return layout


def reference_checks(layout):
    for vector in layout["test_vectors"]:
        if "input" in vector:
            require(reference.crc16(vector["input"].encode("ascii")) == int(vector["crc16"], 16), "CRC vector")
        else:
            fields = tuple(vector[key] for key in ("sequence", "slot", "generation"))
            require(reference.code(*fields) == int(vector["code"], 16), "code vector")
    for width, height in ((128, 128), (160, 144)):
        for fields in ((1, 0, 0), (3735928559, 2, 2748), (1000, 1, 7)):
            image = reference.expected_image(width, height, *fields)
            require(reference.decode(image, width, height) == fields, "round trip both corners")
            fill = bytes((32 + 64 * fields[1], fields[0] % 251, 16 * (fields[2] % 16), 255))
            require(image[64 * 4:65 * 4] == fill, "non-code fill texel")
            require(list(image[:4]) == layout["one_rgba8" if reference.code(*fields) & 1 else "zero_rgba8"], "code block color")
            corrupt = bytearray(image)
            # Flip an entire block, first in one corner, then in both corners (CRC check).
            for x0, y0 in ((0, 0), (width - 64, height - 64)):
                for y in range(8):
                    for x in range(8):
                        corrupt[((y0 + y) * width + x0 + x) * 4] ^= 255
                try:
                    reference.decode(corrupt, width, height)
                except ValueError:
                    pass
                else:
                    raise AssertionError("flipped block accepted")
            cw, ch = width + 43, height + 29
            composite = bytearray(bytes((17, 33, 99, 255)) * (cw * ch))
            for y in range(height):
                composite[y * cw * 4:(y * cw + width) * 4] = image[y * width * 4:(y + 1) * width * 4]
            # Different red/blue values at centers make format selection observable.
            for x0, y0 in ((0, 0), (width - 64, height - 64)):
                for bit in range(64):
                    center = ((y0 + (bit // 8) * 8 + 4) * cw + x0 + (bit % 8) * 8 + 4) * 4
                    composite[center + 2] = 255 - composite[center]
            require(reference.decode(composite, cw, ch, width, height, "RGBA8") == fields, "larger RGBA8 composite")
            for pixel in range(cw * ch):
                composite[4 * pixel], composite[4 * pixel + 2] = composite[4 * pixel + 2], composite[4 * pixel]
            require(reference.decode(composite, cw, ch, width, height, "BGRA8") == fields, "larger BGRA8 composite")
            try:
                reference.decode(composite, cw, ch, width, height, "RGBA8")
            except ValueError:
                pass
            else:
                raise AssertionError("wrong channel order accepted")
    require(reference.code(0x100000001, 16, 4096) == reference.code(1, 0, 0), "field wraparound")


def runner_checks():
    for loss in (False, True):
        command = [sys.executable, str(HERE / "run_smoke.py"), "--dry-run"] + (["--device-loss"] if loss else [])
        completed = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, timeout=30, check=False)
        require(completed.returncode == 0, "dry-run failed: " + completed.stderr)
        rows = [json.loads(line.split(" ", 1)[1]) for line in completed.stdout.splitlines()
                if re.match(r"Q\d+b? \{", line)]
        require(rows == run_smoke.matrix(loss), "dry-run matrix mismatch")
        require([row["id"] for row in rows] == ["Q0", "Q0b", *[f"Q{i}" for i in range(1, 16 if loss else 14)]], "matrix IDs")
        require(len([line for line in completed.stdout.splitlines() if line.startswith("COMMAND:")]) == len(rows), "command per row")
    rows = {row["id"]: row for row in run_smoke.matrix(True)}
    require(rows["Q8"]["render_loop"] == "basic" and rows["Q8"]["route"] == "import-direct", "basic-loop row")
    require(rows["Q11"]["frames"] == 3000 and not rows["Q11"]["debug_layer"], "long row")
    require(rows["Q10"]["handover"] == "declared" and rows["Q12"]["barriers"] == "match", "probe rows")
    require([r["id"] for r in rows.values() if r.get("inject_drain")] == ["Q13"], "drain injection scoped to Q13")
    adapter = "Synthetic GPU"
    result = dict(format="magic600-sd-smoke-run-v1", status="pass", device=dict(adapter_name=adapter), teardown=dict(phase="complete"))
    require(run_smoke.judge(rows["Q2"], result, 0, False, False, adapter) == "as-expected", "pass judgement")
    require(run_smoke.judge(rows["Q2"], result, 0, False, False, "other") == "unexpected", "adapter judgement")
    pending = copy.deepcopy(result)
    pending["teardown"]["phase"] = "pending"
    pending["frames"] = {"run": 120}
    require(run_smoke.judge(rows["Q13"], pending, 3, False, True, adapter) == "as-expected", "injected drain judgement")
    require(run_smoke.judge(rows["Q13"], pending, 3, False, False, adapter) == "unexpected", "drain stderr required")
    require(run_smoke.judge(rows["Q14"], None, -1, False, False, adapter) == "as-expected", "bounded loss without result")
    require(run_smoke.judge(rows["Q15"], result, 0, True, False, adapter) == "unexpected", "loss timeout")
    recorded = dict(format="magic600-sd-smoke-run-v1", status="recorded", reasons=["debug-errors", "decode-mismatch", "unverified-frames"],
                    device=dict(adapter_name=adapter), frames=dict(run=300),
                    teardown=dict(phase="complete", drain_confirmed=True, detached=True))
    require(run_smoke.judge(rows["Q10"], recorded, 0, False, False, adapter) == "as-expected", "recorded probe with observations")
    for change in (dict(reasons=["debug-layer-unavailable"], frames=dict(run=0)), dict(reasons=["identity-mismatch"]),
                   dict(frames=dict(run=299)), dict(teardown=dict(phase="pending", drain_confirmed=False, detached=False)),
                   dict(teardown=dict(phase="complete", drain_confirmed=True, detached=False)), dict(reasons=["teardown-incomplete"])):
        require(run_smoke.judge(rows["Q10"], {**recorded, **change}, 0, False, False, adapter) == "unexpected",
                "recorded probe that did not run: " + json.dumps(change))
    selftest = dict(format=smoke_summary.SELFTEST_FORMAT, checks=[dict(name="abi", status="pass"),
                                                              dict(name="warp", status="unsupported", reason="no adapter")])
    require(run_smoke.judge(rows["Q0"], selftest, 0, False, False, adapter) == "as-expected", "self-test with a reasoned unsupported check")
    for broken in ({**selftest, "checks": [dict(name="warp", status="unsupported", reason="no adapter")]},
                   {**selftest, "checks": [dict(name="abi", status="pass"), dict(name="warp", status="unsupported")]},
                   {**selftest, "format": "other"}, {**selftest, "status": "fail"}, None):
        require(run_smoke.judge(rows["Q0"], broken, 0, False, False, adapter) == "unexpected", "broken self-test accepted")
    require(run_smoke.judge(rows["Q0"], selftest, 1, False, False, adapter) == "unexpected", "self-test exit code")
    require(smoke_summary.run_summary(dict(row=rows["Q0"], result=selftest))["status"] == "pass", "Q0 summary status")
    command = run_smoke.build_command(Path("C:/a b/build.cmd"), Path("C:/a b/out"))
    require(isinstance(command, str) and command.startswith('cmd.exe /d /c call "') and '\\"' not in command, "cmd.exe string form")
    native = run_smoke.NATIVE
    require(run_smoke.excluded(native, ("build-check-1234", "x.obj")) and run_smoke.excluded(run_smoke.HERE, ("results", "s.json"))
            and run_smoke.excluded(run_smoke.HERE, ("app", "build", "x.obj")) and not run_smoke.excluded(native, ("build.cmd",))
            and not run_smoke.excluded(run_smoke.HERE, ("app", "src", "main.cpp")), "source walk exclusions")
    specs = run_smoke.diff_pathspecs()
    require(":(exclude)work/experiments/renderer-sd/results" in specs
            and ":(exclude,glob)work/experiments/renderer-sa2/native/build*/**" in specs
            and specs[0] == "work/experiments/renderer-sd" and "work/experiments/renderer-sa2/native" in specs, "diff exclusions")
    with patch.dict(os.environ, {"QSG_RHI_DEBUG_LAYER": "1", "M600_SA2_INJECT_UNCONFIRMED_DRAIN": "1", "VCINSTALLDIR": "vc"}):
        cleaned = run_smoke.clean_environment()
        require("QSG_RHI_DEBUG_LAYER" not in cleaned and "M600_SA2_INJECT_UNCONFIRMED_DRAIN" not in cleaned, "inherited probes removed")
        require(cleaned.get("QT_FORCE_STDERR_LOGGING") == "1", "Qt logs to stderr without a console")
        require("VCINSTALLDIR" not in run_smoke.deploy_environment(cleaned), "no VC redistributable in the deployment")


def summary_checks(directory):
    username = "sd_fixture_private_user"
    drive = "Z:" + chr(92) + "private" + chr(92) + "evidence.log"
    pointer = "0x" + "1234ABCDEF98"
    private = dict(evidence_kind="source-fixture", source=dict(head="a" * 40, sha256="b" * 64, matches_head=True),
                   dll_sha256="c" * 64, executable_sha256="d" * 64, qt_version="6.10.3",
                   deployment=dict(total_bytes=12345, file_count=7), runs=[dict(
                       row=run_smoke.matrix()[3], exit_code=0, timeout=False, judgement="as-expected", result_exists=True,
                       stderr={"total": 1}, result=dict(status="pass", reasons=[], device=dict(
                           adapter_name="Synthetic GPU", umd_version=(31 << 48) | (15 << 16) | 5915, enhanced_barriers=True,
                           identity=dict(device_matches=True, fallback_detected=False)),
                           frames=dict(run=1200, eligible=1200), verify=dict(composite_verified=1200),
                           debug=dict(error=0, corruption=0, ids=[1, 2]), teardown=dict(phase="complete", drain_confirmed=True,
                           detached=True, refcount_after=[0, 0, 0], slots_unregistered=3)))])
    planted = copy.deepcopy(private)
    planted["runs"][0]["result"]["debug"]["messages"] = drive + " " + username + " " + pointer
    planted["runs"][0]["result"]["sa2_failures"] = [{"last_error": drive, "pointer": pointer, "name": username}]
    planted["runs"][0]["row"]["command_line"] = drive
    planted["runs"][0]["result"]["reasons"] = [drive, username, pointer]
    summary = smoke_summary.write_summary(planted, directory / "clean-summary.json", username)
    text = json.dumps(summary)
    require(all(value not in text for value in (drive, username, pointer)), "free text leaked through projection")
    require(summary["driver_version"] == "31.0.15.5915", "driver version formatting")
    smoke_summary.scan(summary)
    for value in (drive, username, pointer, chr(37) + "SECRET" + chr(37)):
        poisoned = copy.deepcopy(private)
        poisoned["runs"][0]["result"]["device"]["adapter_name"] = value
        destination = directory / "refused-summary.json"
        try:
            smoke_summary.write_summary(poisoned, destination, username)
        except ValueError:
            require(not destination.exists(), "privacy rejection wrote a file")
        else:
            raise AssertionError("private adapter string accepted")


def pathspec_checks(directory):
    """In a temporary repository, the diff pathspecs ignore excluded output and see source changes."""
    repo = directory / "pathspec-fixture"
    excluded = ("work/experiments/renderer-sd/results/sd-smoke-summary.json", "work/experiments/renderer-sd/app/build/x.obj",
                "work/experiments/renderer-sa2/native/build-check-1/x.obj")
    sources = ("work/experiments/renderer-sd/run_smoke.py", "work/experiments/renderer-sd/build.cmd",
               "work/experiments/renderer-sa2/native/build.cmd")
    for name in excluded + sources:
        (repo / name).parent.mkdir(parents=True, exist_ok=True)
        (repo / name).write_bytes(b"0")
    git = ["git", "--no-optional-locks", "-c", "user.name=fixture", "-c", "user.email=fixture@example.invalid",
           "-c", "core.autocrlf=false", "-c", "commit.gpgsign=false"]
    for args in (["init", "-q"], ["add", "-f", "."], ["commit", "-q", "--no-verify", "-m", "fixture"]):
        subprocess.run(git + args, cwd=repo, check=True, capture_output=True, timeout=30)

    def changed():
        return subprocess.run(git + ["diff", "--quiet", "HEAD", "--", *run_smoke.diff_pathspecs()],
                              cwd=repo, capture_output=True, timeout=30, check=False).returncode
    for name in excluded:
        (repo / name).write_bytes(b"1")
    require(changed() == 0, "a change under an excluded folder counts as a source change")
    for name in sources:
        (repo / name).write_bytes(b"1")
        require(changed() == 1, "source change missed: " + name)
        (repo / name).write_bytes(b"0")


def remove_tree(path):
    """Remove a fixture tree, including Git's read-only object files."""
    def retry(function, name, _):
        os.chmod(name, stat.S_IWRITE)
        function(name)
    shutil.rmtree(path, onexc=retry)


def reuse_checks(directory):
    root = directory / "reuse-fixture"
    build = root / "work/sdb/20261003T000000Z"
    for relative in ("native/sa2_interop.dll", "native/sa2_selftest.exe", "app/sd_code_layout_test.exe",
                     "app/sd_smoke.exe", "deploy/sd_smoke.exe", "deploy/Qt6Core.dll"):
        path = build / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(relative.encode("ascii"))
    source = dict(head="a" * 40, sha256="b" * 64, matches_head=True)
    identity = dict(source=source, qt_version="6.10.3", artifacts=run_smoke.deployment_manifest(build),
                    dll_sha256=run_smoke.sha256(build / "native/sa2_interop.dll"),
                    executable_sha256=run_smoke.sha256(build / "deploy/sd_smoke.exe"))
    run_smoke.atomic_json(build / "build_identity.json", identity)
    with patch.object(run_smoke, "ROOT", root):
        require(run_smoke.reuse_build(source, "6.10.3")[0] == build, "matching build reuse")
        for altered_source, version in ((dict(source, sha256="e" * 64), "6.10.3"), (source, "6.11.0")):
            try:
                run_smoke.reuse_build(altered_source, version)
            except ValueError:
                pass
            else:
                raise AssertionError("stale build reused")
        (build / "app/sd_code_layout_test.exe").write_bytes(b"tampered fixture")
        try:
            run_smoke.reuse_build(source, "6.10.3")
        except ValueError:
            pass
        else:
            raise AssertionError("modified artifact reused")


def main():
    layout = static_checks()
    reference_checks(layout)
    runner_checks()
    # Plain mkdir, owned uniquely by this invocation, with cleanup on every path.
    base = Path(tempfile.gettempdir()).resolve()
    directory = base / ("m600-sd-check-" + str(os.getpid()) + "-" + uuid.uuid4().hex)
    directory.mkdir()
    try:
        summary_checks(directory)
        reuse_checks(directory)
        pathspec_checks(directory)
    finally:
        require(directory.resolve().parent == base, "temp cleanup boundary")
        remove_tree(directory)
    print("sd project: ok (source/static and Python fixtures; no Qt, C++ build or GPU)")
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (AssertionError, OSError, ValueError, subprocess.SubprocessError) as error:
        print(f"sd project: {error}", file=sys.stderr)
        sys.exit(1)
