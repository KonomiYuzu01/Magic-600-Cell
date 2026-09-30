"""Tests for tools/provenance/continuity_04.py on synthetic receipts (headless; no build)."""
from __future__ import annotations

import hashlib
import json
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools" / "provenance"))
import continuity_04  # noqa: E402


def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


class ContinuityTests(unittest.TestCase):
    def tree(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        root = Path(tmp.name)
        files = {"native/Host.cs": b"class Host {}\r\n", "assets/manifest.json": b"{}\n", "assets/seed.json": b"[1]\n",
                 "work/experiments/magic600-04/tests/run_postapproval.py": b"print(1)\n"}
        for name, data in files.items():
            (root / name).parent.mkdir(parents=True, exist_ok=True)
            (root / name).write_bytes(data)
        receipt = {
            "evidence_version": 1,
            "sources": {"native\\Host.cs": sha(files["native/Host.cs"]),
                        "work\\experiments\\magic600-04\\tests\\run_postapproval.py":
                            sha(files["work/experiments/magic600-04/tests/run_postapproval.py"])},
            "model": {"model_id": "m", "status": "hash-verified", "manifest": {"assets\\manifest.json": sha(b"{}\n")},
                      "assets": {"assets\\seed.json": sha(b"[1]\n")}},
            "toolchain": {"files": {"..\\private\\python\\python.exe": "a" * 64,
                                    "..\\private\\python\\Lib\\site-packages\\numpy\\__init__.py": "b" * 64,
                                    "..\\private\\numpy\\_core\\_multiarray_umath.cp312-win_amd64.pyd": "c" * 64,
                                    "..\\..\\Windows\\Microsoft.NET\\Framework\\v4.0.30319\\csc.exe": "d" * 64},
                          "python": {"version": "3.12.14 (main) [MSC v.1944 64 bit (AMD64)]", "bits": 64,
                                     "executable": "..\\private\\python\\python.exe"},
                          "numpy": {"version": "2.3.5", "origin": "..\\private\\numpy\\__init__.py"},
                          "compiler": {"banner": "Microsoft (R) Visual C# Compiler version 4.8.9232.0",
                                       "path": "..\\..\\Windows\\csc.exe", "target": "x86"},
                          "platform": "Windows-10-10.0.19045-SP0"},
        }
        receipt["build_identity"] = continuity_04.v1_identity(receipt)
        receipt.update(executable_sha256="e" * 64, checks={"after_build": {"status": "unchanged"}},
                       source={"native\\Host.cs": receipt["sources"]["native\\Host.cs"]})
        (root / "docs").mkdir()
        (root / continuity_04.PROVENANCE).write_text(json.dumps({"native": {
            "build_identity": receipt["build_identity"], "executable_sha256": "e" * 64}}), encoding="utf-8")
        return root, receipt

    def test_matching_tree_is_continuous_and_the_record_is_sanitized(self):
        root, receipt = self.tree()
        record = continuity_04.check(receipt, root)
        self.assertTrue(record["payload_rehashes_to_identity"])
        self.assertTrue(record["matches_release_provenance"])
        self.assertEqual(record["summary"], {"same": 4, "different": 0, "missing": 0, "total": 4})
        self.assertIn("work/experiments/magic600-04/tests/run_postapproval.py", record["inputs"])
        self.assertEqual(set(record["toolchain"]["files_by_role"]), {"python", "numpy-init", "numpy-multiarray", "csc"})
        text = continuity_04.sanitized(record)
        for fragment in ("private", "..", "\\", "19045", '"executable":', '"origin":', '"path":'):
            self.assertNotIn(fragment, text)

    def test_changed_and_missing_inputs_are_reported(self):
        root, receipt = self.tree()
        (root / "assets/seed.json").write_bytes(b"[1]\r\n")  # a line-ending conversion counts as a change
        (root / "native/Host.cs").unlink()
        record = continuity_04.check(receipt, root)
        self.assertEqual(record["inputs"]["assets/seed.json"]["here"], "different")
        self.assertEqual(record["inputs"]["native/Host.cs"]["here"], "missing")
        self.assertEqual(record["summary"]["same"], 2)

    def test_an_edited_payload_no_longer_rehashes(self):
        root, receipt = self.tree()
        receipt["toolchain"]["numpy"]["version"] = "2.3.4"
        self.assertFalse(continuity_04.check(receipt, root)["payload_rehashes_to_identity"])

    def test_inputs_outside_the_repository_and_private_records_are_refused(self):
        root, receipt = self.tree()
        receipt["sources"]["..\\secret.cs"] = "f" * 64
        with self.assertRaises(ValueError):
            continuity_04.check(receipt, root)
        with self.assertRaises(ValueError):
            continuity_04.sanitized({"note": "C:/Users/someone/file"})

    def test_only_v1_receipts_are_accepted(self):
        root, receipt = self.tree()
        receipt["evidence_version"] = 2
        with self.assertRaises(ValueError):
            continuity_04.check(receipt, root)

    def test_published_record_matches_the_repository(self):
        record = json.loads((ROOT / "docs/RELEASE_0_4_CONTINUITY.json").read_text(encoding="utf-8"))
        provenance = json.loads((ROOT / continuity_04.PROVENANCE).read_text(encoding="utf-8"))["native"]
        self.assertEqual(record["build_identity"], provenance["build_identity"])
        self.assertTrue(record["payload_rehashes_to_identity"])
        for path, entry in record["inputs"].items():
            with self.subTest(path=path):
                self.assertEqual(sha((ROOT / path).read_bytes()), entry["sha256"])


if __name__ == "__main__":
    unittest.main()
