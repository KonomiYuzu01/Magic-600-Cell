"""Build in a fresh ignored directory and check CPU fixtures; --gpu also runs the owner GPU checks."""

import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys


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


def check_result(executable, build, mode, vectors):
    output = build / f"{mode}.json"
    arguments = [str(executable), f"--{mode}", "--vectors", vectors, "--out", str(output)]
    if mode != "cpu":
        arguments.append("--debug")
    stdout, _ = run_bounded(arguments, 30 if mode == "cpu" else 600, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
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
    expected = {"abi", "exports", "invalid arguments", "last error truncation", "crc", "vectors", "images", "decode rejection", "hex masking", "state tables", "json writer"}
    if mode == "cpu" and {check["name"] for check in report["checks"]} != expected:
        raise RuntimeError("CPU check set is incomplete")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--gpu", action="store_true")
    options = parser.parse_args()
    source = Path(__file__).resolve().parent
    # Application Control can refuse unsigned executables in system temp. Build
    # beside this script, inside the repository's ignored work tree.
    build = source / f"build-check-{os.getpid()}"
    created = False
    try:
        batch = (source / "build.cmd").read_bytes()
        if not batch.endswith(b"\r\n") or b"\n" in batch.replace(b"\r\n", b"") or b"\r" in batch.replace(b"\r\n", b""):
            raise RuntimeError("build.cmd must have CRLF-only line endings")
        layout = json.loads((source.parent / "code_layout.json").read_text(encoding="utf-8"))
        vectors = ";".join(f'{v["sequence"]},{v["slot"]},{v["generation"]},{v["code"]}' for v in layout["test_vectors"] if "code" in v)
        if len(vectors.split(";")) != 3:
            raise RuntimeError("expected three code vectors in code_layout.json")
        # Plain mkdir: mkdtemp/TemporaryDirectory have unusable ACLs in the sandbox.
        build.mkdir()
        created = True
        environment = os.environ.copy()
        environment["M600_SA2_CPU_CHECK"] = "1"
        run_bounded(["cmd.exe", "/d", "/c", "call", str(source / "build.cmd"), str(build)], 300, env=environment)
        executable = build / "sa2_selftest.exe"
        if not (build / "sa2_interop.dll").is_file() or not executable.is_file():
            raise RuntimeError("build outputs are missing from the build directory root")
        check_result(executable, build, "cpu", vectors)
        if options.gpu:
            for mode in ("warp", "hardware"):
                check_result(executable, build, mode, vectors)
        print("check_native: ok" + (" (CPU, WARP and hardware)" if options.gpu else " (source/fixture only; no GPU mode run)"))
        return 0
    except (OSError, ValueError, KeyError, RuntimeError, subprocess.SubprocessError) as error:
        print(f"check_native: failed: {error}", file=sys.stderr)
        if isinstance(error, subprocess.CalledProcessError):
            for text in (error.stdout, error.stderr):
                if text:
                    print(text, file=sys.stderr)
        return 1
    finally:
        if created:
            if build.resolve().parent != source or build.name != f"build-check-{os.getpid()}":
                raise RuntimeError("refusing cleanup outside this check's build directory")
            shutil.rmtree(build)


if __name__ == "__main__":
    sys.exit(main())
