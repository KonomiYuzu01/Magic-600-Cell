"""Headless checks of the C9 acceptance harness (renderer/acceptance/).

The harness runs only on Windows with Godot, PresentMon and a Direct3D 12 device. These checks need only
the Python standard library, start no process, and run anywhere. They check that:
- every source digest in renderer/acceptance/PROVENANCE.md still matches the selection evidence under
  work/experiments/, every "verbatim" file equals its source, and the table lists exactly the harness files;
- the Godot project's interop layer (project/Native.cs) matches the core's ABI 8 header
  renderer/core/include/sa2_interop.h: the x64 layout of every struct it declares, its constants, and the
  signature of every entry point it imports; it declares every ABI 1 to 3 struct and imports every ABI 1 to 3
  call (the texture routes use no ABI 4 or ABI 5 call; route B, C9 U4, imports ABI 5's sa2_create and three
  surface calls);
- the injection, menu and scene lists agree across the header, the project and the run script;
- the run finalizer accepts small synthetic W-J and W3 runs and checks, refuses or fails broken ones, and
  writes records that the unchanged renderer gate (tools/perf/renderer_gate.py) judges as the evidence was;
- route B (C9 U4): a synthetic route-B run is measured on the core's declared chain at sync interval 1, and
  the route-B build keeps the product shell's window settings, Godot's vsync and the guard over override.cfg;
- user stages (C9 U5c): synthetic stage runs are accepted with a stages record, or refused for a failed compile,
  a measured frame without a row or a stopped frame; the stage build's options, identity check and guard, the
  harness argument and the runner flag agree; the gate stages declare one overdraw that limits.json allows.
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
        # C9 U5c: the user-stage report row the harness reads, 64 bytes on x64 and size-checked at load.
        self.assertIn("Sa2StageFrame", native)
        self.assertEqual(x64_layout(header["sa2_stage_frame"]), ([0, 4, 8, 16, 24, 32, 40, 48, 56], 64))
        self.assertIn("public const int StageFrameSize = 64;", self.native)

    def test_constants_match_the_header(self):
        defines = dict(re.findall(r"^#define (SA2_\w+) (\S+)", self.header, re.M))
        constants = re.findall(r"public const (uint|int) (SA2_\w+) = (\d+);", self.native)
        self.assertEqual(len({name for _, name, _ in constants}), len(constants))
        for kind, name, value in constants:
            with self.subTest(constant=name):
                self.assertIn(name, defines)
                self.assertEqual(int(defines[name].rstrip("u"), 0), int(value))
                self.assertEqual(kind == "uint", defines[name].endswith("u"))
        wanted = {name for name in defines
                  if re.fullmatch(r"SA2_(ABI_VERSION|OK|E_\w+|QUEUE_\w+|BARRIERS_\w+|STATE_\w+|SCENE_NO_VRAM)", name)}
        self.assertEqual(sorted(wanted - {name for _, name, _ in constants}), [])

    def test_abi_version_8_is_required(self):
        self.assertEqual(re.findall(r"^#define SA2_ABI_VERSION (\S+)$", self.header, re.M), ["8u"])
        self.assertIn("public const uint SA2_ABI_VERSION = 8;", self.native)
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


def finalize(directory: Path, mode="run", overlays=OVERLAYS, adapter=None, fault_injection=False, unattended=False,
             build=None):
    """The finalizer with the Windows-only build guard replaced by a fixed identity and binding."""
    output = io.StringIO()
    with mock.patch.object(fr, "build_identity", lambda *arguments: (dict(build or BUILD), dict(BINDING))), \
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


# --- Route B (C9 U4) ------------------------------------------------------------------------------------

CORE_CHAIN = "0x000001F2A3B4E000"       # as the harness declares it (Level2.InitializeSurface: "0x" + X16)
CORE_CHAIN_CSV = "0x000001f2a3b4e000"   # PresentMon's own spelling, which the record keeps for the gate
WINDOW_CHAIN = "0x000001F2A3B4C5D0"     # Godot's window chain, also presenting


PRESENT, DISPLAY, COUNT = 5, 21, 100   # the core presents 5 ticks after a trace entry; shown 16 ticks later


def surface_csv(sync=1, core_dropped=1, window_dropped=range(0), missing=()) -> str:
    """The core's chain PRESENT ticks after each trace entry, all Dropped as PresentMon saw it on 20 Nov 2026, and
    Godot's window chain at about 60 Hz; window_dropped is a tick range of dropped window presents."""
    lines = ["Application,ProcessID,SwapChainAddress,PresentRuntime,SyncInterval,PresentFlags,AllowsTearing,"
             "PresentMode,Dropped,QPCTime,msBetweenPresents,msBetweenDisplayChange"]
    rows = [(T0 + frame * STEP + PRESENT, f"godot.exe,{PID},{CORE_CHAIN_CSV},DXGI,{sync},0,0,Composed: Flip,"
             f"{core_dropped},{T0 + frame * STEP + PRESENT},20.000,20.000") for frame in range(FRAMES) if frame not in missing]
    rows += [(tick, f"godot.exe,{PID},{WINDOW_CHAIN},DXGI,1,0,0,Composed: Flip,{int(tick in window_dropped)},{tick},"
              f"16.000,16.000") for tick in range(T0 + 3, STOP, 16)]
    return "\n".join(lines + [line for _, line in sorted(rows)]) + "\n"


