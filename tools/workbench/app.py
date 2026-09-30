"""Magic 600 Cell development workbench (Qt Quick desktop app).

  tools\\.venv\\workbench\\Scripts\\python.exe tools\\workbench\\app.py [--compact]

Shows every local Claude Code session of this repository with its subagents and
Codex calls, their plain-language briefs, a live view of any session or call,
the progress board and an always-on-top compact view. It reads everything in
place, writes only owner notes, opens tools in their own applications, calls no
model and opens no network listener.
"""
from __future__ import annotations

import argparse
import json
import sys
import time
from datetime import datetime
from pathlib import Path

from PySide6.QtCore import (QAbstractListModel, QByteArray, QFileSystemWatcher, QModelIndex, QObject, Qt, QTimer,
                            QUrl, Property, Signal, Slot)
from PySide6.QtGui import QGuiApplication
from PySide6.QtQml import QQmlApplicationEngine
from PySide6.QtQuickControls2 import QQuickStyle

sys.path.insert(0, str(Path(__file__).resolve().parent))
import launch  # noqa: E402
import notes  # noqa: E402
import paths  # noqa: E402
import sources  # noqa: E402

QML_DIR = Path(__file__).resolve().parent / "qml"
BODY_PREVIEW = 600
FULL_MAX = 1024 * 1024
TIMELINE_MAX = 5000
USER_ROLE = int(Qt.ItemDataRole.UserRole)


def clock(t: float | None) -> str:
    return datetime.fromtimestamp(t).strftime("%H:%M:%S") if t else ""


def timeline_items(records: list, source: str) -> list[dict]:
    """Live-view rows from transcript records or hook events; malformed fields are shown as text."""
    rows = []
    for rec in records:
        t = sources.ts(rec.get("timestamp") or rec.get("t"))
        if source == "events":
            body = rec.get("summary") or rec.get("ntype") or rec.get("reason") or rec.get("agent_type") or ""
            who = f"subagent {rec['agent_id']}" if isinstance(rec.get("agent_id"), str) else "hook"
            rows.append({"t": t, "kind": "event", "who": who, "title": str(rec.get("e", "event")), "body": str(body), "full": ""})
            continue
        agent = "subagent" if rec.get("isSidechain") else ""
        for c in sources.content_items(rec):
            kind = c.get("type")
            if rec.get("type") == "user" and kind == "text" and not rec.get("isMeta"):
                text = sources.text_of(c)
                rows.append({"t": t, "kind": "prompt", "who": agent or "owner", "title": "Prompt", "body": text[:BODY_PREVIEW], "full": text[:FULL_MAX]})
            elif rec.get("type") == "assistant" and kind == "text":
                text = sources.text_of(c)
                rows.append({"t": t, "kind": "text", "who": agent or "Claude", "title": "Claude", "body": text[:BODY_PREVIEW], "full": text[:FULL_MAX]})
            elif kind == "tool_use":
                name = str(c.get("name", "tool"))
                full = json.dumps(c.get("input"), ensure_ascii=False, indent=2)[:FULL_MAX]
                rows.append({"t": t, "kind": "tool", "who": agent or "Claude", "title": name,
                             "body": sources.tool_summary(name, c.get("input")), "full": full})
            elif kind == "tool_result":
                text = sources.result_text(c)
                title = "Result (error)" if c.get("is_error") else "Result"
                rows.append({"t": t, "kind": "result", "who": agent or "tool", "title": title, "body": text[:BODY_PREVIEW], "full": text[:FULL_MAX]})
    for r in rows:
        r["time"] = clock(r["t"])
    return rows


