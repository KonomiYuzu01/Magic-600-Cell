"""Static checks of the shared chrome library after ADR-D02 track C's port; never builds, launches Godot or a GPU.

The owner's rule (26 November 2026): the product is the only place where UI is built, shown and accepted, so all UI
and wiring code lives in the library both apps stage (tools/looklab/app/Chrome and tools/looklab/data/design), never
again in Look Lab-only code (tools/looklab/app/Live). These checks keep each ported part in one copy in that library,
staged into the product shell by renderer/shell/build_shell.py and called by both the product's sample mode and Look
Lab. A pass is source evidence only; the builds, the console tests and the windowed checks are run separately.
"""
from __future__ import annotations

import importlib
import shutil
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SHELL = ROOT / "renderer" / "shell"
LAB = ROOT / "tools" / "looklab"
CHROME = LAB / "app" / "Chrome"
LIVE = LAB / "app" / "Live"


def text(path: Path) -> str:
    return path.read_bytes().decode("utf-8")


def shell_module(name: str):
    sys.path.insert(0, str(SHELL))
    try:
        return importlib.import_module(name)
    finally:
        sys.path.remove(str(SHELL))


class Staged(unittest.TestCase):
    """The shell's build stages the shared library whole, so a part moved there reaches the product."""

    @classmethod
    def setUpClass(cls):
        cls.temp = Path(tempfile.mkdtemp(prefix="shared-chrome-"))
        cls.project = cls.temp / "staged"
        shell_module("build_shell").stage(cls.project)

    @classmethod
    def tearDownClass(cls):
        shutil.rmtree(cls.temp, True)

    def staged(self, relative: str) -> bytes:
        return (self.project / "Chrome" / relative).read_bytes()

    def test_the_ported_files_are_staged_as_they_are(self):
        for relative in ("Ui/ChromeErrors.cs", "Model/ErrorLog.cs", "Model/CorePlaces.cs", "Model/LayoutTracker.cs", "Ui/FrameLayout.cs",
                         "Model/ViewFields.cs", "Model/Inspect.cs", "Ui/InspectorPanels.cs"):
            self.assertEqual(self.staged(relative), (CHROME / relative).read_bytes(), relative)


class ErrorLamp(unittest.TestCase):
    """Item 1: failures show in Problems with a lamp, in the product (sample mode and D02 mode) and in Look Lab."""

    def test_one_copy_in_the_shared_library(self):
        lamp = text(CHROME / "Ui" / "ChromeErrors.cs")
        self.assertIn("public static class ChromeErrors", lamp)
        self.assertIn("public static ErrorLamp Mount(ChromeFrame frame, FeedbackModel? problems, string source, string? file = null)", lamp)
        self.assertIn("public sealed class ErrorLog", text(CHROME / "Model" / "ErrorLog.cs"))
        # Look Lab's own copy is gone, and nothing names it.
        self.assertFalse(list(LAB.rglob("LabErrors.cs")))
        for folder in (LAB / "app", SHELL):
            for path in folder.rglob("*.cs"):
                self.assertNotIn("LabErrors", text(path), path.name)

    def test_the_listener_rules_are_kept(self):
        lamp = text(CHROME / "Ui" / "ChromeErrors.cs")
        # Every engine error and every caught listener; warnings are not failures.
        self.assertIn("OS.AddLogger(listener);", lamp)
        self.assertIn("Safe.Failed += Log.Add;", lamp)
        self.assertIn("if (errorType == (int)Logger.ErrorType.Warning) return;", lamp)
        # The listener leaves before the engine's shutdown logs: by Remove, or with the scene tree for a lamp that
        # installed it.
        self.assertIn("Safe.Failed -= Log.Add; OS.RemoveLogger(listener); listener = null;", lamp)
        self.assertIn("tree.Root.TreeExiting += () => { Remove(); leaving = false; };", lamp)
        # The lamp opens Problems, and a host without a Problems panel gets one.
        self.assertIn('frame.Model.ShowDock(DockSide.Bottom, "problems")', lamp)
        self.assertIn('frame.SetContent("problems", panel);', lamp)
        self.assertIn("frame.SetAlert(shown, shown == 0 ? \"\" : first);", lamp)

    def test_every_app_mounts_the_shared_lamp(self):
        self.assertIn("ChromeErrors.Mount(frame, Inspect.Feedback, ErrorSource);", text(LIVE / "SampleMount.cs"))
        self.assertIn('ChromeErrors.Mount(frame, inspect.Feedback, "look-lab");', text(LIVE / "LabFrame.cs"))
        session = text(LIVE / "LiveSession.cs")
        self.assertNotIn("SetAlert(", session)
        self.assertIn("ChromeErrors.Remove();", session)
        self.assertIn("ChromeErrors.Install(", text(LAB / "app" / "Entry.cs"))
        sweep = text(LIVE / "Sweep.cs")
        self.assertIn('ChromeErrors.Install(Path.Combine(folder, "errors.log")); ChromeErrors.Reset();', sweep)
        # The lamp alone opens Problems; no host adds a second handler.
        for name in ("SampleMount.cs", "LabFrame.cs"):
            self.assertNotIn("AlertPressed", text(LIVE / name), name)


