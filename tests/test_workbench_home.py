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


from test_workbench import assistant, result, watch


def image_headers(width, height):
    ihdr = struct.pack(">IIBBBBB", width, height, 8, 0, 0, 0, 0)
    png_head = (b"\x89PNG\r\n\x1a\n" + struct.pack(">I", 13) + b"IHDR" + ihdr
                + struct.pack(">I", zlib.crc32(b"IHDR" + ihdr)))

    def webp(kind, body):
        chunk = kind + struct.pack("<I", len(body)) + body + b"\0" * (len(body) % 2)
        return b"RIFF" + struct.pack("<I", 4 + len(chunk)) + b"WEBP" + chunk

    # VP8 and VP8L have 14-bit dimensions: use a representable, still oversized canvas.
    w, h = max(1, min(width, 16000)), max(1, min(height, 16000))
    vp8 = b"\x10\0\0\x9d\x01\x2a" + struct.pack("<HH", w, h)
    vp8l = b"\x2f" + struct.pack("<I", (w - 1) | ((h - 1) << 14))
    vp8x = b"\0" * 4 + max(width - 1, 0).to_bytes(3, "little") + max(height - 1, 0).to_bytes(3, "little")
    sof = b"\x08" + struct.pack(">HH", height, width) + b"\x01\x01\x11\0"
    jpeg = b"\xff\xd8\xff\xe0\0\x04ok\xff\xc0" + struct.pack(">H", len(sof) + 2) + sof
    return {"a.png": (png_head, (width, height)),
            "a.gif": (b"GIF87a" + struct.pack("<HH", width, height) + b"\0\0\0", (width, height)),
            "b.gif": (b"GIF89a" + struct.pack("<HH", width, height) + b"\0\0\0", (width, height)),
            "a.webp": (webp(b"VP8 ", vp8), (w, h)),
            "b.webp": (webp(b"VP8L", vp8l), (w, h)),
            "c.webp": (webp(b"VP8X", vp8x), (width, height)),
            "a.jpg": (jpeg, (width, height))}


class DerivationFixtureTests(HomeFixtureTests):
    def file(self, relative, content=b"<svg/>", root=None, t=100):
        p = (root or self.fx.main) / relative
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_bytes(content if isinstance(content, bytes) else content.encode("utf-8"))
        os.utime(p, (at(t), at(t)))
        return p

    def cards(self, sessions=None, calls=(), links=None, runs=(), flags=(), seconds=120):
        return home.inbox(self.discover(seconds) if sessions is None else sessions, list(calls), links or {},
                          list(runs), list(flags), self.fx.data, paths.checkouts(self.fx.main), at(seconds))

    def scan(self, sessions=(), runs=(), asks=(), seconds=120, scanner=None, force=False, roots=None):
        return (scanner or home.GalleryScanner()).scan(self.fx.main, roots or paths.checkouts(self.fx.main),
                                                      list(sessions), list(runs), list(asks), at(seconds), force)

    def brief(self, sid=SID, **fields):
        return self.file(f"work/loop-memory/workbench/briefs/{sid}.json", json.dumps({
            "updated": iso(50), "waiting_for": "The owner to choose a design", "goal": "Ship", "step": "Review",
            **fields}))

    def note(self, sid=SID, text=None, nid="20260930T120130Z-11111111", time=iso(80), emitted=False):
        p = self.fx.data / "inbox" / f"{sid}.jsonl"
        p.parent.mkdir(parents=True, exist_ok=True)
        with p.open("a", encoding="utf-8") as f:
            f.write(json.dumps({"id": nid, "time": time, "text": text or home.answer_text(f"ask {ASK}", "B")}) + "\n")
        if emitted:
            with p.with_suffix(".state.jsonl").open("a", encoding="utf-8") as f:
                f.write(json.dumps({"id": nid, "emitted_at": iso(90)}) + "\n")
        return nid

    def symlink(self, target, link, directory=False):
        try:
            link.symlink_to(target, target_is_directory=directory)
        except (OSError, NotImplementedError):
            self.skipTest("symlink creation unavailable in this sandbox")


