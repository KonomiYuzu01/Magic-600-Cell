"""Build the renderer core in a fresh ignored directory and run its CPU self-test; --gpu also runs the owner GPU checks.

--stages DXC_DIR also runs C9 U5's user-stage check on WARP only (never a hardware adapter), with dxcompiler.dll and
dxil.dll copied from DXC_DIR beside the built DLL. --poses also runs the product pose check on WARP only, --shape
(ABI 6) the view shape check on WARP only, --style (ABI 7) the piece style check on WARP only, --panes (ABI 8) the
composite frame check on WARP only, --focus (ABI 9) the view focus check on WARP only, and --hopf (ABI 10) the view
layout check on WARP only, on Look Lab's tools/looklab/fixtures/hopf-rings.json.

Derived from work/experiments/renderer-sa2/native/check_native.py (see PROVENANCE.md).
"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys


SHADERS = [
    "count_vs.dxil", "draw_ps.dxil", "draw_vs.dxil", "geometry_cs.dxil", "id_ps.dxil", "lattice_cs.dxil",
    "product_ps.dxil", "product_vs.dxil",
]
CPU_CHECKS = {
    # ABI 2 checks, unchanged.
    "abi", "exports", "invalid arguments", "last error truncation", "crc", "vectors", "images", "decode rejection",
    "hex masking", "state tables", "json writer", "scene arguments", "scene state", "scene assets",
    "scene native writer", "scene output IO", "identity",
    # ABI 3: the W-J scene API without a device.
    "wj arguments", "wj scene state", "wj reader refusals", "wj protocol controls", "wj native writer",
    # ABI 4: the product API's device-free logic.
    "product arguments", "product adopt", "product tickets", "product frames", "product reports", "product picks",
    "product device lost", "product describe",
    # ABI 5: route B's argument checks, present matching and recovery bookkeeping
    # (route B is accepted for now by the owner on 18 Nov 2026, pending three
    # owner-machine checks).
    "route b arguments", "present matching", "recovery bookkeeping",
    # C9 (26 Nov 2026): a product frame's frame-latency wait takes one statistics sample when it lasts longer than
    # 5 ms, so that the single sample after each frame no longer races the vblank's statistics.
    "latency wait",
    # C9 (26 Nov 2026): released preparation resources are kept for the next preparation (at most two, newest first,
    # none after a device removal), so a twist's preparation creates no GPU resources.
    "preparation reuse",
    # C9 U5: user stages' compiler profile, source composition and compile record, and RPI.72's raster bound.
    "stage arguments", "raster bound",
    # ABI 6: a view's shape: its checks, its defaults, per view, recorded with each frame and kept across a recovery.
    "view shape",
    # ABI 7: the piece style and the style slots: their checks, when a frame draws styled, the upload key, and an
    # adoption dropping the style.
    "piece style",
    # ABI 8: the composite frame: its checks, its pane rows and a pick that resolves its pane.
    "pane arguments",
    # ADR-D02 B21: the raw view's focus and the antipode chart's handedness.
    "raw view",
}
DXC_LIBRARIES = ("dxcompiler.dll", "dxil.dll")
BUILD_TEXT = {".make", ".cmake", ".d", ".rsp", ".ninja"}


def run_bounded(arguments, timeout, **kwargs):
    process = subprocess.Popen(arguments, **kwargs)
    try:
        stdout, stderr = process.communicate(timeout=timeout)
    except subprocess.TimeoutExpired:
        try:
            subprocess.run(
                ["taskkill.exe", "/PID", str(process.pid), "/T", "/F"],
                stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                timeout=10, check=False,
            )
        finally:
            if process.poll() is None:
                process.kill()
            process.communicate(timeout=10)
        raise
    if process.returncode:
        raise subprocess.CalledProcessError(process.returncode, arguments, stdout, stderr)
    return stdout, stderr


def check_result(executable, build, mode, vectors, stages=False, poses=False, warp_mode=None):
    """warp_mode names an alone WARP check by its self-test flag (shape: ABI 6's --shape; style: ABI 7's --style; panes: ABI 8's --panes)."""
    poses = poses or warp_mode == "poses"
    suffix = "-stages" if stages else "-poses" if poses else f"-{warp_mode}" if warp_mode else ""
    output = build / f"{mode}{suffix}.json"
    arguments = [str(executable), f"--{mode}", "--vectors", vectors, "--out", str(output)]
    if stages:
        arguments.append("--stages")
    if poses:
        arguments.append("--poses")
    if warp_mode and warp_mode != "poses":
        arguments.append(f"--{warp_mode}")
    if warp_mode == "hopf":
        arguments.append(str(Path(__file__).resolve().parents[2] / "tools" / "looklab" / "fixtures" / "hopf-rings.json"))
    if mode != "cpu":
        arguments.append("--debug")
    timeout = 300 if mode == "cpu" else 900 if stages or poses or warp_mode else 7200
    stdout, _ = run_bounded(arguments, timeout, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    print(stdout, end="")
    if not stdout.splitlines() or stdout.splitlines()[-1] != "selftest: ok":
        raise RuntimeError(f"{mode}: missing final success marker")
    report = json.loads(output.read_text(encoding="utf-8"))
    if report["format"] != "magic600-sa2-native-selftest-v1" or report["mode"] != mode or not report["checks"]:
        raise RuntimeError(f"{mode}: invalid JSON envelope")
    for check in report["checks"]:
        if not check["name"] or not check["reason"] or check["status"] not in ("pass", "unsupported"):
            raise RuntimeError(f"{mode}: failed or incomplete check: {check}")
        if mode == "cpu" and check["status"] != "pass":
            raise RuntimeError("CPU acceptance requires all CPU checks to pass")
        if stages and check["status"] != "pass":
            raise RuntimeError("the WARP stage check requires every check to pass")
        if poses and check["status"] != "pass":
            raise RuntimeError("the WARP pose check requires every check to pass")
        if warp_mode and check["status"] != "pass":
            raise RuntimeError(f"the WARP {warp_mode} check requires every check to pass")
    if mode == "cpu" and {check["name"] for check in report["checks"]} != CPU_CHECKS:
        raise RuntimeError("CPU check set is incomplete")
    if mode == "cpu":
        identity = report["identity"]
        shaders = identity["shaders"]
        if identity["dll"]["file"] != "sa2_interop.dll" or [part["file"] for part in shaders] != SHADERS:
            raise RuntimeError("identity does not cover the DLL and every loaded shader in sorted order")
        for part in [identity["dll"], *shaders]:
            digest = hashlib.sha256((build / part["file"]).read_bytes()).hexdigest()
            if digest != part["sha256"]:
                raise RuntimeError(f'identity SHA-256 mismatch: {part["file"]}')
        print(f"identity: DLL and all {len(SHADERS)} shader SHA-256 values recomputed")
        print(f"cpu checks: {len(report['checks'])} pass")


def check_sources(build):
    """Refuse a build whose generated rules or dependencies name a file under work/experiments."""
    files = [path for path in build.rglob("*") if path.is_file() and (path.suffix in BUILD_TEXT or path.name == "build.ninja")]
    if not any(path.name in ("build.make", "build.ninja") for path in files):
        raise RuntimeError("no generated build rules to inspect")
    for path in files:
        text = path.read_text(encoding="utf-8", errors="replace").replace("\\", "/").lower()
        if "work/experiments" in text:
            raise RuntimeError(f"build references work/experiments: {path.relative_to(build)}")
    print(f"sources: {len(files)} generated build files name no work/experiments path")


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--gpu", action="store_true", help="also run the WARP and hardware checks (owner only)")
    parser.add_argument("--stages", metavar="DXC_DIR", type=Path,
                        help="also run the WARP user-stage check with dxcompiler.dll and dxil.dll from DXC_DIR")
    parser.add_argument("--poses", action="store_true",
                        help="also run the WARP check that the product draw applies an engine image's per-piece poses")
    parser.add_argument("--shape", action="store_true",
                        help="also run ABI 6's WARP check of a view's shape (sa2_view_shape)")
    parser.add_argument("--style", action="store_true",
                        help="also run ABI 7's WARP check of the piece style (sa2_style_set, sa2_view_style_set)")
    parser.add_argument("--hopf", action="store_true",
                        help="also run ABI 10's WARP check of a view's layout (sa2_view_layout_set) on Look Lab's Hopf rings")
    parser.add_argument("--focus", action="store_true",
                        help="also run ABI 9's WARP check of a view's focus (sa2_view_focus)")
    parser.add_argument("--sweep", action="store_true",
                        help="also run ADR-D02 B18's WARP check of a twist's transform (sa2_transform_set)")
    parser.add_argument("--slice", action="store_true",
                        help="also run ADR-D02 B20's WARP check of the cut slice (a style slot's slice)")
    parser.add_argument("--raw", action="store_true",
                        help="also run ADR-D02 B21's WARP check of the raw view (the stereographic chart and its antipode)")
    parser.add_argument("--panes", action="store_true",
                        help="also run ABI 8's WARP check of the composite frame (sa2_frame_submit_panes)")
    options = parser.parse_args()
    source = Path(__file__).resolve().parent
    root = source.parent.parent
    # Application Control can refuse unsigned executables in system temp. Build
    # inside the repository's ignored work tree.
    parent = root / "work" / "build"
    name = f"renderer-core-{os.getpid()}"
    build = parent / name
    created = False
    try:
        batch = (source / "build.cmd").read_bytes()
        if not batch.endswith(b"\r\n") or b"\n" in batch.replace(b"\r\n", b"") or b"\r" in batch.replace(b"\r\n", b""):
            raise RuntimeError("build.cmd must have CRLF-only line endings")
        layout = json.loads((source / "code_layout.json").read_text(encoding="utf-8"))
        vectors = ";".join(f'{v["sequence"]},{v["slot"]},{v["generation"]},{v["code"]}' for v in layout["test_vectors"] if "code" in v)
        if len(vectors.split(";")) != 3:
            raise RuntimeError("expected three code vectors in code_layout.json")
        # Plain mkdir: mkdtemp/TemporaryDirectory have unusable ACLs in the sandbox.
        parent.mkdir(parents=True, exist_ok=True)
        build.mkdir()
        created = True
        environment = os.environ.copy()
        environment["M600_SA2_CPU_CHECK"] = "1"
        run_bounded(["cmd.exe", "/d", "/c", "call", str(source / "build.cmd"), str(build)], 1200, env=environment)
        executable = build / "sa2_selftest.exe"
        if not (build / "sa2_interop.dll").is_file() or not executable.is_file():
            raise RuntimeError("build outputs are missing from the build directory root")
        check_sources(build)
        check_result(executable, build, "cpu", vectors)
        if options.gpu:
            for mode in ("warp", "hardware"):
                check_result(executable, build, mode, vectors)
        if options.stages:
            for library in DXC_LIBRARIES:
                shutil.copyfile(options.stages / library, build / library)
            check_result(executable, build, "warp", vectors, stages=True)
        if options.poses:
            check_result(executable, build, "warp", vectors, poses=True)
        if options.shape:
            check_result(executable, build, "warp", vectors, warp_mode="shape")
        if options.style:
            check_result(executable, build, "warp", vectors, warp_mode="style")
        if options.panes:
            check_result(executable, build, "warp", vectors, warp_mode="panes")
        if options.focus:
            check_result(executable, build, "warp", vectors, warp_mode="focus")
        if options.hopf:
            check_result(executable, build, "warp", vectors, warp_mode="hopf")
        if options.sweep:
            check_result(executable, build, "warp", vectors, warp_mode="sweep")
        if options.slice:
            check_result(executable, build, "warp", vectors, warp_mode="slice")
        if options.raw:
            check_result(executable, build, "warp", vectors, warp_mode="raw")
        summary = " (CPU, WARP and hardware)" if options.gpu else " (source/fixture only; no GPU mode run)"
        warp = " and ".join(name for name, run in (("stage", options.stages), ("pose", options.poses),
                                                   ("shape", options.shape), ("style", options.style),
                                                   ("panes", options.panes), ("focus", options.focus),
                                                   ("hopf", options.hopf), ("sweep", options.sweep),
                                                   ("slice", options.slice), ("raw", options.raw)) if run)
        if warp:
            summary = f" (CPU, WARP and hardware; WARP {warp} checks)" if options.gpu else f" (CPU and the WARP {warp} checks; no hardware run)"
        print("check_core: ok" + summary)
        return 0
    except (OSError, ValueError, KeyError, RuntimeError, subprocess.SubprocessError) as error:
        print(f"check_core: failed: {error}", file=sys.stderr)
        if isinstance(error, subprocess.CalledProcessError):
            for text in (error.stdout, error.stderr):
                if text:
                    print(text, file=sys.stderr)
        return 1
    finally:
        if created:
            if build.resolve().parent != parent.resolve() or build.name != name:
                raise RuntimeError("refusing cleanup outside this check's build directory")
            shutil.rmtree(build)


if __name__ == "__main__":
    sys.exit(main())
