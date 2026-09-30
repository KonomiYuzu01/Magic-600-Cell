"""Headless watch and session launch tests, using only disposable synthetic fixtures."""
from __future__ import annotations

import ast
import json
import os
import re
import subprocess
import unittest
from datetime import datetime, timezone
from pathlib import Path
from unittest.mock import patch

from test_workbench import Fixture, SID, WB, assistant, at, ev, iso, launch, paths, result, sources, user
import watch

WRAPPER = ("python tools/agents/codex_review.py --kind review --packet work/reviews/packets/p.md "
           "--model gpt-6.1-sol --effort max --speed fast")
CALL = "20260930T120000Z-aaaaaaaa"


def run_record(rid="run-a", **fields):
    return {"rid": rid, "kind": "registry", "run": "check", "title": "Tests", "status": "failed",
            "reason": None, "created": iso(0), "started": iso(1), "ended": iso(2), "exits": [1],
            "call_id": None, **fields}


class FixtureTests(unittest.TestCase):
    def setUp(self):
        mkdir = os.mkdir

        def fixture_mkdir(path, mode=0o777, *, dir_fd=None):
            # Python 3.14's Windows 0700 ACL excludes the sandbox token; inherit fixture permissions.
            return mkdir(path, 0o777 if os.name == "nt" and mode == 0o700 else mode, dir_fd=dir_fd)

        with patch("tempfile._os.mkdir", fixture_mkdir):
            self.fx = Fixture()
        self.addCleanup(self.fx.cleanup)

    def session(self, records=(), events=(), children=None, sid=SID, checkout=None):
        root = checkout or self.fx.main
        transcript = self.fx.transcript(root, sid, [user(0, cwd=str(root)), *records])
        self.fx.events(sid, list(events))
        for agent, recs in (children or {}).items():
            directory = transcript.parent / sid / "subagents"
            directory.mkdir(parents=True, exist_ok=True)
            (directory / f"agent-{agent}.jsonl").write_text(
                "".join(json.dumps(r) + "\n" for r in recs), encoding="utf-8")
        return transcript

    def discover(self, seconds=1201, cache=None):
        cache = {} if cache is None else cache
        for _ in range(50):
            sessions = sources.discover_sessions(self.fx.main, self.fx.projects, now=at(seconds), cache=cache)
            if all(s.state is not None and s.state.caught_up() for s in sessions):
                return sessions
        self.fail("history never caught up")

    def flags(self, seconds=1201, calls=(), runs=(), cache=None):
        return watch.flags(self.discover(seconds, cache), list(calls), list(runs), at(seconds))

    def call(self, seconds=0, suffix="aaaaaaaa", meta=None, checkout=None):
        cid = datetime.fromtimestamp(at(seconds), timezone.utc).strftime("%Y%m%dT%H%M%SZ") + "-" + suffix
        directory = (checkout or self.fx.main) / "work" / "reviews" / cid
        directory.mkdir(parents=True)
        (directory / "packet.md").write_text("synthetic packet", encoding="utf-8")
        if meta is not None:
            (directory / "meta.json").write_text(json.dumps(meta), encoding="utf-8")
        return cid

    def calls(self, seconds=1201):
        return sources.codex_calls(paths.checkouts(self.fx.main), now=at(seconds))