def presented_rows() -> list[dict]:
    """presented.jsonl as the core writes it when every traced frame was confirmed displayed."""
    return [{"frame": frame, "present_count": COUNT + frame, "display_qpc": T0 + frame * STEP + DISPLAY}
            for frame in range(FRAMES)]


def write_presented(directory: Path, rows) -> None:
    (directory / "presented.jsonl").write_bytes("".join(
        json.dumps(row, sort_keys=True, separators=(",", ":")) + "\n" for row in rows).encode("utf-8"))


def surface_run(directory: Path, run_id: str, declared=CORE_CHAIN, sync=1, route="surface", presented=None,
                **csv) -> Path:
    write_run(directory, "w3", run_id=run_id)
    record = read(directory / "harness.json")
    record["configuration"]["route"] = route
    record["presentation"] = {"route": "surface", "swap_chain": declared, "sync_interval": 1}
    write_json(directory / "harness.json", record)
    (directory / "presentmon.csv").write_bytes(surface_csv(sync, **csv).encode("utf-8"))
    if presented is not False:
        write_presented(directory, presented_rows() if presented is None else presented)
    return directory


@unittest.skipIf(fr is None, f"finalize_run does not import here: {FINALIZER_ERROR!r}")
class RouteBFinalizerTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.base = Path(self.temporary.name)

    def tearDown(self):
        self.temporary.cleanup()

    def test_route_b_runs_measure_the_declared_chain(self):
        self.assertEqual(fr.SURFACE_SYNC_INTERVAL, 1)
        self.assertIn("#define SA2_SURFACE_SYNC_INTERVAL 1u", text(HEADER))
        runs = [surface_run(self.base / f"b{index}", f"c9-w3-b-{index}") for index in range(3)]
        windows = len(range(T0 + 3, STOP, 16))
        for run in runs:
            self.assertEqual(finalize(run), 0)
            record = read(run / "run.json")
            self.assertEqual(record["presentmon"]["swap_chain"], CORE_CHAIN_CSV)
            self.assertEqual(record["l2"]["presentation"],
                             {"route": "surface", "swap_chain": CORE_CHAIN, "sync_interval": 1,
                              "swap_chains_ignored": [{"swap_chain": WINDOW_CHAIN, "rows": windows}],
                              "display": "core-frame-statistics", "confirmed_frames": FRAMES})
            self.assertEqual(record["l2"]["checks"]["presented"], "pass")
            self.assertEqual(record["l2"]["presentmon"]["rows"], FRAMES)
            self.assertEqual(record["l2"]["presentmon"]["sync_intervals"], {"1": FRAMES})
            self.assertIs(record["environment"]["tearing"], False)
        result = scene_result(gate.summarize_private(runs), "w3")
        self.assertEqual((result["verdict"], len(result["runs"])), ("met", 3))
        self.assertAlmostEqual(result["pooled"]["fps"], 50.0)

    def test_route_b_blind_seconds_come_from_the_cores_frame_statistics(self):
        """C9, owner item 9 (20 Nov 2026): PresentMon marks every present of the core's composition chain Dropped,
        so route B's displayed seconds come from the frames the core confirmed with its own frame statistics,
        cross-checked against PresentMon; PresentMon's Dropped column of the core's chain no longer decides."""
        self.assertEqual(fr.SURFACE_DISPLAY, "core-frame-statistics")
        for dropped in (0, 1):
            with self.subTest(core_dropped=dropped):
                run = surface_run(self.base / f"dropped-{dropped}", f"c9-dropped-{dropped}", core_dropped=dropped)
                self.assertEqual(finalize(run), 0)
        # A second without a confirmed display: frames 999 to 1048 are shown in [T0 + 20 s, T0 + 21 s).
        unseen = [row for row in presented_rows() if not T0 + 20 * FREQ <= row["display_qpc"] < T0 + 21 * FREQ]
        self.assertEqual(len(unseen), FRAMES - 50)
        # A second whose window presents were all dropped.
        cases = {"confirmed": {"presented": unseen},
                 "window": {"window_dropped": range(T0 + 30 * FREQ, T0 + 31 * FREQ)}}
        for name, options in cases.items():
            with self.subTest(blind=name):
                run = surface_run(self.base / f"blind-{name}", f"c9-blind-{name}", **options)
                self.assertEqual(finalize(run), 5)
                self.assertEqual(read(run / "refusal.json")["reasons"], ["blind-seconds"])
        # Seconds outside the checked window need no confirmation.
        early = [row for row in presented_rows() if row["display_qpc"] >= T0 + 10 * FREQ]
        self.assertEqual(finalize(surface_run(self.base / "early", "c9-early", presented=early)), 0)

    def test_route_b_presented_refusals(self):
        def changed(frame, **values):
            return [{**row, **values} if row["frame"] == frame else row for row in presented_rows()]

        gap = [{**row, "present_count": row["present_count"] + 1} if row["frame"] >= 500 else row
               for row in presented_rows()]
        cases = {
            "missing": {"presented": False},
            "empty-line": {"presented": presented_rows()[:3] + [None]},
            "count-gap": {"presented": gap},                                       # a frame presented twice
            "extra-field": {"presented": changed(7, hr=0)},
            "frame-range": {"presented": presented_rows() + [{"frame": FRAMES, "present_count": COUNT + FRAMES,
                                                              "display_qpc": STOP + DISPLAY}]},
            "float": {"presented": changed(9, display_qpc=T0 + 9 * STEP + DISPLAY + 0.5)},
            "display-order": {"presented": changed(301, display_qpc=T0 + 300 * STEP + DISPLAY)},
            "before-present": {"presented": changed(300, display_qpc=T0 + 300 * STEP + PRESENT - 2)},
            "no-present": {"missing": (400,)},                                    # its own present is not captured
        }
        for name, options in cases.items():
            with self.subTest(case=name):
                rows = options.get("presented")
                if rows is not None and rows is not False and None in rows:
                    run = surface_run(self.base / name, f"c9-{name}", presented=False)
                    (run / "presented.jsonl").write_bytes(b"".join(
                        b"\n" if row is None else json.dumps(row, sort_keys=True, separators=(",", ":")).encode()
                        + b"\n" for row in rows))
                else:
                    run = surface_run(self.base / name, f"c9-{name}", **options)
                self.assertEqual(finalize(run), 5)
                reasons = read(run / "refusal.json")["reasons"]
                self.assertIn("presented", reasons)
                if name != "no-present":
                    self.assertEqual(reasons, ["presented"])
        stray = write_run(self.base / "stray", "w3", run_id="c9-stray")   # a texture run never writes one
        write_presented(stray, presented_rows())
        self.assertEqual(finalize(stray), 5)
        self.assertEqual(read(stray / "refusal.json")["reasons"], ["presented"])

    def test_stage_rows_end_with_one_newline(self):
        """C9 U5: the harness wrote a newline per row and WriteNew one more, a blank line the finalizer refuses."""
        source = text(ACCEPTANCE / "project" / "Level2.cs")
        self.assertIn("Encoding.UTF8.GetBytes(string.Join('\\n', _stageRows))", source)
        self.assertNotIn("rows.Append(row).Append('\\n')", source)
        writer = source[source.index("private static void WriteNew("):]
        self.assertIn("stream.WriteByte((byte)'\\n');", writer[:writer.index("\n    }")])

    def test_texture_runs_keep_their_rules(self):
        plain = write_run(self.base / "plain", "w3", run_id="c9-plain")
        self.assertEqual(finalize(plain), 0)
        self.assertNotIn("presented", read(plain / "run.json")["l2"]["checks"])
        self.assertNotIn("display", read(plain / "run.json")["l2"].get("presentation") or {})
        run = write_run(self.base / "texture", "w3", run_id="c9-texture")
        csv_text = (run / "presentmon.csv").read_text(encoding="utf-8")
        (run / "presentmon.csv").write_bytes((csv_text + f"godot.exe,{PID},{WINDOW_CHAIN[:-1]}1,DXGI,0,0,1,"
                                              f"Composed: Flip,0,{T0 + 11},20.000,20.000\n").encode("utf-8"))
        self.assertEqual(finalize(run), 5)
        self.assertEqual(read(run / "refusal.json")["reasons"], ["swap-chain"])

    def test_route_b_refusals(self):
        cases = {
            "swap-chain": {"declared": "0x000001F2A3B4F000"},   # the declared chain never presented
            "sync-interval": {"sync": 0},                        # the core presented without vsync
            "harness": {"route": "export"},                      # a presentation record on a texture route
        }
        for reason, options in cases.items():
            with self.subTest(reason=reason):
                run = surface_run(self.base / reason, f"c9-{reason}", **options)
                self.assertEqual(finalize(run), 5)
                self.assertEqual(read(run / "refusal.json")["reasons"], [reason])
                self.assertFalse((run / "run.json").exists())
        for name, declared in (("short", "0x1F2A3B4E000"), ("lower", CORE_CHAIN_CSV), ("zero", "0x" + "0" * 16)):
            with self.subTest(declared=name):
                run = surface_run(self.base / name, f"c9-{name}", declared=declared)
                self.assertEqual(finalize(run), 5)
                self.assertEqual(read(run / "refusal.json")["reasons"], ["harness"])
        bare = surface_run(self.base / "bare", "c9-bare")
        record = read(bare / "harness.json")
        record["presentation"] = None   # a route-B configuration without the presentation record
        write_json(bare / "harness.json", record)
        self.assertEqual(finalize(bare), 5)
        self.assertEqual(read(bare / "refusal.json")["reasons"], ["harness"])


