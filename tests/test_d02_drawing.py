"""Static checks of ADR-D02's drawing track (B18c onward); never builds, launches Godot or touches a GPU.

B18c draws Local beside Global as the second pane of the composite frame, with its own shape (Look Lab's
local-view.json): orthographic, only the named cap's turning region, pulled apart. It needs the core's ABI 9 view focus
(sa2_view_focus_set) and the host library's version 12 (m6h_set_view_focus). This file pins both sides of each
interface, the shell's use of them behind --d02 only, the chrome's opening for Local, and the two-views run's event
check. A pass is source evidence only; the WARP focus check (renderer/core/check_core.py --focus) and the two-views run
(renderer/shell/d02_run.py --script two-views) are the drawn evidence.
"""
from __future__ import annotations

import importlib
import json
import math
import re
import shutil
import sys
import unittest
import uuid
from contextlib import contextmanager
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SHELL = ROOT / "renderer" / "shell"
PROJECT = SHELL / "project"
CORE = ROOT / "renderer" / "core"
CHROME = ROOT / "tools" / "looklab" / "app" / "Chrome"
LOCAL_VIEW = ROOT / "tools" / "looklab" / "data" / "local-view.json"


def text(path: Path) -> str:
    return path.read_bytes().decode("utf-8")


def shell_module(name: str):
    sys.path.insert(0, str(SHELL))
    try:
        return importlib.import_module(name)
    finally:
        sys.path.remove(str(SHELL))


@contextmanager
def workspace_fixture(prefix):
    # Inherit the workspace's ACL. The 0o700 mode of mkdtemp restricts the Windows sandbox's test subprocesses.
    work = ROOT / "work"
    work.mkdir(exist_ok=True)
    if work.resolve().parent != ROOT:
        raise RuntimeError("test work folder escaped the checkout")
    folder = work / (prefix + uuid.uuid4().hex[:12])
    folder.mkdir()
    if folder.resolve().parent != work.resolve():
        raise RuntimeError("test fixture escaped work/")
    try:
        yield folder
    finally:
        shutil.rmtree(folder)


def fixture_folder(test, prefix):
    return test.enterContext(workspace_fixture(prefix))


class CoreFocus(unittest.TestCase):
    """The core's ABI 9 on its three sides: the C header, c600-render's Rust ABI and the bridge's function table."""

    def test_the_version_is_10_everywhere(self):
        header = text(CORE / "include" / "sa2_interop.h")
        self.assertEqual(re.findall(r"^#define SA2_ABI_VERSION (\S+)$", header, re.M), ["10u"])
        self.assertIn("pub const ABI_VERSION: u32 = 10;", text(ROOT / "rust" / "c600-render" / "src" / "abi.rs"))
        self.assertIn("public const uint AbiVersion = 10;", text(PROJECT / "CoreLibrary.cs"))
        self.assertIn("public const uint SA2_ABI_VERSION = 10;", text(ROOT / "renderer" / "acceptance" / "project" / "Native.cs"))

    def test_the_focus_struct_and_constants_agree(self):
        header = text(CORE / "include" / "sa2_interop.h")
        block = re.search(r"typedef struct sa2_view_focus \{(.*?)\} sa2_view_focus;", header, re.S)
        self.assertTrue(block)
        self.assertEqual(re.findall(r"(uint32_t|float) (\w+);", block.group(1)),
                         [("uint32_t", "struct_size"), ("uint32_t", "projection"), ("uint32_t", "focus_cell"),
                          ("float", "focus_cos"), ("float", "spread"), ("uint32_t", "reserved")])
        self.assertIn('static_assert(sizeof(sa2_view_focus) == 24, "sa2_view_focus requires the x64 ABI");', header)
        abi = text(ROOT / "rust" / "c600-render" / "src" / "abi.rs")
        host = text(PROJECT / "HostLibrary.cs")
        for name, c, rust, csharp in (("SA2_PROJECTION_PERSPECTIVE", "0u", "PROJECTION_PERSPECTIVE: u32 = 0", "ProjectionPerspective = 0"),
                                      ("SA2_PROJECTION_ORTHOGRAPHIC", "1u", "PROJECTION_ORTHOGRAPHIC: u32 = 1", "ProjectionOrthographic = 1"),
                                      ("SA2_FOCUS_NONE", "0xFFFFFFFFu", "FOCUS_NONE: u32 = 0xFFFF_FFFF","FocusNone = uint.MaxValue")):
            with self.subTest(constant=name):
                self.assertIn(f"#define {name} {c}", header)
                self.assertIn(f"pub const {rust};", abi)
                self.assertIn(f"public const uint {csharp};", host)
        # The default focus is perspective, no cell, -1 and spread 1, and it draws as ABI 8 (viewFlags 0).
        self.assertRegex(abi, r"projection: PROJECTION_PERSPECTIVE,\s*focus_cell: FOCUS_NONE,\s*focus_cos: -1\.0,\s*spread: 1\.0")
        self.assertIn("uint viewFlags; // ABI 9: bit 0 orthographic, bit 1 the focus test, bit 2 the spread; 0 draws as ABI 8",
                      text(CORE / "shaders" / "geometry.hlsl"))

    def test_the_bridge_binds_the_new_function_last(self):
        bridge = text(ROOT / "renderer" / "bridge" / "src" / "lib.rs")
        block = re.search(r"pub const SYMBOLS: \[&str; (\d+)\] = \[(.*?)\];", bridge, re.S)
        self.assertTrue(block)
        names = re.findall(r'"(sa2_[a-z_]+)"', block.group(2))
        # Each feature appends its exports after the previous ABI 10 rows.
        last = ["sa2_view_focus_set", "sa2_view_layout_set", "sa2_transform_set", "sa2_sticker_colours_set",
                "sa2_scene_look_set", "sa2_charts_set", "sa2_status_style_set", "sa2_status_drawn"]
        self.assertEqual(names[-len(last):], last)
        self.assertEqual(len(names), int(block.group(1)))
        core = text(PROJECT / "CoreLibrary.cs")
        exports = re.findall(r'"(sa2_[a-z_]+)"', re.search(r"\bSymbols = new\[\]\s*\{(.*?)\};", core, re.S).group(1))
        self.assertEqual(exports[-len(last):], last)
        self.assertIn("view_focus_set,\n        view_layout_set,\n        transform_set,\n        sticker_colours_set,\n        scene_look_set,\n        charts_set,\n        status_style_set,\n        status_drawn,\n    }\n}", text(ROOT / "renderer" / "bridge" / "tests" / "common" / "mod.rs"))


class HostFocus(unittest.TestCase):
    """The host library's version 12: m6h_set_view_focus, the same on the Rust and C# sides."""

    def test_the_host_and_the_shell_speak_version_15(self):
        lib = text(ROOT / "renderer" / "bridge" / "host" / "src" / "lib.rs")
        host = text(PROJECT / "HostLibrary.cs")
        self.assertIn("pub const M6H_ABI_VERSION: u32 = 15;", lib)
        self.assertIn("public const uint AbiVersion = 15;", host)
        self.assertIn("assert_eq!(m6h_abi_version(), 15);", text(ROOT / "renderer" / "bridge" / "host" / "tests" / "turns.rs"))
        self.assertRegex(lib, r'pub unsafe extern "C" fn m6h_set_view_focus\(\s*handle: \*mut ShellHost,\s*view: u64,\s*'
                              r'projection: u32,\s*focus_cell: u32,\s*focus_cos: f32,\s*spread: f32,\s*\) -> i32')
        self.assertIn("public int SetViewFocus(IntPtr host, ulong view, uint projection, uint focusCell, float focusCos, float spread)",
                      host)
        # An unopened view and a value out of range are refused before the core is called; the default clears it.
        self.assertIn("if !shell.names_open_view(id) {", lib[lib.index("fn m6h_set_view_focus("):])
        self.assertIn("let focus = (focus != Sa2ViewFocus::defaults()).then_some(focus);", lib)
        self.assertIn("fn the_host_sets_a_view_focus()", text(ROOT / "renderer" / "bridge" / "host" / "tests" / "c_api.rs"))


