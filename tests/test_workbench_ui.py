"""Offscreen smoke test of the workbench's Qt Quick UI on synthetic fixtures.

Run with the workbench environment (the test skips without PySide6):

  tools\\.venv\\workbench\\Scripts\\python.exe tests\\test_workbench_ui.py

It loads every QML file, follows a synthetic session, sends a note and exercises the
tool buttons, the run and launch controls and the attention list against a recording
launcher, so nothing is opened or started on the desktop and no real session is read.
"""
from __future__ import annotations

import json
import os
import sys
import unittest
from datetime import datetime, timedelta, timezone
from pathlib import Path

os.environ.setdefault("QT_QPA_PLATFORM", "offscreen")
ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tests"))
sys.path.insert(0, str(ROOT / "tools" / "workbench"))

try:
    import shiboken6
    from PySide6.QtCore import QMetaObject, QUrl
    from PySide6.QtGui import QGuiApplication
    from PySide6.QtQml import QQmlApplicationEngine, QQmlExpression
    from PySide6.QtQuickControls2 import QQuickStyle
except ImportError:  # the workbench environment is not installed here
    QGuiApplication = None

import test_workbench as tw  # noqa: E402  (fixtures)
import runs  # noqa: E402

FAKE_RID = "20260930T120000Z-0000abcd"


class Recorder:
    def __init__(self):
        self.calls = []

    def open(self, plan):
        self.calls.append(("open", plan))

    def command(self, argv, cwd):
        self.calls.append(("command", argv))

    def claude(self):
        self.calls.append(("claude",))

    def start_run(self, main, checkout, run_id, expect):
        self.calls.append(("start_run", str(checkout), run_id, expect))
        return FAKE_RID

    def start_codex(self, main, checkout, spec):
        self.calls.append(("start_codex", str(checkout), spec))
        return FAKE_RID

    def cancel_run(self, data, rid):
        self.calls.append(("cancel_run", rid))


def items(root):
    """Every visual item under `root`, including Repeater delegates (findChild misses them) and inactive tabs.

    Delegates replaced by a model reset can still be listed until Qt deletes them; skip them."""
    stack, out = [root], []
    while stack:
        item = stack.pop()
        if not shiboken6.isValid(item):
            continue
        out.append(item)
        stack.extend(item.childItems())
    return out