class AskReaderRulesTests(DerivationFixtureTests):
    def test_every_malformed_field_is_rejected(self):
        changes = [
            {"schema": 2}, {"schema": True}, {"id": "wrong"}, {"id": ASK + "\n"},
            {"id": "20260930T120100Z-aaaaaaaa"}, {"session_id": "other"}, {"created": "bad"},
            {"created": 1}, {"kind": "vote"}, {"kind": []}, {"blocking": 1},
            {"question": ""}, {"question": " "}, {"question": 1},
            {"question": "x" * 501}, {"question": "\U0001f600" * 126},
            {"question": "a\nb"}, {"question": "a\rb"},
            {"context": None}, {"context": "\U0001f600" * 251},
            {"options": "A"}, {"options": [str(i) for i in range(9)]},
            {"options": ["A", "A"]}, {"options": ["", "B"]}, {"options": [1, "B"]},
            {"options": ["\U0001f600" * 31, "B"]}, {"options": ["a\nb", "B"]},
            {"options": ["a\rb", "B"]}, {"options": ["A"]},
            {"attachments": "x"}, {"attachments": ["x"] * 9}, {"attachments": [1]},
            {"kind": "pick", "options": ["A", "B"], "attachments": ["a.png", "b.png"]},
            {"kind": "pick", "options": [], "attachments": ["a.png"]},
            {"kind": "pick", "options": [], "attachments": ["a.png", "b.txt"]},
        ]
        for fields in changes:
            with self.subTest(fields=fields):
                self.write_ask(**fields)
                self.assertEqual(home.asks(self.fx.data, SID), ([], 1))
        self.write_ask(kind="pick", options=[], attachments=["a.png", "b.PNG"])   # ask.py compares extensions in lower case
        self.assertEqual((len(home.asks(self.fx.data, SID)[0]), home.asks(self.fx.data, SID)[1]), (1, 0))
        valid = self.write_ask()
        for key in valid:
            with self.subTest(missing=key):
                p = self.fx.data / "asks" / SID / f"{ASK}.json"
                p.write_text(json.dumps({k: v for k, v in valid.items() if k != key}), encoding="utf-8")
                self.assertEqual(home.asks(self.fx.data, SID), ([], 1))

    def test_kind_and_utf8_boundaries_are_accepted(self):
        self.write_ask(kind="approve", question="\U0001f600" * 125, context="\U0001f600" * 250,
                       options=["\U0001f600" * 30], attachments=["missing"] * 8)
        records, skipped = home.asks(self.fx.data, SID)
        self.assertEqual((len(records), skipped, records[0]["attachments"]), (1, 0, []))
        self.write_ask(kind="question", options=[])
        self.assertEqual(len(home.asks(self.fx.data, SID)[0]), 1)
        self.write_ask(kind="pick", options=[], attachments=["missing.png", "missing.svg"])
        self.assertEqual(len(home.asks(self.fx.data, SID)[0]), 1)

    def test_invalid_sid_missing_and_unreadable_files(self):
        self.assertEqual(home.asks(self.fx.data, "../bad"), ([], 0))
        self.assertEqual(home.asks(self.fx.data, SID), ([], 0))
        self.write_ask()
        with patch.object(Path, "open", side_effect=OSError("unreadable")):
            self.assertEqual(home.asks(self.fx.data, SID), ([], 1))
        p = self.fx.data / "asks" / SID / f"{ASK}.json"
        valid = self.write_ask()
        for content in ("[]", "null", json.dumps(valid) + " " * home.ASK_MAX_BYTES):
            p.write_text(content, encoding="utf-8")
            self.assertEqual(home.asks(self.fx.data, SID), ([], 1))

    def test_json_directories_do_not_consume_the_file_limit(self):
        self.write_ask()
        (self.fx.data / "asks" / SID / "zzz.json").mkdir()
        with patch.object(home, "ASKS_PER_SESSION", 1):
            records, skipped = home.asks(self.fx.data, SID)
        self.assertEqual(([r["id"] for r in records], skipped), ([ASK], 0))

    def test_ask_directory_listing_is_bounded_and_reports_the_rest_as_skipped(self):
        for i in range(5):
            self.write_ask(ask_id=f"20260930T120100Z-{i:08x}", created=iso(10 + i))
        listed, self_dir = [], self.fx.data / "asks" / SID
        real = os.scandir

        class Counting:   # os.scandir is patched process-wide; count only the ask directory
            def __init__(self, path):
                self.it, self.ours = real(path), Path(path) == self_dir

            def __enter__(self):
                return self

            def __exit__(self, *exc):
                self.it.close()

            def __iter__(self):
                return self

            def __next__(self):
                entry = next(self.it)
                if self.ours:
                    listed.append(entry.name)
                return entry

        with patch.object(home, "ASK_SCAN_MAX", 3), patch.object(home.os, "scandir", Counting):
            records, skipped = home.asks(self.fx.data, SID)
        self.assertEqual((len(records), skipped), (3, 1))
        self.assertEqual(len(listed), 4)   # the limit plus one entry to detect the rest
        records, skipped = home.asks(self.fx.data, SID)
        self.assertEqual((len(records), skipped), (5, 0))

    def test_only_the_newest_hundred_filenames_are_read_then_sorted_by_created(self):
        for i in range(103):
            self.write_ask(ask_id=f"20260930T120100Z-{i:08x}", created=iso(103 - i))
        old = self.fx.data / "asks" / SID / "20260930T120100Z-00000000.json"
        old.write_text("invalid", encoding="utf-8")
        records, skipped = home.asks(self.fx.data, SID)
        self.assertEqual((len(records), skipped), (100, 0))
        self.assertEqual([r["id"] for r in records], [f"20260930T120100Z-{i:08x}" for i in range(3, 103)])

    def test_attachments_keep_original_indexes_and_resolve_default_checkouts(self):
        a = self.file("docs/a.svg")
        b = self.file("docs/b.svg", root=self.fx.ext)
        outside = self.file("a.svg", root=self.fx.outsider)
        self.write_ask(attachments=[str(outside), str(a), str(a.parent), str(b), "missing.png"])
        records, skipped = home.asks(self.fx.data, SID)
        self.assertEqual((records[0]["attachments"], skipped),
                         ([{"index": 2, "path": str(a.resolve())}, {"index": 4, "path": str(b.resolve())}], 0))
        self.assertEqual(home.asks(self.fx.data, SID, [self.fx.main])[0][0]["attachments"],
                         [{"index": 2, "path": str(a.resolve())}])

    def test_attachment_symlink_cannot_escape_roots(self):
        outside = self.file("outside.svg", root=self.fx.outsider)
        linked = self.fx.main / "escape.svg"
        self.symlink(outside, linked)
        self.write_ask(attachments=[str(linked)])
        self.assertEqual(home.asks(self.fx.data, SID)[0][0]["attachments"], [])


