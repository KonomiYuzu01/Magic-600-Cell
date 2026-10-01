"""Offscreen smoke test of the workbench's Qt Quick UI on synthetic fixtures.

Run with the workbench environment (the test skips without PySide6):

  tools\\.venv\\workbench\\Scripts\\python.exe tests\\test_workbench_ui.py

It loads every QML file, follows a synthetic session, sends a note and exercises the
tool buttons, the run and launch controls and the attention list against a recording
launcher, so nothing is opened or started on the desktop and no real session is read.
"""
from __future__ import annotations

import contextlib
import io
import json
import os
import sys
import time
import unittest
from datetime import datetime, timedelta, timezone
from pathlib import Path
from unittest.mock import patch

os.environ.setdefault("QT_QPA_PLATFORM", "offscreen")
os.environ.setdefault("QML_DISABLE_DISK_CACHE", "1")
os.environ.setdefault("PYTHONDONTWRITEBYTECODE", "1")
sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tests"))
sys.path.insert(0, str(ROOT / "tools" / "workbench"))

try:
    import shiboken6
    from PySide6.QtCore import QCoreApplication, QEvent, QObject, QUrl, Signal
    from PySide6.QtQml import QQmlApplicationEngine, QQmlExpression
    from PySide6.QtQuick import QQuickWindow
    from PySide6.QtQuickControls2 import QQuickStyle
    from PySide6.QtWidgets import QApplication
except ImportError:  # the workbench environment is not installed here
    QApplication = None

import test_workbench as tw  # noqa: E402  (fixtures)
from test_workbench_home import ASK, png, sample_status  # noqa: E402
import checklist  # noqa: E402
import home  # noqa: E402
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