def iso_now(delta=0.0) -> str:
    return (datetime.now(timezone.utc) + timedelta(seconds=delta)).strftime("%Y-%m-%dT%H:%M:%SZ")


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
        self.assertTrue(wbapp.load_ui(self.engine, self.wb, compact=False))  # the app's own startup path
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

    def record(self, rid: str, status: str, **extra):
        doc = {"schema": 1, "rid": rid, "kind": "registry", "run": "demo", "title": "Demo checks",
               "checkout": str(self.fx.main), "cwd": str(self.fx.main), "display": [["python", "tests/demo.py"]],
               "steps": [["python", "tests/demo.py"]], "digest": None, "registry_sha256": None, "python": None,
               "created": iso_now(-120), "started": iso_now(-119), "ended": None, "alive_at": iso_now(),
               "status": status, "reason": None, "exits": [], "call_id": None,
               "inputs": {"files": {}, "incomplete": False}, "outputs": [], "log_dropped": 0,
               "lock": f"rid-{rid}.lock", "entry_lock": None, **extra}
        (self.fx.data / "runs" / f"{rid}.json").write_text(json.dumps(doc), encoding="utf-8")

    def click(self, name: str):
        item = self.item(name)
        if not QMetaObject.invokeMethod(item, "click"):
            QMetaObject.invokeMethod(item, "clicked")
        self.pump()

    def item(self, name: str):
        root = self.engine.rootObjects()[0].property("contentItem")  # contentItem() can return a stale wrapper
        found = [i for i in items(root) if i.objectName() == name]
        self.assertEqual(len(found), 1, name)
        return found[0]

    def hold(self, rid: str):
        """Hold the run's liveness lock, as a live runner does."""
        locks = self.fx.data / "runs" / "locks"
        locks.mkdir(parents=True, exist_ok=True)
        fd = os.open(locks / f"rid-{rid}.lock", os.O_RDWR | os.O_CREAT)
        if os.name == "nt":
            import msvcrt
            msvcrt.locking(fd, msvcrt.LK_NBLCK, 1)
            self.addCleanup(lambda: (os.lseek(fd, 0, 0), msvcrt.locking(fd, msvcrt.LK_UNLCK, 1), os.close(fd)))
        else:
            import fcntl
            fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
            self.addCleanup(os.close, fd)

    def choose(self, name: str, path: str):
        """Pick a checkout the way the owner does: select it and activate it."""
        box = self.item(name)
        code = f"currentIndex = indexOfValue({json.dumps(path)}); activated(currentIndex)"
        QQmlExpression(self.engine.contextForObject(box), box, code).evaluate()
        self.pump()

    def choose_packet(self, name: str):
        box = self.item("packetBox")
        code = f"currentIndex = find({json.dumps(name)}); activated(currentIndex)"
        QQmlExpression(self.engine.contextForObject(box), box, code).evaluate()
        self.pump()

    def test_checkout_choice_survives_list_changes(self):
        fx, wb, rec = self.fx, self.wb, self.recorder
        registry = fx.main / "tools" / "workbench" / "runs.json"
        registry.parent.mkdir(parents=True, exist_ok=True)
        registry.write_text(json.dumps({"schema": 1, "runs": [
            {"id": "demo", "title": "Demo checks", "steps": [["python", "tests/demo.py"]], "cwd": ".",
             "inputs": [], "outputs": [], "timeout_s": 60, "windows_required": False}]}), encoding="utf-8")
        for checkout, names in ((fx.main, ("p-main.md",)), (fx.wt, ("chosen.md", "newer.md"))):
            (checkout / "work" / "reviews" / "packets").mkdir(parents=True, exist_ok=True)
            for name in names:
                (checkout / "work" / "reviews" / "packets" / name).write_text("# packet\n", encoding="utf-8")
        chosen = fx.wt / "work" / "reviews" / "packets" / "chosen.md"
        os.utime(chosen, (1, 1))  # listed after newer.md
        wb.refresh()
        self.pump()
        wt = str(fx.wt.resolve())
        self.choose("runCheckout", wt)
        self.choose("launchCheckout", wt)
        self.assertEqual(self.item("packetBox").property("currentText"), "newer.md")  # the newest, until the owner chooses
        self.choose_packet("chosen.md")
        # A new worktree sorts ahead of the chosen one; the choice is kept by path, never moved to main.
        tree = fx.main.parent / "a new tree"
        meta = fx.main / ".git" / "worktrees" / "a-new"
        meta.mkdir(parents=True)
        (meta / "gitdir").write_text(str(tree / ".git") + "\n", encoding="utf-8")
        tree.mkdir()
        (tree / ".git").write_text(f"gitdir: {meta}\n", encoding="utf-8")
        wb.refresh()
        self.pump(10)
        self.assertIn(str(tree.resolve()), [c["path"] for c in wb.checkouts])
        for name in ("runCheckout", "launchCheckout"):
            box = self.item(name)
            self.assertEqual((box.property("path"), box.property("currentValue"), box.property("present")), (wt, wt, True), name)
        self.click("run-demo")
        self.assertEqual(rec.calls[-1][:3], ("start_run", wt, "demo"))
        # The packet choice survives too (the list reloads while the checkout model is replaced).
        self.assertEqual(self.item("packetBox").property("currentText"), "chosen.md")
        self.click("startCodex")
        self.assertEqual(rec.calls[-1][:2], ("start_codex", wt))
        self.assertEqual(rec.calls[-1][2]["packet"], "chosen.md")
        self.click("startSession")
        self.assertEqual(rec.calls[-1], ("command", ["claude", "Work on the problem packet work/reviews/packets/chosen.md. "
                                                               "Read it first, then follow CLAUDE.md."]))
        # The chosen packet goes away: nothing is selected in its place.
        chosen.unlink()
        self.click("reloadPackets")
        self.assertEqual(self.item("packetBox").property("currentIndex"), -1)
        for name in ("startSession", "startCodex"):
            self.assertFalse(self.item(name).property("enabled"), name)
        # The chosen worktree goes away: the choosers say so and nothing can start, rather than falling back to main.
        (meta.parent / "wt1" / "gitdir").unlink()
        wb.refresh()
        self.pump(10)
        for name in ("runCheckout", "launchCheckout"):
            self.assertFalse(self.item(name).property("present"), name)
        for name in ("run-demo", "startSession", "startCodex"):
            self.assertFalse(self.item(name).property("enabled"), name)
        self.assertEqual(self.item("packetBox").property("count"), 0)
        self.assertEqual(self.warnings, [])

    def test_malformed_and_older_runs_neither_break_nor_hide_attention(self):
        fx, wb = self.fx, self.wb
        (fx.data / "runs").mkdir(parents=True)
        overdue = "20260930T000000Z-0000c0de"
        self.record(overdue, "running", kind="codex", run="codex-review", title="Codex review",
                    created=iso_now(-10800), started=iso_now(-10800))
        self.hold(overdue)
        self.record("20260930T000100Z-0000bad1", "running", run=[])  # once raised TypeError in refresh
        for i in range(self.wbapp.RUN_HISTORY + 1):
            self.record(f"20260930T01{i // 60:02d}{i % 60:02d}Z-{i:08x}", "passed", exits=[0], ended=iso_now(-30))
        wb.refresh()
        self.pump()
        self.assertEqual(len(wb.runs), self.wbapp.RUN_HISTORY)
        self.assertNotIn(overdue, [r["rid"] for r in wb.runs])
        self.assertIn(("run_overdue", overdue), [(f["kind"], f["target_id"]) for f in wb.flags])
        self.assertEqual(runs.recent.malformed, 1)
        wb.selectFlag("run", overdue)  # the flag still opens the run beyond the listed history
        self.assertEqual((wb.selected["kind"], wb.selected["id"]), ("run", overdue))
        self.assertEqual(self.warnings, [])

    def test_runs_launch_and_attention(self):
        fx, wb, rec = self.fx, self.wb, self.recorder
        (fx.main / "tests").mkdir(exist_ok=True)
        (fx.main / "tests" / "demo.py").write_text("print('demo')\n", encoding="utf-8")
        registry = fx.main / "tools" / "workbench" / "runs.json"
        registry.parent.mkdir(parents=True, exist_ok=True)
        registry.write_text(json.dumps({"schema": 1, "runs": [
            {"id": "demo", "title": "Demo checks", "steps": [["python", "tests/demo.py"]], "cwd": ".",
             "inputs": ["tests/demo.py"], "outputs": [], "timeout_s": 60, "windows_required": False}]}), encoding="utf-8")
        packets = fx.main / "work" / "reviews" / "packets"
        packets.mkdir(parents=True)
        (packets / "p1.md").write_text("# packet\n", encoding="utf-8")
        (fx.data / "runs" / "locks").mkdir(parents=True)
        failed, live = "20260930T110000Z-000000f1", "20260930T110500Z-000000a1"
        self.record(failed, "failed", exits=[1], ended=iso_now(-60))
        (fx.data / "runs" / f"{failed}.log").write_text("$ python tests/demo.py\nboom\n[exit 1]\n", encoding="utf-8")
        self.record(live, "running")
        fd = os.open(fx.data / "runs" / "locks" / f"rid-{live}.lock", os.O_RDWR | os.O_CREAT)  # a live runner
        if os.name == "nt":
            import msvcrt
            msvcrt.locking(fd, msvcrt.LK_NBLCK, 1)
            self.addCleanup(lambda: (os.lseek(fd, 0, 0), msvcrt.locking(fd, msvcrt.LK_UNLCK, 1), os.close(fd)))
        else:
            import fcntl
            fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
            self.addCleanup(os.close, fd)
        wb.refresh()
        self.pump()
        # Boards: the registry, the records with derived liveness, and what needs attention.
        self.assertEqual([e["id"] for e in wb.registry["entries"]], ["demo"])
        self.assertEqual([x["rid"] for x in wb.registry["entries"][0]["live"]], [live])
        self.assertEqual({r["rid"]: r["status"] for r in wb.runs}, {failed: "failed", live: "running"})
        # The fixture's own session may also be flagged, depending on today's date; only runs are checked here.
        self.assertIn(("run_failed", failed), [(f["kind"], f["target_id"]) for f in wb.flags])
        self.assertNotIn(live, [f["target_id"] for f in wb.flags])
        self.assertEqual(wb.compact["attention"], len(wb.flags))
        # Run and Stop use the registry entry as shown; nothing else can be started from here.
        self.click("run-demo")
        self.assertEqual(rec.calls[-1], ("start_run", str(fx.main), "demo", runs.entry_digest(runs.load_registry(fx.main).entries[0])))
        self.click("stop-" + live)
        self.assertEqual(rec.calls[-1], ("cancel_run", live))
        wb.startRun("demo", str(fx.outsider), "x")
        self.assertIn("Not started", wb.message)
        self.assertEqual(rec.calls[-1], ("cancel_run", live))
        # The attention list selects the failed run and follows its log.
        self.click(f"flag-run_failed-{failed}")
        self.assertEqual((wb.selected["kind"], wb.selected["id"], wb.selected["status"]), ("run", failed, "failed"))
        self.assertIn("boom", [wb.timeline.rows[i]["body"] for i in range(wb.timeline.rowCount())])
        self.assertIn(f"{failed}.log (private, plain text)", [f["label"] for f in wb.files])
        self.assertFalse(self.item("stopRun").isVisible())
        wb.selectRun(live)
        self.pump()
        self.assertTrue(self.item("stopRun").isVisible())
        self.click("stopRun")
        self.assertEqual(rec.calls[-1], ("cancel_run", live))
        # Launch: a new session on a packet, then a Codex call with allowed values.
        self.click("reloadPackets")
        self.assertEqual(self.item("packetBox").property("currentText"), "p1.md")
        prompt = "Work on the problem packet work/reviews/packets/p1.md. Read it first, then follow CLAUDE.md."
        self.click("startSession")
        self.assertEqual(rec.calls[-1], ("command", ["claude", prompt]))
        self.item("remoteBox").setProperty("checked", True)
        self.click("startSession")
        self.assertEqual(rec.calls[-1], ("command", ["claude", prompt, "--remote-control"]))
        self.item("kindBox").setProperty("currentIndex", list(runs.CODEX_KINDS).index("implement"))
        self.pump()
        self.click("startCodex")
        self.assertEqual(rec.calls[-1], ("start_codex", str(fx.main), {"kind": "implement", "packet": "p1.md", "model": "gpt-6.1-sol",
                                                                       "effort": "max", "speed": "standard"}))
        self.assertIn("No commits", wb.message)
        # Read this engine's compact label inside QML: PySide can hand back a stale wrapper for a live item here.
        root = self.engine.rootObjects()[0]
        find = ("(function find(i) { if (i.objectName === 'compactAttention') return i.text;"
                " for (var k = 0; k < i.children.length; ++k) { var r = find(i.children[k]); if (r !== undefined) return r; }"
                " })(compactWindow.contentItem)")
        text, undefined = QQmlExpression(self.engine.contextForObject(root), root, find).evaluate()
        self.assertFalse(undefined)
        self.assertEqual(text, f"Needs attention: {len(wb.flags)}")
        self.assertEqual(self.warnings, [])

    def test_compact_mode_shows_a_top_level_window(self):
        # --compact once loaded a transient child of a hidden main window, so nothing ever appeared.
        engine = QQmlApplicationEngine()
        warnings = []
        engine.warnings.connect(lambda ws: warnings.extend(w.toString() for w in ws))
        self.assertTrue(self.wbapp.load_ui(engine, self.wb, compact=True))
        self.pump()
        (win,) = engine.rootObjects()
        self.assertEqual(win.title(), "Workbench progress")
        self.assertTrue(win.isVisible())
        self.assertIsNone(win.transientParent())
        self.assertTrue(win.property("standalone"))
        self.assertEqual(warnings, [])
        engine.deleteLater()

    def test_compact_view_from_the_main_window_is_top_level(self):
        main = self.engine.rootObjects()[0]
        compact = next(w for w in QGuiApplication.allWindows() if w.title() == "Workbench progress")
        self.assertIsNone(compact.transientParent())  # stays visible when the main window is minimized
        self.assertFalse(compact.property("standalone"))
        compact.show()
        main.showMinimized()
        self.pump()
        self.assertTrue(compact.isVisible())
        compact.close()


if __name__ == "__main__":
    unittest.main(verbosity=2)
