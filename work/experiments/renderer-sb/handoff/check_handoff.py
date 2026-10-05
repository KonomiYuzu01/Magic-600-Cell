"""Build out of tree and exercise CPU fixtures. This never runs a GPU mode."""

import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile


def build_checked(arguments):
    environment = os.environ.copy()
    environment["M600_HANDOFF_CPU_CHECK"] = "1"
    process = subprocess.Popen(arguments, env=environment)
    try:
        code = process.wait(timeout=180)
    except subprocess.TimeoutExpired:
        # Kill only this owned build's process tree. Avoid Popen's context
        # manager: its exit performs an unbounded wait after a failed timeout.
        try:
            subprocess.run(
                ["taskkill.exe", "/PID", str(process.pid), "/T", "/F"],
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                timeout=10,
                check=False,
            )
        finally:
            if process.poll() is None:
                process.kill()
            process.wait(timeout=10)
        raise
    if code:
        raise subprocess.CalledProcessError(code, arguments)


def main():
    source = Path(__file__).resolve().parent
    temp_root = Path(tempfile.gettempdir()).resolve()
    build = temp_root / f"m600-sb-handoff-{os.getpid()}"
    created = False
    try:
        # Plain mkdir is deliberate: mkdtemp/TemporaryDirectory receive unusable
        # ACLs in the Windows implementation sandbox.
        build.mkdir()
        created = True
        build_checked(["cmd.exe", "/d", "/c", "call", str(source / "build.cmd"), str(build)])
        fixture = build / "fixture.json"
        result = subprocess.run(
            [str(build / "sb_handoff.exe"), "--selftest", "--out", str(fixture)],
            check=True,
            timeout=30,
            text=True,
            capture_output=True,
        )
        print(result.stdout, end="")
        if result.stdout.strip() != "selftest: ok":
            raise RuntimeError("selftest did not print its success marker")
        report = json.loads(fixture.read_text(encoding="utf-8"))
        if report["format"] != "magic600-sb-handoff-v1" or len(report["modes"]) != 1:
            raise RuntimeError("invalid fixture JSON envelope")
        mode = report["modes"][0]
        if mode["mode"] != "selftest" or mode["status"] != "pass" or mode["verified_iterations"] != 1:
            raise RuntimeError("selftest JSON does not confirm success")
        if mode["singleton_result"]["producer_same_pointer"] is not None or mode["adapter_luids"]["producer"] is not None:
            raise RuntimeError("CPU fixture incorrectly claims device evidence")
        if mode["first_failing_iteration"] is not None or mode["errors"]:
            raise RuntimeError("CPU fixture recorded a failure")
        print("check_handoff: ok (source/fixture only; no GPU mode run)")
        return 0
    except (OSError, ValueError, KeyError, RuntimeError, subprocess.SubprocessError) as error:
        print(f"check_handoff: failed: {error}", file=sys.stderr)
        if isinstance(error, subprocess.CalledProcessError) and error.stderr:
            print(error.stderr, file=sys.stderr)
        return 1
    finally:
        if created:
            # Delete only this invocation's freshly created, resolved temp child.
            if build.resolve().parent != temp_root or build.name != f"m600-sb-handoff-{os.getpid()}":
                raise RuntimeError("refusing cleanup outside the acceptance temp directory")
            shutil.rmtree(build)


if __name__ == "__main__":
    sys.exit(main())