class InboxRulesTests(DerivationFixtureTests):
    def test_ask_fields_pick_indexes_and_deepest_checkout(self):
        self.session(sid=SID, checkout=self.fx.wt)
        a = self.file("figures/design.png", png(), root=self.fx.wt)
        b = self.file("figures/design.png", png(), root=self.fx.ext)
        self.write_ask(kind="pick", options=[], blocking=True, attachments=[str(a), str(b)])
        (card,), skipped = self.cards()
        self.assertEqual(set(card), CARD_KEYS)
        self.assertEqual((card["sid"], card["sessionTitle"], card["t"], card["age"], card["askKind"],
                          card["blocking"], card["detail"]), (SID, "do it", at(60), "1 min", "pick", True, ""))
        self.assertEqual(card["options"], ["picked 1: wt1/figures/design.png", "picked 2: feature/figures/design.png"])
        self.assertEqual(card["attachments"], [
            {"index": 1, "path": str(a.resolve()), "rel": "figures/design.png", "checkout": "wt1", "previewable": True},
            {"index": 2, "path": str(b.resolve()), "rel": "figures/design.png", "checkout": "feature", "previewable": True}])
        self.assertEqual(skipped, 0)
        a.unlink()
        (card,), _ = self.cards()
        self.assertEqual(card["options"], ["picked 2: feature/figures/design.png"])
        self.assertEqual(card["detail"], "Some attachments are no longer available.")

    def test_attachment_preview_bounds_and_title_fallback(self):
        self.session()
        big = self.file("big.png", image_headers(20000, 20000)["a.png"][0])
        bad = self.file("bad.gif", b"invalid")
        small = self.file("small.svg")
        large = self.file("large.svg", b"x" * 100)
        self.write_ask(attachments=[str(p) for p in (big, bad, small, large)])
        sessions = self.discover()
        sessions[0].title = ""
        with patch.object(home, "PREVIEW_MAX", 50):
            (card,), _ = self.cards(sessions)
        self.assertEqual([a["previewable"] for a in card["attachments"]], [False, False, True, False])
        self.assertEqual(card["sessionTitle"], SID[:8])

    def test_two_questions_are_distinct_and_resolve_independently(self):
        self.session([assistant(10, uses=[("q1", "AskUserQuestion", {}), ("q2", "AskUserQuestion", {})])])
        sessions = self.discover()
        self.assertEqual([w["key"] for w in sessions[0].status.wait_items], ["q:q1", "q:q2"])
        cards, skipped = self.cards(sessions)
        self.assertEqual((len(cards), skipped, len({c["id"] for c in cards})), (2, 0, 2))
        self.assertEqual({c["title"] for c in cards},
                         {"Question in Claude Code (AskUserQuestion)"})
        self.session([assistant(10, uses=[("q1", "AskUserQuestion", {}), ("q2", "AskUserQuestion", {})]), result(20, "q1")])
        (card,), _ = self.cards()
        self.assertIn(card["id"], [c["id"] for c in cards])
        self.assertEqual((card["answerRef"], card["state"]), ("", "open"))
        self.assertEqual(self.discover()[0].status.wait_items[0]["key"], "q:q2")
        self.assertTrue(card["needsOwner"])
        self.assertEqual(card["context"], "Answer this in Claude Code; the workbench cannot approve or answer it.")
        self.session([assistant(10, uses=[("plan", "ExitPlanMode", {})])])
        (card,), _ = self.cards()
        self.assertEqual(card["title"], "Question in Claude Code (ExitPlanMode)")

    def test_subagent_permission_and_elicitation_waits_are_oldest_first(self):
        self.session(events=[ev(30, "PermissionRequest", tool="Bash", key="k", agent_id="child", summary="git status"),
                             ev(20, "Notification", ntype="elicitation_dialog")])
        sessions = self.discover()
        waits = sessions[0].status.wait_items
        self.assertEqual(waits, [
            {"key": "elicit", "agent": None, "t": at(20), "label": "input requested", "tool": None, "summary": ""},
            {"key": "perm:Bash:k", "agent": "child", "t": at(30), "label": "permission (Bash)", "tool": "Bash", "summary": "git status"}])
        cards, _ = self.cards(sessions)
        self.assertEqual([c["title"] for c in cards], ["Permission: Bash (subagent child)", "Input requested in Claude Code"])
        self.assertEqual(cards[0]["detail"], "git status")
        self.assertIn(f"wait:{SID}:child:", cards[0]["id"])
        sig = sources.event_signals([ev(40, "PermissionRequest", tool="Read", key="r", summary=1)])[0]
        self.assertIsNone(sig.data["summary"])
        status = sources.reduce_status([sig])
        self.assertEqual(status.wait_items[0]["summary"], "")
        status.wait_items[0]["summary"] = "changed snapshot"
        self.assertEqual(sig.data["summary"], None)

    def test_failure_gates_use_watch_flags_and_preserve_wrapper_exemption(self):
        call_id = "20260930T120100Z-aaaaaaaa"
        directory = self.fx.main / "work" / "reviews" / call_id
        files = [self.file(f"work/reviews/{call_id}/{name}") for name in ("review.json", "report.md", "meta.json")]
        call = {"call_id": call_id, "dir": str(directory), "time": at(60), "kind": "review", "status": "failed", "problems": ["bad"]}
        self.session()
        sessions = self.discover()
        flags = watch.flags(sessions, [call], [], at(120))
        self.assertEqual(self.cards(sessions, [call], {call_id: (SID, "call id")}, flags=flags)[0], [])
        for status in ("finished", "failed", "waiting"):
            with self.subTest(status=status):
                sessions[0].status.status = status
                (card,), skipped = self.cards(sessions, [call], {call_id: (SID, "call id")}, flags=flags)
                self.assertEqual((card["id"], card["kind"], card["answerRef"], card["files"], skipped),
                                 (f"codex:{call_id}", "failure", f"codex call {call_id}", [str(p) for p in files], 0))
        sessions[0].status.status = "waiting"
        sessions[0].status.waits = ["question"]
        self.assertEqual(self.cards(sessions, [call], {call_id: (SID, "call id")}, flags=flags)[0], [])
        self.session()
        sessions = self.discover(1200)
        flags = watch.flags(sessions, [call], [], at(1200))
        self.assertIn("stalled", [f["kind"] for f in flags])
        self.assertEqual(len(self.cards(sessions, [call], {call_id: (SID, "call id")}, flags=flags, seconds=1200)[0]), 1)
        self.session([assistant(1, uses=[("wrapper", "Bash", {"command": "python tools/agents/codex_review.py --kind review --packet p.md"})])])
        sessions = self.discover(1200)
        flags = watch.flags(sessions, [call], [], at(1200))
        self.assertNotIn("stalled", [f["kind"] for f in flags])
        self.assertEqual(self.cards(sessions, [call], {call_id: (SID, "call id")}, flags=flags, seconds=1200)[0], [])

    def test_unlinked_call_and_run_failure_are_immediate_and_sorted(self):
        self.session()
        log = self.file("work/loop-memory/workbench/runs/run-a.log", "failure")
        flags = [{"kind": "run_failed", "target_id": "run-a", "text": "run failed", "t": at(70)},
                 {"kind": "codex_failed", "target_id": "call-b", "text": "call failed", "t": at(70)},
                 {"kind": "codex_failed", "target_id": "call-a", "text": "call failed", "t": at(60)}]
        cards, skipped = self.cards(calls=[{"call_id": "call-b", "dir": str(log.parent)}], flags=flags,
                                    links={"call-a": ("not-discovered", "call id")})
        self.assertEqual(([c["id"] for c in cards], skipped), (["codex:call-b", "run:run-a", "codex:call-a"], 0))
        self.assertEqual(cards[1]["files"], [str(log)])
        for c in cards:
            self.assertEqual(set(c), CARD_KEYS)
            self.assertEqual((c["answerRef"], c["canOpenSession"]), ("", False))
            self.assertIsInstance(c["t"], float)

    def test_waiting_for_owner_match_window_finished_and_reference(self):
        self.session()
        for waiting, shown in (("Owner to review", True), ("  THE OWNER to decide", True),
                               ("nothing from the owner", False), ("ownership checks", False), ("", False)):
            with self.subTest(waiting=waiting):
                self.brief(waiting_for=waiting)
                cards, _ = self.cards()
                self.assertEqual(bool(cards), shown)
                if shown:
                    self.assertEqual((cards[0]["title"], cards[0]["context"], cards[0]["detail"], cards[0]["answerRef"]),
                                     ("Waiting for you", waiting, "Goal: Ship · Step: Review", f"waiting-for {iso(50)}"))
        for goal, current, detail in (("", "Review", "Step: Review"), ("Ship", "", "Goal: Ship"), ("", "", "")):
            self.brief(goal=goal, step=current)
            self.assertEqual(self.cards()[0][0]["detail"], detail)
        self.brief(updated=iso(120 - watch.WATCH_WINDOW))
        self.assertEqual(len(self.cards()[0]), 1)
        self.brief(updated=iso(119 - watch.WATCH_WINDOW))
        self.assertEqual(self.cards()[0], [])
        self.brief(updated="invalid")
        self.assertEqual(self.cards()[0], [])
        self.brief(updated="2026-09-30 12:00:50+00:00")
        self.assertEqual(self.cards()[0][0]["answerRef"], "")
        self.brief()
        self.session(events=[ev(80, "SessionEnd")])
        self.assertEqual(self.cards()[0], [])

    def test_bad_source_items_are_counted_without_hiding_valid_cards(self):
        self.session()
        self.write_ask()
        sessions = self.discover()
        sessions[0].status.wait_items = [None, {"key": "bad", "agent": None, "label": "question", "t": "bad"}]
        cards, skipped = self.cards([None, *sessions], calls=[None], flags=[None, {"kind": "run_failed"}])
        self.assertEqual(([c["id"] for c in cards], skipped), ([f"ask:{ASK}"], 6))

    def test_bad_failure_text_is_counted(self):
        cards, skipped = self.cards(sessions=[], flags=[{"kind": "run_failed", "target_id": "bad", "t": at(60), "text": []}])
        self.assertEqual((cards, skipped), ([], 1))


