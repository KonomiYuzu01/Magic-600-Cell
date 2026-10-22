"""Acceptance checks for the tutorial diagrams, with outputs confined to /tmp."""

import copy
import hashlib
import importlib.util
import json
from pathlib import Path, PurePosixPath
import re
import subprocess
import sys
import tempfile
import unittest
import xml.etree.ElementTree as ET


HERE = Path(__file__).resolve().parent
SCRIPT = HERE / "diagrams.py"
ROOT = HERE.parents[3]
spec = importlib.util.spec_from_file_location("tutorial_diagrams", SCRIPT)
D = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = D
spec.loader.exec_module(D)
NS = {"s": D.SVG_NS}

EXPECTED = {
    "01-menu-items", "01-height-histograms", "01-cover-graph-7101",
    "01-phase-transitions", "01-step-tree-closure", "bulk-n-7101",
    "bulk-conj-7101-s30", "03-counts-7302", "03-r22d75-prediction-cap64",
    "03-call-flow", "ch04-family-order", "ch04-tier-shapes", "ch04-d19-tree",
    "ch04-six-stops-caps", "05-tail-loop", "05-tree-b07-certificate",
    "05-e2-trajectory", "ch06-phase-flow", "ch06-nu-grid", "ch06-twisted-cycle",
    "ch06-orbit6-tree", "ch06-late-three-cycle", "ch06-retained-first",
    "ch06-d20-steps", "07-t1prime-b04-tree", "07-seed7001-bulk-steps",
}


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def load(relative):
    return json.loads((ROOT / relative).read_text(encoding="utf-8"))


class DiagramsTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.temporary = tempfile.TemporaryDirectory(prefix="magic600-diagrams-test-")
        cls.base = Path(cls.temporary.name)
        cls.first, cls.second = cls.base / "first", cls.base / "second"
        cls.manifest = D.generate(cls.first)
        D.generate(cls.second)
        cls.entries = D.validate_trees(load(D.TREES_PATH))
        cls.names = EXPECTED | set(cls.entries)

    @classmethod
    def tearDownClass(cls):
        cls.temporary.cleanup()

    def svg(self, name):
        return ET.parse(self.first / (name + ".svg")).getroot()

    def cli(self, *arguments):
        return subprocess.run([sys.executable, "-I", "-B", str(SCRIPT), *map(str, arguments)],
                              cwd=self.base, text=True, stdout=subprocess.PIPE,
                              stderr=subprocess.PIPE, check=False)

    def test_every_specification_and_configured_tree_is_drawn(self):
        self.assertEqual(set(D.FIGURES), EXPECTED)
        embedded = set()
        for chapter in sorted((ROOT / D.T).glob("0*.md")):
            embedded.update(re.findall(r"\(figures/svg/([a-z0-9-]+)\.svg\)", chapter.read_text(encoding="utf-8")))
        self.assertEqual(embedded, EXPECTED)
        self.assertEqual(set(self.manifest["figures"]), self.names)
        self.assertEqual({p.stem for p in self.first.glob("*.svg")}, self.names)
        self.assertEqual(self.entries["tree-7106-19"], {
            "name": "tree-7106-19", "record": D.DEV + "/d0-s7106/run", "step": 19,
        })

    def test_all_outputs_are_self_contained_svg_11(self):
        for name in sorted(self.names):
            with self.subTest(figure=name):
                root = self.svg(name)
                self.assertEqual(root.tag, "{" + D.SVG_NS + "}svg")
                self.assertEqual(root.get("version"), "1.1")
                view = list(map(float, root.get("viewBox").split()))
                self.assertEqual(view[:3], [0, 0, 1200])
                self.assertGreater(view[3], 100)
                self.assertIn("sans-serif", root.get("font-family"))
                self.assertTrue(root.find("s:title", NS).text.startswith("Figure "))
                self.assertTrue(root.find("s:desc", NS).text)
                self.assertTrue(any(r.get("fill") == D.PALETTE["white"] and r.get("width") == "1200"
                                    and r.get("height") == root.get("height") for r in root.findall("s:rect", NS)))
                for text in root.findall(".//s:text", NS):
                    self.assertGreaterEqual(float(text.get("font-size", root.get("font-size"))), 13)
                self.assertFalse(root.findall(".//s:image", NS))
                self.assertFalse(root.findall(".//s:script", NS))
                for element in root.iter():
                    for attribute, value in element.attrib.items():
                        if attribute.endswith("href"):
                            self.assertTrue(value.startswith("#"))
                metadata = json.loads(root.find("s:metadata", NS).text)
                self.assertEqual(metadata["figure"], name)
                self.assertEqual(metadata["inputs"], self.manifest["figures"][name]["inputs"])
                self.assertIn("diagrams.py --only " + name, metadata["command"])

    def test_two_full_runs_are_byte_identical(self):
        first = {p.name: p.read_bytes() for p in self.first.iterdir()}
        second = {p.name: p.read_bytes() for p in self.second.iterdir()}
        self.assertEqual(first, second)

    def test_manifest_hashes_and_relative_paths(self):
        self.assertEqual(self.manifest["format"], "magic600-tutorial-diagrams/1")
        encoded = json.dumps(self.manifest)
        self.assertNotIn(str(ROOT), encoded)
        self.assertNotRegex(encoded, r"[A-Za-z]:[\\/]")
        for name, entry in self.manifest["figures"].items():
            with self.subTest(figure=name):
                self.assertEqual(entry["output"]["path"], name + ".svg")
                self.assertEqual(entry["output"]["sha256"], digest((self.first / (name + ".svg")).read_bytes()))
                self.assertIn(D.F + "/diagrams.py", entry["inputs"])
                for path, sha in entry["inputs"].items():
                    self.assertFalse(PurePosixPath(path).is_absolute())
                    self.assertNotIn("..", PurePosixPath(path).parts)
                    self.assertEqual(path, PurePosixPath(path).as_posix())
                    self.assertRegex(sha, r"^[0-9a-f]{64}$")
                    self.assertEqual(sha, digest((ROOT / path).read_bytes()))

    def test_check_detects_changed_missing_output_and_provenance_without_writes(self):
        out = self.base / "check"
        D.generate(out)
        command = ("--out", out, "--check")
        clean = self.cli(*command)
        self.assertEqual(clean.returncode, 0, clean.stderr + clean.stdout)
        name = "01-menu-items"
        target = out / (name + ".svg")
        original = target.read_bytes()
        target.write_bytes(original.replace(b"#ffffff", b"#fffffe", 1))
        before = {p.name: p.read_bytes() for p in out.iterdir()}
        changed = self.cli(*command)
        self.assertEqual(changed.returncode, 1, changed.stderr + changed.stdout)
        self.assertIn(name, changed.stdout)
        self.assertEqual(before, {p.name: p.read_bytes() for p in out.iterdir()})
        target.unlink()
        missing = self.cli(*command)
        self.assertEqual(missing.returncode, 1)
        self.assertIn(name, missing.stdout)
        self.assertFalse(target.exists())
        target.write_bytes(original)
        manifest_file = out / "manifest.json"
        document = json.loads(manifest_file.read_text(encoding="utf-8"))
        document["figures"][name]["inputs"][D.F + "/diagrams.py"] = "0" * 64
        manifest_file.write_text(json.dumps(document), encoding="utf-8")
        altered_manifest = manifest_file.read_bytes()
        stale = self.cli(*command)
        self.assertEqual(stale.returncode, 1)
        self.assertIn(name, stale.stdout)
        self.assertEqual(manifest_file.read_bytes(), altered_manifest)

    def test_only_draws_and_checks_the_requested_selection(self):
        out = self.base / "selected"
        name = "tree-7106-19"
        result = self.cli("--out", out, "--only", name)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual({p.name for p in out.iterdir()}, {name + ".svg", "manifest.json"})
        self.assertEqual(set(json.loads((out / "manifest.json").read_text())["figures"]), {name})
        result = self.cli("--out", out, "--only", name, "--check")
        self.assertEqual(result.returncode, 0, result.stderr + result.stdout)
        # Partial redraw of an existing full set preserves every other entry.
        full = self.base / "partial"
        document = D.generate(full)
        untouched = (full / "01-menu-items.svg").read_bytes()
        result = self.cli("--out", full, "--only", name)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(document, json.loads((full / "manifest.json").read_text()))
        self.assertEqual(untouched, (full / "01-menu-items.svg").read_bytes())
        (full / "01-menu-items.svg").write_bytes(b"changed")
        result = self.cli("--out", full, "--only", name, "--check")
        self.assertEqual(result.returncode, 0, result.stderr + result.stdout)

    def test_unknown_selection_fails_without_creating_output(self):
        out = self.base / "unknown"
        result = self.cli("--out", out, "--only", "no-such-figure")
        self.assertEqual(result.returncode, 1)
        self.assertIn("no-such-figure", result.stderr)
        self.assertFalse(out.exists())

    def test_tree_names_paths_indexes_and_collisions_are_validated(self):
        valid = load(D.TREES_PATH)
        invalid_entries = []
        for key, values in (
            ("name", ["../escape", "/absolute", "Upper", "", "x" * 65, "01-menu-items"]),
            ("record", ["/tmp/run", "../run", D.RUNNER + "/runs/../escape", "C:\\run", D.T + "/records"]),
            ("step", [-1, True, "19", 1.5]),
        ):
            for value in values:
                document = copy.deepcopy(valid)
                document["entries"][0][key] = value
                invalid_entries.append(document)
        duplicate = copy.deepcopy(valid)
        duplicate["entries"].append(copy.deepcopy(duplicate["entries"][0]))
        invalid_entries += [duplicate, {"format": "wrong", "entries": []}, {"format": valid["format"], "entries": {}}]
        for document in invalid_entries:
            with self.subTest(document=document):
                with self.assertRaises(ValueError):
                    D.validate_trees(document)

    def test_record_tree_preserves_every_part_and_reference(self):
        e = D.Evidence("tree-7106-19")
        row, tree = D.record_tree(e, D.DEV + "/d0-s7106/run", 19)
        root = self.svg("tree-7106-19")
        text = " ".join(root.itertext())
        for key in (row["rule"], row["phase"], str(row["index"]), D.count(row["letters"])):
            self.assertIn(key, root.find("s:title", NS).text)
        reference_count = 0
        for part, path, depth in D.tree_api().walk(tree):
            self.assertIn(part["name"] + " [" + D.count(part["letters"]) + "]", text)
            for key in ("inverse_of", "same_as"):
                if key in part:
                    reference_count += 1
                    self.assertIn(key + " " + str(part[key]), text)
        arrows = [p for p in root.findall("s:path", NS) if p.get("marker-end")]
        self.assertEqual(len(arrows), reference_count)
        self.assertIn("word 1⁻¹ [0]", text)
        self.assertEqual(row["tree"]["size"], sum(1 for _ in D.tree_api().walk(tree)))

    def test_generic_tree_can_draw_a_different_record_and_step(self):
        entry = {"name": "tree-7001-0", "record": D.DEV + "/d0-s7001/run", "step": 0}
        entries = D.validate_trees({"format": "magic600-tutorial-trees/1", "entries": [entry]})
        raw, manifest = D.render(entry["name"], entries)
        root = ET.fromstring(raw)
        self.assertIn("seam", root.find("s:title", NS).text)
        self.assertIn("phase bulk", root.find("s:title", NS).text)
        self.assertIn("1 letters", root.find("s:title", NS).text)
        self.assertIn("seam [1]", " ".join(root.itertext()))
        self.assertIn(entry["record"] + "/step-trees/00000.json.gz", manifest["inputs"])

    def test_menu_items_and_inverse_links_follow_the_record(self):
        menu = load(D.T + "/records/ch01-examples.json")["A_menu"]
        root = self.svg("01-menu-items")
        labels = [n.text for n in root.findall("s:text", NS) if re.fullmatch(r"\d+", n.text or "")]
        self.assertEqual(sorted(map(int, labels)), list(range(menu["items"])))
        links = root.findall("s:path", NS)
        pairs = sum(i <= inverse for i, inverse in enumerate(menu["inverse"]))
        self.assertEqual(len(links), pairs)

    def test_cover_graph_contains_the_recorded_misaligned_caps(self):
        data = load(D.T + "/records/ch01-examples.json")["F_seed7101"]["before_step_16"]
        root = self.svg("01-cover-graph-7101")
        labels = {int(n.text) for n in root.findall("s:text", NS) if re.fullmatch(r"\d+", n.text or "")}
        self.assertEqual(labels, set(map(int, data["cover_graph"])) | set(data["cover_tops"]))
        self.assertEqual(len(labels), data["misaligned_caps"])

    def test_bulk_chart_uses_cumulative_round_two_once(self):
        census = load(D.REST + "/11-scramble-census/runs/round2/ia-k40-s7101.json")
        root = self.svg("bulk-n-7101")
        titles = [element.text for element in root.findall(".//s:title", NS)]
        observations = [text for text in titles if " · N: " in text]
        self.assertEqual(len(observations), len(census["steps"]) + 1)
        self.assertIn("0 · N: " + D.count(census["steps"][0]["N_before"]), observations)
        for i, row in enumerate(census["steps"], 1):
            self.assertIn(str(i) + " · N: " + D.count(row["N_after"]), observations)

    def test_e2_chart_follows_all_successful_committed_steps(self):
        rows = []
        for directory in ("run1", "run2", "run2b", "run3/t1"):
            path = ROOT / D.ROUND / "line-a/literal" / directory / "seq.jsonl"
            rows += [row for row in map(json.loads, path.read_text().splitlines()) if "off_after" in row and "letters" in row]
        titles = [n.text for n in self.svg("05-e2-trajectory").findall(".//s:title", NS)]
        self.assertIn("0 · off: " + D.count(rows[0]["off_before"]), titles)
        for i, row in enumerate(rows, 1):
            self.assertIn(str(i) + " · off: " + D.count(row["off_after"]), titles)
            kind = "certificate V" if "cert" in row else "literal"
            self.assertIn(str(i) + " · " + kind + ": " + D.count(row["letters"]), titles)

    def test_b04_parts_are_proportional_to_their_recorded_ranges(self):
        tree = load(D.T + "/records/ch07-examples.json")["E_trees"]["b04"]["tree"]
        root = self.svg("07-t1prime-b04-tree")
        parts = [r for r in root.findall("s:rect", NS) if r.get("height") == "70" and r.get("fill") in D.ACCENTS]
        self.assertEqual(len(parts), len(tree["parts"]))
        for rectangle, node in zip(parts, tree["parts"]):
            self.assertAlmostEqual(float(rectangle.get("width")), node["letters"] / tree["letters"] * 1120, delta=.001)
            self.assertAlmostEqual(float(rectangle.get("x")), 40 + node["span"][0] / tree["letters"] * 1120, delta=.001)

    def test_evidence_gaps_are_visible_and_in_the_manifest(self):
        gaps = {
            "03-counts-7302": "u is a gap",
            "03-r22d75-prediction-cap64": "do not give F_h",
            "ch04-six-stops-caps": "aggregate prune counts only",
            "05-tree-b07-certificate": "certificate's contents are not committed",
            "bulk-conj-7101-s30": "after z is not recorded",
            "ch06-twisted-cycle": "complete support of each part",
        }
        for name, phrase in gaps.items():
            notes = self.manifest["figures"][name]["notes"]
            self.assertIn(phrase, " ".join(notes))
            visible = " ".join(n.text or "" for n in self.svg(name).findall("s:text", NS))
            self.assertIn("Data note:", visible)
            for note in notes:
                for line in D.Svg.wrapped("Data note: " + note, 1080, 15):
                    self.assertIn(line, visible)


if __name__ == "__main__":
    unittest.main(verbosity=2)
