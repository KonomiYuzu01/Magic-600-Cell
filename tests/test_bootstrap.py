"""Tests for the toolchain installer and the install guard hook. No network, no real installs."""
from __future__ import annotations

import contextlib
import copy
import hashlib
import http.client
import io
import json
import os
import re
import shutil
import socket
import ssl
import stat
import subprocess
import sys
import tempfile
import threading
import unittest
import uuid
from types import SimpleNamespace
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parents[1]
sys.dont_write_bytecode = True
sys.path.insert(0, str(ROOT / "tools"))
sys.path.insert(0, str(ROOT / "tools" / "toolchain"))
sys.path.insert(0, str(ROOT / ".claude" / "hooks"))
import bootstrap  # noqa: E402
import install_guard  # noqa: E402


# The real process and link functions, for the few integration tests that start a real process.
REAL = {}


@contextlib.contextmanager
def real_processes(test):
    """Allow real processes inside the block; skip where none can start (the Codex sandbox)."""
    with mock.patch.object(subprocess, "run", REAL["run"]), mock.patch.object(subprocess, "Popen", REAL["Popen"]):
        try:
            yield
        except PermissionError as exc:
            test.skipTest(f"cannot start a process here ({exc})")


@contextlib.contextmanager
def real_sockets():
    """Allow real local sockets inside the block, for the transport tests that build a socket pair."""
    with mock.patch.object(socket, "create_connection", REAL["create_connection"]), \
            mock.patch.object(socket.socket, "connect", REAL["connect"]):
        yield


def real_links_work(root: Path) -> bool:
    """True where this process can make a real directory link, as on CI; the link tests then use real links."""
    target, link = root / "link-probe-target", root / "link-probe"
    target.mkdir()
    try:
        os.symlink(target, link, target_is_directory=True)
    except (OSError, NotImplementedError):
        return False
    os.unlink(link)
    return True


def synthetic_links():
    """Stand in for links where none can be made (Windows without the symlink privilege, the Codex sandbox).

    The fixture models realpath, readlink and is_symlink only, not stat or scandir, so it cannot
    replace real links: the real-link runs on CI remain the coverage of record."""
    links = {}
    realpath, readlink, unlink, is_symlink = os.path.realpath, os.readlink, Path.unlink, Path.is_symlink

    def link_fixture(source, dest, target_is_directory=False):
        # Store synthetic link metadata on an ordinary empty file; never create a real link.
        Path(dest).write_bytes(b"")
        links[os.path.abspath(dest)] = os.fspath(source)

    def fixture_realpath(path, **kwargs):
        path = os.path.abspath(path)
        for dest, source in links.items():
            if os.path.normcase(path) == os.path.normcase(dest) or os.path.normcase(path).startswith(os.path.normcase(dest) + os.sep):
                path = os.path.join(os.path.dirname(dest), source) + path[len(dest):]
                break
        return realpath(path, **kwargs)

    def fixture_unlink(path, **kwargs):
        unlink(path, **kwargs)
        links.pop(os.path.abspath(path), None)

    unittest.enterModuleContext(mock.patch.object(os, "symlink", side_effect=link_fixture))
    unittest.enterModuleContext(mock.patch.object(os.path, "realpath", side_effect=fixture_realpath))
    unittest.enterModuleContext(mock.patch.object(os, "readlink", side_effect=lambda p: links.get(os.path.abspath(p)) or readlink(p)))
    unittest.enterModuleContext(mock.patch.object(Path, "is_symlink", autospec=True,
                                                  side_effect=lambda p: os.path.abspath(p) in links or is_symlink(p)))
    unittest.enterModuleContext(mock.patch.object(Path, "unlink", autospec=True, side_effect=fixture_unlink))


def setUpModule():
    global TEST_TEMP
    REAL.update(run=subprocess.run, Popen=subprocess.Popen, symlink=os.symlink,
                create_connection=socket.create_connection, connect=socket.socket.connect)
    for target in ("subprocess.run", "subprocess.Popen", "socket.create_connection", "socket.socket.connect"):
        unittest.enterModuleContext(mock.patch(target, side_effect=AssertionError(f"unmocked boundary: {target}")))
    mkdtemp, mkdir = tempfile.mkdtemp, os.mkdir

    def fixture_dir(suffix=None, prefix=None, dir=None):
        # CPython 3.14's Windows mode 0o700 ACL excludes the restricted sandbox token.
        # Fixtures are created without that mode and inherit the temp directory's permissions instead.
        with mock.patch.object(os, "mkdir", side_effect=lambda path, mode: mkdir(path)):
            return mkdtemp(suffix, prefix, dir or fixture_root)

    fixture_root = None  # the system temp directory, never the repository
    suite_dir = fixture_dir(prefix="bootstrap-tests-")
    fixture_root = Path(suite_dir)
    TEST_TEMP = fixture_root
    unittest.enterModuleContext(mock.patch.object(bootstrap, "LEDGER_PATH", TEST_TEMP / "installs.jsonl"))
    unittest.addModuleCleanup(shutil.rmtree, suite_dir)
    unittest.enterModuleContext(mock.patch.object(tempfile, "mkdtemp", side_effect=fixture_dir))
    if not real_links_work(fixture_root):
        synthetic_links()


def run(argv):
    err = io.StringIO()
    with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(err):
        try:
            code = bootstrap.main(argv)
        except SystemExit as exc:
            code = exc.code
    return code, err.getvalue()


class LockfileTests(unittest.TestCase):
    def test_install_ledger_is_isolated_for_the_entire_suite(self):
        self.assertNotEqual(bootstrap.LEDGER_PATH, ROOT / "work/loop-memory/ledgers/installs.jsonl")
        self.assertEqual(bootstrap.LEDGER_PATH.parent, TEST_TEMP)

    def test_lockfile_loads_and_is_consistent(self):
        lock = bootstrap.load_lock()
        profiles = set(lock["profiles"])
        for entry in lock["tools"]:
            with self.subTest(tool=entry["id"]):
                self.assertTrue(set(entry["profiles"]) <= profiles)
                if entry["installable"]:
                    self.assertEqual(entry["status"], "verified")
                    self.assertTrue(entry.get("version"), "installable entries are version-pinned")
                if entry["method"] in ("pip-hashed", "uv-venv-hashed"):
                    text = (ROOT / entry["requirements"]).read_text(encoding="utf-8")
                    self.assertIn("--hash=sha256:", text)
                    self.assertIn(entry["version"], bootstrap.requirement_pins(ROOT / entry["requirements"]).values())
                    bootstrap.check_requirement_sources(entry)  # exact name==version pins only
                if entry["method"] == "download-hashed":
                    self.assertTrue(bootstrap.valid_download(entry))
                if entry["method"] == "npm-ci":
                    if entry["installable"]:
                        lockfile = json.loads((ROOT / entry["prefix"] / "package-lock.json").read_text(encoding="utf-8"))
                        packages = [p for k, p in lockfile["packages"].items() if k]
                        self.assertTrue(all("integrity" in p for p in packages if not p.get("link")))
                if entry["method"] == "winget" and entry["installable"]:
                    self.assertRegex(entry.get("installer_sha256") or "", r"^[0-9a-f]{64}$")
                for dep in entry.get("requires", []):
                    bootstrap.entry_for(lock, dep)

    def test_lockfile_is_english_only(self):
        text = (ROOT / "tools" / "toolchain.lock.json").read_text(encoding="utf-8")
        self.assertIsNone(__import__("re").search(r"[぀-ヿ㐀-鿿]", text))

    def test_duplicate_ids_are_rejected(self):
        with tempfile.TemporaryDirectory() as td:
            lock = json.loads(bootstrap.LOCK_PATH.read_text(encoding="utf-8"))
            lock["tools"].append(dict(lock["tools"][0]))
            path = Path(td) / "lock.json"
            path.write_text(json.dumps(lock), encoding="utf-8")
            saved = bootstrap.LOCK_PATH
            bootstrap.LOCK_PATH = path
            try:
                with self.assertRaises(SystemExit):
                    bootstrap.load_lock()
            finally:
                bootstrap.LOCK_PATH = saved

    def test_invalid_python_requests_are_rejected(self):
        good = {"implementation": "cpython", "version": "3.14.7", "bits": 64}
        cases = {"32-bit": dict(good, bits=32), "not exact": dict(good, version="3.14"), "other implementation": dict(good, implementation="pypy"),
                 "extra key": dict(good, path="C:/Python314/python.exe"), "numeric version": dict(good, version=3.14),
                 "float bits": dict(good, bits=64.0), "boolean bits": dict(good, bits=True)}
        with tempfile.TemporaryDirectory() as td:
            path = Path(td) / "lock.json"
            saved = bootstrap.LOCK_PATH
            bootstrap.LOCK_PATH = path
            try:
                for name, request in [*cases.items(), ("wrong method", good)]:
                    lock = json.loads(saved.read_text(encoding="utf-8"))
                    entry = bootstrap.entry_for(lock, "uv" if name == "wrong method" else "engine-python")
                    entry["python"] = request
                    path.write_text(json.dumps(lock), encoding="utf-8")
                    with self.subTest(case=name), self.assertRaises(SystemExit):
                        bootstrap.load_lock()
            finally:
                bootstrap.LOCK_PATH = saved