class Timeline(QAbstractListModel):
    ROLES = ("time", "kind", "who", "title", "body", "full")

    def __init__(self):
        super().__init__()
        self.rows: list[dict] = []

    def rowCount(self, parent=QModelIndex()):
        return 0 if parent.isValid() else len(self.rows)

    def data(self, index, role=Qt.DisplayRole):
        if not index.isValid() or not 0 <= index.row() < len(self.rows):
            return None
        role = int(role)
        key = self.ROLES[role - USER_ROLE - 1] if USER_ROLE < role <= USER_ROLE + len(self.ROLES) else "body"
        return self.rows[index.row()].get(key, "")

    def roleNames(self):
        return {USER_ROLE + 1 + i: QByteArray(name.encode()) for i, name in enumerate(self.ROLES)}

    def reset(self, rows=()):
        self.beginResetModel()
        self.rows = list(rows)[-TIMELINE_MAX:]
        self.endResetModel()

    def extend(self, rows: list):
        if not rows:
            return
        rows = sorted(rows, key=lambda r: r["t"] or 0)
        self.beginInsertRows(QModelIndex(), len(self.rows), len(self.rows) + len(rows) - 1)
        self.rows += rows
        self.endInsertRows()
        excess = len(self.rows) - TIMELINE_MAX
        if excess > 0:
            self.beginRemoveRows(QModelIndex(), 0, excess - 1)
            del self.rows[:excess]
            self.endRemoveRows()


class Launcher:
    """The real desktop launcher; tests substitute a recorder."""

    def open(self, plan):
        launch.run(plan)

    def command(self, argv, cwd):
        launch.run_command(argv, cwd)

    def claude(self):
        launch.bring_claude_forward()