class RouteBBuildTests(unittest.TestCase):
    """The route-B build: the shell's window settings, Godot's vsync, the runner's flag and the guard."""

    EXECUTABLE = "C:\\godot\\Godot_v4.7.2-stable_mono_win64.exe"

    def setUp(self):
        try:
            import prepare_host
        except (ImportError, OSError) as error:
            self.skipTest(f"prepare_host does not import here: {error!r}")
        self.host = prepare_host

    @staticmethod
    def settings(path: Path) -> dict:
        section, values = "", {}
        for line in text(path).splitlines():
            line = line.strip()
            if not line or line.startswith(";"):
                continue
            if line.startswith("["):
                section = line.strip("[]")
            elif "=" in line:
                key, value = line.split("=", 1)
                values[f"{section}/{key}" if section else key] = value
        return values

    def test_override_keeps_the_shell_window_settings(self):
        override = self.settings(self.host.OVERRIDE)
        shell = self.settings(ROOT / "renderer" / "shell" / "project" / "project.godot")
        self.assertEqual(self.host.OVERRIDE, ACCEPTANCE / "route-b" / "override.cfg")
        self.assertEqual(set(override), {"display/window/size/transparent", "display/window/per_pixel_transparency/allowed",
                                         "display/window/vsync/vsync_mode", "rendering/viewport/transparent_background",
                                         "rendering/environment/defaults/default_clear_color"})
        self.assertEqual(override, {key: shell[key] for key in override})
        self.assertEqual(self.settings(PROJECT / "project.godot")["display/window/vsync/vsync_mode"], "0")
        self.assertIn("renderer/acceptance/route-b", self.host.SOURCES)

    def test_launch_keeps_vsync_only_on_route_b(self):
        build = ROOT / "work" / "build" / "c9-acceptance" / "20261119T000000Z"
        texture = self.host.launch_value(build, self.EXECUTABLE, {})
        surface = self.host.launch_value(build, self.EXECUTABLE, {}, "surface")
        self.assertEqual(texture, self.host.launch_value(build, self.EXECUTABLE, {}, "texture"))
        self.assertIn("--disable-vsync", texture["arguments"])
        self.assertEqual(surface["arguments"], [a for a in texture["arguments"] if a != "--disable-vsync"])
        self.assertEqual({k: v for k, v in surface.items() if k != "arguments"},
                         {k: v for k, v in texture.items() if k != "arguments"})
        with self.assertRaises(ValueError):
            self.host.launch_value(build, self.EXECUTABLE, {}, "export")
        self.assertEqual(self.host.ROUTES, ("texture", "surface"))

    def test_runner_and_harness_name_the_route(self):
        runner, arguments = text(ACCEPTANCE / "run_scene.ps1"), text(PROJECT / "Level2Arguments.cs")
        self.assertIn("if ($route -eq 'surface') { $arguments += @('--sa2-route','surface') }", runner)
        self.assertIn("@('texture','surface') -cnotcontains $route", runner)
        self.assertIn('"surface"', re.search(r'case "--sa2-route": a\.route = Choice\(value, ([^)]*)\);', arguments).group(1))
        level2 = text(PROJECT / "Level2.cs")
        detach = level2.index('_native.sa2_surface_detach(_context)')
        self.assertLess(level2.index('_native.sa2_scene_unload(_context)'), detach)
        self.assertLess(detach, level2.index('_native.sa2_detach(_context)'))
        self.assertIn("Native.SA2_TARGET_SURFACE, frame", level2)

    @unittest.skipIf(fr is None, f"file_guard does not import here: {FINALIZER_ERROR!r}")
    def test_guard_protects_a_route_b_override(self):
        import file_guard
        with tempfile.TemporaryDirectory() as temporary:
            base = Path(temporary)
            project = base / "project"
            debug = project / ".godot" / "mono" / "temp" / "bin" / "Debug"
            debug.mkdir(parents=True)
            (debug / "SA2Smoke.dll").write_bytes(b"")
            launch = {"format": "magic600-l2-launch-v1", "candidate": "sa2", "executable": str(base / "godot.exe"),
                      "dll": str(base / "native" / "sa2_interop.dll"), "working_directory": str(project)}
            names = lambda: {Path(path).name for path in file_guard.protected_set(launch, "sa2")}
            self.assertNotIn("override.cfg", names())
            (project / "override.cfg").write_bytes(b"")
            self.assertIn("override.cfg", names())


