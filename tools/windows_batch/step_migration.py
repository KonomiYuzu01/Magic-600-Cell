"""Run and judge the synthetic Windows migration probes."""
from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import batch_interface as wb

IS_WINDOWS = os.name == "nt"
PROBES = ("p3", "pipeline", "p1", "p2")


def _directories(path):
    return {item.name for item in path.iterdir() if item.is_dir()} if path.is_dir() else set()


def _last_line(output):
    return next((line.strip() for line in reversed(output.splitlines()) if line.strip()), "")


class MigrationStep:
    name = "migration"
    title = "Migration probes P3, pipeline, P1 and P2 (tools/migration_probes)"

    def unavailable(self, ctx):
        if not IS_WINDOWS:
            return "Windows only"
        if not (ctx.repo / "tools/migration_probes/run_probes.py").is_file():
            return "migration probes not in this checkout"
        if not (ctx.repo / "tools/.venv/engine/Scripts/python.exe").is_file():
            return "engine environment tools/.venv/engine is missing (docs/DEVELOPMENT.md)"
        return None

    def run(self, ctx):
        result = wb.StepResult(wb.ERROR, counts=dict.fromkeys((
            "p3_cases", "p3_passed", "pipeline_cases", "pipeline_passed", "p1_c3_attempts",
            "p1_c3_passed_attempts", "p1_controls", "p1_controls_failed", "p2_interruptions",
            "p2_interruptions_passed"), 0))
        ctx.say("Migration probes: they start and stop 0.4 by themselves on synthetic sessions; do not start or close C600 Studio until they finish.")
        probe_dir = ctx.repo / "tools/migration_probes"
        runs = ctx.repo / "work/migration-probes/runs"
        env = dict(os.environ, PYTHONUTF8="1")
        try:
            digest = hashlib.sha256()
            for path in sorted(probe_dir.glob("*.py")):
                digest.update((path.name + "\0" + wb.sha256_file(path) + "\n").encode("utf-8"))
            result.digests["probe_sources"] = digest.hexdigest()
            before = _directories(runs)
            completed = ctx.run([str(ctx.repo / "tools/.venv/engine/Scripts/python.exe"),
                                 str(probe_dir / "run_probes.py"), *PROBES],
                                timeout=7200, cwd=ctx.repo, env=env)
            result.details.update(returncode=completed.returncode, seconds=completed.seconds)
            if completed.timed_out:
                result.reason = wb.clip("migration probes timed out after 7200 s")
                return result
            if completed.returncode == 2:
                result.reason = wb.clip("run_probes.py exited 2: no public result (private text after sanitising, or bad arguments)")
                return result
            if completed.returncode != 0:
                result.reason = wb.clip(_last_line(completed.output) or "run_probes.py exited %s" % completed.returncode)
                return result
            new = _directories(runs) - before
            if len(new) != 1:
                result.reason = wb.clip("could not identify the probe run directory (%s new)" % len(new))
                return result
            run = new.pop()
            result.details["run"] = run
            public_path = runs / run / "public.json"
            if not public_path.is_file():
                result.reason = wb.clip("missing public.json")
                return result
            result.digests["public_json"] = wb.sha256_file(public_path)
            try:
                public = json.loads(public_path.read_text(encoding="utf-8"))
            except (OSError, ValueError) as error:
                result.reason = wb.clip("unreadable public.json: %s" % error)
                return result
            probes = public["probes"]
            missing = [name for name in PROBES if name not in probes]
            if missing:
                result.reason = wb.clip("missing probes: " + ", ".join(missing))
                return result
            result.details["environment"] = public.get("environment", {})
            cases = {name: {case: record.get("passed") is True for case, record in probes[name].items()}
                     for name in ("p3", "pipeline")}
            result.details["cases"] = cases
            failed = []
            for name, records in cases.items():
                result.counts[name + "_cases"] = len(records)
                result.counts[name + "_passed"] = sum(records.values())
                failed.extend(name + ": " + case for case, passed in records.items() if not passed)
            p1 = probes["p1"]
            verdicts = {strategy: record["verdict"] for strategy, record in p1["verdicts"].items()}
            c3 = p1["verdicts"]["C3"]
            controls = p1["controls"]
            failed_controls = [name for name, record in controls.items() if record.get("passed") is False]
            result.counts.update(p1_c3_attempts=c3["attempts"], p1_c3_passed_attempts=c3["passed_attempts"],
                                 p1_controls=len(controls), p1_controls_failed=len(failed_controls))
            result.details.update(p1_verdicts=verdicts, p1_chosen=p1.get("chosen"), failed_controls=failed_controls)
            if c3["verdict"] != "pass":
                failed.append("p1: C3 " + str(c3["verdict"]))
            failed.extend("p1 control: " + name for name in failed_controls)
            p2 = probes["p2"]
            interruptions = p2["interruptions"]
            result.counts.update(p2_interruptions=len(interruptions),
                                 p2_interruptions_passed=sum(record.get("passed") is True for record in interruptions))
            if p2.get("passed") is not True:
                failed.append("p2")
            result.status = wb.FAIL if failed else wb.PASS
            result.reason = wb.clip("; ".join(failed))
            result.lines = [
                "P3 and F20: %s of %s cases pass" % (result.counts["p3_passed"], result.counts["p3_cases"]),
                "Pipeline fixtures: %s of %s pass" % (result.counts["pipeline_passed"], result.counts["pipeline_cases"]),
                wb.clip("P1: C3 %s %s of %s attempts; first passing strategy %s; %s" % (
                    "passes" if c3["verdict"] == "pass" else "fails", c3["passed_attempts"], c3["attempts"],
                    p1.get("chosen"), "controls failed: " + ", ".join(failed_controls) if failed_controls else "no control failed")),
                "P2: %s; %s of %s interruptions pass" % (
                    "pass" if p2.get("passed") is True else "fail", result.counts["p2_interruptions_passed"], len(interruptions)),
                "Run %s took %.0f min" % (run, completed.seconds / 60),
            ]
        except (OSError, ValueError, KeyError, TypeError, AttributeError) as error:
            result.status = wb.ERROR
            result.reason = wb.clip(error)
        return result


STEP = MigrationStep()
