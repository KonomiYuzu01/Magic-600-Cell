"""Headless checks of the C9 acceptance harness (renderer/acceptance/).

The harness runs only on Windows with Godot, PresentMon and a Direct3D 12 device. These checks need only
the Python standard library, start no process, and run anywhere. They check that:
- every source digest in renderer/acceptance/PROVENANCE.md still matches the selection evidence under
  work/experiments/, every "verbatim" file equals its source, and the table lists exactly the harness files;
- the Godot project's interop layer (project/Native.cs) matches the core's ABI 5 header
  renderer/core/include/sa2_interop.h: the x64 layout of every struct it declares, its constants, and the
  signature of every entry point it imports; it declares every ABI 1 to 3 struct and imports every ABI 1 to 3
  call (the harness uses no ABI 4 or ABI 5 call);
- the injection, menu and scene lists agree across the header, the project and the run script;
- the run finalizer accepts small synthetic W-J and W3 runs and checks, refuses or fails broken ones, and
  writes records that the unchanged renderer gate (tools/perf/renderer_gate.py) judges as the evidence was.
"""
from __future__ import annotations

import ast
import contextlib
import hashlib
import io
import json
import re
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parents[1]
ACCEPTANCE = ROOT / "renderer" / "acceptance"
PROJECT = ACCEPTANCE / "project"
HEADER = ROOT / "renderer" / "core" / "include" / "sa2_interop.h"
SOURCE_COMMIT = "0f067202ae16ee981db7eef2ece05b54fe226b98"
# The run script keeps its source's CRLF line endings; every other harness file is LF.
CRLF_FILES = {"renderer/acceptance/run_scene.ps1"}
ROW = re.compile(r"^\| `(renderer/acceptance/[^`]+)` \| (?:`(work/experiments/[^`]+)`|(new)) \| (?:`([0-9a-f]{64})`|-) \| (.+) \|$")

sys.dont_write_bytecode = True  # importing the harness writes nothing beside it
sys.path.insert(0, str(ACCEPTANCE))
sys.path.insert(1, str(ROOT / "tools" / "perf"))
import prepare_data  # noqa: E402
import renderer_gate as gate  # noqa: E402

try:
    import finalize_run as fr  # noqa: E402
except (ImportError, ValueError, OSError) as error:  # file_guard needs ctypes.wintypes
    fr, FINALIZER_ERROR = None, error
else:
    FINALIZER_ERROR = None


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def text(path: Path) -> str:
    return path.read_bytes().decode("utf-8").replace("\r\n", "\n")


def provenance() -> list[dict]:
    rows = []
    for line in (ACCEPTANCE / "PROVENANCE.md").read_text(encoding="utf-8").splitlines():
        if not line.startswith("| `renderer/acceptance/"):
            continue
        match = ROW.match(line)
        if not match:
            raise AssertionError(f"malformed provenance row: {line}")
        product, source, new, digest, change = match.groups()
        rows.append({"product": product, "source": source, "new": bool(new), "digest": digest, "change": change.strip()})
    return rows


def strings(fragment: str, quote: str = '"') -> list[str]:
    return re.findall(f"{quote}([^{quote}]+){quote}", fragment)


class ProvenanceTests(unittest.TestCase):
    def setUp(self):
        self.rows = provenance()

    def test_names_the_source_commit(self):
        self.assertIn(SOURCE_COMMIT, (ACCEPTANCE / "PROVENANCE.md").read_text(encoding="utf-8"))

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
        verbatim = [row for row in self.rows if row["change"] == "verbatim"]
        self.assertGreaterEqual(len(verbatim), 10)
        for row in verbatim:
            with self.subTest(product=row["product"]):
                self.assertEqual(sha256(ROOT / row["product"]), row["digest"])

    def test_changed_files_differ_from_their_source(self):
        for row in self.rows:
            if row["new"] or row["change"] == "verbatim":
                continue
            with self.subTest(product=row["product"], source=row["source"]):
                self.assertNotEqual(sha256(ROOT / row["product"]), row["digest"], "an unchanged copy should say verbatim")

    def test_table_lists_exactly_the_harness_files(self):
        listed = {row["product"] for row in self.rows}
        present = {path.relative_to(ROOT).as_posix() for path in ACCEPTANCE.rglob("*")
                   if path.is_file() and "__pycache__" not in path.parts}
        self.assertEqual(sorted(listed - present), [], "listed harness files that do not exist")
        self.assertEqual(sorted(present - listed), [], "harness files missing from PROVENANCE.md")

    def test_line_endings(self):
        for product in sorted({row["product"] for row in self.rows}):
            data = (ROOT / product).read_bytes()
            with self.subTest(product=product):
                if product in CRLF_FILES:
                    bare = data.replace(b"\r\n", b"")
                    self.assertNotIn(b"\n", bare, "mixed line endings")
                    self.assertNotIn(b"\r", bare, "mixed line endings")
                    data.decode("ascii")  # Windows PowerShell 5.1 reads a file without a BOM as ANSI.
                else:
                    self.assertNotIn(b"\r", data, "repository text files are LF")


