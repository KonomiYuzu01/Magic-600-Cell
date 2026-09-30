"""Tests for the toolchain installer and the install guard hook. No network, no real installs."""
from __future__ import annotations

import contextlib
import io
import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))
sys.path.insert(0, str(ROOT / "tools" / "toolchain"))
sys.path.insert(0, str(ROOT / ".claude" / "hooks"))
import bootstrap  # noqa: E402
import install_guard  # noqa: E402


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
                    self.assertIn(f"=={entry['version']}", text)
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
        with tempfile.TemporaryDirectory() as outside:
            link = ROOT / "tools" / ".venv" / "link-test"
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
            saved = (bootstrap.run_probe, bootstrap.present, bootstrap.install_entry, bootstrap.safe_dest)
            installs = []
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
                bootstrap.run_probe, bootstrap.present, bootstrap.install_entry, bootstrap.safe_dest = saved
        self.assertEqual(installs, ["py-spy"])

    def test_linked_venv_descendants_are_refused(self):
        entry = bootstrap.entry_for(bootstrap.load_lock(), "marimo")
        with tempfile.TemporaryDirectory() as td, tempfile.TemporaryDirectory() as outside:
            venv = Path(td)
            (venv / "bin").mkdir()
            (venv / "lib" / "python3.11").mkdir(parents=True)
            os.symlink("lib", venv / "lib64", target_is_directory=True)  # legitimate internal link
            try:
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
                os.symlink(Path(outside) / "x", venv / "bin" / "marimo")
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
                (venv / "bin" / "marimo").unlink()
                os.symlink(sys.executable, venv / "bin" / "python")  # interpreter link is allowed
                self.assertEqual(bootstrap.venv_conflicts(entry), [])
                (venv / "bin" / "python").unlink()
                os.symlink(Path(outside) / "evil-python", venv / "bin" / "python")  # any other interpreter is not
                with self.assertRaises(bootstrap.Refused):
                    bootstrap.venv_conflicts(entry)
                (venv / "bin" / "python").unlink()
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
            for listing, ok in (("Name     Id          Version\ndraw.io  JGraph.Draw 31.5.3\n", True),
                                ("Name     Id          Version\ndraw.io  JGraph.Draw 31.5.30\n", False),
                                ("No installed package found matching input criteria.\n", False)):
                bootstrap.subprocess.run = lambda *a, **k: subprocess.CompletedProcess(a, 0, listing, "")
                with self.subTest(listing=listing):
                    self.assertEqual(bootstrap.run_probe(entry)[0], ok)
        finally:
            bootstrap.PLATFORM, bootstrap.resolve_executable, bootstrap.shutil.which, bootstrap.subprocess.run = saved

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
                                     ("uv", "uv 0.12.20.post1", False), ("uv", "uv 0.12.20+local", False)):
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
                bootstrap.refuse_git_rewrites("https://github.com/owner/repo", env)  # the ambient rewrite is gone
                hooks = subprocess.run(bootstrap.git_cmd(empty, "config", "--get", "core.hooksPath"), env=env, cwd=td,
                                       capture_output=True, text=True).stdout.strip()
                self.assertEqual(hooks, str(empty))
                with self.assertRaises(bootstrap.Refused):
                    bootstrap.refuse_git_rewrites("https://github.com/owner/repo", dict(env, **{
                        "GIT_CONFIG_COUNT": "1", "GIT_CONFIG_KEY_0": "url.https://mirror.invalid/.insteadOf",
                        "GIT_CONFIG_VALUE_0": "https://github.com/"}))
        finally:
            for k, v in saved.items():
                if v is None:
                    os.environ.pop(k, None)
                else:
                    os.environ[k] = v

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
        with tempfile.TemporaryDirectory() as outside, tempfile.TemporaryDirectory() as td:
            lock_path = Path(td) / "skills.lock.json"
            lock_path.write_text(json.dumps({"skills": {"zz-link-test": {
                "origin": "third-party", "repo": "https://github.com/owner/repo", "commit": "0" * 40,
                "subdir": "x", "digest": "0" * 64}}}), encoding="utf-8")
            link = ROOT / ".claude" / "skills" / "zz-link-test"
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
                    "tools/python/planning.txt", "tools/node/mermaid-cli/package-lock.json"):
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
