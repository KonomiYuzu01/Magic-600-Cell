"""Offscreen smoke test of the workbench's Qt Quick UI on synthetic fixtures.

Run with the workbench environment (the test skips without PySide6):

  tools\\.venv\\workbench\\Scripts\\python.exe tests\\test_workbench_ui.py

It loads every QML file, follows a synthetic session, sends a note and exercises the
tool buttons against a recording launcher, so nothing is opened on the desktop and
no real session is read.
"""
from __future__ import annotations

import json
import os
import sys
import unittest
from datetime import datetime, timezone
from pathlib import Path

os.environ.setdefault("QT_QPA_PLATFORM", "offscreen")
ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tests"))
sys.path.insert(0, str(ROOT / "tools" / "workbench"))

try:
    from PySide6.QtCore import QUrl
    from PySide6.QtGui import QGuiApplication
    from PySide6.QtQml import QQmlApplicationEngine
    from PySide6.QtQuickControls2 import QQuickStyle
except ImportError:  # the workbench environment is not installed here
    QGuiApplication = None

import test_workbench as tw  # noqa: E402  (fixtures)


class Recorder:
    def __init__(self):
        self.calls = []

    def open(self, plan):
        self.calls.append(("open", plan))

    def command(self, argv, cwd):
        self.calls.append(("command", argv))

    def claude(self):
        self.calls.append(("claude",))


