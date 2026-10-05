"""Allowlisted public projection of private SA2 runs. No log/error text is copied."""

import argparse
import getpass
import json
import re
from pathlib import Path

FORMAT = "magic600-sa2-smoke-summary-v1"
GODOT_VERSION = "4.7.2.stable.mono.official.ed1daf0bf"
REASONS = frozenset({
    "decode-mismatch", "readback-missing", "verify-mismatch", "debug-errors",
    "refcount-nonzero", "sa2-call-failed", "driver-not-d3d12", "engine-version",
    "texture2drd-refused", "device-removed", "not-drawn", "resource-changed",
    "device-ownership", "viewport-too-small", "dll-load-failed", "godot-call-failed",
    "readback-coverage", "teardown-incomplete", "timeout", "result-missing",
    "result-invalid", "exit-code", "adapter-mismatch", "expectation-mismatch",
    "native-selftest-failed", "drain-message-missing", "config-mismatch", "validation-not-active",
})
# Non-gating judge notes (run_smoke.judge); they never change a judgement.
NOTES = frozenset({"godot-output-errors"})
SELFTEST_FORMAT = "magic600-sa2-native-selftest-v1"
FRAME_FIELDS = ("run", "produced", "drawn", "not_drawn", "eligible", "verified", "mismatched",
                "readbacks_requested", "readbacks_completed")
VERIFY_FIELDS = ("native_checks", "rd_checks", "native_mismatched_texels", "rd_mismatched_texels", "mismatched_texels")
DEBUG_FIELDS = ("struct_size", "distinct_id_count", "corruption", "error", "warning", "info", "message",
               "mismatching_clear_value", "mentioning_sa2")


def _digest(value, length=64):
    if not isinstance(value, str) or not re.fullmatch(r"[0-9a-fA-F]{%d}" % length, value):
        raise ValueError("invalid source/build digest")
    return value.lower()


def _counts(value, fields):
    if not isinstance(value, dict):
        return {}
    return {key: value[key] for key in fields
            if type(value.get(key)) is int and value[key] >= 0}


def _choice(value, allowed):
    if value not in allowed:
        raise ValueError("invalid enumerated summary value")
    return value


def scan_public(value, current_user=None):
    """Scan values and keys after projection; a refused summary is never written."""
    user = getpass.getuser() if current_user is None else current_user
    users = "Use" + "rs"
    private_patterns = [r"[A-Za-z]:[\\/]", r"[\\/]" + users + r"[\\/]",
                        "App" + "Data", r"%[^%\r\n]+%"]

    def visit(item):
        if isinstance(item, str):
            if any(re.search(pattern, item, re.IGNORECASE) for pattern in private_patterns):
                raise ValueError("private path/environment text in public summary")
            if user and user.casefold() in item.casefold():
                raise ValueError("user name in public summary")
        elif isinstance(item, dict):
            for key, child in item.items():
                visit(key)
                visit(child)
        elif isinstance(item, list):
            for child in item:
                visit(child)
    visit(value)


def selftest_status(result):
    """R0's status from its checks; the self-test JSON has no overall status field.

    It passes only if every check passes or is unsupported with a reason, and at
    least one check passes. An explicit overall status, if present, must be pass.
    """
    if not isinstance(result, dict):
        return "fail"
    checks = result.get("checks")
    if isinstance(checks, dict):
        checks = list(checks.values())
    if not isinstance(checks, list):
        return "fail"
    ok = (result.get("format") == SELFTEST_FORMAT and result.get("status", "pass") == "pass"
          and any(isinstance(c, dict) and c.get("status") == "pass" for c in checks)
          and all(isinstance(c, dict) and (c.get("status") == "pass"
                                           or (c.get("status") == "unsupported" and bool(c.get("reason"))))
                  for c in checks))
    return "pass" if ok else "fail"


def driver_version(umd):
    if type(umd) is not int or not 0 < umd < 2 ** 64:
        return None
    return ".".join(str((umd >> shift) & 0xFFFF) for shift in (48, 32, 16, 0))


def _row(row):
    # Free-form row keys and command lines cannot enter the public file.
    if row.get("route") == "native-selftest":
        return {"route": "native-selftest", "debug": True, "hardware": True, "expected": "pass"}
    out = {
        "route": _choice(row.get("route"), {"rd-compute", "export", "import-copy", "import-texture2drd"}),
        "queue": _choice(row.get("queue"), {"same", "own"}),
        "handover": _choice(row.get("handover"), {"tracked", "render-target"}),
        "barriers": _choice(row.get("barriers"), {"match", "legacy", "enhanced"}),
        "render_thread": _choice(row.get("render_thread"), {"safe", "separate"}),
        "expected": _choice(row.get("expected"), {"pass", "recorded", "unsupported", "pending-drain", "device-loss"}),
        "validation": row.get("validation") is True,
    }
    out.update(_counts(row, ("frames", "resize_every", "device_loss_at")))
    return out


