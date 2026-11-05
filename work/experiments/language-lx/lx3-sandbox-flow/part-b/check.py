"""Acceptance: read-only frozen inputs, tests, fresh controls, byte-identical exit."""

import sys

sys.dont_write_bytecode = True

import hashlib
import importlib
import json
from pathlib import Path
import shutil
import subprocess
ROOT = Path(__file__).resolve().parent
CONTROLS = ("candidates/control-open/", "candidates/control-closed/")


def pinned(name):
    """This manifest covers the bench and its controls; a later candidate pins itself."""
    return name != "MANIFEST.json" and (not name.startswith("candidates/") or name.startswith(CONTROLS))


def snapshot():
    return {p.relative_to(ROOT).as_posix(): hashlib.sha256(p.read_bytes()).hexdigest()
            for p in sorted(ROOT.rglob("*")) if p.is_file()}


def check_controls(name, summary):
    assert summary["row_count"] == 522 and not summary["twenty_run_series"]
    assert len(summary["cells"]) == 6
    for cell in summary["cells"]:
        assert cell["allowed_denominator"] == 35
        assert not cell["reproducibility_counterexamples"]
        assert all(not ({"crash", "hang"} & set(values))
                   for group in cell["outcomes"].values() for values in group.values())
        if name == "control-open":
            assert cell["must_refuse_failures"] == 20
            assert cell["false_refusal_count"] == 0
            assert all(values == ["allowed_ok"] for values in cell["outcomes"]["allowed-tools"].values())
            assert all(values == ["planted_read"] for values in cell["outcomes"]["planted"].values())
            assert all(row["status"] == ("not_run" if row["key"].startswith("B10/") else "budget_ok")
                       for row in cell["budget_cache_interruption"])
        else:
            assert cell["false_refusal_count"] == 35
            assert all(row["code"] == "LG3:capability.not_granted" for row in cell["false_refusals"])


def main():
    before = snapshot()
    build = (ROOT / "build").resolve()
    success, created = False, False
    try:
        assert not build.exists(), "build must be absent before acceptance"
        assert build.parent == ROOT and build.name == "build"
        created = True  # Any later generated build output belongs to this check.
        from harness.common import FREEZE_SHA256, CORPUS, digest, sha
        from harness.matrix import run
        assert (CORPUS / "FREEZE.json").is_file(), "missing corpus freeze"
        assert sha((CORPUS / "FREEZE.json").read_bytes()) == FREEZE_SHA256, "corpus freeze digest changed"
        corpus = subprocess.run([sys.executable, "-B", str(CORPUS / "check_corpus.py")],
                                capture_output=True, timeout=120)
        assert corpus.returncode == 0, "corpus checker failed"
        print("PASS: frozen corpus and raw FREEZE pin", flush=True)
        manifest = json.loads((ROOT / "MANIFEST.json").read_bytes())
        assert manifest["files"] == {k: v for k, v in before.items() if pinned(k)}, "artifact manifest changed"
        assert manifest["content_sha256"] == digest(manifest["files"]), "artifact content digest changed"
        for path in sorted((ROOT / "harness").glob("*.py")):
            module = importlib.import_module("harness" if path.stem == "__init__" else "harness." + path.stem)
            module.self_test()
            print("PASS:", path.relative_to(ROOT).as_posix(), flush=True)
        for name in ("control-open", "control-closed"):
            summary = run(ROOT / "candidates" / name, build / name)
            check_controls(name, summary)
            assert (build / name / "summary.json").read_bytes() == (ROOT / "summaries" / (name + ".json")).read_bytes(), "stored control summary changed"
            print("PASS:", name, "522 observations match stored summary", flush=True)
        success = True
    except Exception as error:
        print("FAIL:", type(error).__name__, str(error), flush=True)
    finally:
        if created and build.exists():
            assert build.resolve().parent == ROOT
            shutil.rmtree(build)
        after = snapshot()
        if before != after:
            success = False
            changes = sorted(k for k in set(before) | set(after) if before.get(k) != after.get(k))
            print("FAIL: changed, missing or new files:", ", ".join(changes), flush=True)
    if success:
        print("PASS: all part-b file hashes identical; build removed", flush=True)
    return 0 if success else 1


if __name__ == "__main__":
    raise SystemExit(main())