class WaitTests(FixtureTests):
    def test_parent_permission_wait_survives_25_hours(self):
        self.session(events=[ev(1, "PermissionRequest", tool="Bash", key="k")])
        (session,) = self.discover(25 * 3600 + 1)
        self.assertEqual(session.status.wait_since, at(1))
        (flag,) = watch.flags([session], [], [], at(25 * 3600 + 1))
        self.assertEqual((flag["kind"], flag["severity"], flag["target_id"], flag["t"]),
                         ("owner_wait", 0, SID, at(1)))
        self.assertEqual(flag["text"], "waiting 1.0 d: permission (Bash)")

    def test_fresh_child_wait_under_old_parent(self):
        self.session(events=[ev(90000, "PermissionRequest", tool="Bash", key="k", agent_id="child")],
                     children={"child": [assistant(89999, "working")]})
        (session,) = self.discover(90060)
        self.assertEqual(session.status.last_activity, at(0))
        self.assertEqual(session.status.last_any, at(90000))
        (flag,) = watch.flags([session], [], [], at(90060))
        self.assertEqual((flag["kind"], flag["text"]), ("owner_wait", "waiting 1 min: permission (Bash)"))

    def test_idle_turn_end_is_not_an_owner_wait(self):
        self.session([assistant(1, "done", stop="end_turn")])
        (session,) = self.discover()
        self.assertEqual((session.status.status, session.status.waits, session.status.wait_since),
                         ("waiting", [], None))
        self.assertEqual(self.flags(), [])

    def test_only_matching_post_resolves_wait(self):
        command = {"command": "do something"}
        key = sources.digest(command)
        self.session([assistant(1, uses=[("b", "Bash", command)])],
                     [ev(2, "PermissionRequest", tool="Bash", key=key)])
        cache = {}
        self.assertEqual(self.flags(30, cache=cache)[0]["kind"], "owner_wait")
        self.fx.events(SID, [ev(3, "PostToolUse", tool="Bash", key=key, agent_id="other"),
                             ev(4, "PostToolUse", tool="Bash", key="different")])
        self.assertEqual(self.flags(30, cache=cache)[0]["kind"], "owner_wait")
        self.fx.events(SID, [ev(5, "PostToolUse", tool="Bash", key=key)])
        self.assertEqual(self.flags(30, cache=cache), [])
        self.assertIsNone(self.discover(30, cache)[0].status.wait_since)

    def test_session_end_clears_parent_and_child_waits(self):
        self.session(events=[ev(1, "PermissionRequest", tool="Bash", key="p"),
                             ev(2, "PermissionRequest", tool="Read", key="c", agent_id="child"),
                             ev(3, "SessionEnd", reason="exit")])
        (session,) = self.discover()
        self.assertEqual((session.status.status, session.status.waits, session.status.wait_since),
                         ("finished", [], None))
        self.assertEqual(self.flags(), [])

    def test_oldest_wait_and_sorted_unique_labels(self):
        self.session(events=[ev(1, "PermissionRequest", tool="Read", key="a", agent_id="child"),
                             ev(2, "PermissionRequest", tool="Bash", key="b"),
                             ev(3, "PermissionRequest", tool="Read", key="c")])
        (flag,) = self.flags(61)
        self.assertEqual((flag["t"], flag["text"]),
                         (at(1), "waiting 1 min: permission (Bash), permission (Read)"))


class FailureTests(FixtureTests):
    def test_failure_then_session_end_keeps_failure_and_resume_status(self):
        self.session([assistant(1, uses=[("b", "Bash", {"command": "x"})])],
                     [ev(2, "StopFailure"), ev(3, "SessionEnd", reason="exit")])
        (session,) = self.discover()
        self.assertEqual((session.status.status, session.status.failed_at, session.status.open_tools),
                         ("finished", at(2), []))
        (flag,) = self.flags()
        self.assertEqual((flag["kind"], flag["t"], flag["text"]),
                         ("failed_turn", at(2), "turn failed 19 min ago; session ended"))
        self.assertEqual(launch.plan_command("resume", self.fx.main, sid=SID,
                                            session_status=session.status.status)[0], ["claude", "--resume", SID])

    def test_new_main_activity_clears_failure(self):
        self.session([assistant(2, "API error", isApiErrorMessage=True), user(3, "retry")])
        (session,) = self.discover(30)
        self.assertEqual((session.status.status, session.status.failed_at), ("running", None))
        self.assertEqual(self.flags(30), [])

    def test_failure_25_hours_old_is_not_flagged(self):
        self.session(events=[ev(1, "StopFailure"), ev(2, "SessionEnd", reason="exit")])
        self.assertEqual(self.flags(25 * 3600 + 1), [])

    def test_newest_main_failure_is_retained_through_child_activity(self):
        self.session([user(3, "retry")], [ev(1, "StopFailure"), ev(4, "StopFailure")],
                     children={"child": [assistant(5, "child working")]})
        (session,) = self.discover(10)
        self.assertEqual((session.status.failed_at, session.status.last_activity, session.status.last_any),
                         (at(4), at(4), at(5)))
        self.assertEqual(self.flags(10)[0]["t"], at(4))


