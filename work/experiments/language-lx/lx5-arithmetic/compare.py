"""Byte-for-byte LX-5 comparison. A difference removes the entire candidate."""
import argparse
import json
from pathlib import Path
import sys

sys.dont_write_bytecode = True
from common import ROOT, reference_bytes, sha
from reference import load


def compare(expected, actual, cases, values):
    wanted = expected.splitlines(keepends=True)
    got = actual.splitlines(keepends=True)
    assert len(wanted) == len(cases)
    counts = {size: {"cases": 0, "equal": 0, "different": 0} for size in sorted({c["size"] for c in cases})}
    differences = []
    for index, (line, case) in enumerate(zip(wanted, cases)):
        assert line.startswith((case["id"] + "\t").encode("ascii")) and line.endswith(b"\n")
        result = got[index] if index < len(got) else b""
        row = counts[case["size"]]
        row["cases"] += 1
        row["equal" if result == line else "different"] += 1
        if result != line:
            offset = next((i for i, pair in enumerate(zip(line, result)) if pair[0] != pair[1]), min(len(line), len(result)))
            if len(differences) < 8:
                differences.append({"case": case["id"], "size": case["size"], "operation": case["op"],
                    "operand_refs": case["args"],
                    "operands": case["args"] if case["op"] == "word" else [str(values[k]) for k in case["args"]],
                    "first_byte_offset": offset, "expected_sha256": sha(line), "actual_sha256": sha(result),
                    "expected_excerpt": line[max(0, offset - 24):offset + 72].decode("ascii", errors="replace"),
                    "actual_excerpt": result[max(0, offset - 24):offset + 72].decode("ascii", errors="replace")})
    extra = max(0, len(got) - len(wanted))
    bad = sum(r["different"] for r in counts.values()) + extra
    return {"status": "removed" if bad else "exact", "difference_count": bad, "extra_lines": extra,
            "by_size": counts, "first_differences": differences,
            "expected_sha256": sha(expected), "actual_sha256": sha(actual), "actual_bytes": len(actual)}


def self_test():
    cases = [dict(id="a", size="test", op="add", args=["one", "two"]),
             dict(id="b", size="test", op="sub", args=["one", "two"])]
    values = {"one": 1, "two": 2}
    wanted = b"a\t3\nb\t-1\n"
    assert compare(wanted, wanted, cases, values)["status"] == "exact"
    for changed in (b"a\t4\nb\t-1\n", wanted[:-1], wanted + b"x\t0\n", b"b\t-1\na\t3\n", wanted.replace(b"\n", b"\r\n"), b""):
        report = compare(wanted, changed, cases, values)
        assert report["status"] == "removed" and report["difference_count"] > 0


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("answer", type=Path)
    args = parser.parse_args()
    values, cases = load(ROOT / "fixtures/operands.tsv")
    result = compare(reference_bytes(), args.answer.read_bytes(), cases, values)
    print(json.dumps(result, indent=2))
    raise SystemExit(0 if result["status"] == "exact" else 1)