def _teardown(value):
    if not isinstance(value, dict):
        return {}
    out = {}
    if value.get("phase") in {"not-started", "pending", "complete", "incomplete"}:
        out["phase"] = value["phase"]
    if type(value.get("drain_result")) is int:
        out["drain_result"] = value["drain_result"]
    elif value.get("drain_result") is None:
        out["drain_result"] = None
    out.update(_counts(value, ("slots_unregistered",)))
    out["detached"] = value.get("detached") is True
    out["refcount_after"] = []
    for entry in value.get("refcount_after", []):
        if isinstance(entry, dict):
            safe = _counts(entry, ("generation", "slot", "refcount_after"))
            if type(entry.get("status")) is int:
                safe["status"] = entry["status"]
            out["refcount_after"].append(safe)
    out["rebuild_drain_results"] = [n for n in value.get("rebuild_drain_results", []) if type(n) is int]
    return out


def build_summary(private, current_user=None):
    source = private["source"]
    builds = private["build"]
    runs = private["runs"]
    engine_run = next((run.get("result", {}) for run in runs
                       if isinstance(run.get("result"), dict) and run["result"].get("engine", {}).get("adapter_name")), {})
    engine = engine_run.get("engine", {})
    device = engine_run.get("device", {})
    summary = {
        "format": FORMAT,
        "source": {"head": _digest(source["head"], 40), "sha256": _digest(source["sha256"]),
                   "matches_head": source.get("matches_head") is True, **_counts(source, ("file_count",))},
        "build": {"dll_sha256": _digest(builds["dll_sha256"]), "assembly_sha256": _digest(builds["assembly_sha256"])},
        "godot": {"version": _choice(builds["godot_version"], {GODOT_VERSION}),
                  "editor_build": engine.get("editor_build") if type(engine.get("editor_build")) is bool else None},
        "adapter": {"name": engine.get("adapter_name"), "driver_version": driver_version(device.get("umd_version"))},
        "enhanced_barriers": bool(device["enhanced_barriers"]) if type(device.get("enhanced_barriers")) is int else None,
        "runs": [],
    }
    if summary["adapter"]["name"] is not None and not isinstance(summary["adapter"]["name"], str):
        raise ValueError("invalid adapter name")
    for run in runs:
        run_id = run["id"]
        if not isinstance(run_id, str) or not re.fullmatch(r"R(?:[0-9]|1[0-3])", run_id):
            raise ValueError("invalid run ID")
        result = run.get("result") or {}
        if run.get("timed_out"):
            status = "timeout"
        elif run["row"].get("route") == "native-selftest" and result:
            status = selftest_status(result)
        else:
            status = result.get("status", "missing")
        notes = run.get("judge_notes")
        item = {
            "id": run_id, "row": _row(run["row"]),
            "status": _choice(status, {"pass", "fail", "recorded", "unsupported", "missing", "timeout"}),
            "judgement": _choice(run["judgement"], {"as-expected", "unexpected"}),
            "reasons": sorted({r for r in result.get("reasons", []) + run.get("judge_reasons", [])
                               if isinstance(r, str) and r in REASONS}),
            "judge_notes": sorted({n for n in (notes if isinstance(notes, list) else [])
                                   if isinstance(n, str) and n in NOTES}),
            "output_line_counts": _counts(run.get("output_line_counts"), ("error", "warning")),
            "frames": _counts(result.get("frames"), FRAME_FIELDS),
            "verify": _counts(result.get("verify"), VERIFY_FIELDS),
            "debug": _counts(result.get("debug"), DEBUG_FIELDS),
            "teardown": _teardown(result.get("teardown")),
            "exit_code": run.get("exit_code") if type(run.get("exit_code")) is int else None,
        }
        frames = result.get("frames", {})
        if isinstance(frames, dict):
            item["frames"]["skipped"] = _counts(frames.get("skipped"), ("warmup", "transition"))
        debug = result.get("debug", {})
        if isinstance(debug, dict):
            item["debug"]["ids"] = [n for n in debug.get("ids", []) if type(n) is int][:16]
        summary["runs"].append(item)
    scan_public(summary, current_user)
    return summary


def write_summary(private, destination, current_user=None):
    summary = build_summary(private, current_user)
    destination = Path(destination)
    destination.parent.mkdir(parents=True, exist_ok=True)
    temporary = destination.with_name(destination.name + ".tmp")
    temporary.write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
    temporary.replace(destination)
    return summary


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("private_summary", type=Path)
    parser.add_argument("--out", type=Path, default=Path(__file__).resolve().parent / "results/sa2-smoke-summary.json")
    args = parser.parse_args()
    try:
        write_summary(json.loads(args.private_summary.read_text(encoding="utf-8")), args.out)
    except (ValueError, KeyError, TypeError) as error:
        parser.exit(1, f"sa2 summary refused: {error}\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
