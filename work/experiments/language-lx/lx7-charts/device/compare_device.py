"""Validate capture binding, then report errors without a precision threshold."""
import sys
sys.dont_write_bytecode = True
from hashlib import sha256
import json
from pathlib import Path
import struct
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from reference import HERE, input_bytes, summarize, json_bytes


def compare(capture):
    fixtures = json.loads((HERE / "fixtures.json").read_bytes())
    manifest = (HERE / "manifest.json").read_bytes()
    if capture["format"] != "lx7-device-capture-v1":
        raise ValueError("capture format mismatch")
    for field, expected in (
            ("manifest_sha256", sha256(manifest).hexdigest()),
            ("input_sha256", sha256(input_bytes(fixtures)).hexdigest()),
            ("hlsl_sha256", sha256((HERE / "generated.hlsl").read_bytes()).hexdigest())):
        if capture[field] != expected:
            raise ValueError(field + " mismatch")
    # Check all frozen source/input bytes, not merely the identity claimed in the capture.
    from freeze import verify_manifest
    verify_manifest()
    if capture["evidence_class"] not in ("windows", "synthetic"):
        raise ValueError("invalid evidence class")
    device = capture["device"]
    if (not isinstance(device["adapter"], str) or not device["adapter"]
            or device["adapter"] != capture["requested_adapter"] or device["software"] is not False
            or device["shader_model"] != "6.2"):
        raise ValueError("named hardware device binding missing")
    if capture["compile_flags"] != ["-T", "cs_6_2", "-E", "main", "-HV", "2021", "-Gis", "-denorm", "preserve", "-O3", "-Ges", "-WX"]:
        raise ValueError("compiler flags differ from frozen profile")
    if len(capture["records"]) != device["records"]:
        raise ValueError("device record count mismatch")
    for field in ("dxil_sha256", "executable_sha256"):
        digest = capture[field]
        if not isinstance(digest, str) or len(digest) != 64 or any(c not in "0123456789abcdef" for c in digest):
            raise ValueError("missing build digest")
    binary = b"".join(struct.pack("<4I", *row) for row in capture["records"])
    if sha256(binary).hexdigest() != capture["output_sha256"]:
        raise ValueError("output digest mismatch")
    result = summarize(fixtures, capture["records"])
    result.update({"evidence_class": capture["evidence_class"], "sandbox_lead": capture["sandbox_lead"],
                   "device": device, "capture_sha256": sha256(json_bytes(capture)).hexdigest(),
                   "performance": "not run here", "chart_admission": "open"})
    return result


if __name__ == "__main__":
    if len(sys.argv) != 2:
        raise SystemExit("Usage: python device/compare_device.py <capture.json>")
    try:
        result = compare(json.loads(Path(sys.argv[1]).read_bytes()))
    except (ValueError, KeyError, TypeError, OSError, struct.error) as error:
        raise SystemExit("Capture refused: " + type(error).__name__) from None
    sys.stdout.buffer.write(json_bytes(result))
