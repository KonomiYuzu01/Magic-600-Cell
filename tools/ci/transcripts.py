"""Transcripts for PHS.19's cross-platform item: the same pinned inputs on Windows and on headless Linux, with the
exact outputs compared byte for byte (`.github/workflows/lp2-full.yml`).

With C600_TRANSCRIPT_DIR set, the checks that drive the Rust adapters keep their exact outputs there:
- the jumbling adapter's request and answer lines, written by
  `research/jumbling/differential/candidate/rust_jumble.py` itself (the candidate may import only the standard
  library), for the J1 comparisons and the jumbling command route;
- every file the kernel's and the command layer's checks write in their work folders (traces, receipts and the
  cases they derive), through `keep`, called by `tests/test_kernel_oracle.py`, `tests/test_lp3_loop.py` and
  `tests/test_lp3_source_route.py` at the end of their runs.

`python tools/ci/transcripts.py compare A B` compares two such folders, one per system: the same relative file names,
none empty, each pair byte-identical; the first difference of each file is printed. Wall-clock times are no payload:
the adapters print them in summaries that are not kept, and rust_jumble.py drops its ready line's `load_ms`. None of
these outputs carries a display field, so no field is compared at a declared precision.

Exit status: 0 when the folders agree, 1 on any difference, 2 on a usage error.
"""
from __future__ import annotations

import os
import sys
from pathlib import Path

ENVIRONMENT = "C600_TRANSCRIPT_DIR"
SHOWN = 160  # characters of a differing line printed


def keep(check: str, folder: Path) -> None:
    """Copies every file under `folder` to C600_TRANSCRIPT_DIR/<check>/, keeping relative paths; no-op without it."""
    root = os.environ.get(ENVIRONMENT)
    if not root:
        return
    target = Path(root) / check
    for path in sorted(p for p in folder.rglob("*") if p.is_file()):
        out = target / path.relative_to(folder)
        out.parent.mkdir(parents=True, exist_ok=True)
        with open(out, "xb") as handle:
            handle.write(path.read_bytes())


def files(folder: Path) -> dict[str, Path]:
    return {p.relative_to(folder).as_posix(): p for p in sorted(folder.rglob("*")) if p.is_file()}


def first_difference(a: bytes, b: bytes) -> str:
    lines_a, lines_b = a.split(b"\n"), b.split(b"\n")
    for number, (x, y) in enumerate(zip(lines_a, lines_b), 1):
        if x != y:
            return (f"line {number}: {x[:SHOWN].decode('utf-8', 'replace')!r} "
                    f"against {y[:SHOWN].decode('utf-8', 'replace')!r}")
    return f"{len(lines_a)} lines against {len(lines_b)}"


def compare(a: Path, b: Path) -> tuple[list[str], int, int]:
    """Returns the problems, the number of files compared and their total bytes."""
    in_a, in_b = files(a), files(b)
    problems = [f"only in {a.name}: {name}" for name in sorted(set(in_a) - set(in_b))]
    problems += [f"only in {b.name}: {name}" for name in sorted(set(in_b) - set(in_a))]
    if not in_a and not in_b:
        problems.append("no transcript on either system")
    total = 0
    for name in sorted(set(in_a) & set(in_b)):
        x, y = in_a[name].read_bytes(), in_b[name].read_bytes()
        total += len(x)
        if not x or not y:
            problems.append(f"{name}: empty on {a.name if not x else b.name}")
        elif x != y:
            problems.append(f"{name}: {len(x)} bytes against {len(y)}; {first_difference(x, y)}")
    return problems, len(set(in_a) & set(in_b)), total


def main(argv: list[str]) -> int:
    if len(argv) != 3 or argv[0] != "compare" or not all(Path(p).is_dir() for p in argv[1:]):
        print("usage: python tools/ci/transcripts.py compare FOLDER_A FOLDER_B (two existing folders)")
        return 2
    a, b = Path(argv[1]), Path(argv[2])
    problems, count, total = compare(a, b)
    for problem in problems:
        print(f"DIFFERS: {problem}")
    if problems:
        print(f"FAIL: {len(problems)} difference(s) between {a.name} and {b.name}")
        return 1
    print(f"PASS: {count} files, {total} bytes, identical on {a.name} and {b.name}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
