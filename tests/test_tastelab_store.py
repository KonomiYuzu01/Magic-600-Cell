"""Taste Lab store, seed plan and input guards (standard library; image and seed-file tests need the tastelab environment).

Every test uses a fresh temporary data folder; no real Taste Lab data, 0.4
session or network is touched. Without Pillow, numpy or PyYAML (the tastelab
environment) the tests that need them are skipped.
"""
from __future__ import annotations

import io
import sqlite3
import sys
import tempfile
import shutil
import subprocess
import threading
from uuid import uuid4
import unittest
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parents[1]
sys.dont_write_bytecode = True
sys.path.insert(0, str(ROOT / "tools"))

from tastelab import common, seeds, store  # noqa: E402

try:
    import numpy  # noqa: F401
    from PIL import Image
    HAVE_IMAGING = True
except ImportError:
    HAVE_IMAGING = False

try:
    import yaml  # noqa: F401
    HAVE_YAML = True
except ImportError:
    HAVE_YAML = False

SHA_A = "a" * 64
SHA_B = "b" * 64
SHA_C = "c" * 64


def meta(sha: str, **over) -> store.ImageMeta:
    fields = dict(sha256=sha, source="wikimedia", licence="CC0-1.0", attribution="Jane Doe", tier="A",
                  category="steampunk", width=800, height=600, source_id="File:x.jpg",
                  page_url="https://commons.wikimedia.org/wiki/File:x.jpg", query="orrery")
    fields.update(over)
    return store.ImageMeta(**fields)


class TempDir(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.gettempdir()) / f"tastelab-test-{uuid4().hex}"
        self.tmp.mkdir()
        self.base_patch = mock.patch.object(common, "PRIVATE_BASE", self.tmp)
        self.base_patch.start()

    def tearDown(self):
        self.doCleanups()
        self.base_patch.stop()
        shutil.rmtree(self.tmp, ignore_errors=True)


