"""Tests for the toolchain installer and the install guard hook. No network, no real installs."""
from __future__ import annotations

import contextlib
import io
import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
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
    REAL.update(run=subprocess.run, Popen=subprocess.Popen, symlink=os.symlink)
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
            installs = []
            bootstrap.run_probe = lambda e: (e["id"] == "uv" or e["id"] in installs, "missing")  # uv (a dependency) works
            bootstrap.present = lambda e: False
            bootstrap.install_entry = lambda e: installs.append(e["id"])
            bootstrap.safe_dest = lambda rel: Path(td)
            bootstrap.ledger = lambda *a, **k: None
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
