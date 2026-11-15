"""Later WSL leg: plain CPython, no NumPy, same bit inputs and result format."""
import sys
sys.dont_write_bytecode = True

import gzip
import platform

from common import (HERE, digest, json_bytes, read_requests, read_results, request_bytes,
                    verify_candidate, verify_manifest)
from compare_legs import linux_inclusion, load_windows
from make_requests import verify_construction
from run_leg import run


def main():
    assert sys.platform == "linux", "this leg must run under Linux"
    manifest = verify_manifest()
    verify_candidate()
    verify_construction(manifest)
    windows_summary, windows = load_windows()
    step1 = read_requests(HERE / "step1.tsv")
    raw1, _ = run(step1)
    ids = linux_inclusion(windows, read_results(raw1), manifest)
    lookup = {r[0]: r for r in step1}
    step2 = read_requests(HERE / "bisectors.tsv") + [lookup[k] for k in ids]
    raw2, _ = run(step2)
    input2 = request_bytes(step2)
    (HERE / "linux-step2.tsv").write_bytes(input2)
    linux_manifest = {"schema": "lx9-linux-inputs-v1", "decision": "open", "decider": "owner",
                      "parent_manifest_sha256": windows_summary["input_manifest_sha256"],
                      "step2_included_step1_ids": ids, "step2_count": len(step2),
                      "step2_sha256": digest(input2)}
    (HERE / "LINUX-MANIFEST.json").write_bytes(json_bytes(linux_manifest))
    metadata = {"schema": "lx9-linux-v1", "python": platform.python_version(),
                "platform": sys.platform, "evidence_class": "synthetic", "decision": "open", "decider": "owner",
                "input_manifest_sha256": windows_summary["input_manifest_sha256"],
                "candidate_manifest_sha256": windows_summary["candidate_manifest_sha256"],
                "linux_manifest_sha256": digest((HERE / "LINUX-MANIFEST.json").read_bytes()), "results": {}}
    for step, data in ((1, raw1), (2, raw2)):
        assert len(data) < 2_000_000
        packed = gzip.compress(data, mtime=0)
        (HERE / f"linux-pure-step{step}.tsv.gz").write_bytes(packed)
        metadata["results"][str(step)] = {"sha256": digest(packed), "raw_sha256": digest(data),
                                          "requests": len(read_results(data))}
    (HERE / "LINUX.json").write_bytes(json_bytes(metadata))
    print(f"Linux CPython {metadata['python']}: {len(step1)} step-1 and {len(step2)} step-2 requests.")
    print("Run compare_legs.py to compare Windows evidence and prepare any additional amendment evidence.")


if __name__ == "__main__":
    main()