class Workbench(QObject):
    boardsChanged = Signal()
    selectedChanged = Signal()
    notesChanged = Signal()
    messageChanged = Signal()

    def __init__(self, main: Path, projects: Path | None = None, launcher=None, refresh_ms: int = 2000):
        super().__init__()
        self.main = Path(main)
        self.projects = Path(projects) if projects else paths.claude_projects_dir()
        self.data = paths.data_root(self.main)
        self.launcher = launcher or Launcher()
        self._cache: dict = {}
        self._ledgers = sources.Ledgers()
        self._sessions: list = []
        self._session_rows: list = []
        self._calls: list = []
        self._progress: dict = {}
        self._compact: dict = {}
        self._selected: dict = {}
        self._notes: list = []
        self._files: list = []
        self._message = ""
        self._timeline = Timeline()
        self._tails: list = []
        self._watcher = QFileSystemWatcher(self)
        self._watcher.fileChanged.connect(lambda _p: self.poll())
        self._poll = QTimer(self, interval=1000, timeout=self.poll)
        self._refresh = QTimer(self, interval=refresh_ms, timeout=self.refresh)
        self.refresh()
        self._refresh.start()

    # ------------------------------------------------------------ boards

    @Slot()
    def refresh(self):
        now = time.time()
        roots = paths.checkouts(self.main)
        self._sessions = sources.discover_sessions(self.main, self.projects, now=now, cache=self._cache)
        calls = sources.codex_calls(roots, now=now, ledgers=self._ledgers)
        links = sources.link_calls(self._sessions, calls)
        by_sid: dict = {}
        for c in calls:
            link = links.get(c["call_id"])
            c["session"], c["link"] = (link or ("", ""))
            c["started"] = clock(c.get("time"))
            c["checkoutName"] = Path(c["checkout"]).name
            c["blockingText"] = ", ".join(c.get("blocking") or [])
            if link:
                by_sid.setdefault(link[0], []).append({"call_id": c["call_id"], "kind": c.get("kind") or "", "status": c["status"]})
        self._calls = calls
        rows = []
        for s in self._sessions:
            summ = sources.summary(self.data, s)
            subs = [{"id": k, "type": v.get("type") or "", "status": v.get("status") or "unknown", "transcript": v.get("transcript", "")}
                    for k, v in s.status.subagents.items()]
            rows.append({"sid": s.sid, "title": s.title or s.sid[:8], "checkout": s.checkout.name, "branch": s.branch,
                         "status": s.status.status, "detail": s.status.detail, "last": clock(s.status.last_activity),
                         "summarySource": summ["source"], "goal": summ.get("goal", ""), "step": summ.get("step", ""),
                         "doing": summ.get("doing", ""), "why": summ.get("why", ""), "waitingFor": summ.get("waiting_for", ""),
                         "subagents": subs, "calls": by_sid.get(s.sid, []), "malformed": s.malformed, "capped": s.capped})
        self._session_rows = rows
        doc = sources.progress(self.main) or {}
        step = sources.current_step(doc) or {}
        b = self._ledgers.budget()
        self._progress = {"steps": doc.get("steps", []) if isinstance(doc.get("steps"), list) else [],
                          "current": str(doc.get("current", "")), "updated": str(doc.get("updated", ""))}
        api = ("incomplete: a ledger is over the read limit" if b["incomplete"]
               else f"${b['billed']:.2f} billed / ${doc.get('paid_api_ceiling_usd', '?')}")
        self._compact = {
            "step": f"{step.get('id', '?')} {step.get('title', 'step unknown')}", "stepStatus": str(step.get("status", "")).replace("_", " "),
            "acceptance": step.get("acceptance") or "not defined", "blocker": step.get("blocker") or "none",
            "findings": len(sources.open_findings(roots)), "codex": sources.codex_login(), "api": api,
            "apiDetail": f"estimated ${b['estimated']:.2f}, reserved ${b['reserved']:.2f}, "
                         f"{b['unknown_calls']} paid call(s) with unknown cost, {b['subscription_calls']} subscription call(s)"}
        self.boardsChanged.emit()
        if self._selected.get("kind") == "session":
            self._refresh_selected_session()
        elif self._selected.get("kind") == "call":
            call = next((c for c in self._calls if c["call_id"] == self._selected.get("id")), None)
            if call and (call["status"], call.get("verdict")) != (self._selected.get("status"), self._selected.get("verdict")):
                self._show_call(call)  # the call finished (or failed) while it was selected

    @Property("QVariantList", notify=boardsChanged)
    def sessions(self):
        return self._session_rows

    @Property("QVariantList", notify=boardsChanged)
    def calls(self):
        return self._calls

    @Property("QVariantMap", notify=boardsChanged)
    def progress(self):
        return self._progress

    @Property("QVariantMap", notify=boardsChanged)
    def compact(self):
        return self._compact

    # ------------------------------------------------------------ live view

    @Property(QObject, constant=True)
    def timeline(self):
        return self._timeline

    @Property("QVariantMap", notify=selectedChanged)
    def selected(self):
        return self._selected

    @Property("QVariantList", notify=notesChanged)
    def noteStates(self):
        return self._notes

    @Property("QVariantList", notify=selectedChanged)
    def files(self):
        return self._files

    def _follow(self, *files: Path):
        if self._watcher.files():
            self._watcher.removePaths(self._watcher.files())
        self._tails = [(sources.Tail(f), "events" if f.parent.name == "events" else "transcript") for f in files]
        existing = [str(f) for f in files if f.is_file()]
        if existing:
            self._watcher.addPaths(existing)
        self._timeline.reset()
        self.poll()
        self._poll.start()

    @Slot()
    def poll(self):
        rows = []
        for tail, source in self._tails:
            rows += timeline_items(tail.poll(), source)
            if str(tail.path) not in self._watcher.files() and tail.path.is_file():
                self._watcher.addPath(str(tail.path))
        self._timeline.extend(rows)

    def _session(self, sid):
        return next((s for s in self._sessions if s.sid == sid), None)

    @Slot(str)
    def selectSession(self, sid: str):
        s = self._session(sid)
        if s is None:
            return self._say("That session is no longer listed.")
        self._selected = {"kind": "session", "id": sid}
        self._follow(s.transcript, self.data / "events" / f"{sid}.jsonl")
        self._refresh_selected_session()

    def _refresh_selected_session(self):
        sid = self._selected.get("id")
        s = self._session(sid)
        row = next((r for r in self._session_rows if r["sid"] == sid), None)
        if s is None or row is None:
            return
        roots = paths.checkouts(self.main)
        files = [{"label": Path(p).name, "path": p, "marimo": p.endswith(".py") and self._is_marimo(Path(p))}
                 for p in sources.referenced_files(s.state, roots)]
        files.append({"label": "Transcript (private, plain text)", "path": str(s.transcript), "marimo": False})
        wiki = self.main / "docs" / "wiki" / "index.md"
        if wiki.is_file():
            files.append({"label": "Wiki index", "path": str(wiki), "marimo": False})
        self._files = files
        self._selected = {**row, "kind": "session", "id": sid, "cwd": s.cwd, "checkoutPath": str(s.checkout),
                          "malformedLive": sum(t.malformed for t, _ in self._tails),
                          "resume": launch.resume_command_text(sid)}
        self.selectedChanged.emit()
        self._notes = sources.notes(self.data, sid, s.state)
        self.notesChanged.emit()

    @staticmethod
    def _is_marimo(p: Path) -> bool:
        try:
            with p.open("r", encoding="utf-8", errors="replace") as f:
                return "marimo.App(" in f.read(4096)
        except OSError:
            return False

    @Slot(str, str)
    def selectSubagent(self, sid: str, agent: str):
        row = next((r for r in self._session_rows if r["sid"] == sid), None)
        sub = next((a for a in (row or {}).get("subagents", []) if a["id"] == agent), None)
        if not sub or not sub.get("transcript"):
            return self._say("No transcript for this subagent yet.")
        self._selected = {"kind": "subagent", "id": agent, "title": f"Subagent {agent} ({sub.get('type') or 'unknown type'})",
                          "status": sub.get("status"), "detail": f"of session {sid[:8]}"}
        self._files = [{"label": "Subagent transcript (private, plain text)", "path": sub["transcript"], "marimo": False}]
        self._notes = []
        self._follow(Path(sub["transcript"]))
        self.selectedChanged.emit()
        self.notesChanged.emit()

    @Slot(str)
    def selectCall(self, call_id: str):
        call = next((c for c in self._calls if c["call_id"] == call_id), None)
        if call is None:
            return self._say("That Codex call is no longer listed.")
        self._show_call(call)

    def _show_call(self, call: dict):
        d = Path(call["dir"])
        self._poll.stop()
        self._tails = []
        rows = [{"t": call.get("time"), "time": clock(call.get("time")), "kind": "event", "who": "Codex",
                 "title": f"{call.get('kind') or 'call'}: {call['status']}",
                 "body": ("verdict " + str(call.get("verdict"))) if call.get("verdict") else
                         "No live output: the review wrapper writes its result when the call returns.",
                 "full": "\n".join(call.get("problems") or [])}]
        result = sources.read_json(d / "review.json")
        if result is sources.OVERSIZED:
            rows.append({"t": None, "time": "", "kind": "event", "who": "Codex", "title": "Result too large",
                         "body": "review.json is larger than the read limit; open it as plain text below.", "full": ""})
        elif isinstance(result, dict):
            summary_text = str(result.get("summary", ""))
            rows.append({"t": None, "time": "", "kind": "text", "who": "Codex", "title": "Summary",
                         "body": summary_text[:BODY_PREVIEW], "full": summary_text[:FULL_MAX]})
            answers = sources.read_json(d / "dispositions.json")
            answers = answers if isinstance(answers, dict) else {}
            for f in result.get("findings") if isinstance(result.get("findings"), list) else []:
                if isinstance(f, dict):
                    rows.append({"t": None, "time": "", "kind": "result",
                                 "who": f"{f.get('severity')} / {answers.get(f.get('id'), 'unanswered')}",
                                 "title": f"{f.get('id')}: {f.get('title')}", "body": str(f.get("detail", ""))[:BODY_PREVIEW],
                                 "full": json.dumps(f, ensure_ascii=False, indent=2)[:FULL_MAX]})
        self._timeline.reset(rows)
        self._files = [{"label": f"{name} (private, plain text)", "path": str(d / name), "marimo": False}
                       for name in ("packet.md", "review.json", "meta.json", "codex.log", "dispositions.json") if (d / name).is_file()]
        self._selected = {"kind": "call", "id": call["call_id"], "title": f"Codex {call.get('kind') or 'call'} {call['call_id']}",
                          "status": call["status"], "verdict": call.get("verdict"),
                          "detail": f"{call['checkoutName']}; linked session {call['session'][:8] or 'none'}"}
        self._notes = []
        self.selectedChanged.emit()
        self.notesChanged.emit()

    # ------------------------------------------------------------ owner notes

    @Slot(str, result=bool)
    def sendNote(self, text: str) -> bool:
        if self._selected.get("kind") != "session":
            self._say("Select a session first.")
            return False
        try:
            nid = notes.append_note(self.data, self._selected["id"], text)
        except notes.NoteRejected as exc:
            self._say(f"Note not sent: {exc}.")
            return False
        self._say(f"Note {nid} queued; Claude receives it at its next tool step.")
        s = self._session(self._selected["id"])
        self._notes = sources.notes(self.data, self._selected["id"], s.state if s else None)
        self.notesChanged.emit()
        return True

    # ------------------------------------------------------------ tools

    @Slot(str)
    def openFile(self, path: str):
        try:
            self.launcher.open(launch.plan_open(path, self.main, self.projects))
        except launch.LaunchRefused as exc:
            self._say(f"Not opened: {exc}.")
        except OSError as exc:
            self._say(f"Could not open the file: {exc.strerror or exc}.")

    @Slot(str, str)
    def runTool(self, kind: str, target: str = ""):
        try:
            if kind == "git-diff":
                argv, cwd = launch.plan_command(kind, self.main, checkout=self._selected.get("checkoutPath") or self.main)
            elif kind == "marimo":
                argv, cwd = launch.plan_command(kind, self.main, notebook=target)
            else:
                argv, cwd = launch.plan_command(kind, self.main)
            self.launcher.command(argv, cwd)
        except launch.LaunchRefused as exc:
            self._say(f"Not started: {exc}.")
        except OSError as exc:
            self._say(f"Could not start the tool: {exc.strerror or exc}.")

    @Slot(bool)
    def openInClaude(self, remote: bool):
        if self._selected.get("kind") != "session":
            return self._say("Select a session first.")
        sid = self._selected["id"]
        self.refresh()  # decide on the session's current lifecycle evidence, never a cached status
        s = self._session(sid)
        status = s.status.status if s else "unknown"
        try:
            argv, cwd = launch.plan_command("resume", self.main, sid=sid, cwd=s.cwd if s else None,
                                            session_status=status, remote=remote)
            self.launcher.command(argv, cwd)
            self._say("Resuming the ended session in a new console.")
        except launch.LaunchRefused:
            try:
                self.launcher.claude()
            except OSError:
                pass
            self._say("The session may still be open in another client, so it was not resumed here. "
                      f"Claude Code was brought forward; after closing the other client you can run: "
                      f"{launch.resume_command_text(sid, remote)}")

    # ------------------------------------------------------------ messages

    @Property(str, notify=messageChanged)
    def message(self):
        return self._message

    def _say(self, text: str):
        self._message = text
        self.messageChanged.emit()


