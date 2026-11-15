"""Recompute counts once, never timings; preserve every retained byte."""

import sys

sys.dont_write_bytecode = True

from support import (BUILD, HERE, clean_build, count_all, read, require,
                     snapshot, verify_pins)


def main():
    require(not BUILD.exists(), "build must be absent before acceptance")
    before = snapshot()
    try:
        verify_pins()
        BUILD.mkdir()
        fresh = count_all()
        require(fresh == read(HERE / "counters.json"), "a stored count, outcome or payload hash did not repeat")
        print("PASS: frozen corpus, all five candidate pin sets, 45 cells, one count pass, "
              "cross-mode results and comparable stored summary hashes; no timing measured.")
    finally:
        clean_build()
        # Raw byte hashes only: timing samples are never parsed or acceptance inputs.
        require(snapshot() == before, "acceptance changed retained folder bytes")
        require(not BUILD.exists(), "acceptance left build products")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
