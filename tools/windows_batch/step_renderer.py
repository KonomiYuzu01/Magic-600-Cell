"""Drive S-B captures through the batch console and process context."""
from __future__ import annotations

import json
import os
from pathlib import Path
import re
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import batch_interface as wb

IS_WINDOWS = os.name == "nt"
PROBE_DIR = Path("work/experiments/renderer-sb/probe")
FAULTS = ("corrupt-label", "swap-same-colour", "delay-adoption", "stale-binding")
STAMP = r"\d{8}T\d{9}Z"
# The two declarations are free text, so only their documented defaults are published.
OVERLAYS_DEFAULT = "none running"
VENDOR_DEFAULT = "NVIDIA discrete GPU (MUX), high performance"
PRIVATE_ANSWER = "custom answer (private record)"


def _directories(path):
    return {item.name for item in path.iterdir() if item.is_dir()} if path.is_dir() else set()


def _last_line(output):
    return next((line.strip() for line in reversed(output.splitlines()) if line.strip()), "")


def _presentmon():
    return Path(os.environ.get("ProgramFiles", r"C:\Program Files")) / "Intel/PresentMon/PresentMonConsoleApplication/PresentMon-2.6.0-x64.exe"


def _number(value, digits):
    return format(value, ".%sf" % digits) if isinstance(value, (int, float)) else "unknown"


def _judge_fault(captures, new, completed, detail):
    """None when an injected fault was caught, otherwise how it was missed.

    Caught: the probe applied the fault and its label check failed, the capture script
    completed, and the gate summary of that invocation refused the run for the label check.
    An incomplete or unreadable run raises."""
    runs = sorted(name for name in new if re.fullmatch(STAMP + r"-w3-1", name))
    if len(runs) != 1:
        raise ValueError("could not identify run.json (%s new run directories)" % len(runs))
    run_path = captures / runs[0] / "run.json"
    if not run_path.is_file():
        raise ValueError("missing run.json")
    run = json.loads(run_path.read_text(encoding="utf-8"))
    label = run["label_check"]
    detail.update(status=label.get("status"), injection_applied=run.get("injection_applied"),
                  counts={key: label.get(key) for key in (
                      "mismatches", "late_adoptions", "binding_mismatches", "missing", "revisions", "copies")})
    if run.get("injection_applied") is not True:
        raise ValueError("the probe did not apply the fault")
    if label.get("status") == "pass":
        # The probe's own check missed the fault, whatever happened to the capture afterwards.
        return "label check passed"
    if label.get("status") != "fail":
        raise ValueError("missing label-check status")
    if completed.timed_out or completed.returncode != 0:
        raise ValueError("capture incomplete (script %s)" % (
            "timed out" if completed.timed_out else "exit %s" % completed.returncode))
    summary_path = captures / (runs[0][:-len("-1")] + "-summary") / "summary.json"
    if not summary_path.is_file():
        raise ValueError("missing gate summary.json")
    summary = json.loads(summary_path.read_text(encoding="utf-8"))
    run_id = run["run_id"]
    if any(record["run_id"] == run_id for scene in summary["scenes"] for record in scene["runs"]):
        detail["gate_refused"] = False
        return "the gate accepted the run"
    detail["gate_refused"] = any(entry.get("run_id") == run_id and "label-check-failed" in entry.get("reasons", [])
                                 for entry in summary["invalid_runs"])
    if not detail["gate_refused"]:
        raise ValueError("the gate did not refuse the run for its label check")
    return None


