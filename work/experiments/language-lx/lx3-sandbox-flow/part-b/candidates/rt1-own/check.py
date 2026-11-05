"""Verify pins and reproduction without refreshing any retained evidence."""

from pathlib import Path
import json
import shutil
import subprocess
import sys

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
PART = HERE.parents[1]
sys.path.append(str(PART))

from harness.bindings import candidate
from harness.common import CORPUS, FREEZE_SHA256, canonical, digest, sha
from harness.report import expected_rows, row_identity
from prepare import rendering
from tests import self_test


def snapshot():
    return {p.relative_to(HERE).as_posix(): sha(p.read_bytes())
            for p in sorted(HERE.rglob("*")) if p.is_file()}


def frozen_corpus():
    raw = (CORPUS / "FREEZE.json").read_bytes()
    assert sha(raw) == FREEZE_SHA256, "frozen corpus identity changed"
    freeze = json.loads(raw)
    assert {p.name for p in CORPUS.iterdir() if p.is_file()} == set(freeze["files"]), "corpus file set changed"
    files = {n: sha((CORPUS / n).read_bytes()) for n in freeze["files"] if n != "FREEZE.json"}
    assert files == {n: h for n, h in freeze["files"].items() if n != "FREEZE.json"}, "frozen corpus bytes changed"
    assert digest(files) == freeze["content_sha256"], "corpus content pin changed"
    expected_self = freeze["files"]["FREEZE.json"]
    freeze["files"]["FREEZE.json"] = None
    assert digest(freeze) == expected_self, "normalized corpus self hash changed"


def one_pass(summary):
    """Keep repeat zero after requiring one exact hash/key across all repeats."""
    projected = json.loads(canonical(summary))
    count = projected["runs_per_key_per_leg"]
    assert count >= 1 and projected["legs"] == ["Windows"], "unexpected stored series"
    assert projected["row_count"] == 522 * count, "missing series outcomes"
    projected["runs_per_key_per_leg"], projected["row_count"] = 1, 522
    for cell in projected["cells"]:
        for rows in cell["hashes"].values():
            assert {r["repeat"] for r in rows} == set(range(count)), "missing repeat hash"
            assert len(rows) == count, "duplicate repeat hash"
            assert len({(r["sha256"], r["run_key"]) for r in rows}) == 1, "nonreproducible exact hash or key"
        cell["hashes"] = {k: [r for r in rows if r["repeat"] == 0] for k, rows in cell["hashes"].items()}
        for field in ("not_run", "budget_cache_interruption"):
            cell[field] = [r for r in cell[field] if r["repeat"] == 0]
        assert all(c["kind"] == "display_reference" for c in cell["reproducibility_counterexamples"]), "reproducibility counterexample"
    return projected


def main():
    before, created, success = snapshot(), False, False
    build = (HERE / "build").resolve()
    try:
        assert not build.exists(), "build must be absent before acceptance"
        assert build.parent == HERE and build.name == "build"
        created = True
        frozen_corpus()
        manifest = json.loads((HERE / "MANIFEST.json").read_bytes())
        assert manifest["files"] == {k: v for k, v in before.items() if k != "MANIFEST.json"}, "candidate artifact pin changed"
        assert manifest["content_sha256"] == digest(manifest["files"]), "candidate content digest changed"
        declaration, bindings = candidate(HERE)
        source, index, mapping = rendering()
        (build / "rendered").mkdir(parents=True)
        (build / "rendered/programs.rt1").write_bytes(source)
        (build / "rendered/programs.json").write_bytes(canonical(index) + b"\n")
        for name in ("programs.rt1", "programs.json"):
            assert (build / "rendered" / name).read_bytes() == (HERE / name).read_bytes(), "rendered bytes changed"
        assert mapping == bindings["source_map"], "source mapping changed"
        print("PASS: frozen corpus, candidate/dependency pins and mechanical rendering", flush=True)
        self_test()
        command = [sys.executable, "-B", str(PART / "run_matrix.py"), "--candidate", "candidates/rt1-own",
                   "--out", "candidates/rt1-own/build/check", "--runs", "1", "--leg", "Windows"]
        completed = subprocess.run(command, capture_output=True, timeout=840)
        assert completed.returncode == 0, "matrix command failed"
        rows = [json.loads(line) for line in (build / "check/results.jsonl").read_bytes().splitlines()]
        assert len(rows) == 522 and {row_identity(r) for r in rows} == expected_rows(declaration, 1, ["Windows"]), "missing outcome"
        assert all(r["classification"].get("status") for r in rows), "missing recorded status"
        actual = json.loads((build / "check/summary.json").read_bytes())
        stored = json.loads((HERE / "summary.json").read_bytes())
        assert one_pass(actual) == one_pass(stored), "stored classifications, run keys or exact payload hashes differ"
        print("PASS: all 522 outcomes, expanded run keys and exact payload hashes match stored Windows series", flush=True)
        success = True
    except Exception as error:
        # Exception type only: child/machine diagnostics are not public evidence.
        print("FAIL:", type(error).__name__, str(error) if isinstance(error, AssertionError) else "acceptance operation failed", flush=True)
    finally:
        if created and build.exists():
            assert build.resolve().parent == HERE and build.name == "build"
            shutil.rmtree(build)
        if snapshot() != before:
            print("FAIL: candidate folder changed during acceptance", flush=True)
            success = False
    if success:
        print("PASS: build removed; candidate folder byte-identical", flush=True)
    return 0 if success else 1


if __name__ == "__main__":
    raise SystemExit(main())
