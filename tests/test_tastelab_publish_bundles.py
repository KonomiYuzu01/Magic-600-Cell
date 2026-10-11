"""Offline bundle mirrors and provenance refusals using only synthetic libraries."""
from __future__ import annotations

import base64
import copy
import io
import json
import os
import shutil
import stat
import subprocess
import sys
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path
from unittest import mock
from uuid import uuid4

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))
import numpy as np
from tastelab import bundle, common, embed, publish_bundles as publish, sources, store
import test_tastelab_bundle as test_bundle
import test_tastelab_store as test_store


def temporary_folder():
    path = Path(tempfile.gettempdir()) / f"tastelab-publish-{uuid4().hex}"
    path.mkdir()
    return path


class PublishTests(test_store.TempDir):
    # Retain TempDir's private-root patch and cleanup, with the packet's prefix
    # and inherited ACLs instead of TemporaryDirectory's owner-only ACL.
    def setUp(self):
        self.tmp = temporary_folder()
        self.base_patch = mock.patch.object(common, "PRIVATE_BASE", self.tmp)
        self.base_patch.start()
        self.target = temporary_folder()
        self.addCleanup(shutil.rmtree, self.target, ignore_errors=True)
        self.library = store.Store(common.default_data_root())
        self.addCleanup(self.library.close)
        pins = mock.patch.object(embed, "model_pins", return_value={
            "clip-vit-b-16": {"sha256": "d" * 64}, "clip-vit-b-16-merges": {"sha256": "e" * 64}})
        pins.start()
        self.addCleanup(pins.stop)
        self.number = 0
        self.unit = np.eye(512, dtype=np.float32)[0]
        for name in ("LICENSE", "README.md", ".nojekyll", ".git/sentinel"):
            self.write(name, ("public repository: " + name).encode())

    add = test_bundle.BundleTests.add
    manifest = test_bundle.BundleTests.manifest

    def write(self, name, data):
        path = self.target / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)

    def export(self, *, maximum=bundle.MAX_ITEMS):
        folder, counts = bundle.export(self.library, maximum=maximum)
        self.assertIsNotNone(folder, counts)
        self.assertIsNotNone(bundle._complete(folder))
        return folder

    def save_manifest(self, folder, manifest):
        (folder / "manifest.json").write_bytes((json.dumps(manifest, indent=2) + "\n").encode())
        self.assertIsNotNone(bundle._complete(folder), "tampering must remain internally valid")

    def snapshot(self, root=None):
        root = self.target if root is None else root
        tree = {}

        def visit(directory):
            for path in sorted(directory.iterdir()):
                name = path.relative_to(root).as_posix()
                info = os.lstat(path)
                if stat.S_ISLNK(info.st_mode) or path.is_junction():
                    tree[name] = ("link", os.readlink(path))
                elif stat.S_ISDIR(info.st_mode):
                    tree[name] = None
                    visit(path)
                else:
                    tree[name] = path.read_bytes()

        visit(root)
        return tree

    def invoke(self, *args, target=None):
        output, errors = io.StringIO(), io.StringIO()
        with redirect_stdout(output), redirect_stderr(errors):
            code = publish.main(["--target", str(self.target if target is None else target), *args])
        return code, output.getvalue(), errors.getvalue()

    def assert_refused(self, reason, *args):
        before = self.snapshot()
        code, output, errors = self.invoke(*args)
        self.assertEqual(code, 2, (output, errors))
        self.assertEqual(self.snapshot(), before)
        self.assertIn(reason, errors)
        for value in ("Invented Artist", "Invented instrument", "https://", "PRIVATE CREDIT CANARY"):
            self.assertNotIn(value, output + errors)
        return errors

    def test_fresh_mirror_exact_bytes_sorted_lf_index_and_unmanaged_files_untouched(self):
        self.add(title=None)
        first = self.export()
        self.add()
        second = self.export()
        before = self.snapshot()
        times = {name: (self.target / name).stat().st_mtime_ns for name in before}
        code, output, errors = self.invoke()
        self.assertEqual((code, errors), (0, ""))
        ids = sorted([first.name, second.name])
        index = (json.dumps({"format": "tastelab-bundle-index", "version": 1, "bundles": ids}, indent=2) + "\n").encode()
        expected = {"index.json": index}
        for folder in (first, second):
            expected.update({folder.name + "/" + path.name: path.read_bytes() for path in folder.iterdir()})
        actual = {name: data for name, data in self.snapshot(self.target / "bundles").items() if data is not None}
        self.assertEqual(actual, expected)
        self.assertNotIn(b"\r", actual["index.json"])
        self.assertEqual(json.loads(actual["index.json"])["bundles"], ids)
        self.assertEqual(output.splitlines(), [*("added " + ident for ident in ids), "index changed",
                                              f"bundles 2 images 2 bytes {sum(map(len, expected.values()))}"])
        after = self.snapshot()
        self.assertEqual({name: after[name] for name in before}, before)
        self.assertEqual({name: (self.target / name).stat().st_mtime_ns for name in before}, times)

    def test_second_run_has_no_changes_and_check_exits_zero(self):
        self.add()
        self.export()
        self.assertEqual(self.invoke()[0], 0)
        before = self.snapshot()
        times = {name: (self.target / name).stat().st_mtime_ns for name in before}
        for args in ((), ("--check",)):
            code, output, errors = self.invoke(*args)
            self.assertEqual((code, errors), (0, ""))
            self.assertEqual(len(output.splitlines()), 1)
            self.assertTrue(output.startswith("bundles 1 images 1 bytes "))
            self.assertEqual(self.snapshot(), before)
            self.assertEqual({name: (self.target / name).stat().st_mtime_ns for name in before}, times)

    def test_removes_stale_bundles_stray_files_and_empty_directories_ignores_source_extras(self):
        self.add()
        folder = self.export()
        (folder / "private-extra.txt").write_bytes(b"PRIVATE CREDIT CANARY")
        ignored = folder.parent / ".tmp-unfinished"
        ignored.mkdir()
        (ignored / "private.txt").write_bytes(b"private")
        (folder.parent / "other-name").mkdir()
        stale = "19990101T000000Z-00000000"
        self.write("bundles/" + stale + "/nested/old.jpg", b"stale")
        self.write("bundles/stray.txt", b"stray")
        (self.target / "bundles" / "empty").mkdir()
        code, output, errors = self.invoke()
        self.assertEqual((code, errors), (0, ""))
        self.assertEqual(output.splitlines()[:-1], ["removed " + stale, "added " + folder.name, "removed file stray.txt", "index changed"])
        self.assertFalse((self.target / "bundles" / stale).exists())
        self.assertFalse((self.target / "bundles" / "stray.txt").exists())
        self.assertFalse((self.target / "bundles" / "empty").exists())
        self.assertFalse((self.target / "bundles" / folder.name / "private-extra.txt").exists())
        self.assertNotIn("skipped", output)

    def test_changed_bundle_reports_added_changed_and_removed_files(self):
        self.add()
        folder = self.export()
        self.assertEqual(self.invoke()[0], 0)
        target = self.target / "bundles" / folder.name
        manifest = (target / "manifest.json").read_bytes()
        (target / "manifest.json").write_bytes(b"different")
        thumb = next(target.glob("*.jpg"))
        thumb.unlink()
        (target / "stray.txt").write_bytes(b"stray")
        (target / "empty").mkdir()
        code, output, errors = self.invoke()
        self.assertEqual((code, errors), (0, ""))
        self.assertEqual(output.splitlines()[:-1], ["changed " + folder.name])
        self.assertEqual((target / "manifest.json").read_bytes(), manifest)
        self.assertTrue(thumb.is_file())
        self.assertFalse((target / "stray.txt").exists())
        self.assertFalse((target / "empty").exists())

    def test_check_reports_same_plan_without_writing(self):
        self.add()
        self.export()
        before = self.snapshot()
        code, check_output, errors = self.invoke("--check")
        self.assertEqual((code, errors), (1, ""))
        self.assertEqual(self.snapshot(), before)
        code, output, errors = self.invoke()
        self.assertEqual((code, errors), (0, ""))
        self.assertEqual(output, check_output)

    def test_check_detects_removal_only_even_without_bundle_lines(self):
        self.add()
        self.export()
        self.assertEqual(self.invoke()[0], 0)
        self.write("bundles/stray.txt", b"extra")
        before = self.snapshot()
        code, output, errors = self.invoke("--check")
        self.assertEqual((code, errors), (1, ""))
        self.assertEqual(self.snapshot(), before)
        self.assertEqual(output.splitlines()[:-1], ["removed file stray.txt"])

    def test_incomplete_source_bundle_is_skipped_and_not_published(self):
        self.add()
        folder = self.export()
        next(folder.glob("*.jpg")).unlink()
        code, output, errors = self.invoke()
        self.assertEqual((code, errors), (0, ""))
        self.assertEqual(output.splitlines()[:2], ["skipped " + folder.name + " incomplete", "index changed"])
        self.assertEqual(json.loads((self.target / "bundles" / "index.json").read_bytes())["bundles"], [])
        self.assertFalse((self.target / "bundles" / folder.name).exists())

    def test_tier_b_library_row_refused(self):
        sha = self.add()
        self.export()
        with self.library.db:
            self.library.db.execute("UPDATE images SET tier='B' WHERE sha256=?", (sha,))
        with mock.patch.object(sources, "admit", return_value=True):
            self.assert_refused("tier")

    def test_missing_library_row_refused(self):
        sha = self.add()
        self.export()
        self.library.remove_image(sha)
        self.assert_refused("missing")

    def test_no_longer_admitted_row_refused_without_attribution_difference(self):
        sha = self.add()
        self.export()
        with self.library.db:
            self.library.db.execute("UPDATE images SET image_url='https://outside.invalid/x.jpg' WHERE sha256=?", (sha,))
        errors = self.assert_refused("admission")
        self.assertNotIn("attribution", errors)

    def test_licence_check_is_independent_of_source_admission(self):
        self.add()
        self.export()
        original = publish.licences.licence_ok
        # The manifest stays valid; only the row's independent licence proof fails.
        calls = 0

        def licence_ok(*args):
            nonlocal calls
            calls += 1
            return original(*args) if calls == 1 else False

        with mock.patch.object(sources, "admit", return_value=True), \
                mock.patch.object(publish.licences, "licence_ok", side_effect=licence_ok):
            self.assert_refused("licence")

    def test_thb006_other_library_thumbnail_refused_with_consistent_manifest(self):
        self.add()
        self.add()
        folder = self.export()
        manifest = self.manifest(folder)
        first, second = manifest["items"]
        self.assertNotEqual(first["thumbSha256"], second["thumbSha256"])
        for key in ("thumbSha256", "bytes", "width", "height"):
            first[key] = second[key]
        self.save_manifest(folder, manifest)
        errors = self.assert_refused("thumbnail")
        self.assertEqual(errors, f"refused {folder.name}: 1 items fail the class A check (thumbnail)\n")

    def test_thb006_changed_attribution_refused_with_consistent_manifest(self):
        self.add(source="wikimedia", licence="CC-BY-SA-4.0", licence_url="https://creativecommons.org/licenses/by-sa/4.0/",
                 page_url="https://commons.wikimedia.org/wiki/File:Invented.jpg",
                 image_url="https://upload.wikimedia.org/synthetic/invented.jpg")
        folder = self.export()
        original = self.manifest(folder)
        changes = [{"sourceId": "different"}, {"title": "Different title"}, {"credit": "PRIVATE CREDIT CANARY"},
                   {"pageUrl": "https://commons.wikimedia.org/wiki/File:Other.jpg"},
                   {"licence": "CC0-1.0", "licenceUrl": sources.CC0_URL},
                   {"licenceUrl": "https://creativecommons.org/licenses/by-sa/3.0/"},
                   {"source": "openverse", "pageUrl": "https://openverse.org/image/synthetic/"}]
        for change in changes:
            with self.subTest(fields=sorted(change)):
                manifest = copy.deepcopy(original)
                manifest["items"][0].update(change)
                self.save_manifest(folder, manifest)
                errors = self.assert_refused("attribution")
                self.assertEqual(errors, f"refused {folder.name}: 1 items fail the class A check (attribution)\n")

    def test_thb006_changed_embedding_slice_refused_with_consistent_manifest(self):
        self.add()
        self.add(vector=np.eye(512, dtype=np.float32)[1])
        folder = self.export()
        manifest = self.manifest(folder)
        raw = base64.b64decode(manifest["embeddings"]["data"])
        self.assertNotEqual(raw[:1024], raw[1024:])
        manifest["embeddings"]["data"] = base64.b64encode(raw[1024:] * 2).decode()
        self.save_manifest(folder, manifest)
        errors = self.assert_refused("embedding")
        self.assertEqual(errors, f"refused {folder.name}: 1 items fail the class A check (embedding)\n")

    def test_embedding_model_and_missing_library_embedding_refused(self):
        sha = self.add()
        folder = self.export()
        original = self.manifest(folder)
        manifest = copy.deepcopy(original)
        manifest["model"]["id"] = "obsolete-model"
        self.save_manifest(folder, manifest)
        self.assert_refused("model")
        self.save_manifest(folder, original)
        with self.library.db:
            self.library.db.execute("DELETE FROM embeddings WHERE sha256=?", (sha,))
        self.assert_refused("embedding")

    def test_thc001_duplicate_manifest_keys_refused_before_any_write(self):
        self.add()
        folder = self.export()
        original = (folder / "manifest.json").read_bytes()
        for key in (b"credit", b"data"):
            with self.subTest(key=key.decode()):
                marker = b'"' + key + b'": '
                self.assertEqual(original.count(marker), 1)
                (folder / "manifest.json").write_bytes(original.replace(marker, marker + b'"PRIVATE CREDIT CANARY", ' + marker, 1))
                self.assertIsNotNone(bundle._complete(folder), "an ordinary parse ignores the earlier value")
                for args in ((), ("--check",)):
                    errors = self.assert_refused("duplicate keys", *args)
                    self.assertEqual(errors, f"refused {folder.name}: manifest has duplicate keys or is not UTF-8 JSON\n")
        (folder / "manifest.json").write_bytes(original)
        self.assertEqual(self.invoke()[0], 0)

    def test_every_failing_bundle_reported_before_any_write(self):
        shas, folders = [], []
        for _ in range(2):
            shas.append(self.add())
            folders.append(self.export())
        with self.library.db:
            self.library.db.executemany("UPDATE images SET tier='B' WHERE sha256=?", [(sha,) for sha in shas])
        errors = self.assert_refused("class A check", "--check")
        self.assertEqual([line.split()[1].rstrip(":") for line in errors.splitlines()], sorted(f.name for f in folders))

    def test_more_than_100_complete_bundles_refused(self):
        self.add()
        folder = self.export()
        manifest = self.manifest(folder)
        for number in range(100):
            ident = f"20000101T000000Z-{number:08x}"
            destination = folder.parent / ident
            shutil.copytree(folder, destination)
            manifest["id"] = ident
            self.save_manifest(destination, manifest)
        self.assert_refused("100 complete bundles")

    def test_target_validated_before_library_open_and_nojekyll_is_required(self):
        (self.target / ".nojekyll").unlink()
        with mock.patch.object(store, "Store") as constructor:
            self.assert_refused(".nojekyll")
        constructor.assert_not_called()

    def test_repository_root_target_refused_without_creating_anything(self):
        present = (ROOT / "bundles").exists()
        with mock.patch.object(store, "Store") as constructor, mock.patch.object(common, "data_root") as data_root:
            code, _, errors = self.invoke("--check", target=ROOT)
        self.assertEqual(code, 2)
        self.assertIn("source repository", errors)
        self.assertEqual((ROOT / "bundles").exists(), present)
        constructor.assert_not_called()
        data_root.assert_not_called()

    def test_missing_database_refused_before_store_or_folder_creation(self):
        missing = self.tmp / "missing-private"
        with mock.patch.object(common, "PRIVATE_BASE", missing), mock.patch.object(store, "Store") as constructor:
            self.assert_refused("existing library")
        constructor.assert_not_called()
        self.assertFalse(missing.exists())

    def link_case(self, name, *, junction=False, ancestor=False, dangling=False):
        outside = temporary_folder()
        self.addCleanup(shutil.rmtree, outside, ignore_errors=True)
        (outside / "canary").write_bytes(b"outside bytes")
        if ancestor:
            link = self.target / "alias"
            destination = self.target
        else:
            link = self.target / name
            link.parent.mkdir(parents=True, exist_ok=True)
            destination = outside / "missing" if dangling else outside
        if junction:
            if os.name != "nt" or not hasattr(Path, "is_junction"):
                self.skipTest("junction fixtures require Windows and Python 3.12 or later")
            result = subprocess.run(["cmd", "/c", "mklink", "/J", str(link), str(destination)],
                                    capture_output=True, timeout=10, creationflags=subprocess.CREATE_NO_WINDOW)
            if result.returncode:
                self.skipTest(f"temporary junction creation unavailable (exit {result.returncode})")
            self.addCleanup(lambda: link.rmdir() if link.is_junction() else None)
            self.assertTrue(link.is_junction())
        else:
            try:
                link.symlink_to(destination, target_is_directory=True)
            except (OSError, NotImplementedError) as error:
                self.skipTest(f"temporary symbolic link creation unavailable: {error}")
            self.addCleanup(link.unlink, missing_ok=True)
        before = self.snapshot()
        with mock.patch.object(store, "Store") as constructor:
            code, _, errors = self.invoke(target=link if ancestor else self.target)
        self.assertEqual(code, 2)
        self.assertIn("link or junction", errors)
        self.assertEqual(self.snapshot(), before)
        self.assertEqual(self.snapshot(outside), {"canary": b"outside bytes"})
        constructor.assert_not_called()

    def test_symlink_at_bundles_refused(self):
        self.link_case("bundles")

    def test_symlink_inside_bundles_refused(self):
        self.link_case("bundles/nested")

    def test_dangling_symlink_inside_bundles_refused(self):
        self.link_case("bundles/stale", dangling=True)

    def test_symlink_on_way_to_target_refused(self):
        self.link_case("alias", ancestor=True)

    def test_junction_at_bundles_refused(self):
        self.link_case("bundles", junction=True)

    def test_junction_inside_bundles_refused(self):
        self.link_case("bundles/nested", junction=True)

    def test_junction_on_way_to_target_refused(self):
        self.link_case("alias", junction=True, ancestor=True)

    def test_atomic_replacements_and_index_written_after_complete_folders(self):
        self.add()
        folder = self.export()
        replace, replacements = os.replace, []

        def replace_file(source, destination):
            self.assertEqual(Path(source).parent, Path(destination).parent)
            if Path(destination).name == "index.json":
                self.assertIsNotNone(bundle._complete(self.target / "bundles" / folder.name))
            replacements.append(Path(destination).relative_to(self.target / "bundles").as_posix())
            replace(source, destination)

        with mock.patch.object(publish.os, "replace", side_effect=replace_file):
            self.assertEqual(self.invoke()[0], 0)
        self.assertEqual(replacements[-1], "index.json")
        self.assertEqual(list((self.target / "bundles").rglob(".publish-*")), [])

    def test_unknown_argument_exits_two_without_writes(self):
        before = self.snapshot()
        with redirect_stdout(io.StringIO()), redirect_stderr(io.StringIO()), self.assertRaises(SystemExit) as error:
            publish.main(["--target", str(self.target), "--unknown"])
        self.assertEqual(error.exception.code, 2)
        self.assertEqual(self.snapshot(), before)


if __name__ == "__main__":
    unittest.main()