class RendererStep:
    name = "renderer"
    title = "S-B renderer captures (work/experiments/renderer-sb)"

    def unavailable(self, ctx):
        if not IS_WINDOWS:
            return "Windows only"
        probe_dir = ctx.repo / PROBE_DIR
        if not (probe_dir / "run_scene.ps1").is_file() or not (ctx.repo / "tools/perf/renderer_gate.py").is_file():
            return "S-B capture script not in this checkout"
        if not (probe_dir / "build/sb_probe.exe").is_file() and not (probe_dir / "build.cmd").is_file():
            return "S-B probe build files missing"
        if not ctx.elevated:
            return "needs an administrator PowerShell (PresentMon)"
        if not _presentmon().is_file():
            return "PresentMon 2.6.0 not found at the pinned location"
        if not ctx.options["scenes"] and not ctx.options["faults"]:
            return "no captures requested"
        return None

    def run(self, ctx):
        scenes = ctx.options["scenes"]
        runs = ctx.options["runs"]
        result = wb.StepResult(wb.ERROR, counts={
            "scenes_requested": len(scenes), "scenes_complete": 0, "valid_runs": 0,
            "faults_run": 0, "faults_caught": 0,
        }, details={"scenes": {}, "faults": {}})
        probe_dir = ctx.repo / PROBE_DIR
        script = probe_dir / "run_scene.ps1"
        executable = probe_dir / "build/sb_probe.exe"
        gate = ctx.repo / "tools/perf/renderer_gate.py"
        captures = ctx.repo / "work/loop-memory/perf/renderer/sb"
        powershell = Path(os.environ.get("SystemRoot", r"C:\Windows")) / "System32/WindowsPowerShell/v1.0/powershell.exe"
        failures, errors, all_builds = [], [], []
        skipped = None
        captures_started = 0
        try:
            for name, path in (("run_scene_ps1", script), ("renderer_gate_py", gate)):
                result.digests[name] = wb.sha256_file(path)
            if not executable.is_file() or ctx.options["rebuild_probe"]:
                ctx.say("Building the S-B probe before renderer captures.")
                build = ctx.run(["cmd.exe", "/d", "/c", str(probe_dir / "build.cmd")],
                                cwd=probe_dir, timeout=1800)
                if build.timed_out or build.returncode != 0 or not executable.is_file():
                    result.reason = wb.clip("probe build failed: " + (
                        _last_line(build.output) or ("timed out after 1800 s" if build.timed_out else "sb_probe.exe missing")))
                    return result
            result.digests["sb_probe_exe"] = wb.sha256_file(executable)
            ctx.manual("Renderer captures: mains power plugged in, NVIDIA GPU in high-performance mode, display on with no sleep or screen saver, other applications and overlays closed, nothing over the probe window; each run needs your yes after you watched it.")
            skipped = ctx.require_quiet()
            if skipped:
                result.status = wb.SKIPPED
                result.reason = wb.clip(skipped)
                return result
            overlays = ctx.ask("Overlays running during the captures", OVERLAYS_DEFAULT)
            vendor = ctx.ask("GPU and performance mode set in the vendor software", VENDOR_DEFAULT)
            (ctx.private / "declared.json").write_text(json.dumps(
                {"overlays": overlays, "vendor_mode": vendor}, indent=1) + "\n", encoding="utf-8", newline="\n")
            result.details["declared"] = {
                "overlays": overlays if overlays == OVERLAYS_DEFAULT else PRIVATE_ANSWER,
                "vendor_mode": vendor if vendor == VENDOR_DEFAULT else PRIVATE_ANSWER}
            env = dict(os.environ)
            path_key = next((key for key in env if key.upper() == "PATH"), "PATH")
            old_path = env.pop(path_key, "")
            env["PATH"] = str(Path(sys.executable).parent) + os.pathsep + old_path
            for scene in scenes:
                ctx.manual(f"Next: {scene.upper()} captures, {runs} runs of about 3.5 minutes with 20 s pauses; watch each run with nothing covering the probe window and answer yes after it.")
                skipped = ctx.require_quiet()
                if skipped:
                    break
                before = _directories(captures)
                completed = ctx.run([str(powershell), "-NoProfile", "-File", str(script),
                                     "-Scene", scene, "-Runs", str(runs), "-Overlays", overlays,
                                     "-Declare", f"vendor_mode={vendor}"],
                                    timeout=runs * 600 + 600, cwd=ctx.repo, env=env, interactive=True)
                captures_started += 1
                detail = {"returncode": completed.returncode, "verdict": "no-data", "valid_runs": 0,
                          "invalid_run_reasons": [], "pooled": {"n": None, "fps": None, "p99_ms": None},
                          "vram_peak_mb": None, "build_identities": [], "run_ids": []}
                result.details["scenes"][scene] = detail
                scene_record = None
                issue = ""
                try:
                    new = _directories(captures) - before
                    summaries = sorted(name for name in new if re.fullmatch(STAMP + "-" + re.escape(scene) + "-summary", name))
                    if len(summaries) != 1:
                        raise ValueError("could not identify summary directory (%s new)" % len(summaries))
                    summary_path = captures / summaries[0] / "summary.json"
                    if not summary_path.is_file():
                        raise ValueError("missing summary.json")
                    result.digests["summary_" + scene] = wb.sha256_file(summary_path)
                    summary = json.loads(summary_path.read_text(encoding="utf-8"))
                    scene_record = next((record for record in summary["scenes"] if record["scene"].lower() == scene), None)
                    if scene_record is None:
                        raise ValueError("summary has no %s scene" % scene.upper())
                    valid = scene_record["runs"]
                    invalid = [entry for entry in summary.get("invalid_runs", [])
                               if not entry.get("scene") or entry["scene"].lower() == scene]
                    # Unscoped unreadable entries belong to this single-scene invocation.
                    unreadable = [entry for entry in summary.get("unreadable", [])
                                  if not entry.get("scene") or entry["scene"].lower() == scene]
                    invalid_reasons = [wb.clip(reason) for entry in invalid
                                       for reason in entry.get("reasons", [entry.get("reason", "invalid run")])]
                    invalid_reasons.extend(wb.clip(entry.get("reason", "unreadable run")) for entry in unreadable)
                    pooled = scene_record.get("pooled") or {}
                    identities = [record.get("build_identity") for record in valid]
                    all_builds.extend(identities)
                    detail.update(verdict=scene_record["verdict"], valid_runs=len(valid), invalid_run_reasons=invalid_reasons,
                                  pooled={key: pooled.get(key) for key in ("n", "fps", "p99_ms")},
                                  vram_peak_mb=scene_record.get("vram_peak_mb"),
                                  build_identities=sorted({value for value in identities if isinstance(value, str)}),
                                  run_ids=[record["run_id"] for record in valid])
                    result.counts["valid_runs"] += len(valid)
                    gate_scene = scene_record.get("gate_scene", scene == "w3")
                    if gate_scene and detail["verdict"] == "not-met":
                        failures.append(scene.upper() + ": not-met")
                    verdicts = ("met", "not-met") if gate_scene else ("attribution",)
                    complete = (completed.returncode == 0 and not completed.timed_out and len(valid) == runs
                                and not invalid and not unreadable and detail["verdict"] in verdicts)
                except (OSError, ValueError, KeyError, TypeError, AttributeError) as error:
                    complete = False
                    issue = wb.clip(error)
                if complete:
                    result.counts["scenes_complete"] += 1
                    builds = detail["build_identities"]
                    build_label = builds[0][:8] if len(builds) == 1 else "mixed"
                    result.lines.append(wb.clip("%s: %s; %s valid runs of build %s; pooled %s fps, p99 %s ms; peak VRAM %s MB" % (
                        scene.upper(), detail["verdict"], detail["valid_runs"], build_label,
                        _number(detail["pooled"]["fps"], 2), _number(detail["pooled"]["p99_ms"], 3),
                        _number(detail["vram_peak_mb"], 1))))
                else:
                    message = "%s: no complete capture (script exit %s; %s of %s runs valid%s)" % (
                        scene.upper(), completed.returncode, detail["valid_runs"], runs,
                        "; " + issue if issue else "; " + ", ".join(detail["invalid_run_reasons"]) if detail["invalid_run_reasons"] else "")
                    errors.append(wb.clip(message))
                    result.lines.append(wb.clip(message))
            if ctx.options["faults"] and not skipped:
                ctx.manual("Next: four 10-second W3 fault runs; they need no answers.")
                for fault in FAULTS:
                    # Heavy work can start between runs, so every run has its own guard.
                    skipped = ctx.require_quiet()
                    if skipped:
                        break
                    before = _directories(captures)
                    completed = ctx.run([str(powershell), "-NoProfile", "-File", str(script),
                                         "-Scene", "w3", "-Runs", "1", "-Duration", "10", "-Inject", fault],
                                        timeout=900, cwd=ctx.repo, env=env)
                    captures_started += 1
                    result.counts["faults_run"] += 1
                    detail = {"returncode": completed.returncode, "status": None, "counts": {},
                              "injection_applied": None, "gate_refused": None}
                    result.details["faults"][fault] = detail
                    try:
                        missed = _judge_fault(captures, _directories(captures) - before, completed, detail)
                    except (OSError, ValueError, KeyError, TypeError, AttributeError) as error:
                        errors.append(wb.clip("fault %s: %s" % (fault, error)))
                        continue
                    if missed:
                        failures.append("fault %s: %s" % (fault, missed))
                    else:
                        result.counts["faults_caught"] += 1
                result.lines.append("Faults: %s of %s caught by the label check and refused by the gate" % (
                    result.counts["faults_caught"], result.counts["faults_run"]))
        except (OSError, ValueError, KeyError, TypeError, AttributeError) as error:
            errors.append(wb.clip(error))
        if all_builds and all(value == all_builds[0] for value in all_builds):
            identity = all_builds[0]
            if isinstance(identity, str) and re.fullmatch(r"[0-9a-fA-F]{64}", identity):
                result.digests["build_identity"] = identity.lower()
        if skipped:
            errors.append("remaining captures skipped: " + str(skipped))
        if failures:
            result.status = wb.FAIL
        elif errors and (not skipped or captures_started):
            result.status = wb.ERROR
        elif skipped:
            result.status = wb.SKIPPED
        else:
            result.status = wb.PASS
        result.reason = wb.clip("; ".join(failures + errors))
        result.lines = result.lines[:wb.MAX_LINES]
        return result


STEP = RendererStep()