class DeliveryRulesTests(DerivationFixtureTests):
    def test_latest_time_then_list_order_and_leading_ref_only(self):
        ref = f"ask {ASK}"
        entries = [
            {"id": "new", "time": "b", "text": home.answer_text(ref, "B"), "state": "emitted"},
            {"id": "old", "time": "a", "text": home.answer_text(ref, "A"), "state": "answered"},
            {"id": "tie", "time": "b", "text": home.answer_text(ref, "C"), "state": "received"},
            {"id": "quote", "time": "z", "text": "quoted " + home.answer_text(ref, "D"), "state": "answered"}, None]
        self.assertEqual(home.answer_states(entries), {ref: {"state": "sent", "note": "tie", "body": "C"}})
        for state, expected in (("pending", "queued"), ("emitted", "sent"), ("received", "sent"), ("answered", "acknowledged")):
            entries[0]["state"] = state
            self.assertEqual(home.answer_states(entries[:1])[ref]["state"], expected)

    def test_emitted_without_receipt_and_unrelated_tool_output_stay_sent(self):
        self.session()
        self.write_ask()
        nid = self.note(emitted=True)
        (card,), _ = self.cards()
        self.assertEqual((card["state"], card["needsOwner"]), ("sent", False))
        self.session([result(100, "unrelated", f"tool output mentions {nid}")])
        (card,), _ = self.cards()
        self.assertEqual((card["state"], card["needsOwner"]), ("sent", False))
        self.session([user(100, f"Owner note {nid}"), assistant(110, f"Answering {nid}: done")])
        (card,), _ = self.cards()
        self.assertEqual((card["state"], card["needsOwner"], card["answer"]), ("acknowledged", False, "B"))

    def test_only_owning_inbox_is_read_once_and_queued_needs_a_wake_after_turn(self):
        self.session()
        self.session(sid="other", checkout=self.fx.ext)
        self.write_ask()
        self.brief()
        self.note(sid="other")
        self.note(text="Quoted: " + home.answer_text(f"ask {ASK}", "C"))
        with patch.object(sources, "notes", wraps=sources.notes) as reader:
            cards, _ = self.cards()
        self.assertEqual(reader.call_count, 1)
        self.assertEqual(reader.call_args.args[1], SID)
        self.assertTrue(all(c["state"] == "open" for c in cards))
        self.note()
        card = next(c for c in self.cards()[0] if c["kind"] == "ask")
        self.assertEqual((card["state"], card["needsOwner"]), ("queued", False))
        self.session(events=[ev(110, "Stop")])
        card = next(c for c in self.cards()[0] if c["kind"] == "ask")
        self.assertEqual((card["state"], card["needsOwner"]), ("queued", True))

    def test_failure_and_brief_answers_share_the_same_delivery_rules(self):
        self.session(events=[ev(100, "Stop")])
        self.brief()
        self.note(text=home.answer_text(f"waiting-for {iso(50)}", "continue"))
        self.note(text=home.answer_text("codex call c1", "retry"))
        flags = [{"kind": "codex_failed", "target_id": "c1", "text": "failed", "t": at(60)}]
        cards, _ = self.cards(flags=flags, links={"c1": (SID, "call id")})
        self.assertEqual([(c["state"], c["answer"], c["needsOwner"]) for c in cards],
                         [("queued", "retry", True), ("queued", "continue", True)])