class StallTests(FixtureTests):
    def test_child_activity_prevents_parent_stall(self):
        self.session(children={"child": [assistant(1100, "still working")]})
        (session,) = self.discover()
        self.assertEqual((session.status.status, session.status.last_activity, session.status.last_any),
                         ("running", at(0), at(1100)))
        self.assertEqual(self.flags(), [])

    def test_foreground_wrapper_exempts_only_its_session(self):
        self.session([assistant(1, uses=[("b", "Bash", {"command": WRAPPER})])], sid="wrapper")
        self.session(sid="silent", checkout=self.fx.wt)
        sessions = self.discover()
        (flag,) = watch.flags(sessions, [], [], at(1201))
        self.assertEqual((flag["kind"], flag["target_kind"], flag["target_id"]),
                         ("stalled", "session", "silent"))
        wrapper = next(s for s in sessions if s.sid == "wrapper")
        self.assertEqual(wrapper.status.open_tools,
                         [{"agent": None, "name": "Bash", "t": at(1), "wrapper": True}])

    def test_ordinary_bash_stalls_and_names_the_tool(self):
        self.session([assistant(1, uses=[("b", "Bash", {"command": "sleep 9999"})])])
        (flag,) = self.flags()
        self.assertEqual(flag["text"], "no activity for 20 min; open: Bash")

    def test_child_wrapper_exempts_while_running_and_clears_on_every_end(self):
        for ending in ("SubagentStop", "turn_fail", "turn_end"):
            with self.subTest(ending=ending):
                sid = ending.replace("_", "-")
                children = {"child": [assistant(1, uses=[("b", "Bash", {"command": WRAPPER})])]}
                self.session(children=children, sid=sid)
                cache = {}
                session = next(s for s in self.discover(cache=cache) if s.sid == sid)
                self.assertEqual(watch.flags([session], [], [], at(1201)), [])
                if ending == "SubagentStop":
                    self.fx.events(sid, [ev(2, "SubagentStop", agent_id="child")])
                else:
                    path = session.project_dir / sid / "subagents" / "agent-child.jsonl"
                    rec = assistant(2, "error" if ending == "turn_fail" else "done",
                                    isApiErrorMessage=ending == "turn_fail",
                                    stop="end_turn" if ending == "turn_end" else None)
                    with path.open("a", encoding="utf-8") as stream:
                        stream.write(json.dumps(rec) + "\n")
                session = next(s for s in self.discover(1202, cache) if s.sid == sid)
                self.assertEqual(session.status.open_tools, [])
                (flag,) = watch.flags([session], [], [], at(1202))
                self.assertEqual(flag["kind"], "stalled")

    def test_main_failure_and_turn_end_clear_only_main_tools(self):
        for ending in ("StopFailure", "Stop"):
            with self.subTest(ending=ending):
                self.session([assistant(1, uses=[("main", "Bash", {"command": WRAPPER})])],
                             [ev(3, ending)], sid=ending,
                             children={"child": [assistant(2, uses=[("child", "Read", {})])]})
                session = next(s for s in self.discover() if s.sid == ending)
                self.assertEqual(session.status.open_tools,
                                 [{"agent": "child", "name": "Read", "t": at(2), "wrapper": False}])

    def test_mentions_shell_chains_and_background_calls_do_not_exempt(self):
        commands = [("grep -n x tools/agents/codex_review.py", False),
                    ("python tools/agents/codex_review.py --kind review --packet p.md; sleep 99", False),
                    (WRAPPER, True)]
        for i, (command, background) in enumerate(commands):
            sid = f"bad-wrapper-{i}"
            self.session([assistant(1, uses=[("b", "Bash", {"command": command,
                                                          "run_in_background": background})])], sid=sid)
        flags = self.flags()
        self.assertEqual({f["target_id"] for f in flags}, {f"bad-wrapper-{i}" for i in range(3)})
        self.assertTrue(all(f["kind"] == "stalled" and "Bash" in f["text"] for f in flags))

    def test_wrapper_cap_is_inclusive_then_reports_age(self):
        self.session([assistant(0, uses=[("b", "Bash", {"command": WRAPPER})])])
        self.assertEqual(self.flags(7800), [])
        (flag,) = self.flags(7801)
        self.assertEqual(flag["text"], "no activity for 2.2 h; open: Bash (codex wrapper, 2.2 h)")

    def test_silent_session_25_hours_old_is_not_flagged(self):
        self.session([assistant(1, uses=[("b", "Bash", {"command": "sleep"})])])
        self.assertEqual(self.flags(25 * 3600 + 1), [])

    def test_stall_threshold_is_strict(self):
        self.session()
        self.assertEqual(self.flags(900), [])
        self.assertEqual(self.flags(901)[0]["kind"], "stalled")

    def test_snapshot_tools_are_sorted_and_copied(self):
        self.session([assistant(1, uses=[("a", "Read", {})]),
                      assistant(3, uses=[("b", "Bash", {"command": WRAPPER})])],
                     children={"child": [assistant(2, uses=[("c", "Grep", {})])]})
        cache = {}
        (session,) = self.discover(10, cache)
        self.assertEqual([t["name"] for t in session.status.open_tools], ["Read", "Grep", "Bash"])
        session.status.open_tools[0]["name"] = "changed snapshot"
        self.assertEqual(self.discover(10, cache)[0].status.open_tools[0]["name"], "Read")


