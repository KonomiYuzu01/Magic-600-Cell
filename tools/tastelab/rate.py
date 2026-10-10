"""The rating window (PySide6 Qt Quick; tastelab environment).

  python tools/tastelab/rate.py [--data DIR]

A large image with its source, licence and tier in small type. Keys:
  Right like, Left dislike, Down skip,
  N note (one sentence; Enter keeps it for the next verdict, Escape drops it),
  Backspace undo, Delete twice removes the image for good (files deleted, sha256 blocked).
Header: ratings today and in total, the AUC ("n/a" until the model is ready),
"stable" with the stop suggestion (the owner may continue), and a hint when the
plateau is weak. After every 100 ratings the proposed search terms appear as
checkboxes; ticked terms become accepted proposals that `fetch.py --proposals`
fetches. The next images preload, so each verdict shows the next image promptly.

The controller owns the UI-thread Store. Training uses a detached snapshot in
its worker; completions return to the UI thread. Images may be class A or class
B and every display and verdict rechecks removal and blocking. Nothing is
uploaded and no listener is opened.
"""
from __future__ import annotations

import argparse
import secrets
import sys
from concurrent.futures import ThreadPoolExecutor
from datetime import datetime
from pathlib import Path

if not __package__:
    sys.path[0] = str(Path(__file__).resolve().parents[1])

from PySide6.QtCore import QCoreApplication, QEvent, QObject, Qt, QUrl, Property, Signal, Slot
from PySide6.QtGui import QGuiApplication
from PySide6.QtQml import QQmlApplicationEngine
from PySide6.QtQuickControls2 import QQuickStyle

from tastelab import common, learn

QML_DIR = Path(__file__).resolve().parent / "qml"
REMOVE_TEXT = "Press Delete again to remove this image for good."