class ImageHeaderTests(DerivationFixtureTests):
    def test_all_formats_dimensions_without_decoding_and_oversized_preview_refused(self):
        for width, height in ((4, 3), (20000, 20000)):
            for name, (content, expected) in image_headers(width, height).items():
                with self.subTest(size=(width, height), format=name):
                    p = self.file("work/gallery/" + name, content)
                    self.assertEqual(home.image_size(p), expected)
                    item = next(i for i in self.scan() if i["path"] == str(p))
                    self.assertEqual(item["pixels"], list(expected))
                    self.assertEqual(bool(item["preview"]), width == 4)

    def test_truncated_unknown_zero_and_unreadable_headers(self):
        p = self.file("a.bin")
        for name, (content, _) in image_headers(4, 3).items():
            for cut in (0, 5, len(content) - 3):
                with self.subTest(format=name, cut=cut):
                    p.write_bytes(content[:cut])
                    self.assertIsNone(home.image_size(p))
        for content in (b"not an image", image_headers(0, 3)["a.png"][0], image_headers(4, 0)["a.gif"][0],
                        b"\xff\xd8\xff\xe0\0\x01", b"RIFF\0\0\0\0WEBPVP8X\0\0\0\0"):
            p.write_bytes(content)
            self.assertIsNone(home.image_size(p))
        with patch.object(Path, "open", side_effect=OSError("unreadable")):
            self.assertIsNone(home.image_size(p))

    def test_jpeg_markers_and_header_budget(self):
        content = image_headers(4, 3)["a.jpg"][0]
        p = self.file("a.jpg", content)
        for marker in range(0xc0, 0xd0):
            p.write_bytes(content.replace(b"\xff\xc0", bytes([0xff, marker])))
            self.assertEqual(home.image_size(p), None if marker in (0xc4, 0xc8, 0xcc) else (4, 3))
        p.write_bytes(content.replace(b"\xff\xc0", b"\xff\xff\xc0"))
        self.assertEqual(home.image_size(p), (4, 3))
        with patch.object(home, "IMAGE_HEAD_BYTES", 10):
            self.assertIsNone(home.image_size(p))