# --- User stages (C9 U5c) -------------------------------------------------------------------------------

STAGES = ACCEPTANCE / "stages"
STAGE_FILE_NAMES = ("arrays.json", "fragment.hlsl", "fragment.json", "vertex.hlsl", "vertex.json")
STAGE_DLLS = ("dxcompiler.dll", "dxil.dll")
STAGE_KIND_NAMES = ("vertex", "fragment")
STAGE_HANDLES = {"vertex": 7, "fragment": 9}
STAGE_OVERDRAWS = {"vertex": 120, "fragment": 96}   # the harness attaches the smaller
STAGE_PREROLL = 40                                   # chain frames drawn before the trace
STAGE_ALLOWANCE = SIZE["width"] * SIZE["height"] * min(STAGE_OVERDRAWS.values())
D4_BITS = "0x3f970a3d"                               # 1.18f, the built-in projection's distance
STAGE_PARTS = {f"stage:{name}": hashlib.sha256(f"stage:{name}".encode()).hexdigest() for name in STAGE_FILE_NAMES}
STAGE_PARTS.update({f"dxc:{name}": hashlib.sha256(name.encode()).hexdigest() for name in STAGE_DLLS})
STAGE_BUILD = dict(BUILD, parts=[{"name": name, "sha256": digest} for name, digest in sorted(STAGE_PARTS.items())])
STAGE_COMPILER = {"library": "dxcompiler.dll", "version": "1.9", "commit_count": 5000, "commit_hash": "0123abc",
                  "custom_version": "", "validator": {"library": "dxil.dll", "version": "1.9"}}
# The largest raster bound per pixel of W3 over 1920x1080, 2560x1440 and 2560x1600 (C9 U5c, renderer/acceptance/
# overdraw: 1920x1080, frame 5803, turn angle -3pi/8), and the sizing margin.
W3_LARGEST_BOUND_PER_PIXEL = 85.6749
OVERDRAW_MARGIN = 1.25