@unittest.skipIf(QApplication is None, "PySide6 is not installed in this interpreter")
class WorkbenchUiTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        QQuickStyle.setStyle("Fusion")
        cls.app = QApplication.instance() or QApplication([])

    def setUp(self):
        import app as wbapp
        self.wbapp = wbapp
        mkdir = os.mkdir

        def fixture_mkdir(path, mode=0o777, *, dir_fd=None):
            # Python 3.14's Windows 0700 ACL excludes the sandbox token; inherit fixture permissions.
            return mkdir(path, 0o777 if os.name == "nt" and mode == 0o700 else mode, dir_fd=dir_fd)

        with patch("tempfile._os.mkdir", fixture_mkdir):
            self.fx = tw.Fixture()
        self.addCleanup(self.fx.cleanup)
        fx = self.fx
        (fx.main / "docs" / "wiki").mkdir(parents=True)
        (fx.main / "docs" / "wiki" / "index.md").write_text("# Index\n", encoding="utf-8")
        (fx.main / "docs" / "progress").mkdir(parents=True)
        (fx.main / "docs" / "progress" / "status.json").write_text(checklist.dump(sample_status()), encoding="utf-8")
        doc = fx.main / "docs" / "notes.md"
        doc.write_text("x", encoding="utf-8")
        fx.transcript(fx.wt, tw.SID, [tw.user(0, cwd=str(fx.wt)),
                                      tw.assistant(1, "Working on it.", uses=[("u1", "Read", {"file_path": str(doc)})]),
                                      tw.result(2, "u1", "file text"),
                                      tw.assistant(3, uses=[("u2", "Bash", {"command": "python tests/test_core.py"})])],
                      raw_lines=["{broken"])
        fx.events(tw.SID, [tw.ev(2, "SubagentStart", agent_id="ag1", agent_type="Explore")])
        self.fake_cards = []
        self.fake_items = []
        self.scan_calls = []
        self.patch_home()
        self.recorder = Recorder()
        self.wb = wbapp.Workbench(fx.main, fx.projects, launcher=self.recorder, refresh_ms=60_000)
        self.engine = QQmlApplicationEngine()
        self.warnings = []
        self.engine.warnings.connect(lambda ws: self.warnings.extend(w.toString() for w in ws))
        self.assertTrue(wbapp.load_ui(self.engine, self.wb, compact=False))  # the app's own startup path
        self.addCleanup(self.close_ui)

    def patch_home(self):
        """Patch only the parallel packet's unfinished derivations, for this entire test."""
        owner = self

        class FakeScanner:
            def scan(self, main, roots, sessions, records, asks, now, force=False):
                owner.scan_calls.append({"main": main, "roots": roots, "sessions": sessions, "records": records,
                                         "asks": asks, "now": now, "force": force})
                return [dict(row) for row in owner.fake_items]

        def fake_inbox(sessions, calls, links, records, flags, data, roots, now):
            by_sid = {s.sid: s for s in sessions}
            cards = []
            for seed in owner.fake_cards:
                card = dict(seed)
                s = by_sid.get(card["sid"])
                prefix = f"answer to {card['answerRef']}: " if card["answerRef"] else None
                answers = [n for n in tw.sources.notes(data, card["sid"], s.state if s else None)
                           if prefix and n["text"].startswith(prefix)] if card["sid"] else []
                if answers:
                    note = answers[-1]
                    card["answer"] = note["text"][len(prefix):]
                    card["state"] = {"pending": "queued", "emitted": "sent", "received": "sent", "answered": "acknowledged"}[note["state"]]
                    card["needsOwner"] = card["state"] == "queued" and (s is None or s.status.status != "running")
                cards.append(card)
            return sorted(cards, key=lambda c: c["t"], reverse=True), 0

        def fake_progress(doc, sessions):
            no_checklist = doc.get("schema") != 2
            tracks = []
            for tid in checklist.TRACKS:
                steps = [s for s in doc.get("steps", []) if s["track"] == tid]
                if not steps:
                    continue
                current = next((s for s in steps if s["id"] == doc["current"]), steps[0])
                percent = lambda s: None if no_checklist else checklist.step_percent(s)
                tracks.append({"id": tid, "title": f"Track {tid}", "percent": checklist.track_percent(doc, tid),
                               "steps": [{"id": s["id"], "title": s["title"], "status": s["status"], "percent": percent(s),
                                          "done": sum(checklist.counted(i) for i in s.get("items", [])),
                                          "total": len(s.get("items", []))} for s in steps],
                               "current": {"id": current["id"], "title": current["title"], "percent": percent(current),
                                           "remaining": [{k: i[k] for k in ("id", "title", "weight")} for i in checklist.remaining(current)]}})
            tasks = []
            for s in sessions:
                task_list = s.state.task_list() if s.state else []
                if task_list:
                    tasks.append({"sid": s.sid, "title": s.title or s.sid[:8],
                                  "done": sum(t["status"] == "completed" for t in task_list), "total": len(task_list)})
            return {"schema": doc.get("schema"), "noChecklist": no_checklist, "updated": doc.get("updated", ""),
                    "tracks": tracks, "sessions": tasks}

        for name, fake in (("inbox", fake_inbox), ("GalleryScanner", FakeScanner),
                           ("progress_view", fake_progress), ("image_size", lambda p: (4, 3))):
            patcher = patch.object(home, name, fake)
            patcher.start()
            self.addCleanup(patcher.stop)

    def close_ui(self):
        self.wb._refresh.stop()
        self.wb._poll.stop()
        for win in self.engine.rootObjects():
            win.close()
        self.engine.deleteLater()
        QCoreApplication.sendPostedEvents(None, QEvent.Type.DeferredDelete)
        self.wb.deleteLater()
        QCoreApplication.sendPostedEvents(None, QEvent.Type.DeferredDelete)

    def tearDown(self):
        self.pump()
        self.assertEqual(self.warnings, [])

    def pump(self, n=5):
        for _ in range(n):
            self.app.processEvents()
        QCoreApplication.sendPostedEvents(None, QEvent.Type.DeferredDelete)

    def test_boards_live_view_notes_and_tools(self):
        self.assertTrue(self.engine.rootObjects(), self.warnings)
        self.pump()
        wb = self.wb
        self.assertEqual([s["sid"] for s in wb.sessions], [tw.SID])
        self.assertEqual(wb.sessions[0]["status"], "running")
        self.assertEqual(wb.sessions[0]["subagents"][0]["id"], "ag1")
        self.machine(0)
        self.assertTrue(wb.compact["step"].startswith("0.4.1-2"))
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
        self.assertEqual(written, ["events/" + tw.SID + ".jsonl", "home.json", "inbox/" + tw.SID + ".jsonl"])
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
        (self.fx.data / "runs").mkdir(parents=True, exist_ok=True)
        (self.fx.data / "runs" / f"{rid}.json").write_text(json.dumps(doc), encoding="utf-8")

    def click(self, name: str):
        self.qml_eval(name, "(function() { i.click(); return true; })()")
        self.pump()

    def item(self, name: str):
        root = self.engine.rootObjects()[0].property("contentItem")  # contentItem() can return a stale wrapper
        found = [i for i in items(root) if i.objectName() == name]
        self.assertEqual(len(found), 1, name)
        return found[0]

    def machine(self, index=0):
        self.item("mainTabs").setProperty("currentIndex", 1)
        self.item("machineTabs").setProperty("currentIndex", index)
        self.pump()

    def card(self, index=1, **fields):
        ask_id = f"20260930T120{index:03d}Z-{index:08x}"
        return {"id": f"ask:{ask_id}", "kind": "ask", "askKind": "decision", "blocking": False,
                "sid": tw.SID, "sessionTitle": "Fixture session", "t": time.time() - index, "age": "1m",
                "title": "Choose a layout", "context": "Two layouts are ready.", "detail": "Plain <text>.",
                "options": ["A", "B"], "attachments": [], "answerRef": f"ask {ask_id}", "state": "open",
                "needsOwner": True, "answer": "", "canOpenSession": True, "files": [], **fields}

    def gallery_item(self, name="a.png", **fields):
        root = self.fx.main / "docs" / "figures"
        root.mkdir(parents=True, exist_ok=True)
        path = root / name
        ext = path.suffix
        if ext == ".png":
            path.write_bytes(png(4, 3))
        elif ext == ".svg":
            path.write_text('<svg xmlns="http://www.w3.org/2000/svg" width="4" height="3"><rect width="4" height="3" fill="gray"/></svg>', encoding="utf-8")
        else:
            path.write_text("import marimo\napp = marimo.App()\n", encoding="utf-8")
        return {"path": str(path), "rel": path.relative_to(self.fx.main).as_posix(), "checkout": self.fx.main.name,
                "type": "image", "ext": ext, "mtime": path.stat().st_mtime, "size": path.stat().st_size,
                "preview": str(path) if ext in (".png", ".svg") else "", "pixels": [4, 3] if ext == ".png" else [],
                "source": str(path), "origins": ["figures"], "sessions": [tw.SID], "isNew": True, **fields}

    def show_cards(self, *cards):
        self.fake_cards = list(cards)
        self.wb.refresh()
        self.pump()

    def focus_card(self, card):
        self.wb.showCard(card["id"])
        self.pump(10)

    def note_texts(self, sid=tw.SID):
        path = self.fx.data / "inbox" / f"{sid}.jsonl"
        return [json.loads(line)["text"] for line in path.read_text(encoding="utf-8").splitlines()] if path.exists() else []

    def idle_session(self):
        self.fx.events(tw.SID, [tw.ev(14, "SubagentStop", agent_id="ag1"), tw.ev(15, "Stop")])
        self.wb.refresh()
        self.assertNotEqual(self.wb.sessions[0]["status"], "running")

    def compact_item(self, name):
        compact = next(w for w in QApplication.allWindows() if w.title() == "Workbench progress")
        found = [i for i in items(compact.property("contentItem")) if i.objectName() == name]
        self.assertEqual(len(found), 1, name)
        return found[0]

    def qml_property(self, name, prop):
        """Read inside QML when PySide returns a stale wrapper for a live header or Popup."""
        return self.qml_eval(name, f"i[{json.dumps(prop)}]")

    def qml_eval(self, name, expression):
        root = self.engine.rootObjects()[0]
        code = (f"(function() {{ function find(i) {{ if (i.objectName === {json.dumps(name)}) return {expression};"
                " var children = i.data || i.children || []; for (var k = 0; k < children.length; ++k) {"
                " var r = find(children[k]); if (r !== undefined) return r; }"
                " if (i.contentItem) return find(i.contentItem); }"
                " var value = find(root.contentItem); return value === undefined ? find(root.header) : value; })()")
        expression = QQmlExpression(self.engine.contextForObject(root), root, code)
        value, undefined = expression.evaluate()
        self.assertFalse(expression.hasError(), expression.error().toString())
        self.assertFalse(undefined, name)
        return value

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
        self.machine(3)
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
        self.machine(2)
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
        self.assertEqual(wb.home["machineBadge"], home.machine_badge(wb.flags))
        # Run and Stop use the registry entry as shown; nothing else can be started from here.
        self.click("run-demo")
        self.assertEqual(rec.calls[-1], ("start_run", str(fx.main), "demo", runs.entry_digest(runs.load_registry(fx.main).entries[0])))
        self.click("stop-" + live)
        self.assertEqual(rec.calls[-1], ("cancel_run", live))
        wb.startRun("demo", str(fx.outsider), "x")
        self.assertIn("Not started", wb.message)
        self.assertEqual(rec.calls[-1], ("cancel_run", live))
        # The attention list selects the failed run and follows its log.
        self.machine(5)
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
        self.machine(3)
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
        find = ("(function find(i) { if (i.objectName === 'compactCounts') return i.text;"
                " for (var k = 0; k < i.children.length; ++k) { var r = find(i.children[k]); if (r !== undefined) return r; }"
                " })(compactWindow.contentItem)")
        text, undefined = QQmlExpression(self.engine.contextForObject(root), root, find).evaluate()
        self.assertFalse(undefined)
        self.assertEqual(text, f"For you {wb.home['forYou']} · Gallery +{wb.home['galleryNew']}")
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

    def test_a_notification_in_compact_mode_opens_the_full_window_on_its_card(self):
        a, b = self.card(1), self.card(2)
        self.show_cards(a, b)
        engine = QQmlApplicationEngine()
        warnings = []
        engine.warnings.connect(lambda ws: warnings.extend(w.toString() for w in ws))
        self.assertTrue(self.wbapp.load_ui(engine, self.wb, compact=True))
        self.pump()
        self.assertEqual([w.title() for w in engine.rootObjects()], ["Workbench progress"])
        self.wbapp.open_card(engine, self.wb, b["id"])
        self.pump(10)
        full = [w for w in engine.rootObjects() if w.objectName() == "mainWindow"]
        self.assertEqual(len(full), 1)
        self.assertTrue(full[0].isVisible())
        self.assertTrue(engine.rootObjects()[0].isVisible())   # the compact window stays
        inbox = [i for i in items(full[0].property("contentItem")) if i.objectName() == "inboxList"]
        self.assertEqual(inbox[0].property("currentIndex"), 1)
        self.wbapp.open_card(engine, self.wb, a["id"])
        self.pump(10)
        self.assertEqual(len([w for w in engine.rootObjects() if w.objectName() == "mainWindow"]), 1)
        self.assertEqual(inbox[0].property("currentIndex"), 0)
        self.assertEqual(warnings, [])
        for win in engine.rootObjects():
            win.close()
        engine.deleteLater()

    def test_compact_view_from_the_main_window_is_top_level(self):
        main = self.engine.rootObjects()[0]
        compact = next(w for w in QApplication.allWindows() if w.title() == "Workbench progress")
        self.assertIsNone(compact.transientParent())  # stays visible when the main window is minimized
        self.assertFalse(compact.property("standalone"))
        compact.show()
        main.showMinimized()
        self.pump()
        self.assertTrue(compact.isVisible())
        compact.close()

    def test_home_is_default_and_header_counts_and_machine_badge(self):
        self.assertEqual(self.item("mainTabs").property("currentIndex"), 0)
        self.fake_items = [self.gallery_item()]
        card = self.card()
        wait = {"kind": "owner_wait", "target_kind": "session", "target_id": tw.SID, "text": "Owner needed",
                "severity": 0, "t": time.time()}
        with patch.object(tw.watch, "flags", return_value=[wait]):
            self.show_cards(card)
            self.assertEqual(self.wb.home["machineBadge"], 0)
            self.assertFalse(self.item("machineBadge").isVisible())
        self.assertEqual(self.qml_property("headerForYou", "text"), "For you 1")
        self.assertEqual(self.qml_property("headerGallery", "text"), "Gallery +1")
        self.assertEqual(self.qml_property("headerStep", "text"), "Step 0.4.1-2 0%")
        self.assertFalse(self.qml_property("headerApi", "visible"))
        failed = "20260930T110000Z-000000f1"
        self.record(failed, "failed", exits=[1], ended=iso_now(-60))
        self.wb.refresh()
        self.pump()
        self.assertGreater(self.wb.home["machineBadge"], 0)
        self.assertTrue(self.item("machineBadge").isVisible())
        self.show_cards()
        self.assertEqual(self.qml_property("headerForYou", "text"), "For you 0")

    def test_inbox_options_free_reply_and_pick_write_exact_session_notes(self):
        other = "dddddddd-0000-0000-0000-000000000002"
        self.fx.transcript(self.fx.main, other, [tw.user(0, "Other job", cwd=str(self.fx.main))])
        a, b = self.gallery_item("pick-a.png"), self.gallery_item("pick-b.png")
        attachments = [{"index": i + 1, "path": row["path"], "rel": row["rel"], "checkout": row["checkout"], "previewable": True}
                       for i, row in enumerate((a, b))]
        option = self.card(1)
        reply = self.card(2, sid=other, options=[])
        pick = self.card(3, askKind="pick", attachments=attachments,
                         options=[f"picked {i + 1}: {row['checkout']}/{row['rel']}" for i, row in enumerate((a, b))])
        self.show_cards(option, reply, pick)
        self.focus_card(option)
        self.click(f"option-{option['id']}-1")
        self.assertEqual(self.note_texts(), [home.answer_text(option["answerRef"], "B")])
        self.focus_card(reply)
        self.item(f"reply-{reply['id']}").setProperty("text", "Ship the blue layout")
        self.click(f"send-{reply['id']}")
        self.assertEqual(self.note_texts(other), [home.answer_text(reply["answerRef"], "Ship the blue layout")])
        self.focus_card(pick)
        self.click(f"pick-{pick['id']}-1")
        self.assertEqual(self.note_texts(), [home.answer_text(option["answerRef"], "B"), home.answer_text(pick["answerRef"], pick["options"][1])])
        self.assertEqual(self.scan_calls[-1]["asks"][-1]["attachments"], [{"path": a["path"]}, {"path": b["path"]}])

    def test_an_unsent_reply_survives_inbox_changes(self):
        reply = self.card(2, options=[])
        self.show_cards(reply)
        self.focus_card(reply)
        self.qml_eval(f"reply-{reply['id']}", '(function() { i.text = "Half a reply"; i.textEdited(); return true })()')
        before = self.item(f"reply-{reply['id']}")
        self.show_cards(self.card(1), {**reply, "age": "2 min"})  # a new ask and a new age rebuild the delegates
        self.focus_card(reply)
        field = self.item(f"reply-{reply['id']}")
        self.assertIsNot(field, before)
        self.assertEqual(field.property("text"), "Half a reply")
        self.click(f"send-{reply['id']}")
        self.assertEqual(self.note_texts(), [home.answer_text(reply["answerRef"], "Half a reply")])
        self.assertEqual(self.qml_eval("inboxList", "JSON.stringify(homeView.drafts)"), "{}")

    def test_a_card_is_answered_once(self):
        option = self.card(1)
        pick = self.card(2, askKind="pick", options=["picked 1: a.png"],
                         attachments=[{"index": 1, "path": self.gallery_item()["path"], "rel": "docs/figures/a.png",
                                       "checkout": self.fx.main.name, "previewable": True}])
        self.show_cards(option, pick)
        for card, button in ((option, f"option-{option['id']}-0"), (pick, f"pick-{pick['id']}-0")):
            with self.subTest(card=card["askKind"]):
                self.focus_card(card)
                self.click(button)
                self.assertEqual(next(c for c in self.wb.inbox if c["id"] == card["id"])["state"], "queued")
                self.focus_card(card)
                self.assertFalse(self.item(button).property("enabled"))
                self.assertFalse(self.wb.answerCard(card["id"], card["options"][0]))
                self.assertEqual(self.wb.message, "That card is already answered; the answer is queued.")
        self.assertEqual(self.note_texts(), [home.answer_text(option["answerRef"], "A"),
                                             home.answer_text(pick["answerRef"], "picked 1: a.png")])

    def test_wait_cards_cannot_reply_and_open_in_claude(self):
        wait = self.card(kind="wait", askKind="", id=f"wait:{tw.SID}:main:012345abcdef", answerRef="", options=[],
                         title="Permission needed", detail="Bash: git status")
        self.show_cards(wait)
        self.focus_card(wait)
        self.assertFalse(self.item(f"reply-{wait['id']}").isVisible())
        self.assertFalse(self.item(f"send-{wait['id']}").isVisible())
        self.assertFalse(self.wb.answerCard(wait["id"], "yes"))
        self.assertEqual(self.note_texts(), [])
        self.click(f"open-{wait['id']}")
        self.assertEqual(self.recorder.calls[-1][0], "claude")
        self.assertEqual(self.wb.selected["id"], tw.SID)

    def test_answered_is_collapsed_and_queued_idle_hint_is_visible(self):
        self.idle_session()
        queued = self.card(1, blocking=True)
        sent = self.card(2, state="sent", needsOwner=False, answer="A")
        acknowledged = self.card(3, state="acknowledged", needsOwner=False, answer="B")
        self.show_cards(queued, sent, acknowledged)
        self.assertFalse(self.item("answeredToggle").property("checked"))
        self.assertFalse(self.item("answered-" + sent["id"]).isVisible())
        self.focus_card(queued)
        self.click(f"option-{queued['id']}-0")
        self.assertIn("The session is idle", self.wb.message)
        self.focus_card(queued)
        hint = self.item("queued-" + queued["id"])
        self.assertTrue(hint.isVisible())
        self.assertIn("open it in Claude Code and send any message", hint.property("text"))
        self.assertEqual(self.wb.home["forYou"], 1)
        self.click("answeredToggle")
        self.assertTrue(self.item("answered-" + sent["id"]).isVisible())
        texts = [i.property("text") for i in items(self.engine.rootObjects()[0].property("contentItem"))]
        self.assertIn("sent, delivery not confirmed", texts)
        self.assertIn("acknowledged by Claude", texts)
        self.assertIn("Blocking", texts)

    def test_gallery_filters_preview_open_and_producing_session_notes(self):
        other = "dddddddd-0000-0000-0000-000000000002"
        self.fx.transcript(self.fx.main, other, [tw.user(0, "Other gallery job", cwd=str(self.fx.main))])
        vector = self.gallery_item("v.svg", type="vector", isNew=False)
        image = self.gallery_item(source=vector["path"])
        orphan = self.gallery_item("orphan.png", sessions=[], isNew=False)
        huge = self.gallery_item("huge.png", preview="", pixels=[10000, 10000])
        notebook = self.gallery_item("demo.py", type="notebook", sessions=[other])
        self.fake_items = [image, vector, orphan, huge, notebook]
        self.wb.refresh()
        self.pump()
        self.assertEqual(self.item("galleryGrid").property("count"), 5)
        self.item("galleryType").setProperty("currentIndex", 1)
        self.pump()
        self.assertEqual(self.item("galleryGrid").property("count"), 3)
        self.choose("gallerySession", tw.SID)
        self.assertEqual(self.item("galleryGrid").property("count"), 2)
        self.item("galleryType").setProperty("currentIndex", 0)
        self.choose("gallerySession", other)
        self.assertEqual(self.item("galleryGrid").property("count"), 1)
        self.choose("gallerySession", "")
        self.click("galleryTile-a.png")
        self.assertTrue(self.qml_property("galleryPreview", "visible"))
        self.assertEqual(self.qml_property("galleryOpen", "text"), "Open")
        self.click("galleryOpen")
        self.assertEqual(self.recorder.calls[-1], ("open", ("default", image["path"])))
        self.assertTrue(self.qml_property("galleryOpenSource", "visible"))
        self.click("galleryOpenSource")
        self.assertEqual(self.recorder.calls[-1], ("open", ("browser", vector["path"])))
        self.click("galleryStar")
        self.qml_eval("galleryComment", 'i.text = "Use the first draft"')
        self.click("galleryWrong")
        self.click("galleryCommentSend")
        label = f"{image['checkout']}/{image['rel']}"
        self.assertEqual(self.note_texts(), [f"star: {label}", f"wrong direction: {label}: Use the first draft",
                                          f"comment on {label}: Use the first draft"])
        self.click("galleryClose")
        self.click("galleryTile-orphan.png")
        for name in ("galleryStar", "galleryWrong", "galleryComment", "galleryCommentSend"):
            self.assertFalse(self.qml_property(name, "enabled"), name)
        self.assertFalse(self.wb.fileNote(orphan["path"], "star", "", ""))
        self.assertEqual(self.wb.message, "No session produced this file.")
        self.click("galleryClose")
        self.assertIn("10000×10000, too large to preview", self.item("placeholder-huge.png").property("text"))
        self.assertTrue(self.item("placeholder-huge.png").isVisible())
        self.item("galleryType").setProperty("currentIndex", 7)
        self.pump()
        self.click("galleryTile-demo.py")
        self.assertTrue(self.qml_property("galleryMarimo", "visible"))
        self.click("galleryMarimo")
        self.assertEqual(self.recorder.calls[-1][0], "command")
        self.assertEqual(self.recorder.calls[-1][1][1:], ["edit", notebook["path"]])

    def test_gallery_notes_go_to_the_session_the_owner_chooses(self):
        later = "dddddddd-0000-0000-0000-000000000002"
        self.fx.transcript(self.fx.main, later, [tw.user(0, "Later reviewer", cwd=str(self.fx.main))])
        image = self.gallery_item(sessions=[later, tw.SID])   # the later reference is listed first
        self.fake_items = [image]
        self.wb.refresh()
        self.pump()
        label = f"{image['checkout']}/{image['rel']}"
        self.choose("gallerySession", tw.SID)
        self.click("galleryTile-a.png")
        self.assertTrue(self.qml_property("galleryRecipient", "visible"))
        self.assertEqual(self.qml_property("galleryRecipient", "currentValue"), tw.SID)
        self.click("galleryStar")
        self.assertEqual((self.note_texts(), self.note_texts(later)), ([f"star: {label}"], []))
        self.qml_eval("galleryRecipient", "(function() { i.currentIndex = 0; i.activated(0); return true })()")
        self.qml_eval("galleryComment", 'i.text = "Compare with the first draft"')
        self.click("galleryCommentSend")
        self.assertEqual(self.note_texts(later), [f"comment on {label}: Compare with the first draft"])
        self.click("galleryClose")
        self.choose("gallerySession", "")
        self.click("galleryTile-a.png")
        self.assertEqual(self.qml_property("galleryRecipient", "currentValue"), later)
        self.click("galleryClose")
        single = self.gallery_item("b.png", sessions=[tw.SID])
        self.fake_items = [image, single]
        self.wb.refresh()
        self.pump()
        self.click("galleryTile-b.png")
        self.assertFalse(self.qml_property("galleryRecipient", "visible"))
        self.click("galleryStar")
        self.assertEqual(self.note_texts()[-1], f"star: {single['checkout']}/{single['rel']}")

    def test_gallery_session_filter_survives_session_activity(self):
        other = "dddddddd-0000-0000-0000-000000000002"
        self.fx.transcript(self.fx.main, other, [tw.user(0, "Other gallery job", cwd=str(self.fx.main))])
        self.fake_items = [self.gallery_item(), self.gallery_item("demo.py", type="notebook", sessions=[other])]
        self.wb.refresh()
        self.pump()
        self.choose("gallerySession", other)
        self.assertEqual(self.item("galleryGrid").property("count"), 1)
        path = next(self.fx.projects.glob("*/" + tw.SID + ".jsonl"))
        with path.open("a", encoding="utf-8") as f:
            f.write(json.dumps(tw.result(4, "u2", "OK")) + "\n")
        third = "dddddddd-0000-0000-0000-000000000003"
        self.fx.transcript(self.fx.main, third, [tw.user(5, "A new session", cwd=str(self.fx.main))])
        sessions_before = list(self.wb.gallerySessions)
        self.wb.refresh()
        self.pump()
        self.assertNotEqual(self.wb.gallerySessions, sessions_before)
        self.assertEqual(self.item("gallerySession").property("currentValue"), other)
        self.assertEqual(self.item("galleryGrid").property("count"), 1)
        self.wb.refresh()   # nothing changed: no model reset
        self.pump()
        self.assertEqual(self.item("galleryGrid").property("count"), 1)

    def test_gallery_policy_is_cached_and_rescan_is_forced(self):
        image = self.gallery_item("hash #1.png")
        self.fake_items = [image]
        with patch.object(tw.launch, "plan_open", wraps=tw.launch.plan_open) as plan:
            self.wb.refresh()
            self.pump()
            self.wb.refresh()
            self.assertEqual(plan.call_count, 1)
            self.click("galleryRescan")
            self.assertTrue(self.scan_calls[-1]["force"])
            self.assertEqual(plan.call_count, 1)
            self.fake_items[0] = {**image, "mtime": image["mtime"] + 1}
            self.wb.refresh()
            self.assertEqual(plan.call_count, 2)
        self.assertEqual(self.wb.gallery[0]["sessionTitles"], [self.wb.sessions[0]["title"]])
        self.click("galleryTile-hash #1.png")
        self.assertTrue(self.qml_property("galleryPreview", "visible"))

    def test_progress_bars_remaining_chips_tasks_and_no_checklist(self):
        doc = sample_status()
        self.assertAlmostEqual(self.item("track-0.4.1").property("value"), checklist.track_percent(doc, "0.4.1"))
        self.assertEqual(self.item("percent-0.4.1").property("text"), "8%")
        self.assertEqual(self.item("current-0.4.1").property("value"), 0)
        texts = [i.property("text") for i in items(self.engine.rootObjects()[0].property("contentItem"))]
        self.assertIn("\u25cb HARNESS  (weight 3)", texts)
        self.assertIn("\u25cb RUNS", texts)
        tasks = [{"content": f"Task {i}", "status": "completed" if i < 3 else "pending"} for i in range(7)]
        path = next(self.fx.projects.glob("*/" + tw.SID + ".jsonl"))
        with path.open("a", encoding="utf-8") as f:
            f.write(json.dumps(tw.assistant(50, uses=[("t3", "TodoWrite", {"todos": tasks})])) + "\n")
            f.write(json.dumps(tw.result(51, "t3")) + "\n")
        self.wb.refresh()
        self.pump()
        self.assertEqual(self.item("tasks-" + tw.SID).property("text"), "3/7")
        doc["schema"] = 1
        for step in doc["steps"]:
            step.pop("items")
        (self.fx.main / "docs" / "progress" / "status.json").write_text(checklist.dump(doc), encoding="utf-8")
        self.wb.refresh()
        self.pump()
        self.assertEqual(self.item("percent-0.4.1").property("text"), "no checklist")
        self.assertFalse(self.item("track-0.4.1").isVisible())
        self.assertEqual(self.wb.home["stepText"], "Step 0.4.1-2 no checklist")

    def test_session_owner_buttons_and_idle_resume(self):
        self.machine(0)
        self.wb.selectSession(tw.SID)
        self.pump()
        for button in ("pauseSession", "resumeSession", "whySession"):
            self.assertTrue(self.item(button).isVisible())
            self.click(button)
        self.click("prioritySession")
        self.assertTrue(self.item("priorityText").isVisible())
        self.item("priorityText").setProperty("text", "ship A first")
        self.click("sendPriority")
        self.assertEqual(self.note_texts(), ["[pause]", "[resume]", "[why]", "[priority] ship A first"])
        self.assertEqual(self.recorder.calls, [])
        self.idle_session()
        self.click("resumeSession")
        self.assertEqual(self.recorder.calls, [("claude",)])
        self.assertIn("the session reads [resume] at its first tool step", self.wb.message)
        self.assertEqual(self.note_texts()[-1], "[resume]")

    def test_compact_counts_and_api_visibility(self):
        self.fake_items = [self.gallery_item()]
        self.show_cards(self.card())
        compact = next(w for w in QApplication.allWindows() if w.title() == "Workbench progress")
        compact.show()
        self.pump()
        self.assertEqual(self.compact_item("compactCounts").property("text"), "For you 1 · Gallery +1")
        self.assertEqual(self.compact_item("compactStep").property("text"), "Step 0.4.1-2 0%")
        self.assertFalse(self.compact_item("compactApi").isVisible())
        empty = {"billed": 0, "estimated": 0, "reserved": 0, "unknown_calls": 0, "subscription_calls": 9, "incomplete": False}
        for field, value in (("subscription_calls", 9), ("billed", 1), ("estimated", 1), ("reserved", 1),
                             ("unknown_calls", 1), ("incomplete", True)):
            with patch.object(self.wb._ledgers, "budget", return_value={**empty, field: value}):
                self.wb.refresh()
                self.pump()
                visible = field != "subscription_calls"
                self.assertEqual(self.compact_item("compactApi").isVisible(), visible, field)
                self.assertEqual(self.qml_property("headerApi", "visible"), visible, field)
        compact.close()

    def test_snapshot_is_private_and_throttled_until_counts_change_or_sixty_seconds(self):
        path = self.fx.data / "home.json"
        snap = json.loads(path.read_text(encoding="utf-8"))
        self.assertEqual((snap["schema"], snap["for_you"], snap["gallery_new"]), (1, 0, 0))
        os.utime(path, ns=(1_000_000_000, 1_000_000_000))
        before = path.stat().st_mtime_ns
        self.wb.refresh()
        self.assertEqual(path.stat().st_mtime_ns, before)
        self.show_cards(self.card())
        self.assertNotEqual(path.stat().st_mtime_ns, before)
        self.assertEqual(json.loads(path.read_text(encoding="utf-8"))["for_you"], 1)
        os.utime(path, ns=(1_000_000_000, 1_000_000_000))
        later = self.wb._snapshot_at + 61
        with patch.object(self.wbapp.time, "time", return_value=later):
            self.wb.refresh()
        self.assertNotEqual(path.stat().st_mtime_ns, before)
        self.assertEqual(json.loads(path.read_text(encoding="utf-8")), home.snapshot({"for_you": 1, "gallery_new": 0}, later))
        self.assertEqual(list(self.fx.data.glob("home-*.tmp")), [])

    def test_snapshot_failure_is_reported_once_and_keeps_last_snapshot(self):
        path = self.fx.data / "home.json"
        before = path.read_bytes()
        self.fake_cards = [self.card()]
        with patch.object(self.wbapp.os, "replace", side_effect=OSError("fixture failure")), patch.object(self.wb, "_say", wraps=self.wb._say) as say:
            self.wb.refresh()
            self.wb.refresh()
            self.assertEqual(say.call_count, 1)
            self.assertIn("Could not save Home counts", self.wb.message)
        self.assertEqual(path.read_bytes(), before)
        self.assertEqual(list(self.fx.data.glob("home-*.tmp")), [])

    def test_owner_slots_reject_invalid_input_and_note_failures(self):
        card = self.card()
        self.show_cards(card)
        self.assertFalse(self.wb.answerCard("missing", "A"))
        self.assertFalse(self.wb.answerCard(card["id"], "\n"))
        self.assertFalse(self.wb.sessionCommand("pause", ""))
        self.wb.selectSession(tw.SID)
        self.assertFalse(self.wb.sessionCommand("other", ""))
        self.assertFalse(self.wb.sessionCommand("priority", " "))
        image = self.gallery_item()
        self.fake_items = [image]
        self.wb.refresh()
        self.assertFalse(self.wb.fileNote(image["path"], "comment", " ", tw.SID))
        self.assertFalse(self.wb.fileNote(image["path"], "other", "", tw.SID))
        self.assertFalse(self.wb.fileNote(image["path"], "star", "", "dddddddd-0000-0000-0000-00000000000f"))
        self.assertEqual(self.wb.message, "Choose one of the sessions listed for this file.")
        with patch.object(tw.notes, "append_note", side_effect=tw.notes.NoteRejected("fixture rejection")):
            self.assertFalse(self.wb.answerCard(card["id"], "A"))
            self.assertFalse(self.wb.fileNote(image["path"], "star", "", tw.SID))
            self.assertFalse(self.wb.sessionCommand("pause", ""))
        self.assertEqual(self.note_texts(), [])

    def test_notifier_seeds_groups_deduplicates_and_click_focuses_home(self):
        class Tray(QObject):
            messageClicked = Signal()

            def __init__(self):
                super().__init__()
                self.messages = []

            def showMessage(self, *args):
                self.messages.append(args)

        a, b, c = self.card(1), self.card(2), self.card(3)
        tray = Tray()
        notifier = self.wbapp.Notifier(tray=tray)
        opened = []
        notifier.openCard.connect(opened.append)
        notifier.update([a])
        self.assertEqual(tray.messages, [])
        notifier.update([b, c, a])
        self.assertEqual(len(tray.messages), 1)
        self.assertEqual(tray.messages[0][:2], ("New for you", "2 new items for you"))
        notifier.update([b, c, a])
        notifier.update([])
        notifier.update([b, a])
        self.assertEqual(len(tray.messages), 1)
        tray.messageClicked.emit()
        self.assertEqual(opened, [b["id"]])
        self.show_cards(a, b, c)
        self.machine()
        notifier.openCard.connect(self.wb.showCard)
        tray.messageClicked.emit()
        self.pump(10)
        self.assertEqual(self.item("mainTabs").property("currentIndex"), 0)
        self.assertEqual(self.item("inboxList").property("currentIndex"), 1)
        d = self.card(4, needsOwner=False)
        notifier.update([a, b, c, d])
        notifier.update([a, b, c, {**d, "needsOwner": True}])
        self.assertEqual(len(tray.messages), 1)
        e = self.card(5, title="A single new decision")
        notifier.update([a, b, c, d, e])
        self.assertEqual(tray.messages[-1][:2], ("New for you", e["title"]))

    def test_notifier_without_tray_alerts_the_real_quick_window(self):
        window = self.engine.rootObjects()[0]
        self.assertIsInstance(window, QQuickWindow)

        class WindowSpy:
            def __init__(self):
                self.alerts = []

            def alert(self, ms):
                self.alerts.append(ms)
                window.alert(ms)

        spy = WindowSpy()
        notifier = self.wbapp.Notifier(window=spy)
        notifier.update([])
        self.assertEqual(spy.alerts, [])
        a = self.card()
        notifier.update([a])
        notifier.update([a])
        self.assertEqual(spy.alerts, [0])


