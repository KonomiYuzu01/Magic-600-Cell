"""Headless tests of the Home view derivations (tools/workbench/home.py), with disposable synthetic fixtures only.

The tests in this file up to the marker pin the committed interface; the implementation
packet adds the rest (asks reader rules, every inbox source and gate, gallery bounds and
types, progress view, counts)."""
from __future__ import annotations

import json
import os
import struct
import unittest
import zlib
from datetime import datetime, timezone
from pathlib import Path
from unittest.mock import patch

from test_workbench import Fixture, SID, at, ev, iso, notes, paths, sources, user
import checklist
import home

ASK = "20260930T120100Z-0a0b0c0d"
CARD_KEYS = {"id", "kind", "askKind", "blocking", "sid", "sessionTitle", "t", "age", "title", "context", "detail",
             "options", "attachments", "answerRef", "state", "needsOwner", "answer", "canOpenSession", "files"}
ITEM_KEYS = {"path", "rel", "checkout", "type", "ext", "mtime", "size", "preview", "pixels", "source", "origins",
             "sessions", "isNew"}


def png(width: int = 4, height: int = 3) -> bytes:
    """A valid grey PNG of the given size (standard library only)."""
    def chunk(kind: bytes, data: bytes) -> bytes:
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))
    rows = b"".join(b"\x00" + b"\x80" * width for _ in range(height))
    return (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 0, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(rows)) + chunk(b"IEND", b""))


def item(iid, weight=1, done=False, evidence=None):
    return {"id": iid, "title": iid.upper(), "weight": weight, "done": done, "evidence": evidence}


def step(sid, track, status, items, **extra):
    return {"id": sid, "track": track, "title": f"Step {sid}", "status": status, "acceptance": None,
            "acceptance_source": None, "blocker": None, **extra, "items": items}


def sample_status() -> dict:
    """A synthetic schema-2 progress document; tests never depend on the real status.json content."""
    return {"schema": 2, "updated": "2026-09-30", "current": "0.4.1-2", "paid_api_ceiling_usd": 100, "steps": [
        step("0.4.1-1", "0.4.1", "in_progress", [item("a", 1, True, "0123abc"), item("b", 3)]),
        step("0.4.1-2", "0.4.1", "not_started", [item("harness", 3), item("runs")], weight=2),
        step("2.0", "stage-2", "not_started", [item("charter")])]}


class HomeFixtureTests(unittest.TestCase):
    def setUp(self):
        mkdir = os.mkdir

        def fixture_mkdir(path, mode=0o777, *, dir_fd=None):
            # Python 3.14's Windows 0700 ACL excludes the sandbox token; inherit fixture permissions.
            return mkdir(path, 0o777 if os.name == "nt" and mode == 0o700 else mode, dir_fd=dir_fd)

        with patch("tempfile._os.mkdir", fixture_mkdir):
            self.fx = Fixture()
        self.addCleanup(self.fx.cleanup)

    def session(self, records=(), events=(), sid=SID, checkout=None):
        root = checkout or self.fx.main
        self.fx.transcript(root, sid, [user(0, cwd=str(root)), *records])
        self.fx.events(sid, list(events))

    def discover(self, seconds=120):
        cache: dict = {}
        for _ in range(50):
            found = sources.discover_sessions(self.fx.main, self.fx.projects, now=at(seconds), cache=cache)
            if all(s.state is not None and s.state.caught_up() for s in found):
                return found
        self.fail("history never caught up")

    def write_ask(self, sid=SID, ask_id=ASK, **fields):
        record = {"schema": 1, "id": ask_id, "session_id": sid, "created": iso(60), "kind": "decision",
                  "blocking": False, "question": "Which layout?", "context": "Two options.", "options": ["A", "B"],
                  "attachments": [], **fields}
        d = self.fx.data / "asks" / sid
        d.mkdir(parents=True, exist_ok=True)
        (d / f"{ask_id}.json").write_text(json.dumps(record), encoding="utf-8")
        return record

    def inbox(self, seconds=120):
        sessions = self.discover(seconds)
        cards, _skipped = home.inbox(sessions, [], {}, [], [], self.fx.data, paths.checkouts(self.fx.main), at(seconds))
        return cards