class HopfRings(unittest.TestCase):
    """B23, bare: the core's ABI 10 view layout and the host's version 13 m6h_set_view_layout; with --d02 Global opens
    on Look Lab's Hopf rings when the opening file names them, at the file's field of view, turned by the linked spin."""

    def test_the_layout_row_is_the_same_on_every_side(self):
        header = text(CORE / "include" / "sa2_interop.h")
        block = re.search(r"typedef struct sa2_view_layout \{(.*?)\} sa2_view_layout;", header, re.S)
        self.assertTrue(block)
        self.assertEqual(re.findall(r"(uint32_t|float|uint8_t) (\w+(?:\[\d+\])?);", block.group(1)),
                         [("uint32_t", "struct_size"), ("uint32_t", "kind"), ("float", "j[4]"), ("float", "k[4]"),
                          ("float", "origins[80]"), ("uint8_t", "ring_of[600]"), ("uint8_t", "half_step[600]")])
        abi = text(ROOT / "rust" / "c600-render" / "src" / "abi.rs")
        self.assertIn("pub origins: [f32; 80],\n    pub ring_of: [u8; 600],\n    pub half_step: [u8; 600],", abi)
        self.assertIn("public const int LayoutBytes = 1560;", text(PROJECT / "HostLibrary.cs"))
        hopf = text(PROJECT / "D02" / "ShellHopf.cs")
        # The C# row's offsets: J at 8, K at 24, the origins at 40, the rings at 360, the half-steps at 960.
        for offset in ('Floats(root.GetProperty("J"), 8, 4);', 'Floats(root.GetProperty("K"), 24, 4);',
                       'Floats(ring.GetProperty("origin"), 40 + 16 * r++, 4);', 'Bytes(root.GetProperty("ringOf"), 360);',
                       'Bytes(root.GetProperty("halfStep"), 960);'):
            self.assertIn(offset, hopf)
        self.assertIn("fn the_host_sets_a_view_layout()", text(ROOT / "renderer" / "bridge" / "host" / "tests" / "c_api.rs"))

    def test_the_core_checks_the_rings_against_its_cells(self):
        product = text(CORE / "src" / "product.cpp")
        self.assertIn("constexpr double RingRadius = 2.62, BaseRadius = 8.063, RingScale = 0.28", product)
        self.assertIn("the Hopf rings do not fit the core's cell centres", product)
        self.assertIn("the core keeps one set of Hopf rings; a different set is refused", product)
        look = text(ROOT / "tools" / "looklab" / "core" / "GlobalLayout.cs")
        self.assertIn("public const double RingRadius = 2.62, BaseRadius = 8.063, RingScale = 0.28;", look)

    def test_the_rings_fit_the_model_cells(self):
        # Each model cell's centre projects onto its ring's plane at the phase of its half-step, as the core checks.
        import struct
        import zipfile
        with zipfile.ZipFile(ROOT / "assets" / "model.npz") as archive:
            data = archive.read("normals.npy")
        header_length = struct.unpack("<H", data[8:10])[0]
        self.assertIn("'<f8'", data[10:10 + header_length].decode())
        values = struct.unpack("<2400d", data[10 + header_length:10 + header_length + 19200])
        rings = json.loads(text(ROOT / "tools" / "looklab" / "fixtures" / "hopf-rings.json"))
        j = rings["J"]

        def qmul(a, b):
            return [a[0] * b[0] - a[1] * b[1] - a[2] * b[2] - a[3] * b[3], a[0] * b[1] + a[1] * b[0] + a[2] * b[3] - a[3] * b[2],
                    a[0] * b[2] - a[1] * b[3] + a[2] * b[0] + a[3] * b[1], a[0] * b[3] + a[1] * b[2] - a[2] * b[1] + a[3] * b[0]]
        for cell in range(600):
            n = values[4 * cell:4 * cell + 4]
            length = math.sqrt(sum(x * x for x in n))
            origin = rings["rings"][rings["ringOf"][cell]]["origin"]
            along = sum(n[i] * origin[i] for i in range(4)) / length
            across = sum(n[i] * q for i, q in enumerate(qmul(origin, j))) / length
            off = ((math.degrees(math.atan2(across, along)) / 6 - rings["halfStep"][cell]) % 60 + 30) % 60 - 30
            self.assertLess(abs(off), 0.01, cell)
            self.assertGreater(math.hypot(along, across), 0.99, cell)

    def test_the_shell_opens_on_the_rings_only_with_d02(self):
        shell = text(PROJECT / "Shell.cs")
        self.assertIn("var layout = D02.ShellJumble.On ? D02.ShellHopf.Kind(D02.ShellShape.OpeningPath()) : D02.ShellHopf.Chain;",
                      shell)
        self.assertIn("_hopf = layout == D02.ShellHopf.Rings;", shell)
        # B22: the shell opens Global on either layout; the core builds the Hopf table for the sheet too.
        self.assertIn("if (layout != D02.ShellHopf.Chain && !D02.ShellHopf.Opens(layout))", shell)
        self.assertIn("public static bool Opens(uint kind) => kind == Rings || kind == Sheet;", text(PROJECT / "D02" / "ShellHopf.cs"))
        core = text(ROOT / "renderer" / "core" / "src" / "interop.cpp")
        self.assertIn("if (kind == SA2_LAYOUT_HOPF_RINGS || kind == SA2_LAYOUT_RING_SHEET)", core)
        self.assertIn('argument(shape.layout == SA2_LAYOUT_CHAIN || !hopf_.empty(), "a Hopf layout needs the Hopf table");',
                      text(ROOT / "renderer" / "core" / "src" / "product.cpp"))
        self.assertIn("uint chart = min(fixed != 0 ? fixed - 1 : asuint(hopfRows[2461 + e / 4][e % 4]), 599u);",
                      text(ROOT / "renderer" / "core" / "shaders" / "geometry.hlsl"))
        # The opening's layout, and B22 rest's palette chart rows, which switch Global's layout at a step.
        self.assertEqual(shell.count("SetViewLayout("), 2)
        self.assertIn("D02.ShellHopf.TakeRequest() is uint wanted && wanted != D02.ShellHopf.Drawn", shell)
        self.assertIn("var camera = _hopf ? SpinOf(_camera) : _camera.ColumnMajor();", shell)
        self.assertEqual(json.loads(text(ROOT / "tools" / "looklab" / "data" / "opening.json"))["params"]["projection"],
                         "hopfRings")
        hopf = text(PROJECT / "D02" / "ShellHopf.cs")
        self.assertIn('public const string Projection = "hopfRings";', hopf)
        # B22, bare: Look Lab's name for the ring sheet (Parameters.cs) is the opening file's other layout.
        self.assertIn('public const string SheetProjection = "ringSheet";', hopf)
        self.assertIn('"hopfRings" => ProjectionKind.HopfRings, "ringSheet" => ProjectionKind.RingSheet,',
                      text(ROOT / "tools" / "looklab" / "core" / "Parameters.cs"))
        self.assertIn("public const uint Chain = 0, Rings = 1, Sheet = 2;", hopf)
        self.assertIn("pub const LAYOUT_RING_SHEET: u32 = 2;", text(ROOT / "rust" / "c600-render" / "src" / "abi.rs"))

    def test_the_core_lays_the_sheet_out_as_look_lab(self):
        product = text(CORE / "src" / "product.cpp")
        self.assertIn("constexpr double SheetDx = 0.68, SheetDy = 0.57, SheetScale = 0.6;", product)
        self.assertIn("public const double SheetDx = 0.68, SheetDy = 0.57, SheetScale = 0.6;",
                      text(ROOT / "tools" / "looklab" / "core" / "GlobalLayout.cs"))
        self.assertIn("if (view.layout == SA2_LAYOUT_RING_SHEET) { c.viewFlags = 8u | 16u; return c; }",
                      text(CORE / "src" / "scene.cpp"))
        self.assertIn("float3 q = sheet ? placed : mul(Q, float4(placed, 0)).xyz;", text(CORE / "shaders" / "geometry.hlsl"))

    def test_the_build_stages_the_ring_file(self):
        build_shell = shell_module("build_shell")
        temp = fixture_folder(self, "d02-hopf-")
        project = temp / "staged"
        build_shell.stage(project)
        self.assertEqual((project / "look" / "hopf-rings.json").read_bytes(),
                         (ROOT / "tools" / "looklab" / "fixtures" / "hopf-rings.json").read_bytes())
        self.assertTrue((project / "D02" / "ShellHopf.cs").is_file())
        data = ROOT / "tools" / "looklab" / "data"
        for path in ("design/status-patterns.json", "samples/jumble/status-fixture.json"):
            self.assertEqual((project / path).read_bytes(), (data / path).read_bytes())


class LocalShape(unittest.TestCase):
    """Local's own shape: Look Lab's local-view.json, staged by the build and read by ShellLocal, never constants in
    code; only the cap's geometry is fixed, because it is the jumbled puzzle's."""

    def test_the_shape_file_is_the_owner_view(self):
        shape = json.loads(text(LOCAL_VIEW))
        self.assertEqual(shape["format"], "magic600-look-local-view")
        self.assertEqual(shape["projection"], "orthographic")
        self.assertEqual((shape["d4"], shape["gap"], shape["stickerShrink"], shape["margin"], shape["fill"], shape["spread"]),
                         (2, 0.3, 0.6, 0, 0.86, 1.7))

    def test_the_shell_reads_the_shape_from_the_file(self):
        local = text(PROJECT / "D02" / "ShellLocal.cs")
        self.assertIn('ProjectSettings.GlobalizePath("res://look/local-view.json")', local)
        self.assertIn('root.GetProperty("format").GetString() != "magic600-look-local-view"', local)
        for name in ("d4", "gap", "stickerShrink", "margin", "fill", "spread"):
            self.assertIn(f'Number("{name}")', local)
        self.assertNotRegex(local, r"\b(0\.3|0\.6|0\.86|1\.7)f?\b", "a local-view.json value is a constant in code")
        self.assertIn("new[] { 1 - shape.Gap, shape.StickerShrink, shape.D4, (float)Zoom(shape, aspect) };", local)

    def test_the_cap_threshold_is_the_jumbled_puzzles(self):
        local = text(PROJECT / "D02" / "ShellLocal.cs")
        self.assertIn("public const float FocusCos = 121f / 125f;", local)
        locality = text(ROOT / "rust" / "c600-jumble" / "src" / "locality.rs")
        self.assertRegex(locality, r"121")
        self.assertRegex(locality, r"125")

    def test_the_zoom_keeps_the_cap_inside_the_core_range(self):
        # The Look Lab formula ShellLocal ports, on the file's values: a zoom the core accepts (0 < zoom <= 64).
        shape = json.loads(text(LOCAL_VIEW))
        r = math.sqrt((96 - 32 * math.sqrt(5)) / (12 + 4 * math.sqrt(5)))
        self.assertAlmostEqual(r, 1.0803630, places=6)
        cap = min(math.acos((121 / 125) / r) + shape["margin"], 1.45)
        edge = r * math.sin(cap)
        radius = min(edge * shape["spread"] + .12, 4)
        for aspect in (0.5, 1, 2):
            zoom = shape["fill"] * math.sqrt(25 - radius * radius) / radius * min(1, aspect)
            self.assertTrue(0 < zoom <= 64, zoom)
        self.assertTrue(1 < shape["d4"])

    def test_the_build_stages_the_shape_file(self):
        build_shell = shell_module("build_shell")
        temp = fixture_folder(self, "d02-local-")
        project = temp / "staged"
        build_shell.stage(project)
        self.assertEqual((project / "look" / "local-view.json").read_bytes(), LOCAL_VIEW.read_bytes())
        self.assertTrue((project / "D02" / "ShellLocal.cs").is_file())


