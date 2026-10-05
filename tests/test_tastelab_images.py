"""Run each image-tool module with the local Taste Lab interpreter."""
from __future__ import annotations

import argparse
import os
import re
import subprocess
import sys
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    default = ROOT / "tools" / ".venv" / "tastelab" / ("Scripts/python.exe" if os.name == "nt" else "bin/python")
    parser.add_argument("--python", type=Path, default=default)
    parser.add_argument("--require", action="store_true")
    args = parser.parse_args(argv)
    interpreter = args.python.resolve()
    if not interpreter.is_file():
        if args.require:
            print("FAIL: Taste Lab interpreter is missing (--require)")
            return 1
        print("SKIP: Taste Lab interpreter is missing; does not count toward acceptance")
        return 0
    modules = sorted(path for path in (ROOT / "tests").glob("test_tastelab_*.py") if path.name != Path(__file__).name)
    if not modules:
        print("FAIL: no Taste Lab test modules found")
        return 1
    failed, total, skipped_total = False, 0, 0
    for module in modules:
        result = subprocess.run([str(interpreter), "-B", str(module)], cwd=ROOT, capture_output=True, text=True)
        output = result.stdout + result.stderr
        match = re.search(r"Ran ([0-9]+) tests?", output)
        count = int(match[1]) if match else 0
        skip_match = re.search(r"skipped=([0-9]+)", output)
        skipped = int(skip_match[1]) if skip_match else 0
        passed = result.returncode == 0 and count > 0 and (not args.require or skipped < count)
        print(f"{module.name}: {'PASS' if passed else 'FAIL'} ({count} tests, {skipped} skipped)")
        if not passed:
            print(output)
            failed = True
        total += count
        skipped_total += skipped
    print(f"Total: {total} tests, {skipped_total} skipped, {len(modules)} modules")
    return int(failed)


if __name__ == "__main__":
    sys.exit(main())
