"""Sync only the committed Taste Lab page into an isolated public checkout."""

import io
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path
from unittest import mock
from uuid import uuid4

ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "tools" / "tastelab" / "sync_public.py"
SOURCE_PATHS = ("tools/tastelab/index.html", "tools/tastelab/page/", "tools/tastelab/core/")
SHA = "a" * 40

sys.path.insert(0, str(ROOT / "tools"))
from tastelab import sync_public as sync  # noqa: E402


def git(*args):
    return subprocess.run(["git", "--no-optional-locks", "-C", str(ROOT), *args],
                          check=True, capture_output=True).stdout


class TemporaryTarget(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.gettempdir()) / f"tastelab-sync-{uuid4().hex}"
        self.tmp.mkdir()
        self.addCleanup(shutil.rmtree, self.tmp)
        self.target = self.tmp / "target"
        self.target.mkdir()
        for name in ("README.md", "LICENSE", ".nojekyll", ".github/workflows/test.yml", ".git/sentinel"):
            self.write(name, ("owned by public repository: " + name).encode())

    def write(self, name, data):
        path = self.target / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)

    def snapshot(self):
        return {path.relative_to(self.target).as_posix(): path.read_bytes() if path.is_file() else None
                for path in self.target.rglob("*")}


class RealSourceTests(TemporaryTarget):
    def test_tracked_source_set_is_scoped_and_contains_no_untracked_paths(self):
        paths = set(sync.tracked_source_paths(ROOT))
        self.assertTrue({"index.html", "page/app.js", "core/space.js"} <= paths)
        self.assertTrue(all(name == "index.html" or name.startswith(("page/", "core/")) for name in paths))
        untracked = {os.fsdecode(path) for path in
                     git("ls-files", "--others", "--exclude-standard", "-z", "--", *SOURCE_PATHS).split(b"\0") if path}
        self.assertFalse({"tools/tastelab/" + name for name in paths} & untracked)

    def test_command_line_syncs_real_source_or_refuses_dirty_source(self):
        before = self.snapshot()
        dirty = git("status", "--porcelain=v1", "-z", "--", *SOURCE_PATHS)
        paths = [os.fsdecode(path) for path in git("ls-files", "-z", "--", *SOURCE_PATHS).split(b"\0") if path]
        result = subprocess.run([sys.executable, "-B", str(SCRIPT), "--target", str(self.target)],
                                capture_output=True, text=True, encoding="utf-8", timeout=30)
        self.assertEqual(result.returncode, 2 if dirty else 0, result.stderr)
        if dirty:
            self.assertIn("clean", result.stderr.lower())
            self.assertEqual(self.snapshot(), before)
        else:
            self.assertEqual(result.stderr, "")
            self.assertIn("source " + git("rev-parse", "--verify", "HEAD").decode().strip(), result.stdout)
            for name in paths:
                relative = Path(name).relative_to("tools/tastelab")
                self.assertEqual((self.target / relative).read_bytes(), (ROOT / name).read_bytes())
            after = self.snapshot()
            self.assertEqual({name: after[name] for name in before}, before)
            self.assertIn(f"added {len(paths)} changed 0 removed 0 unchanged 0", result.stdout)


