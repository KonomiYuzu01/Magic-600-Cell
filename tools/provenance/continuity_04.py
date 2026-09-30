"""Map the 0.4 release native build receipt onto this repository, file by file.

The 0.4 native host was built from a private workspace. Its receipt (build.json,
evidence_version 1) binds every input by SHA-256. This check proves, without
rebuilding anything:

- the stored v1 payload rehashes to the recorded build identity;
- that identity and the executable hash equal docs/RELEASE_0_4_PROVENANCE.json;
- every bound source, model and harness file has the same raw bytes here.

It cannot recreate the build: the release toolchain (CPython 3.12.14, NumPy
2.3.5, csc 4.8.9232.0 on Windows 10) differs from later machines, and the
compiler is not deterministic. The toolchain is therefore recorded, not checked.

Usage:
  python tools/provenance/continuity_04.py <private build.json> [--record <out.json>]

The receipt stays private; only the sanitized record may be published. It holds
repository-relative paths, hashes and tool versions, never tool locations.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PROVENANCE = "docs/RELEASE_0_4_PROVENANCE.json"
V1_PAYLOAD = ("evidence_version", "sources", "model", "toolchain")
TOOL_ROLES = (("python.exe", "python"), ("numpy/__init__.py", "numpy-init"),
              ("_multiarray_umath", "numpy-multiarray"), ("csc.exe", "csc"))


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def posix(key: str) -> str:
    return key.replace("\\", "/")


def v1_identity(receipt: dict) -> str:
    """native_launch.build() hashed the v1 manifest before it added any other field; checks were empty."""
    payload = {k: receipt[k] for k in V1_PAYLOAD}
    payload["checks"] = {}
    return hashlib.sha256(json.dumps(payload, sort_keys=True).encode()).hexdigest()


def bound_inputs(receipt: dict) -> dict:
    inputs = {}
    for group in (receipt["sources"], receipt["model"]["manifest"], receipt["model"]["assets"]):
        for key, digest in group.items():
            path = posix(key)
            if path.startswith("../") or ":" in path or path.startswith("/"):
                raise ValueError(f"receipt input outside the repository: {path}")
            inputs[path] = digest
    return inputs


def tool_role(key: str) -> str:
    path = posix(key)
    for suffix, role in TOOL_ROLES:
        if suffix in path:
            return role
    raise ValueError("unrecognised toolchain file in the receipt")


def check(receipt: dict, root: Path = ROOT) -> dict:
    if receipt.get("evidence_version") != 1:
        raise ValueError("expected an evidence_version 1 receipt")
    provenance = json.loads((root / PROVENANCE).read_text(encoding="utf-8"))["native"]
    inputs = {}
    for path, digest in sorted(bound_inputs(receipt).items()):
        file = root / path
        inputs[path] = {"sha256": digest,
                        "here": "missing" if not file.is_file() else ("same" if sha256(file) == digest else "different")}
    toolchain = receipt["toolchain"]
    record = {
        "release": "0.4",
        "build_identity": receipt["build_identity"],
        "executable_sha256": receipt["executable_sha256"],
        "payload_rehashes_to_identity": v1_identity(receipt) == receipt["build_identity"],
        "matches_release_provenance": (receipt["build_identity"] == provenance["build_identity"]
                                       and receipt["executable_sha256"] == provenance["executable_sha256"]),
        "after_build_check": receipt.get("checks", {}).get("after_build", {}).get("status"),
        "inputs": inputs,
        "summary": {state: sum(1 for i in inputs.values() if i["here"] == state) for state in ("same", "different", "missing")},
        "toolchain": {
            "python": toolchain["python"]["version"].split()[0],
            "bits": toolchain["python"]["bits"],
            "numpy": toolchain["numpy"]["version"],
            "compiler": toolchain["compiler"]["banner"],
            "target": toolchain["compiler"]["target"],
            "files_by_role": {tool_role(k): v for k, v in sorted(toolchain["files"].items())},
        },
        "limitations": [
            "The build is not recreated: the release toolchain differs from later machines and the compiler is not deterministic.",
            "Toolchain hashes are recorded from the receipt, not checked against any local file.",
        ],
    }
    record["summary"]["total"] = len(inputs)
    return record


def sanitized(record: dict) -> str:
    text = json.dumps(record, indent=2, sort_keys=True) + "\n"
    for marker in ("..", ":\\", ":/", "\\", "Users", "AppData", ".cache"):
        if marker in text:
            raise ValueError(f"record would contain a private path fragment ({marker!r}); nothing written")
    return text


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("receipt", type=Path, help="the private 0.4 release build.json")
    parser.add_argument("--record", type=Path, help="write the sanitized record here")
    args = parser.parse_args(argv)
    record = check(json.loads(args.receipt.read_text(encoding="utf-8")))
    text = sanitized(record)
    if args.record:
        args.record.write_text(text, encoding="utf-8", newline="\n")
    s = record["summary"]
    print(f"identity rehash: {record['payload_rehashes_to_identity']}; "
          f"matches release provenance: {record['matches_release_provenance']}; "
          f"inputs: {s['same']} same, {s['different']} different, {s['missing']} missing of {s['total']}")
    ok = record["payload_rehashes_to_identity"] and record["matches_release_provenance"] and s["same"] == s["total"]
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
