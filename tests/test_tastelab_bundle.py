"""Bundle admission, atomic publication and page-rating round trips with synthetic images."""
from __future__ import annotations

import base64
import hashlib
import io
import json
import shutil
import sqlite3
import subprocess
import sys
import unittest
from contextlib import redirect_stderr, redirect_stdout
from datetime import datetime, timezone
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))
import numpy as np
from PIL import Image
from tastelab import bundle, common, embed, fetch, images, sources, store
import test_tastelab_store as test_store

NODE = shutil.which("node")
PAGE_BUNDLE = (ROOT / "tools" / "tastelab" / "page" / "bundle.js").as_uri()
PAGE_IMAGES_UI = (ROOT / "tools" / "tastelab" / "page" / "images-ui.js").as_uri()
# Rates three images through the page's reducers (no note, a note with a tab, a cleared
# note), adds a pair note, and prints the page's version 3 export.
PAGE_RATINGS_SCRIPT = """
const ui = await import(process.argv[1]);
const [a, b, c] = JSON.parse(process.argv[2]), at = "2026-10-04T07:00:00.000Z";
let state = ui.createRatingState();
state = ui.reduceRating(state, { type: "rate", imageId: a, verdict: "like", ratedAt: at });
state = ui.reduceRating(state, { type: "rate", imageId: b, verdict: "dislike", ratedAt: at });
state = ui.reduceRating(state, { type: "note", imageId: b, note: ui.cleanNote("  Too\\tbusy  "), ratedAt: at });
state = ui.reduceRating(state, { type: "rate", imageId: c, verdict: "like", ratedAt: at });
state = ui.reduceRating(state, { type: "note", imageId: c, note: "Kept", ratedAt: at });
state = ui.reduceRating(state, { type: "note", imageId: c, note: ui.cleanNote(" "), ratedAt: at });
const pair = ui.pairDocument({ likedImageId: a, dislikedImageId: b, note: ui.cleanNote("Calmer\\tlines"), notedAt: at }).body;
const base = { kind: "tastelab-export", version: 2, presets: [] };
process.stdout.write(JSON.stringify(ui.assembleImageExport(base, { bundles: [], ratings: [], pairs: [],
  ratingChanges: state.changes, pairChanges: new Map([["pair", pair]]) })));
"""
# Reads {file name: base64 bytes} from stdin, validates the folder as the page does, and
# prints the outcome, the item ids and the decoded vectors.
PAGE_SCRIPT = """
const { validateBundle } = await import(process.argv[1]);
let input = "";
for await (const chunk of process.stdin) input += chunk;
const files = new Map(Object.entries(JSON.parse(input)).map(([name, data]) => [name, new Uint8Array(Buffer.from(data, "base64"))]));
const result = await validateBundle(new TextDecoder().decode(files.get("manifest.json")), files);
process.stdout.write(JSON.stringify(result.ok
  ? { ok: true, imageIds: result.bundle.items.map((item) => item.imageId), vectors: Array.from(result.bundle.vectors) }
  : result));
"""