class GalleryRulesTests(DerivationFixtureTests):
    def test_every_type_recognition_and_sibling_folding(self):
        files = {"photo.PNG": png(), "vector.svg": b"<svg/>", "paper.pdf": b"%PDF", "movie.mp4": b"video",
                 "clip.webm": b"video", "map.mmd": b"graph TD", "map.svg": b"<svg/>",
                 "flow.drawio": b"xml", "flow.drawio.png": png(), "typeset.typ": b"text",
                 "deck.md": b"---\nmarp: true\n---\n# slide", "deck.png": png(),
                 "app.py": b"import marimo\napp = marimo.App()", "ordinary.md": b"# not marp\nmarp: true",
                 "ordinary.py": b"print(1)", "late.md": b"---\n---\nmarp: true", "text.txt": b"text"}
        for name, content in files.items():
            self.file("work/gallery/" + name, content)
        by_name = {Path(i["path"]).name: i for i in self.scan()}
        self.assertEqual({n: i["type"] for n, i in by_name.items()},
                         {"photo.PNG": "image", "vector.svg": "vector", "paper.pdf": "pdf", "movie.mp4": "video",
                          "clip.webm": "video", "map.mmd": "diagram", "flow.drawio": "diagram",
                          "typeset.typ": "diagram", "deck.md": "slides", "app.py": "notebook"})
        for name, preview in (("map.mmd", "map.svg"), ("flow.drawio", "flow.drawio.png"), ("deck.md", "deck.png")):
            self.assertEqual(Path(by_name[name]["preview"]).name, preview)
            self.assertEqual(by_name[name]["source"], by_name[name]["path"])
        for name in ("paper.pdf", "movie.mp4", "clip.webm", "typeset.typ", "app.py"):
            self.assertEqual(by_name[name]["preview"], "")
        self.assertEqual(by_name["deck.md"]["pixels"], [4, 3])

    def test_diagram_sibling_priority_and_only_previewable_siblings_fold(self):
        self.file("work/gallery/a.mmd", "graph TD")
        self.file("work/gallery/a.svg", b"x" * 100)
        p = self.file("work/gallery/a.png", png())
        self.file("work/gallery/a.mmd.svg")
        with patch.object(home, "PREVIEW_MAX", 90):
            items = self.scan()
        diagram = next(i for i in items if i["type"] == "diagram")
        self.assertEqual(diagram["preview"], str(p))
        self.assertIn("a.svg", [Path(i["path"]).name for i in items])

    def test_all_origins_merge_sessions_newest_first_and_main_figures_only(self):
        p = self.file("work/gallery/output.svg", root=self.fx.wt)
        self.session([assistant(10, uses=[("read", "Read", {"file_path": str(p)})])])
        self.session([assistant(20, uses=[("read", "Read", {"file_path": str(p)})])], sid="newest", checkout=self.fx.ext)
        self.file("work/loop-memory/ui-vision/shot.svg", root=self.fx.ext)
        self.file("docs/design/figures/main.svg")
        self.file("docs/figures/direct.svg")
        self.file("docs/design/figures/ignored.svg", root=self.fx.wt)
        run = {"started": iso(80), "ended": iso(90), "checkout": str(self.fx.wt), "outputs": ["work/gallery/output.svg"]}
        asks = [{"session_id": SID, "attachments": [{"path": str(p)}]}]
        items = self.scan(self.discover(), [run], asks)
        out = next(i for i in items if i["path"] == str(p))
        self.assertEqual(set(out["origins"]), {"referenced", "run-output", "ask-attachment", "gallery"})
        self.assertEqual(out["sessions"], ["newest", SID])
        self.assertEqual((out["rel"], out["checkout"]), ("work/gallery/output.svg", "wt1"))
        self.assertEqual({i["type"] for i in items}, {"vector"})
        self.assertEqual({Path(i["path"]).name for i in items}, {"output.svg", "shot.svg", "main.svg", "direct.svg"})
        self.assertEqual(next(i for i in items if i["rel"].startswith("work/loop-memory"))["origins"], ["ui-vision"])
        self.assertTrue(all(i["isNew"] for i in items))

    def test_outside_references_and_old_run_outputs_are_ignored(self):
        outside = self.file("a.svg", root=self.fx.outsider)
        p = self.file("docs/run.svg", t=-watch.WATCH_WINDOW)
        self.session([assistant(10, uses=[("read", "Read", {"file_path": str(outside)})])])
        run = {"started": iso(0), "ended": iso(119 - watch.WATCH_WINDOW), "checkout": str(self.fx.main), "outputs": ["docs/run.svg"]}
        self.assertEqual(self.scan(self.discover(), [run], [{"session_id": SID, "attachments": [{"path": str(outside)}]}]), [])
        run["ended"] = iso(120 - watch.WATCH_WINDOW)
        (found,) = self.scan(runs=[run])
        self.assertEqual((found["path"], found["isNew"]), (str(p), False))

    def test_depth_entry_size_count_and_preview_limits(self):
        self.file("work/gallery/root.svg", t=100)
        self.file("work/gallery/one/within.svg", t=110)
        self.file("work/gallery/one/two/too-deep.svg", t=115)
        with patch.object(home, "WALK_DEPTH", 1):
            self.assertEqual({Path(i["path"]).name for i in self.scan()}, {"root.svg", "within.svg"})
        with patch.object(home, "WALK_ENTRIES", 1):
            self.assertLessEqual(len(self.scan()), 1)
        self.file("work/gallery/large.svg", b"x" * 100, t=116)
        with patch.object(home, "FILE_MAX", 10):
            self.assertNotIn("large.svg", [Path(i["path"]).name for i in self.scan()])
        with patch.object(home, "GALLERY_ITEMS", 2):
            self.assertEqual([Path(i["path"]).name for i in self.scan()], ["large.svg", "too-deep.svg"])
        with patch.object(home, "PREVIEW_MAX", 10), patch.object(home, "PREVIEW_TOTAL", 6):
            items = self.scan()
        self.assertEqual([Path(i["path"]).name for i in items if i["preview"]], ["too-deep.svg"])

    def test_entry_budget_is_shared_across_starts_and_docs_walk(self):
        for root in (self.fx.main, self.fx.wt, self.fx.ext):
            self.file("work/gallery/a.svg", root=root)
            self.file("work/loop-memory/ui-vision/b.svg", root=root)
        self.file("docs/figures/c.svg")
        with patch.object(home, "WALK_ENTRIES", 2):
            self.assertEqual(len(self.scan()), 2)

    def test_entry_budget_stops_consuming_directory_entries_at_the_limit(self):
        for i in range(4):
            self.file(f"work/gallery/{i}.svg")
        real_scandir, consumed = os.scandir, []

        class CountedEntries:
            def __init__(self, directory):
                self.entries = real_scandir(directory)

            def __enter__(self):
                return self

            def __exit__(self, *args):
                self.entries.close()

            def __iter__(self):
                return self

            def __next__(self):
                entry = next(self.entries)
                consumed.append(entry.name)
                return entry

        with patch.object(home, "WALK_ENTRIES", 2), patch("os.scandir", CountedEntries):
            items = self.scan(roots=[self.fx.main])
        self.assertEqual((len(items), len(consumed)), (2, 2))

    def test_head_budget_for_marp_and_marimo(self):
        self.file("work/gallery/late.md", b"---\n" + b" " * 40 + b"\nmarp: true\n---\n")
        self.file("work/gallery/late.py", b" " * 40 + b"marimo.App()")
        with patch.object(home, "HEAD_BYTES", 32):
            self.assertEqual(self.scan(), [])

    def test_symlinked_directory_not_followed_even_with_an_inside_target(self):
        self.file("docs/hidden/a.svg")
        g = self.fx.main / "work" / "gallery"
        g.mkdir(parents=True)
        self.symlink(self.fx.main / "docs" / "hidden", g / "linked", True)
        self.assertEqual(self.scan(), [])

    def test_malformed_sources_and_oserrors_do_not_abort_scan(self):
        p = self.file("work/gallery/good.svg")
        items = self.scan(sessions=[None], runs=[None, {"ended": "invalid"}], asks=[None, {"attachments": [None]}])
        self.assertEqual([i["path"] for i in items], [str(p)])
        with patch("os.scandir", side_effect=OSError("unreadable")):
            self.assertEqual(self.scan(), [])

    def test_cache_returns_same_list_and_force_refreshes(self):
        p = self.file("work/gallery/a.svg")
        scanner = home.GalleryScanner(interval=10)
        first = self.scan(scanner=scanner)
        self.file("work/gallery/b.svg")
        self.assertIs(self.scan(scanner=scanner, seconds=129), first)
        forced = self.scan(scanner=scanner, seconds=129, force=True)
        self.assertEqual(len(forced), 2)
        self.assertIsNot(forced, first)
        p.unlink()
        expired = self.scan(scanner=scanner, seconds=139)
        self.assertEqual([Path(i["path"]).name for i in expired], ["b.svg"])


