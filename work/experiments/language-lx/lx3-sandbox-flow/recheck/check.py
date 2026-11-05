"""Reproduce stricter observations; negative prototype results are valid evidence."""

import sys
sys.dont_write_bytecode = True

from runner import controls, frozen_pass
from probes import collect as taint_probes
from admission import collect as admission_probe
from harness.common import write_json
from support import BUILD, HERE, clean_build, file_pins, inputs, read, snapshot, verify_inputs


def main():
    before, frozen_before = snapshot(HERE), inputs()
    owned, success = False, False
    try:
        assert not BUILD.exists(), "build must be absent before acceptance"
        manifest = read(HERE / "MANIFEST.json")
        assert manifest["files"] == {k: v for k, v in before.items() if k != "MANIFEST.json"}, "recheck inventory/pins changed"
        assert manifest["input_files"] == frozen_before, "experiment input pins changed"
        file_pins(HERE / "MANIFEST.json")
        versions = verify_inputs()
        BUILD.mkdir()
        owned = True
        actual = {"version": 1, "decision": "open", "decider": "owner", "versions": versions,
                  "controls": controls(BUILD), "frozen": frozen_pass(BUILD),
                  "taint": taint_probes(BUILD), "admission_variant": admission_probe(BUILD)}
        write_json(BUILD / "recheck.json", actual)
        assert actual == read(HERE / "recheck.json"), "stored outcome did not repeat"
        success = True
    except Exception as error:
        print("FAIL:", str(error) if isinstance(error, AssertionError) else type(error).__name__, flush=True)
    finally:
        if owned:
            clean_build()
        if snapshot(HERE) != before or inputs() != frozen_before:
            success = False
            print("FAIL: retained or frozen files changed", flush=True)
    if success:
        print("PASS: pins, controls and all stored outcomes repeat; build removed; folder byte-identical.")
    return 0 if success else 1


if __name__ == "__main__":
    raise SystemExit(main())
