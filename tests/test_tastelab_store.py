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
from dataclasses import asdict
from datetime import datetime, timezone
from contextlib import closing, redirect_stderr, redirect_stdout

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

# Frozen schema-2 DDL from the packet's base; never derive it from the current schema.
SCHEMA_2 = """
CREATE TABLE IF NOT EXISTS images (
  sha256 TEXT PRIMARY KEY,
  source TEXT NOT NULL,
  source_id TEXT,
  page_url TEXT,
  image_url TEXT,
  licence TEXT NOT NULL,
  licence_url TEXT,
  attribution TEXT NOT NULL,
  title TEXT,
  tier TEXT NOT NULL CHECK (tier IN ('A', 'B')),
  category TEXT NOT NULL,
  query TEXT,
  width INTEGER NOT NULL CHECK (width > 0),
  height INTEGER NOT NULL CHECK (height > 0),
  bytes INTEGER NOT NULL CHECK (bytes >= 0),
  original_ext TEXT,
  added TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS images_category ON images(category);
CREATE TABLE IF NOT EXISTS seen (
  source TEXT NOT NULL,
  source_id TEXT NOT NULL,
  PRIMARY KEY (source, source_id)
);
CREATE TABLE IF NOT EXISTS ratings (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  sha256 TEXT NOT NULL REFERENCES images(sha256) ON DELETE CASCADE,
  verdict TEXT NOT NULL CHECK (verdict IN ('like', 'dislike', 'skip')),
  note TEXT,
  ts TEXT NOT NULL,
  session TEXT NOT NULL,
  undone INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS ratings_sha ON ratings(sha256);
CREATE TABLE IF NOT EXISTS embeddings (
  sha256 TEXT NOT NULL REFERENCES images(sha256) ON DELETE CASCADE,
  model TEXT NOT NULL,
  dim INTEGER NOT NULL,
  vector BLOB NOT NULL,
  PRIMARY KEY (sha256, model)
);
CREATE TABLE IF NOT EXISTS text_embeddings (
  text TEXT NOT NULL,
  model TEXT NOT NULL,
  dim INTEGER NOT NULL,
  vector BLOB NOT NULL,
  PRIMARY KEY (text, model)
);
CREATE TABLE IF NOT EXISTS model_runs (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  ts TEXT NOT NULL,
  model TEXT NOT NULL,
  ratings INTEGER NOT NULL,
  likes INTEGER NOT NULL,
  dislikes INTEGER NOT NULL,
  auc REAL,
  auc_delta REAL,
  stable INTEGER NOT NULL DEFAULT 0,
  params TEXT NOT NULL DEFAULT '{}'
);
CREATE TABLE IF NOT EXISTS fetch_state (
  source TEXT NOT NULL,
  query TEXT NOT NULL,
  cursor TEXT,
  done INTEGER NOT NULL DEFAULT 0,
  updated TEXT NOT NULL,
  PRIMARY KEY (source, query)
);
CREATE TABLE IF NOT EXISTS proposals (
  term TEXT PRIMARY KEY,
  category TEXT,
  origin TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('proposed', 'accepted', 'rejected', 'fetched')),
  at_rating INTEGER NOT NULL,
  ts TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS screen_counts (
  source TEXT PRIMARY KEY,
  checked INTEGER NOT NULL DEFAULT 0,
  discarded INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS blocked (
  sha256 TEXT PRIMARY KEY,
  ts TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS meta (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS file_ops (
  id TEXT PRIMARY KEY,
  sha256 TEXT NOT NULL,
  op TEXT NOT NULL CHECK (op IN ('add', 'remove')),
  files TEXT NOT NULL,
  committed INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS pair_notes (
  liked_sha256 TEXT NOT NULL REFERENCES images(sha256) ON DELETE CASCADE,
  disliked_sha256 TEXT NOT NULL REFERENCES images(sha256) ON DELETE CASCADE,
  note TEXT NOT NULL,
  ts TEXT NOT NULL,
  PRIMARY KEY (liked_sha256, disliked_sha256)
);
"""

SHA_A = "a" * 64
SHA_B = "b" * 64
SHA_C = "c" * 64


def meta(sha: str, **over) -> store.ImageMeta:
    fields = dict(sha256=sha, source="wikimedia", licence="CC0-1.0", attribution="Jane Doe", tier="A",
                  category="steampunk", width=800, height=600, source_id="File:x.jpg",
                  page_url="https://commons.wikimedia.org/wiki/File:x.jpg", query="orrery")
    fields.update(over)
    return store.ImageMeta(**fields)