class DataTests(unittest.TestCase):
    def test_data_stays_out_of_the_evidence(self):
        for refused in ("work/experiments/renderer-wj/data", "work/experiments", "research/jumbling/fixtures/x",
                        "work/loop-memory", "work/build", "work", "work/loop-memory/../experiments/x",
                        str(ROOT.parent / "c9-data")):
            with self.subTest(path=refused), self.assertRaises(prepare_data.Refused):
                prepare_data.output_root(refused)
        for allowed in ("work/loop-memory/c9-acceptance/wj-data", "work/build/c9-acceptance/wj-data"):
            with self.subTest(path=allowed):
                self.assertEqual(prepare_data.output_root(allowed), (ROOT / allowed).resolve())

    def test_runner_default_data_is_the_prepared_default(self):
        runner = text(ACCEPTANCE / "run_scene.ps1")
        self.assertIn(f"Join-Path $repository '{prepare_data.DEFAULT_OUT}'", runner)
        self.assertNotIn("work/experiments", runner.replace("\\", "/").split("\n", 1)[1],
                         "the runner names no evidence path after its provenance line")


# --- The interop layer against the header ---------------------------------------------------------

C_SCALARS = {"uint32_t": (4, 4), "int32_t": (4, 4), "uint64_t": (8, 8), "double": (8, 8)}
CS_FIELD_TYPES = {"uint": "uint32_t", "int": "int32_t", "ulong": "uint64_t", "double": "double", "byte*": "pointer"}
CS_SCALARS = {"uint32_t": "uint", "int32_t": "int", "uint64_t": "ulong", "double": "double", "uint8_t": "byte"}
ABI4_SECTION = "\n/* ABI 4 product API"


def abi1_to_3(header: str) -> str:
    """The header before its ABI 4 product section: the ABI 1 to 3 declarations and structs."""
    if header.count(ABI4_SECTION) != 1:
        raise AssertionError("the header must open its ABI 4 section exactly once")
    return header[:header.index(ABI4_SECTION)]


def camel(name: str) -> str:
    return "".join(part.capitalize() for part in name.split("_"))


def header_structs(header: str) -> dict[str, list[tuple[str, str, int]]]:
    """Each struct's fields as (name, C type or "pointer", element count)."""
    result = {}
    for match in re.finditer(r"typedef struct (\w+) \{(.*?)\} \1;", header, re.S):
        fields = []
        for declaration in re.sub(r"/\*.*?\*/", "", match.group(2), flags=re.S).split(";"):
            declaration = " ".join(declaration.split())
            if not declaration:
                continue
            parsed = re.fullmatch(r"(?:const )?(\w+)(\*?) (\w+)(?:\[(\d+)\])?", declaration)
            if not parsed:
                raise AssertionError(f"unparsed field {declaration!r} in {match.group(1)}")
            base, pointer, name, count = parsed.groups()
            fields.append((name, "pointer" if pointer else base, int(count or 1)))
        result[match.group(1)] = fields
    return result


