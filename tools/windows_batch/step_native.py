"""Run the native WinForms self-test on fresh synthetic data."""
from __future__ import annotations

import json
import os
from pathlib import Path
import re
import shutil
import struct
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import batch_interface as wb

IS_WINDOWS = os.name == "nt"


def _last_line(output):
    return next((line.strip() for line in reversed(output.splitlines()) if line.strip()), "")


class NativeStep:
    name = "native"
    title = "Native host self-test (tests/native/NativeHostRegression.cs)"

    def unavailable(self, ctx):
        if not IS_WINDOWS:
            return "Windows only"
        if not (ctx.repo / "native/bootstrap.py").is_file() or not (
                ctx.repo / "tests/native/NativeHostRegression.cs").is_file():
            return "native host regression not in this checkout"
        compiler = Path(os.environ.get("WINDIR", r"C:\Windows")) / "Microsoft.NET/Framework/v4.0.30319/csc.exe"
        if not compiler.is_file():
            return ".NET Framework 4.x compiler not found"
        if not (ctx.repo / "tools/.venv/engine/Scripts/python.exe").is_file() and struct.calcsize("P") != 8:
            return "needs 64-bit Python"
        return None

    def run(self, ctx):
        result = wb.StepResult(wb.ERROR, counts={"checks": 0, "checks_passed": 0, "checks_failed": 0})
        ctx.manual("Native self-test: it compiles the host, then opens and closes test windows by itself for up to two minutes; do not click or type until it reports.")
        data = ctx.scratch / "data"
        python = ctx.repo / "tools/.venv/engine/Scripts/python.exe"
        if not python.is_file():
            python = sys.executable
        env = {key: value for key, value in os.environ.items() if key.upper() != "LIB"}
        env["PYTHONUTF8"] = "1"
        try:
            source = ctx.repo / "tests/native/NativeHostRegression.cs"
            result.digests["NativeHostRegression.cs"] = wb.sha256_file(source)
            completed = ctx.run([str(python), str(ctx.repo / "native/bootstrap.py"),
                                 "--self-test-only", "--data", str(data)], timeout=900, env=env)
            result.details["returncode"] = completed.returncode
            diagnostics = data / "native-host-0.3/diagnostics"
            for pattern in ("*.json", "*.log"):
                for path in sorted(diagnostics.glob(pattern)):
                    shutil.copy2(path, ctx.private / path.name)
            executable = diagnostics / "NativeHostRegression.exe"
            if executable.is_file():
                result.digests["NativeHostRegression.exe"] = wb.sha256_file(executable)
            build_info = diagnostics / "build-info.json"
            if build_info.is_file():
                try:
                    identity = json.loads(build_info.read_text(encoding="utf-8-sig")).get("source_sha256")
                    if isinstance(identity, str) and re.fullmatch(r"[0-9a-fA-F]{64}", identity):
                        result.digests["host_sources"] = identity.lower()
                except (OSError, ValueError, AttributeError):
                    pass
            if completed.timed_out:
                result.reason = wb.clip("native self-test timed out after 900 s")
                return result
            report_path = diagnostics / "winforms-self-test.json"
            if not report_path.is_file():
                result.reason = wb.clip(_last_line(completed.output) or "no self-test report")
                return result
            result.digests["report"] = wb.sha256_file(report_path)
            report = json.loads(report_path.read_text(encoding="utf-8-sig"))
            checks = report["checks"]
            failed = [check["name"] for check in checks if check.get("passed") is not True]
            result.counts.update(checks=len(checks), checks_passed=len(checks) - len(failed), checks_failed=len(failed))
            result.details.update(scope=report.get("scope"), clr=report.get("clr"),
                                  process_bits=report.get("process_bits"), failed_checks=failed)
            result.lines = [wb.clip("WinForms self-test: %s of %s checks passed (CLR %s, %s-bit host)" % (
                result.counts["checks_passed"], len(checks), report.get("clr", "unknown"),
                report.get("process_bits", "unknown")))]
            if report.get("passed") is True and completed.returncode == 0:
                result.status = wb.PASS
            else:
                result.status = wb.FAIL
                error = (report.get("error") or "").strip().splitlines()
                result.reason = wb.clip((error[0] if error else "") or ", ".join(failed)
                                        or _last_line(completed.output) or "native self-test failed")
        except (OSError, ValueError, KeyError, TypeError, AttributeError) as error:
            result.status = wb.ERROR
            result.reason = wb.clip(error)
        return result


STEP = NativeStep()
