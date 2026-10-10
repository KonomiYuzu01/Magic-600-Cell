"""Offscreen rating-window interactions on a real temporary Store and synthetic images.

No model files, content screen or network are used. Training uses a detached
snapshot and delivers its completion in the UI thread.
"""
from __future__ import annotations

import hashlib
import io
import os
import shutil
import sys
import tempfile
import threading
import time
import unittest
from dataclasses import replace
from pathlib import Path
from unittest import mock
from uuid import uuid4

# Both must be set before importing any PySide6 module.
os.environ["QT_QPA_PLATFORM"] = "offscreen"
os.environ["QML_DISABLE_DISK_CACHE"] = "1"
ROOT = Path(__file__).resolve().parents[1]
sys.dont_write_bytecode = True
sys.path.insert(0, str(ROOT / "tools"))

try:
    import shiboken6
    from PySide6.QtCore import QEvent, QMimeData, QPoint, QPointF, Qt, QTimer, QUrl
    from PySide6.QtGui import QDragEnterEvent, QDropEvent, QGuiApplication, QKeyEvent
    from PySide6.QtQml import QQmlApplicationEngine, QQmlExpression
    from PySide6.QtQuick import QQuickWindow  # noqa: F401 (register the QML window wrapper)
    from PySide6.QtQuickControls2 import QQuickStyle
    from PySide6.QtTest import QTest
    HAVE_QT = True
except ImportError:
    HAVE_QT = False

try:
    from PIL import Image
    HAVE_PIL = True
except ImportError:
    HAVE_PIL = False

from tastelab import common, learn, store  # noqa: E402


class Fixture:
    def __init__(self):
        self.tmp = Path(tempfile.gettempdir()) / f"tastelab-test-{uuid4().hex}"
        # Inherit the temporary folder's ACL, including the Windows sandbox SID.
        # CPython 3.14's TemporaryDirectory mode 0700 drops that SID.
        self.tmp.mkdir()
        self.guard = mock.patch.object(common, "PRIVATE_BASE", self.tmp)
        self.guard.start()
        self.root = self.tmp / "data"
        self.store = store.Store(self.root)
        self.shas = [self.add_image(i) for i in range(8)]

    def add_image(self, index, *, db=None, tier="A"):
        db = self.store if db is None else db
        image = Image.new("RGB", (128, 96), (index * 17 % 256, index * 31 % 256, 100))
        original, thumb = io.BytesIO(), io.BytesIO()
        image.save(original, format="PNG")
        image.resize((64, 48)).save(thumb, format="JPEG")
        sha = hashlib.sha256(original.getvalue()).hexdigest()
        meta = store.ImageMeta(sha, "synthetic", "private-reference" if tier == "B" else "CC0-1.0",
                               "synthetic fixture", tier, "test",
                               128, 96, title=f"Image {index}")
        assert db.add_image(meta, thumb.getvalue(), original.getvalue(), "png") == "stored"
        return sha

    def close(self):
        self.store.close()
        self.guard.stop()
        shutil.rmtree(self.tmp, ignore_errors=True)


class FakeModel:
    def __init__(self, db):
        self.db = db
        self.enabled = False
        self.current = learn.ModelState(0, 0, 0, False, None, None, False, False)
        self.applied = None
        self.snapshots, self.trains, self.applies = [], [], []
        self.started = threading.Event()
        self.release = None
        self.error = None

    def signature(self):
        return tuple(r.id for r in self.db.ratings())

    def due(self):
        return self.enabled and self.signature() != self.applied

    def state(self):
        return self.current

    def snapshot(self):
        snap = (self.signature(), self.db.counts(), self.current)
        self.snapshots.append((threading.get_ident(), snap))
        return snap

    def train(self, data):
        self.trains.append(threading.get_ident())
        self.started.set()
        if self.release is not None:
            self.release.wait(5)
        if self.error is not None:
            raise self.error
        return data

    def apply(self, fit):
        self.applies.append(threading.get_ident())
        self.applied, counts, state = fit
        self.current = replace(state, ratings=counts["total"], likes=counts["like"], dislikes=counts["dislike"])
        return self.current


