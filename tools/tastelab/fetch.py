"""Fetch approved sources into a private library, calibrate, or import page ratings.

Candidates and bytes stay in memory through rights, decoding, embedding,
screening, relevance and deduplication. Only admitted images are committed;
discarded cases leave aggregate counts only. No owner-folder or URL ingestion.
The fetch applies `screen.Screen.guard` (owner decisions, 9 October 2026): class A
keeps nudity, swimwear, suggestive content and gore, and both classes discard what
the minor-protection checks flag. `--calibrate` measures the full screen; its
report is not a precondition of the fetch.
`--calibrate --diagnose` also writes reports/calibration-cases.json: the group,
query, source, id, title, keywords and score of each missed or discarded
calibration case, never its image or a URL (owner choice, 9 October 2026).
Calibration skips document files, whose thumbnail is a first page that their
title does not describe, and negatives labelled as nude, suggestive or
swimwear (owner choice, 9 October 2026).
Class A skips document files and, from a source whose credits vary by record
(Wikimedia, Openverse), keeps at most SERIES_MAX images of one credit per (category,
term), so that no uploader's, aggregator's or institution's series fills a term. The
Met, AIC and NASA adapters give every record their own institution's credit, where a
cap would only shrink the source's share of a term, so they are not capped. Both
checks run before the download, and class B is unchanged (owner delegation,
10 October 2026).
`--proposals` fetches the terms ticked in the rating window: each may hold
PROPOSAL_IMAGES images in all, across runs (see `proposal_plan`). Every fetch first
stores the missing probe text vectors, which the window's taste map and term
proposals read.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import re
import sys
import unicodedata
from collections import Counter
from dataclasses import asdict, dataclass, replace
from datetime import datetime, timezone
from pathlib import Path
from urllib.parse import unquote, urlsplit
from uuid import uuid4

if not __package__:
    sys.path[0] = str(Path(__file__).resolve().parents[1])

import numpy as np

from tastelab import common, embed, images, licences, net, screen, seeds, sources, store

DOWNLOAD_MAX_BYTES = 20 * 1024 * 1024
IMAGE_TYPES = ("image/jpeg", "image/png", "image/webp", "image/gif")
# Openverse's thumbnail endpoint answers 406 unless the request allows any type; the content type is checked after download.
IMAGE_ACCEPT = ", ".join(IMAGE_TYPES) + ", */*;q=0.1"
NEAR_DUPLICATE = 0.97
SERIES_MAX = 5      # class A images of one source and attribution per (category, term)
PROPOSAL_IMAGES = 40    # images one accepted proposal may hold, counted over every category
CALIBRATION_REPORT = "calibration.json"
CALIBRATION_CASES = "calibration-cases.json"


@dataclass
class Outcome:
    kind: str
    sha256: str | None = None


class Pipeline:
    def __init__(self, store, embedder, screen, client, *, relevance_min=0.0, near_duplicate=NEAR_DUPLICATE, phrase=None,
                 series_max=SERIES_MAX):
        self.store, self.embedder, self.screen, self.client = store, embedder, screen, client
        self.relevance_min, self.near_duplicate, self.series_max = relevance_min, near_duplicate, series_max
        self.phrase = phrase or (lambda text: text)

    def ingest_candidate(self, candidate, *, category, query, tier, adapter):
        if candidate.source != adapter.name or not sources.admit(candidate, tier):
            return Outcome("rejected")
        if self.store.has_seen(candidate.source, candidate.source_id):
            return Outcome("duplicate")
        if tier == "A" and _document(candidate):
            return Outcome("document")
        if (tier == "A" and adapter.series
                and self.store.series_count(category, query, adapter.name, candidate.attribution) >= self.series_max):
            return Outcome("series")
        try:
            response = self.client.get(candidate.image_url, hosts=adapter.hosts, min_interval=adapter.min_interval,
                                       max_bytes=DOWNLOAD_MAX_BYTES, accept=IMAGE_ACCEPT, extra_headers=adapter.headers())
        except net.NetError:
            return Outcome("error")
        content_type = response.headers.get("content-type", "").split(";", 1)[0].lower()
        if content_type and content_type not in IMAGE_TYPES:
            return Outcome("invalid")
        return self.ingest_bytes(response.body, source=candidate.source, source_id=candidate.source_id,
                                 image_url=candidate.image_url, page_url=candidate.page_url, licence=candidate.licence,
                                 licence_url=candidate.licence_url, attribution=candidate.attribution, title=candidate.title,
                                 keywords=candidate.keywords, tier=tier, category=category, query=query)

    def ingest_bytes(self, data, *, source, licence, attribution, tier, category, source_id=None, page_url=None,
                     image_url=None, licence_url=None, title=None, keywords=(), query=None):
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
                label = " ".join([title or "", *keywords])
                discard = invalid or self.screen.guard(vectors, labels=[label], explicit=tier == "A")[0]
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


def _selected(sources) -> set:
    """The adapters a run searches: `sources`, or by default every class A adapter."""
    return set(sources if sources is not None else (name for name, a in globals()["sources"].ADAPTERS.items() if a.tier == "A"))


def run(store, embedder, screen, client, plan, *, limit=None, sources=None, categories=None, phrase=None, term_caps=None,
        term_cursors=False, log=print):
    """Fetch the selected categories up to their targets.

    The terms of a category share its target, and no single category, term or source fills the library: the run visits
    every (category, term, source) in turn, interleaved across categories, and each visit stores at most one image.
    A first pass fills every term up to its share of its category's target; a second pass lets the remaining terms
    fill what exhausted terms left. A visit keeps the rest of its search page in memory for its next visit, and each
    entry walks the pages from its own position within the run, starting where the stored cursor stood when the run
    first looked it up. The stored cursor advances only when an entry has fully handled the page it points at, so an
    interrupted run resumes on that page and the seen filter skips what was handled. When several entries share a
    cursor key, the stored cursor follows the furthest of them. `term_caps` maps a term to the most images it may hold,
    counted over every category. With `term_cursors`, every term keeps a stored cursor of its own, also where an
    adapter serves several queries from one listing (Demozoo), so a term that another term overtakes keeps its place.
    The proposal fetch uses both."""
    selected = _selected(sources)
    pipeline = Pipeline(store, embedder, screen, client, relevance_min=plan.relevance_min, phrase=phrase)
    counts = Counter()
    failed = set()      # (source, cursor key) that raised a network error; not retried within this run
    pages = {}          # slot -> [cursor, next cursor, unhandled candidates] of the entry's current page
    positions = {}      # slot -> (cursor, done) of the entry's next page within this run
    starts = {}         # (source, cursor key) -> the stored (cursor, done) when this run first looked it up

    def stopped():
        return store.is_full() or limit is not None and counts["stored"] >= limit

    def held(entry, cap):
        category, term, _ = entry
        queries = store.query_counts()
        if term_caps is not None and term.text in term_caps and \
                sum(number for (_, query), number in queries.items() if query == term.text) >= term_caps[term.text]:
            return True
        return (store.category_counts().get(category.name, 0) >= category.target
                or queries.get((category.name, term.text), 0) >= cap)

    def visit(entry, cap):
        """Store at most one image for `entry`: "more", "exhausted" or "stop"."""
        category, term, name = entry
        adapter = globals()["sources"].ADAPTERS[name]
        query, key = term.query(name), _cursor_key(adapter, term, term_cursors)
        slot = (category.name, term.text, name, query)
        if slot not in pages:
            if (name, key) not in starts:
                starts[(name, key)] = store.cursor(name, key)
            cursor, done = positions.get(slot, starts[(name, key)])
            if done or (name, key) in failed:
                return "exhausted"
            try:
                candidates, next_cursor = adapter.search(client, query, cursor, seen=lambda ident: store.has_seen(name, ident))
            except net.NetError:
                counts["error"] += 1
                failed.add((name, key))
                return "exhausted"
            pages[slot] = [cursor, next_cursor, list(candidates)]
        cursor, next_cursor, pending = pages[slot]
        while pending:
            if stopped():
                return "stop"
            if held(entry, cap):
                return "more"
            result = pipeline.ingest_candidate(pending.pop(0), category=category.name, query=term.text,
                                               tier=plan.tier_for(category.name, name), adapter=adapter)
            counts[result.kind] += 1
            if result.kind == "full":
                return "stop"
            if result.kind == "stored":
                break
        if pending:
            return "more"
        del pages[slot]
        done = next_cursor is None or next_cursor == cursor
        positions[slot] = (next_cursor, done)
        if store.cursor(name, key) == (cursor, False):   # unless another entry with this cursor key moved it
            store.set_cursor(name, key, next_cursor, done)
        return "exhausted" if done else "more"

    chosen = [category for category in plan.categories.values()
              if (categories is None or category.name in categories) and category.terms]
    lists = [[(category, term, name) for term in category.terms
              for name in plan.active_sources(category.name) if name in selected] for category in chosen]
    order = [entries[i] for i in range(max(map(len, lists), default=0)) for entries in lists if i < len(entries)]
    caps = {category.name: (math.ceil(category.target / len(category.terms)), category.target) for category in chosen}
    for phase in (0, 1):
        active = list(order)
        while active:
            for entry in list(active):
                if stopped():
                    return dict(counts)
                cap = caps[entry[0].name][phase]
                if held(entry, cap):
                    active.remove(entry)
                    continue
                outcome = visit(entry, cap)
                if outcome == "stop":
                    return dict(counts)
                if outcome == "exhausted":
                    active.remove(entry)
    log("Fetch counts: " + json.dumps(dict(counts), sort_keys=True))
    return dict(counts)


def _cursor_key(adapter, term, per_term):
    """The stored cursor key of `term` on `adapter`: the adapter's own, or with `per_term` one the term has alone."""
    key = adapter.cursor_key(term.query(adapter.name))
    return f"{key} | term: {term.text}" if per_term else key