class RefusalTests(unittest.TestCase):
    def test_unknown_and_unverified_ids_are_refused(self):
        self.assertEqual(run(["install", "no-such-tool"])[0], 2)
        unverified = next(e["id"] for e in bootstrap.load_lock()["tools"] if e["status"] == "unverified")
        code, err = run(["install", unverified])
        self.assertEqual(code, 2)
        self.assertIn("not installable", err)

    def test_owner_installed_tools_are_refused(self):
        code, err = run(["install", "codex-cli"])
        self.assertEqual(code, 2)

    def test_malformed_ids_and_unknown_arguments_are_rejected(self):
        for argv in (["install", "../escape"], ["install", "a;b"], ["install-skill", "../../x"], ["install", "--profile", "Bad Profile"]):
            with self.subTest(argv=argv):
                self.assertEqual(run(argv)[0], 2)
        for argv in (["install", "uv", "--source", "https://example.invalid"], ["install", "uv", "--index-url", "x"], ["frobnicate"]):
            with self.subTest(argv=argv):
                self.assertEqual(run(argv)[0], 2)

    def test_unknown_profile_is_refused(self):
        self.assertEqual(run(["install", "--profile", "everything"])[0], 2)

    def test_approve_requires_an_interactive_terminal(self):
        with mock.patch.object(sys.stdin, "isatty", return_value=False):
            code, err = run(["approve"])
        self.assertEqual(code, 2)
        self.assertIn("interactive terminal", err)

    def test_approve_refuses_piped_input_in_a_real_process(self):
        with real_processes(self):
            r = subprocess.run([sys.executable, str(ROOT / "tools" / "toolchain" / "bootstrap.py"), "approve"],
                               input="approve x\n", capture_output=True, text=True, timeout=30)
        self.assertEqual(r.returncode, 2)
        self.assertIn("interactive terminal", r.stderr)

    def test_unpinned_skill_source_is_refused_before_network(self):
        with tempfile.TemporaryDirectory() as td:
            path = Path(td) / "skills.lock.json"
            path.write_text(json.dumps({"skills": {"evil": {"origin": "third-party", "repo": "https://example.invalid/x",
                                                            "commit": "main", "subdir": ".", "digest": "0"}}}), encoding="utf-8")
            saved = bootstrap.SKILLS_LOCK_PATH
            bootstrap.SKILLS_LOCK_PATH = path
            try:
                code, err = run(["install-skill", "evil"])
            finally:
                bootstrap.SKILLS_LOCK_PATH = saved
        self.assertEqual(code, 2)
        self.assertIn("not a pinned GitHub commit", err)

    def test_winget_hash_mismatch_blocks(self):
        entry = {"id": "x", "method": "winget", "winget_id": "Vendor.X", "version": "1.0", "installer_sha256": "a" * 64}
        saved = (bootstrap.PLATFORM, bootstrap.winget_show)
        bootstrap.PLATFORM = "windows"
        bootstrap.winget_show = lambda *_: {"version": "1.0", "installer_sha256": "b" * 64}
        try:
            with self.assertRaises(bootstrap.Refused) as ctx:
                bootstrap.install_entry(entry)
        finally:
            bootstrap.PLATFORM, bootstrap.winget_show = saved
        self.assertIn("hash differs", str(ctx.exception))

    def test_winget_install_skips_dependencies_and_uses_the_hashed_selection(self):
        entry = {"id": "x", "method": "winget", "winget_id": "Vendor.X", "version": "1.0", "installer_sha256": "a" * 64}
        calls = []
        saved = (bootstrap.PLATFORM, bootstrap.winget_show, bootstrap._run, bootstrap.shutil.which)
        bootstrap.PLATFORM = "windows"
        bootstrap.winget_show = lambda *_: {"version": "1.0", "installer_sha256": "a" * 64}
        bootstrap._run = lambda cmd, cwd=None: calls.append(cmd)
        bootstrap.shutil.which = lambda name: "winget"
        try:
            bootstrap.install_entry(entry)
        finally:
            bootstrap.PLATFORM, bootstrap.winget_show, bootstrap._run, bootstrap.shutil.which = saved
        self.assertIn("--skip-dependencies", calls[0])
        for flag in bootstrap.WINGET_SELECTION:
            self.assertIn(flag, calls[0])

    def test_present_tool_at_another_version_is_not_replaced(self):
        lock = bootstrap.load_lock()
        entry = bootstrap.entry_for(lock, "marimo")
        saved = (bootstrap.run_probe, bootstrap.present, bootstrap.install_entry)
        installs = []
        bootstrap.run_probe = lambda e: (e["id"] != "marimo", "unexpected version: 9.9")
        bootstrap.present = lambda e: True
        bootstrap.install_entry = lambda e: installs.append(e["id"])
        try:
            with self.assertRaises(bootstrap.Refused) as ctx:
                bootstrap.install(lock, "marimo", set())
        finally:
            bootstrap.run_probe, bootstrap.present, bootstrap.install_entry = saved
        self.assertEqual(installs, [])
        self.assertIn("owner approval", str(ctx.exception))

    def test_linked_destinations_are_refused(self):
        with tempfile.TemporaryDirectory() as outside, tempfile.TemporaryDirectory() as td, \
                mock.patch.object(bootstrap, "ROOT", Path(td).resolve()):
            link = bootstrap.ROOT / "tools" / ".venv" / "link-test"
            link.parent.mkdir(parents=True, exist_ok=True)
            try:
                os.symlink(outside, link, target_is_directory=True)
            except (OSError, NotImplementedError):
                self.skipTest("symlinks not available")
            try:
                with self.assertRaises(bootstrap.Refused):
                    bootstrap.safe_dest("tools/.venv/link-test")
                with self.assertRaises(bootstrap.Refused):
                    bootstrap.safe_dest("tools/.venv/link-test/inner")
            finally:
                link.unlink()
        with self.assertRaises(bootstrap.Refused):
            bootstrap.safe_dest("../outside")
        self.assertEqual(bootstrap.safe_dest("tools/.venv/planning"), ROOT.resolve() / "tools" / ".venv" / "planning")

    def test_shared_environment_sync_never_replaces_another_tool(self):
        lock = bootstrap.load_lock()
        with tempfile.TemporaryDirectory() as td:
            site = Path(td) / "lib" / "python3.11" / "site-packages"
            site.mkdir(parents=True)
            saved = (bootstrap.run_probe, bootstrap.present, bootstrap.install_entry, bootstrap.safe_dest, bootstrap.ledger)
            installs, records = [], []
            bootstrap.ledger = records.append
            bootstrap.run_probe = lambda e: (e["id"] == "uv" or e["id"] in installs, "missing")  # uv (a dependency) works
            bootstrap.present = lambda e: False
            bootstrap.install_entry = lambda e: installs.append(e["id"])
            bootstrap.safe_dest = lambda rel: Path(td)
            try:
                (site / "viztracer-1.1.10.dist-info").mkdir()  # near-miss version that a loose probe accepts
                with self.assertRaises(bootstrap.Refused) as ctx:
                    bootstrap.install(lock, "py-spy", set())
                self.assertIn("viztracer 1.1.10", str(ctx.exception))
                (site / "viztracer-1.1.10.dist-info").rmdir()
                (site / "unrelated_pkg-2.0.dist-info").mkdir()  # a sync would remove it
                with self.assertRaises(bootstrap.Refused):
                    bootstrap.install(lock, "py-spy", set())
                (site / "unrelated_pkg-2.0.dist-info").rmdir()
                (site / "viztracer-1.1.1.dist-info").mkdir()
                bootstrap.install(lock, "py-spy", set())
            finally:
                bootstrap.run_probe, bootstrap.present, bootstrap.install_entry, bootstrap.safe_dest, bootstrap.ledger = saved
        self.assertEqual(installs, ["py-spy"])
        self.assertEqual([r["tool"] for r in records], ["py-spy"])

    def test_linked_venv_descendants_are_refused(self):
        entry = bootstrap.entry_for(bootstrap.load_lock(), "marimo")
        with tempfile.TemporaryDirectory() as td, tempfile.TemporaryDirectory() as outside:
            venv = Path(td)
            bindir = bootstrap._bin_dir(venv)  # bin, or Scripts on Windows
            bindir.mkdir()
            (venv / "lib" / "python3.11").mkdir(parents=True)
            try:
                os.symlink("lib", venv / "lib64", target_is_directory=True)  # legitimate internal link
                os.symlink(outside, venv / "lib" / "python3.11" / "site-packages", target_is_directory=True)
            except (OSError, NotImplementedError):
                self.skipTest("symlinks not available")
            saved = bootstrap.safe_dest
            bootstrap.safe_dest = lambda rel: venv
            try:
                with self.assertRaises(bootstrap.Refused):
                    bootstrap.venv_conflicts(entry)
                (venv / "lib" / "python3.11" / "site-packages").unlink()
                (venv / "lib" / "python3.11" / "site-packages").mkdir()
                os.symlink(Path(outside) / "x", bindir / "marimo")
                with self.assertRaises(bootstrap.Refused):
                    bootstrap.venv_conflicts(entry)
                spawned = []
                saved_run = bootstrap.subprocess.run
                bootstrap.subprocess.run = lambda *a, **k: spawned.append(a) or subprocess.CompletedProcess(a, 0, "0.25.0", "")
                try:
                    ok, detail = bootstrap.run_probe(entry)  # linked executable: refused before any spawn
                finally:
                    bootstrap.subprocess.run = saved_run
                self.assertFalse(ok)
                self.assertIn("refused", detail)
                self.assertEqual(spawned, [])
                (bindir / "marimo").unlink()
                os.symlink(sys.executable, bindir / "python")  # interpreter link is allowed
                self.assertEqual(bootstrap.venv_conflicts(entry), [])
                (bindir / "python").unlink()
                os.symlink(Path(outside) / "evil-python", bindir / "python")  # any other interpreter is not
                with self.assertRaises(bootstrap.Refused):
                    bootstrap.venv_conflicts(entry)
                (bindir / "python").unlink()
                nested = venv / "lib" / "python3.11" / "site-packages" / "marimo"
                nested.mkdir()
                os.symlink(outside, nested / "_plugins", target_is_directory=True)  # deeper link
                with self.assertRaises(bootstrap.Refused):
                    bootstrap.venv_conflicts(entry)
            finally:
                bootstrap.safe_dest = saved

    def test_any_unexpected_link_in_a_venv_is_refused(self):
        entry = bootstrap.entry_for(bootstrap.load_lock(), "marimo")
        with tempfile.TemporaryDirectory() as td, tempfile.TemporaryDirectory() as outside:
            venv = Path(td)
            (venv / "alternate").mkdir()
            (venv / "lib" / "python3.11").mkdir(parents=True)
            try:
                os.symlink(venv / "alternate", venv / "lib" / "python3.11" / "site-packages", target_is_directory=True)
                os.symlink(outside, venv / "alternate" / "marimo", target_is_directory=True)
            except (OSError, NotImplementedError):
                self.skipTest("symlinks not available")
            saved = (bootstrap.safe_dest, bootstrap._is_link)
            bootstrap.safe_dest = lambda rel: venv
            try:
                with self.assertRaises(bootstrap.Refused):  # internal alias hiding an external link
                    bootstrap.venv_contained(entry)
                (venv / "lib" / "python3.11" / "site-packages").unlink()
                (venv / "alternate" / "marimo").unlink()
                junction = venv / "lib" / "python3.11" / "junction"
                junction.mkdir()
                (junction / "deep").mkdir()
                visited = []
                real_is_link = saved[1]

                def fake_is_link(p):
                    visited.append(Path(p))
                    return Path(p) == junction or real_is_link(p)
                bootstrap._is_link = fake_is_link
                with self.assertRaises(bootstrap.Refused):  # a junction is refused before descent
                    bootstrap.venv_contained(entry)
                self.assertNotIn(junction / "deep", visited)
            finally:
                bootstrap.safe_dest, bootstrap._is_link = saved

    def test_sync_is_refused_when_uv_would_remove_or_replace(self):
        entry = bootstrap.entry_for(bootstrap.load_lock(), "marimo")
        saved = (bootstrap.subprocess.run, bootstrap.venv_contained, bootstrap._run)
        runs = []
        bootstrap.venv_contained = lambda e: []
        bootstrap._run = lambda cmd, cwd=ROOT, env=None: runs.append(cmd)
        try:
            for output, removals in (("Would make no changes\n", []),
                                     (" - cffi==2.1.1\n + viztracer==1.1.1\n - unrelated-pkg==2.0\n", ["cffi==2.1.1", "unrelated-pkg==2.0"]),
                                     (" \x1b[31m-\x1b[39m cffi==2.1.1\n", ["cffi==2.1.1"])):
                bootstrap.subprocess.run = lambda *a, **k: subprocess.CompletedProcess(a, 0, "", output)
                with self.subTest(output=output):
                    self.assertEqual(bootstrap.sync_removals(entry, ["py", "-m", "uv", "pip", "sync", "req"]), removals)
            bootstrap.subprocess.run = lambda *a, **k: subprocess.CompletedProcess(a, 0, "", " - cffi==2.1.1\n")
            with self.assertRaises(bootstrap.Refused):
                bootstrap.install_entry(entry)
            self.assertFalse([c for c in runs if "sync" in c])
        finally:
            bootstrap.subprocess.run, bootstrap.venv_contained, bootstrap._run = saved

    def test_engine_environment_uses_only_the_exact_owner_installed_python(self):
        lock = bootstrap.load_lock()
        entry = bootstrap.entry_for(lock, "engine-python")
        with tempfile.TemporaryDirectory() as td:
            venv_dir, system, managed = Path(td) / "engine", Path(td) / "system", Path(td) / "uv-python"
            uv_bin = str(Path(td) / "uv.exe")
            saved = (bootstrap._run, bootstrap.safe_dest, bootstrap.sync_removals, bootstrap.subprocess.run, bootstrap.uv_binary)
            bootstrap.safe_dest = lambda rel: venv_dir
            bootstrap.sync_removals = lambda e, sync: []
            bootstrap.uv_binary = lambda: uv_bin
            try:
                for facts, venv_fails, home, ok in (
                        ("cpython 3.14.7 64", False, system, True),
                        ("cpython 3.14.6 64", False, system, False),
                        ("cpython 3.14.7 32", False, system, False),
                        ("pypy 3.14.7 64", False, system, False),
                        ("", True, system, False),
                        ("cpython 3.14.7 64", False, managed / "cpython-3.14-windows-x86_64-none", False),  # uv-managed copy
                        ("cpython 3.14.7 64", False, None, False)):                                           # no pyvenv.cfg
                    runs = []
                    venv_dir.mkdir(exist_ok=True)
                    cfg = venv_dir / "pyvenv.cfg"
                    cfg.unlink(missing_ok=True)
                    if home is not None:
                        cfg.write_text(f"home = {home}\nversion_info = 3.14.7\n", encoding="utf-8")

                    def fake_run(cmd, cwd=ROOT, env=None, venv_fails=venv_fails):
                        runs.append(cmd)
                        if venv_fails and "venv" in cmd:
                            raise bootstrap.Refused("command failed with exit 2")

                    def fake_subprocess(cmd, *a, facts=facts, **k):
                        out = str(managed) if cmd[1:3] == ["python", "dir"] else facts
                        return subprocess.CompletedProcess(cmd, 0, out + "\n", "")

                    bootstrap._run = fake_run
                    bootstrap.subprocess.run = fake_subprocess
                    with self.subTest(facts=facts, venv_fails=venv_fails, home=home):
                        if ok:
                            bootstrap.install_entry(entry)
                        else:
                            with self.assertRaises(bootstrap.Refused) as ctx:
                                bootstrap.install_entry(entry)
                            self.assertIn("CPython 3.14.7" if venv_fails else "replacing it needs owner approval", str(ctx.exception))
                        venv = runs[0]
                        self.assertEqual(venv[0], uv_bin, "the uv executable runs directly, never as `python -m uv`")
                        self.assertEqual(venv[venv.index("--python") + 1], "cpython@3.14.7")
                        self.assertNotIn(sys.executable, venv)
                        self.assertIn("--no-python-downloads", venv)
                        self.assertIn("--no-managed-python", venv)
                        self.assertEqual(any("sync" in c for c in runs), ok, "packages are synced only into the exact interpreter")
                runs = []
                bootstrap._run = lambda cmd, cwd=ROOT, env=None: runs.append(cmd)
                bootstrap.install_entry(bootstrap.entry_for(lock, "marimo"))  # entries without a request keep the installer's Python
                self.assertEqual(runs[0][:3], [sys.executable, "-m", "uv"])
                self.assertEqual(runs[0][runs[0].index("--python") + 1], sys.executable)
            finally:
                (bootstrap._run, bootstrap.safe_dest, bootstrap.sync_removals, bootstrap.subprocess.run,
                 bootstrap.uv_binary) = saved

    def test_present_environment_must_run_the_exact_owner_installed_python(self):
        lock = bootstrap.load_lock()
        entry = bootstrap.entry_for(lock, "tastelab-numerics")
        with tempfile.TemporaryDirectory() as td:
            venv_dir, system, managed = Path(td) / "tastelab", Path(td) / "system", Path(td) / "uv-python"
            venv_dir.mkdir()
            for facts, home, ok in (("cpython 3.14.7 64", system, True),
                                    ("cpython 3.14.6 64", system, False),
                                    ("cpython 3.14.7 64", managed / "cpython-3.14-windows-x86_64-none", False)):  # uv-managed copy
                (venv_dir / "pyvenv.cfg").write_text(f"home = {home}\n", encoding="utf-8")
                runs, installs = [], []

                def fake_subprocess(cmd, *a, facts=facts, **k):
                    runs.append(cmd)
                    out = str(managed) if cmd[1:3] == ["python", "dir"] else facts
                    return subprocess.CompletedProcess(cmd, 0, out + "\n", "")

                # Every package probe passes and nothing conflicts: only the interpreter differs.
                with self.subTest(facts=facts, home=home), \
                        mock.patch.object(bootstrap, "run_probe", return_value=(True, "packages present")), \
                        mock.patch.object(bootstrap, "venv_conflicts", return_value=[]), \
                        mock.patch.object(bootstrap, "safe_dest", return_value=venv_dir), \
                        mock.patch.object(bootstrap, "uv_binary", return_value=str(Path(td) / "uv.exe")), \
                        mock.patch.object(bootstrap.subprocess, "run", side_effect=fake_subprocess), \
                        mock.patch.object(bootstrap, "install_entry", side_effect=lambda e: installs.append(e["id"])), \
                        contextlib.redirect_stdout(io.StringIO()):
                    if ok:
                        self.assertEqual(bootstrap.install(lock, entry["id"], set()), "present")
                    else:
                        with self.assertRaises(bootstrap.Refused) as ctx:
                            bootstrap.install(lock, entry["id"], set())
                        self.assertIn("replacing it needs owner approval", str(ctx.exception))
                    self.assertTrue(any(cmd[1:3] == ["python", "dir"] for cmd in runs), "the interpreter check ran")
                    self.assertEqual(installs, [])

    def test_probes_drop_code_injection_variables(self):
        seen = {}
        saved = (bootstrap.resolve_executable, bootstrap.subprocess.run, os.environ.get("NODE_OPTIONS"))
        bootstrap.resolve_executable = lambda e, n: "tool"
        bootstrap.subprocess.run = lambda *a, **k: seen.update(k.get("env") or {"inherited": "yes"}) or subprocess.CompletedProcess(a, 0, "12.0.0", "")
        os.environ["NODE_OPTIONS"] = "--require /tmp/inject.js"
        try:
            bootstrap.run_probe(bootstrap.entry_for(bootstrap.load_lock(), "mermaid-cli"))
        finally:
            bootstrap.resolve_executable, bootstrap.subprocess.run = saved[:2]
            if saved[2] is None:
                os.environ.pop("NODE_OPTIONS", None)
            else:
                os.environ["NODE_OPTIONS"] = saved[2]
        self.assertNotIn("inherited", seen)
        self.assertNotIn("NODE_OPTIONS", seen)

    def test_winget_gui_tools_are_verified_through_winget_list(self):
        entry = dict(bootstrap.entry_for(bootstrap.load_lock(), "drawio"), version="31.5.3")
        saved = (bootstrap.PLATFORM, bootstrap.resolve_executable, bootstrap.shutil.which, bootstrap.subprocess.run)
        bootstrap.PLATFORM = "windows"
        bootstrap.resolve_executable = lambda e, n: None
        bootstrap.shutil.which = lambda name, **k: "winget.exe" if name == "winget" else None
        try:
            for code, listing, ok in ((0, "Name     Id          Version\ndraw.io  JGraph.Draw 31.5.3\n", True),
                                      (0, "Name     Id          Version\ndraw.io  JGraph.Draw 31.5.3.0\n", True),
                                      (0, "Name     Id          Version\ndraw.io  JGraph.Draw 31.5.30\n", False),
                                      (0, "Name     Id          Version\ndraw.io  JGraph.Draw 31.5.3.1\n", False),
                                      # An available update is never the installed version.
                                      (0, "Name Id Version Available Source\ndraw.io JGraph.Draw 31.5.2.0 31.5.3 winget\n", False),
                                      (0x8A150014, "No installed package found matching input criteria.\n", False),
                                      (1, "", False)):
                bootstrap.subprocess.run = lambda *a, c=code, l=listing, **k: subprocess.CompletedProcess(a, c, l, "")
                with self.subTest(listing=listing):
                    self.assertEqual(bootstrap.run_probe(entry)[0], ok)
        finally:
            bootstrap.PLATFORM, bootstrap.resolve_executable, bootstrap.shutil.which, bootstrap.subprocess.run = saved

    def test_winget_package_at_another_version_is_not_replaced(self):
        lock = bootstrap.load_lock()
        state = {"installed": False}
        saved = (bootstrap.PLATFORM, bootstrap.resolve_executable, bootstrap.shutil.which, bootstrap.subprocess.run,
                 bootstrap.install_entry, bootstrap.ledger)
        bootstrap.PLATFORM = "windows"
        bootstrap.shutil.which = lambda name, **k: "winget.exe" if name == "winget" else None
        bootstrap.ledger = lambda record: None
        missing = (0x8A150014, "No installed package found matching input criteria.\n")
        try:
            # (tool, version its command reports or None when not on PATH, winget list exit and output, refusal)
            for tool, reported, (code, listing), refusal in (
                    ("drawio", None, (0, "draw.io  JGraph.Draw 31.5.2.0\n"), "owner approval"),
                    ("drawio", None, (0, "Name Id Version Available Source\ndraw.io JGraph.Draw 31.5.2.0 31.5.3 winget\n"),
                     "owner approval"),
                    ("drawio", None, (1, ""), "failed with exit 1"),  # a failed listing is not proof of absence
                    ("typst", "typst 0.14.2", (0, "Typst  Typst.Typst 0.14.2\n"), "owner approval"),
                    ("jq", "jq-1.8.1", missing, "owner approval")):
                installs = []
                bootstrap.install_entry = lambda e: installs.append(e["id"])
                bootstrap.resolve_executable = lambda e, n, r=reported: "tool" if r else None
                bootstrap.subprocess.run = lambda cmd, *a, r=reported, c=code, l=listing, **k: (
                    subprocess.CompletedProcess(cmd, c, l, "") if "list" in cmd else subprocess.CompletedProcess(cmd, 0, r, ""))
                with self.subTest(tool=tool, listing=listing):
                    with self.assertRaises(bootstrap.Refused) as ctx:
                        bootstrap.install(lock, tool, set())
                    self.assertIn(refusal, str(ctx.exception))
                    self.assertEqual(installs, [])
            # Control: a package that winget does not list and that is not on PATH is installed.
            installs = []

            def install_drawio(entry):
                installs.append(entry["id"])
                state["installed"] = True
            bootstrap.install_entry = install_drawio
            bootstrap.resolve_executable = lambda e, n: None
            bootstrap.subprocess.run = lambda cmd, *a, **k: subprocess.CompletedProcess(
                cmd, *((0, "draw.io  JGraph.Draw 31.5.3.0\n") if state["installed"] else missing), "")
            with contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(bootstrap.install(lock, "drawio", set()), "installed")
            self.assertEqual(installs, ["drawio"])
        finally:
            (bootstrap.PLATFORM, bootstrap.resolve_executable, bootstrap.shutil.which, bootstrap.subprocess.run,
             bootstrap.install_entry, bootstrap.ledger) = saved

    def test_pinned_versions_are_compared_exactly(self):
        lock = bootstrap.load_lock()
        saved = (bootstrap.resolve_executable, bootstrap.subprocess.run)
        bootstrap.resolve_executable = lambda e, n: "tool"
        try:
            for tool, output, ok in (("uv", "uv 0.12.20 (x86_64)", True), ("uv", "uv 0.12.200", False),
                                     ("uv", "uv 0.12.20.1", False), ("marp-cli", "@marp-team/marp-cli v4.5.1 (w/ core)", True),
                                     ("mermaid-cli", "12.0.01", False),
                                     ("marp-cli", "@marp-team/marp-cli v4.5.10 (w/ @marp-team/marp-core v4.5.1)", False),
                                     ("uv", "uv 0.12.20rc1", False), ("uv", "uv 0.12.20.dev1", False),
                                     ("uv", "uv 0.12.20.post1", False), ("uv", "uv 0.12.20+local", False),
                                     ("typst", "typst 0.15.1 (9dfd3a08)", True), ("typst", "typst 0.14.2", False),
                                     ("typst", "typst 0.15.10", False), ("jq", "jq-1.8.2", True),
                                     ("jq", "jq-1.8.1", False), ("jq", "jq-1.8.2rc1", False)):
                bootstrap.subprocess.run = lambda *a, **k: subprocess.CompletedProcess(a, 0, output, "")
                with self.subTest(tool=tool, output=output):
                    self.assertEqual(bootstrap.run_probe(bootstrap.entry_for(lock, tool))[0], ok)
        finally:
            bootstrap.resolve_executable, bootstrap.subprocess.run = saved

    def test_linked_directories_are_detected_portably(self):
        with tempfile.TemporaryDirectory() as td:
            real = Path(td) / "real"
            real.mkdir()
            try:
                os.symlink(real, Path(td) / "link", target_is_directory=True)
            except (OSError, NotImplementedError):
                self.skipTest("symlinks not available")
            self.assertTrue(bootstrap._is_link(Path(td) / "link"))
            self.assertFalse(bootstrap._is_link(real))
            self.assertTrue(install_guard._is_link(Path(td) / "link"))

    def test_ambient_package_sources_are_ignored(self):
        names = {"PIP_INDEX_URL": "https://mirror.invalid/simple", "PIP_EXTRA_INDEX_URL": "https://mirror.invalid/x",
                 "UV_INDEX_URL": "https://mirror.invalid/simple", "UV_DEFAULT_INDEX": "https://mirror.invalid/d",
                 "npm_config_registry": "https://mirror.invalid/", "NPM_CONFIG_@scope:registry": "https://mirror.invalid/",
                 "PYTHONPATH": "/tmp/inject", "NODE_OPTIONS": "--require /tmp/inject.js",
                 "UV_CONSTRAINT": "/tmp/c.txt", "UV_BUILD_CONSTRAINT": "/tmp/b.txt", "UV_OVERRIDE": "/tmp/o.txt",
                 "PIP_CONSTRAINT": "/tmp/c.txt", "npm_config_before": "2020-01-01"}
        saved = {k: os.environ.get(k) for k in names}
        os.environ.update(names)
        try:
            env = bootstrap.install_env()
        finally:
            for k, v in saved.items():
                if v is None:
                    os.environ.pop(k, None)
                else:
                    os.environ[k] = v
        for key, value in names.items():
            with self.subTest(key=key):
                self.assertNotEqual(env.get(key), value)
        self.assertEqual(env["npm_config_registry"], bootstrap.NPM_REGISTRY)
        self.assertEqual(env["PIP_CONFIG_FILE"], os.devnull)
        self.assertEqual(env["UV_NO_CONFIG"], "1")
        calls = []
        saved_run = bootstrap._run
        bootstrap._run = lambda cmd, cwd=ROOT, env=None: calls.append(cmd)
        try:
            lock = bootstrap.load_lock()
            saved_removals = bootstrap.sync_removals
            bootstrap.sync_removals = lambda entry, sync: []
            try:
                bootstrap.install_entry(bootstrap.entry_for(lock, "uv"))
                bootstrap.install_entry(bootstrap.entry_for(lock, "marimo"))
            finally:
                bootstrap.sync_removals = saved_removals
        finally:
            bootstrap._run = saved_run
        self.assertIn("--isolated", calls[0])
        self.assertEqual(calls[0][calls[0].index("--index-url") + 1], bootstrap.PYPI_INDEX)
        sync = calls[-1]
        self.assertIn("--no-config", sync)
        self.assertEqual(sync[sync.index("--index-url") + 1], bootstrap.PYPI_INDEX)

    def test_git_downloads_ignore_ambient_config_and_hooks(self):
        extra = {"GIT_CONFIG_COUNT": "2", "GIT_CONFIG_KEY_0": "url.https://mirror.invalid/.insteadOf",
                 "GIT_CONFIG_VALUE_0": "https://github.com/", "GIT_CONFIG_KEY_1": "core.hooksPath",
                 "GIT_CONFIG_VALUE_1": "/tmp/evil-hooks"}
        saved = {k: os.environ.get(k) for k in extra}
        os.environ.update(extra)
        try:
            with tempfile.TemporaryDirectory() as td:
                empty = Path(td)
                env = bootstrap.git_env(empty)
                self.assertFalse([k for k in env if k.upper().startswith("GIT_CONFIG_") and k.upper() not in ("GIT_CONFIG_NOSYSTEM", "GIT_CONFIG_GLOBAL")])
                with mock.patch.object(subprocess, "run", return_value=subprocess.CompletedProcess([], 1, "", "")) as spawn:
                    bootstrap.refuse_git_rewrites("https://github.com/owner/repo", env)  # the ambient rewrite is gone
                    self.assertEqual(spawn.call_args.kwargs["env"], env)
                    spawn.return_value = subprocess.CompletedProcess([], 0, "url.https://mirror.invalid/.insteadof https://github.com/\n", "")
                    with self.assertRaises(bootstrap.Refused):
                        bootstrap.refuse_git_rewrites("https://github.com/owner/repo", env)
                cmd = bootstrap.git_cmd(empty, "config", "--get", "core.hooksPath")
                self.assertIn(f"core.hooksPath={empty}", cmd)
        finally:
            for k, v in saved.items():
                if v is None:
                    os.environ.pop(k, None)
                else:
                    os.environ[k] = v

    def test_real_git_sees_no_ambient_rewrite_or_hooks(self):
        extra = {"GIT_CONFIG_COUNT": "2", "GIT_CONFIG_KEY_0": "url.https://mirror.invalid/.insteadOf",
                 "GIT_CONFIG_VALUE_0": "https://github.com/", "GIT_CONFIG_KEY_1": "core.hooksPath",
                 "GIT_CONFIG_VALUE_1": "/tmp/evil-hooks"}
        with mock.patch.dict(os.environ, extra), tempfile.TemporaryDirectory() as td, real_processes(self):
            empty = Path(td)
            env = bootstrap.git_env(empty)
            bootstrap.refuse_git_rewrites("https://github.com/owner/repo", env)  # the ambient rewrite is gone
            hooks = subprocess.run(bootstrap.git_cmd(empty, "config", "--get", "core.hooksPath"), env=env, cwd=td,
                                   capture_output=True, text=True).stdout.strip()
            self.assertEqual(hooks, str(empty))
            with self.assertRaises(bootstrap.Refused):
                bootstrap.refuse_git_rewrites("https://github.com/owner/repo", dict(env, **{
                    "GIT_CONFIG_COUNT": "1", "GIT_CONFIG_KEY_0": "url.https://mirror.invalid/.insteadOf",
                    "GIT_CONFIG_VALUE_0": "https://github.com/"}))

    def test_npm_install_refuses_links_and_unapproved_npm_inputs(self):
        entry = dict(bootstrap.entry_for(bootstrap.load_lock(), "mermaid-cli"))
        calls = []
        saved = (bootstrap._run, bootstrap.ROOT)
        with tempfile.TemporaryDirectory() as td, tempfile.TemporaryDirectory() as outside:
            root = Path(td).resolve()
            prefix = root / entry["prefix"]
            prefix.mkdir(parents=True)
            for rel in ("package.json", "package-lock.json"):
                (prefix / rel).write_text("{}\n")
            bootstrap._run = lambda cmd, cwd=ROOT, env=None: calls.append(cmd)
            bootstrap.ROOT = root
            try:
                for extra in ("npm-shrinkwrap.json", ".npmrc"):
                    (prefix / extra).write_text("{}\n")
                    with self.subTest(extra=extra), self.assertRaises(bootstrap.Refused):
                        bootstrap.install_entry(entry)
                    (prefix / extra).unlink()
                try:
                    os.symlink(outside, prefix / "node_modules", target_is_directory=True)
                except (OSError, NotImplementedError):
                    self.skipTest("symlinks not available")
                with self.assertRaises(bootstrap.Refused):
                    bootstrap.install_entry(entry)
                (prefix / "node_modules").unlink()
                if bootstrap.shutil.which("npm"):
                    bootstrap.install_entry(entry)  # clean fixture: npm is bound to the checked prefix
                    self.assertIn(f"--prefix={prefix}", calls[-1])
                    self.assertIn("--workspaces=false", calls[-1])
                    calls.pop()
            finally:
                bootstrap._run, bootstrap.ROOT = saved
            self.assertEqual(os.listdir(outside), [])
        self.assertEqual(calls, [])

    def test_helpers_are_loaded_from_source_not_bytecode(self):
        self.assertFalse(hasattr(sys.modules["repo_digest"], "__cached__") and sys.modules["repo_digest"].__cached__)
        for rel in ("tools/toolchain/bootstrap.py", "tools/skills/sync.py"):
            text = (ROOT / rel).read_text(encoding="utf-8")
            with self.subTest(rel=rel):
                self.assertNotIn("from repo_digest import", text)
                self.assertIn('_load_source("repo_digest"', text)

    def test_tree_digest_order_is_platform_independent(self):
        import hashlib
        from repo_digest import file_sha256, tree_digest
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            # Path ordering differs from string ordering here (and is case-insensitive on Windows).
            for rel in ("SKILL.md", "agents/openai.yaml", "a-b/x.md", "a/y.md", "Z.md"):
                (root / rel).parent.mkdir(parents=True, exist_ok=True)
                (root / rel).write_text(rel + "\n")
            h = hashlib.sha256()
            for rel in sorted(["SKILL.md", "agents/openai.yaml", "a-b/x.md", "a/y.md", "Z.md"]):
                h.update(f"{rel}\0{file_sha256(root / rel)}\n".encode())
            self.assertEqual(tree_digest(root), h.hexdigest())

    def test_links_inside_digested_trees_are_refused(self):
        from repo_digest import tree_digest
        with tempfile.TemporaryDirectory() as td, tempfile.TemporaryDirectory() as outside:
            (Path(outside) / "private.txt").write_text("private\n")
            skill = Path(td) / "skill"
            skill.mkdir()
            (skill / "SKILL.md").write_text("x\n")
            before = tree_digest(skill)
            try:
                os.symlink(outside, skill / "linked", target_is_directory=True)
            except (OSError, NotImplementedError):
                self.skipTest("symlinks not available")
            with self.assertRaises(ValueError):
                tree_digest(skill)
            (skill / "linked").unlink()
            self.assertEqual(tree_digest(skill), before)

    def test_linked_skill_destinations_are_refused_before_download(self):
        with tempfile.TemporaryDirectory() as outside, tempfile.TemporaryDirectory() as td, \
                mock.patch.object(bootstrap, "ROOT", Path(td).resolve()):
            lock_path = Path(td) / "skills.lock.json"
            lock_path.write_text(json.dumps({"skills": {"zz-link-test": {
                "origin": "third-party", "repo": "https://github.com/owner/repo", "commit": "0" * 40,
                "subdir": "x", "digest": "0" * 64}}}), encoding="utf-8")
            link = bootstrap.ROOT / ".claude" / "skills" / "zz-link-test"
            link.parent.mkdir(parents=True)
            try:
                os.symlink(outside, link, target_is_directory=True)
            except (OSError, NotImplementedError):
                self.skipTest("symlinks not available")
            saved = (bootstrap.SKILLS_LOCK_PATH, bootstrap._run)
            calls = []
            bootstrap.SKILLS_LOCK_PATH, bootstrap._run = lock_path, (lambda cmd, cwd=ROOT, env=None: calls.append(cmd))
            try:
                with self.assertRaises(bootstrap.Refused):
                    bootstrap.install_skill("zz-link-test")
                sys.path.insert(0, str(ROOT / "tools" / "skills"))
                import sync as skills_sync
                with mock.patch.object(skills_sync, "ROOT", bootstrap.ROOT):
                    with self.assertRaises(SystemExit):
                        skills_sync.contained(link)
                    with self.assertRaises(SystemExit):
                        skills_sync.contained(link / "SKILL.md")
            finally:
                bootstrap.SKILLS_LOCK_PATH, bootstrap._run = saved
                link.unlink()
            self.assertEqual(calls, [])
            self.assertEqual(os.listdir(outside), [])

    def test_requirements_outside_repository_are_refused(self):
        with self.assertRaises(bootstrap.Refused):
            bootstrap._require_file("../outside.txt")


