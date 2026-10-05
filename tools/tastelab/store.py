"""The Taste Lab store: one SQLite database plus image files under the private data folder.

Layout under the data folder (see common.data_root):
  tastelab.sqlite3                      images, ratings, embeddings, model runs, fetch state
  thumbs/<aa>/<sha256>.jpg              512 px thumbnail of every image
  originals/<aa>/<sha256>.<ext>         the fetched file, kept only under the size caps
  reports/                             aggregate calibration reports

An image is identified by the sha256 of the bytes that were fetched. Every image
row carries its source, page and image URL, licence, attribution, tier (A open
licence, B private reference) and seed category. Images the owner removes are
deleted and their sha256 is blocked, so no fetch stores them again. Images the
content screen discards are never stored; only per-source counts are kept.

Standard library only. A Store belongs to one thread; open another Store for a
worker thread (SQLite WAL lets them share the database).
"""
from __future__ import annotations

import json
import os
import sqlite3
from contextlib import contextmanager
from dataclasses import asdict, dataclass, fields
from pathlib import Path
from uuid import uuid4

from tastelab import common

SCHEMA_VERSION = 2
DB_NAME = "tastelab.sqlite3"
ORIGINAL_EXTS = ("jpg", "png", "webp", "gif")
PROPOSAL_STATES = ("proposed", "accepted", "rejected", "fetched")

