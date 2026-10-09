"""Project private observations through a typed allowlist before publishing."""
import sys
sys.dont_write_bytecode = True

import getpass
import json
import os
from pathlib import Path
import re

REASONS = frozenset("""decode-mismatch readback-missing verify-mismatch debug-errors
refcount-nonzero sa2-call-failed device-removed resource-changed identity-mismatch
backend-not-d3d12 dpr-not-1 qt-version rhi-create-failed texture-create-failed
eligible-coverage unverified-frames teardown-incomplete resize-incomplete
qt-scenegraph-error exception result-write-failed result-missing result-invalid
debug-layer-unavailable
timeout drain-unconfirmed loss-requested loss-invalidated loss-reinitialized
loss-render-resumed loss-new-rhi loss-reason-observed""".split())
PHRASES = ("Failed to", "Device loss detected", "Graphics device lost", "cannot import device",
           "Using imported device", "Using existing native D3D12 device")
PRIVATE = re.compile(r"[a-z]:[\\/]|[\\/]users[\\/]|appdata|%[^%]+%|0x[0-9a-f]{8,}", re.I)
SELFTEST_FORMAT = "magic600-sa2-native-selftest-v1"


def scan(value, username=None):
    username = getpass.getuser() if username is None else username
    if isinstance(value, str):
        if PRIVATE.search(value) or (username and username.casefold() in value.casefold()):
            raise ValueError("summary contains private text")
    elif isinstance(value, dict):
        for key, item in value.items():
            scan(key, username)
            scan(item, username)
    elif isinstance(value, list):
        for item in value:
            scan(item, username)


def selftest_status(result):
    """Q0's status from its checks; the self-test JSON has no overall status field.

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


def integers(source, keys):
    return {key: source[key] for key in keys if type(source.get(key)) is int}


def booleans(source, keys):
    return {key: source[key] for key in keys if type(source.get(key)) is bool}


def enum(value, choices):
    return value if isinstance(value, str) and value in choices else None


def digest(value, lengths=(64,)):
    if isinstance(value, str) and len(value) in lengths and re.fullmatch(r"[0-9a-f]+", value):
        return value
    return None


def driver_version(value):
    if type(value) is not int or not 0 < value < 1 << 64:
        return None
    return ".".join(str((value >> shift) & 65535) for shift in (48, 32, 16, 0))


def row_summary(row):
    result = {}
    for key, choices in {
        "id": {f"Q{i}" for i in range(16)} | {"Q0b"},
        "device": {"qt", "from-rhi", "from-device"},
        "route": {"rhi-upload", "import-copy", "export-copy", "import-direct"},
        "queue": {"same", "own"}, "handover": {"tracked", "declared"},
        "barriers": {"legacy", "match"}, "render_loop": {"threaded", "basic"},
        "expected": {"pass", "recorded", "pending", "bounded-loss"},
    }.items():
        if (value := enum(row.get(key), choices)) is not None:
            result[key] = value
    result.update(integers(row, ("frames", "resize_every", "device_loss_at", "timeout_s")))
    result.update(booleans(row, ("debug_layer", "inject_drain")))
    return result


def run_summary(run):
    result = run.get("result") or {}
    teardown = result.get("teardown", {})
    debug = result.get("debug", {})
    identity = result.get("device", {}).get("identity", {})
    projected = {
        "row": row_summary(run.get("row", {})),
        "status": (selftest_status(result) if run.get("row", {}).get("id") == "Q0"
                   else enum(result.get("status"), {"pass", "fail", "recorded", "unsupported"})),
        "judgement": enum(run.get("judgement"), {"as-expected", "unexpected"}),
        "reasons": sorted({reason for reason in result.get("reasons", [])
                           if isinstance(reason, str) and reason in REASONS}),
        "result_exists": run.get("result_exists") is True,
        "identity": booleans(identity, ("device_matches", "queue_matches", "rhi_matches",
                                        "qt_queue_device_matches", "fallback_detected")),
        "frames": integers(result.get("frames", {}), ("requested", "steps", "run", "warmup",
                                                      "transition", "eligible", "skipped_warmup",
                                                      "skipped_transition")),
        "verify": integers(result.get("verify", {}), ("native_requested", "native_completed",
                                                      "native_mismatched_texels", "texture_requested",
                                                      "texture_completed", "texture_mismatched_texels",
                                                      "composite_requested", "composite_completed",
                                                      "composite_verified", "composite_mismatches")),
        "debug": integers(debug, ("corruption", "error", "warning", "info", "message",
                                  "mismatching_clear_value", "mentioning_sa2", "distinct_id_count")),
        "teardown": integers(teardown, ("drain_result", "slots_unregistered", "device_refcount_after",
                                        "queue_refcount_after")),
        "stderr": integers(run.get("stderr", {}), ("total", *PHRASES)),
        "exit_code": run.get("exit_code") if type(run.get("exit_code")) is int else None,
        "timeout": run.get("timeout") is True,
    }
    projected["debug"]["ids"] = [x for x in debug.get("ids", []) if type(x) is int][:16]
    projected["teardown"].update(booleans(teardown, ("drain_confirmed", "detached")))
    projected["teardown"]["phase"] = enum(teardown.get("phase"), {"pending", "complete", "not-started"})
    projected["teardown"]["refcount_after"] = [x for x in teardown.get("refcount_after", [])
                                              if type(x) is int]
    return projected


def make_summary(private, username=None):
    source = private.get("source", {})
    devices = [(run.get("result") or {}).get("device", {}) for run in private.get("runs", [])]
    device = next((d for d in devices if d.get("adapter_name")), {})
    summary = {
        "format": "magic600-sd-smoke-summary-v1",
        "evidence_kind": enum(private.get("evidence_kind"), {"owner-runtime", "source-fixture"}),
        "source": {"head": digest(source.get("head"), (40, 64)),
                   "sha256": digest(source.get("sha256")), "matches_head": source.get("matches_head") is True},
        "dll_sha256": digest(private.get("dll_sha256")),
        "executable_sha256": digest(private.get("executable_sha256")),
        "qt_version": enum(private.get("qt_version"), {"6.10.3"}),
        "adapter_name": device.get("adapter_name") if isinstance(device.get("adapter_name"), str) else None,
        "driver_version": driver_version(device.get("umd_version")),
        "enhanced_barriers": device.get("enhanced_barriers") is True,
        "deployment": integers(private.get("deployment", {}), ("total_bytes", "file_count")),
        "runs": [run_summary(run) for run in private.get("runs", [])],
    }
    scan(summary, username)
    return summary


def write_summary(private, destination, username=None):
    summary = make_summary(private, username)
    destination = Path(destination)
    destination.parent.mkdir(parents=True, exist_ok=True)
    temporary = destination.with_name(destination.name + ".tmp")
    temporary.write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
    os.replace(temporary, destination)
    return summary


if __name__ == "__main__":
    import argparse
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("private_summary", type=Path)
    parser.add_argument("destination", type=Path)
    args = parser.parse_args()
    try:
        write_summary(json.loads(args.private_summary.read_text(encoding="utf-8")), args.destination)
    except (ValueError, OSError) as error:
        parser.exit(1, f"sd summary: {error}\n")
