"""Fetch approved sources into a private library, calibrate, or import page ratings.

Candidates and bytes stay in memory through rights, decoding, embedding,
screening, relevance and deduplication. Only admitted images are committed;
discarded cases leave aggregate counts only. No owner-folder or URL ingestion.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import re
import sys
from collections import Counter
from dataclasses import asdict, dataclass
from datetime import datetime, timezone
from pathlib import Path
from uuid import uuid4

if not __package__:
    sys.path[0] = str(Path(__file__).resolve().parents[1])

import numpy as np

from tastelab import common, embed, images, licences, net, screen, seeds, sources, store

DOWNLOAD_MAX_BYTES = 20 * 1024 * 1024
IMAGE_TYPES = ("image/jpeg", "image/png", "image/webp", "image/gif")
NEAR_DUPLICATE = 0.97
CALIBRATION_REPORT = "calibration.json"


@dataclass
class Outcome:
    kind: str
    sha256: str | None = None


class Pipeline:
    def __init__(self, store, embedder, screen, client, *, relevance_min=0.0, near_duplicate=NEAR_DUPLICATE, phrase=None):
        self.store, self.embedder, self.screen, self.client = store, embedder, screen, client
        self.relevance_min, self.near_duplicate = relevance_min, near_duplicate
        self.phrase = phrase or (lambda text: text)

    def ingest_candidate(self, candidate, *, category, query, tier, adapter):
        if candidate.source != adapter.name or not sources.admit(candidate, tier):
            return Outcome("rejected")
        if self.store.has_seen(candidate.source, candidate.source_id):
            return Outcome("duplicate")
        try:
            response = self.client.get(candidate.image_url, hosts=adapter.hosts, min_interval=adapter.min_interval,
                                       max_bytes=DOWNLOAD_MAX_BYTES, accept=", ".join(IMAGE_TYPES), extra_headers=adapter.headers())
        except net.NetError:
            return Outcome("error")
        content_type = response.headers.get("content-type", "").split(";", 1)[0].lower()
        if content_type and content_type not in IMAGE_TYPES:
            return Outcome("invalid")
        return self.ingest_bytes(response.body, source=candidate.source, source_id=candidate.source_id,
                                 image_url=candidate.image_url, page_url=candidate.page_url, licence=candidate.licence,
                                 licence_url=candidate.licence_url, attribution=candidate.attribution, title=candidate.title,
                                 tier=tier, category=category, query=query, strict=tier == "B" or candidate.anime)

    def ingest_bytes(self, data, *, source, licence, attribution, tier, category, strict, source_id=None, page_url=None,
                     image_url=None, licence_url=None, title=None, query=None):
        candidate = sources.Candidate(source, source_id, image_url, page_url, licence, licence_url, attribution, title)
        if not sources.admit(candidate, tier):
            return Outcome("rejected")
        sha = hashlib.sha256(data).hexdigest()
        if self.store.is_blocked(sha):
            return Outcome("blocked")
        if self.store.has_image(sha):
            self.store.mark_seen(source, source_id)
            return Outcome("duplicate")
        try:
            decoded = images.decode(data)
        except images.BadImage:
            return Outcome("invalid")
        try:
            try:
                vectors = np.asarray(self.embedder.embed_images([decoded.image]), dtype=np.float32)
                invalid = (vectors.shape != (1, self.embedder.dim) or not np.isfinite(vectors).all()
                           or abs(float(np.linalg.norm(vectors)) - 1) > 1e-3)
                discard = invalid or self.screen.check(vectors, strict=strict or tier == "B")[0]
            except Exception:
                discard = True
            self.store.add_screen_counts(source, 1, int(discard))
            if discard:
                return Outcome("discarded")
            if self.relevance_min > 0 and query:
                text = np.asarray(self.embedder.embed_texts([self.phrase(query)]), dtype=np.float32)
                if (text.shape != vectors.shape or not np.isfinite(text).all()
                        or float(vectors[0] @ text[0]) < self.relevance_min):
                    return Outcome("irrelevant")
            blobs = list(self.store.embeddings(self.embedder.model_id).values())
            if blobs:
                previous = embed.from_blobs(blobs, self.embedder.dim)
                if np.any(previous @ vectors[0] >= self.near_duplicate):
                    self.store.mark_seen(source, source_id)
                    return Outcome("duplicate")
            thumb = images.thumbnail_jpeg(decoded.image)
            meta = store.ImageMeta(sha, source, licence, attribution, tier, category, decoded.width, decoded.height,
                                   source_id=source_id, page_url=page_url, image_url=image_url, licence_url=licence_url,
                                   title=title, query=query)
            result = self.store.add_image(meta, thumb, None if decoded.animated else data, decoded.ext,
                                          embedding=(self.embedder.model_id, self.embedder.dim, embed.to_blob(vectors[0])))
            return Outcome(result, sha if result == "stored" else None)
        finally:
            decoded.image.close()


def run(store, embedder, screen, client, plan, *, limit=None, sources=None, categories=None, phrase=None, log=print):
    selected = set(sources if sources is not None else (name for name, a in globals()["sources"].ADAPTERS.items() if a.tier == "A"))
    pipeline = Pipeline(store, embedder, screen, client, relevance_min=plan.relevance_min, phrase=phrase)
    counts = Counter()
    for category in plan.categories.values():
        if categories is not None and category.name not in categories:
            continue
        for term in category.terms:
            for name in plan.active_sources(category.name):
                if name not in selected:
                    continue
                adapter = globals()["sources"].ADAPTERS[name]
                query, key = term.query(name), adapter.cursor_key(term.query(name))
                cursor, done = store.cursor(name, key)
                while not done and store.category_counts().get(category.name, 0) < category.target:
                    if store.is_full() or limit is not None and counts["stored"] >= limit:
                        return dict(counts)
                    try:
                        candidates, next_cursor = adapter.search(client, query, cursor, seen=lambda ident: store.has_seen(name, ident))
                    except net.NetError:
                        counts["error"] += 1
                        break
                    completed = True
                    for candidate in candidates:
                        if (store.category_counts().get(category.name, 0) >= category.target or store.is_full()
                                or limit is not None and counts["stored"] >= limit):
                            completed = False
                            break
                        result = pipeline.ingest_candidate(candidate, category=category.name, query=term.text,
                                                           tier=plan.tier_for(category.name, name), adapter=adapter)
                        counts[result.kind] += 1
                        if result.kind == "full":
                            return dict(counts)
                    if not completed:
                        break
                    done = next_cursor is None or next_cursor == cursor
                    store.set_cursor(name, key, next_cursor, done)
                    cursor = next_cursor
    log("Fetch counts: " + json.dumps(dict(counts), sort_keys=True))
    return dict(counts)


def _probe_sha():
    return hashlib.sha256(json.dumps(screen.UNSAFE_PROBES + screen.SAFE_PROBES).encode()).hexdigest()


def _adult_subject(candidate, *, positive):
    text = " ".join([candidate.title or "", *candidate.keywords])
    if re.search(r"\b(?:child(?:ren)?|boy|girl|infant|teen(?:ager)?|adolescent|minor|youth)\b", text, re.I):
        return False
    adult = re.search(r"\b(?:adult|woman|women|man|men)\b", text, re.I)
    people = re.search(r"\b(?:person|people|portrait|nude|naked|body|dancer)\b", text, re.I)
    return bool(adult) if positive or people else True


def calibrate(embedder, client, *, log=print):
    content_screen = screen.Screen(embedder)
    groups, negatives, known = {}, [], set()

    def collect(queries, target, match=None):
        values = []
        for name, query in queries:
            adapter, cursor, cursors = sources.ADAPTERS[name], None, set()
            while len(values) < target:
                try:
                    candidates, next_cursor = adapter.search(client, query, cursor, page_size=screen.GROUP_SIZE,
                                                             seen=lambda ident: (name, ident) in known)
                except net.NetError:
                    break
                for candidate in candidates:
                    if len(values) >= target:
                        break
                    ident = (name, candidate.source_id)
                    label = " ".join([candidate.title or "", *candidate.keywords])
                    if (ident in known or candidate.licence not in sources.PUBLIC_DOMAIN_LICENCES
                            or not sources.admit(candidate, "A") or not _adult_subject(candidate, positive=match is not None)
                            or match is not None and not re.search(match, label, re.I)):
                        continue
                    known.add(ident)
                    score = math.nan
                    decoded = None
                    try:
                        response = client.get(candidate.image_url, hosts=adapter.hosts, min_interval=adapter.min_interval,
                                              max_bytes=DOWNLOAD_MAX_BYTES, extra_headers=adapter.headers())
                        decoded = images.decode(response.body)
                        vectors = embedder.embed_images([decoded.image])
                        score = float(content_screen.unsafe_score(vectors)[0])
                    except Exception:
                        pass  # Only the unscorable count survives this in-memory case.
                    finally:
                        if decoded is not None:
                            decoded.image.close()
                    values.append(score)
                if next_cursor is None or next_cursor in cursors:
                    break
                cursors.add(next_cursor)
                cursor = next_cursor
        return values

    for name, group in screen.CALIBRATION_GROUPS.items():
        groups[name] = collect(group["queries"], screen.GROUP_SIZE, group["match"])
    for query in screen.NEGATIVE_QUERIES:
        negatives.extend(collect((query,), screen.NEGATIVE_SIZE))
    result = screen.gate(groups, negatives)
    if not result.passed:
        result = screen.gate(groups, negatives, threshold=screen.LOWERED[0], strict_threshold=screen.LOWERED[1])
    report = {"model": embedder.model_id, "weightsSha256": getattr(embedder, "weights_sha256", ""),
              "probesSha256": _probe_sha(), "time": common.now_iso(), **asdict(result)}
    log("Calibration counts: " + json.dumps({"passed": report["passed"], "groups": report["groups"],
                                            "negatives": report["negatives"]}, sort_keys=True))
    return report


def write_calibration(store_root, report):
    root = common.data_root(store_root)
    folder = common.check_input(root / "reports", kind="calibration report")
    path = common.check_input(folder / CALIBRATION_REPORT, kind="calibration report")
    folder.mkdir(parents=True, exist_ok=True)
    temporary = folder / (".part-" + uuid4().hex)
    try:
        temporary.write_text(json.dumps(report, allow_nan=False, indent=2) + "\n", encoding="utf-8")
        os.replace(temporary, path)
    finally:
        temporary.unlink(missing_ok=True)
    return path


def load_screen(embedder, store_root):
    path = common.check_input(common.data_root(store_root) / "reports" / CALIBRATION_REPORT)
    try:
        report = json.loads(path.read_text(encoding="utf-8"))
        thresholds = (report["threshold"], report["strict_threshold"])
        valid = (report["passed"] is True and report["model"] == embedder.model_id
                 and report["weightsSha256"] == getattr(embedder, "weights_sha256", "")
                 and report["probesSha256"] == _probe_sha()
                 and thresholds in ((screen.THRESHOLD, screen.STRICT_THRESHOLD), screen.LOWERED)
                 and all(report["groups"][name]["scorable"] >= screen.MIN_GROUP
                         and report["groups"][name]["missed_normal"] <= 1
                         and report["groups"][name]["missed_strict"] == 0 for name in screen.CALIBRATION_GROUPS))
    except (OSError, ValueError, KeyError, TypeError):
        valid = False
    if not valid:
        raise common.Refused("content screen needs a passing --calibrate report for this model and these probes")
    return screen.Screen(embedder, threshold=thresholds[0], strict_threshold=thresholds[1])


def _timestamp(value, field):
    if not isinstance(value, str) or not re.fullmatch(
            r"[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}(?:\.[0-9]+)?(?:Z|[+-][0-9]{2}:[0-9]{2})", value):
        raise common.Refused(f"{field}: expected an ISO 8601 timestamp with timezone")
    try:
        moment = datetime.fromisoformat(value.replace("Z", "+00:00")).astimezone(timezone.utc)
    except ValueError:
        raise common.Refused(f"{field}: invalid timestamp") from None
    return moment, moment.isoformat().replace("+00:00", "Z")


def _note(value, maximum, field, *, nullable=False):
    if value is None and nullable:
        return
    if not licences.printable(value) or not 1 <= len(value) <= maximum:
        raise common.Refused(f"{field}: expected 1 to {maximum} characters" + (" or null" if nullable else ""))


def import_ratings(library, data):
    """Validate the complete page export before writing; apply newest answers atomically."""
    if (not isinstance(data, dict) or data.get("kind") != "tastelab-export" or type(data.get("version")) is not int
            or data.get("version") != 3 or not isinstance(data.get("images"), dict)):
        raise common.Refused("ratings export: expected tastelab-export version 3 with images")
    ratings, pairs = {}, {}
    for name in ("ratings", "pairs"):
        if not isinstance(data["images"].get(name), list):
            raise common.Refused(f"images.{name}: expected a list")
    for index, raw in enumerate(data["images"]["ratings"]):
        field = f"images.ratings[{index}]"
        if not isinstance(raw, dict) or not {"imageId", "verdict", "note", "ratedAt"} <= raw.keys():
            raise common.Refused(f"{field}: missing rating fields")
        sha = raw["imageId"]
        if not common.valid_sha(sha) or raw["verdict"] not in ("like", "dislike"):
            raise common.Refused(f"{field}: invalid imageId or verdict")
        _note(raw["note"], 140, field + ".note", nullable=True)
        moment, ts = _timestamp(raw["ratedAt"], field + ".ratedAt")
        if sha not in ratings or moment > ratings[sha][0]:
            ratings[sha] = (moment, ts, raw)
    for index, raw in enumerate(data["images"]["pairs"]):
        field = f"images.pairs[{index}]"
        if not isinstance(raw, dict) or not {"likedImageId", "dislikedImageId", "note", "notedAt"} <= raw.keys():
            raise common.Refused(f"{field}: missing pair fields")
        key = (raw["likedImageId"], raw["dislikedImageId"])
        if not all(common.valid_sha(sha) for sha in key) or key[0] == key[1]:
            raise common.Refused(f"{field}: invalid image ids")
        _note(raw["note"], 280, field + ".note")
        moment, ts = _timestamp(raw["notedAt"], field + ".notedAt")
        if key not in pairs or moment > pairs[key][0]:
            pairs[key] = (moment, ts, raw)
    ids = set(ratings) | {sha for key in pairs for sha in key}
    counts = {"ratings": 0, "pairs": 0, "unknown": []}
    with library._immediate():
        tiers = {sha: row[0] for sha in ids if (row := library.db.execute("SELECT tier FROM images WHERE sha256=?", (sha,)).fetchone())}
        blocked = sorted(sha for sha, tier in tiers.items() if tier == "B")
        if blocked:
            raise common.Refused("ratings export names tier B images: " + ", ".join(blocked))
        counts["unknown"] = sorted(ids - tiers.keys())
        for sha, (moment, ts, raw) in ratings.items():
            if sha not in tiers:
                continue
            previous = library.db.execute("SELECT ts FROM ratings WHERE sha256=? AND undone=0", (sha,)).fetchall()
            if previous and moment <= max(_timestamp(row[0], "stored ratedAt")[0] for row in previous):
                continue
            library.db.execute("INSERT INTO ratings(sha256, verdict, note, ts, session) VALUES (?, ?, ?, ?, 'page-export')",
                               (sha, raw["verdict"], raw["note"], ts))
            counts["ratings"] += 1
        for key, (moment, ts, raw) in pairs.items():
            if any(sha not in tiers for sha in key):
                continue
            previous = library.db.execute("SELECT ts FROM pair_notes WHERE liked_sha256=? AND disliked_sha256=?", key).fetchone()
            if previous is not None and moment <= _timestamp(previous[0], "stored notedAt")[0]:
                continue
            library.db.execute("INSERT OR REPLACE INTO pair_notes VALUES (?, ?, ?, ?)", (*key, raw["note"], ts))
            counts["pairs"] += 1
    return counts


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--data")
    parser.add_argument("--limit", type=int)
    parser.add_argument("--source", action="append", choices=tuple(sources.ADAPTERS))
    parser.add_argument("--category", action="append")
    modes = parser.add_mutually_exclusive_group()
    modes.add_argument("--calibrate", action="store_true")
    modes.add_argument("--import-ratings", type=Path)
    args = parser.parse_args(argv)
    if args.limit is not None and args.limit <= 0:
        parser.error("--limit must be positive")
    try:
        root = common.data_root(args.data)
        if args.import_ratings:
            # Parse first, and require an existing library; imports never create one.
            data = json.loads(common.check_input(args.import_ratings).read_text(encoding="utf-8"))
            if not (root / store.DB_NAME).is_file():
                raise common.Refused("ratings import needs an existing library")
            with store.Store(root) as library:
                counts = import_ratings(library, data)
            print("Import counts: " + json.dumps(counts, sort_keys=True))
            return 0
        model = embed.load_embedder()
        client = net.Client()
        if args.calibrate:
            report = calibrate(model, client)
            print(write_calibration(root, report))
            return 0 if report["passed"] else 1
        content_screen = load_screen(model, root)
        with store.Store(root) as library:
            plan = seeds.load(seeds.ensure(root))
            if args.category and any(name not in plan.categories for name in args.category):
                raise common.Refused("--category: unknown seed category")
            run(library, model, content_screen, client, plan, limit=args.limit, sources=args.source, categories=args.category)
        return 0
    except (common.Refused, OSError, ValueError, seeds.SeedError, net.NetError) as exc:
        print("refused: " + " ".join(str(exc).split()), file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