class StoreTests(TempDir):
    def setUp(self):
        super().setUp()
        self.store = store.Store(self.tmp / "data", max_images=3, max_bytes=10_000)

    def tearDown(self):
        self.store.close()
        super().tearDown()

    def test_layout_and_schema(self):
        for sub in ("thumbs", "originals", "reports"):
            self.assertTrue((self.tmp / "data" / sub).is_dir())
        self.assertEqual(self.store.db.execute("PRAGMA user_version").fetchone()[0], store.SCHEMA_VERSION)
        self.assertEqual(self.store.db.execute("PRAGMA journal_mode").fetchone()[0], "wal")

    def test_newer_schema_is_refused(self):
        self.store.db.execute(f"PRAGMA user_version={store.SCHEMA_VERSION + 1}")
        self.store.db.commit()
        with self.assertRaises(common.Refused):
            store.Store(self.tmp / "data")

    def test_add_image_publishes_files_after_row_commit_and_dedupes(self):
        self.assertEqual(self.store.add_image(meta(SHA_A), b"thumb", b"orig", "jpg"), "stored")
        self.assertEqual(self.store.thumb_path(SHA_A).read_bytes(), b"thumb")
        self.assertEqual(self.store.original_path(SHA_A).read_bytes(), b"orig")
        self.assertEqual(self.store.display_path(SHA_A), self.store.original_path(SHA_A))
        got = self.store.image(SHA_A)
        self.assertEqual((got.licence, got.attribution, got.tier, got.bytes), ("CC0-1.0", "Jane Doe", "A", 9))
        self.assertTrue(got.added.endswith("Z"))
        self.assertEqual(self.store.add_image(meta(SHA_A), b"other"), "duplicate")
        self.assertEqual(self.store.thumb_path(SHA_A).read_bytes(), b"thumb")

    def test_required_fields_and_names(self):
        for bad in (dict(tier="C"), dict(licence=""), dict(attribution="  "), dict(source=""), dict(category="")):
            with self.subTest(bad=bad), self.assertRaises(ValueError):
                self.store.add_image(meta(SHA_A, **bad), b"t")
        with self.assertRaises(ValueError):
            self.store.add_image(meta("../" + "a" * 61), b"t")
        with self.assertRaises(ValueError):
            self.store.thumb_path("A" * 64)
        self.assertEqual(self.store.count_images(), 0)

    def test_original_kept_only_under_the_caps(self):
        self.assertEqual(self.store.add_image(meta(SHA_A), b"t", b"x" * 8_000, "png"), "stored")
        self.assertIsNone(self.store.original_path(SHA_A))          # 8,001 bytes would pass 80 % of 10,000
        self.assertEqual(self.store.add_image(meta(SHA_B), b"t", b"x" * 100, "exe"), "stored")
        self.assertIsNone(self.store.original_path(SHA_B))          # unknown extension
        with mock.patch.object(common, "ORIGINAL_MAX_BYTES", 50):
            self.assertEqual(self.store.add_image(meta(SHA_C), b"t", b"x" * 100, "jpg"), "stored")
        self.assertIsNone(self.store.original_path(SHA_C))
        self.assertEqual(self.store.bytes_used(), 3)

    def test_caps_stop_storing(self):
        for sha in (SHA_A, SHA_B, SHA_C):
            self.assertEqual(self.store.add_image(meta(sha), b"t"), "stored")
        self.assertEqual(self.store.add_image(meta("d" * 64), b"t"), "full")
        other = store.Store(self.tmp / "other", max_bytes=10)
        try:
            self.assertEqual(other.add_image(meta(SHA_A), b"x" * 11), "full")
            self.assertFalse(other.thumb_path(SHA_A).exists())
        finally:
            other.close()

    def test_failed_insert_leaves_no_files(self):
        self.store.db.execute("CREATE TRIGGER no_insert BEFORE INSERT ON images BEGIN SELECT RAISE(ABORT, 'x'); END")
        with self.assertRaises(sqlite3.DatabaseError):
            self.store.add_image(meta(SHA_A), b"t", b"o", "jpg")
        self.assertFalse(self.store.thumb_path(SHA_A).exists())
        self.assertFalse((self.tmp / "data" / "originals" / "aa" / f"{SHA_A}.jpg").exists())

    def test_remove_blocks_for_good(self):
        self.store.add_image(meta(SHA_A), b"t", b"o", "jpg")
        self.store.put_embeddings("m", 2, [(SHA_A, b"\0" * 8)])
        self.store.add_rating(SHA_A, "like", "s1")
        self.assertTrue(self.store.remove_image(SHA_A))
        self.assertFalse(self.store.thumb_path(SHA_A).exists())
        self.assertFalse(self.store.has_image(SHA_A))
        self.assertEqual(self.store.ratings(), [])
        self.assertEqual(self.store.embeddings("m"), {})
        self.assertEqual(self.store.add_image(meta(SHA_A), b"t"), "blocked")
        self.assertFalse(self.store.remove_image(SHA_B))
        self.assertTrue(self.store.is_blocked(SHA_B))

    def test_ratings_undo_and_effective_verdicts(self):
        for sha in (SHA_A, SHA_B, SHA_C):
            self.store.add_image(meta(sha), b"t")
        self.store.add_rating(SHA_A, "like", "s1", note="  warm \n brass\tglow  ")
        self.store.add_rating(SHA_B, "dislike", "s1")
        self.store.add_rating(SHA_C, "skip", "s2")
        self.assertEqual(self.store.undo_last("s1"), SHA_B)
        self.assertEqual(self.store.undo_last("s3"), None)
        self.store.add_rating(SHA_B, "like", "s1")
        self.store.add_rating(SHA_A, "dislike", "s1")
        effective = {r.sha256: r.verdict for r in self.store.ratings()}
        self.assertEqual(effective, {SHA_A: "dislike", SHA_B: "like", SHA_C: "skip"})
        self.assertEqual([r.sha256 for r in self.store.ratings()], [SHA_C, SHA_B, SHA_A])
        notes = [r[0] for r in self.store.db.execute("SELECT note FROM ratings ORDER BY id")]
        self.assertEqual(notes[0], "warm brass glow")
        counts = self.store.counts()
        self.assertEqual((counts["like"], counts["dislike"], counts["skip"], counts["total"], counts["today"]), (1, 1, 1, 3, 4))
        self.assertEqual(self.store.counts(day="1999-01-01")["today"], 0)
        self.assertEqual(self.store.rated(), {SHA_A, SHA_B, SHA_C})
        with self.assertRaises(ValueError):
            self.store.add_rating(SHA_A, "love", "s1")
        with self.assertRaises(sqlite3.IntegrityError):
            self.store.add_rating("e" * 64, "like", "s1")

    def test_notes_are_one_bounded_line(self):
        self.store.add_image(meta(SHA_A), b"t")
        self.store.add_rating(SHA_A, "like", "s", note="x" * 1000)
        self.store.add_rating(SHA_A, "like", "s", note="   ")
        notes = [r[0] for r in self.store.db.execute("SELECT note FROM ratings ORDER BY id")]
        self.assertEqual(len(notes[0]), common.NOTE_MAX)
        self.assertIsNone(notes[1])

    def test_embeddings_and_text_embeddings(self):
        self.store.add_image(meta(SHA_A), b"t")
        self.store.add_image(meta(SHA_B), b"t")
        self.store.put_embeddings("m", 2, [(SHA_A, b"\1" * 8)])
        self.assertEqual(self.store.missing_embeddings("m"), [SHA_B])
        self.assertEqual(self.store.missing_embeddings("other"), [SHA_A, SHA_B])
        self.assertEqual(self.store.embeddings("m", [SHA_A, SHA_B]), {SHA_A: b"\1" * 8})
        self.assertEqual(self.store.embedded("m"), {SHA_A})
        self.assertEqual(self.store.embedded("other"), set())
        with self.assertRaises(ValueError):
            self.store.put_embeddings("m", 2, [(SHA_B, b"\0" * 7)])
        self.store.put_text_embeddings("m", 2, [("brass", b"\2" * 8)])
        self.assertEqual(self.store.text_embeddings("m", ["brass", "chrome"]), {"brass": b"\2" * 8})

    def test_model_runs_cursors_proposals_and_screen_counts(self):
        self.store.add_model_run("m", 20, 10, 10, 0.71, None, False, {"C": 1.0})
        self.store.add_model_run("m", 30, 15, 15, 0.74, 0.03, False)
        runs = self.store.model_runs()
        self.assertEqual([r["ratings"] for r in runs], [20, 30])
        self.assertEqual(runs[0]["params"], {"C": 1.0})
        self.assertEqual(self.store.model_runs(limit=1)[0]["ratings"], 30)
        self.assertEqual(self.store.cursor("met", "orrery"), (None, False))
        self.store.set_cursor("met", "orrery", "40", done=True)
        self.assertEqual(self.store.cursor("met", "orrery"), ("40", True))
        self.assertEqual(self.store.add_proposals([("Nixie  tube", "early_tech", "adjacent"), ("Nixie tube", None, "probe")], 100), 1)
        self.assertEqual(self.store.last_proposal_rating(), 100)
        self.store.mark_proposal_round(200)     # a round that found no new term still counts
        self.assertEqual(self.store.last_proposal_rating(), 200)
        self.store.set_proposal_status("Nixie tube", "accepted")
        self.assertEqual([p["term"] for p in self.store.proposals("accepted")], ["Nixie tube"])
        with self.assertRaises(ValueError):
            self.store.set_proposal_status("Nixie tube", "maybe")
        self.store.add_screen_counts("safebooru", 10, 2)
        self.store.add_screen_counts("safebooru", 5, 1)
        self.assertEqual(self.store.screen_counts(), {"safebooru": (15, 3)})
        self.store.mark_seen("met", "123")
        self.assertTrue(self.store.has_seen("met", "123"))
        self.assertFalse(self.store.has_seen("aic", "123"))

    def test_category_and_query_counts(self):
        self.store.add_image(meta(SHA_A), b"t")
        self.store.add_image(meta(SHA_B, category="gaming", query="pixel art"), b"t")
        self.assertEqual(self.store.category_counts(), {"steampunk": 1, "gaming": 1})
        self.assertEqual(self.store.query_counts(), {("steampunk", "orrery"): 1, ("gaming", "pixel art"): 1})
        self.assertEqual([m.sha256 for m in self.store.images(category="gaming")], [SHA_B])
        self.assertEqual(self.store.images(tier="B"), [])