SCHEMA = """
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


@dataclass
class ImageMeta:
    sha256: str
    source: str
    licence: str
    attribution: str
    tier: str
    category: str
    width: int
    height: int
    source_id: str | None = None
    page_url: str | None = None
    image_url: str | None = None
    licence_url: str | None = None
    title: str | None = None
    query: str | None = None
    bytes: int = 0
    original_ext: str | None = None
    added: str = ""


@dataclass
class Rating:
    id: int
    sha256: str
    verdict: str
    note: str | None
    ts: str
    session: str


_IMAGE_COLUMNS = [f.name for f in fields(ImageMeta)]


def _clean_note(note) -> str | None:
    if note is None:
        return None
    text = " ".join(str(note).split())[: common.NOTE_MAX]
    return text or None


class Store:
    def __init__(self, root, *, max_images: int = common.MAX_IMAGES, max_bytes: int = common.MAX_BYTES):
        self.root = common.data_root(root)
        self.max_images = max_images
        self.max_bytes = max_bytes
        existed = (self.root / DB_NAME).exists()
        for sub in ("thumbs", "originals", "reports", DB_NAME, DB_NAME + "-wal", DB_NAME + "-shm"):
            common.check_input(self.root / sub, kind="store")
        for sub in ("thumbs", "originals", "reports"):
            (self.root / sub).mkdir(parents=True, exist_ok=True)
        self.db = sqlite3.connect(self.root / DB_NAME, timeout=30)
        self.db.row_factory = sqlite3.Row
        version = self.db.execute("PRAGMA user_version").fetchone()[0]
        if version != SCHEMA_VERSION and (existed or version != 0):
            self.db.close()
            raise common.Refused(f"{DB_NAME} has schema {version}; this Taste Lab requires schema {SCHEMA_VERSION}; no automatic migration")
        self.db.execute("PRAGMA journal_mode=WAL")
        self.db.execute("PRAGMA foreign_keys=ON")
        with self.db:
            self.db.executescript(SCHEMA)
            self.db.execute(f"PRAGMA user_version={SCHEMA_VERSION}")
        self._reconcile()

    def close(self) -> None:
        self.db.close()

    def __enter__(self):
        return self

    def __exit__(self, *exc):
        self.close()

    # -- files -------------------------------------------------------------

    def thumb_path(self, sha: str) -> Path:
        if not common.valid_sha(sha):
            raise ValueError("not a sha256")
        return self.root / "thumbs" / sha[:2] / f"{sha}.jpg"

    def original_path(self, sha: str) -> Path | None:
        row = self.db.execute("SELECT original_ext FROM images WHERE sha256=?", (sha,)).fetchone()
        if row is None or row[0] is None:
            return None
        return self._original_file(sha, row[0])

    def display_path(self, sha: str) -> Path:
        """The largest stored file of an image: the original when kept, else the thumbnail."""
        return self.original_path(sha) or self.thumb_path(sha)

    def _original_file(self, sha: str, ext: str) -> Path:
        if not common.valid_sha(sha) or ext not in ORIGINAL_EXTS:
            raise ValueError("bad original name")
        return self.root / "originals" / sha[:2] / f"{sha}.{ext}"

    @contextmanager
    def _immediate(self):
        self.db.execute("BEGIN IMMEDIATE")
        try:
            yield
            self.db.commit()
        except BaseException:
            self.db.rollback()
            raise

    @staticmethod
    def _write(path: Path, data: bytes) -> None:
        """Write only a journal-owned temporary name, in the destination folder."""
        common.check_input(path, kind="store file")
        path.parent.mkdir(parents=True, exist_ok=True)
        with path.open("xb") as fh:
            fh.write(data)
            fh.flush()
            os.fsync(fh.fileno())

    def _operation_path(self, name: str) -> Path:
        path = Path(os.path.abspath(self.root / name))
        if not path.is_relative_to(self.root):
            raise common.Refused("store journal path escapes the store")
        return common.check_input(path, kind="store journal")

    def _finish_operation(self, op) -> bool:
        files = json.loads(op["files"])
        if op["op"] == "add" and op["committed"] and self.has_image(op["sha256"]):
            if any(not self._operation_path(f["temp"]).exists() and not self._operation_path(f["target"]).exists()
                   for f in files):
                # A committed add with missing bytes is rolled back. Commit its
                # removal journal before attempting the corresponding deletions.
                if not self.is_blocked(op["sha256"]):
                    self.db.execute("DELETE FROM seen WHERE (source, source_id) IN "
                                    "(SELECT source, source_id FROM images WHERE sha256=?)", (op["sha256"],))
                self.db.execute("DELETE FROM images WHERE sha256=?", (op["sha256"],))
                self.db.execute("UPDATE file_ops SET op='remove' WHERE id=?", (op["id"],))
                return False
            for f in files:
                temp, target = (self._operation_path(f[k]) for k in ("temp", "target"))
                if temp.exists():
                    if target.exists():
                        temp.unlink()
                    else:
                        temp.rename(target)
        else:
            for f in files:
                # An uncommitted add owns only its temporary names. In particular,
                # its cleanup can never delete another writer's final files.
                keys = ("temp", "target") if op["op"] == "remove" else ("temp",)
                for key in keys:
                    self._operation_path(f[key]).unlink(missing_ok=True)
        self.db.execute("DELETE FROM file_ops WHERE id=?", (op["id"],))
        return True

    def _reconcile(self) -> None:
        """Serialize recovery with admissions/removals; failed deletes stay journalled."""
        ids = [r[0] for r in self.db.execute("SELECT id FROM file_ops ORDER BY rowid")]
        for ident in ids:
            for _ in range(2):
                try:
                    with self._immediate():
                        op = self.db.execute("SELECT * FROM file_ops WHERE id=?", (ident,)).fetchone()
                        if op is None or self._finish_operation(op):
                            break
                except OSError:
                    break

    # -- images ------------------------------------------------------------

    def has_image(self, sha: str) -> bool:
        return self.db.execute("SELECT 1 FROM images WHERE sha256=?", (sha,)).fetchone() is not None

    def is_blocked(self, sha: str) -> bool:
        return self.db.execute("SELECT 1 FROM blocked WHERE sha256=?", (sha,)).fetchone() is not None

    def has_seen(self, source: str, source_id: str) -> bool:
        return self.db.execute("SELECT 1 FROM seen WHERE source=? AND source_id=?", (source, source_id)).fetchone() is not None

    def mark_seen(self, source: str, source_id: str) -> None:
        """Remember a handled candidate. Never called for an image the content screen discarded."""
        with self.db:
            self.db.execute("INSERT OR IGNORE INTO seen(source, source_id) VALUES (?, ?)", (source, source_id))

    def count_images(self) -> int:
        return self.db.execute("SELECT COUNT(*) FROM images").fetchone()[0]

    def bytes_used(self) -> int:
        return self.db.execute("SELECT COALESCE(SUM(bytes), 0) FROM images").fetchone()[0]

    def is_full(self, extra_bytes: int = 0) -> bool:
        return self.count_images() >= self.max_images or self.bytes_used() + extra_bytes > self.max_bytes

    def add_image(self, meta: ImageMeta, thumb: bytes, original: bytes | None = None, original_ext: str | None = None,
                  *, embedding=None) -> str:
        """Store one screened image. Returns 'stored', 'duplicate', 'blocked' or 'full'.

        The original is kept only when it is at most ORIGINAL_MAX_BYTES and the
        store stays below ORIGINAL_SHARE of its byte cap."""
        if not common.valid_sha(meta.sha256):
            raise ValueError("sha256 must be 64 lowercase hex digits")
        if meta.tier not in common.TIERS:
            raise ValueError("tier must be A or B")
        for name in ("source", "licence", "attribution", "category"):
            if not isinstance(getattr(meta, name), str) or not getattr(meta, name).strip():
                raise ValueError(f"{name} is required")
        if not thumb:
            raise ValueError("thumbnail is empty")
        if embedding is not None and len(embedding[2]) != embedding[1] * 4:
            raise ValueError("vector length does not match dim")
        while True:
            ident = uuid4().hex
            targets = [self.thumb_path(meta.sha256)]
            if original is not None and original_ext in ORIGINAL_EXTS:
                targets.append(self._original_file(meta.sha256, original_ext))
            files = [{"temp": str(p.with_name(f".part-{ident}-{p.name}").relative_to(self.root)),
                      "target": str(p.relative_to(self.root))} for p in targets]
            # This durable intent survives termination and rollback of the admission
            # transaction. Opening another Store may reconcile it before admission;
            # in that case retry without having written any bytes.
            with self._immediate():
                self.db.execute("INSERT INTO file_ops(id, sha256, op, files) VALUES (?, ?, 'add', ?)",
                                (ident, meta.sha256, json.dumps(files)))
            try:
                with self._immediate():
                    if self.db.execute("SELECT 1 FROM file_ops WHERE id=?", (ident,)).fetchone() is None:
                        continue
                    reason = "blocked" if self.is_blocked(meta.sha256) else "duplicate" if self.has_image(meta.sha256) else None
                    used = self.bytes_used()
                    if reason is None and (self.count_images() >= self.max_images or used + len(thumb) > self.max_bytes):
                        reason = "full"
                    if reason is not None:
                        self.db.execute("DELETE FROM file_ops WHERE id=?", (ident,))
                        return reason
                    if self.db.execute("SELECT 1 FROM file_ops WHERE sha256=? AND op='remove'", (meta.sha256,)).fetchone():
                        raise common.Refused("pending file removal; reopen the store to retry cleanup")
                    keep = (len(files) == 2 and len(original) <= common.ORIGINAL_MAX_BYTES
                            and used + len(thumb) + len(original) <= common.ORIGINAL_SHARE * self.max_bytes)
                    files = files if keep else files[:1]
                    self.db.execute("UPDATE file_ops SET files=? WHERE id=?", (json.dumps(files), ident))
                    self._write(self._operation_path(files[0]["temp"]), thumb)
                    if keep:
                        self._write(self._operation_path(files[1]["temp"]), original)
                    row = asdict(meta)
                    row.update(bytes=len(thumb) + (len(original) if keep else 0),
                               original_ext=original_ext if keep else None, added=meta.added or common.now_iso())
                    self.db.execute(f"INSERT INTO images({', '.join(_IMAGE_COLUMNS)}) VALUES ({', '.join('?' * len(_IMAGE_COLUMNS))})",
                                    [row[c] for c in _IMAGE_COLUMNS])
                    if embedding is not None:
                        model, dim, blob = embedding
                        self.db.execute("INSERT INTO embeddings VALUES (?, ?, ?, ?)", (meta.sha256, model, dim, blob))
                    if meta.source_id:
                        self.db.execute("INSERT OR IGNORE INTO seen VALUES (?, ?)", (meta.source, meta.source_id))
                    self.db.execute("UPDATE file_ops SET committed=1 WHERE id=?", (ident,))
                break
            except BaseException:
                self._reconcile()
                raise
        self._reconcile()  # Final names are published only after the row commits.
        return "stored"

    def image(self, sha: str) -> ImageMeta | None:
        row = self.db.execute("SELECT * FROM images WHERE sha256=?", (sha,)).fetchone()
        return ImageMeta(**dict(row)) if row is not None else None

    def images(self, *, tier: str | None = None, category: str | None = None) -> list[ImageMeta]:
        sql, args = "SELECT * FROM images WHERE 1=1", []
        if tier is not None:
            sql, args = sql + " AND tier=?", args + [tier]
        if category is not None:
            sql, args = sql + " AND category=?", args + [category]
        return [ImageMeta(**dict(r)) for r in self.db.execute(sql + " ORDER BY added, sha256", args)]

    def remove_image(self, sha: str) -> bool:
        """Delete an image's files, rows, ratings and embeddings, and block its sha256 for good."""
        if not common.valid_sha(sha):
            raise ValueError("not a sha256")
        with self._immediate():
            meta = self.image(sha)
            if meta is not None:
                paths = [self.thumb_path(sha)]
                if meta.original_ext:
                    paths.append(self._original_file(sha, meta.original_ext))
                files = [{"temp": str(p.relative_to(self.root)), "target": str(p.relative_to(self.root))} for p in paths]
                self.db.execute("INSERT INTO file_ops(id, sha256, op, files) VALUES (?, ?, 'remove', ?)",
                                (uuid4().hex, sha, json.dumps(files)))
            self.db.execute("INSERT OR IGNORE INTO blocked(sha256, ts) VALUES (?, ?)", (sha, common.now_iso()))
            self.db.execute("DELETE FROM images WHERE sha256=?", (sha,))
        self._reconcile()
        return meta is not None

    def category_counts(self) -> dict[str, int]:
        return {r[0]: r[1] for r in self.db.execute("SELECT category, COUNT(*) FROM images GROUP BY category")}

    def query_counts(self) -> dict[tuple[str, str], int]:
        """Stored images per (category, query), for fetch targets."""
        rows = self.db.execute("SELECT category, COALESCE(query, ''), COUNT(*) FROM images GROUP BY category, query")
        return {(r[0], r[1]): r[2] for r in rows}

    # -- ratings -----------------------------------------------------------

    def add_rating(self, sha: str, verdict: str, session: str, note: str | None = None) -> int:
        if verdict not in common.VERDICTS:
            raise ValueError("verdict must be like, dislike or skip")
        if not session:
            raise ValueError("session is required")
        with self.db:
            cur = self.db.execute("INSERT INTO ratings(sha256, verdict, note, ts, session) VALUES (?, ?, ?, ?, ?)",
                                  (sha, verdict, _clean_note(note), common.now_iso(), session))
        return cur.lastrowid

    def undo_last(self, session: str) -> str | None:
        """Undo the latest rating of `session`; returns its image so the window can show it again."""
        row = self.db.execute("SELECT id, sha256 FROM ratings WHERE session=? AND undone=0 ORDER BY id DESC LIMIT 1",
                              (session,)).fetchone()
        if row is None:
            return None
        with self.db:
            self.db.execute("UPDATE ratings SET undone=1 WHERE id=?", (row["id"],))
        return row["sha256"]

    def ratings(self) -> list[Rating]:
        """The effective rating of every rated image (its latest one that was not undone), oldest first."""
        rows = self.db.execute("""
            SELECT r.id, r.sha256, r.verdict, r.note, r.ts, r.session FROM ratings r
            JOIN (SELECT sha256, MAX(id) AS id FROM ratings WHERE undone=0 GROUP BY sha256) last ON last.id = r.id
            ORDER BY r.id""")
        return [Rating(**dict(r)) for r in rows]

    def rated(self) -> set[str]:
        return {r[0] for r in self.db.execute("SELECT DISTINCT sha256 FROM ratings WHERE undone=0")}

    def counts(self, day: str | None = None) -> dict[str, int]:
        """Rating counts: effective likes, dislikes and skips, their total, and ratings given on `day` (local)."""
        day = day or common.local_day()
        out = {"like": 0, "dislike": 0, "skip": 0}
        for r in self.ratings():
            out[r.verdict] += 1
        out["total"] = out["like"] + out["dislike"] + out["skip"]
        out["today"] = sum(1 for (ts,) in self.db.execute("SELECT ts FROM ratings WHERE undone=0") if common.local_day(ts) == day)
        return out

    # -- embeddings --------------------------------------------------------

    def put_embeddings(self, model: str, dim: int, items) -> None:
        """Store float32 vectors: `items` yields (sha256, bytes of length dim * 4)."""
        rows = []
        for sha, blob in items:
            if len(blob) != dim * 4:
                raise ValueError("vector length does not match dim")
            rows.append((sha, model, dim, bytes(blob)))
        with self.db:
            self.db.executemany("INSERT OR REPLACE INTO embeddings(sha256, model, dim, vector) VALUES (?, ?, ?, ?)", rows)

    def embeddings(self, model: str, shas=None) -> dict[str, bytes]:
        if shas is None:
            rows = self.db.execute("SELECT sha256, vector FROM embeddings WHERE model=?", (model,))
            return {r[0]: r[1] for r in rows}
        out = {}
        for sha in shas:
            row = self.db.execute("SELECT vector FROM embeddings WHERE model=? AND sha256=?", (model, sha)).fetchone()
            if row is not None:
                out[sha] = row[0]
        return out

    def embedded(self, model: str) -> set[str]:
        """The images that have a vector of `model`, without reading the vectors."""
        return {r[0] for r in self.db.execute("SELECT sha256 FROM embeddings WHERE model=?", (model,))}

    def missing_embeddings(self, model: str) -> list[str]:
        rows = self.db.execute("""SELECT sha256 FROM images WHERE sha256 NOT IN
                                  (SELECT sha256 FROM embeddings WHERE model=?) ORDER BY added, sha256""", (model,))
        return [r[0] for r in rows]

    def put_text_embeddings(self, model: str, dim: int, items) -> None:
        rows = []
        for text, blob in items:
            if len(blob) != dim * 4:
                raise ValueError("vector length does not match dim")
            rows.append((text, model, dim, bytes(blob)))
        with self.db:
            self.db.executemany("INSERT OR REPLACE INTO text_embeddings(text, model, dim, vector) VALUES (?, ?, ?, ?)", rows)

    def text_embeddings(self, model: str, texts) -> dict[str, bytes]:
        out = {}
        for text in texts:
            row = self.db.execute("SELECT vector FROM text_embeddings WHERE model=? AND text=?", (model, text)).fetchone()
            if row is not None:
                out[text] = row[0]
        return out

    # -- model runs --------------------------------------------------------

    def add_model_run(self, model: str, ratings: int, likes: int, dislikes: int, auc: float | None,
                      auc_delta: float | None, stable: bool, params: dict | None = None) -> int:
        with self.db:
            cur = self.db.execute("""INSERT INTO model_runs(ts, model, ratings, likes, dislikes, auc, auc_delta, stable, params)
                                     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)""",
                                  (common.now_iso(), model, ratings, likes, dislikes, auc, auc_delta, int(bool(stable)),
                                   json.dumps(params or {}, sort_keys=True)))
        return cur.lastrowid

    def model_runs(self, limit: int | None = None) -> list[dict]:
        sql = "SELECT * FROM model_runs ORDER BY id DESC" + (" LIMIT ?" if limit else "")
        rows = [dict(r) for r in self.db.execute(sql, (limit,) if limit else ())]
        for r in rows:
            r["params"] = json.loads(r["params"])
            r["stable"] = bool(r["stable"])
        return rows[::-1]

    # -- fetch state, proposals, screen counts ----------------------------

    def cursor(self, source: str, query: str) -> tuple[str | None, bool]:
        row = self.db.execute("SELECT cursor, done FROM fetch_state WHERE source=? AND query=?", (source, query)).fetchone()
        return (row[0], bool(row[1])) if row is not None else (None, False)

    def set_cursor(self, source: str, query: str, cursor: str | None, done: bool = False) -> None:
        with self.db:
            self.db.execute("INSERT OR REPLACE INTO fetch_state(source, query, cursor, done, updated) VALUES (?, ?, ?, ?, ?)",
                            (source, query, cursor, int(bool(done)), common.now_iso()))

    def add_proposals(self, items, at_rating: int) -> int:
        """Record proposed search terms: `items` yields (term, category or None, origin). Known terms are kept as they are."""
        added = 0
        with self.db:
            for term, category, origin in items:
                term = " ".join(str(term).split())
                if term:
                    cur = self.db.execute("""INSERT OR IGNORE INTO proposals(term, category, origin, status, at_rating, ts)
                                             VALUES (?, ?, ?, 'proposed', ?, ?)""", (term, category, origin, at_rating, common.now_iso()))
                    added += cur.rowcount
        return added

    def proposals(self, status: str | None = None) -> list[dict]:
        sql, args = "SELECT * FROM proposals", ()
        if status is not None:
            sql, args = sql + " WHERE status=?", (status,)
        return [dict(r) for r in self.db.execute(sql + " ORDER BY at_rating, term", args)]

    def set_proposal_status(self, term: str, status: str) -> None:
        if status not in PROPOSAL_STATES:
            raise ValueError("unknown proposal status")
        with self.db:
            self.db.execute("UPDATE proposals SET status=?, ts=? WHERE term=?", (status, common.now_iso(), term))

    def mark_proposal_round(self, at_rating: int) -> None:
        """Record that terms were proposed at `at_rating` ratings, also when the round found no new term."""
        with self.db:
            self.db.execute("INSERT OR REPLACE INTO meta(key, value) VALUES ('proposal_round', ?)", (str(int(at_rating)),))

    def last_proposal_rating(self) -> int:
        """The rating count at the latest proposal round (0 before the first)."""
        row = self.db.execute("SELECT value FROM meta WHERE key='proposal_round'").fetchone()
        stored = self.db.execute("SELECT COALESCE(MAX(at_rating), 0) FROM proposals").fetchone()[0]
        return max(stored, int(row[0]) if row is not None else 0)

    def add_screen_counts(self, source: str, checked: int, discarded: int) -> None:
        """Count screened and discarded images per source. Nothing about a discarded image itself is kept."""
        with self.db:
            self.db.execute("""INSERT INTO screen_counts(source, checked, discarded) VALUES (?, ?, ?)
                               ON CONFLICT(source) DO UPDATE SET checked=checked+excluded.checked,
                                                                 discarded=discarded+excluded.discarded""",
                            (source, checked, discarded))

    def screen_counts(self) -> dict[str, tuple[int, int]]:
        return {r[0]: (r[1], r[2]) for r in self.db.execute("SELECT source, checked, discarded FROM screen_counts")}