class ShellPanes(unittest.TestCase):
    """The shell opens Local only with --d02, follows the named cap, sets the pane table once the surface has its new
    rectangle, and resolves a point to the pane under it."""

    def setUp(self):
        self.shell = text(PROJECT / "Shell.cs")

    def test_local_opens_only_with_d02_and_hidden(self):
        self.assertEqual(self.shell.count("OpenLocal(hostLibrary);"), 1)
        self.assertIn("if (D02.ShellJumble.On)\n        {\n            OpenLocal(hostLibrary);", self.shell)
        body = self.shell[self.shell.index("private void OpenLocal(HostLibrary library)"):]
        body = body[:body.index("\n    }\n")]
        self.assertIn("library.OpenLocal(_host)", body)
        self.assertIn("library.SetViewVisible(_host, HostLibrary.Local, false)", body)
        self.assertIn("D02.ShellShape.Set(library, _host, HostLibrary.Local, D02.ShellLocal.ViewShape(shape, 1))", body)

    def test_local_follows_the_named_cap(self):
        body = self.shell[self.shell.index("private void FollowLocalCap(HostLibrary library)"):]
        body = body[:body.index("\n    }\n")]
        # B17 (ADR-D01.41): the named cap, or a newer selection while auto-framing is on.
        self.assertIn("int? cap = D02.ShellLocal.Frame(_chrome?.Jumble?.Naming.Named);", body)
        self.assertIn("cell == HostLibrary.FocusNone ? -1 : D02.ShellLocal.FocusCos", body)
        self.assertIn("library.SetViewVisible(_host, HostLibrary.Local, cell != HostLibrary.FocusNone && !hidden)", body)
        self.assertIn("chrome.Frame.LocalOpeningShown = cell != HostLibrary.FocusNone;", body)

    def test_the_panes_are_global_then_local_inside_the_surface(self):
        self.assertIn("var copies = D02.ShellSlice.Panes(layout.Panes[1]);", self.shell)
        self.assertIn('new Magic600.Chrome.PaneSpec("Global",', self.shell)
        self.assertIn('new Magic600.Chrome.PaneSpec("Local",', self.shell)
        # The panes are set after the surface's move and viewport, and only for the rectangle the surface has.
        process = self.shell[self.shell.index("FollowLocalCap(library);\n"):]
        self.assertLess(process.index('library.SetViewport(_host, (uint)size.X, (uint)size.Y)'),
                        process.index("SetPanes(library, global, layout);"))
        self.assertIn("if (_localOpen && area == _area)\n        {\n            SetPanes(library, global, layout);", self.shell)
        # Local's zoom follows its pane's aspect and keeps the wheel's factor.
        self.assertIn("D02.ShellShape.ApplyZoom(library, _host, HostLibrary.Local, D02.ShellJumble.Zoom.Factor(HostLibrary.Local))",
                      self.shell)

    def test_a_point_resolves_to_its_pane(self):
        self.assertIn('Magic600.Chrome.ViewPanes.Hit(layout, (int)point.X, (int)point.Y) is { View: "Local" }', self.shell)
        self.assertIn("return HostLibrary.Local;", self.shell)

    def test_local_turns_with_the_linked_spin(self):
        self.assertIn("var spin = SpinOf(turning);", self.shell)
        self.assertIn("library.SetViewCamera(_host, HostLibrary.Local, spin)", self.shell)

    def test_a_drag_in_local_turns_the_linked_rotation(self):
        # ADR-D01.40/.41: linked, a drag in either view turns the one camera; unlinked, Local turns its own.
        self.assertIn("return _camera.Follow(input, point => InOpening(point) || InLocal(point));", self.shell)
        self.assertIn("var local = D02.ShellLocal.Own.Follow(input, InLocal);", self.shell)
        local = text(PROJECT / "D02" / "ShellLocal.cs")
        self.assertIn("public static ShellCamera Camera(ShellCamera shared) => _linked ? shared : Own;", local)
        self.assertIn("private static bool _linked = true;", local)
        script = json.loads(text(SHELL / "d02_scripts" / "two-views.json"))
        self.assertIn({"drag": [160, 0], "in": "local"}, script["steps"])
        self.assertIn({"link": False}, script["steps"])

    def test_the_frame_row_names_composite_frames(self):
        log = text(PROJECT / "EventLog.cs")
        self.assertIn('if (frame.Panes > 0)\n            {\n                json.WriteNumber("panes", frame.Panes);\n'
                      '                json.WriteNumber("held", frame.Held);', log)


class TwistRowShell(unittest.TestCase):
    """B18's review (call 20261126T225307Z-e6ce1514) on the shell side of the twist row."""

    def setUp(self):
        self.library = text(PROJECT / "HostLibrary.cs")
        self.turn = text(PROJECT / "D02" / "ShellTurn.cs")

    def test_a_short_plane_is_refused_before_the_native_call(self):
        # B18-01: a plane that is not 8 floats (or a moving set without one) never reaches m6h_set_transform.
        body = self.library[self.library.index("public int SetTransform(IntPtr host, ulong revision, float[]? plane"):]
        body = body[:body.index("\n    }\n")]
        refuse = body.index("if (plane is { Length: not 8 } || (moving is not null && plane is null))")
        self.assertLess(refuse, body.index("return InvalidArgument;"))
        self.assertLess(body.index("return InvalidArgument;"), body.index('_exports["m6h_set_transform"]'))

    def test_the_row_carries_its_revision_to_the_setter(self):
        # B18-04: the query returns the row's revision and the preview names it.
        self.assertIn("&status, &at);", self.library)
        self.assertIn("new TwistRow(plane, angle, moving, at)", self.library)
        self.assertIn("library.SetTransform(host, row.Revision, row.Plane, -row.Angle, row.Moving, 0)", self.turn)

    def test_the_turn_reads_the_renderers_framing(self):
        # B18-05: one stateful framing call per frame, the shell's FollowLocalCap; the turn reads what it left.
        project = "".join(text(path) for path in sorted(PROJECT.rglob("*.cs")))
        self.assertEqual(project.count("ShellLocal.Frame("), 1)
        self.assertIn("if (ShellLocal.Plan.Framed is not int cap)", self.turn)

    def test_a_refusal_is_kept_for_its_cap_and_revision(self):
        # B18-06: no row query (and no 177,120-byte buffer) every frame while the host refuses.
        self.assertIn("(Cap, Row, RowRevision, _read) = (cap, row, at, (cap, revision));", self.turn)
        self.assertNotIn("_read = Code == HostLibrary.Ok", self.turn)


    def test_the_inside_set_is_bound_to_the_rows_revision(self):
        # B18-04's leftover: m6h_set_piece_style carries no revision, so the shell sends the row's moving set only
        # while the row's revision is the adopted one, and otherwise sends nothing and tries again at the next step.
        body = self.turn[self.turn.index("public static int SendInside(HostLibrary library, IntPtr host, ulong adopted"):]
        body = body[:body.index("\n    }\n")]
        self.assertLess(body.index("if (Row is { } row && row.Revision != adopted)"), body.index("library.SetPieceStyle("))
        self.assertIn("return HostLibrary.Unavailable;", body)
        self.assertNotIn("Row?.Moving", body)
        self.assertIn("r.Revision == adopted ? r.Moving : null", self.turn)
        slice_ = text(PROJECT / "D02" / "ShellSlice.cs")
        self.assertIn("ShellTurn.SendInside(library, host, revision, out var count);", slice_)
        self.assertIn("if (code == HostLibrary.Unavailable)", slice_)


class SweepShell(unittest.TestCase):
    """B18 rest: the timed sweep, the half-turn flash, reduced motion and the refused twist."""

    def setUp(self):
        self.turn = text(PROJECT / "D02" / "ShellTurn.cs")

    def test_the_twist_keys_are_the_routers(self):
        keymap = text(ROOT / "rust" / "c600-command" / "src" / "keys" / "keymap.rs")
        rust = re.search(r"pub const TWIST_KEYS: \[\(&str, u64, u64\); 3\] = \[(.*?)\];", keymap).group(1)
        shell = re.search(r"TwistKeys = \{ (.*?) \};", self.turn).group(1)
        self.assertEqual(re.findall(r'"(Key\w)", (\d+), (\d+)', rust), re.findall(r'"(Key\w)", (\d+), (\d+)', shell))
        self.assertIn("public const uint GripCap = 0;", self.turn)
        self.assertIn('specs.push(BindingSpec::new(\n            &[*key],\n            Global,\n            "grip.select"', keymap)

    def test_the_shell_arms_before_the_press_and_reports_its_answer(self):
        shell = text(PROJECT / "Shell.cs")
        self.assertIn("ShellMotion.TryQueue(input, SendTurn)", shell)
        # Brief SF2: the key is posted with its arming; the row and every visible change wait for its commit answer.
        arm = shell.index("arm = D02.ShellTurn.ArmFor(chord);")
        key_post = shell.index("library.KeyPost(_host, chord, key.Echo, arm is not null")
        armed = shell.index("D02.ShellTurn.Armed(answer.RowStatus == 1 ? library.KeyRow(_host, answer.Ticket, answer.RowRevision) : null);")
        committed = shell.index("D02.ShellTurn.Committed(answer.EngineRevision, posted.StaticMotion);")
        self.assertTrue(arm < key_post < armed < committed)
        self.assertIn("PollKeys(library);\n        FollowLocalCap(library);", shell)
        self.assertIn("D02.ShellTurn.Refused();", shell)
        self.assertIn("D02.ShellTurn.Framed(frame.EngineRevision);", shell)
        self.assertIn("if (row is null || revision != row.Revision + 1 || MotionRow is null)", self.turn)
        self.assertIn("engineRevision == motion.Revision", self.turn)
        self.assertIn("motion.FirstFrameRevision = engineRevision;", self.turn)

    def test_the_rows_and_reduced_motion_come_from_the_motion_table(self):
        chrome = text(PROJECT / "ShellChrome.cs")
        self.assertIn("D02.ShellTurn.MotionRow = id => (motions.Seconds(id, design, look.ReducedMotion), "
                      "design.Curves[motions[id].Curve]);", chrome)
        self.assertIn("half ? MotionUses.HalfTurn : MotionUses.JumbleSweep", self.turn)
        self.assertIn("MotionRow(MotionUses.QueuedSweep)", self.turn)
        self.assertIn('seconds <= 0 ? "static" : half ? "flash" : "sweep"', self.turn)
        motion = json.loads(text(CHROME.parent.parent / "data" / "design" / "motion.json"))
        rows = {row["id"]: row for row in motion["motions"]}
        self.assertEqual((rows["jumble.sweep"]["duration"], rows["jumble.half_turn"]["duration"]), ("twist", "flash"))

    def test_the_flash_draws_on_the_overlay_entry_and_the_sweep_on_the_row(self):
        style = text(PROJECT / "D02" / "ShellStyle.cs")
        self.assertIn("if (flash[piece] != 0) outline[piece] = JumbleOverlay.Entry;", style)
        self.assertLess(style.index("ShellTurn.Flash is { } flash"), style.index("outline[hover.Piece] = Hovered;"))
        self.assertIn("library.SetTransform(host, motion.Revision, row.Plane, row.Angle, row.Moving, phase)", self.turn)
        self.assertIn("var phase = motion.Timing.Phase(elapsed, motion.Curve);", self.turn)

    def run_dumps(self, kinds, refused, reduced_from=18, popped=False, rebuilt=False, grown=False):
        motions = []
        for revision, kind in enumerate(kinds, 3):
            motion = {"kind": kind, "revision": revision, "angle": math.pi if kind == "flash" else 1.2566, "moving": 1909,
                      "duration_ms": {"flash": 320, "sweep": 220}.get(kind, 0), "ended": "static", "popped": False,
                      "shown_ms": 0, "ms": [], "phases": []}
            if kind == "flash":
                motion.update(ended="done", shown_ms=336.4)
            if kind == "sweep":
                motion.update(ended="done", popped=popped, ms=[0, 16.7, 120.1, 233.3], phases=[0, 0.08, 0.6, 1])
            motions.append(motion)
        dos = ["start", "wait 3000 ms", "chord Space", "text > jumble.cap.name 1", "chord Enter", "wait 1500 ms",
               "click 0.3033 0.3227", "wait 600 ms", "click 0.6813 0.3719", "wait 600 ms", "click 0.5696 0.7217",
               "wait 800 ms", "chord J", "wait 900 ms", "chord K", "wait 900 ms", "chord Space",
               "text > motion.reduced.set on", "chord Enter", "wait 800 ms", "chord K", "wait 900 ms", "chord J",
               "wait 900 ms"]
        dumps = []
        for step, do in enumerate(dos):
            framed = 1 if step < 9 else 7
            dumps.append({"step": step, "do": do, "local_plan": {"framed": framed, "by_selection": step >= 9},
                          "turn": {"cap": framed}, "pick": {"selection": {"slot": 7 * 433 + 5} if step >= 9 else None},
                          "sweep": {"refused": refused, "reduced": step >= reduced_from, "running": None,
                                    "motions": motions if step == len(dos) - 1 else []},
                          "rebuilds": 41 + (rebuilt and step >= 13),
                          "global_opening": [946, 1093 + 23 * (grown and step == 13)]})
        return dumps

    def test_the_sweep_run_passes_and_the_failures_are_named(self):
        run = shell_module("d02_run")
        self.assertIs(run.EXPECT["sweep"], run.expect_sweep)
        self.assertNotIn("sweep", run.ON_FIXTURE)
        script = json.loads(text(SHELL / "d02_scripts" / "sweep.json"))
        self.assertEqual(script["steps"][1], {"chord": "Space"})
        self.assertEqual(run.expect_sweep(self.run_dumps(["flash", "sweep", "static", "static"], 0), {}, None), [])
        self.assertEqual(run.expect_sweep(self.run_dumps([], 4), {}, "wj-ia-300"), [])
        popped = run.expect_sweep(self.run_dumps(["flash", "sweep", "static", "static"], 0, popped=True), {}, None)
        self.assertEqual(len(popped), 1)
        self.assertIn("end pose", popped[0])
        swept = run.expect_sweep(self.run_dumps(["sweep"], 4), {}, "wj-ia-300")
        self.assertIn("not 4 refusals and none", swept[0])
        moved = run.expect_sweep(self.run_dumps(["flash", "sweep", "sweep", "static"], 0), {}, None)
        self.assertIn("not flash, sweep, static, static", moved[0])
        late = run.expect_sweep(self.run_dumps(["flash", "sweep", "static", "static"], 0, reduced_from=3), {}, None)
        self.assertIn("not off then on", late[0])
        # SF (27 Nov 2026): a twist that rebuilds the chrome frame is named.
        rebuilt = run.expect_sweep(self.run_dumps(["flash", "sweep", "static", "static"], 0, rebuilt=True), {}, None)
        self.assertEqual(len(rebuilt), 1)
        self.assertIn("rebuilt across the twists", rebuilt[0])
        # SF5 (27 Nov 2026): a twist that changes the Global opening's height (a border row going and coming back) is named.
        grown = run.expect_sweep(self.run_dumps(["flash", "sweep", "static", "static"], 0, grown=True), {}, None)
        self.assertEqual(len(grown), 1)
        self.assertIn("the Global opening was", grown[0])