class ArchiveLockTests(unittest.TestCase):
    def test_qt_and_extractor_entries_and_lockfile_format(self):
        lock = bootstrap.load_lock()
        ids = [e["id"] for e in lock["tools"]]
        self.assertEqual(ids[ids.index("aqtinstall"):ids.index("aqtinstall") + 3], ["aqtinstall", "py7zr", "qt-6-10-3"])
        self.assertEqual(bootstrap.entry_for(lock, "py7zr")["version"], "1.1.3")
        qt = bootstrap.entry_for(lock, "qt-6-10-3")
        self.assertEqual(qt["method"], "archive-7z-hashed")
        self.assertEqual([a["name"] for a in qt["archives"]], ["qtbase.7z", "qtshadertools.7z", "qtsvg.7z", "qtdeclarative.7z"])
        self.assertEqual(sum(a["bytes"] for a in qt["archives"]), 203969690)
        self.assertEqual(bootstrap.LOCK_PATH.read_bytes(), (json.dumps(lock, indent=2) + "\n").encode())
        self.assertIn("tools/qt/", (ROOT / ".gitignore").read_text().splitlines())

    def test_malformed_archive_fields_are_rejected(self):
        lock = bootstrap.load_lock()
        good = bootstrap.entry_for(lock, "qt-6-10-3")
        cases = []

        def changed(field, value):
            entry = copy.deepcopy(good)
            entry[field] = value
            cases.append((field + "=" + repr(value), entry))

        for field in {"prefix", "root", "base_url", "redirects", "archives", "max_unpacked_bytes", "required_files", "prefix_check", "extractor", "requires"}:
            entry = copy.deepcopy(good)
            del entry[field]
            cases.append(("missing " + field, entry))
        for value in ([], ["linux"], ["windows", "linux"], "windows"):
            changed("platforms", value)
        for field in ("prefix", "root"):
            for value in (None, 1, "", "/absolute", "C:/qt", "a\\b", "a//b", "a/../b", "a/", "../a", "a/./b"):
                changed(field, value)
        for value in (None, "http://download.qt.io/", "https://download.qt.io", "https://download.qt.io/path/",
                      "https://user@download.qt.io/", "https://download.qt.io:443/", "https://download.qt.io/?secret"):
            changed("base_url", value)
        changed("network_hosts", [])
        changed("network_hosts", "download.qt.io")
        for host in ("127.0.0.1", "localhost", "a.localhost", "internal"):
            entry = copy.deepcopy(good)
            entry["base_url"], entry["network_hosts"] = f"https://{host}/", [host]
            cases.append(("listed local host " + host, entry))
        for value in (None, {}, dict(good["redirects"], extra=1), dict(good["redirects"], scheme="http"),
                      dict(good["redirects"], same_path=False), dict(good["redirects"], same_path=1)):
            changed("redirects", value)
        for hops in (-1, 11, 1.5, True, "5"):
            changed("redirects", dict(good["redirects"], max_hops=hops))
        for value in (None, [], {}, ["bad"], [dict(good["archives"][0], extra=1)]):
            changed("archives", value)
        changed("archives", [good["archives"][0], good["archives"][0]])
        for field in good["archives"][0]:
            archive = dict(good["archives"][0])
            del archive[field]
            changed("archives", [archive])
        archive_cases = {
            "name": [None, "", "Qt.7z", "../a.7z", "a.zip", "a.7z\n"],
            "path": [None, "", "/a", "a//b", "a/../b", "a/./b", "C:/a", "a\\b", "a?secret", "a#fragment", "a space/b"],
            "bytes": [None, 0, -1, True, 1.0, "1"],
            "sha256": [None, "a" * 63, "A" * 64, "g" * 64, "a" * 64 + "\n"],
            "install_path": [None, "other/bin", "6.10.30/bin", "6.10.3/../outside", "6.10.3//bin", "6.10.3/bin/"]
        }
        for field, values in archive_cases.items():
            for value in values:
                changed("archives", [dict(good["archives"][0], **{field: value})])
        for value in (None, 0, -1, True, 4.0, "4"):
            changed("max_unpacked_bytes", value)
        for value in (None, [], "6.10.3/bin/a", [None], ["other/bin/a"], ["6.10.3/../a"], ["6.10.3//a"], ["6.10.3"]):
            changed("required_files", value)
        for value in (None, {}, dict(good["prefix_check"], extra=True), {"command": [], "path": "{prefix}/bin"},
                      {"command": [None], "path": "{prefix}/bin"}, {"command": ["qmake"], "path": "{prefix}/bin"},
                      {"command": "{prefix}/qmake", "path": "{prefix}/bin"},
                      {"command": ["{prefix}/bin/qmake", 1], "path": "{prefix}/bin"},
                      dict(good["prefix_check"], path=None), dict(good["prefix_check"], path="other/bin"),
                      dict(good["prefix_check"], path="{prefix}/../outside")):
            changed("prefix_check", value)
        for value in (None, "absent", "uv"):
            changed("extractor", value)
        for value in (None, [], "py7zr", ["uv"]):
            changed("requires", value)
        saved = bootstrap.LOCK_PATH
        with tempfile.TemporaryDirectory() as td:
            bootstrap.LOCK_PATH = Path(td) / "lock.json"
            try:
                for name, candidate in cases:
                    trial = copy.deepcopy(lock)
                    trial["tools"][trial["tools"].index(bootstrap.entry_for(trial, "qt-6-10-3"))] = candidate
                    bootstrap.LOCK_PATH.write_text(json.dumps(trial), encoding="utf-8")
                    with self.subTest(case=name), self.assertRaisesRegex(SystemExit, r"lockfile: qt-6-10-3 "):
                        bootstrap.load_lock()
            finally:
                bootstrap.LOCK_PATH = saved