class WrapperTests(unittest.TestCase):
    def test_full_command_recognition(self):
        accepted = [WRAPPER, "python3 ./tools/agents/codex_review.py --kind plan", "py tools/agents/codex_review.py",
                    r"C:\repo\.venv\Scripts\python.exe tools\agents\codex_review.py --kind review"]
        rejected = ["grep -n x tools/agents/codex_review.py",
                    "python tools/agents/codex_review.py --kind review --packet p.md; sleep 99",
                    "python\ntools/agents/codex_review.py", "python -c 'x' tools/agents/codex_review.py"]
        for command in accepted + rejected:
            with self.subTest(command=command):
                self.assertEqual(sources.WRAPPER_RE.fullmatch(command) is not None, command in accepted)
                sigs, bad = sources.transcript_signals([assistant(0, uses=[("b", "Bash", {"command": command})])])
                self.assertEqual(bad, 0)
                use = next(s for s in sigs if s.kind == "tool_use")
                self.assertEqual(use.data["wrapper"], command in accepted)

    def test_background_wrong_tool_and_malformed_input(self):
        cases = [("Bash", {"command": WRAPPER, "run_in_background": True}),
                 ("Read", {"command": WRAPPER}), ("Bash", None), ("Bash", []),
                 ("Bash", {"command": 5})]
        for name, inp in cases:
            with self.subTest(name=name, inp=inp):
                sigs, _ = sources.transcript_signals([assistant(0, uses=[("b", name, inp)])])
                self.assertFalse(next(s for s in sigs if s.kind == "tool_use").data["wrapper"])
        sigs, bad = sources.transcript_signals([assistant(0, uses=[("b", "Bash", {"command": {1, 2}})])])
        self.assertEqual(bad, 1)
        self.assertFalse(any(s.kind == "tool_use" for s in sigs))


class CallTests(FixtureTests):
    def test_invalid_acceptance_failed_and_passing_calls(self):
        invalid = self.call(meta={"kind": "review", "valid": False, "problems": ["invalid model", "second"]})
        failed = self.call(1, "bbbbbbbb", {"kind": "implement", "valid": True, "acceptance": {"exit_code": 1}})
        self.call(2, "cccccccc", {"kind": "implement", "valid": True, "acceptance": {"exit_code": 0}})
        self.call(3, "dddddddd", {"kind": "review", "valid": True})
        calls = self.calls()
        acceptance = next(c for c in calls if c["call_id"] == failed)
        self.assertEqual((acceptance["status"], acceptance["acceptance_exit"]), ("finished", 1))
        flags = watch.flags([], calls, [], at(1201))
        self.assertEqual([f["target_id"] for f in flags], [failed, invalid])
        self.assertEqual([f["kind"] for f in flags], ["codex_failed", "codex_failed"])
        self.assertEqual(flags[0]["text"], f"implement {failed}: acceptance failed (exit 1)")
        self.assertEqual(flags[1]["text"], f"review {invalid}: invalid model")

    def test_old_failed_call_is_not_flagged(self):
        self.call(meta={"valid": False})
        self.assertEqual(watch.flags([], self.calls(25 * 3600), [], at(25 * 3600)), [])

    def test_resultless_call_at_three_hours_is_still_running(self):
        self.call()
        self.assertEqual(sources.CODEX_STALE_AFTER, 10800)
        (call,) = self.calls(10800)
        self.assertEqual(call["status"], "running")
        self.assertEqual(watch.flags([], [call], [], at(10800)), [])
        self.assertEqual(self.calls(10801)[0]["status"], "failed")

    def test_acceptance_exit_requires_an_integer_other_than_bool(self):
        metas = [{"acceptance": {"exit_code": v}} for v in (True, False, "1", 1.0, None)] + [
            {"acceptance": []}, {}, {"acceptance": {"exit_code": 2}}]
        for i, meta in enumerate(metas):
            self.call(i, f"{i:08x}", {"kind": "implement", "valid": True, **meta})
        calls = self.calls()
        self.assertEqual([c["acceptance_exit"] for c in calls], [2] + [None] * (len(metas) - 1))
        self.assertEqual(len(watch.flags([], calls, [], at(1201))), 1)

    def test_failed_launch_deduplicates_call(self):
        cid = self.call(meta={"kind": "review", "valid": False})
        run = run_record(kind="codex", run="codex-review", call_id=cid, exits=[2])
        (flag,) = watch.flags([], self.calls(), [run], at(1201))
        self.assertEqual((flag["kind"], flag["target_kind"], flag["target_id"]), ("run_failed", "run", "run-a"))
        self.assertEqual(flag["text"], f"Codex codex-review: refused or invalid (exit 2) {cid}")
        run["status"] = "cancelled"
        self.assertEqual(watch.flags([], self.calls(), [run], at(1201))[0]["kind"], "codex_failed")


