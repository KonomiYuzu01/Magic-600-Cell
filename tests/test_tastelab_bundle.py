"""Bundle admission, atomic publication and page-rating round trips with synthetic images."""
from __future__ import annotations

import base64
import hashlib
import io
import json
import sqlite3
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

    def page_export(self, ratings=(), pairs=()):
        return {"kind": "tastelab-export", "version": 3, "ignored": {"unrelated": True}, "images": {
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
