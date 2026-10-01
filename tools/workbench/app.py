"""Magic 600 Cell development workbench (Qt Quick desktop app).

  tools\\.venv\\workbench\\Scripts\\python.exe tools\\workbench\\app.py [--compact]

Shows every local Claude Code session of this repository with its subagents and
Codex calls, their plain-language briefs, a live view of any session, call or run,
what needs attention, the progress board and an always-on-top compact view. It
reads everything in place and writes only owner notes, run records, cancel
flags and a Home count snapshot under the private data root. On the owner's click only, it starts a
registered run, a review-wrapper call or a new Claude Code session; it calls no
model itself and opens no network listener.
"""
from __future__ import annotations

import argparse
import json
import math
import os
import re
import sys
import tempfile
import time
from datetime import datetime
from pathlib import Path

from PySide6.QtCore import (QAbstractListModel, QByteArray, QFileSystemWatcher, QModelIndex, QObject, Qt, QTimer,
                            QUrl, Property, Signal, Slot)
from PySide6.QtGui import QColor, QIcon, QImageReader, QPainter, QPixmap
from PySide6.QtQml import QQmlApplicationEngine
from PySide6.QtQuickControls2 import QQuickStyle
from PySide6.QtWidgets import QApplication, QSystemTrayIcon

sys.path.insert(0, str(Path(__file__).resolve().parent))
import checklist  # noqa: E402
import home  # noqa: E402
import launch  # noqa: E402
import notes  # noqa: E402
import paths  # noqa: E402
import runs  # noqa: E402
import sources  # noqa: E402
import watch  # noqa: E402

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

    def start_run(self, main, checkout, run_id, expect):
        return runs.start(main, checkout, run_id=run_id, expect=expect)

    def start_codex(self, main, checkout, spec):
        return runs.start(main, checkout, codex=spec)

    def cancel_run(self, data, rid):
        runs.cancel(data, rid)


RUN_HISTORY = 50  # run records listed on the Runs board; flags and Stop buttons see every recent one
RUN_TONE = {"starting": "running", "running": "running", "passed": "finished", "failed": "failed",
            "timed_out": "failed", "interrupted": "failed", "cancelled": "unknown", "refused": "unknown"}


def duration_text(seconds) -> str:
    return sources.age_text(seconds) if isinstance(seconds, (int, float)) and not isinstance(seconds, bool) else ""


def run_row(rec: dict) -> dict:
    """A run record as a board row; the record's own fields stay as they are."""
    exits = rec.get("exits") if isinstance(rec.get("exits"), list) else []
    return {**rec, "tone": RUN_TONE.get(rec.get("status"), "unknown"), "startedText": clock(sources.ts(rec.get("started") or rec.get("created"))),
            "durationText": duration_text(rec.get("duration_s")), "checkoutName": Path(str(rec.get("checkout") or "")).name,
            "exitsText": " ".join("-" if e is None else str(e) for e in exits), "reasonText": str(rec.get("reason") or "")}


class Notifier(QObject):
    openCard = Signal(str)

    def __init__(self, tray=None, window=None):
        super().__init__()
        self.tray, self.window = tray, window
        self._seen: set | None = None
        self._first_new = ""
        if tray is not None:
            tray.messageClicked.connect(self._clicked)

    def _clicked(self):
        if self._first_new:
            self.openCard.emit(self._first_new)

    def update(self, cards):
        ids = {c["id"] for c in cards}
        if self._seen is None:
            self._seen = ids
            return
        new = [c for c in cards if c["needsOwner"] and c["id"] not in self._seen]
        self._seen.update(ids)
        if not new:
            return
        self._first_new = new[0]["id"]
        if self.tray is not None:
            text = new[0]["title"] if len(new) == 1 else f"{len(new)} new items for you"
            self.tray.showMessage("New for you", text, QSystemTrayIcon.MessageIcon.Information, 5000)
        elif self.window is not None:
            self.window.alert(0)