class RunTests(unittest.TestCase):
    def test_only_failed_timed_out_and_interrupted_are_flagged(self):
        states = ("failed", "timed_out", "interrupted", "cancelled", "refused", "passed", "running", "starting")
        runs = [run_record(state, status=state, reason="detail") for state in states]
        flags = watch.flags([], [], runs, at(1201))
        self.assertEqual([f["target_id"] for f in flags], ["failed", "interrupted", "timed_out"])
        self.assertTrue(all(f["kind"] == "run_failed" and f["severity"] == 1 for f in flags))
        self.assertEqual(flags[0]["text"], "Tests: failed; detail")

    def test_codex_exit_text_uses_the_last_exit(self):
        for code, text in [(2, "refused or invalid (exit 2)"), (3, "timeout (exit 3)"),
                           (4, "acceptance failed (exit 4)"), (9, "exit 9"), (None, "interrupted")]:
            with self.subTest(code=code):
                run = run_record(kind="codex", run="codex-implement", status="interrupted", exits=[0, code], call_id=CALL)
                (flag,) = watch.flags([], [], [run], at(1201))
                self.assertEqual(flag["text"], f"Codex codex-implement: {text} {CALL}")
        run = run_record(kind="codex", run="codex-plan", exits=[], call_id=None)
        self.assertEqual(watch.flags([], [], [run], at(1201))[0]["text"], "Codex codex-plan: failed")

    def test_timestamp_fallbacks_and_window(self):
        runs = [run_record("ended", ended=iso(3)), run_record("started", ended=None),
                run_record("created", ended=None, started=None)]
        flags = watch.flags([], [], runs, at(10))
        self.assertEqual([(f["target_id"], f["t"]) for f in flags],
                         [("ended", at(3)), ("started", at(1)), ("created", at(0))])
        self.assertEqual(watch.flags([], [], runs, at(25 * 3600)), [])
        self.assertEqual(len(watch.flags([], [], [run_record(ended=iso(0))], at(24 * 3600))), 1)

    def test_interrupted_attempt_is_independent_of_running_attempt(self):
        runs = [run_record("attempt-a", status="interrupted"),
                run_record("attempt-b", status="running", ended=None)]
        (flag,) = watch.flags([], [], runs, at(1201))
        self.assertEqual(flag["target_id"], "attempt-a")

    def test_codex_overdue_thresholds_and_registry_exclusion(self):
        now = 10000
        runs = [run_record("implement-over", kind="codex", run="codex-implement", status="running", started=iso(now - 7201)),
                run_record("implement-under", kind="codex", run="codex-implement", status="running", started=iso(now - 7100)),
                run_record("review-over", kind="codex", run="codex-review", status="running", started=iso(now - 5401)),
                run_record("registry", status="running", started=iso(now - 3 * 3600)),
                run_record("unknown", kind="codex", run="codex-unknown", status="running", started=iso(0))]
        flags = watch.flags([], [], runs, at(now))
        self.assertEqual([f["target_id"] for f in flags], ["review-over", "implement-over"])
        self.assertEqual(flags[1]["text"], "Codex codex-implement running for 2.0 h, beyond the expected 2.0 h; Stop ends it")
        self.assertTrue(all(f["kind"] == "run_overdue" and f["severity"] == 2 for f in flags))

    def test_running_codex_launch_has_no_watch_window(self):
        run = run_record(kind="codex", run="codex-plan", status="running", started=iso(0), ended=None)
        (flag,) = watch.flags([], [], [run], at(25 * 3600))
        self.assertEqual((flag["kind"], flag["t"]), ("run_overdue", at(0)))