def proposal_plan(store, embedder, plan, proposals, *, sources=None):
    """The plan narrowed to `proposals`, and the proposed terms it covers.

    An adjacent proposal joins its own category. A probe proposal has none and joins the category whose class A images
    lie closest, on average, to the term's text vector; a category that forces tier B never takes one. Only categories
    that the selected sources can search take part, and a category that forces tier B only when the caller names the
    sources. Each category keeps its proposals as its only terms. A proposal may hold PROPOSAL_IMAGES images in all,
    whatever their category, so a retry only fills what earlier runs left; `fetch_proposals` passes that cap to `run`.
    The category target leaves room for exactly that. A proposal that no category can take is not covered."""
    selected = _selected(sources)
    open_categories = [name for name in plan.categories if selected.intersection(plan.active_sources(name))
                       and (sources is not None or plan.categories[name].tier != "B")]
    placeable = [name for name in open_categories if plan.categories[name].tier != "B"]
    members = {}
    for meta in store.images(tier="A"):
        if meta.category in placeable:
            members.setdefault(meta.category, []).append(meta.sha256)
    centres = {}
    for name, shas in members.items():
        blobs = store.embeddings(embedder.model_id, shas)
        if blobs:
            centres[name] = embed.normalise(embed.from_blobs(list(blobs.values()), embedder.dim).mean(axis=0))
    chosen = {}
    for item in proposals:
        category = item["category"]
        if category is None and centres:
            text = embed.normalise(np.asarray(embedder.embed_texts([item["term"]]), dtype=np.float32)[0])
            category = max(centres, key=lambda name: float(centres[name] @ text))
        if category in open_categories:
            chosen.setdefault(category, []).append(item["term"])
    counts, held = store.category_counts(), _term_totals(store)
    categories = {name: replace(plan.categories[name], terms=tuple(seeds.Term(term) for term in terms), adjacent=(),
                                target=counts.get(name, 0) + sum(max(0, PROPOSAL_IMAGES - held.get(term, 0))
                                                                 for term in terms))
                  for name, terms in chosen.items()}
    return replace(plan, categories=categories), [term for terms in chosen.values() for term in terms]