def metadata_images():
    """Synthetic metadata canaries, shared by both thumbnail boundary tests."""
    from PIL import PngImagePlugin
    canary = br"PRIVATE-PATH-CANARY C:\synthetic-private\notes.txt"
    with Image.new("RGB", (96, 96), (80, 130, 170)) as image:
        exif = Image.Exif()
        exif[0x010e] = canary.decode()
        text = PngImagePlugin.PngInfo()
        text.add_text("private", canary.decode())
        text.add_itxt("private-xmp", canary.decode())
        for name, fmt, kwargs in (
                ("comment", "JPEG", {"comment": canary}),
                ("exif", "JPEG", {"exif": exif.tobytes()}),
                ("xmp", "JPEG", {"xmp": canary}),
                ("icc_profile", "JPEG", {"icc_profile": canary}),
                ("png_text", "PNG", {"pnginfo": text})):
            output = io.BytesIO()
            image.save(output, format=fmt, **kwargs)
            yield name, output.getvalue(), canary


class TempDir(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.gettempdir()) / f"tastelab-{uuid4().hex}"
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

    def test_schema_three_love_column_default_and_check(self):
        self.assertEqual(store.SCHEMA_VERSION, 3)
        self.assertEqual(self.store.db.execute("PRAGMA user_version").fetchone()[0], 3)
        column = self.store.db.execute("PRAGMA table_info(ratings)").fetchall()[-1]
        self.assertEqual((column["name"], column["type"], column["notnull"], column["dflt_value"]),
                         ("love", "INTEGER", 1, "0"))
        self.store.add_image(meta(SHA_A), b"t")
        self.store.add_rating(SHA_A, "like", "test")
        self.assertIs(self.store.ratings()[0].love, False)
        for verdict, love in (("like", None), ("like", -1), ("like", 2), ("dislike", 1), ("skip", 1)):
            with self.subTest(verdict=verdict, love=love), self.assertRaises(sqlite3.IntegrityError):
                with self.store.db:
                    self.store.db.execute("INSERT INTO ratings(sha256, verdict, ts, session, love) VALUES (?, ?, ?, ?, ?)",
                                          (SHA_A, verdict, common.now_iso(), "test", love))

    def test_love_validation_before_any_write(self):
        self.store.add_image(meta(SHA_A), b"t")
        for verdict, love in (("like", 1), ("like", 0), ("like", None), ("like", "true"),
                              ("dislike", True), ("skip", True), ("love", True)):
            before = self.store.db.total_changes
            with self.subTest(verdict=verdict, love=love), self.assertRaises(ValueError):
                self.store.add_rating(SHA_A, verdict, "test", love=love)
            self.assertEqual(self.store.db.total_changes, before)

    def test_effective_loves_count_as_likes_and_undo_restores_previous_flag(self):
        for sha in (SHA_A, SHA_B, SHA_C):
            self.store.add_image(meta(sha), b"t")
        self.store.add_rating(SHA_A, "like", "old")
        self.store.add_rating(SHA_A, "like", "love", "warm brass", love=True)
        self.store.add_rating(SHA_B, "like", "plain", love=False)
        self.store.add_rating(SHA_C, "dislike", "plain")
        ratings = self.store.ratings()
        self.assertEqual([r.sha256 for r in ratings], [SHA_A, SHA_B, SHA_C])
        self.assertEqual([r.love for r in ratings], [True, False, False])
        self.assertTrue(all(type(r.love) is bool for r in ratings))
        self.assertEqual((ratings[0].verdict, ratings[0].note), ("like", "warm brass"))
        self.assertEqual(self.store.counts(), dict(like=2, dislike=1, skip=0, love=1, total=3, today=4))
        self.store.add_rating(SHA_A, "skip", "override")
        self.assertEqual(self.store.counts()["love"], 0)
        self.store.undo_last("override")
        self.assertEqual(self.store.counts()["love"], 1)
        self.assertEqual(self.store.undo_last("love"), SHA_A)
        self.assertEqual(self.store.counts(), dict(like=2, dislike=1, skip=0, love=0, total=3, today=3))
        self.assertIs(self.store.ratings()[0].love, False)
        self.assertEqual(self.store.db.execute("SELECT love, undone FROM ratings WHERE session='love'").fetchone()[:], (1, 1))

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


