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
        for relative in ("Ui/ChromeErrors.cs", "Model/ErrorLog.cs"):
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


if __name__ == "__main__":
    unittest.main()
