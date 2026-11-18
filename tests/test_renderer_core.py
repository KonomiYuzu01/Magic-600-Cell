"""Headless checks of the renderer core's product copy (renderer/core/, S-A2 with ABI 3).

The core builds only on Windows with MSVC (renderer/core/check_core.py). These checks need only the
Python standard library and run anywhere. They check that:
- every source digest in renderer/core/PROVENANCE.md still matches the selection evidence under
  work/experiments/, and every "verbatim" product file equals its source;
- the provenance table lists exactly the files under renderer/core/;
- the CMake build names no source outside renderer/core/;
- the ABI header pins version 3 and keeps every ABI 2 declaration, layout and constant.
"""
from __future__ import annotations

import ast
import hashlib
import re
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CORE = ROOT / "renderer" / "core"
SOURCE_COMMIT = "0f067202ae16ee981db7eef2ece05b54fe226b98"
SA2_HEADER = ROOT / "work" / "experiments" / "renderer-sa2" / "native" / "include" / "sa2_interop.h"
# Files whose source has CRLF line endings that the product copy keeps.
CRLF_FILES = {"renderer/core/build.cmd", "renderer/core/code_layout.json"}
ROW = re.compile(r"^\| `(renderer/core/[^`]+)` \| (?:`(work/experiments/[^`]+)`|(new)) \| (?:`([0-9a-f]{64})`|-) \| (.+) \|$")


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def provenance() -> list[dict]:
    rows = []
    for line in (CORE / "PROVENANCE.md").read_text(encoding="utf-8").splitlines():
        if not line.startswith("| `renderer/core/"):
            continue
        match = ROW.match(line)
        if not match:
            raise AssertionError(f"malformed provenance row: {line}")
        product, source, new, digest, change = match.groups()
        rows.append({"product": product, "source": source, "new": bool(new), "digest": digest, "change": change.strip()})
    return rows


def text(path: Path) -> str:
    return path.read_bytes().decode("utf-8").replace("\r\n", "\n")


def structs(header: str) -> dict[str, str]:
    return {m.group(1): m.group(0) for m in re.finditer(r"typedef struct (\w+) \{.*?\} \1;", header, re.S)}


def declarations(header: str) -> set[str]:
    return {line.strip() for line in header.splitlines() if line.startswith("SA2_API ") and line.rstrip().endswith(";")}


def defines(header: str) -> dict[str, str]:
    return dict(re.findall(r"^#define (SA2_\w+) (\S+)", header, re.M))


class ProvenanceTests(unittest.TestCase):
    def setUp(self):
        self.rows = provenance()

    def test_names_the_source_commit(self):
        self.assertIn(SOURCE_COMMIT, (CORE / "PROVENANCE.md").read_text(encoding="utf-8"))

    def test_rows_are_consistent(self):
        self.assertTrue(self.rows)
        for row in self.rows:
            with self.subTest(product=row["product"], source=row["source"]):
                if row["new"]:
                    self.assertIsNone(row["digest"], "a new file has no source digest")
                    self.assertNotEqual(row["change"], "verbatim")
                else:
                    self.assertIsNotNone(row["digest"], "a copied file needs its source digest")
        pairs = [(row["product"], row["source"]) for row in self.rows]
        self.assertEqual(len(pairs), len(set(pairs)), "duplicate provenance row")

    def test_source_digests_match_the_evidence(self):
        for row in self.rows:
            if row["new"]:
                continue
            with self.subTest(source=row["source"]):
                source = ROOT / row["source"]
                self.assertTrue(source.is_file(), f"missing source {row['source']}")
                self.assertEqual(sha256(source), row["digest"], "the selection evidence changed")

    def test_verbatim_files_equal_their_source(self):
        verbatim = [row for row in self.rows if row["change"].split(" (")[0] == "verbatim"]
        self.assertTrue(verbatim)
        for row in verbatim:
            with self.subTest(product=row["product"]):
                self.assertEqual(sha256(ROOT / row["product"]), row["digest"])

    def test_changed_files_differ_from_their_source(self):
        for row in self.rows:
            if row["new"] or row["change"].startswith("verbatim"):
                continue
            with self.subTest(product=row["product"], source=row["source"]):
                self.assertNotEqual(sha256(ROOT / row["product"]), row["digest"], "an unchanged copy should say verbatim")

    def test_table_lists_exactly_the_product_files(self):
        listed = {row["product"] for row in self.rows}
        present = {path.relative_to(ROOT).as_posix() for path in CORE.rglob("*")
                   if path.is_file() and "__pycache__" not in path.parts}
        self.assertEqual(sorted(listed - present), [], "listed product files that do not exist")
        self.assertEqual(sorted(present - listed), [], "product files missing from PROVENANCE.md")

    def test_line_endings(self):
        for row in self.rows:
            data = (ROOT / row["product"]).read_bytes()
            with self.subTest(product=row["product"]):
                if row["product"] in CRLF_FILES:
                    self.assertNotIn(b"\n", data.replace(b"\r\n", b""), "mixed line endings")
                    self.assertNotIn(b"\r", data.replace(b"\r\n", b""), "mixed line endings")
                else:
                    self.assertNotIn(b"\r", data, "repository text files are LF")


