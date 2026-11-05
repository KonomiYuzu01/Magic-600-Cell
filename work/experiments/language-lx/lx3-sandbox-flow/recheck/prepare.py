"""Explicit evidence production; acceptance never invokes this script."""

import sys
sys.dont_write_bytecode = True

import argparse
from runner import controls, frozen_pass
from probes import collect as taint_probes
from admission import collect as admission_probe
from harness.common import digest, write_json
from support import BUILD, HERE, clean_build, inputs, snapshot, verify_inputs


def produce():
    assert not BUILD.exists(), "build must be absent"
    before = inputs()
    try:
        versions = verify_inputs()
        BUILD.mkdir()
        result = {"version": 1, "decision": "open", "decider": "owner", "versions": versions,
                  "controls": controls(BUILD), "frozen": frozen_pass(BUILD),
                  "taint": taint_probes(BUILD), "admission_variant": admission_probe(BUILD)}
        write_json(HERE / "recheck.json", result)
    finally:
        clean_build()
        assert inputs() == before, "frozen files changed"


def seal():
    assert not BUILD.exists(), "build must be absent"
    files = {k: v for k, v in snapshot(HERE).items() if k != "MANIFEST.json"}
    write_json(HERE / "MANIFEST.json", {"version": 1, "decision": "open", "files": files,
               "content_sha256": digest(files), "input_files": inputs()})


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--seal", action="store_true")
    args = parser.parse_args()
    seal() if args.seal else produce()