class RateController(QObject):
    imageChanged = Signal()
    headerChanged = Signal()
    noteChanged = Signal()
    removeChanged = Signal()
    proposalsChanged = Signal()
    messageChanged = Signal()
    _finished = Signal(object)

    def __init__(self, store, model, selector, *, session: str, plan=None,
                 probe_vectors=None, proposals_due=learn.proposals_due,
                 propose=learn.propose_terms, run_async=None, parent=None):
        super().__init__(parent)
        self.store, self.model, self.selector = store, model, selector
        self.session = session
        self.plan, self.probe_vectors = plan, probe_vectors or {}
        self._proposals_due, self._propose = proposals_due, propose
        self._run_async = run_async
        self._training_executor = ThreadPoolExecutor(max_workers=1) if run_async is None else None
        self._finished.connect(self._deliver, Qt.ConnectionType.QueuedConnection)
        self._closed = self._training = False
        self._current = None
        self._queue = []
        self._image = self._caption = self._note = self._message = ""
        self._note_editing = self._remove_armed = False
        self._proposals, self._checked = [], set()
        self._counts = store.counts()
        self._state = model.state()
        self._advance()
        self._maybe_train()

    @Property(str, notify=imageChanged)
    def image(self):
        return self._image

    @Property("QVariantList", notify=imageChanged)
    def preload(self):
        return [self._file_url(sha) for sha in self._queue[:2] if self._available(sha)]

    @Property(str, notify=imageChanged)
    def caption(self):
        return self._caption

    @Property(bool, notify=imageChanged)
    def empty(self):
        return self._current is None

    @Property(int, notify=headerChanged)
    def today(self):
        return self._counts["today"]

    @Property(int, notify=headerChanged)
    def total(self):
        return self._counts["total"]

    @Property(str, notify=headerChanged)
    def aucText(self):
        return f"{self._state.auc:.2f}" if self._state.ready and self._state.auc is not None else "n/a"

    @Property(bool, notify=headerChanged)
    def stable(self):
        return self._state.stable

    @Property(bool, notify=headerChanged)
    def weak(self):
        return self._state.weak

    @Property(str, notify=headerChanged)
    def headerText(self):
        text = f"Today {self.today} · Total {self.total} · AUC {self.aucText}"
        if self.stable:
            text += " · stable: you can stop for today (or continue)"
        if self.weak:
            text += " · plateau with a weak AUC: likes and dislikes do not separate yet"
        return text

    @Property(str, notify=noteChanged)
    def note(self):
        return self._note

    @Property(bool, notify=noteChanged)
    def noteEditing(self):
        return self._note_editing

    @Property(bool, notify=removeChanged)
    def removeArmed(self):
        return self._remove_armed

    @Property("QVariantList", notify=proposalsChanged)
    def proposals(self):
        return self._proposals

    @Property(str, notify=messageChanged)
    def message(self):
        return self._message

    def _say(self, text):
        self._message = text
        self.messageChanged.emit()

    def _file_url(self, sha):
        return QUrl.fromLocalFile(str(self.store.display_path(sha).resolve())).toString()

    def _available(self, sha):
        return not self._closed and bool(sha) and self.store.has_image(sha) and not self.store.is_blocked(sha)

    @Slot(str, result=bool)
    def imageLoaded(self, url):
        """Approve a delayed decode only while its image is still eligible and current or queued."""
        return any(self._available(sha) and self._file_url(sha) == url
                   for sha in [self._current, *self._queue])

    def _top_up(self):
        self._queue = [sha for sha in self._queue if self._available(sha)]
        n = 2 - len(self._queue)
        if n > 0:
            exclude = ([self._current] if self._current else []) + self._queue
            self._queue.extend(sha for sha in self.selector.next(n, exclude=exclude) if self._available(sha))

    def _show(self, sha):
        if self._closed or sha is not None and not self._available(sha):
            return
        self._current = sha
        self._image = self._file_url(sha) if sha else ""
        meta = self.store.image(sha) if sha else None
        self._caption = " · ".join((meta.source, meta.licence, f"tier {meta.tier}", meta.title or "")) if meta else ""
        if sha:
            self.selector.shown(sha)
        self._top_up()
        self.imageChanged.emit()

    def _advance(self):
        self._queue = [sha for sha in self._queue if self._available(sha)]
        if not self._queue:
            self._queue.extend(sha for sha in self.selector.next(3, exclude=([self._current] if self._current else []))
                               if self._available(sha))
        self._show(self._queue.pop(0) if self._queue else None)

    def _refresh_header(self, state=None):
        self._counts = self.store.counts()
        self._state = state if state is not None else self.model.state()
        self.headerChanged.emit()

    def _submit(self, fn, on_done, executor):
        if self._run_async is not None:
            self._run_async(fn, on_done)
            return

        def finished(future):
            try:
                result = future.result()
            except Exception as exc:
                result = exc
            self._finished.emit((on_done, result))

        executor.submit(fn).add_done_callback(finished)

    @Slot(object)
    def _deliver(self, payload):
        if self._closed:
            return
        on_done, result = payload
        on_done(result)

    def _maybe_train(self):
        if self._closed or self._training or not self.model.due():
            return
        snap = self.model.snapshot()
        self._training = True
        self._submit(lambda: self.model.train(snap), self._trained, self._training_executor)

    def _trained(self, result):
        self._training = False
        if self._closed:
            return
        if isinstance(result, Exception):
            self._say(f"Training failed: {type(result).__name__}.")
            return
        self._refresh_header(self.model.apply(result))
        # Ratings or undo may have changed the data while this snapshot trained.
        self._maybe_train()

    def _maybe_propose(self):
        if self._closed or self._proposals or not self._proposals_due(self.store):
            return
        items = self._propose(self.store, self.model, self.plan, self.probe_vectors)
        if items:
            self.disarmRemoval()
            self._proposals = [dict(term=t, category=c, origin=o) for t, c, o in items]
            self._checked.clear()
            self.proposalsChanged.emit()

    def _changed_ratings(self):
        self._refresh_header()
        self._maybe_train()
        self._maybe_propose()

    @Slot()
    def disarmRemoval(self):
        if self._remove_armed:
            self._remove_armed = False
            self.removeChanged.emit()
            if self._message == REMOVE_TEXT:
                self._say("")

    def _clear_note(self):
        self._note, self._note_editing = "", False
        self.noteChanged.emit()

    @Slot(str)
    def setNote(self, text):
        self._note = text.replace("\r", " ").replace("\n", " ")[:common.NOTE_MAX]
        self.noteChanged.emit()

    @Slot(bool)
    def finishNote(self, keep):
        if not keep:
            self._note = ""
        self._note_editing = False
        self.noteChanged.emit()

    @Slot(int, int, result=bool)
    @Slot(int, int, bool, result=bool)
    def key(self, key, modifiers, auto_repeat=False):
        if self._closed or auto_repeat and key in (Qt.Key_Left, Qt.Key_Right, Qt.Key_Down, Qt.Key_Delete, Qt.Key_Backspace):
            return True
        if key != Qt.Key_Delete:
            self.disarmRemoval()
        if self._proposals:
            return True
        if self._note_editing:
            if key in (Qt.Key_Return, Qt.Key_Enter, Qt.Key_Escape):
                self.finishNote(key != Qt.Key_Escape)
                return True
            return False
        if modifiers & (Qt.ControlModifier.value | Qt.AltModifier.value | Qt.MetaModifier.value):
            return False
        verdict = {Qt.Key_Right: "like", Qt.Key_Left: "dislike", Qt.Key_Down: "skip"}.get(key)
        if verdict:
            if self._available(self._current):
                self.store.add_rating(self._current, verdict, self.session, self._note)
                self._clear_note()
                self._advance()
                self._changed_ratings()
            return True
        if key == Qt.Key_Backspace:
            sha = self.store.undo_last(self.session)
            if sha is not None:
                self._queue = [s for s in self._queue if s != sha]
                if self._current and self._current != sha:
                    self._queue.insert(0, self._current)
                self._clear_note()
                self._show(sha)
                self._changed_ratings()
            return True
        if key == Qt.Key_N:
            if self._current:
                self._note_editing = True
                self.noteChanged.emit()
            return True
        if key == Qt.Key_Delete:
            if self._current:
                if self._remove_armed:
                    self.disarmRemoval()
                    self.store.remove_image(self._current)
                    self._clear_note()
                    self._advance()
                    self._changed_ratings()
                else:
                    self._remove_armed = True
                    self.removeChanged.emit()
                    self._say(REMOVE_TEXT)
            return True
        return False

    @Slot(str, bool)
    def checkProposal(self, term, checked):
        if checked:
            self._checked.add(term)
        else:
            self._checked.discard(term)

    def _finish_proposals(self, accept):
        for item in self._proposals:
            status = "accepted" if accept and item["term"] in self._checked else "rejected"
            self.store.set_proposal_status(item["term"], status)
        self._proposals, self._checked = [], set()
        self.proposalsChanged.emit()
        self._say("Run fetch.py --proposals to fetch the ticked terms.")

    @Slot()
    def acceptProposals(self):
        self._finish_proposals(True)

    @Slot()
    def rejectProposals(self):
        self._finish_proposals(False)

    def shutdown(self):
        self._closed = True
        if self._training_executor is not None:
            self._training_executor.shutdown(wait=True)
        QCoreApplication.sendPostedEvents(self, QEvent.Type.MetaCall)