@unittest.skipIf(QGuiApplication is None, "PySide6 is not installed in this interpreter")
class WorkbenchUiTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        QQuickStyle.setStyle("Fusion")
        cls.app = QGuiApplication.instance() or QGuiApplication([])

    def setUp(self):
        import app as wbapp
        self.wbapp = wbapp
        self.fx = tw.Fixture()
        self.addCleanup(self.fx.cleanup)
        fx = self.fx
        (fx.main / "docs" / "wiki").mkdir(parents=True)
        (fx.main / "docs" / "wiki" / "index.md").write_text("# Index\n", encoding="utf-8")
        (fx.main / "docs" / "progress").mkdir(parents=True)
        (fx.main / "docs" / "progress" / "status.json").write_bytes((ROOT / "docs" / "progress" / "status.json").read_bytes())
        doc = fx.main / "docs" / "notes.md"
        doc.write_text("x", encoding="utf-8")
        fx.transcript(fx.wt, tw.SID, [tw.user(0, cwd=str(fx.wt)),
                                      tw.assistant(1, "Working on it.", uses=[("u1", "Read", {"file_path": str(doc)})]),
                                      tw.result(2, "u1", "file text"),
                                      tw.assistant(3, uses=[("u2", "Bash", {"command": "python tests/test_core.py"})])],
                      raw_lines=["{broken"])
        fx.events(tw.SID, [tw.ev(2, "SubagentStart", agent_id="ag1", agent_type="Explore")])
        self.recorder = Recorder()
        self.wb = wbapp.Workbench(fx.main, fx.projects, launcher=self.recorder, refresh_ms=60_000)
        self.engine = QQmlApplicationEngine()
        self.warnings = []
        self.engine.warnings.connect(lambda ws: self.warnings.extend(w.toString() for w in ws))
        self.engine.rootContext().setContextProperty("wb", self.wb)
        self.engine.setInitialProperties({"startCompact": False})
        self.engine.load(QUrl.fromLocalFile(str(wbapp.QML_DIR / "Main.qml")))
        self.addCleanup(self.engine.deleteLater)

    def pump(self, n=5):
        for _ in range(n):
            self.app.processEvents()

    def test_boards_live_view_notes_and_tools(self):
        self.assertTrue(self.engine.rootObjects(), self.warnings)
        self.pump()
        wb = self.wb
        self.assertEqual([s["sid"] for s in wb.sessions], [tw.SID])
        self.assertEqual(wb.sessions[0]["status"], "running")
        self.assertEqual(wb.sessions[0]["subagents"][0]["id"], "ag1")
        self.assertTrue(wb.compact["step"].startswith("0.4.1-1"))
        wb.selectSession(tw.SID)
        self.pump()
        self.assertGreaterEqual(wb.timeline.rowCount(), 5)
        self.assertEqual(wb.selected["malformedLive"], 1)
        labels = [f["label"] for f in wb.files]
        self.assertIn("notes.md", labels)
        self.assertIn("Wiki index", labels)
        # Appended records reach the live view.
        path = next(self.fx.projects.glob("*/" + tw.SID + ".jsonl"))
        before = wb.timeline.rowCount()
        with path.open("a", encoding="utf-8") as f:
            f.write(json.dumps(tw.result(4, "u2", "OK")) + "\n")
        wb.poll()
        self.assertEqual(wb.timeline.rowCount(), before + 1)
        # Owner note: the only write, into the private inbox.
        self.assertTrue(wb.sendNote("Please stop after the tests."))
        self.assertFalse(wb.sendNote("\U0001F600" * 1001))
        self.assertEqual([n["state"] for n in wb.noteStates], ["pending"])
        written = sorted(p.relative_to(self.fx.data).as_posix() for p in self.fx.data.rglob("*") if p.is_file())
        self.assertEqual(written, ["events/" + tw.SID + ".jsonl", "inbox/" + tw.SID + ".jsonl"])
        # Tools: files, git diff, and "open in Claude Code" never resumes a running session.
        wb.openFile(str(self.fx.main / "docs" / "notes.md"))
        wb.openFile(str(path))
        wb.runTool("git-diff", "")
        wb.openInClaude(False)
        kinds = [c[0] for c in self.recorder.calls]
        self.assertEqual(kinds, ["open", "open", "command", "claude"])
        self.assertEqual(self.recorder.calls[1][1][0], "notepad")
        self.assertEqual(self.recorder.calls[2][1][:3], ["git", "-C", str(self.fx.wt)])
        self.assertIn("claude --resume", wb.message)
        # Subagent and Codex call views load.
        wb.selectSubagent(tw.SID, "ag1")
        self.assertIn("No transcript", wb.message)
        self.pump()
        self.assertEqual(self.warnings, [])

    def test_resume_decides_on_fresh_lifecycle_evidence(self):
        fx, wb = self.fx, self.wb
        ended = "dddddddd-0000-0000-0000-000000000001"
        path = fx.transcript(fx.main, ended, [tw.user(0, cwd=str(fx.main))])
        fx.events(ended, [tw.ev(5, "SessionEnd", reason="exit")])
        wb.refresh()
        wb.selectSession(ended)
        self.assertEqual(wb.selected["status"], "finished")
        # Another client resumes the session after the last refresh.
        with path.open("a", encoding="utf-8") as f:
            f.write(json.dumps(tw.user(30, "resumed elsewhere")) + "\n")
        wb.openInClaude(False)
        self.assertEqual([c[0] for c in self.recorder.calls], ["claude"])
        # A session that really ended is resumed in its own console.
        fx.events(ended, [tw.ev(40, "SessionEnd", reason="exit")])
        self.recorder.calls.clear()
        wb.openInClaude(True)
        self.assertEqual(self.recorder.calls, [("command", ["claude", "--resume", ended, "--remote-control"])])

    def test_selected_call_updates_when_it_finishes(self):
        fx, wb = self.fx, self.wb
        call_id = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ") + "-abcdef12"  # younger than the stale limit
        d = fx.main / "work" / "reviews" / call_id
        d.mkdir(parents=True)
        (d / "packet.md").write_text("packet", encoding="utf-8")
        wb.refresh()
        wb.selectCall(call_id)
        self.assertEqual(wb.selected["status"], "running")
        (d / "review.json").write_text(json.dumps({"verdict": "pass", "summary": "All good.", "findings": []}), encoding="utf-8")
        (d / "meta.json").write_text(json.dumps({"call_id": call_id, "kind": "review", "valid": True, "verdict": "pass"}), encoding="utf-8")
        wb.refresh()
        self.assertEqual((wb.selected["status"], wb.selected["verdict"]), ("finished", "pass"))
        titles = [wb.timeline.rows[i]["title"] for i in range(wb.timeline.rowCount())]
        self.assertIn("Summary", titles)
        self.assertIn("review.json (private, plain text)", [f["label"] for f in wb.files])

    def test_compact_window_loads(self):
        engine = QQmlApplicationEngine()
        warnings = []
        engine.warnings.connect(lambda ws: warnings.extend(w.toString() for w in ws))
        engine.rootContext().setContextProperty("wb", self.wb)
        engine.setInitialProperties({"startCompact": True})
        engine.load(QUrl.fromLocalFile(str(self.wbapp.QML_DIR / "Main.qml")))
        self.pump()
        self.assertTrue(engine.rootObjects())
        self.assertEqual(warnings, [])
        engine.deleteLater()


if __name__ == "__main__":
    unittest.main(verbosity=2)