class GuardTests(TempDir):
    def test_default_is_checkout_local_and_uses_same_guard_as_override(self):
        self.assertEqual(common.default_data_root(), self.tmp / "library")
        self.assertEqual(common.data_root(), common.data_root(self.tmp / "library"))
        for marker in common.SESSION_MARKERS:
            parent = self.tmp / ("case-" + marker)
            parent.mkdir()
            (parent / marker).write_bytes(b"synthetic session")
            with mock.patch.object(common, "PRIVATE_BASE", parent):
                before = set(parent.iterdir())
                for override in (None, parent / "library"):
                    with self.assertRaisesRegex(common.Refused, "session"):
                        store.Store(common.data_root(override))
                self.assertEqual(set(parent.iterdir()), before)

    def test_default_junction_to_session_refused_before_mkdir_or_database(self):
        target = self.tmp / "session-target"
        target.mkdir()
        (target / "session.sqlite3").write_bytes(b"synthetic session")
        default = common.default_data_root()
        # A deterministic junction probe avoids creating links in this sandbox.
        with mock.patch.object(Path, "is_junction", autospec=True, side_effect=lambda p: p == default), \
                mock.patch.object(Path, "mkdir") as mkdir, mock.patch.object(sqlite3, "connect") as connect:
            for override in (None, default):
                with self.assertRaisesRegex(common.Refused, "junction"):
                    store.Store(common.data_root(override))
        mkdir.assert_not_called()
        connect.assert_not_called()
        self.assertFalse(default.exists())

    def test_links_anywhere_from_private_base_and_store_children_refused(self):
        default = common.default_data_root()
        for component in (self.tmp, default, default / "thumbs", default / store.DB_NAME):
            with self.subTest(component=component.name), mock.patch.object(
                    Path, "is_symlink", autospec=True, side_effect=lambda p: p == component), \
                    mock.patch.object(Path, "mkdir") as mkdir, mock.patch.object(sqlite3, "connect") as connect:
                with self.assertRaises(common.Refused):
                    store.Store(default)
                mkdir.assert_not_called()
                connect.assert_not_called()

    def test_overrides_outside_private_tree_refused_without_creating_anything(self):
        checkout = self.tmp / "checkout"
        base = checkout / "work" / "loop-memory" / "tastelab"
        with mock.patch.object(common, "PRIVATE_BASE", base):
            for path in (checkout / "work" / "experiments" / "public-export", checkout / "work" / "other",
                         self.tmp / "outside-checkout", checkout / "tools" / "data", base / ".." / "ledgers"):
                with self.subTest(path=path.name), self.assertRaises(common.Refused):
                    store.Store(path)
                self.assertFalse(path.exists())
            self.assertFalse(checkout.exists())

    def test_sha_full_string_guard(self):
        self.assertTrue(common.valid_sha(SHA_A))
        for value in (SHA_A + "\n", SHA_A + " ", " " + SHA_A, SHA_A.upper(), SHA_A[:-1], SHA_A + "a", None, True):
            with self.subTest(value=value):
                self.assertFalse(common.valid_sha(value))