class RobustnessTests(FixtureTests):
    def test_sort_by_severity_then_newest_then_target_id(self):
        self.session(events=[ev(1, "PermissionRequest", tool="Read", key="k")], sid="wait")
        self.session([user(2)], sid="stall")
        cid = self.call(4, meta={"kind": "plan", "valid": False})
        flags = self.flags(calls=self.calls(), runs=[run_record("b", ended=iso(3)), run_record("a", ended=iso(3))])
        self.assertEqual([(f["severity"], f["target_id"]) for f in flags],
                         [(0, "wait"), (1, cid), (1, "a"), (1, "b"), (2, "stall")])
        self.assertTrue(all(isinstance(f["t"], float) for f in flags))
        self.assertTrue(all(set(f) == {"kind", "severity", "target_kind", "target_id", "text", "t"} for f in flags))

    def test_malformed_items_are_counted_and_count_resets(self):
        self.session()
        (session,) = self.discover()
        session.status = None
        call = {"status": "failed", "time": at(0)}
        self.assertEqual(watch.flags([session], [call], [run_record(status=5)], at(1201)), [])
        self.assertEqual(watch.flags.skipped, 3)
        self.assertEqual(watch.flags([], [], [], at(1201)), [])
        self.assertEqual(watch.flags.skipped, 0)

    def test_wrong_nested_types_and_nonfinite_times_are_skipped(self):
        self.session()
        (session,) = self.discover()
        bad_statuses = [{"waits": [5]}, {"wait_since": "yesterday"}, {"last_any": float("nan")},
                        {"open_tools": [{"agent": None, "name": "Bash", "t": at(1), "wrapper": "yes"}]},
                        {"subagents": {"child": None}}]
        for fields in bad_statuses:
            with self.subTest(fields=fields):
                session.status = sources.Status(**{"status": "running", "last_any": at(0), **fields})
                self.assertEqual(watch.flags([session], [], [], at(1201)), [])
                self.assertEqual(watch.flags.skipped, 1)
        calls = [{"call_id": CALL, "status": "failed", "time": t} for t in (None, True, "0", float("inf"))]
        calls += [{"call_id": CALL, "status": "failed", "time": at(0), "problems": [5]}]
        runs = [run_record(exits=[True]), run_record(ended=5), run_record(reason=[])]
        self.assertEqual(watch.flags([], calls, runs, at(1201)), [])
        self.assertEqual(watch.flags.skipped, len(calls) + len(runs))

    def test_unknown_status_suppresses_failure_and_stall_but_keeps_known_wait(self):
        self.session(events=[ev(1, "StopFailure")])
        (session,) = self.discover()
        session.status.status = "unknown"
        self.assertEqual(watch.flags([session], [], [], at(1201)), [])
        session.status.waits = ["permission (Bash)"]
        session.status.wait_since = at(2)
        (flag,) = watch.flags([session], [], [], at(1201))
        self.assertEqual(flag["kind"], "owner_wait")

    def test_watch_has_no_io_clock_or_qt_dependencies(self):
        tree = ast.parse((WB / "watch.py").read_text(encoding="utf-8"))
        imports = {a.name.split(".")[0] for node in ast.walk(tree) if isinstance(node, ast.Import) for a in node.names}
        imports |= {node.module.split(".")[0] for node in ast.walk(tree) if isinstance(node, ast.ImportFrom) and node.module}
        self.assertFalse(imports & {"time", "datetime", "os", "pathlib", "subprocess", "PySide6", "PyQt6", "runs"})
        with patch.object(sources.time, "time", side_effect=AssertionError("clock read")), \
                patch("builtins.open", side_effect=AssertionError("I/O")):
            self.assertEqual(watch.flags([], [], [], at(0)), [])