class D01MotionShell(unittest.TestCase):
    def dumps(self, steps):
        def twist(revision, duration=0):
            return {"kind": "sweep" if duration else "static", "row_id": "twist.queued" if duration else "jumble.sweep",
                    "duration_ms": duration, "revision": revision, "armed_revision": revision - 1,
                    "first_frame_revision": revision if duration else None, "popped": False,
                    "ended": "done" if duration else "static"}

        animated = [twist(3, 110), twist(4, 91.333), twist(5, 76.4)]
        rewind = [twist(6, 110), twist(7, 91.333), twist(8, 76.4)]
        reduced = [twist(r) for r in (9, 10, 11)]
        zooms = {8: (1, 1.05, 1.1), 9: (1.05, 1.18, 1.1 ** 2), 10: (1.05, 1.1 ** 2, 1.1 ** 2)}
        dumps = []
        for i, do in enumerate(steps):
            turns = (animated[:0 if i < 4 else 1 if i < 7 else 3] + (rewind if i >= 14 else [])
                     + reduced[:max(0, i - 15)])
            start, drawn, target = zooms.get(i, (1, 1, 1))
            motion = {"queue_length": {4: 1, 5: 3, 6: 5}.get(i, 0),
                      "queued_events": 0 if i < 4 else {4: 1, 5: 3}.get(i, 5) if i < 11 else 10,
                      "state_revision": turns[-1]["revision"] if turns else 2,
                      "state_digest": "turned" if 7 <= i < 11 or i >= 18 else "solved",
                      "reduced": 15 <= i < 20, "turns": turns,
                      "zoom": {"row_id": "view.transition", "from": start, "drawn": drawn, "target": target,
                               "duration_ms": 260 if i in zooms else 0}}
            angle = 4.25 * .004 if i >= 3 else 0
            camera = [math.cos(angle), 0, 0, -math.sin(angle), 0, 1, 0, 0, 0, 0, 1, 0, math.sin(angle), 0, 0, math.cos(angle)]
            dumps.append({"do": do, "camera": camera, "motion": motion})
        return dumps

    def test_the_motion_script_checks_committed_revisions_and_retargeting(self):
        run = shell_module("d02_run")
        self.assertIs(run.EXPECT["motion"], run.expect_motion)
        self.assertNotIn("motion", run.ON_FIXTURE)
        script = json.loads(text(SHELL / "d02_scripts" / "motion.json"))
        self.assertNotIn("state", script)
        self.assertEqual(script["steps"][3:6], [{"chord": key, "quick": True} for key in ("K", "L", "K")])
        self.assertEqual(script["steps"][7:9], [{"wheel": 1, "quick": True}] * 2)
        self.assertEqual(run.expect_motion(self.dumps(run.MOTION_STEPS), {}, None), [])
        for step, field, value, finding in (
                (5, "queued_events", 1, "enter the twist queue"),
                (7, "state_revision", 4, "three new revisions"),
                (19, "state_digest", "different", "ended at different states"),
                (16, "queued_events", 11, "queued under reduced motion")):
            dumps = self.dumps(run.MOTION_STEPS)
            dumps[step]["motion"][field] = value
            self.assertTrue(any(finding in p for p in run.expect_motion(dumps, {}, None)))
        dumps = self.dumps(run.MOTION_STEPS)
        dumps[7]["motion"]["turns"][1]["first_frame_revision"] = 5
        self.assertTrue(any("committed revision" in p for p in run.expect_motion(dumps, {}, None)))
        dumps = self.dumps(run.MOTION_STEPS)
        dumps[9]["motion"]["zoom"]["from"] = 1.1
        self.assertTrue(any("second zoom notch" in p for p in run.expect_motion(dumps, {}, None)))

    def test_only_key_release_names_a_slice_twist(self):
        slice_shell = text(PROJECT / "D02" / "ShellSlice.cs")
        self.assertRegex(slice_shell, r"else if \(key.Pressed\)\s*\{\s*Model.Press\(cap, ShellTurn.Row is \{ \} row \? row.Angle : null\);"
                                      r"\s*\}\s*else\s*\{\s*Model.Release\(\);")
        self.assertNotIn("InputEventMouse", slice_shell)
        self.assertIn("Model.SetShell(max);", slice_shell)
        camera = text(PROJECT / "D02" / "ShellCamera.cs")
        self.assertIn("_pending += motion.Relative;", camera)
        self.assertIn("Turn(_pending.X * RadiansPerPixel, _pending.Y * RadiansPerPixel);", camera)
        self.assertNotIn("HostLibrary", camera)

    def test_raw_clip_steps_use_the_view_transition_at_draw_time(self):
        shell = text(PROJECT / "Shell.cs")
        motion = text(PROJECT / "D02" / "ShellMotion.cs")
        self.assertIn("StepRawClip(_localCap, _localRaw)", shell)
        self.assertIn("MotionRow?.Invoke(MotionUses.ViewTransition)", motion)
        self.assertIn("clip.Ease.Retarget(h, now, seconds, curve)", motion)
        self.assertIn("clip.Ease.Step(now, seconds <= 0)", motion)
        self.assertNotIn("ShellRaw.Model.SetClip", motion)


class ChromeOpening(unittest.TestCase):
    """The chrome opens the top of Local's slot (a square of its width, at most 60% of its height), scrolls Local's
    content below it, keeps the empty sentence while no cap is named, and cuts both openings out of its ground."""

    def test_the_opening_height(self):
        panes = text(CHROME / "Model" / "ViewPanes.cs")
        self.assertIn("public const double MaxOpeningShare = .6;", panes)
        self.assertIn("shown && width > 0 && height > 0 ? (int)Math.Floor(Math.Min(width, height * MaxOpeningShare)) : 0;", panes)

    def test_the_slot_and_the_ground(self):
        frame = text(CHROME / "Ui" / "ChromeFrame.cs")
        self.assertIn('Local = View("Local", localDrawn ? LocalWrap(localContent) : localContent);', frame)
        self.assertIn('var scroll = new ScrollContainer { Name = "LocalScroll", HorizontalScrollMode = ScrollContainer.ScrollMode.Disabled };',
                      frame)
        self.assertIn("int height = ViewPanes.OpeningHeight((int)Size.X, (int)Size.Y, frame.LocalOpen);", frame)
        self.assertIn("foreach (Rect2 open in new[] { last, lastLocal })", frame)
        self.assertIn("foreach (PaneRect piece in ViewPanes.Ground(all, holes))", frame)
        chrome = text(PROJECT / "ShellChrome.cs")
        self.assertIn("{ OpenGround = live, LocalDrawn = live && ShellJumble.On }", chrome)
        self.assertIn('Frame.Local.SetStatus(Frame.LocalDrawn ? null : "not drawn yet", Array.Empty<string>(), null);', chrome)
        self.assertIn("public Rect2I LocalOpening() => Pixels(Frame.LocalOpening);", chrome)
        program = text(ROOT / "tools" / "looklab" / "tests" / "Program.cs")
        self.assertIn('("D02GroundCutsBothOpenings", D02GroundCutsBothOpenings)', program)