class RecoveryTests(TempDir):
    def test_failed_unlink_retried_on_reopen(self):
        root = self.tmp / "library"
        with store.Store(root) as library:
            library.add_image(meta(SHA_A), b"thumb", b"original", "png")
            path = library.thumb_path(SHA_A)
            unlink = Path.unlink

            def denied(p, *args, **kw):
                if p == path:
                    raise PermissionError("injected unlink failure")
                return unlink(p, *args, **kw)

            with mock.patch.object(Path, "unlink", denied):
                self.assertTrue(library.remove_image(SHA_A))
            self.assertTrue(path.exists())
            self.assertFalse(library.has_image(SHA_A))
            self.assertEqual(library.db.execute("SELECT COUNT(*) FROM file_ops").fetchone()[0], 1)
        with store.Store(root) as reopened:
            self.assertFalse(path.exists())
            self.assertEqual(reopened.db.execute("SELECT COUNT(*) FROM file_ops").fetchone()[0], 0)
            self.assertTrue(reopened.is_blocked(SHA_A))

    def test_termination_after_temporary_write_before_insert_recovers(self):
        root = self.tmp / "library"
        script = """
import os, sys
from pathlib import Path
sys.path.insert(0, sys.argv[1])
from tastelab import common, store
common.PRIVATE_BASE = Path(sys.argv[2])
library = store.Store(Path(sys.argv[3]))
write = library._write
def terminated(path, data):
    write(path, data)
    os._exit(73)
library._write = terminated
meta = store.ImageMeta('a' * 64, 'met', 'CC0-1.0', 'Invented credit', 'A', 'test', 100, 100)
library.add_image(meta, b'thumb', b'original', 'png')
"""
        result = subprocess.run([sys.executable, "-B", "-c", script, str(ROOT / "tools"), str(self.tmp), str(root)],
                                cwd=self.tmp, capture_output=True, text=True)
        self.assertEqual(result.returncode, 73, result.stderr)
        self.assertEqual(len(list(root.rglob(".part-*"))), 1)
        with store.Store(root) as reopened:
            self.assertEqual(reopened.count_images(), 0)
            self.assertEqual(reopened.db.execute("SELECT COUNT(*) FROM file_ops").fetchone()[0], 0)
            self.assertEqual(list(root.rglob(".part-*")), [])
            self.assertEqual(list((root / "thumbs").rglob("*.jpg")), [])

    def test_abandoned_loser_intent_does_not_rollback_or_delete_winner(self):
        root = self.tmp / "library"
        with store.Store(root) as library:
            library.add_image(meta(SHA_A), b"winner")
            target = str(library.thumb_path(SHA_A).relative_to(root))
            import json
            with library.db:
                library.db.execute("INSERT INTO file_ops(id, sha256, op, files) VALUES ('loser', ?, 'add', ?)",
                                   (SHA_A, json.dumps([{"temp": "thumbs/aa/.part-loser.jpg", "target": target},
                                                      {"temp": "originals/aa/.part-loser.png", "target": "originals/aa/" + SHA_A + ".png"}])))
        with store.Store(root) as reopened:
            self.assertTrue(reopened.has_image(SHA_A))
            self.assertEqual(reopened.thumb_path(SHA_A).read_bytes(), b"winner")
            self.assertEqual(reopened.db.execute("SELECT COUNT(*) FROM file_ops").fetchone()[0], 0)

    def test_committed_add_is_finished_on_reopen(self):
        root = self.tmp / "library"
        with store.Store(root) as library:
            with mock.patch.object(library, "_reconcile"):
                library.add_image(meta(SHA_A), b"thumb", b"original", "png")
            self.assertFalse(library.thumb_path(SHA_A).exists())
            self.assertTrue(library.has_image(SHA_A))
            self.assertEqual(len(list(root.rglob(".part-*"))), 2)
        with store.Store(root) as reopened:
            self.assertEqual(reopened.thumb_path(SHA_A).read_bytes(), b"thumb")
            self.assertEqual(reopened.original_path(SHA_A).read_bytes(), b"original")
            self.assertEqual(list(root.rglob(".part-*")), [])

    def test_older_schema_refused_without_migration(self):
        root = self.tmp / "old"
        root.mkdir()
        with sqlite3.connect(root / store.DB_NAME) as old:
            old.execute("PRAGMA user_version=1")
            old.execute("CREATE TABLE preserved(value TEXT)")
        with self.assertRaisesRegex(common.Refused, "schema 1.*no automatic migration"):
            store.Store(root)
        with sqlite3.connect(root / store.DB_NAME) as old:
            self.assertEqual(old.execute("PRAGMA user_version").fetchone()[0], 1)
            self.assertIsNone(old.execute("SELECT name FROM sqlite_master WHERE name='pair_notes'").fetchone())