def _term_totals(store) -> Counter:
    """Stored images per query text, counted over every category."""
    totals = Counter()
    for (_, query), number in store.query_counts().items():
        totals[query] += number
    return totals


def fetch_proposals(store, embedder, screen, client, plan, *, limit=None, sources=None, log=print):
    """Fetch the proposals the owner ticked in the rating window. Returns None when none is accepted, else the run's
    counts.

    A covered proposal becomes fetched once it holds PROPOSAL_IMAGES images or its own search (`run`'s term_cursors) is
    finished on every source its category uses; a limit, a full store or a failed search leaves it accepted, and the
    next run resumes it where it stopped, also when another term has finished a listing the two share. A
    candidate whose download still fails after the client's own retries (net.Client) is passed over, as in a seed
    fetch, and does not hold the proposal back."""
    accepted = store.proposals("accepted")
    if not accepted:
        return None
    narrowed, covered = proposal_plan(store, embedder, plan, accepted, sources=sources)
    caps = {term: PROPOSAL_IMAGES for term in covered}
    counts = (run(store, embedder, screen, client, narrowed, limit=limit, sources=sources, term_caps=caps,
                  term_cursors=True, log=log) if covered else {})
    selected, held, fetched = _selected(sources), _term_totals(store), []
    for name, category in narrowed.categories.items():
        names = [source for source in narrowed.active_sources(name) if source in selected]
        for term in category.terms:
            adapters = [globals()["sources"].ADAPTERS[source] for source in names]
            if held.get(term.text, 0) >= PROPOSAL_IMAGES or \
                    all(store.cursor(adapter.name, _cursor_key(adapter, term, True))[1] for adapter in adapters):
                fetched.append(term.text)
    for term in fetched:
        store.set_proposal_status(term, "fetched")
    log(f"Proposals: {len(fetched)} fetched, {len(accepted) - len(fetched)} still accepted")
    return counts