class SyncTests(TemporaryTarget):
    def setUp(self):
        super().setUp()
        self.source = {"index.html": b"<html>\r\n</html>\n", "page/app.js": b"app\nline\n",
                       "core/space.js": b"space\r\nline\r\n"}

    def invoke(self, *args, source=None):
        with mock.patch.object(sync, "read_source", return_value=(SHA, self.source if source is None else source)), \
                redirect_stdout(io.StringIO()) as output, redirect_stderr(io.StringIO()) as errors:
            code = sync.main(["--target", str(self.target), *args])
        return code, output.getvalue(), errors.getvalue()

    def fixture_source(self):
        root = self.tmp / "source"
        for name, data in self.source.items():
            path = root / "tools" / "tastelab" / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)
        return root

    def source_git(self, status=b"", paths=None):
        tracked = self.source if paths is None else paths
        def run(root, *args):
            if args[0] == "ls-files":
                return b"".join(("tools/tastelab/" + name).encode() + b"\0" for name in tracked)
            if args[0] == "status":
                return status
            if args[0] == "rev-parse":
                return SHA.encode() + b"\n"
            raise AssertionError(f"unexpected Git command: {args}")
        return run

    def symlink(self, path, destination, directory=False):
        try:
            path.symlink_to(destination, target_is_directory=directory)
        except (OSError, NotImplementedError) as error:
            self.skipTest(f"temporary symbolic links are unavailable: {error}")
        self.addCleanup(path.unlink, missing_ok=True)

    def junction(self, path, destination):
        if os.name != "nt" or not hasattr(Path, "is_junction"):
            self.skipTest("junctions require Windows and Python 3.12 or later")
        result = subprocess.run(["cmd", "/c", "mklink", "/J", str(path), str(destination)],
                                capture_output=True, timeout=10, creationflags=subprocess.CREATE_NO_WINDOW)
        if result.returncode:
            self.skipTest(f"the platform could not create a temporary junction (exit {result.returncode})")
        self.addCleanup(lambda: path.rmdir() if path.is_junction() else None)
        self.assertTrue(path.is_junction())

    def test_fresh_target_gets_every_file_and_planning_writes_nothing(self):
        self.source["page/assets/data.bin"] = b"\x00\xff\r\n\n"
        before = self.snapshot()
        plan = sync.compute_plan(self.source, self.target)
        self.assertEqual(self.snapshot(), before)
        self.assertEqual(plan.changes, (("added", "core/space.js"), ("added", "index.html"),
                                        ("added", "page/app.js"), ("added", "page/assets/data.bin")))
        self.assertEqual(plan.unchanged, 0)
        sync.apply_plan(self.source, self.target, plan)
        for name, data in self.source.items():
            self.assertEqual((self.target / name).read_bytes(), data)

    def test_changed_files_keep_crlf_and_lf_bytes_exactly(self):
        for name in self.source:
            self.write(name, b"old\n")
        plan = sync.compute_plan(self.source, self.target)
        self.assertEqual({action for action, _ in plan.changes}, {"changed"})
        sync.apply_plan(self.source, self.target, plan)
        for name, data in self.source.items():
            self.assertEqual((self.target / name).read_bytes(), data)

    def test_stale_files_are_removed_and_empty_managed_directories_pruned(self):
        self.write("page/obsolete/deep/stale.js", b"stale")
        (self.target / "core" / "empty").mkdir(parents=True)
        code, output, errors = self.invoke()
        self.assertEqual((code, errors), (0, ""))
        self.assertFalse((self.target / "page" / "obsolete").exists())
        self.assertFalse((self.target / "core" / "empty").exists())
        self.assertIn("removed page/obsolete/deep/stale.js", output)
        self.assertTrue((self.target / "page" / "app.js").is_file())

    def test_public_repository_files_are_untouched_and_second_run_is_unchanged(self):
        before = self.snapshot()
        self.assertEqual(self.invoke()[0], 0)
        after = self.snapshot()
        self.assertEqual({name: after[name] for name in before}, before)
        code, output, errors = self.invoke()
        self.assertEqual((code, errors), (0, ""))
        self.assertEqual(output, f"source {SHA}\nadded 0 changed 0 removed 0 unchanged 3\n")
        self.assertEqual(self.snapshot(), after)

    def test_git_marker_may_be_a_file(self):
        (self.target / ".git" / "sentinel").unlink()
        (self.target / ".git").rmdir()
        self.write(".git", b"gitdir: owned-by-public-repository\n")
        before = (self.target / ".git").read_bytes()
        self.assertEqual(self.invoke()[0], 0)
        self.assertEqual((self.target / ".git").read_bytes(), before)

    def test_check_reports_sorted_changes_without_writing_and_returns_zero_after_sync(self):
        self.write("index.html", b"old")
        self.write("page/old.js", b"stale")
        before = self.snapshot()
        code, output, errors = self.invoke("--check")
        expected = (f"source {SHA}\nadded core/space.js\nchanged index.html\nadded page/app.js\n"
                    "removed page/old.js\nadded 2 changed 1 removed 1 unchanged 0\n")
        self.assertEqual((code, output, errors), (1, expected, ""))
        self.assertEqual(self.snapshot(), before)
        self.assertEqual(self.invoke(), (0, expected, ""))
        after = self.snapshot()
        self.assertEqual(self.invoke("--check"), (0, f"source {SHA}\nadded 0 changed 0 removed 0 unchanged 3\n", ""))
        self.assertEqual(self.snapshot(), after)

    def test_missing_target_and_file_target_are_refused_without_writes(self):
        file_target = self.tmp / "file"
        file_target.write_bytes(b"untouched")
        before = self.snapshot()
        for target in (self.tmp / "missing", file_target):
            with self.subTest(target=target), redirect_stdout(io.StringIO()), redirect_stderr(io.StringIO()):
                self.assertEqual(sync.main(["--target", str(target)]), 2)
            self.assertEqual(self.snapshot(), before)
            self.assertEqual(file_target.read_bytes(), b"untouched")
        self.assertFalse((self.tmp / "missing").exists())

    def test_each_missing_target_marker_is_refused_without_writes(self):
        for name in (".git", "LICENSE", ".nojekyll"):
            path = self.target / name
            if path.is_dir():
                (path / "sentinel").unlink()
                path.rmdir()
            else:
                path.unlink()
            before = self.snapshot()
            with self.subTest(marker=name):
                code, _, errors = self.invoke()
                self.assertEqual(code, 2)
                self.assertIn(name, errors)
                self.assertEqual(self.snapshot(), before)
            self.write(".git/sentinel" if name == ".git" else name, b"restored marker")

    def test_target_at_source_root_or_inside_it_is_refused_without_writes(self):
        before = self.snapshot()
        for root in (self.target, self.tmp):
            with self.subTest(root=root), mock.patch.object(sync, "ROOT", root):
                code, _, errors = self.invoke()
                self.assertEqual(code, 2)
                self.assertIn("inside", errors)
                self.assertEqual(self.snapshot(), before)

    def test_empty_source_is_refused_without_writes(self):
        before = self.snapshot()
        code, _, errors = self.invoke(source={})
        self.assertEqual(code, 2)
        self.assertIn("empty", errors)
        self.assertEqual(self.snapshot(), before)

    def test_unknown_arguments_are_refused_without_writes(self):
        before = self.snapshot()
        with self.assertRaises(SystemExit) as error:
            self.invoke("--unknown")
        self.assertEqual(error.exception.code, 2)
        self.assertEqual(self.snapshot(), before)

    def test_failed_atomic_replace_preserves_old_bytes_and_cleans_temporary_file(self):
        self.write("index.html", b"old bytes")
        source = {"index.html": b"new bytes\r\n"}
        before = self.snapshot()
        plan = sync.compute_plan(source, self.target)
        with mock.patch.object(sync.os, "replace", side_effect=OSError("injected replace failure")), \
                self.assertRaisesRegex(OSError, "injected"):
            sync.apply_plan(source, self.target, plan)
        self.assertEqual(self.snapshot(), before)

    def test_source_reader_uses_only_git_tracked_paths_and_preserves_bytes(self):
        root = self.fixture_source()
        (root / "tools/tastelab/page/ignored.log").write_bytes(b"not tracked")
        with mock.patch.object(sync, "_git", side_effect=self.source_git()):
            commit, source = sync.read_source(root)
        self.assertEqual((commit, source), (SHA, self.source))

    def test_git_calls_disable_optional_locks_and_only_read_source_repository(self):
        with mock.patch.object(sync.subprocess, "run", wraps=subprocess.run) as run:
            sync.tracked_source_paths(ROOT)
        command = run.call_args.args[0]
        self.assertEqual(command, ["git", "--no-optional-locks", "-C", str(ROOT), "ls-files", "-z", "--", *SOURCE_PATHS])

    def test_all_dirty_source_statuses_name_paths_and_refuse_before_writes(self):
        root = self.fixture_source()
        path = "tools/tastelab/page/app.js"
        cases = [(status.encode() + b" " + path.encode() + b"\0", (path,))
                 for status in (" M", "M ", " D", "D ", "??")]
        cases.append((b"R  tools/tastelab/page/new name.js\0tools/tastelab/page/old name.js\0",
                      ("tools/tastelab/page/new name.js", "tools/tastelab/page/old name.js")))
        before = self.snapshot()
        for status, names in cases:
            with self.subTest(status=status), mock.patch.object(sync, "ROOT", root), \
                    mock.patch.object(sync, "_git", side_effect=self.source_git(status)), \
                    redirect_stdout(io.StringIO()), redirect_stderr(io.StringIO()) as errors:
                self.assertEqual(sync.main(["--target", str(self.target)]), 2)
            for name in names:
                self.assertIn(name, errors.getvalue())
            self.assertEqual(self.snapshot(), before)

    def test_empty_git_source_set_is_refused_without_writes(self):
        before = self.snapshot()
        with mock.patch.object(sync, "_git", side_effect=self.source_git(paths=[])), \
                redirect_stdout(io.StringIO()), redirect_stderr(io.StringIO()) as errors:
            self.assertEqual(sync.main(["--target", str(self.target)]), 2)
        self.assertIn("empty", errors.getvalue())
        self.assertEqual(self.snapshot(), before)

    def test_source_file_symbolic_link_is_refused_before_writes(self):
        root = self.fixture_source()
        path = root / "tools/tastelab/page/app.js"
        path.unlink()
        outside = self.tmp / "outside.js"
        outside.write_bytes(b"outside bytes")
        self.symlink(path, outside)
        before = self.snapshot()
        with mock.patch.object(sync, "ROOT", root), mock.patch.object(sync, "_git", side_effect=self.source_git()), \
                redirect_stdout(io.StringIO()), redirect_stderr(io.StringIO()) as errors:
            self.assertEqual(sync.main(["--target", str(self.target)]), 2)
        self.assertIn("link", errors.getvalue().lower())
        self.assertEqual(self.snapshot(), before)
        self.assertEqual(outside.read_bytes(), b"outside bytes")

    def test_managed_target_file_link_including_dangling_link_is_refused(self):
        outside = self.tmp / "outside.js"
        outside.write_bytes(b"outside bytes")
        for destination in (outside, self.tmp / "missing.js"):
            with self.subTest(destination=destination):
                link = self.target / "index.html"
                self.symlink(link, destination)
                code, _, errors = self.invoke()
                self.assertEqual(code, 2)
                self.assertIn("link", errors.lower())
                self.assertTrue(link.is_symlink())
                self.assertFalse((self.target / "core").exists())
                self.assertEqual(outside.read_bytes(), b"outside bytes")
                link.unlink()

    def test_stale_managed_target_file_link_is_refused_before_any_write(self):
        self.write("index.html", b"old bytes")
        outside = self.tmp / "outside.js"
        outside.write_bytes(b"outside bytes")
        (self.target / "page").mkdir()
        self.symlink(self.target / "page/stale.js", outside)
        code, _, errors = self.invoke()
        self.assertEqual(code, 2)
        self.assertIn("link", errors.lower())
        self.assertEqual((self.target / "index.html").read_bytes(), b"old bytes")
        self.assertEqual(outside.read_bytes(), b"outside bytes")
        self.assertFalse((self.target / "core").exists())

    def test_managed_directories_and_intermediate_symbolic_links_are_refused(self):
        self.write("index.html", b"old bytes")
        outside = self.tmp / "outside"
        outside.mkdir()
        (outside / "canary").write_bytes(b"outside bytes")
        for name in ("page", "core", "page/nested"):
            with self.subTest(path=name):
                link = self.target / name
                link.parent.mkdir(parents=True, exist_ok=True)
                self.symlink(link, outside, directory=True)
                code, _, errors = self.invoke()
                self.assertEqual(code, 2)
                self.assertIn("link", errors.lower())
                self.assertEqual((self.target / "index.html").read_bytes(), b"old bytes")
                self.assertEqual(list(outside.iterdir()), [outside / "canary"])
                link.unlink()

    def test_managed_directories_and_intermediate_junctions_are_refused(self):
        self.write("index.html", b"old bytes")
        outside = self.tmp / "outside"
        outside.mkdir()
        (outside / "canary").write_bytes(b"outside bytes")
        for name in ("page", "core", "page/nested"):
            with self.subTest(path=name):
                link = self.target / name
                link.parent.mkdir(parents=True, exist_ok=True)
                self.junction(link, outside)
                code, _, errors = self.invoke()
                self.assertEqual(code, 2)
                self.assertIn("link", errors.lower())
                self.assertEqual((self.target / "index.html").read_bytes(), b"old bytes")
                self.assertEqual(list(outside.iterdir()), [outside / "canary"])
                link.rmdir()

    def test_source_and_managed_file_junctions_are_refused_before_writes(self):
        root = self.fixture_source()
        outside = self.tmp / "outside"
        outside.mkdir()
        (outside / "canary").write_bytes(b"outside bytes")
        self.write("page/app.js", b"old bytes")
        for location in ("source", "target"):
            with self.subTest(location=location):
                link = root / "tools/tastelab/index.html" if location == "source" else self.target / "index.html"
                if link.is_file():
                    link.unlink()
                self.junction(link, outside)
                with mock.patch.object(sync, "ROOT", root), mock.patch.object(sync, "_git", side_effect=self.source_git()), \
                        redirect_stdout(io.StringIO()), redirect_stderr(io.StringIO()) as errors:
                    self.assertEqual(sync.main(["--target", str(self.target)]), 2)
                self.assertIn("link", errors.getvalue().lower())
                self.assertEqual((self.target / "page/app.js").read_bytes(), b"old bytes")
                self.assertEqual(list(outside.iterdir()), [outside / "canary"])
                self.assertFalse((self.target / "core").exists())
                link.rmdir()
                if location == "source":
                    link.write_bytes(self.source["index.html"])


if __name__ == "__main__":
    unittest.main()