class Workbench(QObject):
    boardsChanged = Signal()
    checkoutsChanged = Signal()
    selectedChanged = Signal()
    notesChanged = Signal()
    messageChanged = Signal()
    focusCard = Signal(str)
    inboxChanged = Signal()
    galleryChanged = Signal()
    gallerySessionsChanged = Signal()
    homeProgressChanged = Signal()

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
        self._runs: list = []
        self._all_runs: list = []
        self._checkouts: list = []
        self._registry: dict = {"entries": [], "problems": []}
        self._flags: list = []
        self._raw_flags: list = []
        self._cards: list = []
        self._inbox: list = []
        self._answered: list = []
        self._scanner = home.GalleryScanner()
        self._gallery: list = []
        self._gallery_sessions: list = []
        self._gallery_actions: dict = {}
        self._records: list = []
        self._home: dict = {}
        self._home_progress: dict = {}
        self._snapshot_counts = None
        self._snapshot_at = 0.0
        self._snapshot_error_shown = False
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
        listed = [{"name": p.name if p != self.main.resolve() else f"{p.name} (main)", "path": str(p)} for p in roots]
        if listed != self._checkouts:  # only a real change resets the choosers' models
            self._checkouts = listed
            self.checkoutsChanged.emit()
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
        records = runs.recent(self.data, now, limit=RUN_HISTORY, window=watch.WATCH_WINDOW)
        self._records = records
        self._all_runs = [run_row(r) for r in records]
        self._runs = self._all_runs[:RUN_HISTORY]
        reg = runs.load_registry(self.main)
        live: dict = {}
        for r in self._all_runs:
            if r.get("kind") == "registry" and r.get("status") in ("starting", "running"):
                live.setdefault(r.get("run"), []).append({"rid": r["rid"], "checkoutName": r["checkoutName"]})
        self._registry = {"problems": list(reg.problems), "entries": [
            {"id": e["id"], "title": e["title"], "display": [" ".join(step) for step in e["steps"]], "digest": runs.entry_digest(e),
             "windowsRequired": e["windows_required"], "timeout": duration_text(e["timeout_s"]), "live": live.get(e["id"], [])}
            for e in reg.entries]}
        self._raw_flags = watch.flags(self._sessions, calls, records, now)
        self._flags = [{**f, "age": duration_text(now - f["t"])} for f in self._raw_flags]
        cards, _ = home.inbox(self._sessions, calls, links, records, self._raw_flags, self.data, roots, now)
        self._cards = cards
        self._publish("_inbox", [c for c in cards if c["state"] in ("open", "queued")], self.inboxChanged)
        self._publish("_answered", [c for c in cards if c["state"] in ("sent", "acknowledged")][:50], self.inboxChanged)
        self._publish("_gallery_sessions", [{"sid": s.sid, "title": s.title or s.sid[:8]} for s in self._sessions],
                      self.gallerySessionsChanged)
        items = self._scan_gallery(roots, records, cards, now)
        doc = sources.progress(self.main) or {}
        step = sources.current_step(doc) or {}
        self._publish("_home_progress", home.progress_view(doc, self._sessions), self.homeProgressChanged)
        b = self._ledgers.budget()
        self._progress = {"steps": doc.get("steps", []) if isinstance(doc.get("steps"), list) else [],
                          "current": str(doc.get("current", "")), "updated": str(doc.get("updated", ""))}
        api = ("incomplete: a ledger is over the read limit" if b["incomplete"]
               else f"${b['billed']:.2f} billed / ${doc.get('paid_api_ceiling_usd', '?')}")
        percent = checklist.step_percent(step)
        step_text = (f"Step {step['id']} {math.floor(percent)}%" if percent is not None
                     else f"Step {step['id']} no checklist") if step else "step unknown"
        counts = home.counts(cards, items)
        self._home = {"forYou": counts["for_you"], "galleryNew": counts["gallery_new"],
                      "machineBadge": home.machine_badge(self._raw_flags), "stepText": step_text,
                      "apiText": api, "apiVisible": bool(b["incomplete"] or any(
                          b[k] for k in ("billed", "estimated", "reserved", "unknown_calls")))}
        self._compact = {
            "step": f"{step.get('id', '?')} {step.get('title', 'step unknown')}", "stepStatus": str(step.get("status", "")).replace("_", " "),
            "acceptance": step.get("acceptance") or "not defined", "blocker": step.get("blocker") or "none",
            "findings": len(sources.open_findings(roots)), "codex": sources.codex_login(), "api": api,
            "apiDetail": f"estimated ${b['estimated']:.2f}, reserved ${b['reserved']:.2f}, "
                         f"{b['unknown_calls']} paid call(s) with unknown cost, {b['subscription_calls']} subscription call(s)",
            **{k: self._home[k] for k in ("forYou", "galleryNew", "stepText", "apiVisible", "apiText")}}
        self._write_snapshot(counts, now)
        self.boardsChanged.emit()
        if self._selected.get("kind") == "session":
            self._refresh_selected_session()
        elif self._selected.get("kind") == "call":
            call = next((c for c in self._calls if c["call_id"] == self._selected.get("id")), None)
            if call and (call["status"], call.get("verdict")) != (self._selected.get("status"), self._selected.get("verdict")):
                self._show_call(call)  # the call finished (or failed) while it was selected
        elif self._selected.get("kind") == "run":
            row = next((r for r in self._all_runs if r["rid"] == self._selected.get("id")), None)
            if row and (row.get("status"), row.get("exitsText")) != (self._selected.get("runStatus"), self._selected.get("exitsText")):
                self._show_run(row, follow=False)  # keep following the log; update the header and files

    def _publish(self, attr: str, value, signal) -> None:
        """Set a Home model and notify only on a real change, so QML keeps reply drafts, filters and scroll positions."""
        if getattr(self, attr) != value:
            setattr(self, attr, value)
            signal.emit()

    def _scan_gallery(self, roots, records, cards, now, force=False):
        ask_min = [{"session_id": c["sid"], "attachments": [{"path": a["path"]} for a in c["attachments"]]}
                   for c in cards if c["kind"] == "ask"]
        items = self._scanner.scan(self.main, roots, self._sessions, records, ask_min, now, force=force)
        titles = {s.sid: s.title or s.sid[:8] for s in self._sessions}
        actions = {}
        rows = []
        for item in items:
            key = (item["path"], item["mtime"])
            decoration = self._gallery_actions.get(key)
            if decoration is None:
                def open_how(path):
                    try:
                        kind = launch.plan_open(path, self.main, self.projects)[0]
                        return kind if kind in ("default", "browser", "notepad") else ""
                    except Exception:
                        return ""

                can_marimo = False
                if item["type"] == "notebook":
                    try:
                        launch.plan_command("marimo", self.main, notebook=item["path"])
                        can_marimo = True
                    except Exception:
                        pass
                decoration = {"openHow": open_how(item["path"]),
                              "sourceHow": open_how(item["source"]) if item["source"] != item["path"] else "",
                              "canMarimo": can_marimo}
            actions[key] = decoration
            rows.append({**item, **decoration, "sessionTitles": [titles.get(sid, sid[:8]) for sid in item["sessions"]]})
        self._gallery_actions = actions
        self._publish("_gallery", rows, self.galleryChanged)
        return items

    def _write_snapshot(self, counts, now):
        if counts == self._snapshot_counts and now - self._snapshot_at < 60:
            return
        temporary = None
        try:
            self.data.mkdir(parents=True, exist_ok=True)
            with tempfile.NamedTemporaryFile(mode="w", encoding="utf-8", dir=self.data, prefix="home-", suffix=".tmp",
                                             delete=False) as f:
                temporary = Path(f.name)
                json.dump(home.snapshot(counts, now), f)
                f.write("\n")
            os.replace(temporary, self.data / home.SNAPSHOT_NAME)
            self._snapshot_counts, self._snapshot_at = dict(counts), now
        except OSError as exc:
            if not self._snapshot_error_shown:
                self._snapshot_error_shown = True
                self._say(f"Could not save Home counts: {exc.strerror or exc}.")
        finally:
            if temporary is not None:
                try:
                    temporary.unlink(missing_ok=True)
                except OSError:
                    pass

    @Property("QVariantList", notify=inboxChanged)
    def inbox(self):
        return self._inbox

    @Property("QVariantList", notify=inboxChanged)
    def answered(self):
        return self._answered

    @Property("QVariantList", notify=galleryChanged)
    def gallery(self):
        return self._gallery

    @Property("QVariantList", notify=gallerySessionsChanged)
    def gallerySessions(self):
        return self._gallery_sessions

    @Property("QVariantMap", notify=boardsChanged)
    def home(self):
        return self._home

    @Property("QVariantMap", notify=homeProgressChanged)
    def homeProgress(self):
        return self._home_progress

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

    @Property("QVariantList", notify=boardsChanged)
    def runs(self):
        return self._runs

    @Property("QVariantMap", notify=boardsChanged)
    def registry(self):
        return self._registry

    @Property("QVariantList", notify=boardsChanged)
    def flags(self):
        return self._flags

    @Property("QVariantList", notify=checkoutsChanged)
    def checkouts(self):
        return self._checkouts

    @Property("QVariantMap", constant=True)
    def codexChoices(self):
        return {"kinds": list(runs.CODEX_KINDS), "models": list(runs.CODEX_MODELS),
                "efforts": list(runs.CODEX_EFFORTS), "speeds": list(runs.CODEX_SPEEDS)}

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
            if source == "runlog":
                rows += [{"t": None, "time": "", "kind": "tool" if line.startswith(("$ ", "[exit ")) else "result",
                          "who": "", "title": "", "body": line[:BODY_PREVIEW], "full": line[:FULL_MAX]} for line in tail.poll()]
                continue
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

    # ------------------------------------------------------------ runs and attention

    @Slot(str)
    def selectRun(self, rid: str):
        row = next((r for r in self._all_runs if r["rid"] == rid), None)
        if row is None:
            return self._say("That run is no longer listed.")
        self._show_run(row, follow=True)

    def _show_run(self, row: dict, follow: bool):
        rid = row["rid"]
        d = self.data / "runs"
        if follow:
            if self._watcher.files():
                self._watcher.removePaths(self._watcher.files())
            self._tails = [(runs.LogTail(d / f"{rid}.log"), "runlog")]
            self._timeline.reset()
            self.poll()
            self._poll.start()
        files = [{"label": f"{name} (private, plain text)", "path": str(d / name), "marimo": False}
                 for name in (f"{rid}.log", f"{rid}.json") if (d / name).is_file()]
        checkout = Path(str(row.get("checkout") or ""))
        for rel in row.get("outputs") if isinstance(row.get("outputs"), list) else []:
            if isinstance(rel, str) and (checkout / rel).is_file():
                files.append({"label": rel, "path": str(checkout / rel), "marimo": False})
        self._files = files
        live = row.get("status") == "running" and row.get("stale_heartbeat_s")
        detail = f"{row['checkoutName']}; exits {row['exitsText'] or 'none'}; {row['durationText'] or 'not started'}"
        if live:
            detail += f"; no heartbeat for {duration_text(row['stale_heartbeat_s'])}"
        if row["reasonText"]:
            detail += f"; {row['reasonText']}"
        self._selected = {"kind": "run", "id": rid, "title": f"{row.get('title') or row.get('run')} ({rid})",
                          "status": row["tone"], "runStatus": row.get("status"), "exitsText": row["exitsText"],
                          "detail": detail, "cancellable": row.get("status") in ("starting", "running")}
        self._notes = []
        self.selectedChanged.emit()
        self.notesChanged.emit()

    @Slot(str, str)
    def selectFlag(self, target_kind: str, target_id: str):
        {"session": self.selectSession, "call": self.selectCall, "run": self.selectRun}.get(
            target_kind, lambda _i: self._say("Unknown item."))(target_id)

    def _checkout(self, checkout: str) -> Path | None:
        """The chosen checkout, only if it is one of this repository's; the plans check it again."""
        want = os.path.normcase(str(Path(checkout).resolve())) if checkout else ""
        return next((p for p in paths.checkouts(self.main) if os.path.normcase(str(p)) == want), None)

    @Slot(str, result="QVariantList")
    def packetsFor(self, checkout: str):
        c = self._checkout(checkout)
        return runs.packets(c) if c else []

    @Slot(str, str, str)
    def startRun(self, run_id: str, checkout: str, digest: str):
        c = self._checkout(checkout)
        if c is None:
            return self._say("Not started: choose a checkout of this repository.")
        try:
            rid = self.launcher.start_run(self.main, c, run_id, digest)
        except runs.RunRefused as exc:
            return self._say(f"Not started: {exc}.")
        except OSError as exc:
            return self._say(f"Could not start the runner: {exc.strerror or exc}.")
        self._say(f"Run {run_id} started in {c.name} ({rid}).")
        self._follow_new_run(rid)

    def _follow_new_run(self, rid: str):
        self.refresh()
        row = next((r for r in self._all_runs if r["rid"] == rid), None)
        if row is not None:
            self._show_run(row, follow=True)

    @Slot(str)
    def cancelRun(self, rid: str):
        if not re.match(runs.RID_RE, rid or ""):
            return self._say("Not a run id.")
        try:
            self.launcher.cancel_run(self.data, rid)
        except OSError as exc:
            return self._say(f"Could not ask the run to stop: {exc.strerror or exc}.")
        self._say(f"Stop requested for run {rid}; the runner ends its process tree within a second.")

    @Slot(str, str, str, str, str, str)
    def startCodex(self, checkout: str, kind: str, packet: str, model: str, effort: str, speed: str):
        c = self._checkout(checkout)
        if c is None:
            return self._say("Not started: choose a checkout of this repository.")
        spec = {"kind": kind, "packet": packet, "model": model, "effort": effort, "speed": speed}
        try:
            rid = self.launcher.start_codex(self.main, c, spec)
        except runs.RunRefused as exc:
            return self._say(f"Not started: {exc}.")
        except OSError as exc:
            return self._say(f"Could not start the runner: {exc.strerror or exc}.")
        rule = " No commits, tags or pushes in the repository until it ends." if kind == "implement" else ""
        self._say(f"Codex {kind} call on {packet} started in {c.name} through the runner ({rid}).{rule}")
        self._follow_new_run(rid)

    @Slot(str, str, bool)
    def startSession(self, checkout: str, packet: str, remote: bool):
        try:
            argv, cwd = launch.plan_session(self.main, checkout, packet, remote)
            self.launcher.command(argv, cwd)
        except launch.LaunchRefused as exc:
            return self._say(f"Not started: {exc}.")
        except OSError as exc:
            return self._say(f"Could not start Claude Code: {exc.strerror or exc}.")
        self._say(f"New Claude Code session started in {Path(cwd).name}: {argv[1]}")

    # ------------------------------------------------------------ owner notes

    @Slot(str, str, result=bool)
    def answerCard(self, cardId: str, body: str) -> bool:
        card = next((c for c in self._inbox if c["id"] == cardId), None)
        s = self._session(card["sid"]) if card else None
        if card is None or not card["answerRef"] or s is None:
            self._say("That card cannot be answered here.")
            return False
        if card["state"] != "open":
            self._say("That card is already answered; the answer is queued.")
            return False
        try:
            text = home.answer_text(card["answerRef"], body)
            notes.append_note(self.data, s.sid, text)
        except (ValueError, notes.NoteRejected, OSError) as exc:
            self._say(f"Answer not queued: {exc}.")
            return False
        message = "Answer queued; Claude receives it at its next tool step."
        if s.status.status != "running":
            message += " The session is idle: open it in Claude Code and send any message."
        self._say(message)
        self.refresh()
        return True

    @Slot(str)
    def openCardSession(self, cardId: str):
        card = next((c for c in self._inbox + self._answered if c["id"] == cardId), None)
        if card is None or self._session(card["sid"]) is None:
            return self._say("That session is no longer listed.")
        try:
            self.selectSession(card["sid"])
            self.openInClaude(False)
        except (OSError, ValueError) as exc:
            self._say(f"Could not open the session: {exc}.")

    @Slot(str, str, str, str, result=bool)
    def fileNote(self, path: str, action: str, text: str, sid: str) -> bool:
        """A star, wrong-direction or comment note about a gallery file, sent to `sid`: one of the
        sessions the gallery lists for that file, chosen by the owner in the preview."""
        if action not in ("star", "wrong", "comment") or not isinstance(text, str):
            self._say("Unknown file note action.")
            return False
        row = next((r for r in self._gallery if r["path"] == path), None)
        if row is None or not row["sessions"]:
            self._say("No session produced this file.")
            return False
        if sid not in row["sessions"] or self._session(sid) is None:
            self._say("Choose one of the sessions listed for this file.")
            return False
        text = text.strip()
        if action == "comment" and not text:
            self._say("Write a comment first.")
            return False
        label = f"{row['checkout']}/{row['rel']}"
        body = (f"star: {label}" if action == "star" else f"wrong direction: {label}" + (f": {text}" if text else "")
                if action == "wrong" else f"comment on {label}: {text}")
        try:
            notes.append_note(self.data, sid, body)
        except (notes.NoteRejected, OSError) as exc:
            self._say(f"Note not sent: {exc}.")
            return False
        self._say("File note queued; Claude receives it at its next tool step.")
        return True

    @Slot(str, str, result=bool)
    def sessionCommand(self, kind: str, text: str) -> bool:
        if kind not in ("pause", "resume", "why", "priority") or not isinstance(text, str):
            self._say("Unknown session command.")
            return False
        if self._selected.get("kind") != "session" or self._session(self._selected.get("id")) is None:
            self._say("Select a session first.")
            return False
        if kind == "priority" and not text.strip():
            self._say("Write a priority first.")
            return False
        body = f"[{kind}]" + (f" {text.strip()}" if kind == "priority" else "")
        try:
            if not self.sendNote(body):
                return False
        except OSError as exc:
            self._say(f"Note not sent: {exc}.")
            return False
        if kind == "resume" and self._session(self._selected["id"]).status.status != "running":
            message = "Send any message in Claude Code; the session reads [resume] at its first tool step."
            try:
                self.launcher.claude()
            except OSError as exc:
                message += f" Could not bring Claude Code forward: {exc}."
            self._say(message)
        return True

    @Slot()
    def rescanGallery(self):
        now = time.time()
        try:
            items = self._scan_gallery(paths.checkouts(self.main), self._records, self._cards, now, force=True)
        except (OSError, ValueError) as exc:
            return self._say(f"Could not rescan the gallery: {exc}.")
        counts = home.counts(self._cards, items)
        for key, value in (("forYou", counts["for_you"]), ("galleryNew", counts["gallery_new"])):
            self._home[key] = self._compact[key] = value
        self._write_snapshot(counts, now)
        self.boardsChanged.emit()
        self._say("Gallery rescanned.")

    @Slot(str)
    def showCard(self, cardId: str):
        if not any(c["id"] == cardId for c in self._inbox + self._answered):
            return self._say("That card is no longer listed.")
        self.focusCard.emit(cardId)

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