class RaceTests(TempDir):
    def race(self, name, stage, *, same_hash=False, cap=3, remove_second=False, remove_first=False):
        root = self.tmp / name
        with store.Store(root, max_images=cap):
            pass
        ready = threading.Barrier(3)
        paused, release, attempted, advanced = (threading.Event() for _ in range(4))
        errors, results = [], {}

        def pause():
            if not paused.is_set():
                paused.set()
                if not release.wait(10):
                    errors.append("first writer timed out")

        def worker(first):
            try:
                with store.Store(root, max_images=cap) as library:
                    check = library.image if remove_first and first or remove_second and not first else library.has_image
                    def checked(sha):
                        result = check(sha)
                        if first and stage == "checks":
                            pause()
                        elif not first:
                            advanced.set()
                        return result
                    setattr(library, "image" if remove_first and first or remove_second and not first else "has_image", checked)
                    if first and stage == "write":
                        write = library._write
                        def written(path, data):
                            write(path, data)
                            self.assertFalse(library.thumb_path(SHA_A).exists())
                            pause()
                        library._write = written
                    if first and stage == "insert":
                        library.db.set_trace_callback(lambda sql: pause() if sql.startswith("INSERT INTO images(") else None)
                    ready.wait(10)
                    if not first:
                        if not paused.wait(10):
                            raise AssertionError("first writer did not reach its barrier")
                        attempted.set()
                    if first and remove_first or not first and remove_second:
                        results[first] = library.remove_image(SHA_A)
                    else:
                        sha = SHA_A if first or same_hash else SHA_B
                        results[first] = library.add_image(meta(sha), b"winner" if first else b"loser", b"original", "png")
            except BaseException as exc:
                errors.append(repr(exc))
                ready.abort()

        threads = [threading.Thread(target=worker, args=(first,)) for first in (True, False)]
        for thread in threads:
            thread.start()
        try:
            ready.wait(10)
            self.assertTrue(paused.wait(10))
            self.assertTrue(attempted.wait(10))
            self.assertFalse(advanced.wait(0.15), "second writer entered checks while first held the write lock")
        finally:
            release.set()
            for thread in threads:
                thread.join(15)
        self.assertTrue(all(not thread.is_alive() for thread in threads))
        self.assertEqual(errors, [])
        return root, results

    def test_same_hash_races_at_checks_write_and_insert(self):
        for stage in ("checks", "write", "insert"):
            with self.subTest(stage=stage):
                root, results = self.race("same-" + stage, stage, same_hash=True)
                self.assertEqual(results, {True: "stored", False: "duplicate"})
                with store.Store(root) as library:
                    self.assertEqual(library.count_images(), 1)
                    self.assertEqual(library.thumb_path(SHA_A).read_bytes(), b"winner")
                    self.assertEqual(library.original_path(SHA_A).read_bytes(), b"original")
                self.assertEqual(list(root.rglob(".part-*")), [])

    def test_one_image_cap_races_at_checks_write_and_insert(self):
        for stage in ("checks", "write", "insert"):
            with self.subTest(stage=stage):
                root, results = self.race("cap-" + stage, stage, cap=1)
                self.assertEqual(results, {True: "stored", False: "full"})
                with store.Store(root) as library:
                    self.assertEqual(library.count_images(), 1)
                    self.assertFalse(library.thumb_path(SHA_B).exists())

    def test_concurrent_removal_after_add_blocks_and_removes_files(self):
        for stage in ("checks", "write", "insert"):
            with self.subTest(stage=stage):
                root, results = self.race("remove-" + stage, stage, remove_second=True)
                self.assertEqual(results, {True: "stored", False: True})
                with store.Store(root) as library:
                    self.assertTrue(library.is_blocked(SHA_A))
                    self.assertFalse(library.has_image(SHA_A))
                    self.assertFalse(library.thumb_path(SHA_A).exists())
                    self.assertEqual(library.add_image(meta(SHA_A), b"retry"), "blocked")
                self.assertEqual(list(root.rglob(".part-*")), [])

    def test_concurrent_block_before_add_never_writes(self):
        root, results = self.race("block-first", "checks", same_hash=True, remove_first=True)
        self.assertEqual(results, {True: False, False: "blocked"})
        with store.Store(root) as library:
            self.assertEqual(library.count_images(), 0)
            self.assertFalse(library.thumb_path(SHA_A).exists())
            self.assertTrue(library.is_blocked(SHA_A))