def stage_pass(entry, profile, status="compiled"):
    return {"entry": entry, "profile": profile, "arguments": ["-E", entry, "-T", profile, "-HV", "2021"],
            "source_sha256": HASH, "dxil_sha256": "e" * 64 if status == "compiled" else None, "status": status}


def stage_compile(kind, status="compiled"):
    """The core's compile record (renderer/core/src/stages.cpp, Registry::compile) as the harness writes it."""
    compiled = status == "compiled"
    profile = "cs_6_6" if kind == "vertex" else "ps_6_6"
    return {"format": "magic600-stage-compile/1", "kind": kind, "status": status, "code": None if compiled else -9,
            "handle": STAGE_HANDLES[kind] if compiled else 0, "stage_sha256": STAGE_PARTS[f"stage:{kind}.hlsl"],
            "compiler": STAGE_COMPILER, "passes": [stage_pass("main", profile, "compiled" if compiled else "failed")],
            "core_passes": [stage_pass("eval", "cs_6_6"), stage_pass("bound", "cs_6_6")],
            "error": "" if compiled else f"main ({profile}): error X3004: undeclared identifier", "error_truncated": False}


def stage_row(frame, bound=None, stopped=None):
    bound = 3 * STAGE_ALLOWANCE // 4 if bound is None else bound
    return {"frame": frame, "stopped": int(bound > STAGE_ALLOWANCE) if stopped is None else stopped, "bound": bound,
            "allowance": STAGE_ALLOWANCE, "vertex_stage": STAGE_HANDLES["vertex"],
            "fragment_stage": STAGE_HANDLES["fragment"], "user_pixel_invocations": 1000,
            "builtin_pixel_invocations": 0}


def stage_rows():
    """One row per chain frame: the preroll, then the measured frames; the first measured frame's bound is exactly
    the allowance, which does not stop it."""
    return [stage_row(frame, STAGE_ALLOWANCE if frame == STAGE_PREROLL + 1 else None)
            for frame in range(1, STAGE_PREROLL + FRAMES + 1)]


def write_rows(directory: Path, rows) -> None:
    (directory / "stage-frames.jsonl").write_bytes("".join(json.dumps(row) + "\n" for row in rows).encode("utf-8"))


def stage_run(directory: Path, run_id: str, failed_kind=None, rows=None) -> Path:
    """A W3 run with the user stages attached (Level2.AttachStages), or with `failed_kind`'s compile failed."""
    write_run(directory, "w3", run_id=run_id)
    record = read(directory / "harness.json")
    folder = "C:\\c9\\build\\stages"
    record["files"].update({f"stage:{name}": f"{folder}\\{name}" for name in STAGE_FILE_NAMES})
    record["files"].update({f"dxc:{name}": f"C:\\c9\\build\\native\\{name}" for name in STAGE_DLLS})
    rows = stage_rows() if rows is None else rows
    stages = {"directory": folder, "contract": "m600-stage-hlsl/1", "overdraw": min(STAGE_OVERDRAWS.values()),
              "d4": {"bits": D4_BITS, "builtin_bits": D4_BITS, "equal": True}, "attached": failed_kind is None,
              "rows": 0, "trace_frames": None}
    for kind in STAGE_KIND_NAMES:
        compile_record = stage_compile(kind, "compile_failed" if kind == failed_kind else "compiled")
        stages[kind] = {"digest": hashlib.sha256(kind.encode()).hexdigest(), "hlsl_sha256": compile_record["stage_sha256"],
                        "overdraw": STAGE_OVERDRAWS[kind], "arrays": 1 if kind == "vertex" else 0,
                        "handle": compile_record["handle"], "compile": compile_record["status"]}
        (directory / f"stage-compile-{kind}.json").write_bytes((json.dumps(compile_record) + "\n").encode("utf-8"))
        if kind == failed_kind:
            break   # the harness stops at the first failed compile
    if failed_kind is None:
        stages.update(rows=len(rows), trace_frames={"first": STAGE_PREROLL + 1, "last": STAGE_PREROLL + FRAMES})
        write_rows(directory, rows)
    record["stages"] = stages
    write_json(directory / "harness.json", record)
    return directory