class TwoViewsRun(unittest.TestCase):
    """The two-views run: Local is never empty (owner, 26 November 2026), so the dumps show the default cap 0 framed
    before the naming and cap 150 after it, and the event check reads composite frames of two drawn panes at every
    revision the run reached, with no one-view frame after the first composite one."""

    def check(self, rows):
        run = shell_module("d02_run")
        temp = fixture_folder(self, "d02-two-views-")
        events = temp / "events.jsonl"
        events.write_text("".join(json.dumps({"t": "frame", **row}) + "\n" for row in rows), encoding="utf-8")
        return run.check_two_views_events(events)

    def test_the_script_and_the_check_exist(self):
        run = shell_module("d02_run")
        self.assertIn("two-views", run.EXPECT)
        self.assertIn("two-views", run.ON_FIXTURE)
        self.assertIs(run.EVENT_CHECKS["two-views"], run.check_two_views_events)
        script = json.loads(text(SHELL / "d02_scripts" / "two-views.json"))
        self.assertIn({"text": "> jumble.cap.name 150"}, script["steps"])
        self.assertIn({"chord": "Ctrl+Z"}, script["steps"])
        written = [f"wait {step['wait_ms']} ms" if "wait_ms" in step else f"chord {step['chord']}" if "chord" in step
                   else f"text {step['text']}" for step in script["steps"][:len(run.TWO_VIEWS_STEPS) - 1]]
        self.assertEqual(["start"] + written, run.TWO_VIEWS_STEPS)

    def test_the_dumps_show_the_default_then_the_named_cap(self):
        run = shell_module("d02_run")

        def dumps(default=True, kept=150):
            out = []
            for index, do in enumerate(run.TWO_VIEWS_STEPS):
                named = index >= 4
                cap = 150 if named else 0 if default else -1
                words = "" if named or not default else run.LOCAL_DEFAULT
                lines = [["view:Local", words or run.LOCAL_EMPTY]] if not named else []
                local = {"cap": cap, "framed": kept if named else cap, "opening": cap >= 0, "words": words}
                out.append({"step": index, "do": do, "lines": lines, "named": 150 if named else -1, "local": local,
                            "state_revision": 300, "tint": {"revision": 300},
                            "overlay": {"trace_cap": 150 if named else None, "outline_entries": 0, "kept": 0}})
            return out
        self.assertEqual(run.expect_two_views(dumps(), {}, "wj-ia-300"), [])
        empty = run.expect_two_views(dumps(default=False), {}, "wj-ia-300")
        self.assertEqual(len(empty), 4, empty)
        self.assertTrue(empty[0].startswith("step 1: Local draws cap -1"), empty)
        self.assertEqual(len(run.expect_two_views(dumps(kept=0), {}, "wj-ia-300")), 7)
        wrong = dumps()
        wrong[5]["overlay"]["outline_entries"] = 1909
        wrong[7]["tint"]["revision"] = 299
        self.assertEqual(len(run.expect_two_views(wrong, {}, "wj-ia-300")), 2)
        self.assertTrue(run.expect_two_views(dumps()[:6], {}, "wj-ia-300"))

    def rotation(self, linked, turns, spin, sent_global, sent_local):
        return {"linked": linked, "auto_frame": True, "global": {"turns": turns, "q": [0] * 16, "spin": spin},
                "local": {"turns": 0, "q": [0] * 16, "spin": spin}, "sent_global": sent_global,
                "sent_local": sent_local}

    def drag_dumps(self, unlinked_turns_global=False):
        a, b, c, d, e = ([float(i)] * 16 for i in range(5))
        rows = [("wait 4000 ms", self.rotation(True, 0, a, a, a)), ("drag 160 0 in local", None),
                ("wait 701 ms", self.rotation(True, 10, b, b, b)), ("drag 0 100", None),
                ("wait 702 ms", self.rotation(True, 20, c, c, c)), ("link off", None), ("drag -120 0 in local", None),
                ("wait 703 ms", self.rotation(False, 20, c, d if unlinked_turns_global else c, d)),
                ("drag 0 -80", None), ("wait 704 ms", self.rotation(False, 30, e, e, d)), ("link on", None),
                ("wait 705 ms", self.rotation(True, 30, e, e, e))]
        dumps = []
        for step, rotation in rows:
            dumps.append({"do": step, "camera_turns": rotation["global"]["turns"] if rotation else 0,
                          **({"rotation": rotation} if rotation else {})})
        return dumps

    def test_the_drag_check_reads_both_views(self):
        run = shell_module("d02_run")
        self.assertEqual(run.check_local_drag(self.drag_dumps()), [])
        leak = run.check_local_drag(self.drag_dumps(unlinked_turns_global=True))
        self.assertTrue(any("unlinked drag in Local turned Global" in p for p in leak), leak)
        self.assertTrue(any("steps written" in p for p in run.check_local_drag(self.drag_dumps()[:3])))

    def test_a_good_run_passes(self):
        rows = [{"frame_id": 1, "engine_revision": 302}, {"frame_id": 2, "engine_revision": 302, "panes": 2, "held": 0},
                {"frame_id": 3, "engine_revision": 303, "panes": 2, "held": 1},
                {"frame_id": 4, "engine_revision": 303, "panes": 2, "held": 0}]
        self.assertEqual(self.check(rows), [])

    def test_the_failures_are_named(self):
        self.assertEqual(self.check([{"frame_id": 1, "engine_revision": 302, "panes": 2, "held": 0},
                                     {"frame_id": 2, "engine_revision": 303, "panes": 2, "held": 0}]), [])
        emptied = self.check([{"frame_id": 1, "engine_revision": 302, "panes": 2, "held": 0},
                              {"frame_id": 2, "engine_revision": 303},
                              {"frame_id": 3, "engine_revision": 303, "panes": 2, "held": 0}])
        self.assertEqual(emptied, ["1 one-view frames after Local was drawn (the first: frame 2)"])
        undrawn = self.check([{"frame_id": 1, "engine_revision": 302}, {"frame_id": 2, "engine_revision": 302, "panes": 2, "held": 0},
                              {"frame_id": 3, "engine_revision": 303, "panes": 2, "held": 1}])
        self.assertTrue(any("[303] were never drawn" in p for p in undrawn), undrawn)
        still = self.check([{"frame_id": 1, "engine_revision": 302}, {"frame_id": 2, "engine_revision": 302, "panes": 2, "held": 0}])
        self.assertTrue(any("no revision changed" in p for p in still), still)
        one = self.check([{"frame_id": 1, "engine_revision": 302}, {"frame_id": 2, "engine_revision": 303}])
        self.assertTrue(any("no composite frame of two drawn panes" in p for p in one), one)


class Picks(unittest.TestCase):
    """B17: the posed row's slots are pickable as their home slot + 1; the host resolves a hit to its piece through the
    pose projection and drops a hit whose frame drew a revision the head left; the shell keeps one selection and one
    hover for both views and outlines them through B14's style path."""

    def test_the_posed_row_stamps_each_home_slot(self):
        backend = text(ROOT / "rust" / "c600-render" / "src" / "backend.rs")
        self.assertIn("rows.instances.iter_mut().zip(1u32..).for_each(|(instance, id)| *instance = id);", backend)
        self.assertIn('assert!(instances.iter().all(|(&slot, &id)| id as usize == slot + 1), "each as its home slot + 1");',
                      text(ROOT / "rust" / "c600-render" / "tests" / "bridge.rs"))

    def test_the_host_resolves_through_the_pose_projection(self):
        lib = text(ROOT / "renderer" / "bridge" / "host" / "src" / "lib.rs")
        self.assertIn("let projection = jumble::PoseProjection::new(&exact, &state)", lib)
        self.assertIn("if row.state == c600_render::abi::PICK_STALE || revision != shell.assigned {", lib)
        self.assertIn("const _: () = assert!(std::mem::size_of::<M6hPick>() == 40);", lib)
        turns = text(ROOT / "renderer" / "bridge" / "host" / "tests" / "turns.rs")
        for name in ("fn a_jumble_pick_resolves_to_its_piece()", "fn a_stale_jumble_pick_is_dropped()"):
            self.assertIn(name, turns)

    def test_the_pick_row_is_the_same_on_both_sides(self):
        lib = text(ROOT / "renderer" / "bridge" / "host" / "src" / "lib.rs")
        rust = re.search(r"pub struct M6hPick \{(.*?)\n\}", lib, re.S).group(1)
        fields = re.findall(r"pub (\w+): (u32|u64),", rust)
        host = text(PROJECT / "HostLibrary.cs")
        cs = re.search(r"public struct Pick\n    \{(.*?)\n    \}", host, re.S).group(1)
        cs_fields = re.findall(r"public (uint|ulong) (\w+);", cs)
        self.assertEqual([(n.replace("_", ""), t) for n, t in fields],
                         [(n.lower(), {"uint": "u32", "ulong": "u64"}[t]) for t, n in cs_fields])

    def test_the_shell_picks_on_a_click_that_did_not_turn(self):
        shell = text(PROJECT / "Shell.cs")
        self.assertIn("case InputEventMouseButton { ButtonIndex: MouseButton.Left, Pressed: false } up when !turned:", shell)
        self.assertIn("D02.ShellPick.Point(SurfacePoint(move.Position));", shell)
        self.assertIn("if (D02.ShellJumble.On && _session && D02.ShellPick.Step(library, _host))", shell)
        self.assertIn("D02.ShellStyle.Apply(library, _host, _adoptedRevision, _boundRevision);", shell)
        # Global's pane draws with style slot 1 and Local's with slot 2 (B20: 3 for the split's inside copy), so both
        # views show the outlines.
        self.assertIn("g.X, g.Y, (uint)g.Width, (uint)g.Height, 1));", shell)
        self.assertIn("r.X, r.Y, (uint)r.Width, (uint)r.Height, slot));", shell)
        pick = text(PROJECT / "D02" / "ShellPick.cs")
        self.assertIn("public static Picked? Selection { get; private set; }", pick)
        self.assertIn("public static Picked? Hover { get; private set; }", pick)
        style = text(PROJECT / "D02" / "ShellStyle.cs")
        self.assertIn("static ShellStyle() => ShellPick.Changed += Invalidate;", style)
        self.assertIn("public const int PieceCount = 177_120;", style)
        self.assertIn("#define SA2_PIECES 177120u", text(CORE / "include" / "sa2_interop.h"))