class SeedTests(TempDir):
    def plan(self, **over):
        data = {"version": 1, "relevance_min": 0.2, "sources": {"met": True, "safebooru": False},
                "categories": {"steampunk": {"kind": "focus", "target": 10, "sources": ["met", "safebooru"],
                                             "terms": ["orrery", {"text": "brass", "met": "brass instrument"}],
                                             "adjacent": ["astrolabe"]},
                               "anime": {"kind": "focus", "target": 5, "tier": "B", "sources": ["met"], "terms": ["x"]}}}
        data.update(over)
        return data

    def test_parse_and_helpers(self):
        plan = seeds.parse(self.plan())
        cat = plan.categories["steampunk"]
        self.assertEqual(cat.terms[1].query("met"), "brass instrument")
        self.assertEqual(cat.terms[1].query("aic"), "brass")
        self.assertEqual(plan.active_sources("steampunk"), ["met"])
        self.assertEqual(plan.focus(), ["steampunk", "anime"])
        self.assertEqual(plan.tier_for("steampunk", "met"), "A")
        self.assertEqual(plan.tier_for("steampunk", "safebooru"), "B")
        self.assertEqual(plan.tier_for("anime", "met"), "B")

    def test_owner_cannot_add_sources_or_force_tier_a(self):
        bad = [
            self.plan(sources={"pinterest": True}),
            self.plan(version=2),
            self.plan(relevance_min=2),
            self.plan(categories={"x": {"kind": "focus", "target": 1, "sources": ["arena"], "terms": ["a"]}}),
            self.plan(categories={"x": {"kind": "focus", "target": 1, "tier": "A", "terms": ["a"]}}),
            self.plan(categories={"x": {"kind": "mood", "target": 1, "terms": ["a"]}}),
            self.plan(categories={"x": {"kind": "focus", "target": 1, "terms": []}}),
            self.plan(categories={"X y": {"kind": "focus", "target": 1, "terms": ["a"]}}),
            self.plan(categories={"x": {"kind": "focus", "target": 1, "terms": [{"text": "a", "flickr": "b"}]}}),
            self.plan(categories={"x": {"kind": "focus", "target": common.MAX_IMAGES + 1, "terms": ["a"]}}),
            self.plan(categories={"x": {"kind": "focus", "target": 1, "terms": ["a\x07"]}}),
        ]
        for data in bad:
            with self.subTest(data=data), self.assertRaises(seeds.SeedError):
                seeds.parse(data)

    def test_malformed_top_level_shapes_name_the_field(self):
        for field in ("sources", "categories"):
            for value in (["wikimedia"], "wikimedia", True, None):
                with self.subTest(field=field, value=value), self.assertRaisesRegex(seeds.SeedError, field):
                    seeds.parse(self.plan(**{field: value}))
        for value in ({}, [], "met", None):
            with self.subTest(value=value), self.assertRaisesRegex(seeds.SeedError, "sources.met"):
                seeds.parse(self.plan(sources={"met": value}))

    def test_malformed_category_sources_terms_and_entries_name_the_field(self):
        for field in ("sources", "terms"):
            for value in ({}, "met", True, None):
                data = self.plan()
                data["categories"]["steampunk"][field] = value
                with self.subTest(field=field, value=value), self.assertRaisesRegex(seeds.SeedError, field):
                    seeds.parse(data)
        for value in ({}, [], "invalid-source", True, None):
            data = self.plan()
            data["categories"]["steampunk"]["sources"] = [value]
            with self.subTest(value=value), self.assertRaisesRegex(seeds.SeedError, r"sources\[0\]"):
                seeds.parse(data)
        for value in ({}, [], "", True, None, {"text": None}, {"text": "a", "met": []}):
            data = self.plan()
            data["categories"]["steampunk"]["terms"] = [value]
            with self.subTest(value=value), self.assertRaisesRegex(seeds.SeedError, r"terms\[0\]"):
                seeds.parse(data)
        for value in ([], "a", True, None):
            with self.subTest(value=value), self.assertRaisesRegex(seeds.SeedError, "categories.x"):
                seeds.parse(self.plan(categories={"x": value}))

    def test_probes(self):
        probes = seeds.parse_probes({"version": 1, "template": "an image of {}", "axes": [["a", "b"]], "terms": ["c", "a"]})
        self.assertEqual(probes.phrase("x"), "an image of x")
        self.assertEqual(probes.texts(), ["a", "b", "c"])
        for bad in ({"version": 1, "template": "no slot"}, {"version": 1, "axes": [["a"]]}, {"version": 2},
                    {"version": 1, "terms": "a"}):
            with self.subTest(bad=bad), self.assertRaises(seeds.SeedError):
                seeds.parse_probes(bad)

    @unittest.skipUnless(HAVE_YAML, "needs the tastelab environment (PyYAML)")
    def test_default_probes_are_valid(self):
        probes = seeds.load_probes(seeds.PROBES_DEFAULT)
        self.assertGreaterEqual(len(probes.axes), 6)
        self.assertIn("{}", probes.template)

    @unittest.skipUnless(HAVE_YAML, "needs the tastelab environment (PyYAML)")
    def test_default_plan_is_valid_and_copied_once(self):
        plan = seeds.load(seeds.DEFAULT)
        self.assertEqual(sum(c.target for c in plan.categories.values()), 4000)
        self.assertEqual(len(plan.focus()), 8)
        self.assertEqual(set(plan.enabled), set(seeds.SOURCE_TIER))
        self.assertEqual(plan.categories["bishoujo_2000s"].tier, "B")
        path = seeds.ensure(self.tmp / "seed-library")
        path.write_text(path.read_text(encoding="utf-8") + "\n# owner edit\n", encoding="utf-8")
        self.assertEqual(seeds.ensure(self.tmp / "seed-library"), path)
        self.assertIn("# owner edit", path.read_text(encoding="utf-8"))