class ChecklistTests(unittest.TestCase):
    def test_evidence_forms(self):
        ok = {"0123abc": "sha", "0123456789abcdef0123456789abcdef01234567": "sha",
              "https://github.com/KonomiYuzu01/Magic-600-Cell/pull/20": "pr", "tests/result.txt": "path",
              "docs/progress/0.4.1/screening-findings.md": "path"}
        for ref, kind in ok.items():
            self.assertEqual(checklist.evidence_kind(ref), kind, ref)
        for ref in (None, "", "https://github.com/other/repo/pull/1",
                    "https://github.com/KonomiYuzu01/Magic-600-Cell/pull/0", "work/result.txt", "../x.txt",
                    "tests/../x.txt", "./tests/x.txt", "/tests/x.txt", "C:/x.txt", "tests\\x.txt", "a" * 201, 7,
                    "Work/result.txt", "WORK/result.txt", "0123abc\n", "tests/result.txt\n",
                    "https://github.com/KonomiYuzu01/Magic-600-Cell/pull/20\n"):
            self.assertIsNone(checklist.evidence_kind(ref), ref)

    def test_percentages_count_only_done_items_with_evidence(self):
        doc = sample_status()
        s1, s2, s3 = doc["steps"]
        self.assertEqual(checklist.step_percent(s1), 25.0)
        self.assertEqual(checklist.step_percent(s2), 0.0)
        self.assertAlmostEqual(checklist.track_percent(doc, "0.4.1"), 25.0 / 3)   # step weights 1 and 2
        self.assertEqual(checklist.track_percent(doc, "stage-2"), 0.0)
        s1["items"][1]["done"] = True                                           # done without evidence
        self.assertEqual(checklist.step_percent(s1), 25.0)
        self.assertEqual([i["id"] for i in checklist.remaining(s1)], ["b"])
        self.assertIsNone(checklist.step_percent({"id": "x"}))
        self.assertIsNone(checklist.track_percent({**doc, "schema": 1}, "0.4.1"))
        s1["items"][1].update(done=False, evidence="tests/result.txt")           # evidence on an open item: allowed, not counted
        self.assertEqual(checklist.validate(doc), [])
        self.assertEqual(checklist.step_percent(s1), 25.0)
        s1["items"][1]["weight"] = 0
        self.assertIsNone(checklist.step_percent(s1))

    def test_validate_names_every_broken_rule(self):
        self.assertEqual(checklist.validate(sample_status()), [])
        cases = [
            (lambda d: d.update(schema=1), "schema-2"),
            (lambda d: d.update(current="nope"), "current"),
            (lambda d: d["steps"].append(dict(d["steps"][0])), "duplicate step"),
            (lambda d: d["steps"][0].update(track="x"), "track"),
            (lambda d: d["steps"][0].update(status="x"), "status"),
            (lambda d: d["steps"][0].update(weight=0), "step weight"),
            (lambda d: d["steps"][0].update(items=[]), "no checklist"),
            (lambda d: d["steps"][0]["items"][1].update(id="B"), "invalid"),
            (lambda d: d["steps"][0]["items"][1].update(id="a"), "duplicate item"),
            (lambda d: d["steps"][0]["items"][1].update(title=" "), "title"),
            (lambda d: d["steps"][0]["items"][1].update(weight=True), "weight"),
            (lambda d: d["steps"][0]["items"][1].update(done="yes"), "true or false"),
            (lambda d: d["steps"][0]["items"][1].update(done=True), "evidence"),
            (lambda d: d["steps"][0].update(status="not_started"), "not_started"),
            (lambda d: d["steps"][0].update(status="done"), "not every item"),
        ]
        for change, words in cases:
            doc = sample_status()
            change(doc)
            problems = checklist.validate(doc)
            self.assertTrue(any(words in p for p in problems), (words, problems))

    def test_dump_is_stable_json_with_one_line_per_item(self):
        doc = sample_status()
        text = checklist.dump(doc)
        self.assertEqual(json.loads(text), doc)
        self.assertEqual(checklist.dump(json.loads(text)), text)
        lines = text.splitlines()
        self.assertEqual(sum(1 for line in lines if line.startswith('      {"id": ')), 5)
        doc["steps"][1]["items"][0].update(done=True, evidence="tests/result.txt")
        changed = [a for a, b in zip(lines, checklist.dump(doc).splitlines()) if a != b]
        self.assertEqual(len(changed), 1)  # a checked item is a one-line diff


class AnswerTextTests(unittest.TestCase):
    def test_answer_text_format(self):
        self.assertEqual(home.answer_text(f"ask {ASK}", "A"), f"answer to ask {ASK}: A")
        self.assertRegex(home.answer_text(f"ask {ASK}", "A"), home.ANSWER_RE)
        self.assertEqual(home.answer_text("waiting-for 2026-09-30T12:00:00+00:00", " yes "),
                         "answer to waiting-for 2026-09-30T12:00:00+00:00: yes")
        for ref, body in ((f"task {ASK}", "A"), (f"ask {ASK}", ""), (f"ask {ASK}", "a\nb"), (f"ask {ASK}", "a\rb"),
                          (f"ask {ASK} x", "A"), (f"ask {ASK}", "x" * notes.NOTE_MAX_BYTES)):
            with self.assertRaises(ValueError):
                home.answer_text(ref, body)

    def test_machine_badge_counts_failures_and_stalls_only(self):
        flags = [{"kind": k} for k in ("owner_wait", "failed_turn", "stalled", "codex_failed", "run_failed", "run_overdue")]
        self.assertEqual(home.machine_badge(flags + ["junk", {}]), 5)