class FakeSelector:
    def __init__(self, db, shas):
        self.db, self.shas = db, list(shas)
        self.calls, self.shown_shas = [], []

    def next(self, n, exclude=()):
        self.calls.append((n, list(exclude)))
        rated = self.db.rated()
        return [s for s in self.shas if s not in exclude and s not in rated and self.db.has_image(s)][:n]

    def shown(self, sha):
        self.shown_shas.append(sha)


class Runner:
    """Usually synchronous; deferred mode lets the UI run before a controlled completion."""

    def __init__(self):
        self.defer = False
        self.pending = []

    def __call__(self, fn, on_done):
        if self.defer:
            self.pending.append((fn, on_done))
        else:
            self.run(fn, on_done)

    @staticmethod
    def run(fn, on_done):
        try:
            result = fn()
        except Exception as exc:
            result = exc
        on_done(result)

    def complete(self):
        self.run(*self.pending.pop(0))


class Proposals:
    def __init__(self):
        self.enabled = False
        self.calls = []
        self.items = [("brass observatory", "test", "adjacent"), ("quiet geometry", None, "probe")]

    def due(self, db):
        return self.enabled and db.counts()["total"] > db.last_proposal_rating()

    def propose(self, db, model, plan, vectors):
        self.calls.append((plan, vectors))
        count = db.counts()["total"]
        db.add_proposals(self.items, count)
        db.mark_proposal_round(count)
        return self.items