def _probe_sha():
    return hashlib.sha256(json.dumps(screen.UNSAFE_PROBES + screen.SAFE_PROBES).encode()).hexdigest()


def _adult_subject(candidate, *, positive):
    text = " ".join([candidate.title or "", *candidate.keywords])
    if re.search(r"\b(?:child(?:ren)?|boy|girl|infant|teen(?:ager)?|adolescent|minor|youth)\b", text, re.I):
        return False
    adult = re.search(r"\b(?:adult|woman|women|man|men)\b", text, re.I)
    people = re.search(r"\b(?:person|people|portrait|nude|naked|body|dancer)\b", text, re.I)
    return bool(adult) if positive or people else True


# After NFKC normalization, a word of a title or keyword with a slash, a backslash, an "@" or a colon, or with a dot
# followed by more text, is replaced by "[link]": every URL, scheme, domain, address and path has one. Over-redacting
# words such as "Study:", "and/or" or "c.1890" is accepted.
_LINK = re.compile(r"[/\\@:／＼＠：∕⁄]|[.．。｡](?=\S)")


def _plain(text, limit=200):
    words = unicodedata.normalize("NFKC", (text or "")[:4 * limit]).split()
    return " ".join("[link]" if _LINK.search(word) else word for word in words)[:limit]


# Calibration labels and class A fetches: a document file's thumbnail is its first page, which the document's title
# does not describe;
# a negative carrying a nudity, suggestive or swimwear label is not an ordinary image (gore's pattern is left out, as
# it also matches botanical words such as "bloodroot").
_DOCUMENT_SUFFIXES = (".pdf", ".djvu", ".djv")
_LABELLED_NEGATIVE = re.compile("|".join(f"(?:{screen.CALIBRATION_GROUPS[name]['match']})"
                                         for name in ("nudity", "suggestive", "underwear_swimwear")), re.I)


def _document(candidate):
    return unquote(urlsplit(candidate.page_url or "").path).lower().endswith(_DOCUMENT_SUFFIXES)


def calibrate(embedder, client, *, log=print, cases=None):
    """With a `cases` list, each case's metadata (never its image or URL) is appended to it for --diagnose."""
    content_screen = screen.Screen(embedder)
    groups, negatives, known, skipped = {}, [], set(), Counter()

    def collect(queries, target, match=None, group_name="negative"):
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
                    if _document(candidate) or match is None and _LABELLED_NEGATIVE.search(label):
                        skipped["document" if _document(candidate) else "labelled_negative"] += 1
                        known.add(ident)
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
                    if cases is not None:
                        cases.append({"group": group_name, "query": query, "source": name, "id": _plain(candidate.source_id, 100),
                                      "title": _plain(candidate.title), "keywords": [_plain(k, 60) for k in candidate.keywords[:20]],
                                      "score": score if math.isfinite(score) and 0 <= score <= 1 else None})
                if next_cursor is None or next_cursor in cursors:
                    break
                cursors.add(next_cursor)
                cursor = next_cursor
        return values

    for name, group in screen.CALIBRATION_GROUPS.items():
        groups[name] = collect(group["queries"], screen.GROUP_SIZE, group["match"], name)
    for query in screen.NEGATIVE_QUERIES:
        negatives.extend(collect((query,), screen.NEGATIVE_SIZE))
    result = screen.gate(groups, negatives)
    if not result.passed:
        result = screen.gate(groups, negatives, threshold=screen.LOWERED[0], strict_threshold=screen.LOWERED[1])
    report = {"model": embedder.model_id, "weightsSha256": getattr(embedder, "weights_sha256", ""),
              "probesSha256": _probe_sha(), "time": common.now_iso(), **asdict(result)}
    log("Calibration counts: " + json.dumps({"passed": report["passed"], "groups": report["groups"],
                                            "negatives": report["negatives"]}, sort_keys=True))
    log("Calibration skipped: " + json.dumps({"document": skipped["document"],
                                              "labelled_negative": skipped["labelled_negative"]}, sort_keys=True))
    return report