class UpgradeTests(TempDir):
    MOMENT = datetime(2026, 10, 10, 15, 0, 0, tzinfo=timezone.utc)
    BACKUP_NAME = "tastelab-schema2-20261010T150000Z.sqlite3"

    def setUp(self):
        super().setUp()
        clock = mock.patch.object(store, "datetime", create=True)
        self.clock = clock.start()
        self.clock.now.return_value = self.MOMENT
        self.addCleanup(clock.stop)

    def fixture(self, name="library"):
        root = self.tmp / name
        root.mkdir()
        db = sqlite3.connect(root / store.DB_NAME)
        self.addCleanup(db.close)
        db.execute("PRAGMA journal_mode=WAL")
        db.execute("PRAGMA wal_autocheckpoint=0")
        db.execute("PRAGMA foreign_keys=ON")
        db.executescript(SCHEMA_2)
        db.execute("PRAGMA user_version=2")
        for sha in (SHA_A, SHA_B):
            row = asdict(meta(sha, added="2026-10-01T00:00:00Z"))
            db.execute(f"INSERT INTO images({', '.join(row)}) VALUES ({', '.join('?' * len(row))})", list(row.values()))
        db.execute("INSERT INTO ratings VALUES (1, ?, 'like', 'brass glow', ?, 'first', 0)",
                   (SHA_A, "2026-10-01T01:00:00Z"))
        db.commit()
        db.execute("PRAGMA wal_checkpoint(TRUNCATE)")
        # All these records live in the WAL while the source connection remains open.
        row = asdict(meta(SHA_C, tier="B", added="2026-10-02T00:00:00Z"))
        db.execute(f"INSERT INTO images({', '.join(row)}) VALUES ({', '.join('?' * len(row))})", list(row.values()))
        db.executemany("INSERT INTO ratings VALUES (?, ?, ?, ?, ?, ?, ?)", [
            (2, SHA_A, "dislike", "undone", "2026-10-02T01:00:00Z", "second", 1),
            (3, SHA_A, "like", 'warm "gold" \u2605', "2026-10-02T02:00:00Z", "second", 0),
            (4, SHA_B, "skip", None, "2026-10-02T03:00:00Z", "second", 0),
            (5, SHA_C, "like", "private fixture", "2026-10-02T04:00:00Z", "second", 0)])
        db.execute("INSERT INTO pair_notes VALUES (?, ?, ?, ?)", (SHA_A, SHA_B, "calmer lines", "2026-10-02T05:00:00Z"))
        db.execute("INSERT INTO seen VALUES ('met', '123')")
        db.execute("INSERT INTO embeddings VALUES (?, 'fake', 2, ?)", (SHA_A, b"\0" * 8))
        db.execute("INSERT INTO text_embeddings VALUES ('gold', 'fake', 2, ?)", (b"\1" * 8,))
        db.execute("INSERT INTO model_runs VALUES (1, ?, 'fake', 2, 1, 1, 0.7, NULL, 0, '{}')", ("2026-10-02T06:00:00Z",))
        db.execute("INSERT INTO fetch_state VALUES ('met', 'gold', 'next', 0, ?)", ("2026-10-02T07:00:00Z",))
        db.execute("INSERT INTO proposals VALUES ('brass', 'steampunk', 'adjacent', 'accepted', 100, ?)",
                   ("2026-10-02T08:00:00Z",))
        db.execute("INSERT INTO screen_counts VALUES ('met', 10, 1)")
        db.execute("INSERT INTO blocked VALUES (?, ?)", ("d" * 64, "2026-10-02T09:00:00Z"))
        db.execute("INSERT INTO meta VALUES ('proposal_round', '100')")
        db.execute("INSERT INTO file_ops VALUES ('pending', ?, 'add', '[]', 0)", (SHA_B,))
        db.commit()
        self.assertGreater((root / (store.DB_NAME + "-wal")).stat().st_size, 0)
        return root, db, self.snapshot(db)

    @staticmethod
    def snapshot(db):
        result = {}
        for (table,) in db.execute("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name"):
            columns = [r[1] for r in db.execute(f"PRAGMA table_info({table})") if r[1] != "love"]
            result[table] = db.execute(f"SELECT {', '.join(columns)} FROM {table} ORDER BY rowid").fetchall()
        return result

    def assert_schema_two_unchanged(self, root, expected):
        with closing(sqlite3.connect(root / store.DB_NAME)) as db:
            self.assertEqual(db.execute("PRAGMA user_version").fetchone()[0], 2)
            self.assertNotIn("love", [r[1] for r in db.execute("PRAGMA table_info(ratings)")])
            self.assertEqual(self.snapshot(db), expected)

    def test_upgrade_preserves_every_old_column_and_backups_include_wal_commits(self):
        root, db, expected = self.fixture()
        main_only = self.tmp / "main-only.sqlite3"
        shutil.copyfile(root / store.DB_NAME, main_only)
        with closing(sqlite3.connect(main_only)) as checkpoint:
            self.assertEqual(checkpoint.execute("SELECT COUNT(*) FROM ratings").fetchone()[0], 1)
        self.assertEqual(store.upgrade_library(root), {"ratings": 5, "backup": self.BACKUP_NAME})
        with closing(sqlite3.connect(root / store.DB_NAME)) as reopened:
            self.assertEqual(reopened.execute("PRAGMA user_version").fetchone()[0], 3)
            self.assertEqual(self.snapshot(reopened), expected)
            self.assertEqual([r[0] for r in reopened.execute("SELECT love FROM ratings ORDER BY id")], [0] * 5)
            with self.assertRaises(sqlite3.IntegrityError):
                reopened.execute("UPDATE ratings SET love=1 WHERE verdict='skip'")
            reopened.rollback()
        # A standalone copy of only the backup file must recover every record.
        backup = root / "backups" / self.BACKUP_NAME
        self.assertFalse(Path(str(backup) + "-wal").exists())
        standalone = self.tmp / "standalone.sqlite3"
        shutil.copyfile(backup, standalone)
        with closing(sqlite3.connect(standalone)) as copy:
            self.assertEqual(copy.execute("PRAGMA user_version").fetchone()[0], 2)
            self.assertNotIn("love", [r[1] for r in copy.execute("PRAGMA table_info(ratings)")])
            self.assertEqual(self.snapshot(copy), expected)

    def test_upgrade_rolls_back_alter_version_and_all_preservation_checks(self):
        connect = sqlite3.connect
        for fault in ("alter", "version", "count", "digest", "nonzero"):
            root, db, expected = self.fixture(fault)

            class FaultyConnection(sqlite3.Connection):
                def execute(self, sql, *args):
                    result = super().execute(sql, *args)
                    if fault == "alter" and sql.startswith("ALTER TABLE ratings"):
                        raise sqlite3.OperationalError("injected after ALTER")
                    if sql == "PRAGMA user_version=3":
                        if fault == "version":
                            raise sqlite3.OperationalError("injected after version")
                        if fault == "count":
                            super().execute("DELETE FROM ratings WHERE id=5")
                        if fault == "digest":
                            super().execute("UPDATE ratings SET note='changed' WHERE id=3")
                    if fault == "nonzero" and sql == "SELECT COUNT(*) FROM ratings WHERE love != 0":
                        return mock.Mock(fetchone=lambda: (1,))
                    return result

            def opened(path, *args, **kwargs):
                if Path(path) == root / store.DB_NAME:
                    kwargs["factory"] = FaultyConnection
                return connect(path, *args, **kwargs)

            with self.subTest(fault=fault), mock.patch.object(sqlite3, "connect", side_effect=opened):
                with self.assertRaises(common.Refused):
                    store.upgrade_library(root)
            self.assert_schema_two_unchanged(root, expected)
            self.assertTrue((root / "backups" / self.BACKUP_NAME).is_file())

    def test_failed_backup_leaves_source_unchanged(self):
        root, db, expected = self.fixture()
        connect = sqlite3.connect

        class FailedBackup(sqlite3.Connection):
            def backup(self, target, *args, **kwargs):
                target.execute("CREATE TABLE partial(value TEXT)")
                target.commit()
                raise sqlite3.OperationalError("injected backup failure")

        def opened(path, *args, **kwargs):
            if Path(path) == root / store.DB_NAME:
                kwargs["factory"] = FailedBackup
            return connect(path, *args, **kwargs)

        with mock.patch.object(sqlite3, "connect", side_effect=opened), self.assertRaises(common.Refused):
            store.upgrade_library(root)
        self.assert_schema_two_unchanged(root, expected)
        self.assertFalse((root / "backups" / self.BACKUP_NAME).exists())

    def test_all_upgrade_paths_guard_links_junctions_and_sessions_before_writes(self):
        root, db, expected = self.fixture()
        paths = (root / store.DB_NAME, root / (store.DB_NAME + "-wal"), root / (store.DB_NAME + "-shm"),
                 root / "backups", root / "backups" / self.BACKUP_NAME)
        exists = Path.exists
        for component in paths:
            for guard in ("is_symlink", "is_junction", "session"):
                patch = (mock.patch.object(Path, "exists", autospec=True,
                                           side_effect=lambda p: p == component / "session.sqlite3" or exists(p))
                         if guard == "session" else
                         mock.patch.object(Path, guard, autospec=True, side_effect=lambda p: p == component))
                with self.subTest(path=component.name, guard=guard), patch, \
                        mock.patch.object(Path, "mkdir") as mkdir, mock.patch.object(sqlite3, "connect") as connect:
                    with self.assertRaises(common.Refused):
                        store.upgrade_library(root)
                    mkdir.assert_not_called()
                    connect.assert_not_called()
        self.assertFalse((root / "backups").exists())
        self.assert_schema_two_unchanged(root, expected)

    def test_store_schema_two_refusal_names_explicit_upgrade(self):
        root, db, expected = self.fixture()
        with self.assertRaisesRegex(common.Refused, r"has schema 2; run fetch.py --upgrade-library .*requires schema 3"):
            with store.Store(root):
                pass
        self.assert_schema_two_unchanged(root, expected)

    def test_upgrade_refuses_missing_schema_one_second_upgrade_and_existing_backup(self):
        missing = self.tmp / "missing"
        with self.assertRaises(common.Refused):
            store.upgrade_library(missing)
        self.assertFalse(missing.exists())
        root, db, expected = self.fixture()
        db.execute("PRAGMA user_version=1")
        with self.assertRaisesRegex(common.Refused, "schema 1"):
            store.upgrade_library(root)
        self.assertFalse((root / "backups").exists())
        db.execute("PRAGMA user_version=2")
        backups = root / "backups"
        backups.mkdir()
        path = backups / self.BACKUP_NAME
        path.write_bytes(b"existing backup canary")
        with self.assertRaisesRegex(common.Refused, "exists"):
            store.upgrade_library(root)
        self.assertEqual(path.read_bytes(), b"existing backup canary")
        self.assert_schema_two_unchanged(root, expected)
        path.unlink()
        store.upgrade_library(root)
        with self.assertRaisesRegex(common.Refused, "schema 3"):
            store.upgrade_library(root)
        with closing(sqlite3.connect(root / store.DB_NAME)) as reopened:
            self.assertEqual(self.snapshot(reopened), expected)

    def test_upgrade_cli_needs_no_model_or_client_and_refusals_exit_two(self):
        from tastelab import embed, fetch
        root, db, expected = self.fixture()
        with mock.patch.object(embed, "load_embedder", side_effect=AssertionError("upgrade needs no model")), \
                mock.patch.object(fetch.net, "Client", side_effect=AssertionError("upgrade needs no client")), \
                redirect_stdout(io.StringIO()) as output, redirect_stderr(io.StringIO()) as errors:
            self.assertEqual(fetch.main(["--data", str(root), "--upgrade-library"]), 0)
        self.assertEqual(errors.getvalue(), "")
        self.assertEqual(output.getvalue(), f"Upgraded the library to schema 3: 5 ratings kept; backup backups/{self.BACKUP_NAME}\n")
        with redirect_stderr(io.StringIO()) as errors:
            self.assertEqual(fetch.main(["--data", str(root), "--upgrade-library"]), 2)
        self.assertIn("schema 3", errors.getvalue())


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
    def test_missing_committed_bytes_roll_back_the_admission_seen_marker(self):
        root = self.tmp / "library"
        with store.Store(root) as library:
            with mock.patch.object(library, "_reconcile"):
                library.add_image(meta(SHA_A), b"thumb", b"original", "png")
            library.mark_seen("met", "unrelated")
            for path in root.rglob(".part-*"):
                path.unlink()
        with store.Store(root) as reopened:
            self.assertFalse(reopened.has_image(SHA_A))
            self.assertFalse(reopened.has_seen("wikimedia", "File:x.jpg"))
            self.assertTrue(reopened.has_seen("met", "unrelated"))
            self.assertFalse(reopened.is_blocked(SHA_A))
            self.assertEqual(reopened.db.execute("SELECT COUNT(*) FROM file_ops").fetchone()[0], 0)

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
        for size in ((64, 4097), (4097, 64)):
            with self.subTest(size=size), self.assertRaisesRegex(images.BadImage, "aspect"):
                images.decode(self.encode(Image.new("RGB", size), "PNG"))
        for size in ((64, 4096), (4096, 64)):
            with self.subTest(size=size):
                self.assertEqual(images.decode(self.encode(Image.new("RGB", size), "PNG")).image.size, size)

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

    def test_thumbnail_strips_comment_exif_xmp_icc_and_png_text(self):
        from tastelab import images
        for name, data, canary in metadata_images():
            with self.subTest(metadata=name):
                self.assertIn(canary, data)
                decoded = images.decode(data)
                try:
                    output = images.thumbnail_jpeg(decoded.image)
                finally:
                    decoded.image.close()
                self.assertNotIn(canary, output)
                with Image.open(io.BytesIO(output)) as thumbnail:
                    self.assertLessEqual(set(thumbnail.info), {"jfif", "jfif_version", "jfif_unit", "jfif_density"})
                    self.assertEqual(len(thumbnail.getexif()), 0)

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