def open_card(engine: QQmlApplicationEngine, wb: Workbench, card_id: str) -> None:
    """Bring the full window forward on a card. After --compact startup the full window is loaded
    into the same engine on the first click, so a notification always opens the card it announced."""
    def full():
        return next((w for w in engine.rootObjects() if w.objectName() == "mainWindow"), None)

    window = full()
    if window is None:
        engine.setInitialProperties({})   # the compact window's initial properties are not the full window's
        engine.load(QUrl.fromLocalFile(str(QML_DIR / "Main.qml")))
        window = full()
        if window is None:
            return
    window.show()
    window.raise_()
    window.requestActivate()
    wb.showCard(card_id)


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description="Magic 600 Cell development workbench")
    parser.add_argument("--compact", action="store_true", help="open only the always-on-top progress view")
    args = parser.parse_args(argv)
    main_dir = paths.main_checkout(Path(__file__).resolve().parent)
    if main_dir is None:
        print("workbench: not inside a checkout of this repository", file=sys.stderr)
        return 2
    QQuickStyle.setStyle("Fusion")
    app = QApplication(sys.argv[:1])
    app.setApplicationName("Magic 600 Cell workbench")
    QImageReader.setAllocationLimit(192)
    wb = Workbench(main_dir)
    engine = QQmlApplicationEngine()
    if not load_ui(engine, wb, args.compact):
        return 1
    window = engine.rootObjects()[0]
    tray = None
    if QSystemTrayIcon.isSystemTrayAvailable():
        pixmap = QPixmap(32, 32)
        pixmap.fill(Qt.GlobalColor.transparent)
        painter = QPainter(pixmap)
        painter.setRenderHint(QPainter.RenderHint.Antialiasing)
        painter.setPen(Qt.PenStyle.NoPen)
        painter.setBrush(QColor("#3569a8"))
        painter.drawEllipse(2, 2, 28, 28)
        painter.end()
        tray = QSystemTrayIcon(QIcon(pixmap), app)
        tray.setToolTip("Magic 600 Cell workbench")
        tray.show()
    notifier = Notifier(tray, window)
    notifier.update(wb.inbox)
    wb.inboxChanged.connect(lambda: notifier.update(wb.inbox))
    notifier.openCard.connect(lambda card_id: open_card(engine, wb, card_id))
    return app.exec()


if __name__ == "__main__":
    sys.exit(main())
