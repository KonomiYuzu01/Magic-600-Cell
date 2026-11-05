"""Reproduce outcomes and pins without refreshing any retained artifact."""

import json
import shutil
import subprocess
import sys
from pathlib import Path

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT))

from harness.bindings import candidate, neutral_paths
from harness.common import CORPUS, FREEZE_SHA256, canonical, cases, digest, sha
from harness.report import expected_rows, row_identity
from probes import observations
from render import render
from tests import self_test


def require(condition, message):
    if not condition:
        raise ValueError(message)


def snapshot():
    return {p.relative_to(HERE).as_posix(): sha(p.read_bytes()) for p in sorted(HERE.rglob("*")) if p.is_file()}


def check_freeze():
    raw = (CORPUS / "FREEZE.json").read_bytes()
    require(sha(raw) == FREEZE_SHA256, "raw corpus FREEZE pin changed")
    freeze = json.loads(raw)
    files = {name: value for name, value in freeze["files"].items() if name != "FREEZE.json"}
    require(all(sha((CORPUS / name).read_bytes()) == value for name, value in files.items()), "frozen corpus bytes changed")
    require(digest(files) == freeze["content_sha256"], "corpus content digest changed")
    self_pin = freeze["files"]["FREEZE.json"]
    freeze["files"]["FREEZE.json"] = None
    require(digest(freeze) == self_pin, "normalized corpus self pin changed")


def check_rendering(bindings):
    expected_index = {}
    for _, case in cases():
        data, spans = render(case["program"])
        file = "candidates/rt2-py/program-" + sha(data)[:24] + ".py"
        require((ROOT / file).read_bytes() == data, "rendered bytes changed")
        require(set(spans) == set(neutral_paths(case["program"])), "renderer omitted branch or instruction")
        expected_index[digest(case["program"])] = file
        require(bindings["source_map"][case["id"]] == {p: {"file": file, **s} for p, s in spans.items()}, "source map changed")
    require((HERE / "programs.json").read_bytes() == canonical(expected_index) + b"\n", "program index changed")


def check_summary(fresh, stored):
    count = stored["runs_per_key_per_leg"]
    require(type(count) is int and count >= 1, "invalid retained repetition count")
    require(stored["legs"] == fresh["legs"] == ["Windows"], "Windows comparison leg changed")
    require(stored["row_count"] == fresh["row_count"] * count, "retained matrix count changed")
    require(len(fresh["cells"]) == len(stored["cells"]) == 6, "omitted cell")
    for actual, baseline in zip(fresh["cells"], stored["cells"]):
        require(set(actual["hashes"]) == set(baseline["hashes"]), "missing exact result key")
        for key, observations_for_key in baseline["hashes"].items():
            require(len(observations_for_key) == count and {o["repeat"] for o in observations_for_key} == set(range(count)), "omitted repeat")
            fresh_hashes = actual["hashes"][key]
            require(len(fresh_hashes) == 1, "omitted fresh payload")
            require(all((o["sha256"], o["run_key"]) == (fresh_hashes[0]["sha256"], fresh_hashes[0]["run_key"])
                        for o in observations_for_key), "exact payload or run key changed: " + key)
        for field in actual:
            if field == "hashes":
                continue
            expected = baseline[field]
            if field in ("budget_cache_interruption", "not_run"):
                expected = [item for item in expected if item["repeat"] == 0]
            require(actual[field] == expected, "outcome changed: " + field)
        require(not actual["reproducibility_counterexamples"], "nonreproducible candidate")


def main():
    before = snapshot()
    build = HERE / "build"
    created, success = False, False
    try:
        require(not build.exists(), "build must be absent before acceptance")
        manifest = json.loads((HERE / "MANIFEST.json").read_bytes())
        require(manifest["files"] == {k: v for k, v in before.items() if k != "MANIFEST.json"}, "artifact manifest mismatch")
        require(manifest["content_sha256"] == digest(manifest["files"]), "artifact content pin changed")
        check_freeze()
        declaration, bindings = candidate(HERE)
        check_rendering(bindings)
        self_test()
        require((HERE / "probes.json").read_bytes() == canonical(observations()) + b"\n", "escape observations changed")
        print("PASS: corpus, candidate pins, byte-for-byte rendering, checker fixtures and escape table", flush=True)
        build.mkdir()
        created = True
        with (build / "check.log").open("wb") as log:
            process = subprocess.run([sys.executable, "-B", str(ROOT / "run_matrix.py"), "--candidate", "candidates/rt2-py",
                                      "--out", "candidates/rt2-py/build/check", "--runs", "1", "--leg", "Windows"],
                                     stdout=log, stderr=log, timeout=840)
        require(process.returncode == 0, "fresh matrix failed")
        fresh = json.loads((build / "check/summary.json").read_bytes())
        stored = json.loads((HERE / "summary.json").read_bytes())
        identities = set()
        for line in (build / "check/results.jsonl").read_bytes().splitlines():
            row = json.loads(line)
            identity = row_identity(row)
            require(identity not in identities, "duplicate observation")
            identities.add(identity)
            require(bool(row["classification"]["status"]), "missing outcome")
            if row["record"] is None:
                require(row["classification"]["status"] in ("crash", "hang"), "missing exact payload")
            else:
                require(row["record"]["sha256"] == digest(row["record"]["exact"]), "payload hash mismatch")
        require(identities == expected_rows(declaration, 1, ["Windows"]), "omitted matrix outcome")
        check_summary(fresh, stored)
        print("PASS:", len(identities), "outcomes; exact hashes and run keys match every retained repetition", flush=True)
        success = True
    except Exception as error:
        # Never retain raw tracebacks, machine paths or diagnostics in artifacts.
        print("FAIL:", type(error).__name__, str(error), flush=True)
    finally:
        if created and build.exists():
            require(build.resolve().parent == HERE and build.name == "build", "cleanup escaped candidate")
            shutil.rmtree(build)
        after = snapshot()
        if before != after:
            success = False
            changes = sorted(k for k in set(before) | set(after) if before.get(k) != after.get(k))
            print("FAIL: folder changed:", ", ".join(changes), flush=True)
    if success:
        print("PASS: candidate folder byte-identical; build removed; form failures remain results", flush=True)
    return 0 if success else 1


if __name__ == "__main__":
    raise SystemExit(main())