def load_ui(engine: QQmlApplicationEngine, controller: RateController) -> bool:
    engine.rootContext().setContextProperty("rate", controller)
    engine.load(QUrl.fromLocalFile(str(QML_DIR / "Rate.qml")))
    return bool(engine.rootObjects())


def main(argv=None) -> int:
    from tastelab import embed, seeds, store

    parser = argparse.ArgumentParser(description="Taste Lab rating window")
    parser.add_argument("--data", metavar="DIR", help="private Taste Lab data folder")
    args = parser.parse_args(argv)
    try:
        root = common.data_root(args.data)
        with store.Store(root) as db:
            plan = seeds.load(seeds.ensure(root))
            model = learn.TasteModel(db, embed.MODEL_ID)
            selector = learn.Selector(db, model, plan)
            probes = seeds.load_probes(seeds.ensure(root, seeds.PROBES_FILE, seeds.PROBES_DEFAULT))
            blobs = db.text_embeddings(embed.MODEL_ID, [probes.phrase(t) for t in probes.terms])
            vectors = {t: embed.from_blobs([blobs[probes.phrase(t)]], len(blobs[probes.phrase(t)]) // 4)[0]
                       for t in probes.terms if probes.phrase(t) in blobs}
            session = datetime.now().strftime("%Y%m%dT%H%M%S") + "-" + secrets.token_hex(3)
            QQuickStyle.setStyle("Fusion")
            app = QGuiApplication.instance() or QGuiApplication(sys.argv[:1])
            app.setApplicationName("Taste Lab")
            engine = QQmlApplicationEngine()
            controller = RateController(db, model, selector, session=session, plan=plan,
                                        probe_vectors=vectors, parent=engine)
            try:
                if not load_ui(engine, controller):
                    return 1
                return app.exec()
            finally:
                controller.shutdown()
                engine.deleteLater()
                QCoreApplication.sendPostedEvents(engine, QEvent.Type.DeferredDelete)
    except common.Refused as exc:
        print("rate: " + " ".join(str(exc).splitlines()), file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