@unittest.skipIf(fr is None, f"finalize_run does not import here: {FINALIZER_ERROR!r}")
class StageFinalizerTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.base = Path(self.temporary.name)

    def tearDown(self):
        self.temporary.cleanup()

    def refused(self, run, reason):
        self.assertEqual(finalize(run, build=STAGE_BUILD), 5)
        self.assertEqual(read(run / "refusal.json")["reasons"], [reason])
        self.assertFalse((run / "run.json").exists())

    def test_constants_match_the_harness(self):
        self.assertEqual(fr.STAGE_FILES, STAGE_FILE_NAMES)
        self.assertEqual(fr.STAGE_LIBRARIES, STAGE_DLLS)
        self.assertEqual(fr.STAGE_KINDS, STAGE_KIND_NAMES)
        self.assertEqual(fr.BUILTIN_D4_BITS, D4_BITS)
        level2, arguments = text(PROJECT / "Level2.cs"), text(PROJECT / "Level2Arguments.cs")
        self.assertIn("BuiltinD4 = 1.18f", level2)
        self.assertIn(f'StageContract = "{fr.STAGE_CONTRACT}"', level2)
        for output in fr.STAGE_OUTPUTS:
            self.assertIn(f'"{output}"', arguments)
        self.assertEqual(re.findall(r'\["(\w+)"\] = row\.\w+', level2), list(fr.STAGE_ROW_FIELDS))
        self.assertIn(f'RecordFormat[] = "{fr.STAGE_COMPILE_FORMAT}"', text(ROOT / "renderer" / "core" / "src" / "stage_rules.h"))

    def test_stage_runs_pass_and_three_meet_the_gate(self):
        runs = [stage_run(self.base / f"s{index}", f"c9-w3-stages-{index}") for index in range(3)]
        for run in runs:
            self.assertEqual(finalize(run, build=STAGE_BUILD), 0)
            record = read(run / "run.json")
            stages = record["stages"]
            self.assertEqual((stages["contract"], stages["overdraw"], stages["allowance"]),
                             ("m600-stage-hlsl/1", 96, STAGE_ALLOWANCE))
            self.assertEqual((stages["rows"], stages["measured_frames"], stages["measured_frames_with_row"]),
                             (STAGE_PREROLL + FRAMES, FRAMES, FRAMES))
            self.assertEqual((stages["stopped_frames"], stages["stopped"]), (0, []))
            self.assertEqual((stages["largest_bound"], stages["largest_bound_frame"], stages["largest_bound_per_pixel"]),
                             (STAGE_ALLOWANCE, STAGE_PREROLL + 1, 96.0))
            self.assertEqual(stages["trace_frames"], {"first": STAGE_PREROLL + 1, "last": STAGE_PREROLL + FRAMES})
            self.assertEqual(stages["measured_user_pixel_invocations"], 1000 * FRAMES)
            self.assertEqual(stages["d4"]["bits"], D4_BITS)
            self.assertEqual(stages["dxc"], {name: STAGE_PARTS[f"dxc:{name}"] for name in STAGE_DLLS})
            self.assertEqual(stages["arrays_sha256"], STAGE_PARTS["stage:arrays.json"])
            for kind in STAGE_KIND_NAMES:
                self.assertEqual(stages[kind]["handle"], STAGE_HANDLES[kind])
                self.assertEqual(stages[kind]["json_sha256"], STAGE_PARTS[f"stage:{kind}.json"])
                self.assertEqual(stages[kind]["compile"]["status"], "compiled")
            self.assertEqual({code: record["l2"]["checks"][code] for code in fr.STAGE_CHECKS},
                             dict.fromkeys(fr.STAGE_CHECKS, "pass"))
        result = scene_result(gate.summarize_private(runs), "w3")
        self.assertEqual((result["verdict"], len(result["runs"])), ("met", 3))

    def test_runs_without_stages_list_no_stage_checks(self):
        plain = write_run(self.base / "plain", "w3", run_id="c9-plain")
        self.assertEqual(finalize(plain), 0)
        record = read(plain / "run.json")
        self.assertNotIn("stages", record)
        self.assertFalse(set(fr.STAGE_CHECKS) & set(record["l2"]["checks"]))
        stray = write_run(self.base / "stray", "w3", run_id="c9-stray")
        write_rows(stray, stage_rows()[:3])   # a stage output in a run without stages
        self.assertEqual(finalize(stray), 5)
        self.assertEqual(read(stray / "refusal.json")["reasons"], ["stage-compile"])

    def test_failed_compile_is_refused(self):
        for kind in STAGE_KIND_NAMES:
            with self.subTest(failed=kind):
                run = stage_run(self.base / f"failed-{kind}", f"c9-failed-{kind}", failed_kind=kind)
                self.refused(run, "stage-compile")
        missing = stage_run(self.base / "missing-record", "c9-missing-record")
        (missing / "stage-compile-fragment.json").unlink()
        self.refused(missing, "stage-compile")
        other = stage_run(self.base / "other-hlsl", "c9-other-hlsl")
        record = read(other / "stage-compile-vertex.json")
        record["stage_sha256"] = "d" * 64   # compiled from another HLSL than the build's
        write_json(other / "stage-compile-vertex.json", record)
        self.refused(other, "stage-compile")
        unlisted = stage_run(self.base / "unlisted", "c9-unlisted")
        self.assertEqual(finalize(unlisted), 5, "the stage files must be among the build's parts")
        self.assertEqual(read(unlisted / "refusal.json")["reasons"], ["stage-compile"])

    def test_missing_rows_are_refused(self):
        cases = {
            "no-file": None,
            "measured-gap": [row for row in stage_rows() if row["frame"] != STAGE_PREROLL + 100],
            "short-trace": stage_rows()[:-1],
            "order": [dict(reversed(list(row.items()))) for row in stage_rows()],
            "handles": [dict(row, fragment_stage=8) for row in stage_rows()],
            "flag": [dict(row, stopped=0, bound=STAGE_ALLOWANCE + 1) if row["frame"] == 5 else row
                     for row in stage_rows()],
        }
        for name, rows in cases.items():
            with self.subTest(case=name):
                run = stage_run(self.base / name, f"c9-rows-{name}", rows=rows)
                if rows is None:
                    (run / "stage-frames.jsonl").unlink()
                self.refused(run, "stage-rows")
        miscounted = stage_run(self.base / "miscounted", "c9-miscounted")
        record = read(miscounted / "harness.json")
        record["stages"]["rows"] += 1   # the harness counted a row the file lacks
        write_json(miscounted / "harness.json", record)
        self.refused(miscounted, "stage-rows")

    def test_stopped_frames_are_refused(self):
        for name, frame in (("measured", STAGE_PREROLL + 7), ("preroll", 3)):
            with self.subTest(frame=name):
                rows = [stage_row(row["frame"], STAGE_ALLOWANCE + 1) if row["frame"] == frame else row
                        for row in stage_rows()]
                self.refused(stage_run(self.base / name, f"c9-stopped-{name}", rows=rows), "stage-stopped")

    def test_stage_harness_refusals(self):
        run = stage_run(self.base / "d4", "c9-d4")
        record = read(run / "harness.json")
        record["stages"]["d4"] = {"bits": "0x3f800000", "builtin_bits": D4_BITS, "equal": False}
        write_json(run / "harness.json", record)
        self.refused(run, "stage-compile")
        larger = stage_run(self.base / "larger", "c9-larger")
        record = read(larger / "harness.json")
        record["stages"]["overdraw"] = max(STAGE_OVERDRAWS.values())   # not the smaller declared value
        write_json(larger / "harness.json", record)
        self.assertEqual(finalize(larger, build=STAGE_BUILD), 5)
        self.assertIn("stage-compile", read(larger / "refusal.json")["reasons"])
        injected = inject(stage_run(self.base / "injected", "c9-injected"), "stale-binding")
        self.assertEqual(finalize(injected, overlays=None, fault_injection=True, build=STAGE_BUILD), 5)
        self.assertIn("harness", read(injected / "refusal.json")["reasons"])