def diagnostic_cases(cases):
    """Positives under the approved threshold or unscorable, and negatives at or over the lowest strict threshold:
    the cases that any threshold of the gate counts as missed or discarded, lowest score first per group."""
    lowest = min(screen.STRICT_THRESHOLD, screen.LOWERED[1])

    def kept(case):
        if case["group"] == "negative":
            return case["score"] is not None and case["score"] >= lowest
        return case["score"] is None or case["score"] < screen.THRESHOLD

    return sorted((case for case in cases if kept(case)),
                  key=lambda case: (case["group"], -1.0 if case["score"] is None else case["score"]))


def write_calibration(store_root, report, name=CALIBRATION_REPORT):
    root = common.data_root(store_root)
    folder = common.check_input(root / "reports", kind="calibration report")
    path = common.check_input(folder / name, kind="calibration report")
    folder.mkdir(parents=True, exist_ok=True)
    temporary = folder / (".part-" + uuid4().hex)
    try:
        temporary.write_text(json.dumps(report, allow_nan=False, indent=2) + "\n", encoding="utf-8")
        os.replace(temporary, path)
    finally:
        temporary.unlink(missing_ok=True)
    return path


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
            or data.get("version") not in (3, 4) or not isinstance(data.get("images"), dict)):
        raise common.Refused("ratings export: expected tastelab-export version 3 or 4 with images")
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
        if data["version"] == 4:
            if type(raw.get("love")) is not bool:
                raise common.Refused(f"{field}.love: expected a bool")
            if raw["love"] and raw["verdict"] != "like":
                raise common.Refused(f"{field}.love: true requires verdict like")
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
            library.db.execute("INSERT INTO ratings(sha256, verdict, love, note, ts, session) VALUES (?, ?, ?, ?, ?, 'page-export')",
                               (sha, raw["verdict"], raw["love"] if data["version"] == 4 else False, raw["note"], ts))
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
    modes.add_argument("--upgrade-library", action="store_true", help="back up and upgrade a schema-2 library to schema 3")
    modes.add_argument("--proposals", action="store_true", help="fetch the terms ticked in the rating window")
    parser.add_argument("--diagnose", action="store_true",
                        help="with --calibrate: also write the metadata of missed and discarded cases")
    args = parser.parse_args(argv)
    if args.limit is not None and args.limit <= 0:
        parser.error("--limit must be positive")
    if args.diagnose and not args.calibrate:
        parser.error("--diagnose needs --calibrate")
    if args.proposals and args.category:
        parser.error("--proposals takes the categories from the proposals")
    try:
        root = common.data_root(args.data)
        if args.upgrade_library:
            result = store.upgrade_library(root)
            print(f"Upgraded the library to schema 3: {result['ratings']} ratings kept; backup backups/{result['backup']}")
            return 0
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
            cases = [] if args.diagnose else None
            report = calibrate(model, client, cases=cases)
            print(write_calibration(root, report))
            if cases is not None:
                print(write_calibration(root, {"time": report["time"], "cases": diagnostic_cases(cases)}, CALIBRATION_CASES))
            return 0 if report["passed"] else 1
        content_screen = screen.Screen(model)   # the guard needs no calibration report (owner decision, 9 October 2026)
        with store.Store(root) as library:
            plan = seeds.load(seeds.ensure(root))
            if args.category and any(name not in plan.categories for name in args.category):
                raise common.Refused("--category: unknown seed category")
            embed.embed_probe_texts(library, model, root)
            if args.proposals:
                if fetch_proposals(library, model, content_screen, client, plan, limit=args.limit,
                                   sources=args.source) is None:
                    print("No accepted proposals.")
                return 0
            run(library, model, content_screen, client, plan, limit=args.limit, sources=args.source, categories=args.category)
        return 0
    except (common.Refused, OSError, ValueError, seeds.SeedError, net.NetError) as exc:
        print("refused: " + " ".join(str(exc).split()), file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