@unittest.skipUnless(HAVE_IMAGING, "needs the tastelab environment (Pillow, numpy)")
class ImageTests(unittest.TestCase):
    def encode(self, image, fmt, **kw):
        out = io.BytesIO()
        image.save(out, format=fmt, **kw)
        return out.getvalue()

    def test_static_formats_decode_to_rgb(self):
        from tastelab import images
        for fmt, ext in (("JPEG", "jpg"), ("PNG", "png"), ("WEBP", "webp"), ("GIF", "gif")):
            with self.subTest(fmt=fmt):
                got = images.decode(self.encode(Image.new("RGB", (300, 200), (10, 120, 200)), fmt))
                self.assertEqual((got.width, got.height, got.ext, got.animated, got.image.mode), (300, 200, ext, False, "RGB"))

    def test_animations_keep_only_the_first_frame(self):
        from tastelab import images
        frames = [Image.new("RGB", (128, 128), (0, 0, 255)), Image.new("RGB", (128, 128), (255, 0, 0))]
        for fmt in ("GIF", "PNG", "WEBP"):
            with self.subTest(fmt=fmt):
                data = self.encode(frames[0], fmt, save_all=True, append_images=frames[1:], duration=100, loop=0)
                got = images.decode(data)
                self.assertTrue(got.animated)
                r, g, b = got.image.getpixel((64, 64))
                self.assertGreater(b, 200)
                self.assertLess(r, 60)

    def test_bad_inputs_are_refused(self):
        from tastelab import images
        cases = [b"", b"not an image", self.encode(Image.new("RGB", (32, 32)), "PNG"),
                 self.encode(Image.new("RGB", (100, 100)), "BMP"), self.encode(Image.new("RGB", (100, 100)), "TIFF")]
        for data in cases:
            with self.subTest(n=len(data)), self.assertRaises(images.BadImage):
                images.decode(data)
        with mock.patch.object(images, "MAX_PIXELS", 100 * 99):
            with self.assertRaises(images.BadImage):
                images.decode(self.encode(Image.new("RGB", (100, 100)), "PNG"))

    def test_transparency_is_flattened_on_white_and_orientation_applied(self):
        from tastelab import images
        clear = Image.new("RGBA", (100, 100), (0, 0, 0, 0))
        self.assertEqual(images.decode(self.encode(clear, "PNG")).image.getpixel((50, 50)), (255, 255, 255))
        tall = Image.new("RGB", (100, 200), (0, 128, 0))
        exif = tall.getexif()
        exif[0x0112] = 6        # rotate 90 degrees on display
        got = images.decode(self.encode(tall, "JPEG", exif=exif.tobytes()))
        self.assertEqual((got.width, got.height), (200, 100))

    def test_thumbnail_is_bounded_jpeg_without_metadata(self):
        from tastelab import images
        image = Image.new("RGB", (2000, 1000), (200, 100, 50))
        data = images.thumbnail_jpeg(image)
        thumb = Image.open(io.BytesIO(data))
        self.assertEqual((thumb.format, thumb.size), ("JPEG", (512, 256)))
        self.assertEqual(len(thumb.getexif()), 0)

    def test_fake_embedder_is_deterministic_and_pinnable(self):
        from tastelab import embed
        fake = embed.FakeEmbedder(texts={"probe": [1.0] + [0.0] * 15})
        red, red2, blue = (Image.new("RGB", (64, 64), c) for c in ((250, 0, 0), (240, 10, 10), (0, 0, 250)))
        v = fake.embed_images([red, red2, blue])
        self.assertEqual(v.shape, (3, 16))
        self.assertTrue(numpy.allclose(numpy.linalg.norm(v, axis=1), 1.0))
        self.assertGreater(float(v[0] @ v[1]), float(v[0] @ v[2]))
        self.assertTrue(numpy.array_equal(v, embed.FakeEmbedder().embed_images([red, red2, blue])))
        t = fake.embed_texts(["probe", "other", "other"])
        self.assertEqual(float(t[0][0]), 1.0)
        self.assertTrue(numpy.array_equal(t[1], t[2]))
        grey = fake.embed_images([Image.new("RGB", (64, 64), (128, 128, 128))])
        self.assertTrue(numpy.isfinite(grey).all())
        blob = embed.to_blob(v[0])
        self.assertTrue(numpy.array_equal(embed.from_blobs([blob, blob], 16)[1], v[0]))


if __name__ == "__main__":
    unittest.main()
