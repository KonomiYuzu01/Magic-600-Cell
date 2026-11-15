"""Run the shared synthetic bench. See harness/README.md for the interface."""

import sys

sys.dont_write_bytecode = True

from harness.matrix import main


if __name__ == "__main__":
    raise SystemExit(main())