class FocusLayersInTheCore(unittest.TestCase):
    """B17, ADR-D01.46 (added to the core's ABI 10 before any release): the pieces a focus layer leaves out are dimmed
    or hidden by the style slots, and the ID pass follows the style the frame drew with, so neither is picked."""

    def test_the_core_hides_and_picks_follow_visibility(self):
        header = text(CORE / "include" / "sa2_interop.h")
        self.assertIn("#define SA2_STYLE_HIDE_MARKED 3u", header)
        self.assertIn("#define SA2_STYLE_HIDE_UNMARKED 4u", header)
        self.assertIn("Picking follows visibility", header)
        styled = text(CORE / "shaders" / "styled.hlsl")
        self.assertIn("return !statusConflict(style) && ((dimMode == 3 && outside) || (dimMode == 4 && !outside));", styled)
        self.assertIn("if (hides(style) && (style & ReferenceLine) == 0) result.position = float4(0, 0, -1, 1);", styled)
        self.assertIn("if (referenceOnly && edge >= outlineWidth) discard;", styled)
        self.assertIn("uint styledIdPs(StyledOut input) : SV_Target {", styled)
        self.assertIn("if (hides(input.style)) discard;", styled)
        scene = text(CORE / "src" / "scene.cpp")
        self.assertIn("auto* pipeline = input.styled ? g.styled_id_pipeline() : p.id_pso.Get();", scene)
        self.assertIn("compile_embedded_shader(styled_id_ps.dxil styled.hlsl styledIdPs ps_6_0)", text(CORE / "CMakeLists.txt"))
        self.assertIn("foreach(name styled_vs styled_ps styled_id_ps)", text(CORE / "embed_dxil.cmake"))
        product = text(CORE / "src" / "product.cpp")
        # B18-03: a turning frame's moving mask lives in the same style words, so its pick checks the version too.
        self.assertIn("const bool bound = record->styled || record->shape.turn != 0.0f;", product)
        self.assertIn("(bound && copy.pick_style != record->style_version)", product)
        self.assertIn('suite.check("focus layers warp", [&] { product_focus_layers(options); });', text(CORE / "src" / "gpu_test.cpp"))
        abi = text(ROOT / "rust" / "c600-render" / "src" / "abi.rs")
        self.assertIn("pub const STYLE_HIDE_MARKED: u32 = 3;", abi)
        self.assertIn("&& self.dim_mode <= STYLE_HIDE_UNMARKED", abi)

    def test_the_shell_composes_the_focus_through_the_chrome_model(self):
        style = text(PROJECT / "D02" / "ShellStyle.cs")
        self.assertIn("var mode = JumbleStyle.FocusMode(layers, member);", style)
        self.assertIn("outside[piece] = !member!(piece) && !conflict(piece);", style)
        self.assertIn("View(slot, slot == 1 ? global.Mode : local.Mode,", style)
        self.assertIn("style.DimMode = mode == InactiveMode.Hidden ? HostLibrary.HideMarked : HostLibrary.DimMarked;", style)
        self.assertIn("public const uint HideMarked = 3, HideUnmarked = 4;", text(PROJECT / "HostLibrary.cs"))
        # ShownPiece's quarter is the core's dim amount.
        self.assertIn("public const float DimAmount = 0.75f;", style)
        self.assertIn("Inactive == InactiveMode.Dimmed ? .25 : 0", text(ROOT / "tools" / "looklab" / "app" / "Chrome" / "Model" / "Gallery.cs"))


class LocalReframes(unittest.TestCase):
    """B17, ADR-D01.41: Local re-frames on a selection change while auto-framing is on, else follows the named cap."""

    def test_local_frames_the_newer_source(self):
        local = text(PROJECT / "D02" / "ShellLocal.cs")
        # Step C-.40: the rule is the shared library's LocalPlan, for both puzzles.
        self.assertIn("public static bool AutoFrame { get => Plan.AutoFrame; set => Plan.AutoFrame = value; }", local)
        self.assertIn("public static int? Frame(int? namedCap) => Plan.Follow(namedCap, ShellJumble.Host?.LocalCap.Shown, ShellPick.Selection?.Cell);",
                      local)
        plan = text(CHROME / "Model" / "LocalPlan.cs")
        self.assertIn("public bool AutoFrame { get; set; } = true;", plan)
        self.assertIn("BySelection = AutoFrame && selectionNewer && selected != null;", plan)
        self.assertIn("Framed = BySelection ? selected : Cap;", plan)
        self.assertIn("int? cap = D02.ShellLocal.Frame(_chrome?.Jumble?.Naming.Named);", text(PROJECT / "Shell.cs"))
        self.assertIn("public int? Cell => Slot is uint slot ? (int)(slot / 433) : null;", text(PROJECT / "D02" / "ShellPick.cs"))


class LocalOnTheBasePuzzle(unittest.TestCase):
    """Step C-.40 (ADR-D01.40): Local draws on the base puzzle (Jumble off) with no D02 state, through the D02 path."""

    def test_the_plan_knows_the_puzzle_and_draws_cell_zero(self):
        local = text(PROJECT / "D02" / "ShellLocal.cs")
        self.assertIn('new(System.Environment.GetEnvironmentVariable("M600_PUZZLE") == EventLog.Puzzles[1]);', local)
        plan = text(CHROME / "Model" / "LocalPlan.cs")
        self.assertIn("Cap = Jumble ? namedCap ?? hostCap : Default;", plan)
        self.assertIn("public const int Default = LocalCap.Default;", plan)
        program = text(ROOT / "tools" / "looklab" / "tests" / "Program.cs")
        self.assertIn('("D02LocalPlanDrawsCellZeroOnTheBasePuzzle", D02LocalPlanDrawsCellZeroOnTheBasePuzzle)', program)
        self.assertIn('("D02LocalPlanKeepsTheJumbleRule", D02LocalPlanKeepsTheJumbleRule)', program)

    def test_the_jumble_sentence_hides_and_the_readout_shows(self):
        chrome = text(PROJECT / "ShellChrome.cs")
        self.assertIn("host.SetDrawnWithoutCap(live && !ShellLocal.Plan.Jumble);", chrome)
        self.assertIn('ViewLine.Set(Frame.Local, "LocalPlanLine", words, LocalPlan.Hint, Frame.Tokens.C("text.dim"));', chrome)
        # ADR-D02 B21: the raw view's line takes Local's line while the raw view is on.
        self.assertIn("_chrome?.LocalLine(hidden ? HiddenLocalWords : D02.ShellRaw.Line(cap) ?? D02.ShellLocal.Plan.Words);", text(PROJECT / "Shell.cs"))
        host = text(CHROME / "Ui" / "JumbleHost.cs")
        self.assertIn("Surface.SetDrawn(LocalCap.Shown != null || drawnWithoutCap);", host)
        self.assertIn('ViewLine.Set(view, "JumbleLocalDefault", LocalWords, hint, frame.Tokens.C("text.dim"));', host)

    def test_the_base_local_run(self):
        d02_run = shell_module("d02_run")
        script = json.loads(text(ROOT / "renderer" / "shell" / "d02_scripts" / "base-local.json"))
        self.assertEqual(script["puzzle"], "jumble-off")
        self.assertIn("base-local", d02_run.EXPECT)
        self.assertNotIn("base-local", d02_run.ON_FIXTURE)
        self.assertNotIn("base-local", d02_run.IN_SAMPLE_MODE)

    def plan(self, framed, by_selection=False, cap=0):
        words = (f"Local frames cell {framed}, the selected sticker's cell. Clear the selection to return to cell 0."
                 if by_selection else f"Local shows cell {framed}'s turning region by default. Select a sticker to frame its cell.")
        return {"jumble": False, "cap": cap, "framed": framed, "by_selection": by_selection, "words": words}

    def dump(self, do, framed, slot=None, by_selection=False, opening=True, lines=None):
        selection = None if slot is None else {"piece": 1, "sticker": slot, "slot": slot, "lattice": True}
        plan = self.plan(framed, by_selection)
        return {"do": do, "pick": {"selection": selection}, "local_plan": plan, "local": {"opening": opening},
                "lines": lines if lines is not None else [["view:Local", plan["words"]]]}

    def test_the_base_local_check(self):
        d02_run = shell_module("d02_run")
        steps = d02_run.BASE_LOCAL_STEPS
        # The first two clicks hit a sticker of cell 17, the third misses, the corner click clears.
        hit = 433 * 17 + 5
        dumps = [self.dump(do, 17, slot=hit, by_selection=True) if i in (3, 4, 5, 6) else self.dump(do, 0)
                 for i, do in enumerate(steps)]
        self.assertEqual(d02_run.base_local_problems(dumps), [])
        # A run whose clicks all miss (the host picks only in Jumble) keeps Local on cell 0 and passes.
        self.assertEqual(d02_run.base_local_problems([self.dump(do, 0) for do in steps]), [])
        # Local hidden, a Jumble plan, or a selection that did not re-frame are each found.
        hidden = [dict(d, local={"opening": False}) if i == 1 else d for i, d in enumerate(dumps)]
        self.assertTrue(any("opening" in p for p in d02_run.base_local_problems(hidden)))
        jumble = [dict(d, local_plan=dict(d["local_plan"], jumble=True)) if i == 1 else d for i, d in enumerate(dumps)]
        self.assertTrue(any("Jumble" in p for p in d02_run.base_local_problems(jumble)))
        stuck = [self.dump(d["do"], 0, slot=d["pick"]["selection"]["slot"]) if d["pick"]["selection"] else d for d in dumps]
        self.assertTrue(any("selected" in p for p in d02_run.base_local_problems(stuck)))
        unworded = [dict(d, lines=[]) if i == 1 else d for i, d in enumerate(dumps)]
        self.assertTrue(any("readout" in p for p in d02_run.base_local_problems(unworded)))