class AskCardTests(HomeFixtureTests):
    def test_ask_card_moves_from_open_to_queued_to_answered(self):
        self.session()
        self.write_ask()
        (card,) = [c for c in self.inbox() if c["kind"] == "ask"]
        self.assertEqual(set(card), CARD_KEYS)
        self.assertEqual((card["id"], card["state"], card["answerRef"], card["options"]), (f"ask:{ASK}", "open", f"ask {ASK}", ["A", "B"]))
        nid = notes.append_note(self.fx.data, SID, home.answer_text(card["answerRef"], "B"))
        (card,) = [c for c in self.inbox() if c["kind"] == "ask"]
        self.assertEqual((card["state"], card["answer"]), ("queued", "B"))
        with (self.fx.data / "inbox" / f"{SID}.state.jsonl").open("a", encoding="utf-8") as f:
            f.write(json.dumps({"id": nid, "emitted_at": iso(90)}) + "\n")
        (card,) = [c for c in self.inbox() if c["kind"] == "ask"]
        self.assertEqual((card["state"], card["needsOwner"]), ("sent", False))  # sent; delivery not confirmed

    def test_malformed_ask_is_skipped_and_counted(self):
        self.write_ask()
        (self.fx.data / "asks" / SID / "20260930T120200Z-00000000.json").write_text("{not json", encoding="utf-8")
        records, skipped = home.asks(self.fx.data, SID)
        self.assertEqual(([r["id"] for r in records], skipped), ([ASK], 1))


class PermissionCardTests(HomeFixtureTests):
    def test_permission_wait_shows_tool_and_command_and_cannot_be_answered(self):
        self.session(events=[ev(30, "PermissionRequest", tool="Bash", key="k1", summary="git push origin main")])
        (card,) = [c for c in self.inbox() if c["kind"] == "wait"]
        self.assertIn("Bash", card["title"] + card["detail"])
        self.assertIn("git push origin main", card["detail"])
        self.assertEqual((card["answerRef"], card["state"], card["canOpenSession"]), ("", "open", True))


class ProgressViewTests(unittest.TestCase):
    def test_progress_view_matches_the_checklist_rules(self):
        doc = sample_status()
        self.assertEqual(checklist.validate(doc), [])
        view = home.progress_view(doc, [])
        self.assertEqual([t["id"] for t in view["tracks"]], ["0.4.1", "stage-2"])
        for track in view["tracks"]:
            self.assertAlmostEqual(track["percent"], checklist.track_percent(doc, track["id"]))
        schema1 = {**doc, "schema": 1, "steps": [{k: v for k, v in s.items() if k not in ("items", "weight")} for s in doc["steps"]]}
        view = home.progress_view(schema1, [])
        self.assertTrue(view["noChecklist"])
        self.assertTrue(all(t["percent"] is None for t in view["tracks"]))


class SnapshotTests(unittest.TestCase):
    def test_counts_and_snapshot_format(self):
        cards = [{"state": "open", "needsOwner": True}, {"state": "queued", "needsOwner": False},
                 {"state": "sent", "needsOwner": False}]
        items = [{"isNew": True}, {"isNew": False}]
        c = home.counts(cards, items)
        self.assertEqual(c, {"for_you": 1, "gallery_new": 1})
        snap = home.snapshot(c, at(0))
        self.assertEqual(snap, {"schema": 1, "written": datetime.fromtimestamp(at(0), timezone.utc).isoformat(timespec="seconds"),
                                "for_you": 1, "gallery_new": 1})


class GalleryContractTests(HomeFixtureTests):
    def test_gallery_item_shape_and_new_flag(self):
        g = self.fx.main / "work" / "gallery"
        g.mkdir(parents=True)
        (g / "a.png").write_bytes(png(4, 3))
        (g / "b.png").write_bytes(b"\x89PNG\r\n\x1a\n" + b"0" * 64)     # malformed header: listed, never previewed
        os.utime(g / "b.png", (os.stat(g / "a.png").st_mtime - 60,) * 2)
        self.assertEqual(home.image_size(g / "a.png"), (4, 3))
        self.assertIsNone(home.image_size(g / "b.png"))
        items = home.GalleryScanner().scan(self.fx.main, paths.checkouts(self.fx.main), [], [], [], now=Path(g / "a.png").stat().st_mtime + 5)
        item, bad = items
        self.assertEqual(set(item), ITEM_KEYS)
        self.assertEqual((item["type"], item["rel"], item["origins"], item["isNew"]), ("image", "work/gallery/a.png", ["gallery"], True))
        self.assertTrue(Path(item["preview"]).samefile(g / "a.png"))
        self.assertEqual(item["pixels"], [4, 3])
        self.assertEqual((bad["rel"], bad["preview"], bad["pixels"]), ("work/gallery/b.png", "", []))


# ---- implementation packet P1 adds its tests below this line ----


if __name__ == "__main__":
    unittest.main(verbosity=2)