class StripAnchors(unittest.TestCase):
    """Item 2 (ADR-D01.62): the strip's anchors in Local and its markers in Global, where the product's core draws the
    positions; Look Lab keeps its own views' projection, the product gets the core's."""

    def test_the_core_projection_is_shared_and_engine_free(self):
        places = text(CHROME / "Model" / "CorePlaces.cs")
        self.assertIn("public sealed class CorePlaces", places)
        self.assertIn("public sealed record CoreView(", places)
        # The core's own formulas (renderer/core/shaders/geometry.hlsl) and defaults (sa2_interop.h).
        shader = text(ROOT / "renderer" / "core" / "shaders" / "geometry.hlsl")
        self.assertIn("float3 p = d4 * world.xyz / (d4 - world.w);", shader)
        self.assertIn("double scale = view.Orthographic ? 1 : view.D4 / (view.D4 - w[3]);", places)
        self.assertIn("result.clip = float4(p.x*zoom/aspect, p.y*zoom, 100.0/(100.0-0.05)*(z-0.05), z);", shader)
        self.assertIn("double x = px * view.Zoom / view.Aspect / z, y = py * view.Zoom / z;", places)
        header = text(ROOT / "renderer" / "core" / "include" / "sa2_interop.h")
        for name, value in (("CELL_SHRINK", "0.76"), ("STICKER_SHRINK", "0.82"), ("D4", "1.18"), ("ZOOM", "1.15")):
            self.assertIn(f"#define SA2_SHAPE_DEFAULT_{name} {value}f", header)
        self.assertIn("DefaultCellShrink = .76, DefaultStickerShrink = .82, DefaultD4 = 1.18, DefaultZoom = 1.15;", places)
        # The shared library needs nothing of Look Lab's engine.
        self.assertNotIn("LookLab", places)

    def test_sample_mode_gives_the_strip_both_projections(self):
        mount = text(LIVE / "SampleMount.cs")
        self.assertIn("Solve = new LabSolve(frame, root, seconds, run, Demo, Places?.Projector(Global), Places is { } local ? LocalPlace(local) : null);", mount)
        self.assertIn('System.Environment.GetEnvironmentVariable("M600_ASSETS")', mount)
        self.assertIn("public Func<CoreView?>? GlobalView { get; set; }", mount)
        self.assertIn("public Func<CoreView?>? LocalView { get; set; }", mount)
        # Under --d02 Global draws no anchor until the host describes it (the Hopf rings are not ported).
        self.assertIn('if (System.Environment.GetEnvironmentVariable("M600_D02") == "1") return null;', mount)
        # Look Lab still gives its own views' projection.
        self.assertIn("(orbit, position) => Place(app.View, orbit, position), (orbit, position) => Place(app.Linked.Local, orbit, position)", text(LIVE / "LabFrame.cs"))