class PickRun(unittest.TestCase):
    """The pick run's check reads each click's answer from the dump after its wait."""

    def dumps(self, answers, asked=None, stale=0):
        dumps = [{"step": 0, "do": "start", "pick": {"selection": None, "hover": None, "asked": 0, "hits": 0,
                                                       "misses": 0, "stale": 0}}]
        hits = misses = 0
        for answer in answers:
            hits += answer is not None
            misses += answer is None
            selection = None if answer is None else {"piece": answer[1], "sticker": answer[0], "slot": answer[0],
                                                     "lattice": True, "view": 1, "revision": 302}
            pick = {"selection": selection, "hover": None, "asked": hits + misses if asked is None else asked,
                    "hits": hits, "misses": misses, "stale": stale}
            dumps += [{"step": len(dumps), "do": "click 0.5 0.5", "pick": dumps[-1]["pick"]},
                      {"step": len(dumps) + 1, "do": "wait 600 ms", "pick": pick}]
        return dumps

    def test_the_script_and_the_check_exist(self):
        run = shell_module("d02_run")
        self.assertIs(run.EXPECT["pick"], run.expect_pick)
        self.assertIn("pick", run.ON_FIXTURE)
        script = json.loads(text(SHELL / "d02_scripts" / "pick.json"))
        self.assertEqual(script["steps"][-2], {"click": [0.02, 0.02]})
        self.assertIn("public static string ScriptClick(Rect2 opening, JsonElement click, bool local = false)",
                      text(PROJECT / "D02" / "ShellPick.cs"))

    def test_the_model_pieces_are_read(self):
        pieces = shell_module("d02_run").slot_pieces()
        self.assertEqual(len(pieces), 259800)
        self.assertEqual(max(pieces), 177119)

    def test_a_good_run_passes_and_the_failures_are_named(self):
        run = shell_module("d02_run")
        pieces = [s // 2 for s in range(10)]
        self.assertEqual(run.expect_pick(self.dumps([(4, 2), None]), {}, None, pieces), [])
        wrong = run.expect_pick(self.dumps([(4, 3), None]), {}, None, pieces)
        self.assertTrue(any("not the model's 2" in p for p in wrong), wrong)
        kept = run.expect_pick(self.dumps([None, (4, 2)]), {}, None, pieces)
        self.assertTrue(any("did not clear" in p for p in kept), kept)
        none = run.expect_pick(self.dumps([None, None]), {}, None, pieces)
        self.assertTrue(any("no click hit" in p for p in none), none)
        stale = run.expect_pick(self.dumps([(4, 2), None], stale=1), {}, None, pieces)
        self.assertTrue(any("stale 1" in p for p in stale), stale)


class HopfRun(unittest.TestCase):
    """The hopf run: the zoom sent is the opening file's Hopf zoom, and Local draws the default cap: composite frames,
    and no one-view frame after the first of them (owner, 26 November 2026: Local is never empty)."""

    def dumps(self, base, after):
        steps = ["start", "wait 2500 ms", "wheel 2", "wait 1000 ms"]
        zooms = [{"notches": 0, "factor": 1}, {"notches": 0, "factor": 1, "sent": base},
                 {"notches": 2, "factor": 1.21, "sent": after}, {"notches": 2, "factor": 1.21, "sent": after}]
        return [{"step": i, "do": do, "zoom": zoom} for i, (do, zoom) in enumerate(zip(steps, zooms))]

    def test_the_script_and_the_checks_exist(self):
        run = shell_module("d02_run")
        self.assertIs(run.EXPECT["hopf"].__wrapped__, run.expect_hopf)  # C3/C4 F5: under the steady-state check
        self.assertIn("hopf", run.ON_FIXTURE)
        self.assertIs(run.EVENT_CHECKS["hopf"], run.check_hopf_events)
        script = json.loads(text(SHELL / "d02_scripts" / "hopf.json"))
        self.assertEqual(script["steps"], [{"wait_ms": 2500}, {"wheel": 2}, {"wait_ms": 1000}])

    def test_the_zoom_is_the_opening_files(self):
        run = shell_module("d02_run")
        fov = json.loads(text(ROOT / "tools" / "looklab" / "data" / "opening.json"))["params"]["fieldOfView"]
        base = 1 / math.tan(fov / 2)
        self.assertEqual(run.expect_hopf(self.dumps(base, base * 1.21), {}, None), [])
        chain = run.expect_hopf(self.dumps(1.15, 1.15 * 1.21), {}, None)
        self.assertTrue(any("not the Hopf rings'" in p for p in chain), chain)

    def test_the_ring_sheet_run_reads_its_own_opening(self):
        run = shell_module("d02_run")
        self.assertIs(run.EXPECT["ring-sheet"].__wrapped__, run.expect_ring_sheet)  # C3/C4 F5
        self.assertIn("ring-sheet", run.ON_FIXTURE)
        self.assertIs(run.DATA_FOR["ring-sheet"], run.ring_sheet_data)
        temp = fixture_folder(self, "d02-ring-sheet-")
        folder = run.ring_sheet_data(temp / "run")
        self.assertEqual(folder, temp / "run-data")
        opening = json.loads(text(folder / "opening.json"))
        self.assertEqual(opening["params"]["projection"], "ringSheet")
        self.assertEqual(opening["params"]["fieldOfView"],
                         json.loads(text(ROOT / "tools" / "looklab" / "data" / "opening.json"))["params"]["fieldOfView"])
        self.assertTrue((folder / "samples" / "jumble" / "states.json").is_file())
        with self.assertRaises(ValueError):
            run.ring_sheet_data(temp / "run")
        fov = opening["params"]["fieldOfView"]
        base = 1 / math.tan(fov / 2)
        self.assertEqual(run.expect_hopf(self.dumps(base, base * 1.21), {}, None, projection="ringSheet"), [])

    def test_the_ring_sheet_run_reads_its_marks_clicks_and_chart_rows(self):
        run = shell_module("d02_run")
        sheet = [["view:Global", words] for words in run.SHEET_MARKS]
        chain = [["view:Global", run.CHAIN_MARK]]
        hit = {"selection": {"sticker": 7, "piece": 3, "slot": 7}}

        def dumps(first=sheet, back=chain, again=sheet, pick=hit):
            rows = [("wait 1000 ms", first, None), ("click 0.507 0.45", first, None), ("wait 600 ms", first, pick),
                    ("command view.preset.global.chain", first, None), ("wait 1000 ms", back, None),
                    ("command view.preset.global.chart", back, None), ("wait 1000 ms", again, None)]
            return [{"do": do, "global_marks": [words for _, words in lines], "pick": p or {"selection": None}}
                    for do, lines, p in rows]
        pieces = [0] * 7 + [3]
        self.assertEqual(run.expect_sheet_chart(dumps(), pieces), [])
        self.assertEqual(len(run.expect_sheet_chart(dumps(first=chain), pieces)), 1)
        self.assertEqual(len(run.expect_sheet_chart(dumps(back=sheet), pieces)), 1)
        self.assertEqual(len(run.expect_sheet_chart(dumps(again=chain), pieces)), 1)
        self.assertEqual(len(run.expect_sheet_chart(dumps(pick={"selection": None}), pieces)), 1)
        self.assertEqual(len(run.expect_sheet_chart(dumps(), [0] * 8)), 1)

    def test_under_a_chart_global_draws_no_sweep(self):
        core = text(CORE / "shaders" / "geometry.hlsl")
        self.assertIn("const bool turns = theta != 0 && (viewFlags & 8u) == 0 && (animated[slotPieces[slot]] & 0x10000u) != 0;", core)
        self.assertIn('"the ring sheet drew a sweep"', text(CORE / "src" / "gpu_test.cpp"))
        turn = text(ROOT / "renderer" / "shell" / "project" / "D02" / "ShellTurn.cs")
        self.assertIn('if (kind == "sweep" && ShellHopf.Drawn != ShellHopf.Chain)', turn)
        self.assertIn("if (Cap != (int)ActiveGripCap)", turn)
        style = text(ROOT / "renderer" / "shell" / "project" / "D02" / "ShellStyle.cs")
        self.assertIn("if (slot != 1 && ShellTurn.OutlineOnly)", style)

    def test_local_draws_the_default_cap(self):
        run = shell_module("d02_run")
        temp = fixture_folder(self, "d02-hopf-run-")
        events = temp / "events.jsonl"

        def check(rows):
            events.write_text("".join(json.dumps(row) + "\n" for row in rows), encoding="utf-8")
            return run.check_hopf_events(events)
        rows = [{"t": "frame", "frame_id": i, "engine_revision": 302} for i in range(1, 31)]
        for row in rows[2:]:
            row["panes"], row["held"] = 2, 0
        self.assertEqual(check(rows), [])
        never = check([{"t": "frame", "frame_id": i, "engine_revision": 302} for i in range(1, 31)])
        self.assertEqual(never, ["no composite frame: Local never drew the default cap"])
        rows[10] = {"t": "frame", "frame_id": 11, "engine_revision": 302}
        problems = check(rows[:20])
        self.assertTrue(any("fewer than 30" in p for p in problems) and any("one-view frames after" in p for p in problems),
                        problems)


class TintRun(unittest.TestCase):
    """B15: the tint run's check recomputes the bytes the shell sent to the core from the sample, piece by piece."""

    def test_the_script_the_check_and_the_shell_side_exist(self):
        run = shell_module("d02_run")
        self.assertIs(run.EXPECT["tint"], run.expect_tint)
        self.assertIn("tint", run.ON_FIXTURE)
        script = json.loads(text(SHELL / "d02_scripts" / "tint.json"))
        waits = [f"wait {step['wait_ms']} ms" for step in script["steps"] if "wait_ms" in step]
        self.assertEqual(waits, [mark for mark, _, _ in run.TINT_STEPS])
        self.assertIn("ShellTint.Attach(host, frame);", text(PROJECT / "ShellChrome.cs"))
        self.assertIn("ShellTint.Dump(json, _frame);", text(PROJECT / "D02" / "ShellJumble.cs"))
        style = text(PROJECT / "D02" / "ShellStyle.cs")
        self.assertIn("var outline = tint is null ? new byte[PieceCount] : (byte[])tint.Clone();", style)
        self.assertIn("JumbleStyle.Encode(outline[piece], height, heights != null, ink,", style)

    def test_a_good_run_passes_and_a_wrong_byte_is_named(self):
        run = shell_module("d02_run")
        tables = run.tint_tables("wj-ia-300")
        self.assertEqual(len(tables["ids"]), 177120)
        self.assertEqual(sorted(set(tables["classes"]) - {0}), [1, 2, 3, 4])
        dumps = [{"do": mark, "tint": run.tint_expected(tables, mode, focused, None, 7 if mode == "height" else None)}
                 for mark, mode, focused in run.TINT_STEPS]
        self.assertEqual(run.expect_tint(dumps, {}, "wj-ia-300", tables), [])
        self.assertTrue(dumps[0]["tint"]["line"].startswith("Contact classes · 1 "), dumps[0]["tint"]["line"])
        self.assertEqual(dumps[4]["tint"]["dims"], [True, True])
        dumps[2]["tint"]["outline_sha256"] = dumps[3]["tint"]["outline_sha256"]
        problems = run.expect_tint(dumps, {}, "wj-ia-300", tables)
        self.assertEqual(len(problems), 1)
        self.assertIn("after 'wait 812 ms' (off-lattice) outline_sha256", problems[0])
        dumps[5]["tint"]["fill_sha256"] = run.tint_expected(tables, "height", None, None, 7)["fill_sha256"]
        self.assertTrue(any("domain 5 focused) fill_sha256" in p for p in run.expect_tint(dumps, {}, "wj-ia-300", tables)))


class OverlaysRun(unittest.TestCase):
    """B16: the overlays run's check recomputes the overlay's counts and the bytes sent to the core from the sample."""

    def test_the_script_the_check_and_the_shell_side_exist(self):
        run = shell_module("d02_run")
        self.assertIs(run.EXPECT["overlays"], run.expect_overlays)
        self.assertIn("overlays", run.ON_FIXTURE)
        script = json.loads(text(SHELL / "d02_scripts" / "overlays.json"))
        waits = [f"wait {step['wait_ms']} ms" for step in script["steps"] if "wait_ms" in step and step["wait_ms"] > 1000]
        self.assertEqual(waits, [mark for mark, _, _, _ in run.OVERLAY_STEPS])
        tint = text(PROJECT / "D02" / "ShellTint.cs")
        self.assertIn("host.OverlayChanged += Draw;", tint)
        self.assertIn("ShellStyle.SetFocusViews(global, local, host.FocusMember, ShellStatus.Conflict);", tint)
        style = text(PROJECT / "D02" / "ShellStyle.cs")
        self.assertIn("var preview = ShellPreview.MovingAt(revision);", style)
        shader = text(CORE / "shaders" / "styled.hlsl")
        self.assertIn("result.cutDistance = dot(world, pole) - 121.0 / 125.0;", shader)
        self.assertIn("abs(input.cutDistance) <= cutWidth", shader)

    def test_a_good_run_passes_and_a_missing_reference_outline_is_named(self):
        run = shell_module("d02_run")
        tables = run.tint_tables("wj-ia-300")
        dumps = []
        for mark, cap, step, focus in run.OVERLAY_STEPS:
            want = run.overlay_expected(tables, "wj-ia-300", cap, step, focus, None, None)
            line = run.overlay_words(want)
            overlay = {k: v for k, v in want.items() if k not in ("outline_sha256", "fill_sha256", "modes")}
            overlay.update(line=line, global_line=line, local_line=run.overlay_words(want, local=True))
            dumps.append({"do": mark, "overlay": overlay, "tint": {"hover": None, "selection": None, "modes": want["modes"],
                                                                   "outline_sha256": want["outline_sha256"],
                                                                   "fill_sha256": want["fill_sha256"]}})
        self.assertEqual(run.expect_overlays(dumps, {}, "wj-ia-300", tables), [])
        self.assertEqual(dumps[0]["overlay"]["line"], "")
        self.assertEqual(dumps[1]["overlay"]["line"], "Cut of cap 150 · surface trace")
        self.assertEqual(dumps[3]["overlay"]["hidden"], 0)
        self.assertEqual(dumps[3]["tint"]["modes"], ["hidden", "dimmed"])
        self.assertGreater(dumps[4]["overlay"]["hidden"], 0)
        self.assertTrue(dumps[4]["overlay"]["line"].startswith("Repair 2 of "), dumps[4]["overlay"]["line"])
        self.assertEqual((dumps[5]["overlay"]["pieces"], dumps[5]["overlay"]["certificate_points"]), (231, 462))
        dumps[4]["overlay"]["outline_only"] = [0, 0]
        problems = run.expect_overlays(dumps, {}, "wj-ia-300", tables)
        self.assertEqual(len(problems), 1)
        self.assertIn("after 'wait 1504 ms' outline_only", problems[0])
        dumps[2]["tint"]["fill_sha256"] = dumps[0]["tint"]["fill_sha256"]
        self.assertTrue(any("after 'wait 1502 ms' fill_sha256" in p for p in run.expect_overlays(dumps, {}, "wj-ia-300", tables)))


class GripFrameRun(unittest.TestCase):
    def test_the_grip_script_uses_the_real_fixture_and_registered_check(self):
        run = shell_module("d02_run")
        script = json.loads(text(SHELL / "d02_scripts" / "grip-frame.json"))
        self.assertEqual((script["format"], script["resolution"], script["state"]),
                         ("magic600-d02-script/1", "1280x720", "wj-ia-300"))
        self.assertIn("grip-frame", run.ON_FIXTURE)
        self.assertIs(run.EXPECT["grip-frame"], run.expect_grip_frame)
        self.assertIn({"command": "grip.select"}, script["steps"])
        self.assertIn({"grip_click": "Local"}, script["steps"])

    def test_the_check_reads_the_chosen_item_ghost_and_head(self):
        def grip(cap, frame, corners, items, committed=300, digest="opening", ghost=False):
            chords = ("KeyJ", "Shift+KeyJ", "KeyK", "Shift+KeyK", "KeyL", "Shift+KeyL")
            keys = [dict(chord=chord, item=i, meaning="(14)(23)") for chord, i in zip(chords, items)]
            marks = [dict(index=i, corner=c, x=15 + i, y=20 + i) for i, c in enumerate(corners, 1)]
            drawn = [dict(pane=pane, rect=[10, 10, 100, 100], marks=marks) for pane in ("Global", "Local")]
            if ghost:
                for pane in drawn:
                    pane["direction"] = [30, 30]
            return dict(cap=cap, frame=frame, keys=keys, marks=marks, drawn=drawn, committed=committed, refused=0,
                        admissible_frames=[0, 1], state_digest=digest,
                        host=dict(cap=cap, frame=frame, revision=committed + 2, frames=[dict(frame=frame, keys=keys)],
                                  last=dict(cap=cap, item=items[0])),
                        ghost=dict(on=ghost, cap=cap, item=items[0], modifier="Ctrl", moving=10 if ghost else 0,
                                   outlined=9 if ghost else 0, palette=2, sha256="a" * 64 if ghost else None))

        p, q = [1, 1, 2, 5, 3, 9], [4, 4, 2, 5, 3, 9]
        a, b = [10, 20, 30, 40], [20, 30, 10, 40]
        start = grip(0, 0, a, p)
        turned = grip(150, 1, b, q, 301, "turned")
        held = grip(150, 1, b, q, 301, "turned", True)
        clicked = grip(150, 1, b, q, 302, "opening")
        records = [("wait 2500 ms", start), ("wait 600 ms", start), ("command grip.select", grip(150, 0, a, p)),
                   ("command grip.frame.next", grip(150, 1, b, q)), ("wait 1600 ms", turned), ("wait 601 ms", held),
                   ("wait 602 ms", turned), ("wait 603 ms", held), ("wait 604 ms", turned), ("wait 1601 ms", clicked)]
        dumps = [{"do": step, "grip": row, "pick": {"selection": {"slot": 150 * 433, "revision": 302}}}
                 for step, row in records]
        check = shell_module("d02_run").expect_grip_frame
        self.assertEqual(check(dumps, {}, "wj-ia-300"), [])
        held["state_digest"] = "ghost moved the head"
        held["ghost"]["outlined"] = 0
        clicked["host"]["last"]["item"] = 1
        problems = check(dumps, {}, "wj-ia-300")
        self.assertTrue(any("modifier release changed the head" in p for p in problems))
        self.assertTrue(any("lack the hover outline" in p for p in problems))
        self.assertTrue(any("Ctrl+click committed the wrong cap or item" in p for p in problems))


class GripKeycapsRun(unittest.TestCase):
    def test_the_keycaps_script_uses_the_real_fixture_and_registered_check(self):
        run = shell_module("d02_run")
        script = json.loads(text(SHELL / "d02_scripts" / "grip-keycaps.json"))
        self.assertEqual((script["format"], script["resolution"], script["state"]),
                         ("magic600-d02-script/1", "1280x720", "wj-ia-300"))
        self.assertIn("grip-keycaps", run.ON_FIXTURE)
        self.assertIs(run.EXPECT["grip-keycaps"], run.expect_grip_keycaps)
        self.assertIn({"command": "grip.frame.next"}, script["steps"])
        self.assertIn({"chord": "Shift+Slash"}, script["steps"])

    def test_the_check_reads_the_caps_stamps_and_marks(self):
        def dump(step, frame, items, changed=(), shown=True):
            chords = ("KeyJ", "Shift+KeyJ", "KeyK", "Shift+KeyK", "KeyL", "Shift+KeyL")
            meanings = ("(14)(23)", "(14)(23)", "(123)", "(132)", "(134)", "(143)")
            keys = [dict(chord=c, item=i, meaning=m) for c, i, m in zip(chords, items, meanings)]
            carried = [dict(key=c[3:], chord=c, item=i, meaning=m, label=m, context="grip", changed=c[3:] in changed)
                       for c, i, m in zip(chords, items, meanings) if "+" not in c]
            stamps = [dict(key=cap["key"], label=cap["meaning"], x=40 + 80 * n, y=30, box=[80 * n, 0, 80, 60])
                      for n, cap in enumerate(carried)]
            legend = "whole outline: bound here" + (" · corner mark: changed meaning since the grip frame changed"
                                                    if changed else "")
            keycaps = dict(shown=shown, header=f"Key set: default · pointer over Local · no modifier held · grip: cap 150, "
                           f"frame {frame}", legend=legend, grip=carried, stamps=stamps)
            return {"do": step, "grip": dict(cap=150, frame=frame, keys=keys), "keycaps": keycaps}

        p, q = [1, 1, 2, 5, 3, 9], [1, 1, 4, 7, 6, 8]
        dumps = [dump("wait 700 ms", 0, p), dump("command grip.frame.next", 2, q, "KL"), dump("wait 701 ms", 2, q, "KL"),
                 dump("chord Escape", 2, q, shown=False), dump("wait 702 ms", 2, q)]
        check = shell_module("d02_run").expect_grip_keycaps
        self.assertEqual(check(dumps, {}, "wj-ia-300"), [])
        dumps[2]["keycaps"]["grip"][0]["changed"] = True
        dumps[2]["keycaps"]["stamps"][1]["x"] = 500
        dumps[4]["keycaps"]["header"] = "Key set: default · pointer over Local · no modifier held"
        dumps[3]["keycaps"]["shown"] = True
        problems = check(dumps, {}, "wj-ia-300")
        self.assertTrue(any("marks ['J', 'K', 'L'], not the re-bound keys ['K', 'L']" in p for p in problems))
        self.assertTrue(any("K's stamp differs or lies outside its cap" in p for p in problems))
        self.assertTrue(any("the map shown again's header reads" in p for p in problems))
        self.assertTrue(any("Escape left the key map shown" in p for p in problems))
        self.assertEqual(check(dumps[:2], {}, "wj-ia-300"), ["the grip keycaps script is missing a grip or keycaps checkpoint"])


if __name__ == "__main__":
    unittest.main()
