"""Read-only acceptance: pins, mechanical rendering, one matrix pass and replay.

A reproducible failed form is a result, not an acceptance failure. No evidence,
report, source pin or timing is refreshed by this command.
"""

import sys

sys.dont_write_bytecode = True

import copy
import json
from pathlib import Path
import shutil
import subprocess

FOLDER = Path(__file__).resolve().parent
ROOT = FOLDER.parents[1]
sys.path.insert(0, str(ROOT))
sys.path.insert(0, str(FOLDER))

from harness.bindings import candidate
from harness.common import CORPUS, FREEZE_SHA256, canonical, digest, sha
from prepare import artifacts, observations
from subset import SubsetError, approve


def snapshot():
    return {path.relative_to(FOLDER).as_posix(): sha(path.read_bytes())
            for path in sorted(FOLDER.rglob("*")) if path.is_file()}


def corpus_pins():
    raw = (CORPUS / "FREEZE.json").read_bytes()
    assert sha(raw) == FREEZE_SHA256, "corpus FREEZE pin changed"
    frozen = json.loads(raw)
    assert set(frozen["files"]) == {path.name for path in CORPUS.iterdir() if path.is_file()}, "corpus file inventory changed"
    for name, expected in frozen["files"].items():
        if name != "FREEZE.json":
            assert sha((CORPUS / name).read_bytes()) == expected, "corpus file changed: " + name
    normalized = copy.deepcopy(frozen)
    normalized["files"]["FREEZE.json"] = None
    assert digest(normalized) == frozen["files"]["FREEZE.json"], "corpus normalized self hash changed"
    assert digest({k: v for k, v in frozen["files"].items() if k != "FREEZE.json"}) == frozen["content_sha256"]
    # Historical source-document digests inside keys.json are frozen evidence,
    # not a demand that moving chapters/decisions still have their old bytes.


def one_pass(summary):
    result = copy.deepcopy(summary)
    runs = summary["runs_per_key_per_leg"]
    assert summary["legs"] == ["Windows"] and runs >= 1
    assert summary["row_count"] == 522 * runs, "missing matrix outcomes"
    result.update(row_count=522, runs_per_key_per_leg=1)
    assert len(result["cells"]) == 6
    for cell in result["cells"]:
        assert not cell["reproducibility_counterexamples"], "stored reproducibility counterexample"
        for entries in cell["hashes"].values():
            assert {entry["repeat"] for entry in entries} == set(range(runs)), "missing repeat"
            assert len(entries) == runs and all(entry["leg"] == "Windows" for entry in entries)
            assert len({(entry["sha256"], entry["run_key"]) for entry in entries}) == 1, "payload or key does not repeat"
        cell["hashes"] = {key: [entry for entry in entries if entry["repeat"] == 0]
                          for key, entries in cell["hashes"].items()}
        for field in ("not_run", "budget_cache_interruption"):
            cell[field] = [entry for entry in cell[field] if entry["repeat"] == 0]
        assert all(statuses for group in cell["outcomes"].values() for statuses in group.values()), "missing outcome"
    return result


def subset_probes():
    # Check the actual Python boundary, without using a case or expected code.
    for source in (
        "import os\n", "def r_00(lib):\n    return open('x')\n",
        "def r_00(lib):\n    return lib.__class__\n",
        "def r_00(lib):\n    lib = None\n",
        "def r_00(lib):\n    return lib.input.__globals__\n",
    ):
        try:
            approve(source.encode())
        except SubsetError:
            pass
        else:
            raise AssertionError("unsafe host construct admitted")


def main():
    before = snapshot()
    build = (FOLDER / "build").resolve()
    success, created = False, False
    try:
        assert not build.exists(), "build must be absent before acceptance"
        assert build.parent == FOLDER and build.name == "build"
        created = True
        corpus_pins()
        manifest = json.loads((FOLDER / "MANIFEST.json").read_bytes())
        assert manifest["files"] == {k: v for k, v in before.items() if k != "MANIFEST.json"}, "artifact pins changed"
        assert manifest["content_sha256"] == digest(manifest["files"])
        declaration, bindings = candidate(FOLDER)
        bundle, index, mapping = artifacts()
        assert bundle == (FOLDER / "programs.py").read_bytes(), "rendered bytes changed"
        assert canonical(index) + b"\n" == (FOLDER / "program-index.json").read_bytes(), "rendered index changed"
        assert mapping == bindings["source_map"], "source map changed"
        subset_probes()
        print("PASS: corpus, candidate pins, Python subset and byte-for-byte rendering", flush=True)
        process = subprocess.run([
            sys.executable, "-B", str(ROOT / "run_matrix.py"), "--candidate", "candidates/rt4-pyhost",
            "--out", "candidates/rt4-pyhost/build/check", "--runs", "1", "--leg", "Windows"],
            capture_output=True, timeout=840)
        assert process.returncode == 0, "matrix did not produce a complete result"
        fresh = json.loads((build / "check/summary.json").read_bytes())
        stored = json.loads((FOLDER / "summary.json").read_bytes())
        assert one_pass(fresh) == one_pass(stored), "run keys, exact payloads or recorded outcomes changed"
        evidence = json.loads((FOLDER / "evidence.json").read_bytes())
        assert evidence["bindings_sha256"] == sha((FOLDER / "bindings.json").read_bytes())
        assert observations(build / "check/results.jsonl") == evidence["observations"], "observer evidence changed"
        print("PASS: all 522 key observations match stored outcomes and exact hashes", flush=True)
        success = True
    except Exception as error:
        print("FAIL:", type(error).__name__, str(error), flush=True)
    finally:
        if created and build.exists():
            assert build.resolve().parent == FOLDER and build.name == "build"
            shutil.rmtree(build)
        if before != snapshot():
            success = False
            print("FAIL: candidate folder changed", flush=True)
    if success:
        print("PASS: candidate folder byte-identical; build removed; form decisions remain open", flush=True)
    return 0 if success else 1


if __name__ == "__main__":
    raise SystemExit(main())