def member(name, size=0, directory=False, **extra):
    return dict({"name": name, "dir": directory, "file": not directory, "symlink": False,
                 "junction": False, "socket": False, "size": size}, **extra)


class ArchiveMemberTests(unittest.TestCase):
    def test_clean_nested_members(self):
        self.assertEqual(bootstrap.member_problems([member("bin", directory=True), member("bin/qmake.exe", 3),
                                                  member("lib\\cmake\\Qt6Config.cmake", 4)], "6.10.3/msvc2022_64"), [])

    def test_every_unsafe_member_rule(self):
        cases = [[member(name)] for name in ("", None, 1, "a\x00b", "a\x01b", "a\x7fb", "a\x85b", "/bin/a", "\\bin\\a",
                  "C:/bin/a", "\\\\server\\share\\a", "a//b", "a/./b", "a/../b", "a/", "a./b", "a /b")]
        cases += [[member("a" + c + "b")] for c in '<>:"|?*']
        cases += [[member("bin/" + name)] for name in ("CON", "prn.txt", "AuX", "NUL.exe", "com1.dll", "COM9", "lpt1", "LPT9.txt")]
        cases += [[member("bin/a", **flags)] for flags in ({"dir": True}, {"file": False}, {"symlink": True}, {"junction": True},
                                                          {"socket": True}, {"file": 1}, {"size": -1}, {"size": True})]
        cases += [[member("Bin/A"), member("bin\\a")], ["bad object"]]
        for listing in cases:
            with self.subTest(listing=listing):
                self.assertTrue(bootstrap.member_problems(listing, "6.10.3/msvc2022_64"))
        self.assertTrue(bootstrap.member_problems({}, "6.10.3"))
        self.assertEqual(len(bootstrap.member_problems([member("../bad")] * 50, "6.10.3")), 20)


class CountingStream(io.BytesIO):
    def __init__(self, body):
        super().__init__(body)
        self.read_bytes = 0

    def read(self, n):
        chunk = super().read(n)
        self.read_bytes += len(chunk)
        return chunk


class ArchiveInstallTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix="archives-")
        self.base = Path(self.tmp.name)
        self.entry = copy.deepcopy(bootstrap.entry_for(bootstrap.load_lock(), "qt-6-10-3"))
        state = bootstrap.approval_state()  # the real installer revision, read before the root moves
        self.enterContext(mock.patch.object(bootstrap, "approval_state", lambda: state))
        self.enterContext(mock.patch.object(bootstrap, "ROOT", self.base))  # fixtures stay outside the repository
        self.entry["prefix"] = (self.base.relative_to(bootstrap.ROOT) / "qt").as_posix()
        self.bodies = {"base.7z": b"pinned base archive", "addon.7z": b"pinned addon archive"}
        self.entry["archives"] = [{"name": name, "path": "repository/qt/" + name, "bytes": len(body),
                                   "sha256": hashlib.sha256(body).hexdigest(), "install_path": "6.10.3/msvc2022_64"}
                                  for name, body in self.bodies.items()]
        self.entry["max_unpacked_bytes"] = 4096
        self.entry["required_files"] = ["6.10.3/msvc2022_64/" + path for path in
                                        ("bin/qmake.exe", "bin/Qt6Core.dll", "plugins/platforms/qwindows.dll")]
        self.listings = {"base.7z": [member("bin", directory=True), member("bin/qmake.exe", 3),
                                      member("bin/Qt6Core.dll", 4), member("bin/qt.conf", 2)],
                         "addon.7z": [member("bin/Qt6Core.dll", 6), member("plugins/platforms/qwindows.dll", 5)]}
        self.calls, self.requests, self.streams, self.records, self.events = [], [], [], [], []
        self.damage, self.exit_code, self.list_output = None, 0, None
        self.saved = (bootstrap.PLATFORM, bootstrap._http_get, bootstrap.subprocess.run, bootstrap.venv_contained,
                      bootstrap.venv_conflicts, bootstrap.shutil.disk_usage, bootstrap.ledger)
        bootstrap.PLATFORM = "windows"
        bootstrap._http_get = self.fake_http
        bootstrap.subprocess.run = self.fake_child
        bootstrap.venv_contained = lambda e: []
        bootstrap.venv_conflicts = lambda e: []
        bootstrap.shutil.disk_usage = lambda p: SimpleNamespace(free=100000)
        bootstrap.ledger = self.records.append

    def tearDown(self):
        (bootstrap.PLATFORM, bootstrap._http_get, bootstrap.subprocess.run, bootstrap.venv_contained,
         bootstrap.venv_conflicts, bootstrap.shutil.disk_usage, bootstrap.ledger) = self.saved
        self.tmp.cleanup()

    @property
    def prefix(self):
        return bootstrap.ROOT / self.entry["prefix"]

    @property
    def final(self):
        return self.prefix / self.entry["root"]

    def response(self, body, status=200, headers=None):
        stream = CountingStream(body)
        self.streams.append(stream)
        return status, headers or {}, stream

    def fake_http(self, url, timeout):
        self.assertTrue(0 < timeout <= bootstrap.INSTALL_TIMEOUT)
        self.requests.append(url)
        self.events.append("get")
        body = self.bodies[url.rsplit("/", 1)[-1]]
        return self.response(body, headers={"Content-Length": str(len(body)), "X-Checksum-SHA256": hashlib.sha256(body).hexdigest()})

    def fake_child(self, cmd, **kwargs):
        self.calls.append((cmd, kwargs))
        self.assertEqual(cmd[1:4], ["-I", "-c", bootstrap.EXTRACT_CODE])
        self.assertEqual(kwargs["cwd"], self.prefix / f".st-{os.getpid()}")
        self.assertEqual(kwargs["timeout"], bootstrap.INSTALL_TIMEOUT)
        self.assertEqual(kwargs["env"], bootstrap.probe_env())
        self.assertTrue(kwargs["capture_output"])
        mode, name = cmd[4], Path(cmd[5]).name
        self.events.append(mode)
        if self.exit_code:
            return subprocess.CompletedProcess(cmd, self.exit_code, "", "")
        if mode == "list":
            # Every archive must already have passed its pinned hash before any child.
            for archive in self.entry["archives"]:
                data = (Path(kwargs["cwd"]) / "dl" / archive["name"]).read_bytes()
                self.assertEqual(hashlib.sha256(data).hexdigest(), archive["sha256"])
            return subprocess.CompletedProcess(cmd, 0, self.list_output or json.dumps(self.listings[name]), "")
        self.assertEqual(mode, "extract")
        target = Path(cmd[6])
        for item in self.listings[name]:
            path = target / item["name"].replace("\\", "/")
            if item["dir"]:
                path.mkdir(parents=True, exist_ok=True)
            else:
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(b"x" * item["size"])
        if name == self.entry["archives"][-1]["name"] and self.damage:
            self.damage(target)
        return subprocess.CompletedProcess(cmd, 0, "", "")

    def assert_clean_refusal(self, text):
        with self.assertRaisesRegex(bootstrap.Refused, text):
            bootstrap.install_entry(self.entry)
        self.assertFalse(self.final.exists())
        self.assertEqual(list(self.prefix.glob(".st-*")), [])
        self.assertEqual(self.records, [])

    def test_download_accepts_direct_and_two_hop_mirror_and_records_host(self):
        archive = self.entry["archives"][0]
        start = self.entry["base_url"] + archive["path"]
        middle = "https://redirect.example.net/" + archive["path"]
        mirror = "https://mirror.example.org/sites/qt.io/" + archive["path"]
        for i, redirects in enumerate(({}, {start: (302, middle), middle: (307, mirror)})):
            dest = self.base / str(i)
            dest.mkdir()
            seen = []

            def get(url, timeout):
                seen.append(url)
                self.assertTrue(0 < timeout <= bootstrap.INSTALL_TIMEOUT)
                if url in redirects:
                    status, location = redirects[url]
                    return self.response(b"", status, {"Location": location})
                return self.response(self.bodies[archive["name"]])

            bootstrap._http_get = get
            host = bootstrap.fetch_archive(self.entry, archive, dest)
            self.assertEqual(host, "mirror.example.org" if redirects else "download.qt.io")
            self.assertEqual(seen, [start, middle, mirror] if redirects else [start])
            self.assertEqual((dest / archive["name"]).read_bytes(), self.bodies[archive["name"]])
            self.assertTrue(all(s.closed for s in self.streams))

    def test_download_refusals_are_offline_and_bounded(self):
        archive = self.entry["archives"][0]
        path = archive["path"]
        body = self.bodies[archive["name"]]
        bad_urls = ["http://mirror.example.org/" + path, "https://user@mirror.example.org/" + path,
                    "https://mirror.example.org:8443/" + path, "https://127.0.0.1/" + path,
                    "https://[::1]/" + path, "https://localhost/" + path, "https://a.localhost/" + path,
                    "https://mirror.example.org/" + path + "?secret=private", "https://mirror.example.org/" + path + "#secret",
                    "https://mirror.example.org/" + path + "?", "https://mirror.example.org/" + path + "#",
                    "https://mirror.example.org/elsewhere.7z", "https://mirror/" + path]
        cases = [("redirect " + str(i), 302, {"Location": url}, b"") for i, url in enumerate(bad_urls)]
        cases += [("404", 404, {}, b""), ("206", 206, {}, body),
                  ("length", 200, {"Content-Length": str(len(body) + 1)}, body),
                  ("short", 200, {}, body[:-1]), ("long", 200, {}, body + b"z" * 100000)]
        for i, (name, status, headers, data) in enumerate(cases):
            dest = self.base / str(i)
            dest.mkdir()
            calls = []

            def get(url, timeout):
                calls.append(url)
                return self.response(data, status, headers)

            bootstrap._http_get = get
            with self.subTest(case=name), self.assertRaises(bootstrap.Refused) as ctx:
                bootstrap.fetch_archive(self.entry, archive, dest)
            self.assertIn(archive["name"], str(ctx.exception))
            self.assertIn("installation blocked until verified again", str(ctx.exception))
            self.assertNotIn("secret", str(ctx.exception))
            self.assertEqual(len(calls), 1)
            self.assertEqual(list(dest.iterdir()), [])
            self.assertTrue(self.streams[-1].closed)
            if name == "long":
                self.assertEqual(self.streams[-1].read_bytes, archive["bytes"] + 1)

    def test_download_refuses_one_redirect_beyond_max_hops(self):
        archive = self.entry["archives"][0]
        self.entry["redirects"]["max_hops"] = 2
        calls = []

        def get(url, timeout):
            calls.append(url)
            return self.response(b"", 308, {"Location": f"https://mirror.example.org/hop{len(calls)}/" + archive["path"]})

        bootstrap._http_get = get
        with self.assertRaisesRegex(bootstrap.Refused, "redirect policy refused"):
            bootstrap.fetch_archive(self.entry, archive, self.base)
        self.assertEqual(len(calls), 3)
        self.assertTrue(all(s.closed for s in self.streams))

    def test_hash_pin_wins_over_server_checksum_before_any_extractor(self):
        self.bodies["base.7z"] = b"z" * len(self.bodies["base.7z"])
        contained = []
        bootstrap.venv_contained = lambda e: contained.append(e) or []
        self.assert_clean_refusal("SHA-256 differs from the pin")
        self.assertEqual(self.calls, [])
        self.assertEqual(contained, [])

    def test_download_deadline_closes_stream_and_removes_partial_body(self):
        saved, clock = bootstrap.time.monotonic, [0.0]
        bootstrap.time.monotonic = lambda: clock[0]

        def late(url, timeout):
            status, headers, stream = self.fake_http(url, timeout)
            read = stream.read
            stream.read = lambda n: clock.__setitem__(0, bootstrap.INSTALL_TIMEOUT + 1) or read(n)
            return status, headers, stream

        bootstrap._http_get = late
        try:
            with self.assertRaisesRegex(bootstrap.Refused, "base.7z: download deadline exceeded"):
                bootstrap.fetch_archive(self.entry, self.entry["archives"][0], self.base)
        finally:
            bootstrap.time.monotonic = saved
        self.assertTrue(self.streams[-1].closed)
        self.assertEqual(list(self.base.iterdir()), [])

    def test_trickling_server_is_cut_off_at_the_deadline(self):
        # Every receive stays inside the socket timeout, so only the deadline can end the buffered read.
        body = b"q" * 64
        archive = dict(self.entry["archives"][0], bytes=len(body), sha256=hashlib.sha256(body).hexdigest())
        head = b"HTTP/1.1 200 OK\r\nContent-Length: 64\r\n\r\n"
        clock = [0.0]

        class Trickle:
            """Socket double: after `fast` bytes, one byte per 59 simulated seconds."""

            def __init__(self, data, fast):
                self.data, self.fast, self.timeout = data, fast, None

            def settimeout(self, value):
                self.timeout = value

            def recv_into(self, buffer, nbytes=None, flags=0):
                if self.fast <= 0:
                    if self.timeout < 59:
                        clock[0] += self.timeout
                        raise TimeoutError("timed out")
                    clock[0] += 59
                self.fast -= 1
                if not self.data:
                    return 0
                buffer[0], self.data = self.data[0], self.data[1:]
                return 1

            def makefile(self, mode):
                return io.BufferedReader(socket.SocketIO(self, mode))

            def _decref_socketios(self):
                pass

        saved = bootstrap.time.monotonic
        bootstrap.time.monotonic = lambda: clock[0]
        try:
            for fast, phase in ((0, "headers"), (len(head), "body")):
                clock[0] = 0.0

                def get(url, timeout):
                    sock = type("Sock", (bootstrap._DeadlineSocket, Trickle), {"deadline": clock[0] + timeout})(head + body, fast)
                    response = http.client.HTTPResponse(sock, method="GET")
                    response.begin()
                    return response.status, response.headers, response

                bootstrap._http_get = get
                with self.subTest(phase=phase):
                    with self.assertRaisesRegex(bootstrap.Refused, "download deadline exceeded"):
                        bootstrap.fetch_archive(self.entry, archive, self.base)
                    self.assertEqual(clock[0], bootstrap.INSTALL_TIMEOUT)
                    self.assertEqual(list(self.base.iterdir()), [])
        finally:
            bootstrap.time.monotonic = saved

    def test_repeated_length_or_location_is_refused(self):
        # http.client frames the body by the first Content-Length; a dict copy of the headers keeps the last.
        body = b"abc"
        archive = dict(self.entry["archives"][0], bytes=len(body), sha256=hashlib.sha256(body).hexdigest())
        location = "Location: https://mirror.example.org/" + archive["path"] + "\r\n"
        cases = [("length", b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\nContent-Length: 3\r\n\r\nabc", "repeated Content-Length"),
                 ("location", ("HTTP/1.1 302 Found\r\n" + location * 2 + "Content-Length: 0\r\n\r\n").encode(),
                  "invalid redirect location")]
        for name, raw, reason in cases:
            responses = []

            def get(url, timeout):
                response = http.client.HTTPResponse(SimpleNamespace(makefile=lambda mode: io.BytesIO(raw)), method="GET")
                response.begin()
                responses.append(response)
                return response.status, response.headers, response

            bootstrap._http_get = get
            with self.subTest(case=name):
                with self.assertRaisesRegex(bootstrap.Refused, reason):
                    bootstrap.fetch_archive(self.entry, archive, self.base)
                self.assertEqual(len(responses), 1)
                self.assertTrue(responses[0].isclosed())
                self.assertEqual(list(self.base.iterdir()), [])

    def test_initial_url_policy_applies_before_any_request(self):
        archive = self.entry["archives"][0]
        for host in ("127.0.0.1", "localhost", "a.localhost", "internal"):
            self.entry["base_url"], self.entry["network_hosts"] = f"https://{host}/", [host]
            with self.subTest(host=host), self.assertRaisesRegex(bootstrap.Refused, "URL policy refused"):
                bootstrap.fetch_archive(self.entry, archive, self.base)
        self.assertEqual(self.requests, [])

    def test_transport_exception_is_sanitized_and_blocks_extraction(self):
        def failed_get(url, timeout):
            raise bootstrap.HTTPException("https://mirror.example.org/?secret=private")

        bootstrap._http_get = failed_get
        self.assert_clean_refusal("base.7z: download failed")
        self.assertEqual(self.calls, [])

    def test_success_checks_tree_writes_qt_conf_moves_and_logs(self):
        saved = bootstrap.os.rename

        def move(src, dst):
            if dst == self.final:
                self.events.append("move")
                self.assertEqual(self.events, ["get", "get", "list", "list", "extract", "extract", "move"])
            saved(src, dst)

        bootstrap.os.rename = move
        try:
            bootstrap.install_entry(self.entry)
        finally:
            bootstrap.os.rename = saved
        self.assertEqual((self.final / "msvc2022_64/bin/qt.conf").read_bytes(), b"[Paths]\r\nPrefix=..\r\n")
        self.assertEqual((self.final / "msvc2022_64/bin/Qt6Core.dll").stat().st_size, 6)
        self.assertEqual(list(self.prefix.glob(".st-*")), [])
        extractor = bootstrap.entry_for(bootstrap.load_lock(), "py7zr")
        interpreter = bootstrap.ROOT / extractor["venv"] / "Scripts/python.exe"
        for cmd, kwargs in self.calls:
            self.assertEqual(cmd[0], str(interpreter))
            self.assertNotIn(str(self.final), cmd[0])
            self.assertNotIn(str(kwargs["cwd"]), cmd[0])
        self.assertEqual(len(self.records), 1)
        record = self.records[0]
        self.assertEqual(record["phase"], "archives")
        self.assertEqual(record["tool"], self.entry["id"])
        self.assertEqual(record["overlaps"], 1)
        self.assertEqual(record["qt_conf"], "replaced")
        self.assertEqual(record["archives"], [{"name": a["name"], "bytes": a["bytes"], "host": "download.qt.io"} for a in self.entry["archives"]])
        self.assertEqual(record["longest_path"], max(len(str(p)) for p in self.final.rglob("*")))
        self.assertGreaterEqual(record["seconds"], 0)

    def test_success_creates_qt_conf_when_not_listed(self):
        self.listings["base.7z"] = [m for m in self.listings["base.7z"] if m["name"] != "bin/qt.conf"]
        bootstrap.install_archives(self.entry)
        self.assertEqual(self.records[0]["qt_conf"], "created")
        self.assertEqual((self.final / "msvc2022_64/bin/qt.conf").read_bytes(), b"[Paths]\r\nPrefix=..\r\n")


    def test_extracted_symlink_is_refused_and_staging_is_removed(self):
        sentinel = self.base / "sentinel"
        sentinel.write_bytes(b"untouched")
        link = self.base / "link"
        try:
            os.symlink(sentinel, link)
        except (OSError, NotImplementedError):
            self.skipTest("symlinks not available")
        link.unlink()
        self.damage = lambda target: os.symlink(sentinel, target / "link")
        self.assert_clean_refusal("contains a link")
        self.assertEqual(sentinel.read_bytes(), b"untouched")

    def test_unlisted_extracted_file_is_refused(self):
        self.damage = lambda target: (target / "unlisted").write_bytes(b"bad")
        self.assert_clean_refusal("file set differs")

    def test_read_only_staging_files_are_cleaned_on_failure(self):
        sentinel = self.base / "sentinel"
        sentinel.write_bytes(b"untouched")
        before = sentinel.stat().st_mode

        def damaged(target):
            (target / "unlisted").write_bytes(b"bad")
            os.chmod(target / "bin/qmake.exe", stat.S_IREAD)

        self.damage = damaged
        self.assert_clean_refusal("file set differs")
        self.assertEqual(sentinel.read_bytes(), b"untouched")
        self.assertEqual(sentinel.stat().st_mode, before)

    def test_extracted_junction_is_refused_without_descending(self):
        saved = bootstrap._is_link
        linked = self.prefix / f".st-{os.getpid()}/x/6.10.3/msvc2022_64/bin"
        bootstrap._is_link = lambda p: Path(p) == linked or saved(p)
        try:
            self.assert_clean_refusal("contains a link")
        finally:
            bootstrap._is_link = saved

    def test_extracted_size_mismatch_is_refused(self):
        self.damage = lambda target: (target / "bin/qmake.exe").write_bytes(b"bad size")
        self.assert_clean_refusal("file size differs")

    def test_other_root_in_extracted_tree_is_refused(self):
        self.damage = lambda target: (target.parents[1] / "outside").mkdir()
        self.assert_clean_refusal("outside root")

    def test_existing_destination_is_refused_before_download(self):
        self.final.mkdir(parents=True)
        with self.assertRaisesRegex(bootstrap.Refused, "destination already exists"):
            bootstrap.install_archives(self.entry)
        self.assertEqual(self.requests, [])
        self.assertEqual(self.calls, [])

    def test_too_little_space_is_refused_before_any_request(self):
        bootstrap.shutil.disk_usage = lambda p: SimpleNamespace(free=1)
        self.assert_clean_refusal("too little free space")
        self.assertEqual(self.requests, [])
        self.assertEqual(self.calls, [])

    def test_unmarked_leftover_is_preserved_and_refused(self):
        leftover = self.prefix / ".st-old"
        leftover.mkdir(parents=True)
        (leftover / "sentinel").write_bytes(b"owner")
        with self.assertRaisesRegex(bootstrap.Refused, r"unexpected leftover .st-old; remove it or ask the owner"):
            bootstrap.install_archives(self.entry)
        self.assertEqual((leftover / "sentinel").read_bytes(), b"owner")
        self.assertEqual(self.requests, [])

    def test_marked_leftover_is_removed(self):
        leftover = self.prefix / ".st-old"
        leftover.mkdir(parents=True)
        (leftover / ".magic600-staging").write_bytes(b"marker")
        (leftover / "dl").mkdir()
        (leftover / "dl/partial").write_bytes(b"old")
        bootstrap.install_archives(self.entry)
        self.assertFalse(leftover.exists())
        self.assertTrue(self.final.is_dir())

    def test_marked_leftover_containing_a_link_is_preserved(self):
        leftover = self.prefix / ".st-old"
        leftover.mkdir(parents=True)
        (leftover / ".magic600-staging").write_bytes(b"marker")
        linked = leftover / "junction"
        linked.mkdir()
        saved = bootstrap._is_link
        bootstrap._is_link = lambda p: Path(p) == linked or saved(p)
        try:
            with self.assertRaisesRegex(bootstrap.Refused, "unexpected leftover"):
                bootstrap.install_archives(self.entry)
        finally:
            bootstrap._is_link = saved
        self.assertTrue(linked.is_dir())
        self.assertEqual(self.requests, [])

    def test_missing_required_file_is_refused(self):
        self.entry["required_files"].append("6.10.3/msvc2022_64/bin/missing.dll")
        self.assert_clean_refusal("missing required file")

    def test_password_archive_extractor_exit_3_is_refused(self):
        self.exit_code = 3
        self.assert_clean_refusal("base.7z: extractor exit 3")
        self.assertEqual([cmd[4] for cmd, _ in self.calls], ["list"])

    def test_unparsable_extractor_listing_is_refused(self):
        self.list_output = "not JSON"
        self.assert_clean_refusal("unparsable extractor listing")
        self.assertNotIn("extract", self.events)

    def test_unsafe_listing_is_refused_before_extraction(self):
        self.listings["addon.7z"].append(member("../escape", 10))
        self.assert_clean_refusal("unsafe path segment")
        self.assertNotIn("extract", self.events)

    def test_unpacked_limit_and_free_space_are_checked_before_extraction(self):
        self.entry["max_unpacked_bytes"] = 1
        self.assert_clean_refusal("unpacked size exceeds")
        self.assertNotIn("extract", self.events)
        self.entry["max_unpacked_bytes"] = 4096
        self.events.clear()
        spaces = iter((100000, 1))
        bootstrap.shutil.disk_usage = lambda p: SimpleNamespace(free=next(spaces))
        self.assert_clean_refusal("unpacked size exceeds")
        self.assertNotIn("extract", self.events)

    def test_conflicting_extractor_environment_is_refused(self):
        bootstrap.venv_conflicts = lambda e: ["unapproved 1.0"]
        self.assert_clean_refusal("extractor environment differs")
        self.assertEqual(self.calls, [])

    def test_archive_method_is_windows_only(self):
        bootstrap.PLATFORM = "linux"
        self.assert_clean_refusal("Windows only")
        self.assertEqual(self.requests, [])

    def test_run_probe_requires_exact_version_and_final_prefix(self):
        bootstrap.install_archives(self.entry)
        calls = []
        final_prefix = self.final / "msvc2022_64"
        for version, prefix, ok in (("6.10.3", str(final_prefix), True), ("6.10.3rc1", str(final_prefix), False),
                                    ("6.10.3", str(self.base / "other"), False)):
            calls.clear()

            def probe(cmd, **kwargs):
                calls.append(cmd)
                self.assertEqual(Path(cmd[0]), final_prefix / "bin/qmake.exe")
                self.assertEqual(kwargs["timeout"], bootstrap.PROBE_TIMEOUT)
                self.assertEqual(kwargs["env"], bootstrap.probe_env())
                output = version if cmd[-1] == "QT_VERSION" else prefix
                return subprocess.CompletedProcess(cmd, 0, output + "\n", "")

            bootstrap.subprocess.run = probe
            with self.subTest(version=version, prefix=prefix):
                result, detail = bootstrap.run_probe(self.entry)
                self.assertEqual(result, ok)
                if prefix != str(final_prefix):
                    self.assertIn("prefix check failed:", detail)
                self.assertEqual(len(calls), 1 if "rc1" in version else 2)

    def test_install_runs_probes_after_the_checked_tree_is_moved(self):
        saved = bootstrap.run_probe
        bootstrap.run_probe = lambda e: saved(e) if e["id"] == self.entry["id"] else (True, "dependency ready")
        extractor_child = self.fake_child
        probed = []

        def child_or_probe(cmd, **kwargs):
            if cmd[1:4] == ["-I", "-c", bootstrap.EXTRACT_CODE]:
                return extractor_child(cmd, **kwargs)
            self.assertTrue(self.final.is_dir())
            self.assertEqual(list(self.prefix.glob(".st-*")), [])
            self.assertEqual(Path(cmd[0]), self.final / "msvc2022_64/bin/qmake.exe")
            probed.append(cmd[-1])
            output = self.entry["version"] if cmd[-1] == "QT_VERSION" else str(self.final / "msvc2022_64")
            return subprocess.CompletedProcess(cmd, 0, output, "")

        bootstrap.subprocess.run = child_or_probe
        lock = bootstrap.load_lock()
        lock["tools"] = [self.entry if e["id"] == self.entry["id"] else e for e in lock["tools"]]
        try:
            with contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(bootstrap.install(lock, self.entry["id"], set()), "installed")
        finally:
            bootstrap.run_probe = saved
        self.assertEqual(probed, ["QT_VERSION", "QT_INSTALL_PREFIX"])
        self.assertEqual(sum(r.get("phase") == "archives" for r in self.records), 1)

    def staged_copy(self):
        bootstrap.install_archives(self.entry)
        staged = self.prefix / ".st-old/x/6.10.3"
        shutil.copytree(self.final, staged)
        return staged

    def assert_probe_refused_without_running(self):
        calls = []
        bootstrap.subprocess.run = lambda cmd, **kwargs: calls.append(cmd)
        ok, detail = bootstrap.run_probe(self.entry)
        self.assertFalse(ok)
        self.assertIn("refused", detail)
        self.assertFalse(bootstrap.present(self.entry))
        self.assertEqual(calls, [])

    def test_probe_refuses_a_linked_qmake(self):
        staged = self.staged_copy()
        qmake = self.final / "msvc2022_64/bin/qmake.exe"
        qmake.unlink()
        try:
            os.symlink(staged / "msvc2022_64/bin/qmake.exe", qmake)
        except (OSError, NotImplementedError):
            self.skipTest("symlinks not available")
        self.assert_probe_refused_without_running()

    def test_probe_refuses_a_final_root_junction_into_staging(self):
        staged = self.staged_copy()
        shutil.rmtree(self.final)
        try:
            if sys.platform == "win32":
                import _winapi
                _winapi.CreateJunction(str(staged), str(self.final))
            else:
                os.symlink(staged, self.final, target_is_directory=True)
        except (OSError, NotImplementedError):
            self.skipTest("links not available")
        try:
            self.assert_probe_refused_without_running()
        finally:
            (os.rmdir if sys.platform == "win32" else os.unlink)(self.final)  # the link only, never its target

    def install_with(self, prefix_output=None):
        saved = bootstrap.run_probe
        bootstrap.run_probe = lambda e: saved(e) if e["id"] == self.entry["id"] else (True, "dependency ready")
        extractor_child = self.fake_child

        def child_or_probe(cmd, **kwargs):
            if cmd[1:4] == ["-I", "-c", bootstrap.EXTRACT_CODE]:
                return extractor_child(cmd, **kwargs)
            output = self.entry["version"] if cmd[-1] == "QT_VERSION" else prefix_output or str(self.final / "msvc2022_64")
            return subprocess.CompletedProcess(cmd, 0, output, "")

        bootstrap.subprocess.run = child_or_probe
        lock = bootstrap.load_lock()
        lock["tools"] = [self.entry if e["id"] == self.entry["id"] else e for e in lock["tools"]]
        try:
            with contextlib.redirect_stdout(io.StringIO()):
                return bootstrap.install(lock, self.entry["id"], set())
        finally:
            bootstrap.run_probe = saved

    def test_failures_after_the_move_remove_the_new_tree(self):
        def read_only_ledger(record):
            if record.get("phase") == "archives":
                raise PermissionError("ledger is read-only")
            self.records.append(record)

        bootstrap.ledger = read_only_ledger
        with self.assertRaises(PermissionError):
            self.install_with()
        self.assertFalse(os.path.lexists(self.final))
        self.assertEqual(list(self.prefix.glob(".st-*")), [])
        bootstrap.ledger = self.records.append
        with self.assertRaisesRegex(bootstrap.Refused, "installed but the probe failed"):
            self.install_with(prefix_output=str(self.base / "other"))
        self.assertFalse(os.path.lexists(self.final))
        self.assertEqual(list(self.prefix.glob(".st-*")), [])
        self.assertEqual([r["result"] for r in self.records if "result" in r], ["probe-failed"])

    def test_existing_destination_survives_a_failed_install(self):
        self.final.mkdir(parents=True)
        (self.final / "owner.txt").write_bytes(b"owner")
        with self.assertRaisesRegex(bootstrap.Refused, "destination already exists"):
            self.install_with()
        self.assertEqual((self.final / "owner.txt").read_bytes(), b"owner")
        self.assertEqual(self.requests, [])

    def test_marked_leftover_with_a_hard_link_is_preserved(self):
        sentinel = self.base / "sentinel"
        sentinel.write_bytes(b"owner")
        leftover = self.prefix / ".st-old"
        leftover.mkdir(parents=True)
        (leftover / ".magic600-staging").write_bytes(b"marker")
        try:
            os.link(sentinel, leftover / "readonly.txt")
        except (OSError, NotImplementedError):
            self.skipTest("hard links not available")
        os.chmod(sentinel, stat.S_IREAD)
        try:
            before = os.stat(sentinel).st_mode
            with self.assertRaisesRegex(bootstrap.Refused, "unexpected leftover .st-old"):
                bootstrap.install_archives(self.entry)
            self.assertEqual((os.stat(sentinel).st_mode, os.stat(sentinel).st_nlink), (before, 2))
            self.assertEqual(sentinel.read_bytes(), b"owner")
            self.assertEqual(self.requests, [])
        finally:
            os.chmod(sentinel, stat.S_IREAD | stat.S_IWRITE)

    def test_cleanup_never_makes_a_hard_linked_file_writable(self):
        sentinel = self.base / "sentinel"
        sentinel.write_bytes(b"owner")
        top = self.base / "tree"
        top.mkdir()
        try:
            os.link(sentinel, top / "readonly.txt")
        except (OSError, NotImplementedError):
            self.skipTest("hard links not available")
        os.chmod(sentinel, stat.S_IREAD)
        try:
            if sys.platform == "win32":
                # Windows cannot delete a read-only file, and every hard link shares that attribute.
                with self.assertRaisesRegex(bootstrap.Refused, "hard-linked read-only file"):
                    bootstrap._remove_tree(top)
            else:
                bootstrap._remove_tree(top)
            self.assertEqual(sentinel.read_bytes(), b"owner")
            self.assertFalse(os.stat(sentinel).st_mode & stat.S_IWRITE)
        finally:
            os.chmod(sentinel, stat.S_IREAD | stat.S_IWRITE)


class ArchiveTransportTests(unittest.TestCase):
    def test_http_seam_uses_default_tls_fixed_headers_and_no_redirects(self):
        saved = (bootstrap.ssl.create_default_context, bootstrap.urllib.request.build_opener, bootstrap.time.monotonic)
        context, seen, deadlines = ssl.create_default_context(), [], []
        stream = CountingStream(b"")
        stream.code, stream.headers = 200, {}

        class Opener:
            def open(self, request, timeout):
                seen.append((request, timeout))
                return stream

        def build(*handlers):
            self.assertIsInstance(handlers[0], bootstrap._NoRedirect)
            self.assertIsNone(handlers[0].redirect_request(None, None, 302, "", {}, "https://mirror.example.org/"))
            self.assertIsInstance(handlers[1], bootstrap._DeadlineHTTPSHandler)
            self.assertIs(handlers[1]._context, context)
            deadlines.append(handlers[1].deadline)
            return Opener()

        bootstrap.ssl.create_default_context = lambda: context
        bootstrap.urllib.request.build_opener = build
        bootstrap.time.monotonic = lambda: 1000.0
        try:
            status, headers, response = bootstrap._http_get("https://download.qt.io/archive.7z", 1700)
            deadline = context.sslsocket_class.deadline
            bootstrap._http_get("https://download.qt.io/archive.7z", 10)
            with self.assertRaises(TimeoutError):
                bootstrap._http_get("https://download.qt.io/archive.7z", 0)
        finally:
            (bootstrap.ssl.create_default_context, bootstrap.urllib.request.build_opener, bootstrap.time.monotonic) = saved
            stream.close()
        self.assertEqual(status, 200)
        self.assertIs(response, stream)
        self.assertEqual([timeout for _, timeout in seen], [bootstrap.SOCKET_TIMEOUT, 10])
        self.assertTrue(issubclass(context.sslsocket_class, bootstrap._DeadlineSocket))
        self.assertTrue(issubclass(context.sslsocket_class, ssl.SSLSocket))
        self.assertEqual(deadline, 2700.0)
        self.assertEqual(deadlines, [2700.0, 1010.0])  # the plain socket gets the same deadline
        self.assertEqual(dict((k.lower(), v) for k, v in seen[0][0].header_items()),
                         {"accept-encoding": "identity", "user-agent": "magic600-bootstrap"})

    @real_sockets()
    def test_tls_socket_waits_no_longer_than_the_deadline(self):
        # A real TLS client socket over a local socket pair whose peer never answers the handshake.
        saved = (bootstrap.ssl.create_default_context, bootstrap.urllib.request.build_opener)
        context = ssl.create_default_context()
        stream = CountingStream(b"")
        stream.code, stream.headers = 200, {}
        bootstrap.ssl.create_default_context = lambda: context
        bootstrap.urllib.request.build_opener = lambda *handlers: SimpleNamespace(open=lambda request, timeout: stream)
        try:
            bootstrap._http_get("https://download.qt.io/archive.7z", 0.3)
        finally:
            bootstrap.ssl.create_default_context, bootstrap.urllib.request.build_opener = saved
        client, peer = socket.socketpair()
        try:
            client.settimeout(30)
            wrapped = context.wrap_socket(client, server_hostname="download.qt.io", do_handshake_on_connect=False)
            started = bootstrap.time.monotonic()
            with self.assertRaises(TimeoutError):
                wrapped.recv(1)
            self.assertLess(bootstrap.time.monotonic() - started, 10)
            while bootstrap.time.monotonic() < context.sslsocket_class.deadline:
                bootstrap.time.sleep(0.01)  # a socket timer may fire a little before the deadline
            with self.assertRaisesRegex(TimeoutError, "download deadline exceeded"):
                wrapped.do_handshake()
            wrapped.close()
        finally:
            client.close()
            peer.close()

    @real_sockets()
    def test_real_opener_obeys_the_deadline_with_and_without_a_proxy(self):
        # The real opener, proxy handler, CONNECT tunnel and TLS client; only the TCP connection is a local
        # socket pair. Through the proxy, the peer answers CONNECT with one byte every 0.2 s, 13 s for the
        # whole reply; without a proxy, it never answers the TLS handshake.
        reply = b"HTTP/1.1 407 Proxy Authentication Required\r\nContent-Length: 0\r\n\r\n"
        for proxy in (None, "http://proxy.example.org:8080"):
            client, peer = socket.socketpair()
            received, connected, stop = bytearray(), [], threading.Event()

            def serve():
                with contextlib.suppress(OSError):
                    while b"\r\n\r\n" not in received and not received.startswith(b"\x16"):
                        chunk = peer.recv(4096)
                        if not chunk:
                            return
                        received.extend(chunk)
                    for byte in reply if proxy else b"":
                        if stop.wait(0.2):
                            return
                        peer.sendall(bytes([byte]))
                    stop.wait(30)

            def create_connection(address, timeout=None, source_address=None, **kwargs):
                connected.append((address, timeout))
                return client

            saved_env = {key: os.environ.pop(key) for key in list(os.environ) if key.lower().endswith("_proxy")}
            if proxy:
                os.environ["HTTPS_PROXY"] = proxy
            saved = bootstrap.socket.create_connection
            bootstrap.socket.create_connection = create_connection
            thread = threading.Thread(target=serve, daemon=True)
            thread.start()
            try:
                started = bootstrap.time.monotonic()
                with self.assertRaises(OSError) as raised:
                    bootstrap._http_get("https://download.qt.io/repository/qt/archive.7z", 0.6)
                elapsed = bootstrap.time.monotonic() - started
            finally:
                bootstrap.socket.create_connection = saved
                os.environ.pop("HTTPS_PROXY", None)
                os.environ.update(saved_env)
                stop.set()
                thread.join(10)
                client.close()
                peer.close()
            with self.subTest(proxy=bool(proxy)):
                # urllib reports a connection-phase timeout as URLError(reason=TimeoutError).
                self.assertIsInstance(getattr(raised.exception, "reason", raised.exception), TimeoutError)
                self.assertGreaterEqual(elapsed, 0.5)  # a socket timer may fire a little before the deadline
                self.assertLess(elapsed, 5)
                if proxy:
                    self.assertEqual(connected, [(("proxy.example.org", 8080), 0.6)])
                    self.assertTrue(received.startswith(b"CONNECT download.qt.io:443 HTTP/1.1\r\n"))
                else:
                    self.assertEqual(connected, [(("download.qt.io", 443), 0.6)])
                    self.assertTrue(received.startswith(b"\x16"))  # a TLS handshake record


class AqtAndRealExtractorTests(unittest.TestCase):
    @staticmethod
    def launches_aqt(command):
        if Path(command[0]).stem.lower() == "aqt":
            return True
        for i, arg in enumerate(command[:-1]):
            if arg == "-m" and (command[i + 1] == "aqt" or command[i + 1].startswith("aqt.")):
                return True
            if arg == "-c" and re.search(r"\b(?:import\s+aqt\b|from\s+aqt(?:\.|\s))", command[i + 1]):
                return True
        return False

    def test_lockfile_never_launches_or_imports_aqt(self):
        lock = bootstrap.load_lock()
        expected = ["{venv}/python", "-I", "-c", "import importlib.metadata as m; print(m.version('aqtinstall'))"]
        self.assertEqual(bootstrap.entry_for(lock, "aqtinstall")["probe"], expected)
        self.assertEqual(bootstrap.entry_for(lock, "aqtinstall")["expect"], r"^3\.3\.0\s*$")
        for entry in lock["tools"]:
            commands = [entry["probe"]] if entry.get("probe") else []
            if entry.get("prefix_check"):
                commands.append(entry["prefix_check"]["command"])
            for command in commands:
                with self.subTest(tool=entry["id"], command=command):
                    self.assertFalse(self.launches_aqt(command))

    def test_doctor_and_renderer_profile_routing_never_launch_aqt(self):
        saved = (bootstrap.resolve_executable, bootstrap.subprocess.run, bootstrap.install_entry, bootstrap.venv_conflicts)
        calls = []
        bootstrap.resolve_executable = lambda e, n: str(ROOT / "work" / Path(n).name)
        bootstrap.subprocess.run = lambda cmd, **kwargs: calls.append(cmd) or subprocess.CompletedProcess(cmd, 1, "failed", "")
        bootstrap.install_entry = lambda e: None
        bootstrap.venv_conflicts = lambda e: []
        try:
            self.assertEqual(run(["doctor"])[0], 0)
            self.assertEqual(run(["install", "--profile", "renderer-spike"])[0], 2)
        finally:
            bootstrap.resolve_executable, bootstrap.subprocess.run, bootstrap.install_entry, bootstrap.venv_conflicts = saved
        self.assertTrue(calls)
        self.assertTrue(any("importlib.metadata" in " ".join(cmd) for cmd in calls))
        self.assertFalse(any(self.launches_aqt(cmd) for cmd in calls))

    def test_real_metadata_probe_has_no_aqt_side_effects(self):
        python = ROOT / "tools/.venv/renderer-spike/Scripts/python.exe"
        if not python.is_file():
            self.skipTest("renderer-spike interpreter is absent in this worktree")
        with tempfile.TemporaryDirectory() as td:
            base = Path(td)
            sentinel = base / "appdata/aqt/tmp/sentinel"
            sentinel.parent.mkdir(parents=True)
            sentinel.write_bytes(b"unchanged")
            config = base / "logging.ini"
            config.write_bytes(b"invalid logging config: must never be read")
            saved = (os.getcwd(), os.environ.get("APPDATA"), os.environ.get("LOG_CFG"))
            os.environ.update(APPDATA=str(base / "appdata"), LOG_CFG=str(config))
            try:
                os.chdir(base)
                with real_processes(self):
                    ok, detail = bootstrap.run_probe(bootstrap.entry_for(bootstrap.load_lock(), "aqtinstall"))
                self.assertTrue(ok, detail)
                self.assertEqual(sentinel.read_bytes(), b"unchanged")
                self.assertEqual(list(base.rglob("aqtinstall.log")), [])
            finally:
                os.chdir(saved[0])
                for key, value in zip(("APPDATA", "LOG_CFG"), saved[1:]):
                    if value is None:
                        os.environ.pop(key, None)
                    else:
                        os.environ[key] = value

    def test_real_extractor_child_lists_and_extracts_tiny_archive(self):
        python = ROOT / "tools/.venv/renderer-spike/Scripts/python.exe"
        if not python.is_file():
            self.skipTest("renderer-spike interpreter is absent in this worktree")
        with real_processes(self):
            env = bootstrap.probe_env()
            r = subprocess.run([str(python), "-I", "-c", "import py7zr"], env=env,
                               capture_output=True, text=True, timeout=bootstrap.PROBE_TIMEOUT)
            if r.returncode != 0:
                self.skipTest("renderer-spike interpreter cannot import py7zr")
            with tempfile.TemporaryDirectory() as td:
                base = Path(td)
                source, archive, target = base / "source", base / "tiny.7z", base / "out"
                source.write_bytes(b"tiny fixture")
                build = "import sys,py7zr; a=py7zr.SevenZipFile(sys.argv[1],'w'); a.write(sys.argv[2],'nested/file'); a.close()"
                kwargs = dict(cwd=base, env=env, capture_output=True, text=True, timeout=bootstrap.INSTALL_TIMEOUT)
                r = subprocess.run([str(python), "-I", "-c", build, str(archive), str(source)], **kwargs)
                self.assertEqual(r.returncode, 0, r.stderr)
                command = [str(python), "-I", "-c", bootstrap.EXTRACT_CODE]
                r = subprocess.run([*command, "list", str(archive)], **kwargs)
                self.assertEqual(r.returncode, 0, r.stderr)
                listing = json.loads(r.stdout)
                self.assertEqual(bootstrap.member_problems(listing, "6.10.3/msvc2022_64"), [])
                self.assertEqual([m["name"] for m in listing if m["file"]], ["nested/file"])
                target.mkdir()
                r = subprocess.run([*command, "extract", str(archive), str(target)], **kwargs)
                self.assertEqual(r.returncode, 0, r.stderr)
                self.assertEqual((target / "nested/file").read_bytes(), source.read_bytes())


H = "--hash=sha256:" + "a" * 64
WHEEL = "https://files.pythonhosted.org/packages/ab/cd/numpy-2.5.3-cp314-cp314-win_amd64.whl"


class SourceAndDownloadTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        self.lock = bootstrap.load_lock()
        self.saved = (bootstrap._require_file, bootstrap.safe_dest, bootstrap._download_opener, bootstrap._run,
                      bootstrap.ledger, bootstrap.venv_contained)
        self.req = self.root / "req.txt"
        bootstrap._require_file = lambda rel: self.req
        bootstrap.safe_dest = lambda rel: self.root / rel
        bootstrap.ledger = lambda record: None

    def tearDown(self):
        (bootstrap._require_file, bootstrap.safe_dest, bootstrap._download_opener, bootstrap._run,
         bootstrap.ledger, bootstrap.venv_contained) = self.saved
        self.tmp.cleanup()

    def test_requirement_files_cannot_choose_their_own_sources(self):
        entry = bootstrap.entry_for(self.lock, "scikit-learn")
        self.req.write_text(f"numpy==2.5.3 \\\n    {H}\n    # via scikit-learn\nscipy==1.18.1 \\\n    {H}\n", encoding="utf-8")
        bootstrap.check_requirement_sources(entry)
        self.assertEqual(bootstrap.requirement_pins(self.req), {"numpy": "2.5.3", "scipy": "1.18.1"})
        bad = {"index": "--index-url https://mirror.invalid/simple", "short index": "-i https://mirror.invalid/simple",
               "inline option": f"numpy==2.5.3 {H} --extra-index-url https://mirror.invalid/simple",
               "find links": "--find-links https://mirror.invalid/", "include": "-r other.txt", "constraint": "-c other.txt",
               "editable": "-e git+https://github.com/x/y", "vcs": f"y @ git+https://github.com/x/y {H}",
               "wheel URL on an approved host": f"numpy @ {WHEEL} {H}",
               "foreign host": f"numpy @ https://mirror.invalid/numpy-2.5.3-cp314-cp314-win_amd64.whl {H}",
               "source archive": f"numpy @ https://files.pythonhosted.org/packages/ab/cd/numpy-2.5.3.tar.gz {H}",
               "local file": f"numpy @ file:///C:/x/numpy-2.5.3-cp314-cp314-win_amd64.whl {H}",
               "no hash": "numpy==2.5.3", "extras": f"numpy[all]==2.5.3 {H}"}
        for name, line in bad.items():
            self.req.write_text(line + "\n", encoding="utf-8")
            with self.subTest(case=name), self.assertRaises(bootstrap.Refused):
                bootstrap.check_requirement_sources(entry)
        runs = []
        bootstrap._run = lambda cmd, cwd=ROOT, env=None: runs.append(cmd)
        self.req.write_text(bad["foreign host"] + "\n", encoding="utf-8")
        for tool in ("scikit-learn", "uv"):  # refused before any command runs, for both Python methods
            with self.subTest(tool=tool), self.assertRaises(bootstrap.Refused):
                bootstrap.install_entry(bootstrap.entry_for(self.lock, tool))
        self.assertEqual(runs, [])

    def model(self, **changes) -> dict:
        return dict(bootstrap.entry_for(self.lock, "clip-vit-b-16"), **changes)

    def test_invalid_pins_and_hashes_are_refused_before_installation(self):
        bad = {"wildcard": f"numpy==2.* {H}", "environment variable": f"numpy==${{TASTELAB_REVIEW_PIN}} {H}",
               "empty hash": "numpy==2.5.3 --hash=sha256:", "uppercase hash": "numpy==2.5.3 " + H.upper(),
               "uppercase hex": "numpy==2.5.3 --hash=sha256:" + "A" * 64,
               "short hash": "numpy==2.5.3 --hash=sha256:" + "a" * 63,
               "long hash": "numpy==2.5.3 --hash=sha256:" + "a" * 65,
               "mixed hashes": f"numpy==2.5.3 {H} --hash=sha256:",
               "URL": f"numpy @ {WHEEL} {H}", "index option": "--index-url https://mirror.invalid/simple",
               "two pins": f"numpy==2.5.3 scipy==1.18.1 {H}", "trailing text": f"numpy==2.5.3 junk {H}"}
        for name, line in bad.items():
            self.req.write_text(line + "\n", encoding="utf-8")
            for tool in ("scikit-learn", "uv"):
                with self.subTest(case=name, tool=tool), contextlib.ExitStack() as guards:
                    sentinels = [guards.enter_context(mock.patch.object(obj, attr, side_effect=AssertionError("installation work")))
                                 for obj, attr in ((bootstrap, "_run"), (bootstrap.subprocess, "run"),
                                                   (bootstrap, "install_env"), (Path, "mkdir"),
                                                   (Path, "write_text"), (Path, "write_bytes"))]
                    with self.assertRaises(bootstrap.Refused):
                        bootstrap.install_entry(bootstrap.entry_for(self.lock, tool))
                    for sentinel in sentinels:
                        sentinel.assert_not_called()
            self.assertEqual(list(self.root.iterdir()), [self.req])

    def test_literal_release_versions_and_markers_are_allowed(self):
        entry = bootstrap.entry_for(self.lock, "scikit-learn")
        for version in ("2", "2.5.3", "2.5.3a1", "2.5.3b2", "2.5.3rc1", "2.5.3.post1", "2.5.3.dev1",
                        "2.5.3rc1.post2.dev3"):
            with self.subTest(version=version):
                self.req.write_text(f"numpy=={version}; python_version >= '3.14' {H}\n", encoding="utf-8")
                bootstrap.check_requirement_sources(entry)
                self.assertEqual(bootstrap.requirement_pins(self.req), {"numpy": version})

    def connection(self, data: bytes, redirect: str | None = None):
        from email.message import Message

        class Response(io.BytesIO):
            def __init__(self, body, location=None):
                super().__init__(body)
                self.code = self.status = 302 if location else 200
                self.reason = "Found" if location else "OK"
                self.headers = Message()
                if location:
                    self.headers["Location"] = location

            def geturl(self):
                return self.url

            def info(self):
                return self.headers

        connection = mock.Mock()
        connection.sock = None
        connection.set_tunnel.side_effect = AssertionError("a proxy tunnel was attempted")
        connection.getresponse.side_effect = ([Response(b"", redirect)] if redirect else []) + [Response(data)]
        return connection

    def test_download_connects_directly_without_ambient_credentials(self):
        import hashlib
        import urllib.request
        data = b"weights"
        entry = self.model(size=len(data), sha256=hashlib.sha256(data).hexdigest())
        proxies = {name: "http://user:password@proxy.invalid:8080"
                   for name in ("HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY", "http_proxy", "https_proxy", "all_proxy")}
        proxies.update({"NO_PROXY": "", "no_proxy": ""})
        bootstrap._download_opener = self.saved[2]
        connection = self.connection(data, "https://us.aws.cdn.hf.co/weights")
        with mock.patch.dict(os.environ, proxies), \
                mock.patch("urllib.request.getproxies", return_value={"http": proxies["HTTP_PROXY"], "https": proxies["HTTPS_PROXY"],
                                                                     "all": proxies["ALL_PROXY"]}) as getproxies, \
                mock.patch("netrc.netrc", side_effect=AssertionError("ambient credentials were read")) as netrc, \
                mock.patch.object(bootstrap.http.client, "HTTPSConnection", return_value=connection) as connect, \
                contextlib.redirect_stdout(io.StringIO()):
            opener = bootstrap._download_opener(entry["network_hosts"])
            self.assertTrue(all(not h.proxies for h in opener.handlers if isinstance(h, urllib.request.ProxyHandler)))
            self.assertFalse(any(isinstance(h, (urllib.request.AbstractBasicAuthHandler, urllib.request.AbstractDigestAuthHandler,
                                               urllib.request.HTTPCookieProcessor)) for h in opener.handlers))
            bootstrap.download_hashed(entry)
            getproxies.assert_not_called()
            netrc.assert_not_called()
        self.assertEqual([call.args[0] for call in connect.call_args_list], ["huggingface.co", "us.aws.cdn.hf.co"])
        connection.set_tunnel.assert_not_called()
        for call, host in zip(connection.request.call_args_list, ("huggingface.co", "us.aws.cdn.hf.co")):
            self.assertEqual(call.args[3], {"Host": host, "User-Agent": "magic600-bootstrap", "Connection": "close"})
        self.assertEqual((self.root / entry["dest"]).read_bytes(), data)

    def test_unapproved_redirect_is_refused_before_connecting(self):
        entry = self.model()
        bootstrap._download_opener = self.saved[2]
        connection = self.connection(b"", "https://mirror.invalid/weights")
        with mock.patch.object(bootstrap.http.client, "HTTPSConnection", return_value=connection) as connect, \
                contextlib.redirect_stdout(io.StringIO()), self.assertRaises(bootstrap.Refused) as ctx:
            bootstrap.download_hashed(entry)
        self.assertIn("not among the entry's approved network hosts", str(ctx.exception))
        self.assertEqual([call.args[0] for call in connect.call_args_list], ["huggingface.co"])
        dest = self.root / entry["dest"]
        self.assertFalse(dest.exists())
        self.assertEqual(list(dest.parent.glob("*.part")), [])

    def serve(self, data: bytes, final_url: str | None = None) -> list:
        opened = []

        class Response(io.BytesIO):
            def geturl(self):
                return final_url or opened[-1]

        class Opener:
            def open(self, request, timeout=None):
                opened.append(request.full_url)
                self.headers = dict(request.header_items())
                return Response(data)

        bootstrap._download_opener = lambda hosts: Opener()
        return opened

    def test_download_keeps_only_the_pinned_file(self):
        import hashlib
        data = b"weights" * 1000
        entry = self.model(size=len(data), sha256=hashlib.sha256(data).hexdigest())
        dest = self.root / entry["dest"]
        cases = {"other content": (b"WEIGHTS" * 1000, None), "too large": (data + b"x", None),
                 "truncated": (data[:-1], None), "unapproved final host": (data, "https://mirror.invalid/weights")}
        for name, (served, final) in cases.items():
            self.serve(served, final)
            with self.subTest(case=name):
                with self.assertRaises(bootstrap.Refused), contextlib.redirect_stdout(io.StringIO()):
                    bootstrap.download_hashed(entry)
                self.assertFalse(dest.exists())
                self.assertEqual(list(dest.parent.glob("*.part")), [])
        opened = self.serve(data)
        with contextlib.redirect_stdout(io.StringIO()):
            bootstrap.download_hashed(entry)
        self.assertEqual(opened, [entry["url"]])
        self.assertEqual(dest.read_bytes(), data)
        self.assertEqual(list(dest.parent.glob("*.part")), [])
        self.assertEqual(bootstrap.verify_download(entry)[0], True)
        self.assertTrue(bootstrap.present(entry))

    def test_download_never_writes_through_a_planted_staging_file(self):
        import hashlib
        data = b"weights" * 1000
        entry = self.model(size=len(data), sha256=hashlib.sha256(data).hexdigest())
        dest = self.root / entry["dest"]
        dest.parent.mkdir(parents=True)
        outside = self.root / "outside.bin"
        outside.write_bytes(b"outside")
        fixed = bytes(8)
        planted = [dest.with_name(dest.name + ".part"), dest.with_name(f"{dest.name}.{fixed.hex()}.part")]
        try:
            for path in planted:
                os.link(outside, path)  # hard links: the link checks cannot see them
        except (OSError, NotImplementedError) as exc:
            self.skipTest(f"hard links not available ({exc})")
        self.serve(data)
        with mock.patch.object(os, "urandom", return_value=fixed), contextlib.redirect_stdout(io.StringIO()), \
                self.assertRaises(bootstrap.Refused):
            bootstrap.download_hashed(entry)  # the staging name is taken: refused, never written through
        self.assertEqual(outside.read_bytes(), b"outside")
        self.assertFalse(dest.exists())
        self.assertTrue(all(path.exists() for path in planted), "files this call did not create are left alone")
        self.serve(data)
        with contextlib.redirect_stdout(io.StringIO()):
            bootstrap.download_hashed(entry)
        self.assertEqual(outside.read_bytes(), b"outside")
        self.assertEqual(dest.read_bytes(), data)

    def test_concurrent_download_cannot_replace_checked_bytes(self):
        import hashlib
        data = b"weights" * 1000
        entry = self.model(size=len(data), sha256=hashlib.sha256(data).hexdigest())
        dest = self.root / entry["dest"]
        replace, others = os.replace, []

        def another_download_first(src, dst):
            # This call has checked its bytes; a second download of the same entry runs and fails before the rename.
            if not others:
                others.append(entry["id"])
                self.serve(data[:100])
                with self.assertRaises(bootstrap.Refused):
                    bootstrap.download_hashed(entry)
            return replace(src, dst)

        self.serve(data)
        with mock.patch.object(os, "replace", side_effect=another_download_first), contextlib.redirect_stdout(io.StringIO()):
            bootstrap.download_hashed(entry)
        self.assertEqual(others, [entry["id"]])
        self.assertEqual(dest.read_bytes(), data)
        self.assertEqual([p.name for p in dest.parent.iterdir()], [dest.name], "each call removes only its own staging file")

    def test_present_model_with_other_content_is_not_replaced(self):
        entry = self.model()
        dest = self.root / entry["dest"]
        dest.parent.mkdir(parents=True)
        dest.write_bytes(b"something else")
        lock = dict(self.lock, tools=[entry if e["id"] == entry["id"] else e for e in self.lock["tools"]])
        opened = self.serve(b"")
        with self.assertRaises(bootstrap.Refused) as ctx:
            bootstrap.install(lock, entry["id"], set())
        self.assertIn("owner approval", str(ctx.exception))
        self.assertEqual(opened, [])
        self.assertEqual(dest.read_bytes(), b"something else")

    def test_redirects_stay_on_approved_hosts(self):
        handler = bootstrap._ApprovedRedirects(["huggingface.co", "us.aws.cdn.hf.co"])
        request = __import__("urllib.request").request.Request("https://huggingface.co/m/resolve/x/w.safetensors")
        follow = handler.redirect_request(request, None, 302, "Found", {}, "https://us.aws.cdn.hf.co/xet-bridge-us/abc?x=1")
        self.assertEqual(follow.full_url, "https://us.aws.cdn.hf.co/xet-bridge-us/abc?x=1")
        for target in ("https://mirror.invalid/w", "http://us.aws.cdn.hf.co/w", "https://us.aws.cdn.hf.co:8443/w",
                       "https://user@us.aws.cdn.hf.co/w", "file:///C:/w.safetensors"):
            with self.subTest(target=target), self.assertRaises(bootstrap.Refused):
                handler.redirect_request(request, None, 302, "Found", {}, target)

    def test_download_pins_are_validated_in_the_lockfile(self):
        good = self.model()
        self.assertTrue(bootstrap.valid_download(good))
        bad = {"plain http": dict(good, url=good["url"].replace("https:", "http:")),
               "unlisted host": dict(good, network_hosts=["us.aws.cdn.hf.co"]),
               "short hash": dict(good, sha256="abc"), "upper-case hash": dict(good, sha256=good["sha256"].upper()),
               "boolean size": dict(good, size=True), "zero size": dict(good, size=0),
               "outside models": dict(good, dest="tools/.venv/x.safetensors"), "parent step": dict(good, dest="tools/.models/../x"),
               "no subdirectory": dict(good, dest="tools/.models/x.safetensors"), "partial name": dict(good, dest="tools/.models/m/x.part")}
        for name, entry in bad.items():
            with self.subTest(case=name):
                self.assertFalse(bootstrap.valid_download(entry))
        with tempfile.TemporaryDirectory() as td:
            path = Path(td) / "lock.json"
            lock = json.loads(bootstrap.LOCK_PATH.read_text(encoding="utf-8"))
            bootstrap.entry_for(lock, "clip-vit-b-16")["url"] = bad["plain http"]["url"]
            path.write_text(json.dumps(lock), encoding="utf-8")
            saved = bootstrap.LOCK_PATH
            bootstrap.LOCK_PATH = path
            try:
                with self.assertRaises(SystemExit):
                    bootstrap.load_lock()
            finally:
                bootstrap.LOCK_PATH = saved


class ApprovalTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.saved = (bootstrap.APPROVAL_PATH, install_guard.APPROVAL_PATH)
        bootstrap.APPROVAL_PATH = install_guard.APPROVAL_PATH = Path(self.tmp.name) / "toolchain.json"

    def tearDown(self):
        bootstrap.APPROVAL_PATH, install_guard.APPROVAL_PATH = self.saved
        self.tmp.cleanup()

    def test_states(self):
        self.assertEqual(bootstrap.approval_state(), "not-approved")
        bootstrap.APPROVAL_PATH.write_text(json.dumps({"digest": bootstrap.approval_digest()}), encoding="utf-8")
        self.assertEqual(bootstrap.approval_state(), "approved")
        bootstrap.APPROVAL_PATH.write_text(json.dumps({"digest": "0" * 64}), encoding="utf-8")
        self.assertEqual(bootstrap.approval_state(), "stale")

    def test_approval_covers_installer_and_every_lockfile(self):
        inputs = bootstrap.approval_inputs()
        for rel in ("tools/toolchain/bootstrap.py", "tools/toolchain.lock.json", "tools/skills.lock.json", "tools/skills/sync.py",
                    "tools/repo_digest.py", "tools/toolchain/approval_inputs.json", ".claude/hooks/install_guard.py",
                    "tools/python/planning.txt", "tools/python/engine.txt", "tools/node/mermaid-cli/package-lock.json"):
            self.assertIn(rel, inputs)

    def test_guard_and_installer_compute_the_same_digest(self):
        self.assertEqual(install_guard.approval_inputs(), bootstrap.approval_inputs())
        self.assertEqual(install_guard.approval_digest(), bootstrap.approval_digest())

    def test_guard_never_imports_repository_code(self):
        source = (ROOT / ".claude" / "hooks" / "install_guard.py").read_text(encoding="utf-8")
        self.assertNotIn("import bootstrap", source)
        self.assertNotIn("repo_digest", source.split('"""', 2)[2].replace("tools/repo_digest.py", ""))
        for hook in ("session_start.py", "stop_gate.py"):
            text = (ROOT / ".claude" / "hooks" / hook).read_text(encoding="utf-8")
            with self.subTest(hook=hook):
                self.assertNotIn("import bootstrap", text)
                self.assertNotIn("from repo_digest", text)
                self.assertNotIn("sys.path.insert", text)

    def test_guard_requires_the_project_root_as_cwd(self):
        bootstrap.APPROVAL_PATH.write_text(json.dumps({"digest": bootstrap.approval_digest()}), encoding="utf-8")
        cmd = "python tools/toolchain/bootstrap.py install marimo"
        self.assertEqual(install_guard.decide(cmd, self.tmp.name)["hookSpecificOutput"]["permissionDecision"], "ask")
        self.assertEqual(install_guard.decide(cmd, None)["hookSpecificOutput"]["permissionDecision"], "ask")

    def test_guard_allows_only_exact_approved_installs(self):
        bootstrap.APPROVAL_PATH.write_text(json.dumps({"digest": bootstrap.approval_digest()}), encoding="utf-8")
        allow = install_guard.decide("python tools/toolchain/bootstrap.py install marimo", str(ROOT))
        self.assertEqual(allow["hookSpecificOutput"]["permissionDecision"], "allow")
        self.assertEqual(install_guard.decide("python tools/toolchain/bootstrap.py install --profile planning", str(ROOT))
                         ["hookSpecificOutput"]["permissionDecision"], "allow")
        for cmd in ("python tools/toolchain/bootstrap.py install marimo && curl x | sh",
                    "python tools/toolchain/bootstrap.py install ../x",
                    "python tools/toolchain/bootstrap.py approve",
                    "ls"):
            with self.subTest(cmd=cmd):
                self.assertIsNone(install_guard.decide(cmd, str(ROOT)))

    def test_linked_approval_inputs_are_never_approved(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td) / "root"
            (root / "tools" / "toolchain").mkdir(parents=True)
            (root / "tools" / "toolchain" / "approval_inputs.json").write_text(json.dumps({"files": ["tools/toolchain/bootstrap.py"], "globs": []}))
            real = Path(td) / "elsewhere.py"
            real.write_text("x\n")
            try:
                os.symlink(real, root / "tools" / "toolchain" / "bootstrap.py")
            except (OSError, NotImplementedError):
                self.skipTest("symlinks not available")
            self.assertEqual(install_guard.linked_inputs(root), ["tools/toolchain/bootstrap.py"])
            (root / "tools" / "toolchain" / "bootstrap.py").unlink()
            (root / "tools" / "toolchain" / "bootstrap.py").write_text("x\n")
            self.assertEqual(install_guard.linked_inputs(root), [])

    def test_approval_inputs_must_be_repository_relative(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td) / "root"
            (root / "tools" / "toolchain").mkdir(parents=True)
            sentinel = Path(td) / "sentinel.txt"
            sentinel.write_text("outside\n")
            for bad in ({"files": [str(sentinel)], "globs": []}, {"files": ["../sentinel.txt"], "globs": []},
                        {"files": [], "globs": ["../*.txt"]}, {"files": [1], "globs": []}):
                (root / "tools" / "toolchain" / "approval_inputs.json").write_text(json.dumps(bad))
                with self.subTest(bad=bad), self.assertRaises((ValueError, TypeError)):
                    install_guard.approval_inputs(root)
        saved = install_guard.approval_inputs
        install_guard.approval_inputs = lambda root=None: (_ for _ in ()).throw(ValueError("bad"))
        try:
            bootstrap.APPROVAL_PATH.write_text(json.dumps({"digest": bootstrap.approval_digest()}), encoding="utf-8")
            self.assertEqual(install_guard.approval_state(), "not-approved")
        finally:
            install_guard.approval_inputs = saved

    def test_linked_installer_refuses_before_importing_helpers(self):
        with tempfile.TemporaryDirectory() as td:
            (Path(td) / "tools" / "toolchain").mkdir(parents=True)
            (Path(td) / "tools" / "repo_digest.py").write_text("raise SystemExit('helper executed')\n")
            link = Path(td) / "tools" / "toolchain" / "bootstrap.py"
            try:
                os.symlink(ROOT / "tools" / "toolchain" / "bootstrap.py", link)
            except (OSError, NotImplementedError):
                self.skipTest("symlinks not available")
            source = (ROOT / "tools" / "toolchain" / "bootstrap.py").read_bytes()
            with self.assertRaises(SystemExit) as ctx:
                exec(compile(source, str(link), "exec"), {"__file__": str(link), "__name__": "__main__"})
            self.assertIn("is a link", str(ctx.exception))
            self.assertNotIn("helper executed", str(ctx.exception))

    def test_linked_installer_refuses_in_a_real_process(self):
        with tempfile.TemporaryDirectory() as td:
            (Path(td) / "tools" / "toolchain").mkdir(parents=True)
            (Path(td) / "tools" / "repo_digest.py").write_text("raise SystemExit('helper executed')\n")
            link = Path(td) / "tools" / "toolchain" / "bootstrap.py"
            try:
                REAL["symlink"](ROOT / "tools" / "toolchain" / "bootstrap.py", link)
            except (OSError, NotImplementedError):
                self.skipTest("symlinks not available")
            with real_processes(self):
                r = subprocess.run([sys.executable, str(link), "check"], capture_output=True, text=True, timeout=20)
            self.assertNotEqual(r.returncode, 0)
            self.assertIn("is a link", r.stderr)
            self.assertNotIn("helper executed", r.stderr)

    def test_guard_asks_when_revision_is_not_approved(self):
        bootstrap.APPROVAL_PATH.write_text(json.dumps({"digest": "0" * 64}), encoding="utf-8")
        ask = install_guard.decide("python tools/toolchain/bootstrap.py install-skill marimo-pair", str(ROOT))
        self.assertEqual(ask["hookSpecificOutput"]["permissionDecision"], "ask")


class CheckTests(unittest.TestCase):
    def test_check_is_fast_and_succeeds(self):
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            code = bootstrap.main(["check"])
        self.assertEqual(code, 0)
        self.assertIn("toolchain planning:", out.getvalue())

    def test_check_runs_fast_in_a_real_process(self):
        with real_processes(self):
            r = subprocess.run([sys.executable, str(ROOT / "tools" / "toolchain" / "bootstrap.py"), "check"],
                               capture_output=True, text=True, timeout=10)
        self.assertEqual(r.returncode, 0)
        self.assertIn("toolchain planning:", r.stdout)

    def test_skill_install_of_present_pinned_skill_is_idempotent(self):
        lock = json.loads(bootstrap.SKILLS_LOCK_PATH.read_text(encoding="utf-8"))
        third = [k for k, v in lock["skills"].items() if v["origin"] == "third-party"]
        for name in third:
            from repo_digest import tree_digest
            self.assertEqual(tree_digest(ROOT / ".agents" / "skills" / name), lock["skills"][name]["digest"])


if __name__ == "__main__":
    unittest.main(verbosity=2)