@unittest.skipUnless(HAVE_QT and HAVE_PIL, "PySide6 and Pillow are required")
class RateUiTests(unittest.TestCase):
    def test_rating_keys_ignore_auto_repeat_in_qml_and_controller(self):
        for key in (Qt.Key_Right, Qt.Key_Up, Qt.Key_Left, Qt.Key_Down):
            with self.subTest(key=key):
                before = self.controller.total
                self.key(key)
                self.assertEqual(self.controller.total, before + 1)
                current, total = self.controller.image, self.controller.total
                repeated = QKeyEvent(QEvent.KeyPress, key, Qt.NoModifier, "", True)
                self.app.sendEvent(self.window, repeated)
                self.pump()
                self.assertEqual(self.controller.image, current)
                self.assertEqual(self.controller.total, total)
                self.assertTrue(self.controller.key(key, Qt.NoModifier.value, True))
                self.assertEqual(self.controller.total, total)

    def test_backspace_auto_repeat_never_undoes(self):
        self.key(Qt.Key_Right)
        self.key(Qt.Key_Right)
        total = self.controller.total
        repeated = QKeyEvent(QEvent.KeyPress, Qt.Key_Backspace, Qt.NoModifier, "", True)
        self.app.sendEvent(self.window, repeated)
        self.pump()
        self.assertEqual(self.controller.total, total)
        self.assertTrue(self.controller.key(Qt.Key_Backspace, Qt.NoModifier.value, True))
        self.assertEqual(self.controller.total, total)
        self.key(Qt.Key_Backspace)
        self.assertEqual(self.controller.total, total - 1)

    def test_delayed_image_load_ignores_removed_blocked_or_stale_image(self):
        current, queued = self.fx.shas[:2]
        for name in ("mainImage", "preload-0", "preload-1"):
            item = self.item(name)
            self.until(lambda: self.value(item, "Number(status)") == 1)
        self.assertTrue(self.controller.imageLoaded(self.url(current)))
        for sha in (current, queued):
            url = self.url(sha)
            self.fx.store.remove_image(sha)
            before = self.fx.store.db.total_changes
            shown = list(self.selector.shown_shas)
            self.assertFalse(self.controller.imageLoaded(url))
            self.assertEqual(self.fx.store.db.total_changes, before)
            self.assertEqual(self.selector.shown_shas, shown)
        self.key(Qt.Key_Down)
        self.assertFalse(self.controller.imageLoaded(self.url(current)))

    def test_delayed_rating_rechecks_removed_or_blocked_current_image(self):
        for blocked in (False, True):
            with self.subTest(blocked=blocked):
                sha = self.controller._current
                if blocked:
                    with self.fx.store.db:
                        self.fx.store.db.execute("INSERT INTO blocked VALUES (?, ?)", (sha, common.now_iso()))
                else:
                    self.fx.store.remove_image(sha)
                before = self.fx.store.db.total_changes
                self.assertTrue(self.controller.key(Qt.Key_Right, Qt.NoModifier.value))
                self.assertEqual(self.fx.store.db.total_changes, before)
                self.assertEqual(self.fx.store.ratings(), [])
                self.controller._advance()

    def test_queued_removed_image_is_never_shown_or_preloaded(self):
        sha = self.fx.shas[1]
        url = self.url(sha)
        self.fx.store.remove_image(sha)
        self.assertNotIn(url, self.controller.preload)
        self.key(Qt.Key_Down)
        self.assertEqual(self.controller.image, self.url(self.fx.shas[2]))
        self.assertNotIn(sha, self.selector.shown_shas)

    def test_show_ignores_missing_or_blocked_image_without_side_effects(self):
        sha = self.fx.shas[1]
        self.fx.store.remove_image(sha)
        current = self.controller.image
        shown = list(self.selector.shown_shas)
        self.controller._show(sha)
        self.assertEqual(self.controller.image, current)
        self.assertEqual(self.selector.shown_shas, shown)

    def test_rating_window_rates_class_b_locally_without_pick_feature(self):
        sha = self.fx.add_image(20, tier="B")
        self.selector.shas.append(sha)
        self.controller._show(sha)
        self.assertIn("tier B", self.controller.caption)
        self.key(Qt.Key_Left)
        self.assertEqual(self.fx.store.ratings()[-1].sha256, sha)
        self.assertEqual(self.fx.store.ratings()[-1].verdict, "dislike")
        for name in ("paste", "drop", "_picked", "_pick"):
            self.assertFalse(hasattr(self.controller, name), name)

    def test_delayed_training_completion_after_shutdown_does_nothing(self):
        self.runner.defer = True
        self.model.enabled = True
        self.key(Qt.Key_Right)
        self.assertEqual(len(self.runner.pending), 1)
        self.controller.shutdown()
        self.runner.complete()
        self.assertEqual(self.model.applies, [])

    @classmethod
    def setUpClass(cls):
        from tastelab import rate
        cls.rate = rate
        QQuickStyle.setStyle("Fusion")
        cls.app = QGuiApplication.instance() or QGuiApplication([])

    def setUp(self):
        self.fx = Fixture()
        self.model = FakeModel(self.fx.store)
        self.selector = FakeSelector(self.fx.store, self.fx.shas)
        self.runner = Runner()
        self.proposer = Proposals()
        self.warnings = []
        self.controller = self.engine = self.window = None
        self.build()

    def tearDown(self):
        if self.model.release is not None:
            self.model.release.set()
        self.dispose()
        self.fx.close()
        self.assertEqual(self.warnings, [])

    def dispose(self):
        if self.controller is not None:
            self.controller.shutdown()
        if self.window is not None:
            self.window.close()
        if self.engine is not None:
            shiboken6.delete(self.engine)
        self.controller = self.engine = self.window = None
        self.app.processEvents()

    def build(self, *, run_async="fake"):
        self.controller = self.rate.RateController(
            self.fx.store, self.model, self.selector, session="test-session",
            plan="fake plan", probe_vectors={"quiet": (1.0, 0.0)},
            proposals_due=self.proposer.due, propose=self.proposer.propose,
            run_async=self.runner if run_async == "fake" else run_async)
        self.engine = QQmlApplicationEngine()
        self.engine.warnings.connect(lambda ws: self.warnings.extend(w.toString() for w in ws))
        self.assertTrue(self.rate.load_ui(self.engine, self.controller), self.warnings)
        (self.window,) = self.engine.rootObjects()
        self.pump()

    def pump(self):
        QTest.qWait(10)
        self.app.processEvents()

    def until(self, predicate, timeout=1500):
        deadline = time.monotonic() + timeout / 1000
        while not predicate() and time.monotonic() < deadline:
            self.pump()
        self.assertTrue(predicate())

    def item(self, name):
        stack = [self.window.contentItem()]
        found = []
        while stack:
            item = stack.pop()
            if not shiboken6.isValid(item):
                continue
            if item.objectName() == name:
                found.append(item)
            stack.extend(item.childItems())
        self.assertEqual(len(found), 1, name)
        return found[0]

    def key(self, key, modifiers=None):
        modifiers = Qt.NoModifier if modifiers is None else modifiers
        QTest.keyClick(self.window, key, modifiers)
        self.pump()

    def value(self, item, expression):
        expr = QQmlExpression(self.engine.contextForObject(item), item, expression)
        value, undefined = expr.evaluate()
        self.assertFalse(expr.hasError(), expr.error().toString())
        self.assertFalse(undefined)
        return value

    def text(self, text):
        for char in text:
            QTest.keyClick(self.window, ord(char))
        self.pump()

    def click(self, name):
        item = self.item(name)
        pos = item.mapToScene(QPointF(item.width() / 2, item.height() / 2)).toPoint()
        QTest.mouseClick(self.window, Qt.LeftButton, Qt.NoModifier, pos)
        self.pump()

    def paste(self, text):
        self.app.clipboard().setText(text)
        self.key(Qt.Key_V, Qt.ControlModifier)

    def url(self, sha):
        return QUrl.fromLocalFile(str(self.fx.store.display_path(sha))).toString()

    def test_verdict_keys_counts_caption_and_decoded_preloads(self):
        c = self.controller
        self.assertEqual(self.window.title(), "Taste Lab")
        self.assertEqual(c.image, self.url(self.fx.shas[0]))
        self.assertEqual(c.preload, [self.url(s) for s in self.fx.shas[1:3]])
        self.assertEqual(self.selector.shown_shas, [self.fx.shas[0]])
        self.assertEqual(self.selector.calls[0], (3, []))
        self.assertEqual(c.caption, "synthetic · CC0-1.0 · tier A · Image 0")
        self.assertEqual(self.item("caption").property("text"), c.caption)
        self.assertEqual(c.headerText, "Today 0 · Total 0 · Loved 0 · AUC n/a")
        main = self.item("mainImage")
        self.until(lambda: self.value(main, "Number(status)") == 1)
        for i in range(2):
            preload = self.item(f"preload-{i}")
            self.until(lambda: self.value(preload, "Number(status)") == 1)
            self.assertFalse(preload.isVisible())
            self.assertTrue(preload.property("asynchronous"))
            self.assertTrue(preload.property("cache"))
            self.assertEqual(preload.property("sourceSize"), main.property("sourceSize"))
        size = main.property("sourceSize")
        self.assertLessEqual(size.width(), self.window.width())
        self.assertLessEqual(size.height(), self.window.height())
        self.assertEqual(self.value(main, "Number(fillMode)"), 1)  # Image.PreserveAspectFit
        for i, (key, verdict) in enumerate(((Qt.Key_Right, "like"), (Qt.Key_Left, "dislike"), (Qt.Key_Down, "skip"))):
            started = time.monotonic()
            self.key(key)
            self.assertLess(time.monotonic() - started, 1.0)
            self.assertEqual(c.image, self.url(self.fx.shas[i + 1]))
            self.assertEqual(self.fx.store.ratings()[-1].verdict, verdict)
            self.assertEqual((c.today, c.total), (i + 1, i + 1))
        self.assertEqual(c.preload, [self.url(s) for s in self.fx.shas[4:6]])
        self.assertEqual(self.selector.calls[-1], (1, self.fx.shas[3:5]))
        self.assertEqual(self.item("header").property("text"), c.headerText)

    def test_up_loves_clears_note_advances_trains_proposes_and_undoes(self):
        c = self.controller
        self.model.enabled = self.proposer.enabled = True
        self.key(Qt.Key_N)
        self.text("warm gold")
        self.key(Qt.Key_Return)
        self.key(Qt.Key_Up)
        rating, = self.fx.store.ratings()
        self.assertEqual((rating.sha256, rating.verdict, rating.note, rating.love),
                         (self.fx.shas[0], "like", "warm gold", True))
        self.assertEqual(c.note, "")
        self.assertEqual(c.image, self.url(self.fx.shas[1]))
        self.assertEqual((c.today, c.total), (1, 1))
        self.assertEqual(c.headerText, "Today 1 · Total 1 · Loved 1 · AUC n/a")
        self.assertEqual(self.item("header").property("text"), c.headerText)
        self.assertEqual(len(self.model.snapshots), 1)
        self.assertEqual(len(self.proposer.calls), 1)
        self.key(Qt.Key_Up)  # the proposal overlay blocks love too.
        self.assertTrue(c.key(Qt.Key_Up, Qt.NoModifier.value))
        self.assertEqual(c.total, 1)
        self.assertEqual(c.image, self.url(self.fx.shas[1]))
        self.click("rejectProposals")
        self.proposer.enabled = False
        self.key(Qt.Key_Backspace)
        self.assertEqual(c.image, self.url(self.fx.shas[0]))
        self.assertEqual(c.headerText, "Today 0 · Total 0 · Loved 0 · AUC n/a")
        self.assertEqual(len(self.model.snapshots), 2)
        self.key(Qt.Key_Right)
        self.assertIs(self.fx.store.ratings()[0].love, False)

    def test_up_in_note_field_edits_without_rating(self):
        c = self.controller
        self.key(Qt.Key_N)
        self.text("warm gold")
        self.assertTrue(self.item("noteField").hasActiveFocus())
        self.assertFalse(c.key(Qt.Key_Up, Qt.NoModifier.value))
        self.key(Qt.Key_Up)
        repeated = QKeyEvent(QEvent.KeyPress, Qt.Key_Up, Qt.NoModifier, "", True)
        self.app.sendEvent(self.window, repeated)
        self.pump()
        self.assertEqual(self.fx.store.ratings(), [])
        self.assertEqual(c.total, 0)
        self.assertEqual(c.image, self.url(self.fx.shas[0]))
        self.assertEqual(c.note, "warm gold")
        self.assertTrue(self.item("noteField").hasActiveFocus())

    def test_note_focus_edit_keys_limit_keep_and_escape(self):
        c = self.controller
        self.key(Qt.Key_Delete)
        self.key(Qt.Key_N)
        self.assertFalse(c.removeArmed)
        self.assertTrue(c.noteEditing)
        self.assertTrue(self.item("noteField").hasActiveFocus())
        self.text("warm brass")
        self.key(Qt.Key_Left)
        self.key(Qt.Key_Backspace)
        self.key(Qt.Key_Delete)
        self.key(Qt.Key_Down)
        self.key(Qt.Key_Right)
        self.assertEqual(c.note, "warm bra")
        self.assertEqual(c.total, 0)
        self.assertEqual(c.image, self.url(self.fx.shas[0]))
        self.assertFalse(c.removeArmed)
        self.key(Qt.Key_A, Qt.ControlModifier)
        self.paste("x" * 350)
        self.assertEqual(len(c.note), 300)
        self.key(Qt.Key_Return)
        self.assertFalse(c.noteEditing)
        self.assertFalse(self.item("noteField").hasActiveFocus())
        self.assertTrue(self.item("ratingFocus").hasActiveFocus())
        self.key(Qt.Key_Right)
        self.assertEqual(self.fx.store.ratings()[0].note, "x" * 300)
        self.assertEqual(c.note, "")
        self.key(Qt.Key_N)
        self.text("drop this")
        self.key(Qt.Key_Escape)
        self.assertFalse(c.noteEditing)
        self.assertFalse(self.item("noteField").hasActiveFocus())
        self.assertEqual(c.note, "")
        self.key(Qt.Key_Left)
        self.assertIsNone(self.fx.store.ratings()[-1].note)
        self.key(Qt.Key_N)
        self.assertTrue(c.noteEditing)
        self.assertEqual(c.note, "")
        self.text("keep this")
        self.key(Qt.Key_Enter)
        self.key(Qt.Key_Down)
        self.assertEqual(self.fx.store.ratings()[-1].note, "keep this")

    def test_undo_session_and_queue_order(self):
        c = self.controller
        self.fx.store.add_rating(self.fx.shas[-1], "like", "other-session")
        self.key(Qt.Key_Right)
        self.key(Qt.Key_Down)
        self.model.enabled = True
        self.key(Qt.Key_Backspace)
        self.assertEqual(c.image, self.url(self.fx.shas[1]))
        self.assertEqual(c.preload[0], self.url(self.fx.shas[2]))
        self.assertEqual((c.today, c.total), (2, 2))
        self.assertEqual(len(self.model.snapshots), 1)
        self.key(Qt.Key_Backspace)
        self.assertEqual(c.image, self.url(self.fx.shas[0]))
        self.assertEqual(c.preload, [self.url(s) for s in self.fx.shas[1:3]])
        self.assertEqual((c.today, c.total), (1, 1))
        self.assertEqual(len(self.model.snapshots), 2)
        self.key(Qt.Key_Backspace)
        self.assertEqual(c.total, 1)
        self.assertEqual(self.fx.store.ratings()[0].session, "other-session")
        self.key(Qt.Key_Right)
        self.key(Qt.Key_Left)
        self.key(Qt.Key_Down)
        self.assertEqual(c.image, self.url(self.fx.shas[3]))

    def test_double_delete_disarm_and_no_auto_repeat_removal(self):
        c = self.controller
        sha = self.fx.shas[0]
        original = self.fx.store.original_path(sha)
        self.key(Qt.Key_Delete)
        self.assertTrue(c.removeArmed)
        self.assertEqual(c.message, self.rate.REMOVE_TEXT)
        self.key(Qt.Key_Space)  # even an unbound key disarms
        self.assertFalse(c.removeArmed)
        self.key(Qt.Key_Delete)
        repeated = QKeyEvent(QEvent.KeyPress, Qt.Key_Delete, Qt.NoModifier, "", True)
        self.app.sendEvent(self.window, repeated)
        self.assertTrue(self.fx.store.has_image(sha))
        self.assertTrue(c.removeArmed)
        self.key(Qt.Key_Delete)
        self.assertFalse(c.removeArmed)
        self.assertTrue(self.fx.store.is_blocked(sha))
        self.assertFalse(original.exists())
        self.assertFalse(self.fx.store.thumb_path(sha).exists())
        self.assertEqual(c.image, self.url(self.fx.shas[1]))
        self.assertEqual(c.total, 0)
        self.key(Qt.Key_Delete)
        self.key(Qt.Key_Right)
        self.assertFalse(c.removeArmed)
        self.assertTrue(self.fx.store.has_image(self.fx.shas[1]))

    def test_proposals_unchecked_block_keys_accept_and_not_now(self):
        c = self.controller
        self.proposer.enabled = True
        self.key(Qt.Key_Right)
        self.assertEqual(len(c.proposals), 2)
        self.assertFalse(self.item("proposal-0").property("checked"))
        self.assertFalse(self.item("proposal-1").property("checked"))
        self.assertEqual(self.value(self.item("proposal-0"), "Number(contentItem.textFormat)"), 0)
        self.assertEqual(self.proposer.calls, [("fake plan", {"quiet": (1.0, 0.0)})])
        image = c.image
        for key in (Qt.Key_Right, Qt.Key_Up, Qt.Key_Left, Qt.Key_Down, Qt.Key_N, Qt.Key_Backspace, Qt.Key_Delete, Qt.Key_Escape):
            self.key(key)
        self.assertEqual(c.image, image)
        self.assertEqual(c.total, 1)
        self.assertFalse(c.noteEditing)
        self.assertFalse(c.removeArmed)
        self.click("proposal-0")
        self.assertTrue(self.item("proposal-0").property("checked"))
        self.click("fetchProposals")
        statuses = {p["term"]: p["status"] for p in self.fx.store.proposals()}
        self.assertEqual(statuses, {"brass observatory": "accepted", "quiet geometry": "rejected"})
        self.assertEqual(c.proposals, [])
        self.assertIn("fetch.py --proposals", c.message)
        self.assertNotIn("workbench", c.message)
        self.proposer.items = [("blue mechanism", "test", "adjacent")]
        self.key(Qt.Key_Down)
        self.click("proposal-0")
        self.click("rejectProposals")
        self.assertEqual(self.fx.store.proposals()[-1]["status"], "rejected")
        self.proposer.enabled = False
        self.key(Qt.Key_Left)
        self.assertEqual(c.total, 3)

    def test_empty_proposal_round_does_not_open_overlay(self):
        self.proposer.enabled = True
        self.proposer.items = []
        self.key(Qt.Key_Right)
        self.assertEqual(self.controller.proposals, [])
        self.key(Qt.Key_Down)
        self.assertEqual(self.controller.total, 2)

    def test_header_states_training_and_failure(self):
        c = self.controller
        self.model.enabled = True
        self.model.current = replace(self.model.current, ready=True, auc=0.756, stable=True)
        self.key(Qt.Key_Right)
        self.assertEqual(c.aucText, "0.76")
        self.assertTrue(c.stable)
        self.assertIn("stable: you can stop for today (or continue)", c.headerText)
        self.model.current = replace(self.model.current, auc=0.54, stable=False, weak=True)
        self.key(Qt.Key_Left)
        self.assertEqual(c.aucText, "0.54")
        self.assertTrue(c.weak)
        self.assertIn("plateau with a weak AUC: likes and dislikes do not separate yet", c.headerText)
        self.assertEqual(self.item("header").property("text"), c.headerText)
        self.model.current = replace(self.model.current, ready=False, auc=None, weak=False)
        self.key(Qt.Key_Backspace)
        self.assertEqual(c.aucText, "n/a")
        self.assertFalse(c.weak)
        self.model.error = RuntimeError("private diagnostic")
        self.key(Qt.Key_Down)
        self.assertEqual(c.message, "Training failed: RuntimeError.")

    def test_start_training_and_catch_up_without_blocking_advance(self):
        self.dispose()
        self.runner.defer = True
        self.model.enabled = True
        self.build()
        self.assertEqual(len(self.runner.pending), 1)
        self.assertEqual(self.model.snapshots[0][0], threading.get_ident())
        self.key(Qt.Key_Right)
        self.key(Qt.Key_Left)
        self.assertEqual(self.controller.image, self.url(self.fx.shas[2]))
        self.assertEqual(len(self.runner.pending), 1)
        self.runner.complete()
        self.assertEqual(len(self.runner.pending), 1)  # catches up to ratings made during training
        self.runner.complete()
        self.assertEqual(self.runner.pending, [])
        self.assertEqual(self.controller.total, 2)
        self.assertEqual(self.model.applies, [threading.get_ident()] * 2)

    def test_empty_state_and_undo_from_empty(self):
        c = self.controller
        for _ in self.fx.shas:
            self.key(Qt.Key_Down)
        self.assertTrue(c.empty)
        self.assertEqual(c.image, "")
        self.assertEqual(c.preload, [])
        self.assertTrue(self.item("emptyState").isVisible())
        self.assertEqual(self.item("emptyState").property("text"),
                         "No images to rate. Run fetch.py and embed.py first.")
        for key in (Qt.Key_Right, Qt.Key_Left, Qt.Key_Down, Qt.Key_N, Qt.Key_Delete):
            self.key(key)
        self.assertEqual(c.total, len(self.fx.shas))
        self.assertFalse(c.noteEditing)
        self.assertFalse(c.removeArmed)
        self.key(Qt.Key_Backspace)
        self.assertFalse(c.empty)
        self.assertEqual(c.image, self.url(self.fx.shas[-1]))
        self.assertEqual(c.total, len(self.fx.shas) - 1)
        self.key(Qt.Key_Delete)
        self.key(Qt.Key_Delete)
        self.assertTrue(c.empty)

    def test_training_worker_and_ui_delivery(self):
        self.dispose()
        self.model.enabled = True
        self.model.release = threading.Event()
        self.build(run_async=None)
        self.until(self.model.started.is_set)
        self.key(Qt.Key_Right)
        self.assertEqual(self.controller.image, self.url(self.fx.shas[1]))
        self.model.release.set()
        self.until(lambda: not self.model.due() and len(self.model.applies) >= 2)
        self.assertTrue(all(t == threading.get_ident() for t, _ in self.model.snapshots))
        self.assertTrue(all(t == threading.get_ident() for t in self.model.applies))
        self.assertTrue(all(t != threading.get_ident() for t in self.model.trains))

    def test_main_wiring_probes_session_and_shutdown(self):
        from tastelab import embed, seeds

        self.dispose()
        probes = seeds.load_probes(seeds.PROBES_DEFAULT)
        term = probes.terms[0]
        self.fx.store.put_text_embeddings(embed.MODEL_ID, 2, [(probes.phrase(term), embed.to_blob((1.0, 0.0)))])
        loaded = []
        real_load = self.rate.load_ui

        def load(engine, controller):
            engine.warnings.connect(lambda ws: self.warnings.extend(w.toString() for w in ws))
            loaded.append(controller)
            result = real_load(engine, controller)
            self.assertEqual(engine.rootObjects()[0].title(), "Taste Lab")
            QTimer.singleShot(0, self.app.quit)
            return result

        with mock.patch.object(learn, "TasteModel", side_effect=lambda db, model_id: FakeModel(db)), \
                mock.patch.object(learn, "Selector", side_effect=lambda db, model, plan: FakeSelector(db, self.fx.shas)), \
                mock.patch.object(self.rate, "load_ui", side_effect=load), \
                mock.patch.object(QQuickStyle, "setStyle") as style, \
                mock.patch.object(embed, "load_embedder") as loader:
            self.assertEqual(self.rate.main(["--data", str(self.fx.root)]), 0)
        style.assert_called_once_with("Fusion")
        loader.assert_not_called()
        (controller,) = loaded
        self.assertRegex(controller.session, r"^\d{8}T\d{6}-[0-9a-f]{6}$")
        self.assertEqual(list(controller.probe_vectors), [term])
        self.assertEqual(controller.probe_vectors[term].tolist(), [1.0, 0.0])
        self.assertFalse(hasattr(controller, "_pick"))
        self.assertTrue(controller._closed)

    def test_main_refused_is_one_line_and_exit_two(self):
        output = io.StringIO()
        with mock.patch.object(common, "data_root", side_effect=common.Refused("first line\nsecond line")), \
                mock.patch.object(sys, "stderr", output):
            self.assertEqual(self.rate.main(["--data", str(self.fx.root)]), 2)
        self.assertEqual(output.getvalue(), "rate: first line second line\n")


if __name__ == "__main__":
    unittest.main(verbosity=2)