class StageBuildTests(unittest.TestCase):
    """The stage build (prepare_host --stages), its identity check, the guard, the harness argument and the runner."""

    def setUp(self):
        try:
            import prepare_host
        except (ImportError, OSError) as error:
            self.skipTest(f"prepare_host does not import here: {error!r}")
        self.host = prepare_host

    def test_options(self):
        for argv in (["--dxc", "C:\\dxc"], ["--verify", "C:\\b", "--stages", "renderer/acceptance/stages"],
                     ["--verify", "C:\\b", "--dxc", "C:\\dxc"]):
            with self.subTest(argv=argv), contextlib.redirect_stderr(io.StringIO()):
                with self.assertRaises(SystemExit) as raised:
                    self.host.main(argv)
                self.assertEqual(raised.exception.code, 2)
        self.assertEqual(self.host.STAGE_SOURCE, STAGES)
        self.assertEqual(self.host.STAGE_FILES, STAGE_FILE_NAMES)
        self.assertEqual(tuple(self.host.DXC_LIBRARIES), STAGE_DLLS)
        self.assertEqual(self.host.STAGE_TOOL_TARGET, ROOT / "work" / "build" / "c9-stage-tool")
        self.assertEqual(self.host.BUILTIN_D4_BITS, D4_BITS)

    def test_stage_sources(self):
        source = self.host.stage_source("renderer/acceptance/stages")
        self.assertEqual(source["directory"], "renderer/acceptance/stages")
        self.assertEqual(source["files"], {name: sha256(STAGES / name) for name in self.host.STAGE_INPUTS})
        self.assertEqual(sorted(p.name for p in STAGES.iterdir()), sorted(self.host.STAGE_INPUTS))
        with tempfile.TemporaryDirectory() as outside:
            with self.assertRaises(RuntimeError):
                self.host.stage_source(outside)
        arrays = self.host.read_arrays(STAGES / "arrays.json")
        self.assertEqual(arrays, {"vertex": {"d4": [D4_BITS]}, "fragment": {}})

    def test_gate_stages_declare_one_sized_overdraw(self):
        declared = {kind: int(re.search(rf"^stage {kind} \w+ ops \d+ overdraw (\d+)", text(STAGES / f"{kind}.6cs"),
                                        re.M).group(1)) for kind in STAGE_KIND_NAMES}
        limit = json.loads((STAGES / "limits.json").read_bytes())["max_overdraw"]
        self.assertEqual(set(declared.values()), {limit}, "both stages declare the limit's overdraw")
        self.assertGreaterEqual(limit, -(-W3_LARGEST_BOUND_PER_PIXEL * OVERDRAW_MARGIN // 1),
                                "the measured W3 bound with its margin")

    def test_dxc_folder(self):
        lock = self.host.dxc_lock()
        self.assertEqual(lock["id"], "dxc")
        self.assertTrue(lock["dest"].startswith("tools/.dxc/"))
        with tempfile.TemporaryDirectory() as empty:
            with self.assertRaisesRegex(RuntimeError, "--dxc"):
                self.host.dxc_directory(empty)

    def test_stage_inputs_are_checked_before_the_build(self):
        with tempfile.TemporaryDirectory() as temporary:
            builds = Path(temporary) / "builds"
            with mock.patch.object(self.host, "BUILD_ROOT", builds), \
                    mock.patch.object(self.host, "source_identity", lambda: {}):
                with self.assertRaises(RuntimeError):
                    self.host.build_host(Path("C:\\godot\\godot.exe"), Path("C:\\godot\\nupkgs"),
                                         stages_source=Path(temporary), dxc=Path(temporary))
                with self.assertRaisesRegex(RuntimeError, "lacks"):
                    self.host.build_host(Path("C:\\godot\\godot.exe"), Path("C:\\godot\\nupkgs"),
                                         stages_source=STAGES, dxc=Path(temporary))
            self.assertFalse(builds.exists(), "nothing is built when the stage inputs are missing")

    def stage_identity(self, build: Path):
        digest = lambda name: hashlib.sha256(name.encode()).hexdigest()
        parts = {f"stages:{name}": {"sha256": digest(name)} for name in STAGE_FILE_NAMES}
        parts.update({f"native:{name}": {"sha256": digest(name)} for name in STAGE_DLLS})
        parts["native:sa2_interop.dll"] = {"sha256": digest("core")}
        stages = {"directory": str(build / "stages"), "source": self.host.stage_source(STAGES),
                  "overdraw": 96, "arrays": {"sha256": digest("arrays.json")},
                  "d4": {"bits": D4_BITS, "builtin_bits": D4_BITS, "equal": True},
                  "dxc": {"version": "1.9.2609", "files": {name: {"sha256": digest(name)} for name in STAGE_DLLS}},
                  **{kind: {"hlsl_sha256": digest(f"{kind}.hlsl"), "json_sha256": digest(f"{kind}.json")}
                     for kind in STAGE_KIND_NAMES}}
        return {"parts": parts, "stages": stages}

    def test_verify_checks_the_stages_record(self):
        with tempfile.TemporaryDirectory() as temporary:
            build = Path(temporary)
            identity = self.stage_identity(build)
            self.assertIs(self.host.verify_stages(build, identity), identity["stages"])
            plain = {"parts": {"native:sa2_interop.dll": {"sha256": HASH}}}
            self.assertIsNone(self.host.verify_stages(build, plain))
            changes = {
                "unrecorded": lambda i: i.pop("stages"),
                "hlsl": lambda i: i["parts"]["stages:vertex.hlsl"].update(sha256=HASH),
                "arrays": lambda i: i["parts"]["stages:arrays.json"].update(sha256=HASH),
                "dxc": lambda i: i["parts"]["native:dxil.dll"].update(sha256=HASH),
                "missing": lambda i: i["parts"].pop("stages:fragment.json"),
                "d4": lambda i: i["stages"]["d4"].update(bits="0x3f800000"),
                "source": lambda i: i["stages"]["source"]["files"].update({"limits.json": HASH}),
                "folder": lambda i: i["stages"].update(directory=str(build / "other")),
            }
            for name, change in changes.items():
                with self.subTest(change=name):
                    changed = json.loads(json.dumps(identity))
                    change(changed)
                    with self.assertRaises(RuntimeError):
                        self.host.verify_stages(build, changed)
            (build / "stages").mkdir()
            with self.assertRaises(RuntimeError, msg="a stages folder without a record"):
                self.host.verify_stages(build, plain)

    def test_harness_and_runner_attach_the_stages(self):
        arguments, level2 = text(PROJECT / "Level2Arguments.cs"), text(PROJECT / "Level2.cs")
        runner = text(ACCEPTANCE / "run_scene.ps1")
        self.assertIn('case "--sa2-stages": a.stages_path = value; break;', arguments)
        self.assertIn('if (a.scene == "wj") throw new ArgumentException("user stages attach to W1..W4 only', arguments)
        self.assertIn('if (a.mode != "run" || a.inject != null) throw new ArgumentException(', arguments)
        self.assertIn("if (_args.stages_path != null) AttachStages();", level2)
        self.assertLess(level2.index("_native.sa2_scene_stages(_context, 0, 0, 0, null, 0, null, 0)"),
                        level2.index("private unsafe void ReleaseStages()"))
        self.assertIn("if ($null -ne $stageDirectory) { $arguments += @('--sa2-stages',$stageDirectory) }", runner)
        self.assertIn("$stageDirectory = Join-Path $buildDirectory 'stages'", runner)
        self.assertIn("if ($wj -or $Mode -ne 'run' -or $Inject) { throw 'A stage build runs -Scene w3", runner)

    @unittest.skipIf(fr is None, f"file_guard does not import here: {FINALIZER_ERROR!r}")
    def test_guard_protects_a_stage_build(self):
        import file_guard
        self.assertEqual(file_guard.STAGE_FILES, STAGE_FILE_NAMES)
        with tempfile.TemporaryDirectory() as temporary:
            base = Path(temporary)
            project = base / "project"
            debug = project / ".godot" / "mono" / "temp" / "bin" / "Debug"
            debug.mkdir(parents=True)
            (debug / "SA2Smoke.dll").write_bytes(b"")
            launch = {"format": "magic600-l2-launch-v1", "candidate": "sa2", "executable": str(base / "godot.exe"),
                      "dll": str(base / "native" / "sa2_interop.dll"), "working_directory": str(project)}
            protected = lambda: {Path(path).relative_to(base).as_posix()
                                 for path in file_guard.protected_set(launch, "sa2")}
            staged = {*(f"native/{name}" for name in STAGE_DLLS), *(f"stages/{name}" for name in STAGE_FILE_NAMES)}
            self.assertFalse(staged & protected())
            (base / "stages").mkdir()
            self.assertLessEqual(staged, protected())


if __name__ == "__main__":
    result = unittest.main(exit=False, verbosity=2).result
    sys.exit(0 if result.wasSuccessful() else 1)