class BuildTests(unittest.TestCase):
    def setUp(self):
        self.cmake = text(CORE / "CMakeLists.txt")

    def test_names_no_experiment_path(self):
        self.assertNotIn("work/experiments", self.cmake.replace("\\", "/").lower())

    def test_every_compiled_source_is_a_product_file(self):
        sources = []
        for match in re.finditer(r"add_(?:library|executable)\((\w+)([^)]*)\)", self.cmake):
            sources += [token for token in match.group(2).split() if token not in ("STATIC", "SHARED")]
        self.assertIn("src/scene.cpp", sources)
        for match in re.finditer(r"^compile_shader\(\S+ (\S+) ", self.cmake, re.M):
            sources.append("shaders/" + match.group(1))
        self.assertIn("shaders/lattice.hlsl", sources)
        for source in sources:
            with self.subTest(source=source):
                self.assertNotIn("..", source)
                self.assertTrue((CORE / source).is_file(), f"{source} is not under renderer/core")
        self.assertIn('set(CORE_SHADERS "${CMAKE_CURRENT_SOURCE_DIR}/shaders")', self.cmake)

    def test_check_core_expects_every_built_shader(self):
        built = sorted(re.findall(r"^compile_shader\((\S+\.dxil) ", self.cmake, re.M))
        tree = ast.parse((CORE / "check_core.py").read_text(encoding="utf-8"))
        values = {node.targets[0].id: ast.literal_eval(node.value) for node in tree.body
                  if isinstance(node, ast.Assign) and isinstance(node.targets[0], ast.Name)
                  and node.targets[0].id in ("SHADERS", "CPU_CHECKS")}
        self.assertEqual(values["SHADERS"], built)
        self.assertEqual(len(built), 5)
        self.assertEqual(len(values["CPU_CHECKS"]), 22)
        self.assertTrue({"wj arguments", "wj scene state", "wj reader refusals", "wj protocol controls",
                         "wj native writer"} <= values["CPU_CHECKS"])

    def test_default_build_directory_is_ignored_work(self):
        batch = (CORE / "build.cmd").read_bytes().decode("ascii")
        self.assertIn(r'set "BUILD=%HERE%..\..\work\build\renderer-core"', batch)


class AbiTests(unittest.TestCase):
    def setUp(self):
        self.header = text(CORE / "include" / "sa2_interop.h")
        self.abi2 = text(SA2_HEADER)

    def test_version_is_3(self):
        self.assertEqual(re.findall(r"^#define SA2_ABI_VERSION (\S+)$", self.header, re.M), ["3u"])
        self.assertEqual(re.findall(r"^#define SA2_ABI_VERSION (\S+)$", self.abi2, re.M), ["2u"])

    def test_abi2_scene_config_layout_is_pinned(self):
        self.assertIn('static_assert(sizeof(sa2_scene_config) == 32, "sa2_scene_config requires the x64 ABI");', self.header)
        self.assertIn("static_assert(sizeof(sa2_scene_config) == 32);", text(CORE / "src" / "scene_record.h"))

    def test_abi2_declarations_layouts_and_constants_are_kept(self):
        self.assertEqual(sorted(declarations(self.abi2) - declarations(self.header)), [])
        old, new = structs(self.abi2), structs(self.header)
        self.assertTrue(old)
        for name, body in old.items():
            with self.subTest(struct=name):
                self.assertEqual(new.get(name), body)
        old_defines, new_defines = defines(self.abi2), defines(self.header)
        for name, value in old_defines.items():
            if name != "SA2_ABI_VERSION":
                with self.subTest(define=name):
                    self.assertEqual(new_defines.get(name), value)

    def test_abi3_additions(self):
        added = declarations(self.header) - declarations(self.abi2)
        self.assertEqual(added, {
            "SA2_API int32_t sa2_scene_load_wj(sa2_context* ctx, const sa2_wj_config* config);",
            "SA2_API int32_t sa2_scene_lattice_check(sa2_context* ctx, const char* directory_utf8);",
        })
        self.assertIn("sa2_wj_config", structs(self.header))
        self.assertIn('static_assert(sizeof(sa2_wj_config) == 48, "sa2_wj_config requires the x64 ABI");', self.header)


if __name__ == "__main__":
    result = unittest.main(exit=False, verbosity=2).result
    sys.exit(0 if result.wasSuccessful() else 1)