def x64_layout(fields) -> tuple[list[int], int]:
    """Field offsets and the struct size under the x64 C ABI."""
    offset = largest = 0
    offsets = []
    for _, kind, count in fields:
        size, align = (8, 8) if kind == "pointer" else C_SCALARS[kind]
        offset = -(-offset // align) * align
        offsets.append(offset)
        offset += size * count
        largest = max(largest, align)
    return offsets, -(-offset // largest) * largest


def cs_structs(source: str) -> dict[str, list[tuple[str, str, int]]]:
    result = {}
    for match in re.finditer(r"\[StructLayout\(LayoutKind\.Sequential\)\]\npublic (?:unsafe )?struct (\w+)\n\{\n(.*?)\n\}",
                             source, re.S):
        fields = []
        for declaration in re.finditer(r"public (fixed )?(\w+\*?) ([^;]+);", match.group(2)):
            for name in declaration.group(3).split(","):
                parsed = re.fullmatch(r"(\w+)(?:\[(\d+)\])?", name.strip())
                fields.append((parsed.group(1), CS_FIELD_TYPES[declaration.group(2)], int(parsed.group(2) or 1)))
        result[match.group(1)] = fields
    return result


def header_functions(header: str) -> dict[str, list[str]]:
    """Each SA2_API function as the C# function-pointer type arguments it needs: parameters, then return."""
    result = {}
    for line in header.splitlines():
        if not line.startswith("SA2_API "):
            continue
        match = re.fullmatch(r"SA2_API (\w+) (\w+)\((.*)\);", line.strip())
        if not match:
            raise AssertionError(f"unparsed declaration {line!r}")
        returns, name, parameters = match.groups()
        types = []
        for parameter in ([] if parameters == "void" else parameters.split(",")):
            parsed = re.fullmatch(r"(?:const )?(\w+)(\**) \w+(\[\d+\])?", " ".join(parameter.split()))
            if not parsed:
                raise AssertionError(f"unparsed parameter {parameter!r} of {name}")
            base, stars, array = parsed.groups()
            stars += "*" if array else ""  # an array parameter is a pointer
            if base == "sa2_context":
                types.append("nint" + stars[1:])  # the opaque context handle is a pointer
            elif base == "char":
                types.append("byte" + stars)
            elif base.startswith("sa2_"):
                types.append(camel(base) + stars)
            else:
                types.append(CS_SCALARS[base] + stars)
        result[name] = types + [CS_SCALARS[returns]]
    return result


def split_types(arguments: str) -> list[str]:
    return [part.strip() for part in arguments.split(",")]


class InteropTests(unittest.TestCase):
    def setUp(self):
        self.header = text(HEADER)
        self.native = text(PROJECT / "Native.cs")

    def test_structs_match_the_header_on_x64(self):
        header, native = header_structs(self.header), cs_structs(self.native)
        earlier = header_structs(abi1_to_3(self.header))
        self.assertEqual(len(earlier), 5, "the ABI 1 to 3 structs")
        self.assertEqual(sorted({camel(name) for name in earlier} - native.keys()), [],
                         "Native.cs declares every ABI 1 to 3 struct")
        by_managed = {camel(name): name for name in header}
        self.assertEqual(len(by_managed), len(header))
        self.assertEqual(sorted(native.keys() - by_managed.keys()), [], "every struct of Native.cs is in the header")
        self.assertEqual(self.native.count("struct Sa2"), len(native), "every struct is sequential")
        pattern = r"static_assert\(sizeof\((\w+)\) == (\d+),"
        asserted = dict(re.findall(pattern, self.header))
        self.assertEqual(sorted(dict(re.findall(pattern, abi1_to_3(self.header)))), ["sa2_scene_config", "sa2_wj_config"])
        constants = dict(re.findall(r"public const int (\w+Size) = (\d+);", self.native))
        for managed_name, managed in sorted(native.items()):  # only the structs Native.cs declares
            name = by_managed[managed_name]
            fields = header[name]
            with self.subTest(struct=name):
                self.assertEqual([(n.casefold(), kind, count) for n, kind, count in managed],
                                 [(n.casefold(), kind, count) for n, kind, count in fields])
                _, size = x64_layout(fields)
                constant = camel(name)[3:] + "Size"
                self.assertEqual(int(constants[constant]), size)
                if name in asserted:
                    self.assertEqual(int(asserted[name]), size)
                self.assertIn(f"sizeof({camel(name)}) != {constant}", self.native, "checked at load")
        self.assertEqual(x64_layout(header["sa2_wj_config"]), ([0, 4, 8, 16, 24, 32, 40, 44], 48))

    def test_constants_match_the_header(self):
        defines = dict(re.findall(r"^#define (SA2_\w+) (\S+)", self.header, re.M))
        constants = re.findall(r"public const (uint|int) (SA2_\w+) = (\d+);", self.native)
        self.assertEqual(len({name for _, name, _ in constants}), len(constants))
        for kind, name, value in constants:
            with self.subTest(constant=name):
                self.assertIn(name, defines)
                self.assertEqual(int(defines[name].rstrip("u")), int(value))
                self.assertEqual(kind == "uint", defines[name].endswith("u"))
        wanted = {name for name in defines
                  if re.fullmatch(r"SA2_(ABI_VERSION|OK|E_\w+|QUEUE_\w+|BARRIERS_\w+|STATE_\w+|SCENE_NO_VRAM)", name)}
        self.assertEqual(sorted(wanted - {name for _, name, _ in constants}), [])

    def test_abi_version_5_is_required(self):
        self.assertEqual(re.findall(r"^#define SA2_ABI_VERSION (\S+)$", self.header, re.M), ["5u"])
        self.assertIn("public const uint SA2_ABI_VERSION = 5;", self.native)
        self.assertIn("if (sa2_abi_version() != SA2_ABI_VERSION)", self.native)
        loads = re.findall(r'Export\("(\w+)"\)', self.native)
        self.assertEqual(loads[0], "sa2_abi_version", "the version is checked before any other export")

    def test_entry_points_match_the_header(self):
        functions = header_functions(self.header)
        earlier = header_functions(abi1_to_3(self.header))
        self.assertEqual(len(earlier), 29, "the ABI 1 to 3 calls")
        self.assertTrue({"sa2_scene_load_wj", "sa2_scene_lattice_check"} <= earlier.keys())
        declared = re.findall(r"public readonly delegate\* unmanaged\[Cdecl\]<([^>]*)> (\w+);", self.native)
        fields = {name: split_types(arguments) for arguments, name in declared}
        self.assertEqual(len(fields), len(declared), "one delegate field per entry point")
        self.assertEqual(sorted(earlier.keys() - fields.keys()), [], "Native.cs imports every ABI 1 to 3 call")
        self.assertEqual(sorted(fields.keys() - functions.keys()), [], "every import is declared in the header")
        for name, types in fields.items():
            with self.subTest(function=name):
                self.assertEqual(types, functions[name])
        loads = re.findall(r'(\w+) = \(delegate\* unmanaged\[Cdecl\]<([^>]*)>\)Export\("(\w+)"\);', self.native)
        self.assertEqual(sorted(name for name, _, _ in loads), sorted(fields), "each export loaded once")
        for name, arguments, export in loads:
            with self.subTest(load=name):
                self.assertEqual(export, name)
                self.assertEqual(split_types(arguments), functions[name])


class ListTests(unittest.TestCase):
    """The injections, menus and scenes agree across the header, the project, the runner and the finalizer."""

    def setUp(self):
        self.header = text(HEADER)
        self.arguments = text(PROJECT / "Level2Arguments.cs")
        self.runner = text(ACCEPTANCE / "run_scene.ps1")

    def header_injections(self, struct):
        body = re.search(rf"typedef struct {struct} \{{(.*?)\}} {struct};", self.header, re.S).group(1)
        return re.findall(r"[a-z]+(?:-[a-z]+)+", re.search(r"const char\* inject; /\*(.*?)\*/", body, re.S).group(1))

    def project_list(self, name):
        return strings(re.search(rf"{name} = \{{([^}}]*)\}};", self.arguments).group(1))

    def test_injections(self):
        label, pose = self.header_injections("sa2_scene_config"), self.header_injections("sa2_wj_config")
        self.assertEqual(label, ["corrupt-label", "swap-same-colour", "delay-adoption", "stale-binding"])
        self.assertEqual(pose, ["corrupt-index", "swap-same-colour", "stale-pose", "delay-adoption", "stale-binding"])
        self.assertEqual(self.project_list("LabelInjections"), label)
        self.assertEqual(self.project_list("PoseInjections"), pose)
        choice = re.search(r'case "--l2-inject": a\.inject = Choice\(value, (.*?)\); break;', self.arguments, re.S)
        self.assertEqual(set(strings(choice.group(1))), set(label) | set(pose))
        validate = re.search(r"\[ValidateSet\(([^)]*)\)\]\[string\]\$Inject", self.runner)
        self.assertEqual(set(strings(validate.group(1), "'")), set(label) | set(pose))
        faults = re.search(r"\$faults = if \(\$wj\) \{ @\(([^)]*)\) \} else \{ @\(([^)]*)\) \}", self.runner)
        self.assertEqual(strings(faults.group(1), "'"), pose)
        self.assertEqual(strings(faults.group(2), "'"), label)

    def test_menus_and_scenes(self):
        menus = strings(re.search(r'case "--l2-menu": a\.menu = Choice\(value, ([^)]*)\);', self.arguments).group(1))
        self.assertEqual(menus, list(prepare_data.MENUS))
        scene = re.search(r"\[ValidateSet\(([^)]*)\)\]\[string\]\$Scene", self.runner)
        self.assertEqual(strings(scene.group(1), "'"), ["w3", *menus])
        self.assertIn("wj", strings(re.search(r'case "--l2-scene": a\.scene = Choice\(value, ([^)]*)\);',
                                              self.arguments).group(1)))
        if fr is not None:
            self.assertEqual(list(fr.MENUS), menus)

    def test_guard_protects_every_core_shader(self):
        tree = ast.parse(text(ROOT / "renderer" / "core" / "check_core.py"))
        built = next(ast.literal_eval(node.value) for node in tree.body if isinstance(node, ast.Assign)
                     and isinstance(node.targets[0], ast.Name) and node.targets[0].id == "SHADERS")
        guard = ast.parse(text(ACCEPTANCE / "file_guard.py"))
        protected = next(ast.literal_eval(node.value) for node in guard.body if isinstance(node, ast.Assign)
                         and isinstance(node.targets[0], ast.Name) and node.targets[0].id == "SHADERS")
        self.assertEqual(protected, set(built))


# --- The finalizer on synthetic runs ----------------------------------------------------------------

FREQ, T0, TRACE_TICKS, STEP, TURN_MS, PID = 1000, 1_000_000, 192_000, 20, 190, 4242
STOP = T0 + TRACE_TICKS
FRAMES = TRACE_TICKS // STEP
LAST_TURN = (FRAMES - 1) * STEP // TURN_MS
SIZE = {"width": 2560, "height": 1600}
HASH = "a" * 64
OVERLAYS = "none running; unattended: synthetic test input"
BUILD = {"build_identity": "f" * 64, "parts": [], "dll_identity": {}}
BINDING = {"format": "magic600-l2-guard-v1", "files": 7, "directories": 3, "guard_sha256": "0" * 64,
           "rule": "guard-before-launch"}


def write_json(path: Path, value) -> None:
    path.write_bytes((json.dumps(value, indent=1) + "\n").encode("utf-8"))


def trace_entries() -> list[dict]:
    """One frame every 20 ms on a 1 kHz QPC clock; turns of 190 ms adopted at once (revision = turn)."""
    entries = []
    for frame in range(FRAMES):
        elapsed = frame * STEP
        turn = elapsed // TURN_MS
        entries.append({"frame": frame, "qpc": T0 + elapsed, "revision": turn, "turn": turn,
                        "phase": (elapsed % TURN_MS) / TURN_MS, "camera": frame})
    return entries


def harness(scene, mode="run", menu=None, run_id="c9-test"):
    declared = {"frame_generation": False, "upscaling": False}
    return {
        "format": "magic600-l2-harness-v1", "candidate": "sa2", "mode": mode, "run_id": run_id, "scene": scene,
        "menu": menu, "data": "C:\\c9\\wj-data" if scene == "wj" else None, "process_id": PID, "exit_code": 0,
        "reason": None,
        "options": {"trace_ms": TRACE_TICKS, "preroll_ms": 4000, "turn_ms": TURN_MS, "inject": None,
                    "declared": dict(declared), "gpu_validation": False, "conditions": "enforce",
                    "no_vram": False, "debug_half_target": False},
        "configuration": {"rendering_device/driver": "d3d12", "vsync": False, "msaa": 1},
        "files": {}, "dll_identity": {}, "dll_status": {}, "qpc_frequency": FREQ,
        "sizes": {**{key: dict(SIZE) for key in ("display", "window", "backbuffer", "displayed", "target")},
                  "samples": 1920, "samples_changed": 0},
        "scaling": {"texture_stretch": "none", "content_scale_mode": "disabled", "content_scale_factor": 1},
        "dpi_awareness": "per-monitor-v2",
        "environment": {"adapter": fr.DEFAULT_ADAPTER, "declared": dict(declared), "display": dict(SIZE),
                        "backbuffer": dict(SIZE), "power_source": "mains",
                        "power_samples": {"samples": 1920, "mains": 1920, "battery": 0}},
        "window": {"topmost": True, "display_required": True, "foreground_at_trace_start": True,
                   "sample_period_ms": 100, "samples": 1920, "samples_not_visible": 0, "samples_covered": 0,
                   "samples_not_foreground": 0, "visible_throughout": True, "foreground_throughout": True,
                   "presents": FRAMES},
        "debug": {"enabled": False, "debug_layer": 0, "counts": {}, "messages": []},
    }


def pose_record(required, failing=None):
    """W-J's pose_check.json: one check per first-use copy; `failing` gets one mismatching array."""
    checks = []
    for revision in required:
        bad = revision == failing
        arrays = [{"sha256": "d" * 64 if bad and index == 0 else HASH, "expected_sha256": HASH,
                   "sha256_equal": not (bad and index == 0), "element_equal": not (bad and index == 0),
                   "element_mismatches": 7 if bad and index == 0 else 0} for index in range(3)]
        checks.append({"revision": revision, "frame": -(-revision * TURN_MS // STEP), "resource": revision % 3 + 1,
                       "pose_count": 120, "arrays": arrays, "status": "fail" if bad else "pass"})
    passed = failing is None
    return {"missing": 0, "late_adoptions": 0, "binding_mismatches": 0, "mismatches": 0 if passed else 7,
            "hash_failures": 0 if passed else 1, "required": list(required),
            "revisions": [revision for revision in required if revision != failing], "checks": checks,
            "status": "pass" if passed else "fail"}


def presentmon_csv() -> str:
    """PresentMon 2.6 rows 5 ticks after each trace entry, plus one row of another process."""
    lines = ["Application,ProcessID,SwapChainAddress,PresentRuntime,SyncInterval,PresentFlags,AllowsTearing,"
             "PresentMode,Dropped,QPCTime,msBetweenPresents,msBetweenDisplayChange",
             f"other.exe,{PID + 1},0x0000000000000BAD,DXGI,1,0,0,Composed: Flip,0,{T0 + 7},16.667,16.667"]
    lines += [f"godot.exe,{PID},0x000001F2A3B4C5D0,DXGI,0,512,1,Hardware: Independent Flip,0,"
              f"{T0 + frame * STEP + 5},20.000,20.000" for frame in range(FRAMES)]
    return "\n".join(lines) + "\n"


def write_run(directory: Path, scene, menu=None, run_id="c9-test", pose_failure=None, label_mismatches=0):
    directory.mkdir(parents=True)
    entries = trace_entries()
    write_json(directory / "harness.json", harness(scene, menu=menu, run_id=run_id))
    native = {"format": "magic600-sa2-scene-native-v1", "scene": scene, "qpc_frequency": FREQ,
              "markers": {"trace_start_qpc": T0, "trace_stop_qpc": STOP}, "frames": FRAMES,
              "injection_applied": False, "vram_peak_mb": 1530.25, "vram_samples": FRAMES,
              "target": dict(SIZE), "viewport": dict(SIZE), "identity": {}, "queue_mode": "same",
              "barrier_api": "enhanced", "turn_ms": TURN_MS}
    if scene == "w3":
        native["label_check"] = {"status": "fail" if label_mismatches else "pass", "copies": LAST_TURN,
                                 "revisions": LAST_TURN, "mismatches": label_mismatches, "binding_mismatches": 0,
                                 "late_adoptions": 0, "missing": 0,
                                 "oracle_sha256": {"even": "b" * 64, "odd": "c" * 64}}
    else:
        required = sorted({entry["turn"] for entry in entries})
        pose = pose_record(required, pose_failure)
        native.update(fixture=fr.MENUS[menu], overlay="none", pose_upload=fr.POSE_UPLOAD,
                      label_check={"missing": 0, "late_adoptions": 0, "binding_mismatches": 0, "mismatches": 0,
                                   "hash_failures": 0, "required": required, "status": "pass",
                                   "copies": len(required), "revisions": len(required)},
                      pose_check={"status": pose["status"], "revisions": pose["revisions"]})
        write_json(directory / "pose_check.json", pose)
    write_json(directory / "native.json", native)
    (directory / "trace.jsonl").write_bytes("".join(json.dumps(entry) + "\n" for entry in entries).encode("utf-8"))
    (directory / "presentmon.csv").write_bytes(presentmon_csv().encode("utf-8"))
    return directory


def finalize(directory: Path, mode="run", overlays=OVERLAYS, adapter=None, fault_injection=False, unattended=False):
    """The finalizer with the Windows-only build guard replaced by a fixed identity and binding."""
    output = io.StringIO()
    with mock.patch.object(fr, "build_identity", lambda *arguments: (dict(BUILD), dict(BINDING))), \
            mock.patch.object(fr, "final_guard_check", lambda *arguments: None), \
            contextlib.redirect_stdout(output), contextlib.redirect_stderr(output):
        return fr.finalize(directory, "sa2", mode, PID, 0, 2**40, overlays=overlays, fault_injection=fault_injection,
                           adapter=adapter or fr.DEFAULT_ADAPTER, unattended=unattended)


def inject(directory: Path, fault: str) -> Path:
    """Turn a written run into one that ran an injection which the checks did not catch."""
    record = read(directory / "harness.json")
    record["options"]["inject"] = fault
    write_json(directory / "harness.json", record)
    native = read(directory / "native.json")
    native["injection_applied"] = True
    write_json(directory / "native.json", native)
    return directory


def read(path: Path) -> dict:
    return json.loads(path.read_bytes())


def scene_result(summary, scene):
    found = [entry for entry in summary["scenes"] if entry["scene"] == scene]
    return found[0] if found else None


@unittest.skipIf(fr is None, f"finalize_run does not import here: {FINALIZER_ERROR!r}")
class FinalizerTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.base = Path(self.temporary.name)

    def tearDown(self):
        self.temporary.cleanup()

    def test_is_the_harness_copy(self):
        self.assertEqual(Path(fr.__file__).resolve(), ACCEPTANCE / "finalize_run.py")
        self.assertEqual(Path(gate.__file__).resolve(), ROOT / "tools" / "perf" / "renderer_gate.py")
        self.assertIs(fr.gate, gate)

    def test_constants_match_the_core_writer(self):
        record = text(ROOT / "renderer" / "core" / "src" / "scene_record.cpp")
        scene = text(ROOT / "renderer" / "core" / "src" / "scene.cpp")
        for fixture in fr.MENUS.values():
            self.assertIn(f'"{fixture}"', record)
        self.assertIn(f'"{fr.POSE_UPLOAD}"', record)
        self.assertIn('"magic600-sa2-scene-native-v1"', record)
        for name in ("magic600-wj-geometry-check/1", "magic600-wj-lattice-check/1"):
            self.assertIn(f'"{name}"', scene)

    def test_wj_runs_pass_and_three_meet_the_gate(self):
        runs = [write_run(self.base / f"r{index}", "wj", menu="S4", run_id=f"c9-wj-s4-{index}") for index in range(3)]
        for run in runs:
            self.assertEqual(finalize(run), 0)
            record = read(run / "run.json")
            self.assertEqual((record["scene"], record["fixture"], record["candidate"]), ("wj", "wj-s4", "sa2"))
            self.assertEqual(record["pose_check"], {"status": "pass", "revisions": list(range(LAST_TURN + 1))})
            self.assertEqual(record["label_check"]["status"], "pass")
            self.assertEqual(record["l2"]["condition_reasons"], [])
            self.assertEqual(record["environment"]["declared"]["overlays"], OVERLAYS)
            self.assertIs(record["environment"]["tearing"], True)
            self.assertEqual(record["binding"], BINDING)
            self.assertFalse((run / "refusal.json").exists())
        one = scene_result(gate.summarize_private(runs[:1]), "wj")
        self.assertEqual((one["fixture"], one["verdict"], len(one["runs"])), ("wj-s4", "insufficient-runs", 1))
        summary = gate.summarize(runs)
        self.assertEqual(summary["invalid_runs"], [])
        result = scene_result(summary, "wj")
        self.assertEqual((result["verdict"], len(result["runs"])), ("met", 3))
        self.assertAlmostEqual(result["pooled"]["fps"], 50.0)
        self.assertEqual(result["pooled"]["p99_ms"], 20.0)
        self.assertEqual(result["vram_peak_mb"], 1530.25)
        self.assertFalse(any(run["vram_risk"] for run in result["runs"]))

    def test_w3_runs_pass_and_three_meet_the_gate(self):
        runs = [write_run(self.base / f"r{index}", "w3", run_id=f"c9-w3-{index}") for index in range(3)]
        for run in runs:
            self.assertEqual(finalize(run), 0)
            record = read(run / "run.json")
            self.assertEqual(record["scene"], "w3")
            self.assertNotIn("fixture", record)
            self.assertNotIn("pose_check", record)
            self.assertEqual(record["label_check"]["copies"], LAST_TURN)
        self.assertEqual(scene_result(gate.summarize_private(runs[:1]), "w3")["verdict"], "insufficient-runs")
        result = scene_result(gate.summarize_private(runs), "w3")
        self.assertEqual((result["verdict"], len(result["runs"])), ("met", 3))

    def test_failed_pose_check_is_recorded_and_invalid(self):
        run = write_run(self.base / "pose", "wj", menu="I_a", run_id="c9-wj-i-a", pose_failure=20)
        self.assertEqual(finalize(run), 2)
        record = read(run / "run.json")
        self.assertEqual(record["fixture"], "wj-i-a")
        self.assertEqual(record["pose_check"]["status"], "fail")
        self.assertNotIn(20, record["pose_check"]["revisions"])
        summary = gate.summarize_private([run])
        self.assertEqual(len(summary["invalid_runs"]), 1)
        self.assertIn("pose-check-failed", summary["invalid_runs"][0]["reasons"])
        self.assertEqual(scene_result(summary, "wj")["verdict"], "no-data")

    def test_failed_label_check_is_recorded_and_invalid(self):
        run = write_run(self.base / "label", "w3", run_id="c9-w3-label", label_mismatches=3)
        self.assertEqual(finalize(run), 2)
        self.assertEqual(read(run / "run.json")["label_check"]["status"], "fail")
        self.assertIn("label-check-failed", gate.summarize_private([run])["invalid_runs"][0]["reasons"])

    def test_fault_injection_run_is_never_gate_evidence(self):
        run = inject(write_run(self.base / "fault", "w3", run_id="c9-w3-fault"), "swap-same-colour")
        self.assertIn(finalize(run, overlays=None, fault_injection=True), (0, 2))
        self.assertFalse((run / "run.json").exists())
        record = read(run / "fault-run.json")
        self.assertEqual((record["format"], record["injected_fault"]), ("magic600-l2-fault-run-v1", "swap-same-colour"))
        self.assertNotIn("watched", record["environment"]["declared"])
        result = scene_result(gate.summarize_private([run]), "w3")
        self.assertFalse(result and result["runs"], "the gate must not count a fault-injection run")
        with self.assertRaises(Exception):
            gate.validate_run(record)

    def test_operator_declaration_must_match_the_injection(self):
        cases = {
            "declared-injected": (inject(write_run(self.base / "a", "w3", run_id="c9-a"), "stale-binding"),
                                  {"overlays": OVERLAYS}),
            "unattended-injected": (inject(write_run(self.base / "b", "w3", run_id="c9-b"), "stale-binding"),
                                    {"overlays": None, "fault_injection": True, "unattended": True}),
            "fault-flag-clean": (write_run(self.base / "c", "w3", run_id="c9-c"),
                                 {"overlays": None, "fault_injection": True}),
        }
        for name, (run, options) in cases.items():
            with self.subTest(case=name):
                self.assertEqual(finalize(run, **options), 5)
                self.assertEqual(read(run / "refusal.json")["reasons"], ["operator"])
                self.assertFalse((run / "run.json").exists() or (run / "fault-run.json").exists())

    def test_watched_is_recorded(self):
        watched = write_run(self.base / "watched", "w3", run_id="c9-watched")
        unwatched = write_run(self.base / "unwatched", "w3", run_id="c9-unwatched")
        self.assertEqual(finalize(watched), 0)
        self.assertEqual(finalize(unwatched, unattended=True), 0)
        self.assertIs(read(watched / "run.json")["environment"]["declared"]["watched"], True)
        self.assertIs(read(unwatched / "run.json")["environment"]["declared"]["watched"], False)

    def test_refusals(self):
        cases = {"adapter": {"adapter": "Some Other GPU"}, "operator": {"overlays": None}}
        for reason, options in cases.items():
            with self.subTest(reason=reason):
                run = write_run(self.base / reason, "wj", menu="I_b", run_id=f"c9-{reason}")
                self.assertEqual(finalize(run, **options), 5)
                self.assertEqual(read(run / "refusal.json")["reasons"], [reason])
                self.assertFalse((run / "run.json").exists())
        broken = write_run(self.base / "broken", "wj", menu="S4", run_id="c9-broken")
        pose = read(broken / "pose_check.json")
        pose["revisions"] = pose["revisions"][:-1]  # disagrees with native.json and with its own checks
        write_json(broken / "pose_check.json", pose)
        self.assertEqual(finalize(broken), 5)
        self.assertIn("native", read(broken / "refusal.json")["reasons"])
        before = (broken / "refusal.json").read_bytes()
        self.assertEqual(finalize(broken), 5, "an existing output is never replaced")
        self.assertEqual((broken / "refusal.json").read_bytes(), before)

    def check_record(self, name, scene, menu, mode, record, expected_code=0):
        directory = self.base / name
        directory.mkdir()
        write_json(directory / "harness.json", harness(scene, mode=mode, menu=menu, run_id=f"c9-{name}"))
        write_json(directory / ("lattice_check.json" if mode == "lattice" else "geometry.json"), record)
        self.assertEqual(finalize(directory, mode=mode, overlays=None), expected_code)
        return read(directory / f"{mode}-record.json")

    def test_wj_geometry_check_against_the_reference_index(self):
        index = json.loads((ROOT / "work" / "experiments" / "renderer-wj" / "ref_S4_index.json").read_bytes())
        results = [{"file": output["file"], "samples": index["samples"], "max_abs_error": 1.5e-6, "failures": 0,
                    "status": "pass"} for output in index["outputs"]]
        record = {"format": "magic600-wj-geometry-check/1", "status": "pass", "build_identity": "e" * 64,
                  "adapter": fr.DEFAULT_ADAPTER, "results": results, "cells": 600, "vertices_per_cell": 30480,
                  "invocation_count_failures": 0}
        output = self.check_record("geometry-s4", "wj", "S4", "geometry", record)
        self.assertEqual((output["status"], output["fixture"], output["format"]),
                         ("pass", "wj-s4", "magic600-l2-geometry-record-v1"))
        missing = dict(record, results=results[1:])
        directory = self.base / "geometry-missing"
        directory.mkdir()
        write_json(directory / "harness.json", harness("wj", mode="geometry", menu="S4", run_id="c9-missing"))
        write_json(directory / "geometry.json", missing)
        self.assertEqual(finalize(directory, mode="geometry", overlays=None), 5, "every projection is required")

    def test_w3_geometry_check_against_the_sb_reference(self):
        index = json.loads((ROOT / "work" / "experiments" / "renderer-sb" / "reference" / "index.json").read_bytes())
        pairs = sorted({name.rpartition(".")[0].rpartition("_")[::2] for name in index["files"] if name.endswith(".f32")})
        self.assertTrue(pairs)
        record = {"format": "magic600-sb-geometry-check-v1", "status": "pass", "build_identity": "e" * 64,
                  "adapter": fr.DEFAULT_ADAPTER,
                  "results": [{"camera": camera, "pose": pose, "samples": index["sample_count"],
                               "max_abs_error": 0.0, "failures": 0, "status": "pass"} for camera, pose in pairs],
                  "per_cell_count": {"cells": index["cells"], "expected": index["per_cell_vertex_count"],
                                     "failures": 0, "counts": [index["per_cell_vertex_count"]] * index["cells"],
                                     "status": "pass"}}
        self.assertEqual(self.check_record("geometry-w3", "w3", None, "geometry", record)["status"], "pass")

    def test_lattice_check(self):
        def record(failures):
            states = [{"state": state, "status": "fail" if failures and state == "lattice-retained" else "pass",
                       "slots": 259800, "label_failures": 0, "center_failures": 0, "geometry_bounds_failures": 0,
                       "lattice_failures": failures if state == "lattice-retained" else 0}
                      for state in fr.LATTICE_STATES]
            return {"format": "magic600-wj-lattice-check/1", "status": "fail" if failures else "pass",
                    "build_identity": "e" * 64, "adapter": fr.DEFAULT_ADAPTER, "states": states}
        self.assertEqual(self.check_record("lattice", "wj", "S4", "lattice", record(0))["status"], "pass")
        self.assertEqual(self.check_record("lattice-fail", "wj", "S4", "lattice", record(4), 2)["status"], "fail")


if __name__ == "__main__":
    result = unittest.main(exit=False, verbosity=2).result
    sys.exit(0 if result.wasSuccessful() else 1)
