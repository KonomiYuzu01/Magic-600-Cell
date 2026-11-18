"""Headless checks of the renderer core's product copy (renderer/core/, S-A2 with ABI 5).

The core builds only on Windows with MSVC (renderer/core/check_core.py). These checks need only the
Python standard library and run anywhere. They check that:
- every source digest in renderer/core/PROVENANCE.md still matches the selection evidence under
  work/experiments/, and every "verbatim" product file equals its source;
- the provenance table lists exactly the files under renderer/core/;
- the CMake build names no source outside renderer/core/;
- the ABI header pins version 5, keeps every ABI 2 declaration, layout and constant and the ABI 3
  additions, pins the ABI 4 product declarations, struct sizes and constants, and pins ABI 5's
  route B declarations and constants (route B is accepted for now by the owner on 18 Nov 2026,
  pending three owner-machine checks).
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
        self.assertIn("shaders/product.hlsl", sources)
        self.assertIn("src/product.cpp", sources)
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
        self.assertEqual(len(built), 8)
        self.assertTrue({"id_ps.dxil", "lattice_cs.dxil", "product_ps.dxil", "product_vs.dxil"} <= set(built))
        self.assertEqual(len(values["CPU_CHECKS"]), 33)
        self.assertTrue({"wj arguments", "wj scene state", "wj reader refusals", "wj protocol controls",
                         "wj native writer"} <= values["CPU_CHECKS"])
        self.assertTrue({"product arguments", "product adopt", "product tickets", "product frames", "product reports",
                         "product picks", "product device lost", "product describe"} <= values["CPU_CHECKS"])
        self.assertTrue({"route b arguments", "present matching", "recovery bookkeeping"} <= values["CPU_CHECKS"])

    def test_default_build_directory_is_ignored_work(self):
        batch = (CORE / "build.cmd").read_bytes().decode("ascii")
        self.assertIn(r'set "BUILD=%HERE%..\..\work\build\renderer-core"', batch)


ABI3 = {
    "SA2_API int32_t sa2_scene_load_wj(sa2_context* ctx, const sa2_wj_config* config);",
    "SA2_API int32_t sa2_scene_lattice_check(sa2_context* ctx, const char* directory_utf8);",
}
ABI4 = {
    "SA2_API int32_t sa2_describe(sa2_context* ctx, char* buffer, uint32_t buffer_size, uint32_t* out_size);",
    "SA2_API int32_t sa2_engine_adopt(sa2_context* ctx, const sa2_engine_image* image, uint64_t* out_revision);",
    "SA2_API int32_t sa2_prepare(sa2_context* ctx, const sa2_preparation* preparation, uint64_t* out_ticket);",
    "SA2_API int32_t sa2_poll(sa2_context* ctx, uint64_t ticket, sa2_poll_row* out);",
    "SA2_API int32_t sa2_activate(sa2_context* ctx, uint64_t resource_set, const uint8_t stamp_digest[32]);",
    "SA2_API int32_t sa2_cancel(sa2_context* ctx, uint64_t ticket, int32_t reason, uint64_t superseded_by);",
    "SA2_API int32_t sa2_retire(sa2_context* ctx, uint64_t resource_set);",
    "SA2_API int32_t sa2_frame_submit(sa2_context* ctx, const sa2_frame_input* input, uint64_t* out_frame_id);",
    "SA2_API int32_t sa2_frame_reports(sa2_context* ctx, sa2_frame_report* rows, uint32_t capacity, uint32_t* out_count);",
    "SA2_API int32_t sa2_presented(sa2_context* ctx, sa2_presented_row* rows, uint32_t capacity, uint32_t* out_count);",
    "SA2_API int32_t sa2_pick_request(sa2_context* ctx, uint64_t frame_id, uint32_t x, uint32_t y, uint64_t* out_ticket);",
    "SA2_API int32_t sa2_pick_poll(sa2_context* ctx, uint64_t ticket, sa2_pick_row* out);",
    "SA2_API int32_t sa2_recover(sa2_context* ctx, char* buffer, uint32_t buffer_size, uint32_t* out_size);",
    "SA2_API int32_t sa2_resize(sa2_context* ctx, uint32_t width, uint32_t height);",
    "SA2_API int32_t sa2_counters(sa2_context* ctx, sa2_counter_values* out);",
    "SA2_API int32_t sa2_device_status(sa2_context* ctx, int32_t* out_removed_reason);",
}
ABI4_STRUCTS = {
    "sa2_engine_image": 80, "sa2_preparation": 112, "sa2_poll_row": 32, "sa2_frame_input": 264,
    "sa2_frame_report": 104, "sa2_presented_row": 48, "sa2_pick_row": 112, "sa2_counter_values": 64,
}
ABI4_DEFINES = {
    "SA2_E_UNAVAILABLE": "10", "SA2_SLOTS": "259800u", "SA2_PIECES": "177120u", "SA2_MAX_POSES": "177120u",
    "SA2_POSE_GROUP_ORDER": "7200", "SA2_TICKET_PENDING": "0", "SA2_TICKET_READY": "1", "SA2_TICKET_CANCELLED": "2",
    "SA2_TICKET_FAILED": "3", "SA2_CANCEL_BY_USER": "0", "SA2_CANCEL_SUPERSEDED": "1", "SA2_CANCEL_REPLACED_REVISION": "2",
    "SA2_CANCEL_BUDGET": "3", "SA2_PICK_PENDING": "0", "SA2_PICK_READY": "1", "SA2_PICK_STALE": "2",
    "SA2_PICK_STALE_SURFACE": "1", "SA2_PICK_STALE_VIEW": "2", "SA2_PICK_STALE_TARGET": "3", "SA2_PICK_STALE_ENGINE": "4",
    "SA2_PICK_STALE_PUBLICATION": "5", "SA2_FRAME_PREVIEW": "1u",
}
# ABI 5: route B (accepted for now by the owner on 18 Nov 2026, pending three owner-machine checks).
ABI5 = {
    "SA2_API int32_t sa2_create(uint64_t adapter_luid, const sa2_config* config, sa2_context** out);",
    "SA2_API int32_t sa2_surface_attach(sa2_context* ctx, uint64_t hwnd, int32_t x, int32_t y, uint32_t width, uint32_t height);",
    "SA2_API int32_t sa2_surface_move(sa2_context* ctx, int32_t x, int32_t y);",
    "SA2_API int32_t sa2_surface_detach(sa2_context* ctx);",
}
ABI5_DEFINES = {
    "SA2_TARGET_SURFACE": "0xFFFFFFFFu", "SA2_SURFACE_SYNC_INTERVAL": "1u", "SA2_SURFACE_BUFFERS": "3u",
    "SA2_SURFACE_MAX_PX": "16384u",
}


class AbiTests(unittest.TestCase):
    def setUp(self):
        self.header = text(CORE / "include" / "sa2_interop.h")
        self.abi2 = text(SA2_HEADER)

    def test_version_is_5(self):
        self.assertEqual(re.findall(r"^#define SA2_ABI_VERSION (\S+)$", self.header, re.M), ["5u"])
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

    def test_abi3_additions_are_kept(self):
        self.assertTrue(ABI3 <= declarations(self.header))
        self.assertIn("sa2_wj_config", structs(self.header))
        self.assertIn('static_assert(sizeof(sa2_wj_config) == 48, "sa2_wj_config requires the x64 ABI");', self.header)

    def test_abi4_and_abi5_additions(self):
        added = declarations(self.header) - declarations(self.abi2)
        self.assertEqual(added, ABI3 | ABI4 | ABI5)

    def test_abi4_struct_sizes_are_pinned(self):
        layouts = structs(self.header)
        product = text(CORE / "src" / "product.h")
        for name, size in ABI4_STRUCTS.items():
            with self.subTest(struct=name):
                self.assertIn(name, layouts)
                self.assertTrue(layouts[name].startswith(f"typedef struct {name} {{\n    uint32_t struct_size;"),
                                "every ABI 4 struct starts with struct_size")
                self.assertIn(f'static_assert(sizeof({name}) == {size}, "{name} requires the x64 ABI");', self.header)
                self.assertIn(f"static_assert(sizeof({name}) == {size});", product)

    def test_abi4_constants_are_pinned(self):
        values = defines(self.header)
        for name, value in ABI4_DEFINES.items():
            with self.subTest(define=name):
                self.assertEqual(values.get(name), value)

    def test_abi5_constants_are_pinned(self):
        values = defines(self.header)
        for name, value in ABI5_DEFINES.items():
            with self.subTest(define=name):
                self.assertEqual(values.get(name), value)
        self.assertIn("static_assert(SA2_SURFACE_BUFFERS == SA2_RING_SLOTS", text(CORE / "src" / "interop.cpp"))


if __name__ == "__main__":
    result = unittest.main(exit=False, verbosity=2).result
    sys.exit(0 if result.wasSuccessful() else 1)
