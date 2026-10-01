"""Tests for tools/checkout_bytes.py and the repository's no-conversion rule."""
from __future__ import annotations

import contextlib
import glob
import io
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))
import checkout_bytes  # noqa: E402

FILES = {"lf.txt": b"a\nb\n", "crlf.txt": b"a\r\nb\r\n", "mixed.txt": b"a\r\nb\n", "edited.txt": b"x\ny\n",
         "bin.dat": b"\0\r\n\1", "gone.txt": b"g\n", "ignored/[glob].txt": b"i\n", ".gitignore": b"ignored/\n"}


def git(root, *args, check=True):
    return subprocess.run(["git", "-c", "core.autocrlf=false", "-c", "user.name=t", "-c", "user.email=t@example.invalid",
                           *args], cwd=root, check=check, capture_output=True)


def crlf(data):
    return data.replace(b"\n", b"\r\n")


def kept(root, name):
    """Contents of every backed-up original of a path, from all runs."""
    git_dir = Path(git(root, "rev-parse", "--absolute-git-dir").stdout.decode().strip())
    return sorted(p.read_bytes() for p in (git_dir / "checkout-bytes").glob("*/" + glob.escape(name)))


class CheckoutBytesTests(unittest.TestCase):
    def tmp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        return Path(tmp.name)

    def repo(self, attributes=True):
        root = self.tmp() / "repo"
        root.mkdir()
        git(root, "init", "-q")
        git(root, "config", "core.autocrlf", "false")
        files = dict(FILES, **({".gitattributes": b"* -text\n"} if attributes else {}))
        for name, data in files.items():
            (root / name).parent.mkdir(exist_ok=True)
            (root / name).write_bytes(data)
        git(root, "add", "--force", ".")
        git(root, "commit", "-qm", "init")
        return root

    def run_main(self, root, *argv):
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            code = checkout_bytes.main(list(argv), root=root)
        return code, out.getvalue()

    def index(self, root):
        return git(root, "ls-files", "-s").stdout

    def test_clean_tree_passes(self):
        root = self.repo()
        self.assertEqual(self.run_main(root)[0], 0)
        self.assertEqual(self.run_main(root, "--fix")[0], 0)

    def test_only_exact_line_ending_conversion_is_repaired(self):
        root = self.repo()
        edits = {"lf.txt": crlf(FILES["lf.txt"]),        # the conversion: repaired
                 "crlf.txt": b"a\nb\n",                  # not what autocrlf produces: left alone
                 "mixed.txt": b"a\r\nb\r\n",             # a mixed blob is never converted by Git: left alone
                 "edited.txt": b"x\r\nchanged\r\n",      # a real edit
                 "bin.dat": b"\0\n\1"}                   # a binary edit that only looks like line endings
        for name, data in edits.items():
            (root / name).write_bytes(data)
        (root / "gone.txt").unlink()
        before = self.index(root)
        code, out = self.run_main(root)
        self.assertEqual(code, 1)
        self.assertIn("needs repair: lf.txt", out)
        self.assertEqual((root / "lf.txt").read_bytes(), edits["lf.txt"], "check mode must not write")
        code, out = self.run_main(root, "--fix")
        self.assertEqual(code, 0, out)
        self.assertEqual((root / "lf.txt").read_bytes(), FILES["lf.txt"])
        for name, data in edits.items():
            if name != "lf.txt":
                with self.subTest(name=name):
                    self.assertEqual((root / name).read_bytes(), data)
                    self.assertIn("left unchanged (other edits, links or missing): " + name, out)
        self.assertFalse((root / "gone.txt").exists(), "missing files are reported, not restored")
        self.assertEqual(self.index(root), before)
        self.assertEqual(git(root, "diff", "--cached", "--name-only").stdout, b"", "nothing is ever staged")

    def test_a_converted_checkout_becomes_clean(self):
        source = self.repo(attributes=False)
        clone = self.tmp() / "clone"
        subprocess.run(["git", "-c", "core.autocrlf=true", "clone", "-q", str(source), str(clone)], check=True, capture_output=True)
        subprocess.run(["git", "config", "core.autocrlf", "true"], cwd=clone, check=True)
        self.assertEqual((clone / "lf.txt").read_bytes(), crlf(FILES["lf.txt"]), "precondition: converted checkout")
        (clone / ".gitattributes").write_bytes(b"* -text\n")  # the rule arriving in an existing tree
        (clone / "edited.txt").write_bytes(b"x\r\nchanged\r\n")
        code, out = self.run_main(clone, "--fix")
        self.assertEqual(code, 0, out)
        for name in ("lf.txt", "ignored/[glob].txt", ".gitignore"):
            self.assertEqual((clone / name).read_bytes(), FILES[name])
        status = subprocess.run(["git", "status", "--porcelain"], cwd=clone, check=True, capture_output=True, text=True).stdout
        self.assertEqual(sorted(status.splitlines()), [" M edited.txt", "?? .gitattributes"])
        self.assertEqual(git(clone, "diff", "--cached", "--name-only").stdout, b"")
        self.assertEqual(self.run_main(clone)[0], 0)

    def test_an_edit_saved_during_the_repair_is_kept(self):
        root = self.repo()
        converted = crlf(FILES["lf.txt"])
        (root / "lf.txt").write_bytes(converted)
        saved = checkout_bytes.classify

        def edit_after_classification(*args):
            result = saved(*args)
            (root / "lf.txt").write_bytes(b"new work\r\n")
            return result

        checkout_bytes.classify = edit_after_classification
        try:
            code, out = self.run_main(root, "--fix")
        finally:
            checkout_bytes.classify = saved
        self.assertEqual(code, 1)
        self.assertIn("changed while the repair ran", out)
        self.assertEqual((root / "lf.txt").read_bytes(), b"new work\r\n")

        (root / "lf.txt").write_bytes(converted)
        real_rename = os.rename

        def save_after_move(src, dst):
            real_rename(src, dst)
            if Path(src).name == "lf.txt":
                Path(src).write_bytes(b"saved meanwhile\n")  # an editor recreates the file

        checkout_bytes.os.rename = save_after_move
        try:
            code, out = self.run_main(root, "--fix")
        finally:
            checkout_bytes.os.rename = real_rename
        self.assertEqual(code, 1)
        self.assertEqual((root / "lf.txt").read_bytes(), b"saved meanwhile\n", "checkout-index never overwrites it")
        # A POSIX rollback in the first run also keeps its backup name, so look for this run's copy.
        self.assertIn(converted, kept(root, "lf.txt"), "the original is kept")
        self.assertEqual(git(root, "diff", "--cached", "--name-only").stdout, b"")

    def test_a_file_recreated_with_the_same_bytes_is_never_replaced(self):
        root = self.repo()
        converted = crlf(FILES["lf.txt"])
        (root / "lf.txt").write_bytes(converted)
        real_rename = os.rename

        def recreate_after_move(src, dst):
            real_rename(src, dst)
            if Path(src).name == "lf.txt":
                Path(src).write_bytes(converted)  # checkout-index refuses; rollback must not replace it

        checkout_bytes.os.rename = recreate_after_move
        try:
            code, out = self.run_main(root, "--fix")
        finally:
            checkout_bytes.os.rename = real_rename
        self.assertEqual(code, 1)
        self.assertIn("the original is kept in the backup directory", out)
        self.assertEqual((root / "lf.txt").read_bytes(), converted)
        self.assertEqual(kept(root, "lf.txt"), [converted])

    def test_every_run_gets_its_own_backup_directory(self):
        root = self.repo()
        first, second = checkout_bytes.backup_root(root), checkout_bytes.backup_root(root)
        self.assertNotEqual(first, second)
        self.assertTrue(first.is_dir() and second.is_dir())

    def test_originals_are_kept_and_never_deleted(self):
        root = self.repo()
        converted = crlf(FILES["lf.txt"])
        (root / "lf.txt").write_bytes(converted)
        real_run = subprocess.run

        def write_through_old_handle(cmd, *args, **kw):
            result = real_run(cmd, *args, **kw)
            if cmd[:2] == ["git", "checkout-index"]:
                for original in (Path(git(root, "rev-parse", "--absolute-git-dir").stdout.decode().strip())
                                 / "checkout-bytes").glob("*/lf.txt"):
                    original.write_bytes(b"late write\r\n")  # a writer that kept the old file open
            return result

        checkout_bytes.subprocess.run = write_through_old_handle
        try:
            code, out = self.run_main(root, "--fix")
        finally:
            checkout_bytes.subprocess.run = real_run
        self.assertEqual(code, 0, out)
        self.assertEqual((root / "lf.txt").read_bytes(), FILES["lf.txt"])
        self.assertEqual(kept(root, "lf.txt"), [b"late write\r\n"], "the late write survives in the backup")
        self.assertIn("Originals are kept in", out)

    def test_a_directory_swapped_after_checkout_loses_nothing(self):
        for present in (True, False):
            with self.subTest(outside_has_the_file=present):
                self.swap_after_checkout(present)

    def swap_after_checkout(self, present):
        root = self.repo()
        (root / "ignored" / "[glob].txt").write_bytes(b"i\r\n")
        outside = self.tmp()
        if present:
            (outside / "[glob].txt").write_bytes(b"i\n")
        (outside / "unrelated work.txt").write_bytes(b"keep me\n")
        real_run = subprocess.run

        def swap_after_checkout(cmd, *args, **kw):
            result = real_run(cmd, *args, **kw)
            if cmd[:2] == ["git", "checkout-index"] and not os.path.islink(root / "ignored"):
                (root / "ignored" / "[glob].txt").unlink()
                (root / "ignored").rmdir()
                try:
                    if os.name == "nt":
                        import _winapi
                        _winapi.CreateJunction(str(outside), str(root / "ignored"))
                    else:
                        os.symlink(outside, root / "ignored")
                except OSError:
                    self.skipTest("this system can create neither symlinks nor junctions")
            return result

        checkout_bytes.subprocess.run = swap_after_checkout
        try:
            self.run_main(root, "--fix")
        finally:
            checkout_bytes.subprocess.run = real_run
            if os.name == "nt" and (root / "ignored").exists():
                os.rmdir(root / "ignored")  # remove the junction itself, never its target
        expected = {"unrelated work.txt": b"keep me\n", **({"[glob].txt": b"i\n"} if present else {})}
        self.assertEqual({p.name: p.read_bytes() for p in outside.iterdir()}, expected)
        self.assertEqual(kept(root, "ignored/[glob].txt"), [b"i\r\n"])

    def test_a_mode_change_after_classification_is_kept(self):
        root = self.repo()
        converted = crlf(FILES["lf.txt"])
        (root / "lf.txt").write_bytes(converted)
        saved = checkout_bytes.classify

        def chmod_after_classification(*args):
            repairs, other = saved(*args)
            raw, blob, executable = repairs["lf.txt"]
            repairs["lf.txt"] = (raw, blob, not executable)  # as if the bit changed after classification
            return repairs, other

        checkout_bytes.classify = chmod_after_classification
        try:
            code, out = self.run_main(root, "--fix")
        finally:
            checkout_bytes.classify = saved
        self.assertEqual(code, 1)
        self.assertIn("not repaired: lf.txt (changed while the repair ran)", out)
        self.assertEqual((root / "lf.txt").read_bytes(), converted)

    def test_a_link_introduced_after_classification_is_not_followed(self):
        root = self.repo()
        (root / "ignored" / "[glob].txt").write_bytes(b"i\r\n")
        outside = self.tmp()
        (outside / "[glob].txt").write_bytes(b"i\r\n")
        saved = checkout_bytes.classify

        def swap_directory(*args):
            result = saved(*args)
            (root / "ignored" / "[glob].txt").unlink()
            (root / "ignored").rmdir()
            try:
                if os.name == "nt":
                    import _winapi
                    _winapi.CreateJunction(str(outside), str(root / "ignored"))
                else:
                    os.symlink(outside, root / "ignored")
            except OSError:
                self.skipTest("this system can create neither symlinks nor junctions")
            return result

        checkout_bytes.classify = swap_directory
        try:
            code, out = self.run_main(root, "--fix")
        finally:
            checkout_bytes.classify = saved
            if os.name == "nt" and (root / "ignored").exists():
                os.rmdir(root / "ignored")  # remove the junction itself, never its target
        self.assertIn("not repaired: ignored/[glob].txt", out)
        self.assertEqual(sorted(p.name for p in outside.iterdir()), ["[glob].txt"])
        self.assertEqual((outside / "[glob].txt").read_bytes(), b"i\r\n")

    def test_repair_is_refused_while_git_still_converts(self):
        root = self.repo(attributes=False)
        (root / "lf.txt").write_bytes(crlf(FILES["lf.txt"]))
        code, out = self.run_main(root, "--fix")
        self.assertEqual(code, 2)
        self.assertIn("line-ending conversion still enabled: lf.txt", out)
        self.assertEqual((root / "lf.txt").read_bytes(), crlf(FILES["lf.txt"]))

    def test_links_are_never_followed(self):
        root = self.repo()
        outside = self.tmp()
        (outside / "lf.txt").write_bytes(crlf(FILES["lf.txt"]))
        linked = []
        (root / "ignored" / "[glob].txt").unlink()
        (root / "ignored").rmdir()
        try:
            if os.name == "nt":
                import _winapi
                _winapi.CreateJunction(str(outside), str(root / "ignored"))
            else:
                os.symlink(outside, root / "ignored")
            (outside / "[glob].txt").write_bytes(b"i\r\n")
            linked.append("ignored/[glob].txt")
        except OSError:
            pass
        (root / "lf.txt").unlink()
        try:
            os.symlink(outside / "lf.txt", root / "lf.txt")
            linked.append("lf.txt")
        except OSError:
            (root / "lf.txt").write_bytes(FILES["lf.txt"])  # no symlink privilege; the junction case still runs
        if not linked:
            self.skipTest("this system can create neither symlinks nor junctions")
        code, out = self.run_main(root, "--fix")
        for name in linked:
            with self.subTest(name=name):
                self.assertIn("left unchanged (other edits, links or missing): " + name, out)
        self.assertEqual((outside / "lf.txt").read_bytes(), crlf(FILES["lf.txt"]))
        self.assertEqual((outside / "[glob].txt").read_bytes() if (outside / "[glob].txt").exists() else b"i\r\n", b"i\r\n")
        self.assertEqual(git(root, "diff", "--cached", "--name-only").stdout, b"")
        if os.name == "nt" and (root / "ignored").exists():
            os.rmdir(root / "ignored")  # remove the junction itself, never its target

    def test_unmerged_entries_are_refused(self):
        root = self.repo()
        git(root, "checkout", "-qb", "other")
        (root / "lf.txt").write_bytes(b"theirs\n")
        git(root, "commit", "-qam", "theirs")
        git(root, "checkout", "-q", "-")
        (root / "lf.txt").write_bytes(b"ours\n")
        git(root, "commit", "-qam", "ours")
        self.assertNotEqual(git(root, "merge", "other", check=False).returncode, 0)
        (root / "lf.txt").write_bytes(b"theirs\r\n")  # an unstaged resolution that looks like a conversion
        before = self.index(root)
        code, out = self.run_main(root, "--fix")
        self.assertEqual(code, 2)
        self.assertIn("unmerged: lf.txt", out)
        self.assertEqual(self.index(root), before)
        self.assertEqual((root / "lf.txt").read_bytes(), b"theirs\r\n")

    def test_a_mode_only_change_is_not_touched(self):
        root = self.repo()
        git(root, "config", "core.filemode", "true")
        git(root, "update-index", "--chmod=+x", "lf.txt")
        git(root, "commit", "-qm", "executable")
        os.chmod(root / "lf.txt", 0o644)
        if b"lf.txt" not in git(root, "status", "--porcelain").stdout:
            self.skipTest("this file system does not report executable bits")
        before = self.index(root)
        code, out = self.run_main(root, "--fix")
        self.assertIn("left unchanged (other edits, links or missing): lf.txt", out)
        self.assertEqual(self.index(root), before)
        self.assertEqual(git(root, "diff", "--cached", "--name-only").stdout, b"")

    def test_repository_disables_line_ending_conversion(self):
        out = subprocess.run(["git", "check-attr", "text", "--", "assets/manifest.json", "core.py",
                              "work/experiments/magic600-04/native/ExperimentHelp.cs"],
                             cwd=ROOT, check=True, capture_output=True, text=True).stdout
        self.assertEqual(out.count(": text: unset"), 3, out)


if __name__ == "__main__":
    unittest.main()