class RealHomeUiTests(WorkbenchUiTests):
    """The Home tab end to end on the real readers: ask.py writes, home reads, the UI answers, ask.py wait sees it."""

    def patch_home(self):
        import ask
        gallery = self.fx.wt / "work" / "gallery"
        gallery.mkdir(parents=True)
        for name in ("a.png", "b.png"):
            (gallery / name).write_bytes(png(4, 3))
        cwd = Path.cwd()
        os.chdir(self.fx.wt)
        try:
            out = io.StringIO()
            with contextlib.redirect_stdout(out):
                code = ask.main(["--question", "Which colouring?", "--kind", "pick", "--blocking", "--session", tw.SID,
                                 "--attach", str(gallery / "a.png"), "--attach", str(gallery / "b.png")])
        finally:
            os.chdir(cwd)
        self.assertEqual(code, 0)
        self.ask_id = out.getvalue().split()[1]

    def test_real_readers_fill_home_and_the_answer_reaches_ask_wait(self):
        import ask
        self.pump()
        card_id = "ask:" + self.ask_id
        self.assertEqual([c["id"] for c in self.wb.inbox], [card_id])
        self.assertTrue(self.item("card-" + card_id).isVisible())
        self.assertEqual(self.qml_property("headerForYou", "text"), "For you 1")
        self.assertEqual(self.item("galleryGrid").property("count"), 2)
        self.assertEqual(sorted(self.wb.gallery[0]["origins"]), ["ask-attachment", "gallery"])
        self.assertEqual(self.item("percent-0.4.1").property("text"), "8%")
        self.click(f"pick-{card_id}-0")
        self.assertEqual(self.note_texts(), [home.answer_text("ask " + self.ask_id, f"picked 1: {self.fx.wt.name}/work/gallery/a.png")])
        self.wb.refresh()
        self.pump()
        self.assertEqual(self.wb.inbox[0]["state"], "queued")
        cwd = Path.cwd()
        os.chdir(self.fx.wt)
        try:
            with contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(ask.main(["wait", self.ask_id, "--timeout", "1", "--session", tw.SID]), 0)
        finally:
            os.chdir(cwd)


for _name in [n for n in vars(WorkbenchUiTests) if n.startswith("test_")]:
    setattr(RealHomeUiTests, _name, None)   # run only its own test; unittest skips non-callables


if __name__ == "__main__":
    unittest.main(verbosity=2)
