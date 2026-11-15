"""One fresh Windows pass, frozen pins, exact reproduction, no retained writes."""

import sys

sys.dont_write_bytecode = True

import json
from pathlib import Path
import subprocess

FOLDER = Path(__file__).resolve().parent
ROOT = FOLDER.parents[1]
sys.path.insert(0, str(ROOT))
sys.path.insert(0, str(FOLDER))

from harness.bindings import candidate
from harness.common import CORPUS, FREEZE_SHA256, canonical, cases, digest, sha
from harness.report import expected_rows
from local_checks import checks
from prepare import clean_build, collect_outcomes, tool_identity
from render import render


def snapshot():
    return {p.relative_to(FOLDER).as_posix(): sha(p.read_bytes()) for p in sorted(FOLDER.rglob("*")) if p.is_file()}


def validate_summary(summary, manifest, outcomes):
    assert summary["decision"] == "open" and summary["legs"] == ["Windows"]
    runs = summary["runs_per_key_per_leg"]
    assert summary["row_count"] == len(expected_rows(manifest, runs, ["Windows"]))
    assert outcomes["row_count"] == summary["row_count"] and outcomes["runs"] == runs
    assert set(outcomes["keys"]) == {"/".join((form, variant, case_id, run_id, host_variant))
        for form, variant, case_id, run_id, host_variant, repeat, leg in expected_rows(manifest, 1, ["Windows"])}
    assert len(summary["cells"]) == 6
    for declared, cell in zip(manifest["cells"], summary["cells"]):
        assert (cell["form"], cell["variant"]) == (declared["form"], declared["variant"])
        wanted = {(case_id + "/" + run_id + "/" + host_variant, repeat)
                  for form, variant, case_id, run_id, host_variant, repeat, leg in expected_rows(manifest, runs, ["Windows"])
                  if form == cell["form"] and variant == cell["variant"]}
        actual = {(key, entry["repeat"]) for key, entries in cell["hashes"].items() for entry in entries}
        without_payload = set()
        for key, repeat in wanted:
            evidence = outcomes["keys"][cell["form"] + "/" + cell["variant"] + "/" + key]
            if evidence["sha256"] is None:
                assert evidence["classification"]["status"] in ("crash", "hang"), "missing payload without process outcome"
                assert evidence["run_key"] is None
                without_payload.add((key, repeat))
        assert actual == wanted - without_payload, "missing outcome/payload hash"
        assert not cell["reproducibility_counterexamples"], "nonreproducible candidate"
        for entries in cell["hashes"].values():
            assert len(entries) == runs
            assert len({(e["run_key"], e["sha256"]) for e in entries}) == 1, "key or exact payload changed"


def compare(stored, fresh):
    for a, b in zip(stored["cells"], fresh["cells"]):
        for key in ("form", "variant", "declaration", "reason", "allowed_denominator", "false_refusal_count",
                    "false_refusals", "refusals", "must_refuse_failures", "outcomes", "reproducibility_counterexamples"):
            assert a[key] == b[key], "recorded outcomes changed: " + key
        assert set(a["hashes"]) == set(b["hashes"])
        for key, entries in b["hashes"].items():
            assert {(e["run_key"], e["sha256"]) for e in a["hashes"][key]} == {(e["run_key"], e["sha256"]) for e in entries}, "exact payload changed: " + key
        for key in ("not_run", "budget_cache_interruption"):
            normalized = lambda rows: {canonical({k: v for k, v in row.items() if k not in ("repeat", "leg")}) for row in rows}
            assert normalized(a[key]) == normalized(b[key]), "outcome changed: " + key


def main():
    before, build = snapshot(), FOLDER / "build"
    success, owned = False, False
    try:
        assert not build.exists(), "build must be absent before acceptance"
        owned = True
        assert sha((CORPUS / "FREEZE.json").read_bytes()) == FREEZE_SHA256, "corpus freeze changed"
        frozen = json.loads((CORPUS / "FREEZE.json").read_bytes())
        files = dict(frozen["files"])
        assert set(files) == {p.name for p in CORPUS.iterdir() if p.is_file()}, "corpus file set changed"
        for name, expected in files.items():
            if name != "FREEZE.json":
                assert sha((CORPUS / name).read_bytes()) == expected, "frozen input changed: " + name
        frozen["files"]["FREEZE.json"] = None
        assert sha(canonical(frozen)) == files["FREEZE.json"], "normalized freeze pin changed"
        assert digest({k: v for k, v in files.items() if k != "FREEZE.json"}) == frozen["content_sha256"]
        # Verify frozen experiment bytes, not mutable chapters or decision pages.
        manifest = json.loads((FOLDER / "MANIFEST.json").read_bytes())
        assert manifest["files"] == {k: v for k, v in before.items() if k != "MANIFEST.json"}, "candidate pin changed"
        assert digest(manifest["files"]) == manifest["content_sha256"]
        declaration, bindings = candidate(FOLDER)
        assert tool_identity() == json.loads((FOLDER / "tools.json").read_bytes()), "tool pin changed"
        source = (FOLDER / "programs.js").read_bytes()
        index = json.loads((FOLDER / "program-index.json").read_bytes())
        regenerated = bytearray()
        for _, case in cases():
            data, mapping = render(case["program"])
            offset = len(regenerated)
            regenerated.extend(data)
            assert index[case["id"]] == {"start": offset, "end": len(regenerated), "sha256": sha(data)}
            for path, span in mapping.items():
                expected = {"file": "candidates/rt2-node/programs.js", "start": offset + span["start"],
                            "end": offset + span["end"], "sha256": sha(data[span["start"]:span["end"]])}
                assert bindings["source_map"][case["id"]][path] == expected
        assert source == bytes(regenerated), "rendered bytes changed"
        assert checks() == json.loads((FOLDER / "adversarial.json").read_bytes()), "local probe changed"
        print("PASS: corpus, candidate/tool pins, rendered bytes and local probes", flush=True)
        stored = json.loads((FOLDER / "summary.json").read_bytes())
        stored_outcomes = json.loads((FOLDER / "key-outcomes.json").read_bytes())
        validate_summary(stored, declaration, stored_outcomes)
        subprocess.run([sys.executable, "-B", str(ROOT / "run_matrix.py"), "--candidate", "candidates/rt2-node",
                        "--out", "candidates/rt2-node/build/check", "--runs", "1", "--leg", "Windows"], check=True, timeout=900)
        fresh = json.loads((build / "check/summary.json").read_bytes())
        fresh_outcomes = collect_outcomes(build / "check", declaration)
        validate_summary(fresh, declaration, fresh_outcomes)
        assert stored_outcomes["keys"] == fresh_outcomes["keys"], "per-key outcome changed"
        compare(stored, fresh)
        print("PASS: every run key and exact payload matches the stored Windows series", flush=True)
        success = True
    except Exception as error:
        print("FAIL:", type(error).__name__, str(error), flush=True)
    finally:
        if owned:
            clean_build(build)
        after = snapshot()
        if before != after:
            success = False
            print("FAIL: folder bytes changed", flush=True)
    if success:
        print("PASS: candidate folder byte-identical; build removed; decisions open", flush=True)
    return 0 if success else 1


if __name__ == "__main__":
    raise SystemExit(main())
