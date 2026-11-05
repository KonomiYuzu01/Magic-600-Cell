"""Build/run the named-device experiment; retain small captures, remove build products."""
import sys
sys.dont_write_bytecode = True
import argparse
from hashlib import sha256
import json
import os
from pathlib import Path
import re
import struct
import subprocess
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from reference import HERE, input_bytes, json_bytes
from freeze import verify_manifest, clean_build
from compare_device import compare

FLAGS = ["-T", "cs_6_2", "-E", "main", "-HV", "2021", "-Gis", "-denorm", "preserve", "-O3", "-Ges", "-WX"]


def run(adapter, sandbox_lead):
    verify_manifest()
    if os.name != "nt":
        raise RuntimeError("not run here: Windows is required")
    build = HERE / "build"
    if build.exists():
        raise RuntimeError("build already exists; finish or clean the earlier experiment first")
    (build / "tmp").mkdir(parents=True)
    env = dict(os.environ, TMP=str(build / "tmp"), TEMP=str(build / "tmp"))
    try:
        # Capture diagnostics transiently; never retain machine paths in public artifacts.
        with (build / "compile.log").open("wb") as log:
            result = subprocess.run(["cmd.exe", "/d", "/c", "device\\build.cmd"], cwd=HERE, env=env,
                                    stdout=log, stderr=subprocess.STDOUT, timeout=180)
        if result.returncode:
            raise RuntimeError("not run here: device build failed (exit " + str(result.returncode) + ")")
        fixtures = json.loads((HERE / "fixtures.json").read_bytes())
        inputs = input_bytes(fixtures)
        (build / "inputs.bin").write_bytes(inputs)
        with (build / "device.log").open("wb") as log:
            result = subprocess.run([str(build / "chart_device.exe"), "build/generated.dxil", "build/inputs.bin",
                                     "build/outputs.bin", "build/adapter.json", adapter], cwd=HERE, env=env,
                                    stdout=log, stderr=subprocess.STDOUT, timeout=45)
        if result.returncode:
            raise RuntimeError("not run here: named-device execution failed (exit " + str(result.returncode) + ")")
        output = (build / "outputs.bin").read_bytes()
        version = (build / "dxc-version.txt").read_text(encoding="utf-8", errors="replace").strip()
        # The version command is captured separately; refuse unexpected path-bearing output.
        if "\\" in version or ":/" in version or len(version) > 512:
            raise RuntimeError("unexpected compiler version output")
        compile_log = (build / "compile.log").read_text(encoding="utf-8", errors="replace")
        msvc = re.search(r"Compiler Version ([0-9.]+)", compile_log)
        capture = {"format": "lx7-device-capture-v1", "evidence_class": "windows", "sandbox_lead": sandbox_lead,
                   "precision_decision": "open", "decider": "owner", "requested_adapter": adapter,
                   "device": json.loads((build / "adapter.json").read_bytes()),
                   "manifest_sha256": sha256((HERE / "manifest.json").read_bytes()).hexdigest(),
                   "input_sha256": sha256(inputs).hexdigest(),
                   "hlsl_sha256": sha256((HERE / "generated.hlsl").read_bytes()).hexdigest(),
                   "dxil_sha256": sha256((build / "generated.dxil").read_bytes()).hexdigest(),
                   "executable_sha256": sha256((build / "chart_device.exe").read_bytes()).hexdigest(),
                   "output_sha256": sha256(output).hexdigest(), "compile_flags": FLAGS,
                   "tools": {"python": sys.version.split()[0], "dxc": version,
                             "msvc": msvc.group(1) if msvc else "version not captured"},
                   "records": list(struct.iter_unpack("<4I", output))}
        summary = compare(capture)
        suffix = "sandbox-lead" if sandbox_lead else "device"
        (HERE / "device" / (suffix + "-capture.json")).write_bytes(json_bytes(capture))
        (HERE / "device" / (suffix + "-summary.json")).write_bytes(json_bytes(summary))
        print("Named device: " + capture["device"]["adapter"])
        print("Captured " + str(len(capture["records"])) + " records; precision decision open; no performance result.")
        if sandbox_lead:
            print("Sandbox result is a lead; integrator must rerun outside the sandbox.")
    finally:
        clean_build()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--adapter", default="NVIDIA GeForce RTX 4070 Laptop GPU")
    parser.add_argument("--sandbox-lead", action="store_true")
    args = parser.parse_args()
    try:
        run(args.adapter, args.sandbox_lead)
    except (OSError, ValueError, RuntimeError, subprocess.TimeoutExpired) as error:
        message = str(error) if isinstance(error, RuntimeError) else "not run here: " + type(error).__name__
        raise SystemExit(message) from None