class BundleTests(test_store.TempDir):
    def setUp(self):
        super().setUp()
        self.library = store.Store(common.default_data_root())
        self.addCleanup(self.library.close)
        self.pins = mock.patch.object(embed, "model_pins", return_value={
            "clip-vit-b-16": {"sha256": "d" * 64}, "clip-vit-b-16-merges": {"sha256": "e" * 64}})
        self.pins.start()
        self.addCleanup(self.pins.stop)
        self.number = 0
        self.unit = np.eye(512, dtype=np.float32)[0]

    def add(self, *, tier="A", vector=None, embedded=True, model=None, thumb=None, **over):
        self.number += 1
        picture = Image.new("RGB", (800, 600), (self.number * 31 % 256, 80, 150))
        output = io.BytesIO()
        picture.save(output, format="PNG")
        original = output.getvalue()
        sha = hashlib.sha256(original).hexdigest()
        metadata = dict(source="met", licence="CC0-1.0", attribution="Invented Artist", tier=tier, category="synthetic",
                        width=800, height=600, source_id=str(self.number), title="Invented instrument",
                        page_url="https://www.metmuseum.org/art/collection/search/" + str(self.number),
                        image_url="https://images.metmuseum.org/synthetic/" + str(self.number) + ".png", licence_url=sources.CC0_URL)
        metadata.update(over)
        meta = store.ImageMeta(sha, **metadata)
        self.assertEqual(self.library.add_image(meta, thumb or images.thumbnail_jpeg(picture), original, "png"), "stored")
        if embedded:
            self.library.put_embeddings(model or embed.MODEL_ID, 512, [(sha, embed.to_blob(self.unit if vector is None else vector))])
        return sha

    def manifest(self, folder):
        return json.loads((folder / "manifest.json").read_text(encoding="utf-8"))

    def page_export(self, ratings=(), pairs=(), *, version=3):
        return {"kind": "tastelab-export", "version": version, "ignored": {"unrelated": True}, "images": {
            "bundles": [], "ratings": list(ratings), "pairs": list(pairs)}}

    def rating(self, sha, verdict="like", ts="2027-01-01T00:00:00Z", note=None):
        return {"imageId": sha, "verdict": verdict, "note": note, "ratedAt": ts}

    def pair(self, liked, disliked, ts="2027-01-01T00:00:00Z", note="Invented comparison"):
        return {"likedImageId": liked, "dislikedImageId": disliked, "note": note, "notedAt": ts}

    def test_manifest_exact_keys_jpeg_limits_little_endian_unit_embeddings(self):
        sha = self.add()
        folder, counts = bundle.export(self.library)
        manifest = self.manifest(folder)
        self.assertEqual(counts["exported"], 1)
        self.assertRegex(folder.name, r"^[0-9]{8}T[0-9]{6}Z-[0-9a-f]{8}$")
        self.assertEqual(manifest.keys(), {"format", "version", "id", "createdAt", "model", "items", "embeddings"})
        self.assertEqual(manifest["model"], {"id": embed.MODEL_ID, "weightsSha256": "d" * 64})
        self.assertEqual(manifest["items"][0].keys(), bundle.ITEM_KEYS)
        self.assertEqual(manifest["embeddings"].keys(), {"dtype", "dim", "count", "data"})
        self.assertEqual(manifest["items"][0]["imageId"], sha)
        item = manifest["items"][0]
        data = (folder / (item["thumbSha256"] + ".jpg")).read_bytes()
        self.assertTrue(data.startswith(b"\xff\xd8\xff"))
        self.assertEqual(hashlib.sha256(data).hexdigest(), item["thumbSha256"])
        self.assertNotEqual(sha, item["thumbSha256"])
        self.assertLessEqual(len(data), 204_800)
        with Image.open(io.BytesIO(data)) as image:
            self.assertEqual(image.size, (512, 384))
        raw = base64.b64decode(manifest["embeddings"]["data"], validate=True)
        self.assertEqual(len(raw), 1024)
        values = np.frombuffer(raw, dtype="<f2").reshape(1, 512)
        np.testing.assert_array_equal(values[0], self.unit)
        self.assertIsNotNone(bundle._complete(folder))

    def test_bundle_thumbnail_strips_metadata_from_raw_and_stored_inputs(self):
        for name, original, canary in test_store.metadata_images():
            decoded = images.decode(original)
            try:
                stored = images.thumbnail_jpeg(decoded.image)
            finally:
                decoded.image.close()
            for boundary, data in (("raw", original), ("stored", stored)):
                with self.subTest(metadata=name, boundary=boundary):
                    path = self.tmp / (name + "-" + boundary)
                    path.write_bytes(data)
                    output, width, height = bundle._thumbnail(path)
                    self.assertEqual((width, height), (96, 96))
                    self.assertNotIn(canary, output)
                    with Image.open(io.BytesIO(output)) as thumbnail:
                        self.assertLessEqual(set(thumbnail.info), {"jfif", "jfif_version", "jfif_unit", "jfif_density"})
                        self.assertEqual(len(thumbnail.getexif()), 0)

    @unittest.skipIf(NODE is None, "node is not on PATH")
    def test_page_validator_accepts_an_exported_bundle(self):
        # The page's validateBundle (tools/tastelab/page/bundle.js) must accept what this exporter writes.
        rng = np.random.default_rng(7)
        vectors = [v / np.linalg.norm(v) for v in rng.standard_normal((3, 512)).astype(np.float32)]
        shas = [self.add(vector=vectors[0]),
                self.add(vector=vectors[1], source="wikimedia", licence="CC-BY-SA-4.0",
                         licence_url="https://creativecommons.org/licenses/by-sa/4.0/",
                         page_url="https://commons.wikimedia.org/wiki/File:Invented_mechanism.jpg",
                         image_url="https://upload.wikimedia.org/synthetic/mechanism.jpg"),
                self.add(vector=vectors[2], source="nasa", licence="US-Gov-PD",
                         licence_url="https://www.nasa.gov/nasa-brand-center/images-and-media/",
                         page_url="https://images.nasa.gov/details/PIA00001",
                         image_url="https://images-assets.nasa.gov/image/PIA00001/PIA00001~orig.jpg")]
        folder, counts = bundle.export(self.library)
        self.assertEqual(counts["exported"], 3)
        files = {path.name: base64.b64encode(path.read_bytes()).decode("ascii") for path in folder.iterdir()}
        run = subprocess.run([NODE, "--input-type=module", "-e", PAGE_SCRIPT, PAGE_BUNDLE], input=json.dumps(files),
                             capture_output=True, text=True, encoding="utf-8", timeout=120)
        self.assertEqual(run.returncode, 0, run.stderr[-2000:])
        page = json.loads(run.stdout)
        self.assertTrue(page["ok"], page)
        manifest = self.manifest(folder)
        self.assertEqual(page["imageIds"], [item["imageId"] for item in manifest["items"]])
        self.assertEqual(sorted(page["imageIds"]), sorted(shas))
        expected = np.frombuffer(base64.b64decode(manifest["embeddings"]["data"]), dtype="<f2").astype(np.float32)
        np.testing.assert_array_equal(np.array(page["vectors"], dtype=np.float32), expected)

    def test_tier_b_canary_and_forced_private_class_a_never_exported(self):
        public = self.add()
        private = self.add(tier="B", source="archive", licence="private-reference", title="CLASS B CANARY",
                           attribution="PRIVATE CANARY CREDIT", source_id="synthetic-private",
                           page_url="https://archive.org/details/synthetic-private", image_url="https://archive.org/download/synthetic-private/x.png",
                           licence_url=sources.ADAPTERS["archive"].terms_url)
        forced = self.add(tier="B", title="FORCED PRIVATE CANARY")
        self.library.add_rating(public, "like", "private-session", note="PRIVATE NOTE CANARY")
        folder, counts = bundle.export(self.library)
        manifest = self.manifest(folder)
        self.assertEqual(counts["exported"], 1)
        self.assertEqual([item["imageId"] for item in manifest["items"]], [public])
        text = (folder / "manifest.json").read_text(encoding="utf-8")
        for value in (private, forced, "CANARY", "private-session", str(self.tmp)):
            self.assertNotIn(value, text)

    def test_rechecks_admission_and_counts_missing_or_invalid_embeddings(self):
        good = self.add()
        self.add(page_url="https://outside.invalid/item")
        self.add(licence="CC-BY-NC-4.0", licence_url="https://creativecommons.org/licenses/by-nc/4.0/")
        self.add(source_id="x\n")
        self.add(embedded=False)
        self.add(model="obsolete-model")
        self.add(vector=np.full(512, np.nan))
        self.add(vector=self.unit * 1.02)
        self.add(thumb=b"invalid image")
        folder, counts = bundle.export(self.library)
        self.assertEqual([item["imageId"] for item in self.manifest(folder)["items"]], [good])
        self.assertEqual(counts, dict(exported=1, already_exported=0, invalid_metadata=3,
                                     missing_embedding=2, invalid_embedding=2, invalid_thumbnail=1))

    def test_large_thumbnail_is_resized_and_reencoded_under_caps(self):
        pixels = np.random.default_rng(600).integers(0, 256, (1024, 1024, 3), dtype=np.uint8)
        output = io.BytesIO()
        Image.fromarray(pixels).save(output, format="JPEG", quality=100)
        self.assertGreater(len(output.getvalue()), bundle.THUMB_MAX_BYTES)
        self.add(thumb=output.getvalue())
        folder, counts = bundle.export(self.library)
        item = self.manifest(folder)["items"][0]
        self.assertEqual((item["width"], item["height"]), (512, 512))
        self.assertLessEqual(item["bytes"], bundle.THUMB_MAX_BYTES)
        self.assertEqual(counts["invalid_thumbnail"], 0)

    def test_interrupted_export_after_thumbnail_is_cleaned_on_retry(self):
        sha = self.add()
        write_text = Path.write_text
        def interrupted(path, *args, **kwargs):
            if path.name == "manifest.json":
                self.assertEqual(len(list(path.parent.glob("*.jpg"))), 1)
                raise RuntimeError("injected failure before manifest")
            return write_text(path, *args, **kwargs)
        with mock.patch.object(Path, "write_text", interrupted), self.assertRaisesRegex(RuntimeError, "injected"):
            bundle.export(self.library)
        folders = list((self.tmp / "bundles").iterdir())
        self.assertEqual(len(folders), 1)
        self.assertTrue(folders[0].name.startswith(".tmp-"))
        self.assertFalse((folders[0] / "manifest.json").exists())
        folder, counts = bundle.export(self.library)
        self.assertFalse(folders[0].exists())
        self.assertEqual(list((self.tmp / "bundles").iterdir()), [folder])
        self.assertEqual(self.manifest(folder)["items"][0]["imageId"], sha)
        self.assertEqual(counts["exported"], 1)

    def test_complete_manifests_prevent_reexport_and_empty_bundle(self):
        self.add()
        first, _ = bundle.export(self.library)
        previous = (first / "manifest.json").read_bytes()
        second, counts = bundle.export(self.library)
        self.assertIsNone(second)
        self.assertEqual((counts["exported"], counts["already_exported"]), (0, 1))
        self.assertEqual((first / "manifest.json").read_bytes(), previous)
        self.assertEqual(list((self.tmp / "bundles").iterdir()), [first])

    def test_bundle_id_collision_never_overwrites(self):
        moment = datetime(2026, 10, 4, 12, 0, 0, tzinfo=timezone.utc)
        with mock.patch.object(bundle, "datetime") as clock, mock.patch.object(bundle, "uuid4", return_value=SimpleNamespace(hex="0123abcd" * 4)):
            clock.now.return_value = moment
            self.add()
            first, _ = bundle.export(self.library)
            before = {p.name: p.read_bytes() for p in first.iterdir()}
            self.add()
            with self.assertRaisesRegex(common.Refused, "overwrite"):
                bundle.export(self.library)
            self.assertEqual({p.name: p.read_bytes() for p in first.iterdir()}, before)

    def test_maximum_range_and_no_incomplete_final_folder(self):
        for value in (0, 301, 1.5, True):
            with self.subTest(value=value), self.assertRaises(ValueError):
                bundle.export(self.library, maximum=value)
        first, second = self.add(), self.add()
        folder, counts = bundle.export(self.library, maximum=1)
        self.assertEqual(counts["exported"], 1)
        self.assertEqual(len(self.manifest(folder)["items"]), 1)
        folder2, counts2 = bundle.export(self.library, maximum=1)
        self.assertEqual(counts2["exported"], 1)
        self.assertEqual({self.manifest(folder)["items"][0]["imageId"], self.manifest(folder2)["items"][0]["imageId"]}, {first, second})

    def test_original_image_id_roundtrip_bundle_to_import_ratings_cli(self):
        sha = self.add()
        folder, _ = bundle.export(self.library)
        item = self.manifest(folder)["items"][0]
        data = self.page_export([self.rating(item["imageId"], note="Invented preference")])
        data["images"]["bundles"] = [{"bundleId": folder.name, "items": 1}]
        path = self.tmp / "page-export.json"
        path.write_text(json.dumps(data), encoding="utf-8")
        with mock.patch.object(embed, "load_embedder", side_effect=AssertionError("ratings need no model")), \
                redirect_stdout(io.StringIO()) as output, redirect_stderr(io.StringIO()) as errors:
            result = fetch.main(["--data", str(self.library.root), "--import-ratings", str(path)])
        self.assertEqual((result, errors.getvalue()), (0, ""))
        self.assertIn('"ratings": 1', output.getvalue())
        self.assertEqual([(r.sha256, r.verdict, r.note) for r in self.library.ratings()], [(sha, "like", "Invented preference")])
        self.assertFalse(self.library.has_image(item["thumbSha256"]))

    def test_version_three_ignores_stray_love_keys(self):
        a, b = self.add(), self.add()
        data = self.page_export([dict(self.rating(a), love=True), dict(self.rating(b, "dislike"), love="stray")])
        self.assertEqual(fetch.import_ratings(self.library, data), {"ratings": 2, "pairs": 0, "unknown": []})
        self.assertEqual([(r.verdict, r.love) for r in self.library.ratings()], [("like", False), ("dislike", False)])

    def test_version_four_love_round_trip_and_newest_timestamp(self):
        a, b = self.add(), self.add()
        data = self.page_export([
            dict(self.rating(a, ts="2027-01-02T00:00:00Z", note="Warm gold"), love=True),
            dict(self.rating(a), love=False), dict(self.rating(b, "dislike"), love=False),
            dict(self.rating("f" * 64), love=True)], [self.pair(a, b)], version=4)
        path = self.tmp / "version-four.json"
        path.write_text(json.dumps(data), encoding="utf-8")
        with mock.patch.object(embed, "load_embedder", side_effect=AssertionError("import needs no model")), \
                redirect_stdout(io.StringIO()) as output, redirect_stderr(io.StringIO()) as errors:
            self.assertEqual(fetch.main(["--data", str(self.library.root), "--import-ratings", str(path)]), 0)
        self.assertEqual(errors.getvalue(), "")
        self.assertIn('"ratings": 2', output.getvalue())
        self.assertEqual([(r.sha256, r.verdict, r.love, r.note) for r in self.library.ratings()],
                         [(a, "like", True, "Warm gold"), (b, "dislike", False, None)])
        self.assertEqual(self.library.db.execute("SELECT note FROM pair_notes").fetchone()[0], "Invented comparison")
        self.assertEqual(fetch.import_ratings(self.library, data), {"ratings": 0, "pairs": 0, "unknown": ["f" * 64]})
        newer = dict(self.rating(a, ts="2027-01-02T02:00:00+01:00"), love=False)
        self.assertEqual(fetch.import_ratings(self.library, self.page_export([newer], version=4))["ratings"], 1)
        self.assertIs(self.library.ratings()[-1].love, False)

    def test_version_four_validates_every_love_before_any_write(self):
        a, b = self.add(), self.add()
        self.library.add_rating(a, "skip", "fixture")
        before = [tuple(r) for r in self.library.db.execute("SELECT * FROM ratings ORDER BY id")]
        invalid = [self.rating(b)]  # love is required even when it would be false.
        invalid += [dict(self.rating(b), love=value) for value in (None, 0, 1, "true", [], {})]
        invalid.append(dict(self.rating(b, "dislike"), love=True))
        for bad in invalid:
            data = self.page_export([dict(self.rating(a), love=True), bad], [self.pair(a, b)], version=4)
            with self.subTest(bad=bad), self.assertRaisesRegex(common.Refused, "love"):
                fetch.import_ratings(self.library, data)
            self.assertEqual([tuple(r) for r in self.library.db.execute("SELECT * FROM ratings ORDER BY id")], before)
            self.assertEqual(self.library.db.execute("SELECT COUNT(*) FROM pair_notes").fetchone()[0], 0)

    def test_export_version_refusal_names_both_supported_versions(self):
        for version in (1, 2, 5, True, 4.0, "4"):
            with self.subTest(version=version), self.assertRaisesRegex(
                    common.Refused, "expected tastelab-export version 3 or 4 with images"):
                fetch.import_ratings(self.library, self.page_export(version=version))
            self.assertEqual(self.library.ratings(), [])

    def test_import_refuses_whole_file_for_tier_b_rating_or_pair(self):
        a, b = self.add(), self.add(tier="B")
        for data in (self.page_export([self.rating(a), self.rating(b)]),
                     self.page_export([self.rating(a)], [self.pair(a, b)])):
            with self.assertRaisesRegex(common.Refused, b):
                fetch.import_ratings(self.library, data)
            self.assertEqual(self.library.ratings(), [])
            self.assertEqual(self.library.db.execute("SELECT COUNT(*) FROM pair_notes").fetchone()[0], 0)
        path = self.tmp / "private-rating.json"
        path.write_text(json.dumps(self.page_export([self.rating(b)])), encoding="utf-8")
        with redirect_stdout(io.StringIO()), redirect_stderr(io.StringIO()) as errors:
            result = fetch.main(["--data", str(self.library.root), "--import-ratings", str(path)])
        self.assertEqual(result, 2)
        self.assertIn(b, errors.getvalue())

    def test_import_validates_every_rating_and_pair_before_any_write(self):
        a, b = self.add(), self.add()
        for field, value in (("imageId", a + "\n"), ("verdict", "skip"), ("note", ""), ("note", "x" * 141),
                             ("note", "x\n"), ("ratedAt", "not a timestamp"), ("ratedAt", "2026-01-01T00:00:00")):
            bad = self.rating(b)
            bad[field] = value
            with self.subTest(field=field, value=value), self.assertRaises(common.Refused):
                fetch.import_ratings(self.library, self.page_export([self.rating(a), bad]))
            self.assertEqual(self.library.ratings(), [])
        for field, value in (("note", None), ("note", "x" * 281), ("notedAt", None), ("likedImageId", [])):
            bad = self.pair(a, b)
            bad[field] = value
            with self.subTest(field=field), self.assertRaises(common.Refused):
                fetch.import_ratings(self.library, self.page_export([self.rating(a)], [bad]))
            self.assertEqual(self.library.ratings(), [])

    @unittest.skipIf(NODE is None, "node is not on PATH")
    def test_importer_accepts_the_page_rating_export(self):
        # The page's export (tools/tastelab/page/images-ui.js) must pass fetch.import_ratings unchanged.
        a, b, c = self.add(), self.add(), self.add()
        run = subprocess.run([NODE, "--input-type=module", "-e", PAGE_RATINGS_SCRIPT, PAGE_IMAGES_UI, json.dumps([a, b, c])],
                             capture_output=True, text=True, encoding="utf-8", timeout=120)
        self.assertEqual(run.returncode, 0, run.stderr[-2000:])
        data = json.loads(run.stdout)
        self.assertEqual([rating["note"] for rating in data["images"]["ratings"]], [None, "Too busy", None])
        self.assertEqual(fetch.import_ratings(self.library, data), {"ratings": 3, "pairs": 1, "unknown": []})
        self.assertEqual(sorted((r.sha256, r.verdict, r.note) for r in self.library.ratings()),
                         sorted([(a, "like", None), (b, "dislike", "Too busy"), (c, "like", None)]))
        self.assertEqual([row[0] for row in self.library.db.execute("SELECT note FROM pair_notes")], ["Calmer lines"])

    @unittest.skipIf(NODE is None, "node is not on PATH")
    def test_importer_accepts_added_replaced_and_cleared_page_notes(self):
        a = self.add()
        script = """
const ui = await import(process.argv[1]), imageId = process.argv[2], at = "2026-10-04T07:00:00.000Z";
let state = ui.reduceRating(ui.createRatingState(), { type: "rate", imageId, verdict: "like", ratedAt: at });
const base = { kind: "tastelab-export", version: 2, presets: [] }, exports = [];
const capture = () => exports.push(ui.assembleImageExport(base, { bundles: [], ratings: [...state.ratings.values()], pairs: [] }));
capture();
for (const note of ["Added explanation", "Replacement explanation", ""]) {
  state = ui.reduceRating(state, { type: "note", imageId, note, ratedAt: at });
  capture();
}
process.stdout.write(JSON.stringify(exports));
"""
        run = subprocess.run([NODE, "--input-type=module", "-e", script, PAGE_IMAGES_UI, a],
                             capture_output=True, text=True, encoding="utf-8", timeout=120)
        self.assertEqual(run.returncode, 0, run.stderr[-2000:])
        exports = json.loads(run.stdout)
        self.assertEqual(len(exports), 4)
        for data, note in zip(exports, [None, "Added explanation", "Replacement explanation", None]):
            with self.subTest(note=note):
                self.assertEqual(fetch.import_ratings(self.library, data), {"ratings": 1, "pairs": 0, "unknown": []})
                self.assertEqual([(r.sha256, r.verdict, r.note) for r in self.library.ratings()], [(a, "like", note)])

    def test_import_newest_timestamp_wins_and_unknown_ids_are_reported(self):
        a, b = self.add(), self.add()
        unknown = "f" * 64
        data = self.page_export([self.rating(a, "dislike", "2027-01-02T00:00:00Z"), self.rating(a), self.rating(unknown)],
                                [self.pair(a, b, "2027-01-02T00:00:00+00:00", "Newest"), self.pair(a, b), self.pair(a, unknown)])
        counts = fetch.import_ratings(self.library, data)
        self.assertEqual(counts, {"ratings": 1, "pairs": 1, "unknown": [unknown]})
        self.assertEqual(self.library.ratings()[0].verdict, "dislike")
        self.assertEqual(self.library.db.execute("SELECT note FROM pair_notes").fetchone()[0], "Newest")
        self.assertEqual(fetch.import_ratings(self.library, self.page_export([self.rating(a)], [self.pair(a, b)])),
                         {"ratings": 0, "pairs": 0, "unknown": []})
        offset = self.rating(a, "like", "2027-01-02T02:00:00+01:00")
        self.assertEqual(fetch.import_ratings(self.library, self.page_export([offset]))["ratings"], 1)
        self.assertEqual(self.library.ratings()[0].verdict, "like")

    def test_import_transaction_rolls_back_ratings_when_pair_insert_fails(self):
        a, b = self.add(), self.add()
        self.library.db.execute("CREATE TRIGGER no_pair BEFORE INSERT ON pair_notes BEGIN SELECT RAISE(ABORT, 'injected'); END")
        with self.assertRaises(sqlite3.DatabaseError):
            fetch.import_ratings(self.library, self.page_export([self.rating(a)], [self.pair(a, b)]))
        self.assertEqual(self.library.ratings(), [])

    def test_bundle_cli_prints_folder_and_counts_without_loading_model(self):
        self.add()
        with mock.patch.object(embed, "load_embedder", side_effect=AssertionError("export needs no model")), \
                redirect_stdout(io.StringIO()) as output, redirect_stderr(io.StringIO()) as errors:
            result = bundle.main(["--max", "1"])
        self.assertEqual((result, errors.getvalue()), (0, ""))
        self.assertIn(str(self.tmp / "bundles"), output.getvalue())
        self.assertIn('"exported": 1', output.getvalue())


if __name__ == "__main__":
    unittest.main()