class ProgressDerivationTests(DerivationFixtureTests):
    def test_document_order_weights_and_current_step_in_each_track(self):
        doc = sample_status()
        view = home.progress_view(doc, [])
        self.assertEqual((view["schema"], view["updated"], view["noChecklist"]), (2, "2026-09-30", False))
        release, stage2 = view["tracks"]
        self.assertEqual((release["title"], stage2["title"]), ("0.4.1", "Stage 2"))
        self.assertEqual(release["steps"], [
            {"id": "0.4.1-1", "title": "Step 0.4.1-1", "status": "in_progress", "percent": 25.0, "done": 1, "total": 4},
            {"id": "0.4.1-2", "title": "Step 0.4.1-2", "status": "not_started", "percent": 0.0, "done": 0, "total": 4}])
        self.assertAlmostEqual(release["percent"], 25 / 3)
        self.assertEqual(release["current"], {"id": "0.4.1-2", "title": "Step 0.4.1-2", "percent": 0.0,
                                              "remaining": [{"id": "harness", "title": "HARNESS", "weight": 3},
                                                            {"id": "runs", "title": "RUNS", "weight": 1}]})
        self.assertEqual(stage2["current"]["id"], "2.0")
        doc["current"] = "2.0"
        self.assertEqual([t["current"]["id"] for t in home.progress_view(doc, [])["tracks"]], ["0.4.1-1", "2.0"])
        for s in doc["steps"][:2]:
            s["status"] = "done"
            for i in s["items"]:
                i.update(done=True, evidence="0123abc")
        release = home.progress_view(doc, [])["tracks"][0]
        self.assertEqual((release["current"]["id"], release["percent"], release["current"]["remaining"]),
                         ("0.4.1-2", 100.0, []))

    def test_done_without_evidence_and_all_validation_failures_suppress_percentages(self):
        doc = sample_status()
        doc["steps"][0]["items"][1]["done"] = True
        view = home.progress_view(doc, [])
        self.assertTrue(view["noChecklist"])
        for track in view["tracks"]:
            self.assertIsNone(track["percent"])
            self.assertIsNone(track["current"]["percent"])
            for s in track["steps"]:
                self.assertEqual((s["percent"], s["done"], s["total"]), (None, 0, 0))
        doc = sample_status()
        doc["current"] = "missing"
        self.assertTrue(home.progress_view(doc, [])["noChecklist"])

    def test_schema_one_none_and_malformed_documents_keep_both_tracks(self):
        doc = sample_status()
        doc["schema"] = 1
        for s in doc["steps"]:
            del s["items"]
        view = home.progress_view(doc, [])
        self.assertEqual([len(t["steps"]) for t in view["tracks"]], [2, 1])
        self.assertTrue(view["noChecklist"])
        self.assertEqual(view["tracks"][0]["steps"][0]["total"], 0)
        for malformed in (None, [], "bad", {"steps": None}, {"steps": [None, {}]}, {"schema": 2, "steps": "bad"}):
            view = home.progress_view(malformed, [])
            self.assertEqual([t["steps"] for t in view["tracks"]], [[], []])
            self.assertTrue(view["noChecklist"])
            self.assertTrue(all(t["current"] is None for t in view["tracks"]))

    def test_out_of_range_weights_and_internal_errors_give_the_no_checklist_view(self):
        for weight in (10 ** 400, 1001, 0):
            doc = sample_status()
            doc["steps"][0]["weight"] = weight
            with self.subTest(weight=weight):
                view = home.progress_view(doc, [])
                self.assertTrue(view["noChecklist"])
                self.assertTrue(all(t["percent"] is None for t in view["tracks"]))
        with patch.object(checklist, "track_percent", side_effect=OverflowError("x")):
            view = home.progress_view(sample_status(), [])
        self.assertTrue(view["noChecklist"])
        self.assertEqual([t["steps"] for t in view["tracks"]], [[], []])

    def test_task_progress_only_for_sessions_with_tasks(self):
        self.session([assistant(10, uses=[("todo", "TodoWrite", {"todos": [
            {"content": "read", "status": "completed"}, {"content": "test", "status": "in_progress"},
            {"content": "ship", "status": "pending"}]})])])
        self.session(sid="empty", checkout=self.fx.ext)
        sessions = self.discover()
        sessions[0].title = ""
        view = home.progress_view(None, sessions)
        self.assertEqual(view["sessions"], [{"sid": SID, "title": SID[:8], "done": 1, "total": 3}])


if __name__ == "__main__":
    unittest.main(verbosity=2)