class SessionLaunchTests(FixtureTests):
    def packet(self, checkout, name="p.md"):
        file = checkout / "work" / "reviews" / "packets" / name
        file.parent.mkdir(parents=True, exist_ok=True)
        file.write_text("content never goes in argv: & % ^ !\nsecond line", encoding="utf-8")
        return file

    def test_exact_argv_for_main_and_worktrees_with_remote(self):
        for checkout in (self.fx.main, self.fx.wt, self.fx.ext):
            for name in ("p.md", "Packet-1_2.3.md", "0.md", "a" * 100 + ".md"):
                self.packet(checkout, name)
                for remote in (False, True):
                    with self.subTest(checkout=checkout.name, name=name, remote=remote):
                        prompt = f"Work on the problem packet work/reviews/packets/{name}. Read it first, then follow CLAUDE.md."
                        self.assertEqual(launch.plan_session(self.fx.main, checkout, name, remote),
                                         (["claude", prompt] + (["--remote-control"] if remote else []), str(checkout.resolve())))
                        self.assertRegex(prompt, r"^[A-Za-z0-9 ._/,\-]+$")
        self.assertNotIn("claude", launch.KEEP_OPEN)

    def test_checkout_comparison_uses_resolved_case_normalized_paths(self):
        self.packet(self.fx.wt)
        checkout = self.fx.wt / ".." / self.fx.wt.name
        if os.name == "nt":
            checkout = Path(str(checkout).upper())
        self.assertEqual(launch.plan_session(self.fx.main, checkout, "p.md")[1], str(checkout.resolve()))

    def test_outside_checkout_is_refused(self):
        self.packet(self.fx.outsider)
        with self.assertRaises(launch.LaunchRefused):
            launch.plan_session(self.fx.main, self.fx.outsider, "p.md")

    def test_invalid_packet_names_are_refused(self):
        bad_names = ["dir/p.md", r"dir\p.md", "../p.md", "..", "..md", "p%.md", "p&.md", "p^.md", "p!.md",
                     "a b.md", ".p.md", "-p.md", "p.txt", "p.MD", "p.md\n", "a" * 101 + ".md", None]
        for name in bad_names:
            with self.subTest(name=name):
                with self.assertRaises(launch.LaunchRefused):
                    launch.plan_session(self.fx.main, self.fx.main, name)

    def test_missing_packet_and_directory_are_refused(self):
        directory = self.fx.main / "work" / "reviews" / "packets" / "directory.md"
        directory.mkdir(parents=True)
        for name in ("missing.md", "directory.md"):
            with self.subTest(name=name):
                with self.assertRaises(launch.LaunchRefused):
                    launch.plan_session(self.fx.main, self.fx.main, name)

    def test_resolved_packet_escape_is_refused(self):
        file = self.packet(self.fx.main)
        outside = self.fx.outsider / "p.md"
        outside.parent.mkdir(parents=True, exist_ok=True)
        outside.write_text("outside", encoding="utf-8")
        resolve = Path.resolve

        def escaped(path, *args, **kwargs):
            return outside if path == file else resolve(path, *args, **kwargs)

        with patch.object(Path, "resolve", escaped):
            with self.assertRaises(launch.LaunchRefused):
                launch.plan_session(self.fx.main, self.fx.main, "p.md")

    @unittest.skipUnless(os.name == "nt", "Windows directory junction fixture")
    def test_junction_packet_directory_cannot_escape(self):
        directory = self.fx.main / "work" / "reviews" / "packets"
        directory.parent.mkdir(parents=True)
        target = self.fx.outsider / "packets"
        target.mkdir(parents=True)
        (target / "p.md").write_text("outside", encoding="utf-8")
        proc = subprocess.run(["cmd", "/c", "mklink", "/J", str(directory), str(target)],
                              capture_output=True, timeout=5)
        if proc.returncode != 0:
            self.skipTest("junction creation unavailable in this sandbox")
        self.addCleanup(directory.rmdir)  # Remove the fixture link before the temporary directory.
        self.assertEqual(directory.resolve(), target.resolve())
        with self.assertRaises(launch.LaunchRefused):
            launch.plan_session(self.fx.main, self.fx.main, "p.md")


if __name__ == "__main__":
    unittest.main(verbosity=2)
