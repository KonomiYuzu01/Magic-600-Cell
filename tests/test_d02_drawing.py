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
import tempfile
import unittest
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


class CoreFocus(unittest.TestCase):
    """The core's ABI 9 on its three sides: the C header, c600-render's Rust ABI and the bridge's function table."""

    def test_the_version_is_9_everywhere(self):
        header = text(CORE / "include" / "sa2_interop.h")
        self.assertEqual(re.findall(r"^#define SA2_ABI_VERSION (\S+)$", header, re.M), ["9u"])
        self.assertIn("pub const ABI_VERSION: u32 = 9;", text(ROOT / "rust" / "c600-render" / "src" / "abi.rs"))
        self.assertIn("public const uint AbiVersion = 9;", text(PROJECT / "CoreLibrary.cs"))
        self.assertIn("public const uint SA2_ABI_VERSION = 9;", text(ROOT / "renderer" / "acceptance" / "project" / "Native.cs"))

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
        self.assertEqual(names[-1], "sa2_view_focus_set")
        self.assertEqual(len(names), int(block.group(1)))
        core = text(PROJECT / "CoreLibrary.cs")
        exports = re.findall(r'"(sa2_[a-z_]+)"', re.search(r"\bSymbols = new\[\]\s*\{(.*?)\};", core, re.S).group(1))
        self.assertEqual(exports[-1], "sa2_view_focus_set")
        self.assertIn("view_focus_set,\n    }\n}", text(ROOT / "renderer" / "bridge" / "tests" / "common" / "mod.rs"))


class HostFocus(unittest.TestCase):
    """The host library's version 12: m6h_set_view_focus, the same on the Rust and C# sides."""

    def test_the_host_and_the_shell_speak_version_12(self):
        lib = text(ROOT / "renderer" / "bridge" / "host" / "src" / "lib.rs")
        host = text(PROJECT / "HostLibrary.cs")
        self.assertIn("pub const M6H_ABI_VERSION: u32 = 12;", lib)
        self.assertIn("public const uint AbiVersion = 12;", host)
        self.assertIn("assert_eq!(m6h_abi_version(), 12);", text(ROOT / "renderer" / "bridge" / "host" / "tests" / "turns.rs"))
        self.assertRegex(lib, r'pub unsafe extern "C" fn m6h_set_view_focus\(\s*handle: \*mut ShellHost,\s*view: u64,\s*'
                              r'projection: u32,\s*focus_cell: u32,\s*focus_cos: f32,\s*spread: f32,\s*\) -> i32')
        self.assertIn("public int SetViewFocus(IntPtr host, ulong view, uint projection, uint focusCell, float focusCos, float spread)",
                      host)
        # An unopened view and a value out of range are refused before the core is called; the default clears it.
        self.assertIn("if !shell.names_open_view(id) {", lib[lib.index("fn m6h_set_view_focus("):])
        self.assertIn("let focus = (focus != Sa2ViewFocus::defaults()).then_some(focus);", lib)
        self.assertIn("fn the_host_sets_a_view_focus()", text(ROOT / "renderer" / "bridge" / "host" / "tests" / "c_api.rs"))


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
        temp = Path(tempfile.mkdtemp(prefix="d02-local-"))
        self.addCleanup(shutil.rmtree, temp, True)
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
        self.assertIn("int? cap = _chrome?.Jumble?.Naming.Named;", body)
        self.assertIn("cell == HostLibrary.FocusNone ? -1 : D02.ShellLocal.FocusCos", body)
        self.assertIn("library.SetViewVisible(_host, HostLibrary.Local, cell != HostLibrary.FocusNone)", body)
        self.assertIn("chrome.Frame.LocalOpeningShown = cell != HostLibrary.FocusNone;", body)

    def test_the_panes_are_global_then_local_inside_the_surface(self):
        self.assertIn("var views = new[] { HostLibrary.View, HostLibrary.Local };", self.shell)
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
        self.assertIn("library.SetViewCamera(_host, HostLibrary.Local, LocalCamera())", self.shell)

    def test_the_frame_row_names_composite_frames(self):
        log = text(PROJECT / "EventLog.cs")
        self.assertIn('if (frame.Panes > 0)\n            {\n                json.WriteNumber("panes", frame.Panes);\n'
                      '                json.WriteNumber("held", frame.Held);', log)


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
        self.assertIn("int height = ViewPanes.OpeningHeight((int)Size.X, (int)Size.Y, frame.LocalOpeningShown);", frame)
        self.assertIn("foreach (Rect2 open in new[] { last, lastLocal })", frame)
        self.assertIn("foreach (PaneRect piece in ViewPanes.Ground(all, holes))", frame)
        chrome = text(PROJECT / "ShellChrome.cs")
        self.assertIn("{ OpenGround = live, LocalDrawn = live && ShellJumble.On }", chrome)
        self.assertIn('Frame.Local.SetStatus(Frame.LocalDrawn ? null : "not drawn yet", Array.Empty<string>(), null);', chrome)
        self.assertIn("public Rect2I LocalOpening() => Pixels(Frame.LocalOpening);", chrome)
        program = text(ROOT / "tools" / "looklab" / "tests" / "Program.cs")
        self.assertIn('("D02GroundCutsBothOpenings", D02GroundCutsBothOpenings)', program)


class TwoViewsRun(unittest.TestCase):
    """The two-views run's event check reads the frame rows: one-view frames before a cap is named, then composite
    frames of two drawn panes at every revision the run reached."""

    def check(self, rows):
        run = shell_module("d02_run")
        temp = Path(tempfile.mkdtemp(prefix="d02-two-views-"))
        self.addCleanup(shutil.rmtree, temp, True)
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

    def test_a_good_run_passes(self):
        rows = [{"frame_id": 1, "engine_revision": 302}, {"frame_id": 2, "engine_revision": 302, "panes": 2, "held": 0},
                {"frame_id": 3, "engine_revision": 303, "panes": 2, "held": 1},
                {"frame_id": 4, "engine_revision": 303, "panes": 2, "held": 0}]
        self.assertEqual(self.check(rows), [])

    def test_the_failures_are_named(self):
        early = self.check([{"frame_id": 1, "engine_revision": 302, "panes": 2, "held": 0},
                            {"frame_id": 2, "engine_revision": 303, "panes": 2, "held": 0}])
        self.assertTrue(any("before any cap was named" in p for p in early), early)
        undrawn = self.check([{"frame_id": 1, "engine_revision": 302}, {"frame_id": 2, "engine_revision": 302, "panes": 2, "held": 0},
                              {"frame_id": 3, "engine_revision": 303, "panes": 2, "held": 1}])
        self.assertTrue(any("[303] were never drawn" in p for p in undrawn), undrawn)
        still = self.check([{"frame_id": 1, "engine_revision": 302}, {"frame_id": 2, "engine_revision": 302, "panes": 2, "held": 0}])
        self.assertTrue(any("no revision changed" in p for p in still), still)
        one = self.check([{"frame_id": 1, "engine_revision": 302}, {"frame_id": 2, "engine_revision": 303}])
        self.assertTrue(any("no composite frame of two drawn panes" in p for p in one), one)


if __name__ == "__main__":
    unittest.main()
