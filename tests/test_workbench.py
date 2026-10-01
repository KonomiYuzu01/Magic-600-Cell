"""Headless tests for the development workbench (standard library only).

Every test builds fresh synthetic fixtures in a temporary directory: a fake main
checkout with a managed worktree and an external worktree, a fake Claude Code
projects directory, reviews, ledgers and inboxes. Real sessions, transcripts and
ledgers are never read. The reporting hook runs as a copy inside the fixture, so
its location fallback can only ever point at the fixture.
"""
from __future__ import annotations

import ast
import importlib.util
import json
import math
import os
import re
import shutil
import subprocess
import sys
import tempfile
import textwrap
import time
import unittest
from datetime import datetime, timedelta, timezone
from pathlib import Path
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
WB = ROOT / "tools" / "workbench"
HOOK = ROOT / ".claude" / "hooks" / "report_event.py"
sys.path.insert(0, str(WB))
sys.path.insert(0, str(ROOT / "tools" / "agents"))
import paths  # noqa: E402
import sources  # noqa: E402
import notes  # noqa: E402
import launch  # noqa: E402
import runs  # noqa: E402
import watch  # noqa: E402
import checklist  # noqa: E402
import home  # noqa: E402


def load_module(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


hook = load_module("report_event_under_test", HOOK)
statusline = load_module("statusline_under_test", WB / "statusline.py")

HOOK_ARG = "${CLAUDE_PROJECT_DIR}/.claude/hooks/report_event.py"
HOOK_GUARD = "import os,runpy,sys;p=sys.argv[1];os.path.isfile(p) and runpy.run_path(p,run_name='__main__')"
SID = "11111111-2222-3333-4444-555555555555"
T0 = datetime(2026, 9, 30, 12, 0, 0, tzinfo=timezone.utc)


def iso(seconds: float) -> str:
    return (T0 + timedelta(seconds=seconds)).isoformat().replace("+00:00", "Z")


def at(seconds: float) -> float:
    return (T0 + timedelta(seconds=seconds)).timestamp()


def user(t, text="do it", cwd=None, **extra):
    return {"type": "user", "timestamp": iso(t), "cwd": cwd, "sessionId": SID,
            "message": {"role": "user", "content": text}, **extra}


def assistant(t, text=None, uses=(), stop=None, **extra):
    content = ([{"type": "text", "text": text}] if text else []) + [
        {"type": "tool_use", "id": u[0], "name": u[1], "input": u[2]} for u in uses]
    return {"type": "assistant", "timestamp": iso(t), "sessionId": SID,
            "message": {"role": "assistant", "content": content, "stop_reason": stop}, **extra}


def result(t, use_id, text="ok", error=False):
    return {"type": "user", "timestamp": iso(t), "sessionId": SID,
            "message": {"role": "user", "content": [{"type": "tool_result", "tool_use_id": use_id,
                                                     "content": text, "is_error": error}]}}


def ev(t, name, **fields):
    return {"v": 1, "t": iso(t), "e": name, "sid": SID, **fields}


def sample_status() -> dict:
    """The synthetic schema-2 checklist shared with tests/test_workbench_home.py (imported late: that module imports this one)."""
    from test_workbench_home import sample_status as make
    return make()


class Fixture:
    """A fake repository with worktrees, a fake projects directory and private data roots."""

    def __init__(self):
        self.tmp = tempfile.TemporaryDirectory()
        base = Path(self.tmp.name).resolve()
        self.main = base / "repo main"
        self.wt = self.main / ".claude" / "worktrees" / "wt1"
        self.ext = base / "elsewhere" / "feature"
        self.outsider = base / "repo main-other"  # its slug extends the main slug
        self.projects = base / "claude-projects"
        (self.main / ".git").mkdir(parents=True)
        for name, tree in (("wt1", self.wt), ("feature", self.ext)):
            meta = self.main / ".git" / "worktrees" / name
            meta.mkdir(parents=True)
            (meta / "commondir").write_text("../..\n", encoding="utf-8")
            (meta / "gitdir").write_text(str(tree / ".git") + "\n", encoding="utf-8")
            tree.mkdir(parents=True)
            (tree / ".git").write_text(f"gitdir: {meta}\n", encoding="utf-8")
        (self.outsider / ".git").mkdir(parents=True)
        self.projects.mkdir()
        self.data = paths.data_root(self.main)
        hooks = self.main / ".claude" / "hooks"
        hooks.mkdir(parents=True)
        self.hook = hooks / "report_event.py"
        shutil.copy2(HOOK, self.hook)

    def cleanup(self):
        self.tmp.cleanup()

    def transcript(self, checkout: Path, sid: str, records: list, raw_lines=(), sub="") -> Path:
        d = self.projects / paths.slug(checkout / sub if sub else checkout)
        d.mkdir(parents=True, exist_ok=True)
        path = d / f"{sid}.jsonl"
        with path.open("w", encoding="utf-8", newline="\n") as f:
            for r in records:
                f.write(json.dumps(r) + "\n")
            for line in raw_lines:
                f.write(line + "\n")
        return path

    def events(self, sid: str, records: list) -> None:
        d = self.data / "events"
        d.mkdir(parents=True, exist_ok=True)
        with (d / f"{sid}.jsonl").open("a", encoding="utf-8", newline="\n") as f:
            for r in records:
                f.write(json.dumps(r) + "\n")

    def run_hook(self, payload, cwd=None, env_extra=None, raw: bytes | None = None):
        env = {k: v for k, v in os.environ.items() if k != "CLAUDE_PROJECT_DIR"}
        env.update(env_extra or {})
        data = raw if raw is not None else json.dumps(payload).encode("utf-8")
        return subprocess.run([sys.executable, str(self.hook)], input=data, capture_output=True,
                              cwd=str(cwd or self.main), env=env, timeout=30)


def hook_payload(fx: Fixture, name: str, sid=SID, **fields):
    return {"session_id": sid, "hook_event_name": name, "cwd": str(fx.main), "transcript_path": "unused", **fields}


class PathsTests(unittest.TestCase):
    def setUp(self):
        self.fx = Fixture()
        self.addCleanup(self.fx.cleanup)

    def test_every_checkout_resolves_to_the_main_checkout_in_all_three_copies(self):
        fx = self.fx
        (fx.main / "docs").mkdir()
        for start in (fx.main, fx.main / "docs", fx.wt, fx.ext):
            with self.subTest(start=start.name):
                self.assertEqual(paths.main_checkout(start), fx.main)
                self.assertEqual(hook.main_checkout(start), fx.main)
                self.assertEqual(statusline.main_checkout(start), fx.main)
        self.assertEqual(paths.worktrees(fx.main), [fx.ext, fx.wt])
        self.assertEqual(statusline.checkouts(fx.main), paths.checkouts(fx.main))

    def test_session_ids_are_validated(self):
        for sid in ("../x", "a/b", "", "x" * 65, None, 3):
            self.assertFalse(paths.valid_sid(sid))
        self.assertTrue(paths.valid_sid(SID))

    def test_digest_is_shared_by_hook_and_workbench(self):
        for obj in ({"command": "ls", "b": [1, 2]}, None, {"text": "é中"}):
            self.assertEqual(hook.digest(obj), sources.digest(obj))


class DiscoveryTests(unittest.TestCase):
    def setUp(self):
        self.fx = Fixture()
        self.addCleanup(self.fx.cleanup)

    def test_sessions_of_every_checkout_are_found_and_outsiders_are_not(self):
        fx = self.fx
        (fx.main / "docs").mkdir()
        fx.transcript(fx.main, "aaaaaaaa-0000-0000-0000-000000000001", [user(0, cwd=str(fx.main))])
        fx.transcript(fx.main, "aaaaaaaa-0000-0000-0000-000000000002", [user(0, cwd=str(fx.main / "docs"))], sub="docs")
        fx.transcript(fx.wt, "aaaaaaaa-0000-0000-0000-000000000003", [user(0, cwd=str(fx.wt))])
        fx.transcript(fx.ext, "aaaaaaaa-0000-0000-0000-000000000004", [user(0, cwd=str(fx.ext))])
        fx.transcript(fx.outsider, "aaaaaaaa-0000-0000-0000-000000000005", [user(0, cwd=str(fx.outsider))])
        found = {s.sid[-1]: s for s in sources.discover_sessions(fx.main, fx.projects, now=at(60))}
        self.assertEqual(sorted(found), ["1", "2", "3", "4"])
        self.assertEqual(found["3"].checkout, fx.wt)  # the most specific checkout
        self.assertEqual(found["4"].checkout, fx.ext)

    def test_malformed_partial_and_unknown_lines_are_skipped_and_counted(self):
        fx = self.fx
        path = fx.transcript(fx.main, SID, [user(0, cwd=str(fx.main)), {"type": "future-kind", "x": 1},
                                            assistant(5, "hi", stop="end_turn")],
                             raw_lines=["{not json", "[1, 2]", "\xff\xfe"])
        with path.open("ab") as f:
            f.write(b'{"type": "assistant", "timestamp": "partial')  # still being written
        (s,) = sources.discover_sessions(fx.main, fx.projects, now=at(60))
        self.assertEqual(s.malformed, 3 * 2)  # counted once by the head read and once by the tail read
        self.assertEqual(s.status.status, "waiting")

    def small_record_limit(self, limit=1024 * 1024):
        saved = sources.Tail.MAX_RECORD
        sources.Tail.MAX_RECORD = limit
        self.addCleanup(setattr, sources.Tail, "MAX_RECORD", saved)

    def test_oversized_records_keep_the_session_listed(self):
        fx = self.fx
        self.small_record_limit()
        big = "x" * (600 * 1024)
        fx.transcript(fx.main, "bbbbbbbb-0000-0000-0000-000000000001",
                      [user(0, cwd=str(fx.main)), assistant(1, uses=[("u1", "Bash", {"command": "ls"})]),
                       result(2, "u1", big)])
        huge = "y" * (3 * 1024 * 1024)
        fx.transcript(fx.main, "bbbbbbbb-0000-0000-0000-000000000002", [user(0, text=huge, cwd=str(fx.main))])
        found = {s.sid[-1]: s for s in sources.discover_sessions(fx.main, fx.projects, now=at(60))}
        self.assertEqual(found["1"].status.status, "running")  # a large record within the limit is read
        self.assertEqual(found["2"].status.status, "unknown")  # identity from the record's start, content unreadable
        self.assertIn("exceeds read budget", found["2"].status.detail)

    def test_unreadable_newer_record_after_session_end_blocks_resume(self):
        fx = self.fx
        self.small_record_limit()
        fx.transcript(fx.main, SID, [user(0, cwd=str(fx.main)), user(21, text="z" * (3 * 1024 * 1024))])
        fx.events(SID, [ev(10, "SessionEnd", reason="exit")])
        (s,) = sources.discover_sessions(fx.main, fx.projects, now=at(60))
        self.assertEqual(s.status.status, "unknown")
        with self.assertRaises(launch.LaunchRefused):
            launch.plan_command("resume", fx.main, sid=SID, session_status=s.status.status)

    def test_state_is_incremental_and_keeps_waits_beyond_any_window(self):
        fx = self.fx
        path = fx.transcript(fx.main, SID, [user(0, cwd=str(fx.main)),
                                            assistant(1, uses=[("q1", "AskUserQuestion", {"q": 1})])])
        cache = {}
        (s,) = sources.discover_sessions(fx.main, fx.projects, now=at(60), cache=cache)
        state = cache[str(path)]
        self.assertEqual(s.status.status, "waiting")
        # Unrelated parallel work pushes the question far behind any tail window.
        for i in range(6):
            with path.open("a", encoding="utf-8") as f:
                f.write(json.dumps(assistant(2 + i, uses=[(f"b{i}", "Bash", {"command": "ls"})])) + "\n")
                f.write(json.dumps(result(2 + i, f"b{i}", "r" * (80 * 1024))) + "\n")
            (s,) = sources.discover_sessions(fx.main, fx.projects, now=at(60), cache=cache)
            self.assertEqual(s.status.status, "waiting")
        self.assertIs(cache[str(path)], state)  # the same state, fed only the appended records
        with path.open("a", encoding="utf-8") as f:
            f.write(json.dumps(result(30, "q1", "answer")) + "\n")
        (s,) = sources.discover_sessions(fx.main, fx.projects, now=at(60), cache=cache)
        self.assertEqual(s.status.status, "running")
        # A first read of the same history from scratch agrees.
        (fresh,) = sources.discover_sessions(fx.main, fx.projects, now=at(60))
        self.assertEqual(fresh.status.status, "running")

    def test_permission_resolved_long_after_its_tool_use(self):
        fx = self.fx
        cmd = {"command": "rm build"}
        path = fx.transcript(fx.main, SID, [user(0, cwd=str(fx.main)), assistant(1, uses=[("p1", "Bash", cmd)])]
                             + [result(2, f"x{i}", "r" * (80 * 1024)) for i in range(6)])
        fx.events(SID, [ev(1.5, "PermissionRequest", tool="Bash", key=sources.digest(cmd))])
        cache = {}
        (s,) = sources.discover_sessions(fx.main, fx.projects, now=at(60), cache=cache)
        self.assertEqual(s.status.status, "waiting")
        with path.open("a", encoding="utf-8") as f:
            f.write(json.dumps(result(40, "p1", "denied", error=True)) + "\n")
        (s,) = sources.discover_sessions(fx.main, fx.projects, now=at(60), cache=cache)
        self.assertEqual(s.status.status, "running")

    def test_child_transcript_waits_and_failures_reach_the_session(self):
        fx = self.fx
        fx.transcript(fx.main, SID, [user(0, cwd=str(fx.main)), assistant(1, uses=[("a1", "Agent", {})])])
        sub = fx.projects / paths.slug(fx.main) / SID / "subagents"
        sub.mkdir(parents=True)
        child = sub / "agent-c1.jsonl"
        child.write_text(json.dumps(assistant(2, uses=[("cq", "AskUserQuestion", {"q": 2})])) + "\n", encoding="utf-8")
        failed = assistant(3, "overloaded")
        failed["isApiErrorMessage"] = True
        (sub / "agent-c2.jsonl").write_text(json.dumps(failed) + "\n", encoding="utf-8")
        cache = {}
        (s,) = sources.discover_sessions(fx.main, fx.projects, now=at(60), cache=cache)
        self.assertEqual(s.status.status, "waiting")  # parent activity cannot hide a child's question
        self.assertEqual(s.status.subagents["c1"]["status"], "running")
        self.assertEqual(s.status.subagents["c2"]["status"], "failed")
        with child.open("a", encoding="utf-8") as f:
            f.write(json.dumps(result(4, "cq", "yes")) + "\n")
        (s,) = sources.discover_sessions(fx.main, fx.projects, now=at(60), cache=cache)
        self.assertEqual(s.status.status, "running")

    def small_reads(self, max_read=64 * 1024, per_refresh=128 * 1024):
        saved = (sources.Tail.MAX_READ, sources.HISTORY_PER_REFRESH)
        sources.Tail.MAX_READ, sources.HISTORY_PER_REFRESH = max_read, per_refresh
        self.addCleanup(lambda: (setattr(sources.Tail, "MAX_READ", saved[0]), setattr(sources, "HISTORY_PER_REFRESH", saved[1])))

    def test_history_read_over_several_refreshes_is_reduced_in_time_order(self):
        fx = self.fx
        self.small_reads()
        big = "r" * (100 * 1024)
        records = [user(0, cwd=str(fx.main))] + [result(t, f"x{t}", big) for t in (1, 2, 3, 4)] + [
            assistant(5, "done", stop="end_turn"), user(6, "next"), assistant(7, uses=[("p1", "Bash", {"command": "rm b"})])]
        fx.transcript(fx.main, SID, records)
        fx.events(SID, [ev(8, "PermissionRequest", tool="Bash", key=sources.digest({"command": "rm b"}))])
        cache, refreshes = {}, 0
        while True:
            (s,) = sources.discover_sessions(fx.main, fx.projects, now=at(60), cache=cache)
            refreshes += 1
            if not s.status.detail.startswith("reading history"):
                break
        self.assertGreater(refreshes, 2)  # the history really took several refreshes
        self.assertEqual((s.status.status, s.status.waits), ("waiting", ["permission (Bash)"]))
        sigs, _ = sources.transcript_signals(records)
        full = sources.reduce_status(sigs + sources.event_signals(
            [ev(8, "PermissionRequest", tool="Bash", key=sources.digest({"command": "rm b"}))], len(sigs)), now=at(60))
        self.assertEqual(full.status, s.status.status)

    def run_to_caught_up(self, cache):
        for _ in range(50):
            (s,) = sources.discover_sessions(self.fx.main, self.fx.projects, now=at(60), cache=cache)
            if not s.status.detail.startswith("reading history"):
                return s
        self.fail("history never caught up")

    def equal_time_history(self, big: int):
        fx = self.fx
        body = "r" * big
        cmd = {"command": "rm b"}
        records = [user(0, cwd=str(fx.main))] + [result(t, f"x{i}", body) for i, t in enumerate((1, 2, 4, 4))] + [
            assistant(4, "done", stop="end_turn"), user(4, "next"), assistant(4, uses=[("p1", "Bash", cmd)])]
        events = [ev(4, "PermissionRequest", tool="Bash", key=sources.digest(cmd))]
        fx.transcript(fx.main, SID, records)
        fx.events(SID, events)
        sigs, _ = sources.transcript_signals(records)
        return sources.reduce_status(sigs + sources.event_signals(events, len(sigs)), now=at(60))

    def test_equal_timestamps_at_the_watermark_small_reads(self):
        self.small_reads()
        full = self.equal_time_history(100 * 1024)
        s = self.run_to_caught_up({})
        self.assertEqual(full.status, "waiting")
        self.assertEqual((s.status.status, s.status.waits), (full.status, full.waits))

    def test_equal_timestamps_at_the_watermark_default_limits(self):
        full = self.equal_time_history(10 * 1024 * 1024)  # Codex's counterexample: about 40 MiB of history
        s = self.run_to_caught_up({})
        self.assertEqual((s.status.status, s.status.waits), ("waiting", ["permission (Bash)"]))
        self.assertEqual(s.status.status, full.status)

    def test_refresh_ending_inside_a_shared_timestamp_holds_it_back(self):
        fx = self.fx
        self.small_reads()  # 64 KiB reads, 128 KiB per refresh: refresh one stops inside the records at t=4
        cmd = {"command": "rm b"}
        body = "r" * (100 * 1024)
        records = [user(0, cwd=str(fx.main)), result(4, "x1", body), result(4, "x2", body),
                   assistant(4, "done", stop="end_turn"), user(4, "next"), assistant(4, uses=[("p1", "Bash", cmd)])]
        fx.transcript(fx.main, SID, records)
        fx.events(SID, [ev(4, "PermissionRequest", tool="Bash", key=sources.digest(cmd))])
        cache = {}
        (first,) = sources.discover_sessions(fx.main, fx.projects, now=at(60), cache=cache)
        state = next(iter(cache.values()))
        self.assertEqual(state.frontier[id(state.tail)], at(4))  # the refresh ended on the shared timestamp
        s = self.run_to_caught_up(cache)
        self.assertEqual((s.status.status, s.status.waits), ("waiting", ["permission (Bash)"]))

    def test_unreadable_newest_child_record_blocks_resume(self):
        fx = self.fx
        self.small_record_limit()
        fx.transcript(fx.main, SID, [user(0, cwd=str(fx.main))])
        fx.events(SID, [ev(1, "SessionEnd", reason="exit")])
        sub = fx.projects / paths.slug(fx.main) / SID / "subagents"
        sub.mkdir(parents=True)
        (sub / "agent-c1.jsonl").write_text(
            json.dumps(assistant(2, uses=[("cq", "AskUserQuestion", {"q": "z" * (2 * 1024 * 1024)})])) + "\n", encoding="utf-8")
        cache = {}
        for _ in range(3):
            (s,) = sources.discover_sessions(fx.main, fx.projects, now=at(60), cache=cache)
        self.assertEqual((s.status.status, s.status.detail), ("unknown", "record exceeds read budget"))
        with self.assertRaises(launch.LaunchRefused):
            launch.plan_command("resume", fx.main, sid=SID, session_status=s.status.status)

    def test_malformed_field_types_never_abort_discovery(self):
        fx = self.fx
        weird = [user(0, cwd=str(fx.main)),
                 {"type": "assistant", "timestamp": iso(1), "message": {"content": [
                     {"type": "tool_use", "id": [], "name": {"x": 1}, "input": {}},
                     {"type": "tool_use", "id": "b", "name": "Bash", "input": [1]},
                     {"type": "text", "text": ["not", "text"]}]}},
                 {"type": "user", "timestamp": iso(2), "message": {"content": [{"type": "tool_result", "tool_use_id": {"a": 1}, "content": 5}]}},
                 {"type": "custom-title", "customTitle": 7}]
        fx.transcript(fx.main, SID, weird)
        fx.transcript(fx.wt, "cccccccc-0000-0000-0000-000000000001", [user(0, cwd=str(fx.wt))])
        fx.events(SID, [{"e": "PostToolUse", "t": iso(3), "agent_id": ["x"], "tool": 5, "key": {}}])
        sessions = sources.discover_sessions(fx.main, fx.projects, now=at(60))
        self.assertEqual(len(sessions), 2)
        sources.link_calls(sessions, [{"call_id": "20260930T120000Z-aaaaaaaa", "status": "running", "time": at(1), "checkout": str(fx.main)}])

    def test_subagents_from_events_and_files(self):
        fx = self.fx
        fx.transcript(fx.main, SID, [user(0, cwd=str(fx.main)), assistant(1, uses=[("a1", "Agent", {})])])
        fx.events(SID, [ev(2, "SubagentStart", agent_id="ag1", agent_type="Explore"),
                        ev(3, "SubagentStart", agent_id="ag2", agent_type="Plan"), ev(9, "SubagentStop", agent_id="ag1")])
        sub = fx.projects / paths.slug(fx.main) / SID / "subagents"
        sub.mkdir(parents=True)
        (sub / "agent-ag3.jsonl").write_text(json.dumps(assistant(4, "x", stop="end_turn")) + "\n", encoding="utf-8")
        (s,) = sources.discover_sessions(fx.main, fx.projects, now=at(60))
        subs = s.status.subagents
        self.assertEqual(subs["ag1"]["status"], "finished")
        self.assertEqual(subs["ag2"]["status"], "running")
        self.assertEqual(subs["ag3"]["status"], "finished")


class StatusTests(unittest.TestCase):
    def status(self, records=(), events=(), now=100.0):
        sigs, _ = sources.transcript_signals(list(records))
        sigs += sources.event_signals(list(events), start_seq=len(sigs))
        return sources.reduce_status(sigs, now=at(now))

    def test_turn_states(self):
        self.assertEqual(self.status([user(0), assistant(1, "hi", stop="end_turn")]).status, "waiting")
        self.assertEqual(self.status([user(0), assistant(1, uses=[("u", "Bash", {})])]).status, "running")
        st = self.status([user(0), assistant(1, uses=[("u", "Bash", {})])], now=2000)
        self.assertEqual((st.status, st.detail[:9]), ("running", "no output"))
        self.assertEqual(self.status([user(0)], [ev(1, "StopFailure")]).status, "failed")

    def test_end_and_resume(self):
        st = self.status([user(0), user(20, "again"), assistant(21, uses=[("u", "Bash", {})])], [ev(10, "SessionEnd", reason="exit")])
        self.assertEqual(st.status, "running")
        st = self.status([user(0)], [ev(10, "SessionEnd", reason="exit")])
        self.assertEqual((st.status, st.detail), ("finished", "session ended (exit)"))

    def test_failure_then_retry_and_failure_then_end(self):
        self.assertEqual(self.status([user(0), user(20, "retry")], [ev(10, "StopFailure")]).status, "running")
        st = self.status([user(0)], [ev(10, "StopFailure"), ev(20, "SessionEnd", reason="exit")])
        self.assertEqual(st.status, "finished")
        self.assertTrue(st.detail.endswith("after failure"))

    def test_permission_wait_clears_only_on_its_own_resolution(self):
        cmd = {"command": "rm -rf build"}
        key = sources.digest(cmd)
        base = [user(0), assistant(1, uses=[("u1", "Bash", cmd)])]
        wait = [ev(2, "PermissionRequest", tool="Bash", key=key)]
        self.assertEqual(self.status(base, wait).status, "waiting")
        # A child's tool completion does not answer the parent's prompt.
        child = wait + [ev(3, "PostToolUse", tool="Bash", key=key, agent_id="child")]
        self.assertEqual(self.status(base, child).status, "waiting")
        # Neither does another tool of the parent with different input.
        other = wait + [ev(3, "PostToolUse", tool="Bash", key=sources.digest({"command": "ls"}))]
        self.assertEqual(self.status(base, other).status, "waiting")
        answered = wait + [ev(4, "PostToolUse", tool="Bash", key=key)]
        self.assertEqual(self.status(base, answered).status, "running")
        denied = self.status(base + [result(4, "u1", "denied", error=True)], wait)
        self.assertEqual(denied.status, "running")

    def test_open_question_survives_unrelated_completions(self):
        recs = [user(0), assistant(1, uses=[("q1", "AskUserQuestion", {"q": 1}), ("b1", "Bash", {"command": "ls"})]),
                result(2, "b1")]
        self.assertEqual(self.status(recs).status, "waiting")
        self.assertEqual(self.status(recs + [result(3, "q1", "answer")]).status, "running")

    def test_child_failure_during_parent_activity(self):
        st = self.status([user(0), assistant(1, uses=[("u", "Bash", {})])],
                         [ev(2, "SubagentStart", agent_id="c"), ev(3, "PostToolUseFailure", tool="Bash", agent_id="c")])
        self.assertEqual(st.status, "running")

    def test_idle_age_is_staleness_not_completion(self):
        st = self.status([user(0), assistant(1, "done", stop="end_turn")], now=7 * 3600)
        self.assertEqual(st.status, "waiting")
        self.assertIn("idle 7.0 h", st.detail)


class CodexCallTests(unittest.TestCase):
    def setUp(self):
        self.fx = Fixture()
        self.addCleanup(self.fx.cleanup)

    def review(self, checkout: Path, call_id: str, meta=None, result=None, packet=True, answered=None):
        d = checkout / "work" / "reviews" / call_id
        d.mkdir(parents=True)
        if packet:
            (d / "packet.md").write_text("packet", encoding="utf-8")
        if meta is not None:
            (d / "meta.json").write_text(json.dumps({"call_id": call_id, "kind": "review", **meta}), encoding="utf-8")
        if result is not None:
            (d / "review.json").write_text(json.dumps(result), encoding="utf-8")
        if answered is not None:
            (d / "dispositions.json").write_text(json.dumps(answered), encoding="utf-8")
        return d

    def ledger(self, checkout: Path, records: list):
        d = checkout / "work" / "loop-memory" / "ledgers"
        d.mkdir(parents=True, exist_ok=True)
        with (d / "codex.jsonl").open("a", encoding="utf-8") as f:
            for r in records:
                f.write(json.dumps(r) + "\n")

    def test_call_status_and_links(self):
        fx = self.fx
        findings = {"verdict": "findings", "summary": "s", "findings": [
            {"id": "F-1", "severity": "major"}, {"id": "F-2", "severity": "nit"}]}
        self.review(fx.main, "20260930T120000Z-aaaaaaaa", {"valid": True, "verdict": "findings"}, findings, answered={"F-1": "adopt"})
        self.review(fx.wt, "20260930T120100Z-bbbbbbbb", {"valid": False, "problems": ["model x != y"]})
        self.review(fx.ext, "20260930T120200Z-cccccccc")                      # running
        self.review(fx.main, "20260930T080000Z-dddddddd")                     # stale
        self.review(fx.main, "20260930T120300Z-eeeeeeee")
        self.ledger(fx.main, [{"call_id": "20260930T120300Z-eeeeeeee", "agent": "codex", "outcome": "timeout"}])
        calls = {c["call_id"][-1]: c for c in sources.codex_calls(paths.checkouts(fx.main), now=at(1800))}
        self.assertEqual(calls["a"]["status"], "finished")
        self.assertEqual(calls["a"]["blocking"], ["F-1"])
        self.assertEqual(calls["b"]["status"], "failed")
        self.assertEqual(calls["c"]["status"], "running")
        self.assertEqual(calls["d"]["status"], "failed")
        self.assertEqual(calls["e"]["status"], "failed")
        self.assertEqual(sources.open_findings(paths.checkouts(fx.main)), ["20260930T120000Z-aaaaaaaa:F-2"])
        # Links: by the printed call id after return, by time while running.
        fx.transcript(fx.main, SID, [user(0, cwd=str(fx.main)),
                                     assistant(1, uses=[("b", "Bash", {"command": "python tools/agents/codex_review.py --kind review"})]),
                                     result(2, "b", "review 20260930T120000Z-aaaaaaaa: findings; 2 finding(s)")])
        fx.transcript(fx.ext, "cccccccc-0000-0000-0000-000000000000",
                      [user(150, cwd=str(fx.ext)),
                       assistant(170, uses=[("b", "Bash", {"command": "python tools/agents/codex_review.py --kind plan"})])])
        sessions = sources.discover_sessions(fx.main, fx.projects, now=at(1800))
        links = sources.link_calls(sessions, list(calls.values()))
        self.assertEqual(links["20260930T120000Z-aaaaaaaa"], (SID, "call id"))
        self.assertEqual(links["20260930T120200Z-cccccccc"][1], "time")

    def test_budget_keeps_amounts_apart_and_matches_the_status_line(self):
        fx = self.fx
        self.ledger(fx.main, [
            {"call_id": "a", "channel": "paid-api", "cost": {"status": "reserved", "usd": 5}},
            {"call_id": "a", "channel": "paid-api", "cost": {"status": "billed", "usd": 3.5}},
            {"call_id": "b", "channel": "paid-api", "cost": {"status": "estimated", "usd": 1.25}},
            {"call_id": "c", "channel": "paid-api", "cost": {"status": "unknown", "usd": None}},
            {"call_id": "d", "channel": "subscription", "cost": {"status": "subscription", "usd": None}}])
        self.ledger(fx.wt, [{"call_id": "e", "channel": "paid-api", "cost": {"status": "billed", "usd": 1}}])
        b = sources.budget(paths.checkouts(fx.main))
        self.assertEqual(b, {"billed": 4.5, "estimated": 1.25, "reserved": 5.0, "unknown_calls": 1, "subscription_calls": 1,
                             "incomplete": False})
        s = statusline.budget(paths.checkouts(fx.main))
        keys = ("billed", "estimated", "reserved", "unknown_calls", "incomplete")
        self.assertEqual({k: s[k] for k in keys}, {k: b[k] for k in keys})

    def test_budget_is_cumulative_over_the_whole_ledger_or_marked_incomplete(self):
        fx = self.fx
        filler = [{"call_id": f"s{i}", "channel": "subscription", "cost": {"status": "subscription", "usd": None},
                   "pad": "p" * 400} for i in range(10_000)]  # about 4.6 MB after the paid record
        self.ledger(fx.main, [{"call_id": "old", "channel": "paid-api", "cost": {"status": "billed", "usd": 99}}] + filler)
        self.assertGreater((fx.main / "work/loop-memory/ledgers/codex.jsonl").stat().st_size, 4 * 1024 * 1024)
        roots = paths.checkouts(fx.main)
        self.assertEqual(sources.budget(roots)["billed"], 99.0)
        self.assertEqual(statusline.budget(roots)["billed"], 99.0)
        for mod in (sources, statusline):
            saved = mod.LEDGER_MAX
            mod.LEDGER_MAX = 1024 * 1024
            try:
                self.assertTrue(mod.budget(roots)["incomplete"])
            finally:
                mod.LEDGER_MAX = saved

    def test_unlisted_or_unread_ledgers_mark_the_status_line_total_incomplete(self):
        fx = self.fx
        paid = [{"call_id": "p", "channel": "paid-api", "cost": {"status": "billed", "usd": 2}}]
        self.ledger(fx.main, paid)
        roots = paths.checkouts(fx.main)
        self.assertEqual((statusline.budget(roots)["billed"], statusline.budget(roots)["incomplete"]), (2.0, False))
        real_open = Path.open

        def unreadable(self, *args, **kwargs):
            if self.suffix == ".jsonl":
                raise PermissionError("denied")
            return real_open(self, *args, **kwargs)

        with patch.object(Path, "open", unreadable):
            self.assertTrue(statusline.budget(roots)["incomplete"])
            out = statusline.line(fx.main)
        self.assertIn("API incomplete", out)
        self.assertNotIn("$", out)
        with patch.object(statusline.os, "scandir", side_effect=PermissionError("denied")):
            self.assertTrue(statusline.budget(roots)["incomplete"])
        d = fx.main / "work" / "loop-memory" / "ledgers"
        with patch.object(statusline, "LEDGER_FILES_MAX", 2):
            self.assertFalse(statusline.budget(roots)["incomplete"])
            for i in range(2):
                (d / f"extra{i}.txt").write_text("", encoding="utf-8")
            self.assertTrue(statusline.budget(roots)["incomplete"])
        for i in range(2):
            (d / f"extra{i}.txt").unlink()
        self.ledger(fx.wt, paid)
        size = (d / "codex.jsonl").stat().st_size
        with patch.object(statusline, "LEDGER_TOTAL_MAX", 2 * size):
            self.assertFalse(statusline.budget(roots)["incomplete"])
        with patch.object(statusline, "LEDGER_TOTAL_MAX", 2 * size - 1):
            self.assertTrue(statusline.budget(roots)["incomplete"])

    def test_a_ledger_record_spanning_read_boundaries_is_still_counted(self):
        fx = self.fx
        saved = sources.Tail.MAX_READ
        sources.Tail.MAX_READ = 4 * 1024
        self.addCleanup(setattr, sources.Tail, "MAX_READ", saved)
        d = fx.main / "work" / "loop-memory" / "ledgers"
        d.mkdir(parents=True)
        record = json.dumps({"call_id": "big", "channel": "paid-api", "cost": {"status": "billed", "usd": 99}})
        (d / "codex.jsonl").write_text(" " * (10 * 1024) + record + "\n", encoding="utf-8")  # legal leading whitespace
        roots = paths.checkouts(fx.main)
        b = sources.budget(roots)
        self.assertEqual((b["billed"], b["incomplete"]), (99.0, False))
        self.assertEqual(statusline.budget(roots)["billed"], 99.0)
        sources.Tail.MAX_RECORD, saved_record = 1024, sources.Tail.MAX_RECORD
        try:
            self.assertTrue(sources.budget(roots)["incomplete"])  # an unreadable ledger record is never ignored
        finally:
            sources.Tail.MAX_RECORD = saved_record

    def progress_file(self, doc) -> Path:
        prog = self.fx.main / "docs" / "progress"
        prog.mkdir(parents=True, exist_ok=True)
        (prog / "status.json").write_text(json.dumps(doc), encoding="utf-8")
        return prog / "status.json"

    def home_snapshot(self, now: float, for_you: int = 2, gallery_new: int = 3) -> None:
        fx = self.fx
        fx.data.mkdir(parents=True, exist_ok=True)
        (fx.data / "home.json").write_text(json.dumps(home.snapshot({"for_you": for_you, "gallery_new": gallery_new}, now)),
                                           encoding="utf-8")

    def test_status_line_is_one_bounded_line_whatever_the_progress_file_holds(self):
        fx = self.fx
        doc = sample_status()
        doc["steps"][0]["id"] = doc["current"] = "evil\nsecond line\r\x1b[31m" + "t" * 5000
        self.progress_file(doc)
        out = statusline.line(fx.main)
        self.assertNotIn("\n", out)
        self.assertNotIn("\x1b", out)
        self.assertLessEqual(len(out), statusline.LINE_MAX)
        self.assertTrue(out.startswith("Step evil second"))
        doc = sample_status()
        doc["steps"][0]["title"] = "t" * (2 * 1024 * 1024)
        self.progress_file(doc)
        self.assertTrue(statusline.line(fx.main).startswith("step unknown"))  # over the read limit: not parsed
        saved = statusline.LEDGER_MAX
        statusline.LEDGER_MAX = 10
        self.ledger(fx.main, [{"call_id": "x", "channel": "paid-api", "cost": {"status": "billed", "usd": 1}}])
        try:
            self.assertIn("API incomplete", statusline.line(fx.main))
        finally:
            statusline.LEDGER_MAX = saved

    def test_status_line_step_counts_and_paid_api(self):
        fx, now = self.fx, 1_800_000_000.0
        doc = sample_status()
        doc["current"] = "0.4.1-1"
        self.progress_file(doc)
        self.assertEqual(statusline.line(fx.main, now=now), "Step 0.4.1-1 25% \u00b7 workbench closed")
        self.home_snapshot(now - 30)
        self.assertEqual(statusline.line(fx.main, now=now), "Step 0.4.1-1 25% \u00b7 for you 2 \u00b7 gallery +3")
        self.assertIn("workbench closed", statusline.line(fx.main, now=now + statusline.HOME_FRESH + 31))  # stale snapshot
        (fx.data / "home.json").write_text('{"schema": 1, "written": "x", "for_you": -1}', encoding="utf-8")
        self.assertIn("workbench closed", statusline.line(fx.main, now=now))
        # Paid API only when some amount is not zero or unknown.
        self.ledger(fx.main, [{"call_id": "s", "channel": "subscription", "cost": {"status": "subscription", "usd": None}},
                              {"call_id": "z", "channel": "paid-api", "cost": {"status": "billed", "usd": 0}}])
        self.assertNotIn("API", statusline.line(fx.main, now=now))
        self.ledger(fx.main, [{"call_id": "u", "channel": "paid-api", "cost": {"status": "unknown", "usd": None}}])
        self.assertTrue(statusline.line(fx.main, now=now).endswith("API $0.00/$100 (1 unknown)"))
        # A schema-1 file has no checklist.
        doc["schema"] = 1
        for s in doc["steps"]:
            s.pop("items")
        self.progress_file(doc)
        self.assertTrue(statusline.line(fx.main, now=now).startswith("Step 0.4.1-1 no checklist"))

    def test_status_line_mirrors_the_checklist_rules(self):
        refs = ["0123abc", "0123ab", "0123ABC", "a" * 41, "https://github.com/KonomiYuzu01/Magic-600-Cell/pull/20",
                "https://github.com/KonomiYuzu01/Magic-600-Cell/pull/0", "https://github.com/other/repo/pull/1",
                "tests/test_core.py", "./tests/x.txt", "tests/../x", "work/reviews/r.json", "/abs/x", "C:/x",
                "docs\\x.md", "", None, 7, "x" * 201, "Work/x.txt", "WORK/x", "0123abc\n", "tests/x.txt\n"]
        for ref in refs:
            with self.subTest(ref=ref):
                self.assertEqual(statusline.evidence_ok(ref), checklist.evidence_kind(ref) is not None)
        doc = sample_status()
        steps = [*doc["steps"], {"id": "x", "items": []}, {"id": "y"}, {"id": "z", "items": [{"weight": True}]},
                 {"id": "w", "items": [{"id": "a", "weight": 2, "done": True, "evidence": "work/x"}]},
                 {"id": "m", "items": [{"id": "a", "weight": 1000, "done": True, "evidence": "0123abc"},
                                       {"id": "b", "weight": 1, "done": False}]},
                 {"id": "o", "items": [{"id": "a", "weight": 1001, "done": True, "evidence": "0123abc"}]},
                 {"id": "h", "items": [{"id": "a", "weight": 10 ** 400, "done": True, "evidence": "0123abc"}]}]
        for s in steps:
            with self.subTest(step=s["id"]):
                self.assertEqual(statusline.step_percent(s), checklist.step_percent(s))
        self.assertEqual(statusline.WEIGHT_MAX, checklist.WEIGHT_MAX)
        self.assertIsNone(checklist.step_percent(steps[-1]))
        self.assertIsNotNone(checklist.step_percent(steps[-3]))

    def test_status_line_on_the_public_file_matches_the_checklist(self):
        doc = json.loads((ROOT / "docs" / "progress" / "status.json").read_text(encoding="utf-8"))
        cur = next(s for s in doc["steps"] if s["id"] == doc["current"])
        self.progress_file(doc)
        pct = checklist.step_percent(cur)
        self.assertIsNotNone(pct)
        self.assertTrue(statusline.line(self.fx.main).startswith(f"Step {doc['current']} {math.floor(pct)}%"))

    def test_oversized_documents_are_not_read(self):
        fx = self.fx
        d = self.review(fx.main, "20260930T120000Z-ffffffff", {"valid": True, "verdict": "pass"})
        (d / "meta.json").write_text(json.dumps({"valid": True, "pad": "m" * (3 * 1024 * 1024)}), encoding="utf-8")
        self.assertIs(sources.read_json(d / "meta.json"), sources.OVERSIZED)
        (call,) = sources.codex_calls(paths.checkouts(fx.main), now=at(60))
        self.assertEqual(call["status"], "unknown")
        self.assertEqual(sources.read_json(d / "missing.json"), None)

    def test_status_line_output(self):
        fx = self.fx
        (fx.main / "docs" / "progress").mkdir(parents=True)
        shutil.copy2(ROOT / "docs" / "progress" / "status.json", fx.main / "docs" / "progress" / "status.json")
        env = {**os.environ, "CODEX_HOME": str(fx.main / "no-codex")}
        r = subprocess.run([sys.executable, str(WB / "statusline.py")], input=json.dumps({"workspace": {"current_dir": str(fx.wt)}}),
                           capture_output=True, encoding="utf-8", env=env, timeout=30)
        self.assertEqual(r.returncode, 0)
        self.assertRegex(r.stdout.strip(), r"^Step 0\.4\.1-\d \d{1,3}% \u00b7 workbench closed$")
        # UTF-8 whatever the console encoding.
        r = subprocess.run([sys.executable, str(WB / "statusline.py")], input=json.dumps({"workspace": {"current_dir": str(fx.wt)}}).encode(),
                           capture_output=True, env={**env, "PYTHONIOENCODING": "ascii"}, timeout=30)
        self.assertEqual(r.returncode, 0)
        self.assertIn("\u00b7", r.stdout.decode("utf-8"))
        r = subprocess.run([sys.executable, str(WB / "statusline.py")], input="{bad", capture_output=True, text=True, timeout=30)
        self.assertEqual(r.returncode, 0)
        self.assertTrue(r.stdout.strip())


class SummaryAndNotesTests(unittest.TestCase):
    def setUp(self):
        self.fx = Fixture()
        self.addCleanup(self.fx.cleanup)

    def brief(self, *args, sid=SID):
        env = {**os.environ, "CLAUDE_CODE_SESSION_ID": sid}
        return subprocess.run([sys.executable, str(WB / "brief.py"), *args], cwd=str(self.fx.wt), env=env,
                              capture_output=True, text=True, timeout=30)

    def test_brief_cli_writes_merges_and_refuses(self):
        self.assertEqual(self.brief("--goal", "g1", "--step", "s1").returncode, 0)
        self.assertEqual(self.brief("--step", "s2", "--waiting-for", "owner").returncode, 0)
        b = sources.brief(self.fx.data, SID)
        self.assertEqual((b["goal"], b["step"], b["waiting_for"]), ("g1", "s2", "owner"))
        self.assertEqual(self.brief("--goal", "x", sid="../evil").returncode, 2)
        self.assertEqual(self.brief("--goal", "é" * 251).returncode, 2)
        self.assertEqual(self.brief().returncode, 2)
        self.assertFalse(any(p.suffix == ".tmp" for p in (self.fx.data / "briefs").iterdir()))

    def test_summary_falls_back_to_the_latest_task_list(self):
        fx = self.fx
        fx.transcript(fx.main, SID, [user(0, cwd=str(fx.main)), assistant(1, uses=[("t", "TodoWrite", {"todos": [
            {"content": "read", "status": "completed"}, {"content": "write tests", "status": "in_progress"}]})])])
        (s,) = sources.discover_sessions(fx.main, fx.projects, now=at(60))
        summ = sources.summary(fx.data, s)
        self.assertEqual((summ["source"], summ["step"]), ("task list", "write tests"))
        tasks = sources.task_list([assistant(1, uses=[("a", "TaskCreate", {"subject": "one"}), ("b", "TaskCreate", {"subject": "two"}),
                                                      ("c", "TaskUpdate", {"taskId": "2", "status": "in_progress"})])])
        self.assertEqual([t["status"] for t in tasks], ["pending", "in_progress"])

    def test_note_limits_are_in_bytes_and_the_inbox_is_capped(self):
        fx = self.fx
        with self.assertRaises(notes.NoteRejected):
            notes.append_note(fx.data, SID, "\U0001F600" * 1001)  # 4,004 bytes, 1,001 characters
        notes.append_note(fx.data, SID, "\U0001F600" * 1000)
        with self.assertRaises(notes.NoteRejected):
            notes.append_note(fx.data, "../x", "hi")
        with self.assertRaises(notes.NoteRejected):
            notes.append_note(fx.data, SID, "   ")
        inbox = fx.data / "inbox" / f"{SID}.jsonl"
        with inbox.open("a", encoding="utf-8") as f:
            f.write("x" * (notes.INBOX_MAX_BYTES - inbox.stat().st_size - 10) + "\n")
        with self.assertRaises(notes.NoteRejected):
            notes.append_note(fx.data, SID, "one more")

    def test_note_states_follow_the_transcript(self):
        fx = self.fx
        nid = notes.append_note(fx.data, SID, "please check the cap")
        self.assertEqual(sources.notes(fx.data, SID)[0]["state"], "pending")
        emitted = datetime.now(timezone.utc)
        (fx.data / "inbox" / f"{SID}.state.jsonl").write_text(
            json.dumps({"id": nid, "emitted_at": emitted.isoformat()}) + "\n", encoding="utf-8")
        self.assertEqual(sources.notes(fx.data, SID)[0]["state"], "emitted")
        state = sources.SessionState(Path("."), Path("."), (None, "", 0))
        later = (emitted + timedelta(seconds=1)).isoformat()
        state._extract([{"type": "attachment", "timestamp": later, "attachment": {"content": f"Owner note {nid} ..."}}])
        self.assertEqual(sources.notes(fx.data, SID, state)[0]["state"], "received")
        # A tool input or thinking that names the id is not an answer.
        echo = assistant(0, uses=[("e", "Bash", {"command": f"echo {nid}"})])
        echo["timestamp"] = (emitted + timedelta(seconds=2)).isoformat()
        echo["message"]["content"].append({"type": "thinking", "thinking": f"note {nid}"})
        state._extract([echo])
        self.assertEqual(sources.notes(fx.data, SID, state)[0]["state"], "received")
        reply = assistant(0, f"Answering owner note {nid}: done.")
        reply["timestamp"] = (emitted + timedelta(seconds=3)).isoformat()
        state._extract([reply])
        self.assertEqual(sources.notes(fx.data, SID, state)[0]["state"], "answered")


class HookTests(unittest.TestCase):
    def setUp(self):
        self.fx = Fixture()
        self.addCleanup(self.fx.cleanup)
        self.events = self.fx.data / "events" / f"{SID}.jsonl"

    def lines(self, path=None):
        path = path or self.events
        return [json.loads(l) for l in path.read_text(encoding="utf-8").splitlines()] if path.exists() else []

    def test_one_bounded_event_per_call_and_exit_zero(self):
        fx = self.fx
        big_output = "z" * 100_000
        r = fx.run_hook(hook_payload(fx, "PostToolUse", tool_name="Bash", tool_input={"command": "python tests/test_core.py"},
                                     tool_response={"stdout": big_output}, tool_use_id="toolu_1", duration_ms=12))
        self.assertEqual((r.returncode, r.stdout), (0, b""))
        r = fx.run_hook(hook_payload(fx, "PermissionRequest", tool_name="Bash", tool_input={"command": "rm x"}))
        self.assertEqual((r.returncode, r.stdout), (0, b""))  # never a decision
        for name, extra in (("Notification", {"notification_type": "idle_prompt", "message": "m"}), ("Stop", {}),
                            ("StopFailure", {"error": {"type": "overloaded"}}), ("SessionEnd", {"reason": "exit"}),
                            ("SubagentStart", {"agent_id": "a1", "agent_type": "Explore"})):
            self.assertEqual(fx.run_hook(hook_payload(fx, name, **extra)).returncode, 0)
        recs = self.lines()
        self.assertEqual([r["e"] for r in recs], ["PostToolUse", "PermissionRequest", "Notification", "Stop", "StopFailure",
                                                  "SessionEnd", "SubagentStart"])
        self.assertEqual(recs[0]["summary"], "python tests/test_core.py")
        self.assertEqual(recs[0]["key"], sources.digest({"command": "python tests/test_core.py"}))
        self.assertNotIn(big_output[:100], self.events.read_text(encoding="utf-8"))
        self.assertTrue(all(len(json.dumps(r)) < hook.LINE_MAX for r in recs))

    def test_bad_input_never_fails(self):
        fx = self.fx
        for raw in (b"", b"{bad", b"[1,2]", b"\xff\xfe", json.dumps({"hook_event_name": "Stop"}).encode(),
                    json.dumps(hook_payload(fx, "Stop", sid="../../evil")).encode(),
                    json.dumps(hook_payload(fx, "UnknownEvent")).encode()):
            with self.subTest(raw=raw[:30]):
                r = fx.run_hook(None, raw=raw)
                self.assertEqual((r.returncode, r.stdout), (0, b""))
        self.assertFalse((fx.data / "events").exists())

    def test_oversized_input_keeps_the_leading_fields(self):
        fx = self.fx
        payload = hook_payload(fx, "PostToolUse", tool_name="Read", tool_use_id="toolu_9", tool_response="q" * (5 * 1024 * 1024))
        r = fx.run_hook(payload)
        self.assertEqual(r.returncode, 0)
        (rec,) = self.lines()
        self.assertEqual((rec["e"], rec["tool"], rec["truncated_input"]), ("PostToolUse", "Read", True))

    def test_unwritable_data_root_and_missing_repository(self):
        fx = self.fx
        fx.data.parent.mkdir(parents=True)
        fx.data.write_text("a file where the directory should be", encoding="utf-8")
        self.assertEqual(fx.run_hook(hook_payload(fx, "Stop")).returncode, 0)
        outside = Path(fx.tmp.name) / "no-repo"
        outside.mkdir()
        payload = hook_payload(fx, "Stop")
        payload["cwd"] = str(outside)
        self.assertEqual(fx.run_hook(payload, cwd=outside).returncode, 0)

    def test_writes_are_confined_to_the_hooks_own_repository(self):
        fx = self.fx
        payload = hook_payload(fx, "PostToolUse", tool_name="Bash", tool_input={})
        payload["cwd"] = str(fx.outsider)  # another repository
        r = fx.run_hook(payload, cwd=fx.outsider, env_extra={"CLAUDE_PROJECT_DIR": str(fx.outsider)})
        self.assertEqual((r.returncode, r.stdout), (0, b""))
        self.assertFalse((fx.outsider / "work").exists())
        self.assertFalse(fx.data.exists())
        payload["cwd"] = str(fx.ext)  # a registered worktree of the same repository
        self.assertEqual(fx.run_hook(payload).returncode, 0)
        self.assertEqual(len(self.lines()), 1)

    def test_truncated_input_never_delivers_notes(self):
        fx = self.fx
        notes.append_note(fx.data, SID, "for the main thread only")
        payload = {"session_id": SID, "hook_event_name": "PostToolUse", "cwd": str(fx.main), "tool_name": "Bash",
                   "tool_response": "q" * (5 * 1024 * 1024), "agent_id": "child-1"}  # agent_id beyond the kept prefix
        r = fx.run_hook(payload)
        self.assertEqual((r.returncode, r.stdout), (0, b""))
        self.assertEqual(sources.notes(fx.data, SID)[0]["state"], "pending")
        (rec,) = self.lines()
        self.assertTrue(rec["truncated_input"])

    def test_partial_journal_tails_never_swallow_or_repeat_notes(self):
        fx = self.fx
        inbox_dir = fx.data / "inbox"
        inbox_dir.mkdir(parents=True)
        (inbox_dir / f"{SID}.jsonl").write_bytes(b'{"id": "20260930T120000Z-0000000')  # interrupted note write
        first = notes.append_note(fx.data, SID, "after an interrupted write")
        self.assertEqual([n["id"] for n in sources.notes(fx.data, SID)], [first])
        (inbox_dir / f"{SID}.state.jsonl").write_bytes(b'{"id":')  # interrupted delivery mark
        self.assertIn(first, hook.deliver(fx.data, SID))
        self.assertIsNone(hook.deliver(fx.data, SID))  # marked despite the fragment before it
        self.events.parent.mkdir(parents=True)
        self.events.write_bytes(b'{"e": "Post')
        hook.append_event(self.events.parent, SID, b'{"e": "Stop"}\n')
        self.assertEqual(self.events.read_bytes().split(b"\n")[1], b'{"e": "Stop"}')

    def test_concurrent_note_writers_respect_the_cap_and_every_accepted_note_arrives(self):
        fx = self.fx
        inbox = fx.data / "inbox" / f"{SID}.jsonl"
        notes.append_note(fx.data, SID, "seed")
        with inbox.open("ab") as f:  # nearly full: room for only a few more notes
            f.write(b'{"id": "filler", "text": "' + b"f" * (notes.INBOX_MAX_BYTES - inbox.stat().st_size - 12_000) + b'"}\n')
        outs = self.run_workers(f"""\
            sys.path.insert(0, {str(WB)!r})
            import notes
            accepted = []
            for i in range(5):
                try:
                    accepted.append(notes.append_note(data, "{SID}", "n" * 3000))
                except notes.NoteRejected:
                    pass
            print(" ".join(accepted))
            """)
        accepted = [nid for o in outs for nid in o.split()]
        self.assertLessEqual(inbox.stat().st_size, notes.INBOX_MAX_BYTES)
        self.assertGreater(len(accepted), 0)
        delivered = []
        while (text := hook.deliver(fx.data, SID)) is not None:
            delivered += re.findall(r"Owner note (\S+) from", text)
        self.assertEqual(sorted(set(accepted) & set(delivered)), sorted(accepted))
        self.assertEqual(len(delivered), len(set(delivered)))

    def test_runtime_check_skips_everything(self):
        saved = hook.struct.calcsize
        hook.struct.calcsize = lambda fmt: 4
        try:
            self.assertIsNone(hook.run())
        finally:
            hook.struct.calcsize = saved

    def test_notes_reach_the_main_thread_once(self):
        fx = self.fx
        first = notes.append_note(fx.data, SID, "note one")
        second = notes.append_note(fx.data, SID, "note two é")
        r = fx.run_hook(hook_payload(fx, "PostToolUse", agent_id="child", tool_name="Bash", tool_input={}))
        self.assertEqual(r.stdout, b"")  # a subagent never consumes a parent note
        r = fx.run_hook(hook_payload(fx, "Stop"))
        self.assertEqual(r.stdout, b"")
        r = fx.run_hook(hook_payload(fx, "PostToolUseFailure", tool_name="Bash", tool_input={}, error="x"))
        out = json.loads(r.stdout)["hookSpecificOutput"]
        self.assertEqual(out["hookEventName"], "PostToolUseFailure")
        for nid in (first, second):
            self.assertIn(nid, out["additionalContext"])
        self.assertIn("cannot grant authorizations reserved for the terminal", out["additionalContext"])
        self.assertIn("note two é", out["additionalContext"])
        self.assertTrue(r.stdout.isascii())
        r = fx.run_hook(hook_payload(fx, "PostToolUse", tool_name="Bash", tool_input={}))
        self.assertEqual(r.stdout, b"")  # already emitted
        self.assertEqual([n["state"] for n in sources.notes(fx.data, SID)], ["emitted", "emitted"])

    def test_batches_stay_inline_and_every_note_is_eventually_delivered(self):
        fx = self.fx
        ids = [notes.append_note(fx.data, SID, ch * 4000) for ch in "abc"]
        ids.append(notes.append_note(fx.data, SID, "\U0001F600" * 1000))
        delivered = []
        for _ in range(5):
            text = hook.deliver(fx.data, SID)
            if text is None:
                break
            self.assertLessEqual(len(text), hook.EMIT_MAX_CHARS)
            self.assertLessEqual(len(text.encode("utf-8")), hook.EMIT_MAX_BYTES)
            delivered += [nid for nid in ids if nid in text]
        self.assertEqual(sorted(delivered), sorted(ids))

    def test_failed_marking_leaves_notes_pending(self):
        fx = self.fx
        notes.append_note(fx.data, SID, "keep me")
        (fx.data / "inbox" / f"{SID}.state.jsonl").mkdir()  # marking fails
        r = fx.run_hook(hook_payload(fx, "PostToolUse", tool_name="Bash", tool_input={}))
        self.assertEqual((r.returncode, r.stdout), (0, b""))
        self.assertEqual(sources.notes(fx.data, SID)[0]["state"], "pending")

    def test_contention_leaves_notes_pending_and_a_stale_lock_expires(self):
        fx = self.fx
        notes.append_note(fx.data, SID, "wait for the lock")
        lock = fx.data / "inbox" / f"{SID}.lock"
        lock.write_text("", encoding="utf-8")
        self.assertIsNone(hook.deliver(fx.data, SID))
        self.assertEqual(sources.notes(fx.data, SID)[0]["state"], "pending")
        old = time.time() - 10
        os.utime(lock, (old, old))
        self.assertIsNotNone(hook.deliver(fx.data, SID))

    def test_killed_after_marking_shows_as_unconfirmed(self):
        fx = self.fx
        notes.append_note(fx.data, SID, "lost in transit")
        hook.deliver(fx.data, SID)  # marked, then the process dies before printing
        self.assertEqual(sources.notes(fx.data, SID)[0]["state"], "emitted")
        self.assertIsNone(hook.deliver(fx.data, SID))  # never emitted twice

    # Worker processes load the fixture's copy of the hook, wait for a common start time, then run BODY.
    WORKER_HEAD = textwrap.dedent("""\
        import importlib.util, json, re, sys, time
        from pathlib import Path
        spec = importlib.util.spec_from_file_location("h", sys.argv[1])
        h = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(h)
        data, me, start, cap = Path(sys.argv[2]), sys.argv[3], float(sys.argv[4]), int(sys.argv[5])
        if cap:
            h.EVENT_CAP = cap
        while time.time() < start:
            time.sleep(0.001)
        """)

    def run_workers(self, body: str, n: int = 8, cap: int = 0) -> list[str]:
        script = Path(self.fx.tmp.name) / "worker.py"
        script.write_text(self.WORKER_HEAD + textwrap.dedent(body), encoding="utf-8")
        start = time.time() + 1.5
        procs = [subprocess.Popen([sys.executable, str(script), str(self.fx.hook), str(self.fx.data), str(i), str(start), str(cap)],
                                  stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True) for i in range(n)]
        results = [p.communicate(timeout=120) for p in procs]
        for p, (_, err) in zip(procs, results):
            self.assertEqual(p.returncode, 0, err)
        return [out for out, _ in results]

    def test_concurrent_appends_keep_every_event(self):
        self.run_workers(f"""\
            for i in range(200):
                line = json.dumps({{"e": "x", "id": f"{{me}}-{{i}}"}}) + "\\n"
                h.append_event(data / "events", "{SID}", line.encode())
            """)
        ids = [r["id"] for r in self.lines()]
        self.assertEqual(len(ids), 1600)
        self.assertEqual(len(set(ids)), 1600)

    def test_concurrent_appends_at_the_cap_stay_bounded(self):
        cap = 64 * 1024
        self.run_workers(f"""\
            for i in range(200):
                line = json.dumps({{"e": "x", "id": f"{{me}}-{{i}}", "pad": "p" * 150}}) + "\\n"
                h.append_event(data / "events", "{SID}", line.encode())
            """, cap=cap)
        recs = self.lines()
        self.assertLessEqual(self.events.stat().st_size, cap + 200)
        self.assertEqual(sum(1 for r in recs if r["e"] == "cap_reached"), 1)
        self.assertEqual(recs[-1]["e"], "cap_reached")
        notes.append_note(self.fx.data, SID, "still delivered after the cap")
        r = self.fx.run_hook(hook_payload(self.fx, "PostToolUse", tool_name="Bash", tool_input={}))
        self.assertIn(b"still delivered", r.stdout)

    def test_two_simultaneous_deliverers_never_emit_a_note_twice(self):
        for i in range(12):
            notes.append_note(self.fx.data, SID, f"n{i} " + "w" * 3000)
        outs = self.run_workers(f"""\
            out = []
            for _ in range(20):
                t = h.deliver(data, "{SID}")
                if t:
                    out.append(t)
            print("\\n".join(re.findall(r"Owner note (\\S+) from", "\\n".join(out))))
            """, n=2)
        seen = [line for o in outs for line in o.split()]
        self.assertEqual(len(seen), len(set(seen)))
        self.assertEqual(len(seen), 12)

    def test_hook_and_status_line_import_nothing_dangerous(self):
        for path in (HOOK, WB / "statusline.py"):
            tree = ast.parse(path.read_text(encoding="utf-8"))
            names = {a.name.split(".")[0] for n in ast.walk(tree) if isinstance(n, ast.Import) for a in n.names}
            names |= {n.module.split(".")[0] for n in ast.walk(tree) if isinstance(n, ast.ImportFrom) and n.module}
            with self.subTest(path=path.name):
                self.assertFalse(names & {"subprocess", "socket", "urllib", "http", "ctypes", "importlib", "asyncio",
                                          "paths", "sources", "multiprocessing", "webbrowser"})

    def registrations(self):
        settings = json.loads((ROOT / ".claude" / "settings.json").read_text(encoding="utf-8"))
        return settings, [(event, group, h) for event, groups in settings["hooks"].items() for group in groups
                          for h in group["hooks"] if (h.get("args") or [None])[-1] == HOOK_ARG]

    def test_settings_register_the_hook_and_status_line(self):
        settings, found = self.registrations()
        registered = set()
        for event, group, h in found:
            self.assertEqual((h["type"], h["command"], h["args"], h["timeout"]),
                             ("command", "python", ["-I", "-c", HOOK_GUARD, HOOK_ARG], 5))
            self.assertNotIn("matcher", group)  # every tool, every notification type
            registered.add(event)
        self.assertEqual(registered, hook.EVENTS)
        # A status line runs as a shell string: anchor it to the project, never to the session's cwd.
        self.assertEqual(settings["statusLine"], {
            "type": "command", "command": 'python "${CLAUDE_PROJECT_DIR}/tools/workbench/statusline.py"',
            "refreshInterval": 30})


    def run_registered(self, project: Path, payload: dict):
        _, found = self.registrations()
        args = [a.replace("${CLAUDE_PROJECT_DIR}", str(project).replace("\\", "/")) for a in found[0][2]["args"]]
        env = {k: v for k, v in os.environ.items() if k != "CLAUDE_PROJECT_DIR"}
        return subprocess.run([sys.executable, *args], input=json.dumps(payload).encode("utf-8"),
                              capture_output=True, cwd=str(project), env=env, timeout=30)

    def test_registered_command_is_silent_without_the_script(self):
        # A session can name a main checkout at a commit older than the hook: it must still exit 0 with no output.
        with tempfile.TemporaryDirectory() as tmp:
            old = Path(tmp) / "old main"
            (old / ".git").mkdir(parents=True)
            (old / ".claude" / "hooks").mkdir(parents=True)
            payload = {"session_id": SID, "hook_event_name": "Stop", "cwd": str(old), "transcript_path": "unused"}
            r = self.run_registered(old, payload)
            self.assertEqual((r.returncode, r.stdout), (0, b""))
            self.assertFalse((old / "work").exists())

    def test_registered_command_runs_the_script(self):
        fx = Fixture()
        self.addCleanup(fx.cleanup)
        fx.transcript(fx.main, SID, [user(0, cwd=str(fx.main))])
        notes.append_note(fx.data, SID, "guarded note")
        r = self.run_registered(fx.main, hook_payload(fx, "PostToolUse", tool_name="Bash", tool_input={"command": "ls"}))
        self.assertEqual(r.returncode, 0)
        self.assertIn(b"guarded note", r.stdout)  # delivered through the guard like a direct run
        lines = (fx.data / "events" / f"{SID}.jsonl").read_text(encoding="utf-8").splitlines()
        self.assertEqual(json.loads(lines[-1])["e"], "PostToolUse")


class LaunchTests(unittest.TestCase):
    def setUp(self):
        self.fx = Fixture()
        self.addCleanup(self.fx.cleanup)

    def file(self, rel: str, root: Path | None = None) -> Path:
        p = (root or self.fx.main) / rel
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text("x", encoding="utf-8")
        return p

    def test_viewer_policy(self):
        fx = self.fx
        plan = lambda p: launch.plan_open(p, fx.main, fx.projects)
        self.assertEqual(plan(self.file("work/reviews/x/packet.md"))[0], "notepad")
        self.assertEqual(plan(self.file("work/loop-memory/ledgers/codex.jsonl", fx.wt))[0], "notepad")
        self.assertEqual(plan(self.file("t.jsonl", fx.projects / "p"))[0], "notepad")
        self.assertEqual(plan(self.file("docs/wiki/index.md"))[0], "default")
        self.assertEqual(plan(self.file("out/deck.html", fx.ext))[0], "browser")
        self.assertEqual(plan(self.file("out/d.svg"))[0], "browser")
        # Gallery outputs: inert media in private locations open in their default application, PDFs in the browser,
        # diagram and notebook sources as text; active content (HTML, SVG) never leaves the workbench.
        for rel in ("work/gallery/a.png", "work/loop-memory/ui-vision/b.JPG", "work/x/c.webp", "work/x/d.mp4", "work/x/e.webm"):
            with self.subTest(rel=rel):
                self.assertEqual(plan(self.file(rel))[0], "default")
        self.assertEqual(plan(self.file("work/x/f.pdf", fx.wt))[0], "browser")
        for rel in ("work/x/g.mmd", "work/x/h.drawio", "work/x/i.typ", "work/x/nb.py"):
            with self.subTest(rel=rel):
                self.assertEqual(plan(self.file(rel))[0], "notepad")
        self.assertEqual(plan(self.file("docs/figures/j.jpg"))[0], "default")
        self.assertEqual(plan(self.file("docs/figures/k.webm"))[0], "default")
        for bad in (self.file("work/reviews/x/report.html"), self.file("work/gallery/v.svg"), self.file("work/x/y.htm"),
                    self.file("work/x/z.exe"), self.file("tools/x.py"), self.file("run.bat"),
                    self.file("a.lnk"), self.file("x.ps1"), self.file("thing.exe"), self.file("s.js"),
                    self.file("elsewhere.md", Path(fx.tmp.name)), fx.main / "missing.md"):
            with self.subTest(bad=bad.name):
                with self.assertRaises(launch.LaunchRefused):
                    plan(bad)

    def test_a_link_is_judged_by_its_target(self):
        fx = self.fx
        plan = lambda p: launch.plan_open(p, fx.main, fx.projects)
        targets = {"view.png": self.file("work/x/payload.exe"), "doc.pdf": self.file("work/x/page.html"),
                   "away.png": self.file("away.png", Path(fx.tmp.name)), "fine.png": self.file("work/gallery/real.png")}
        real = Path.resolve

        def resolve(self, strict=False):
            return real(targets[self.name], strict) if self.name in targets else real(self, strict)

        links = {name: self.file(f"work/gallery/{name}") for name in targets}
        with patch.object(Path, "resolve", resolve):
            for name in ("view.png", "doc.pdf", "away.png"):
                with self.subTest(link=name), self.assertRaises(launch.LaunchRefused):
                    plan(links[name])
            self.assertEqual(plan(links["fine.png"]), ("default", str(real(targets["fine.png"]))))

    def test_console_commands_never_pass_through_a_shell(self):
        tricky = str(Path(self.fx.tmp.name) / "repo&echo INJECTED %PATH%")
        argv = launch.console_argv(["python", "-c", "import sys; print(repr(sys.argv[1:]))", tricky])
        self.assertNotIn("cmd.exe", " ".join(argv).lower())
        r = subprocess.run(argv, input="\n", capture_output=True, text=True, timeout=60)
        printed = [line for line in r.stdout.splitlines() if line.strip()]
        self.assertEqual(printed[0], repr([tricky]))
        self.assertFalse(any(line.strip() == "INJECTED" for line in printed))
        self.assertIn("[exit 0]", r.stdout)

    def test_commands_and_resume_rule(self):
        fx = self.fx
        argv, _ = launch.plan_command("git-diff", fx.main, checkout=fx.wt)
        self.assertEqual(argv[:3], ["git", "-C", str(fx.wt)])
        with self.assertRaises(launch.LaunchRefused):
            launch.plan_command("git-diff", fx.main, checkout=Path(fx.tmp.name))
        nb = self.file("research/nb.py")
        self.assertEqual(launch.plan_command("marimo", fx.main, notebook=nb)[0][1:], ["edit", str(nb)])
        with self.assertRaises(launch.LaunchRefused):
            launch.plan_command("marimo", fx.main, notebook=self.file("work/x.py"))
        for status in ("running", "waiting", "failed", "unknown", None):
            with self.subTest(status=status):
                with self.assertRaises(launch.LaunchRefused):
                    launch.plan_command("resume", fx.main, sid=SID, session_status=status)
        argv, cwd = launch.plan_command("resume", fx.main, sid=SID, session_status="finished", cwd=str(fx.wt), remote=True)
        self.assertEqual((argv, cwd), (["claude", "--resume", SID, "--remote-control"], str(fx.wt)))
        with self.assertRaises(launch.LaunchRefused):
            launch.plan_command("resume", fx.main, sid="../x", session_status="finished")


class StageCContractTests(unittest.TestCase):
    """The runner, the watch rules and the launch plans were written in parallel; their shared numbers agree."""

    def test_watch_thresholds_follow_the_runner(self):
        for kind in runs.CODEX_KINDS:
            self.assertEqual(watch.CODEX_EXPECTED[kind], runs.codex_deadline(kind))
        self.assertEqual(watch.WRAPPER_CAP_S, runs.codex_deadline("implement") + 600)
        self.assertGreaterEqual(sources.CODEX_STALE_AFTER, runs.codex_deadline("implement"))

    def test_packet_names_follow_one_rule(self):
        pattern = lambda p: p if isinstance(p, str) else p.pattern  # noqa: E731
        self.assertEqual(pattern(launch.PACKET_RE), pattern(runs.PACKET_RE))


class ProgressStatusTests(unittest.TestCase):
    def test_public_status_file_is_valid_and_sanitized(self):
        import codex_review
        schema = json.loads((ROOT / "schemas" / "progress-status.schema.json").read_text(encoding="utf-8"))
        text = (ROOT / "docs" / "progress" / "status.json").read_text(encoding="utf-8")
        doc = json.loads(text)
        self.assertEqual(codex_review.validate_result(doc, schema), [])
        self.assertEqual(doc["schema"], 2)
        self.assertEqual(checklist.validate(doc), [])
        self.assertEqual(text.replace("\r\n", "\n"), checklist.dump(doc))  # the canonical layout that progress.py writes
        ids = [s["id"] for s in doc["steps"]]
        self.assertEqual(ids, [f"0.4.1-{i}" for i in range(1, 8)] + [f"2.{i}" for i in range(6)])
        self.assertIn(doc["current"], ids)
        self.assertRegex(doc["updated"], r"^\d{4}-\d{2}-\d{2}$")
        for s in doc["steps"]:
            self.assertEqual(s["track"], "0.4.1" if s["id"].startswith("0.4.1") else "stage-2")
            if s["acceptance"] is not None:
                self.assertTrue((ROOT / s["acceptance_source"]).is_file(), s["id"])
            else:
                self.assertIsNone(s["acceptance_source"])
        self.assertIsNone(re.search(r"[\u3040-\u30ff\u3400-\u9fff\uac00-\ud7af]", text))
        self.assertIsNone(re.search(r"(?<![A-Za-z])[A-Za-z]:[\\/]|/Users/|/home/|\\\\Users", text))  # a URL scheme is no drive

    def test_every_done_item_has_existing_evidence(self):
        doc = json.loads((ROOT / "docs" / "progress" / "status.json").read_text(encoding="utf-8"))
        for s in doc["steps"]:
            for it in s.get("items", []):
                if not it["done"]:
                    continue
                with self.subTest(item=f"{s['id']}/{it['id']}"):
                    kind = checklist.evidence_kind(it["evidence"])
                    if kind == "path":
                        self.assertTrue((ROOT / it["evidence"]).is_file())
                    elif kind == "sha":
                        r = subprocess.run(["git", "-C", str(ROOT), "rev-parse", "--verify", "--quiet", it["evidence"] + "^{commit}"],
                                           capture_output=True, timeout=30)
                        self.assertEqual(r.returncode, 0)
                    else:
                        self.assertEqual(kind, "pr")


def load_tests(loader, tests, pattern):
    """The documented workbench check (`python tests/test_workbench.py`) also runs the runner, watch, home and CLI suites."""
    sys.path.insert(0, str(ROOT / "tests"))
    for name in ("test_workbench_runs", "test_workbench_watch", "test_workbench_home", "test_workbench_cli"):
        tests.addTests(loader.loadTestsFromModule(importlib.import_module(name)))
    return tests


if __name__ == "__main__":
    unittest.main(verbosity=2)