def load_ui(engine: QQmlApplicationEngine, wb: Workbench, compact: bool = False) -> bool:
    """Load the full window, or with `compact` only the always-on-top progress view as its own
    top-level window (closing it quits). Returns False when QML failed to load."""
    engine.rootContext().setContextProperty("wb", wb)
    if compact:
        engine.setInitialProperties({"standalone": True, "visible": True})
        engine.load(QUrl.fromLocalFile(str(QML_DIR / "Compact.qml")))
    else:
        engine.load(QUrl.fromLocalFile(str(QML_DIR / "Main.qml")))
    return bool(engine.rootObjects())


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description="Magic 600 Cell development workbench")
    parser.add_argument("--compact", action="store_true", help="open only the always-on-top progress view")
    args = parser.parse_args(argv)
    main_dir = paths.main_checkout(Path(__file__).resolve().parent)
    if main_dir is None:
        print("workbench: not inside a checkout of this repository", file=sys.stderr)
        return 2
    QQuickStyle.setStyle("Fusion")
    app = QGuiApplication(sys.argv[:1])
    app.setApplicationName("Magic 600 Cell workbench")
    wb = Workbench(main_dir)
    engine = QQmlApplicationEngine()
    if not load_ui(engine, wb, args.compact):
        return 1
    return app.exec()


if __name__ == "__main__":
    sys.exit(main())