class LayoutSteps(unittest.TestCase):
    """Item 3 (ADR-D01.14): the layout history records in the product's sample mode as in Look Lab, from one tracker
    in the shared library."""

    def test_one_tracker_in_the_shared_library(self):
        tracker = text(CHROME / "Model" / "LayoutTracker.cs")
        self.assertIn("public sealed class LayoutTracker", tracker)
        self.assertIn('public static bool Handles(string command) => command is "layout.undo" or "layout.redo" or "layout.roll";', tracker)
        frame = text(CHROME / "Ui" / "FrameLayout.cs")
        self.assertIn("public static LayoutTracker Tracker(ChromeFrame frame, LayoutHistory? layout = null)", frame)
        self.assertIn("frame.Rebuilt += tracker.Record; frame.Recoloured += tracker.Record;", frame)
        # Look Lab's demo keeps no copy of its own; it runs the shared tracker.
        demo = text(LIVE / "LabDemo.cs")
        self.assertIn("Tracker = FrameLayout.Tracker(frame, layout);", demo)
        self.assertNotIn("RecordLayout", demo)
        self.assertIn("if (Tracker.Execute(command, argument) is { } still) frame.Say(still, true);", demo)
        # An appearance step is restored in place, so the panel that shows it stays the same object.
        self.assertIn("public void Restore(string stored)", text(CHROME / "Model" / "Appearance.cs"))

    def test_both_apps_track_the_layout(self):
        mount = text(LIVE / "SampleMount.cs")
        self.assertIn("Callable.From(TrackLayout).CallDeferred();", mount)
        self.assertIn('Demo.TrackLayout(FrameLayout.State(frame, () => Look?.Look.Store() ?? "", () => Scripts.Store()),', mount)
        self.assertIn("LayoutHistory? layout = null)", mount)
        self.assertIn('demo.TrackLayout(FrameLayout.State(frame, () => app.ViewParts["frame.appearance"].Get(), () => app.ViewParts["frame.keymap"].Get()),', text(LIVE / "LabFrame.cs"))


class ViewSection(unittest.TestCase):
    """Item 4 (ADR-D01.22 with revision 6's Q3): the View section's fields and its scope, Both (the default), Global or
    Local, in one copy that Look Lab and the product's sample mode both build."""

    def test_one_copy_of_the_fields_and_the_scope(self):
        fields = text(CHROME / "Model" / "ViewFields.cs")
        self.assertIn("public enum ViewScope { Both, Global, Local }", fields)
        # The eligible fields: projection, shrink and frame visibility.
        for line in ('ViewFieldSpec.Choice(Projection, "Projection", "perspective", "orthographic"),',
                     'new ViewFieldSpec(CellShrink, "Cell shrink", .3, 1, .01),',
                     'new ViewFieldSpec(StickerShrink, "Sticker shrink", .3, 1, .01),',
                     'ViewFieldSpec.Choice(Frame, "Frame", "shown", "hidden"),'):
            self.assertIn(line, fields)
        # Under Both an edit writes both views.
        self.assertIn('ViewScope.Global => new[] { "Global" }, ViewScope.Local => new[] { "Local" }, _ => Views };', fields)
        inspect = text(CHROME / "Model" / "Inspect.cs")
        self.assertIn("public ViewScope Scope { get; private set; } = ViewScope.Both;", inspect)
        self.assertIn("public InspectField AddChoice(", inspect)
        self.assertIn("public void Rebase(string id, double value, double @default, Provenance provenance)", inspect)
        # The docked inspector draws the switch in the View head when the host keeps per-view values.
        panel = text(CHROME / "Ui" / "InspectorPanels.cs")
        self.assertIn("if (section == InspectorModel.View && Model.Scopes.Count > 1)", panel)
        self.assertIn("string said = section == InspectorModel.View ? ViewFields.Note(Model.Scope) : Notes[section];", panel)

    def test_both_apps_build_the_section(self):
        mount = text(LIVE / "SampleMount.cs")
        self.assertIn("inspector => views = new ViewFields(inspector, ViewFields.Product, true, start));", mount)
        self.assertIn("public Func<string, ViewFields, string?>? ApplyView { get; set; }", mount)
        self.assertIn('ProjectSettings.GlobalizePath("res://look/local-view.json")', mount)
        lab = text(LIVE / "LabInspect.cs")
        self.assertIn("views = new ViewFields(inspector, specs, false,", lab)
        self.assertNotIn("ViewEdited", lab)
        self.assertNotIn("ViewEdited", text(LIVE / "SampleInspect.cs"))


if __name__ == "__main__":
    unittest.main()
