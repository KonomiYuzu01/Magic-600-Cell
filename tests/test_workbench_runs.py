"""Registry and real detached-runner regressions, using only disposable fixtures."""
from __future__ import annotations

import copy
import ctypes
import hashlib
import json
import os
import re
import shutil
import stat
import subprocess
import sys
import tempfile
import textwrap
import time
import unittest
import uuid
from datetime import datetime, timezone
from pathlib import Path
from unittest import mock

sys.dont_write_bytecode = True
os.environ["PYTHONDONTWRITEBYTECODE"] = "1"
ROOT = Path(__file__).resolve().parents[1]
WB = ROOT / "tools" / "workbench"
sys.path.insert(0, str(WB))
import launch
import paths
import runner
import runs

RUNNER = WB / "runner.py"
REAL_POPEN = subprocess.Popen
CALL_ID = "20260930T000000Z-0123abcd"
FINAL = {"passed", "failed", "timed_out", "cancelled", "refused"}
WINDOWS = os.name == "nt"


def iso_at(timestamp):
    return datetime.fromtimestamp(timestamp, timezone.utc).isoformat().replace("+00:00", "Z")


def wait_for(predicate, timeout=15, message="condition did not arrive"):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        result = predicate()
        if result:
            return result
        time.sleep(0.05)
    raise AssertionError(message)


def pid_alive(pid):
    from ctypes import wintypes as w
    kernel = ctypes.WinDLL("kernel32", use_last_error=True)
    kernel.OpenProcess.argtypes, kernel.OpenProcess.restype = [w.DWORD, w.BOOL, w.DWORD], w.HANDLE
    kernel.GetExitCodeProcess.argtypes, kernel.GetExitCodeProcess.restype = [w.HANDLE, ctypes.POINTER(w.DWORD)], w.BOOL
    kernel.CloseHandle.argtypes, kernel.CloseHandle.restype = [w.HANDLE], w.BOOL
    handle = kernel.OpenProcess(0x1000, False, pid)  # PROCESS_QUERY_LIMITED_INFORMATION
    if not handle:
        error = ctypes.get_last_error()
        if error == 87:  # ERROR_INVALID_PARAMETER: this PID no longer exists
            return False
        raise ctypes.WinError(error)
    try:
        code = w.DWORD()
        if not kernel.GetExitCodeProcess(handle, ctypes.byref(code)):
            raise ctypes.WinError(ctypes.get_last_error())
        return code.value == 259  # STILL_ACTIVE
    finally:
        kernel.CloseHandle(handle)


class Fixture:
    def __init__(self):
        # CPython 3.13+ makes Windows mode-0700 temp directories inaccessible to
        # the restricted token. Inherit the writable temp root's ACL instead.
        self.tmp = Path(tempfile.gettempdir()).resolve() / ("workbench runs " + uuid.uuid4().hex)
        self.tmp.mkdir()
        self.main = self.tmp / "repo main"
        (self.main / ".git").mkdir(parents=True)
        self.data = paths.data_root(self.main)
        self.registry = self.main / "tools" / "workbench" / "runs.json"
        self.entry = {"id": "checks", "title": "Fixture checks", "steps": [["python", "tests/one.py"]],
                      "cwd": ".", "inputs": ["tests/*.py"], "outputs": [], "timeout_s": 10, "windows_required": False}
        self.children, self.by_id, self.commands = [], {}, {}
        self.script("one.py", "print('one output')\n")
        self.script("two.py", "print('two output')\n")
        self.wrapper("raise SystemExit(2)\n")
        self.file("work/reviews/packets/check.md", "Synthetic packet\n")
        self.save()

    def file(self, rel, text, checkout=None):
        file = (checkout or self.main) / rel
        file.parent.mkdir(parents=True, exist_ok=True)
        file.write_text(text, encoding="utf-8")
        return file

    def script(self, name, text, checkout=None):
        return self.file("tests/" + name, textwrap.dedent(text), checkout)

    def wrapper(self, text):
        return self.file("tools/agents/codex_review.py", textwrap.dedent(text))

    def save(self, entries=None):
        self.registry.parent.mkdir(parents=True, exist_ok=True)
        self.registry.write_text(json.dumps({"schema": 1, "runs": entries if entries is not None else [self.entry]}), encoding="utf-8")

    def worktree(self):
        tree = self.tmp / "registered tree"
        meta = self.main / ".git" / "worktrees" / "wt1"
        meta.mkdir(parents=True)
        tree.mkdir()
        (meta / "commondir").write_text("../..\n", encoding="utf-8")
        (meta / "gitdir").write_text(str(tree / ".git") + "\n", encoding="utf-8")
        (tree / ".git").write_text(f"gitdir: {meta}\n", encoding="utf-8")
        self.script("one.py", "print('worktree output')\n", tree)
        return tree

    def start(self, *, checkout=None, codex=None, expect=None, env=None, reserve=False):
        launched, command = [], []

        def spawn(argv, **kwargs):
            command.extend(argv)
            if reserve:
                return mock.Mock()
            proc = REAL_POPEN(argv, **kwargs)
            self.children.append(proc)
            launched.append(proc)
            return proc

        with mock.patch.dict(os.environ, env or {}), mock.patch.object(runs.subprocess, "Popen", side_effect=spawn):
            rid = runs.start(self.main, checkout or self.main, run_id=None if codex else "checks", expect=expect, codex=codex)
        self.commands[rid] = command
        if launched:
            self.by_id[rid] = launched[0]
        return rid

    def launch_reserved(self, rid):
        proc = REAL_POPEN(self.commands[rid], cwd=self.main, stdin=subprocess.DEVNULL,
                          stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        self.children.append(proc)
        self.by_id[rid] = proc
        return proc

    def record_path(self, rid):
        return self.data / "runs" / (rid + ".json")

    def record(self, rid):
        try:
            return json.loads(self.record_path(rid).read_bytes())
        except (OSError, ValueError):
            return {}

    def log(self, rid):
        try:
            return (self.data / "runs" / (rid + ".log")).read_text(encoding="utf-8", errors="replace")
        except OSError:
            return ""

    def done(self, rid, timeout=20):
        record = wait_for(lambda: (r if (r := self.record(rid)).get("status") in FINAL else None), timeout,
                          f"run did not end: {self.record(rid)}; log: {self.log(rid)[-2000:]}")
        if rid in self.by_id:
            self.by_id[rid].wait(timeout=5)
        return record

    def kill(self, rid):
        proc = self.by_id[rid]
        proc.terminate()  # Popen uses TerminateProcess on Windows.
        proc.wait(timeout=5)

    def cleanup(self):
        for rid in self.by_id:
            if self.by_id[rid].poll() is None:
                runs.cancel(self.data, rid)
        for proc in self.children:
            try:
                proc.wait(timeout=3)
            except subprocess.TimeoutExpired:
                proc.kill()
                proc.wait(timeout=5)
        target = self.tmp.resolve()
        if target.parent != Path(tempfile.gettempdir()).resolve() or not target.name.startswith("workbench runs "):
            raise RuntimeError("unexpected fixture cleanup target")
        shutil.rmtree(target)


class RunTests(unittest.TestCase):
    def setUp(self):
        # Give the fixture a native interpreter, not a WindowsApps activation
        # alias, which Windows creates outside the parent's Job Objects.
        environment = mock.patch.dict(os.environ, {"PATH": str(Path(sys.executable).parent) + os.pathsep + os.environ.get("PATH", "")})
        environment.start()
        self.addCleanup(environment.stop)
        self.fx = Fixture()
        self.addCleanup(self.fx.cleanup)

    def codex(self, **overrides):
        return {"kind": "review", "packet": "check.md", "model": "gpt-6.1-sol", "effort": "max", "speed": "standard", **overrides}

    def tree(self):
        self.fx.script("grandchild.py", """
            import os, time
            from pathlib import Path
            Path('grandchild.pid').write_text(str(os.getpid()))
            time.sleep(60)
        """)
        self.fx.script("one.py", """
            import os, subprocess, sys, time
            from pathlib import Path
            Path('step.pid').write_text(str(os.getpid()))
            subprocess.Popen([sys.executable, str(Path(__file__).with_name('grandchild.py'))])
            print('tree ready')
            time.sleep(60)
        """)

    def tree_pid(self):
        file = self.fx.main / "grandchild.pid"
        return wait_for(lambda: int(file.read_text()) if file.is_file() and file.read_text().strip() else None)

    def assert_gone(self, pid):
        wait_for(lambda: not pid_alive(pid), 5, f"PID {pid} survived its job")

    # 4.8.1 and 4.8.20: checked-in schema, registry, and wrapper contract.
    def test_seed_schema_registry_and_wrapper_constants(self):
        sys.path.insert(0, str(ROOT / "tools" / "agents"))
        import codex_review
        schema = json.loads((ROOT / "schemas" / "workbench-runs.schema.json").read_text(encoding="utf-8"))
        doc = json.loads((WB / "runs.json").read_bytes())
        self.assertEqual(codex_review.validate_result(doc, schema), [])
        registry = runs.load_registry(ROOT)
        self.assertEqual(registry.problems, [])
        self.assertEqual(registry.entries, doc["runs"])
        self.assertEqual(set(schema["required"]), runs.REGISTRY_KEYS)
        self.assertEqual(set(schema["properties"]["runs"]["items"]["required"]), runs.ENTRY_KEYS)
        self.assertEqual(registry.sha256, hashlib.sha256((WB / "runs.json").read_bytes()).hexdigest())
        self.assertEqual(runs.CODEX_MODELS, codex_review.MODELS)
        self.assertEqual(runs.CODEX_EFFORTS, codex_review.EFFORTS)
        self.assertEqual(sorted(runs.CODEX_SPEEDS), sorted(codex_review.SPEEDS))
        self.assertEqual(sorted(runs.CODEX_KINDS), sorted(codex_review.KIND_FOCUS))
        self.assertEqual(runs.CODEX_ACCEPTANCE_TIMEOUT, codex_review.ACCEPTANCE_TIMEOUT)
        source = (ROOT / "tools" / "agents" / "codex_review.py").read_text(encoding="utf-8")
        default = re.search(r'parser\.add_argument\("--timeout",\s*type=int,\s*default=(\d+)', source)
        self.assertIsNotNone(default, "wrapper --timeout source line was not found")
        self.assertEqual(runs.CODEX_MODEL_TIMEOUT, int(default[1]))
        self.assertEqual([runs.codex_deadline(k) for k in runs.CODEX_KINDS], [5400, 5400, 7200])
        self.assertEqual([e["id"] for e in registry.entries], ["checks-mechanics", "checks-process", "checks-agent-rules",
                         "checks-workbench", "checks-wiki", "checks-skills", "doctor"])
        for entry in registry.entries:
            self.assertTrue(entry["inputs"])
            self.assertEqual(entry["outputs"], [])

    # 4.8.2: each rejected entry makes the entire registry unusable.
    def test_registry_rejects_keys_types_bounds_and_ids(self):
        invalid = []
        for key in runs.ENTRY_KEYS:
            entry = copy.deepcopy(self.fx.entry)
            del entry[key]
            invalid.append(("missing " + key, [entry]))
            entry = copy.deepcopy(self.fx.entry)
            entry[key] = None
            invalid.append(("wrong type " + key, [entry]))
        for values in ({"surprise": 1}, {"timeout_s": True}, {"timeout_s": 9}, {"timeout_s": 14401},
                       {"windows_required": 1}, {"id": "codex-review"}, {"id": "Bad"}, {"id": "a" * 49},
                       {"title": ""}, {"title": "t" * 121}, {"steps": []},
                       {"steps": [["python", "tests/one.py"]] * 11}, {"steps": [[]]},
                       {"steps": [["python", "tests/one.py", "x" * 513]]},
                       {"steps": [["python", "tests/one.py", *["x"] * 31]]},
                       {"steps": [["python", "tests/one.py", 1]]}, {"steps": [["python", "tests/one.py", "\0"]]},
                       {"inputs": ["tests/*.py"] * 51}, {"outputs": ["out/*.txt"] * 51}):
            invalid.append((repr(values), [{**copy.deepcopy(self.fx.entry), **values}]))
        invalid.append(("duplicate id", [self.fx.entry, copy.deepcopy(self.fx.entry)]))
        invalid.append(("not an object", [1]))
        for label, entries in invalid:
            with self.subTest(label=label):
                self.fx.save(entries)
                registry = runs.load_registry(self.fx.main)
                self.assertTrue(registry.problems)
                self.assertEqual(registry.entries, [])
                with self.assertRaises(runs.RunRefused):
                    runs.plan_run(self.fx.main, self.fx.main, "checks")

    def test_registry_rejects_paths_and_commands(self):
        invalid = []
        for value in ("../one.py", "tests\\..\\one.py", "/one.py", "\\one.py", "C:one.py", "C:/one.py",
                      ".git/one.py", ".git\\one.py", "./.git/one.py", "work/one.py", "work\\one.py", "./work/one.py",
                      "-c", "-m", "-I", "tests/one.txt"):
            invalid.append({"steps": [["python", value]]})
        for exe in ("node", "cmd", "powershell", "tools/node/x.cmd", "tools/.venv/x/Scripts/python.cmd", "../python"):
            invalid.append({"steps": [[exe, "tests/one.py"]]})
        for command in ("push", "commit"):
            invalid.append({"steps": [["git", command]]})
        for value in ("..", "a/../b", "/", "\\root", "X:relative", ".git", ".git/sub", "work", "work/sub", "./work", "", "a\0b"):
            invalid.append({"cwd": value})
        for field in ("inputs", "outputs"):
            for value in ("../*.py", "/absolute", "\\root", "X:thing", ".git", ".git/*", "", "a\0b"):
                invalid.append({field: [value]})
        for values in invalid:
            with self.subTest(values=values):
                bad = {**copy.deepcopy(self.fx.entry), **values}
                good = {**copy.deepcopy(self.fx.entry), "id": "good"}
                self.fx.save([bad, good])
                self.assertTrue(runs.load_registry(self.fx.main).problems)
                with self.assertRaises(runs.RunRefused):
                    runs.plan_run(self.fx.main, self.fx.main, "good")
                with mock.patch.object(runs.subprocess, "Popen") as spawn:
                    with self.assertRaises(runs.RunRefused):
                        runs.start(self.fx.main, self.fx.main, run_id="good")
                    spawn.assert_not_called()

    def test_registry_bad_json_size_and_root_are_problems(self):
        for doc in ([], {"schema": True, "runs": []}, {"schema": 2, "runs": []}, {"runs": []},
                    {"schema": 1, "runs": "bad"}, {"schema": 1, "runs": [], "extra": 1}):
            with self.subTest(doc=doc):
                self.fx.registry.write_text(json.dumps(doc), encoding="utf-8")
                self.assertTrue(runs.load_registry(self.fx.main).problems)
        for raw in (b'{"schema":', b"\xff", b"x" * (runs.JSON_LIMIT + 1)):
            self.fx.registry.write_bytes(raw)
            registry = runs.load_registry(self.fx.main)
            self.assertTrue(registry.problems)
            self.assertEqual(registry.sha256, hashlib.sha256(raw).hexdigest())
        self.fx.registry.unlink()
        registry = runs.load_registry(self.fx.main)
        self.assertTrue(registry.problems)
        self.assertIsNone(registry.sha256)

    # 4.8.3: planning refuses unavailable files and unregistered targets.
    def test_plan_run_refusals_and_registered_worktree(self):
        with self.assertRaises(runs.RunRefused):
            runs.plan_run(self.fx.main, self.fx.main, "unknown")
        outsider = self.fx.tmp / "outsider"
        outsider.mkdir()
        with self.assertRaises(runs.RunRefused):
            runs.plan_run(self.fx.main, outsider, "checks")
        self.fx.entry["steps"] = [["python", "tests/missing.py"]]
        self.fx.save()
        with self.assertRaises(runs.RunRefused):
            runs.plan_run(self.fx.main, self.fx.main, "checks")
        self.fx.entry["steps"] = [["python", "tests/one.py"]]
        self.fx.entry["cwd"] = "missing-directory"
        self.fx.save()
        with self.assertRaises(runs.RunRefused):
            runs.plan_run(self.fx.main, self.fx.main, "checks")
        self.fx.entry["cwd"] = "."
        self.fx.save()
        tree = self.fx.worktree()
        plan = runs.plan_run(self.fx.main, tree, "checks")
        self.assertEqual(plan["checkout"], str(tree.resolve()))
        self.assertEqual(plan["steps"][0][1], str(tree / "tests" / "one.py"))
        self.assertEqual(plan["display"][0], ["python", *plan["steps"][0][1:]])
        self.assertEqual(plan["digest"], hashlib.sha256(json.dumps(self.fx.entry, sort_keys=True, separators=(",", ":")).encode()).hexdigest())
        self.fx.entry["steps"] = [["tools/.venv/missing/" + ("Scripts/python.exe" if WINDOWS else "bin/python"), "tests/one.py"]]
        self.fx.save()
        with self.assertRaises(runs.RunRefused):
            runs.plan_run(self.fx.main, self.fx.main, "checks")
        self.fx.entry["steps"] = [["python", "tests/one.py"]]
        self.fx.save()
        empty = self.fx.tmp / "empty bin"
        empty.mkdir()
        with mock.patch.dict(os.environ, {"PATH": str(empty)}), self.assertRaises(runs.RunRefused):
            runs.plan_run(self.fx.main, self.fx.main, "checks")

    @unittest.skipIf(WINDOWS, "requires a non-Windows platform")
    def test_windows_required_refused_elsewhere(self):
        self.fx.entry["windows_required"] = True
        self.fx.save()
        with self.assertRaises(runs.RunRefused):
            runs.plan_run(self.fx.main, self.fx.main, "checks")

    @unittest.skipUnless(WINDOWS, "Windows executable suffix policy")
    def test_path_python_shims_are_refused(self):
        for name in ("python.cmd", "python.bat", "python.ps1", "python"):
            with self.subTest(name=name):
                shim = self.fx.file("shims/" + name, "unused")
                with mock.patch.dict(os.environ, {"PATH": str(shim.parent)}):
                    with self.assertRaises(runs.RunRefused):
                        runs.plan_run(self.fx.main, self.fx.main, "checks")
                    with self.assertRaises(runs.RunRefused):
                        runs.plan_codex(self.fx.main, self.fx.main, **self.codex())
        # Unlike shutil.which, the current directory is never searched, even through a relative PATH entry.
        planted = self.fx.file("planted/python.exe", "unused")
        cwd = os.getcwd()
        os.chdir(planted.parent)
        self.addCleanup(os.chdir, cwd)
        with mock.patch.dict(os.environ, {"PATH": os.pathsep.join([".", ""])}), self.assertRaises(runs.RunRefused):
            runs.plan_run(self.fx.main, self.fx.main, "checks")

    @unittest.skipUnless(WINDOWS, "Windows App Execution Alias job inheritance")
    def test_app_execution_aliases_are_refused_before_spawn(self):
        alias = self.fx.file("aliases/python.exe", "unused")
        tag = mock.Mock(st_reparse_tag=stat.IO_REPARSE_TAG_APPEXECLINK)
        with mock.patch.dict(os.environ, {"PATH": str(alias.parent)}), mock.patch.object(Path, "lstat", return_value=tag):
            with self.assertRaisesRegex(runs.RunRefused, "App Execution Alias"):
                runs.plan_run(self.fx.main, self.fx.main, "checks")
            with self.assertRaisesRegex(runs.RunRefused, "App Execution Alias"):
                runs.plan_codex(self.fx.main, self.fx.main, **self.codex())
            with mock.patch.object(runs.subprocess, "Popen") as spawn:
                with self.assertRaises(runs.RunRefused):
                    runs.start(self.fx.main, self.fx.main, run_id="checks")
                spawn.assert_not_called()

    @unittest.skipUnless(WINDOWS, "Windows App Execution Alias job inheritance")
    def test_app_execution_alias_is_skipped_for_a_later_native_python(self):
        # The Python install manager puts a WindowsApps alias ahead of its native python.exe on PATH.
        alias = self.fx.file("aliases/python.exe", "unused")
        native = (Path(sys.executable).parent / "python.exe").resolve()
        real_lstat = Path.lstat

        def lstat(path, *args, **kwargs):
            if path == alias:
                return mock.Mock(st_reparse_tag=stat.IO_REPARSE_TAG_APPEXECLINK)
            return real_lstat(path, *args, **kwargs)

        with mock.patch.dict(os.environ, {"PATH": os.pathsep.join([str(alias.parent), str(native.parent)])}), \
                mock.patch.object(Path, "lstat", lstat):
            plan = runs.plan_run(self.fx.main, self.fx.main, "checks")
            codex = runs.plan_codex(self.fx.main, self.fx.main, **self.codex())
        self.assertEqual(plan["steps"][0][0], str(native))
        self.assertEqual(codex["steps"][0][0], str(native))
        self.assertEqual(plan["display"][0][0], "python")

    @unittest.skipUnless(WINDOWS, "Windows App Execution Alias job inheritance")
    def test_step_path_puts_the_native_python_ahead_of_an_alias(self):
        # A descendant that starts bare `python` (cmd.exe, a Git hook, the wrapper's acceptance check) must not reach the alias.
        alias = self.fx.file("aliases/python.exe", "unused")
        native = (Path(sys.executable).parent / "python.exe").resolve()
        real_lstat = Path.lstat

        def lstat(path, *args, **kwargs):
            if path == alias:
                return mock.Mock(st_reparse_tag=stat.IO_REPARSE_TAG_APPEXECLINK)
            return real_lstat(path, *args, **kwargs)

        with mock.patch.dict(os.environ, {"PATH": os.pathsep.join([str(alias.parent), str(native.parent)])}), \
                mock.patch.object(Path, "lstat", lstat):
            entries = runs.step_path().split(os.pathsep)
        self.assertEqual(entries[0], str(native.parent))
        self.assertEqual(entries[-2:], [str(alias.parent), str(native.parent)])  # the rest of PATH is kept in order

    def test_steps_see_the_vetted_python_first_on_path(self):
        self.fx.script("one.py", "import os\nprint('first on PATH: ' + os.environ['PATH'].split(os.pathsep)[0])\n")
        other = self.fx.tmp / "other bin"
        other.mkdir()
        with mock.patch.dict(os.environ, {"PATH": os.pathsep.join([str(other), os.environ["PATH"]])}):
            rid = self.fx.start()
        self.assertEqual(self.fx.done(rid)["status"], "passed")
        self.assertIn("first on PATH: " + str(runs._on_path("python").parent), self.fx.log(rid))

    @unittest.skipUnless(WINDOWS, "Windows directory junction fixture")
    def test_packet_junctions_cannot_escape_the_checkout(self):
        tree = self.fx.worktree()
        self.fx.file("tools/agents/codex_review.py", "raise SystemExit(2)\n", tree)
        outside = self.fx.tmp / "outside"
        self.fx.file("work/reviews/packets/p.md", "outside packet", outside)
        for level in ("work", "work/reviews", "work/reviews/packets"):
            with self.subTest(level=level):
                link = tree / level
                link.parent.mkdir(parents=True, exist_ok=True)
                proc = subprocess.run(["cmd", "/c", "mklink", "/J", str(link), str(outside / level)], capture_output=True, timeout=5)
                if proc.returncode != 0:
                    self.skipTest("junction creation unavailable in this sandbox")
                try:
                    self.assertTrue((tree / "work/reviews/packets/p.md").is_file())  # reachable through the junction
                    with self.assertRaisesRegex(runs.RunRefused, "packet"):
                        runs.plan_codex(self.fx.main, tree, **self.codex(packet="p.md"))
                    with self.assertRaises(launch.LaunchRefused):
                        launch.plan_session(self.fx.main, tree, "p.md")
                    self.assertEqual(runs.packets(tree), [])
                finally:
                    link.rmdir()  # the link only, before the fixture's temporary directory goes
        self.fx.file("work/reviews/packets/q.md", "own packet", tree)
        self.assertEqual(runs.packets(tree), ["q.md"])
        self.assertEqual(runs.plan_codex(self.fx.main, tree, **self.codex(packet="q.md"))["cwd"], str(tree))

    # 4.8.4: fixed wrapper arguments and packet selection.
    def test_plan_codex_refusals_and_exact_argv(self):
        for values in ({"kind": "cleanup"}, {"model": "other"}, {"effort": "low"}, {"speed": "turbo"}):
            with self.subTest(values=values), self.assertRaises(runs.RunRefused):
                runs.plan_codex(self.fx.main, self.fx.main, **self.codex(**values))
        for packet in ("sub/check.md", "../check.md", "..", "bad%.md", "bad&.md", "bad name.md", "check.txt", "missing.md"):
            with self.subTest(packet=packet), self.assertRaises(runs.RunRefused):
                runs.plan_codex(self.fx.main, self.fx.main, **self.codex(packet=packet))
        plan = runs.plan_codex(self.fx.main, self.fx.main, **self.codex())
        python = str(runs._on_path("python"))
        self.assertEqual(plan["steps"], [[python, "tools/agents/codex_review.py", "--kind", "review", "--packet",
                         "work/reviews/packets/check.md", "--model", "gpt-6.1-sol", "--effort", "max", "--speed", "standard"]])
        self.assertEqual(plan["display"], [["python", *plan["steps"][0][1:]]])
        self.assertEqual(plan["cwd"], str(self.fx.main))
        self.assertIsNone(plan["timeout_s"])
        self.assertIsNone(plan["digest"])
        self.assertNotIn("--timeout", plan["steps"][0])
        self.assertNotIn("--gate", plan["steps"][0])
        self.fx.file("work/reviews/packets/new.md", "new")
        os.utime(self.fx.main / "work/reviews/packets/check.md", (1, 1))
        self.fx.file("work/reviews/packets/bad name.md", "ignored")
        self.assertEqual(runs.packets(self.fx.main), ["new.md", "check.md"])
        (self.fx.main / "tools/agents/codex_review.py").unlink()
        with self.assertRaises(runs.RunRefused):
            runs.plan_codex(self.fx.main, self.fx.main, **self.codex())

    # 4.8.5 and 4.8.6: sequential execution, provenance, and nonzero exits.
    def test_passing_run_output_inputs_and_outputs(self):
        self.fx.entry["steps"].append(["python", "tests/two.py"])
        self.fx.entry["outputs"] = ["result*.txt"]
        self.fx.file("result-old.txt", "old output")
        os.utime(self.fx.main / "result-old.txt", (1, 1))
        self.fx.script("two.py", "from pathlib import Path\nprint('two output')\nPath('result-new.txt').write_text('made')\n")
        self.fx.save()
        rid = self.fx.start()
        record = self.fx.done(rid)
        self.assertEqual((record["status"], record["exits"]), ("passed", [0, 0]))
        log = self.fx.log(rid)
        for text in ("one output", "two output", "$ python ", "[exit 0]"):
            self.assertIn(text, log)
        self.assertEqual(log.count("$ python "), 2)
        self.assertEqual(record["outputs"], ["result-new.txt"])
        self.assertFalse(record["inputs"]["incomplete"])
        self.assertEqual(record["inputs"]["files"]["tests/one.py"], hashlib.sha256((self.fx.main / "tests/one.py").read_bytes()).hexdigest())
        self.assertEqual(record["registry_sha256"], hashlib.sha256(self.fx.registry.read_bytes()).hexdigest())
        self.assertTrue(Path(record["python"]).is_file())
        self.assertEqual(record["lock"], "rid-" + rid + ".lock")
        self.assertTrue((self.fx.data / "runs/locks" / record["entry_lock"]).is_file())
        self.assertTrue(all(record[k].endswith("Z") for k in ("created", "started", "ended", "alive_at")))
        recent = runs.recent(self.fx.data)[0]
        self.assertGreaterEqual(recent["duration_s"], 0)
        self.assertEqual(recent["status"], "passed")

    def test_nonzero_exit_does_not_skip_the_next_step(self):
        self.fx.script("one.py", "raise SystemExit(3)\n")
        self.fx.entry["steps"].append(["python", "tests/two.py"])
        self.fx.save()
        rid = self.fx.start()
        record = self.fx.done(rid)
        self.assertEqual((record["status"], record["exits"]), ("failed", [3, 0]))
        self.assertIn("two output", self.fx.log(rid))
        self.assertIn("[exit 3]", self.fx.log(rid))

    # 4.8.7: no flush in the script, and LogTail sees it while still running.
    def test_unflushed_live_output(self):
        self.fx.script("one.py", "import time\nprint('UNFLUSHED_MARKER')\ntime.sleep(5)\n")
        rid = self.fx.start()
        tail = runs.LogTail(self.fx.data / "runs" / (rid + ".log"))
        lines = []

        def marker():
            lines.extend(tail.poll())
            return "UNFLUSHED_MARKER" in lines

        wait_for(marker, 4, "unbuffered marker was not available during the sleep")
        self.assertEqual(self.fx.record(rid)["status"], "running")
        self.assertIn("UNFLUSHED_MARKER", self.fx.log(rid))
        runs.cancel(self.fx.data, rid)
        self.assertEqual(self.fx.done(rid)["status"], "cancelled")

    # 4.8.8-10: nested jobs contain grandchildren on timeout, cancel, and crash.
    @unittest.skipUnless(WINDOWS, "Windows Job Objects")
    def test_timeout_kills_grandchild(self):
        self.tree()
        self.fx.entry["steps"].append(["python", "tests/two.py"])
        self.fx.save()
        rid = self.fx.start()
        pid = self.tree_pid()
        record = self.fx.done(rid, 17)
        self.assertEqual(record["status"], "timed_out")
        self.assertEqual(len(record["exits"]), 1)
        self.assertNotIn("two output", self.fx.log(rid))
        self.assert_gone(pid)

    @unittest.skipUnless(WINDOWS, "Windows Job Objects")
    def test_cancel_kills_grandchild(self):
        self.tree()
        rid = self.fx.start()
        pid = self.tree_pid()
        runs.cancel(self.fx.data, rid)
        self.assertEqual(self.fx.done(rid)["status"], "cancelled")
        self.assert_gone(pid)

    @unittest.skipUnless(WINDOWS, "Windows Job Objects")
    def test_runner_crash_kills_grandchild_and_reports_interrupted(self):
        self.tree()
        rid = self.fx.start()
        pid = self.tree_pid()
        self.fx.kill(rid)
        self.assert_gone(pid)
        before = self.fx.record_path(rid).read_bytes()
        record = next(r for r in runs.recent(self.fx.data) if r["rid"] == rid)
        self.assertEqual(record["status"], "interrupted")
        self.assertEqual(record["reason"], "the runner ended without a final record")
        self.assertEqual(self.fx.record_path(rid).read_bytes(), before)

    # 4.8.11: a real lock holder beats a stale heartbeat, but prevents duplicates.
    def test_stale_heartbeat_with_live_liveness_and_entry_locks(self):
        rid = self.fx.start(reserve=True)
        record = self.fx.record(rid)
        record.update(status="running", started=iso_at(time.time() - 65), alive_at=iso_at(time.time() - 60),
                      entry_lock=runs._entry_lock_name(self.fx.main, "checks"))
        runs._write_record(self.fx.data / "runs", record)
        locks = self.fx.data / "runs/locks"
        locks.mkdir()
        ready = self.fx.data / "runs/holder.ready"
        holder = self.fx.script("holder.py", f"""
            import sys, time
            from pathlib import Path
            sys.path.insert(0, {str(WB)!r})
            import runs
            a = runs._take_lock(Path({str(locks / record['lock'])!r}))
            b = runs._take_lock(Path({str(locks / record['entry_lock'])!r}))
            Path({str(ready)!r}).write_text('ready')
            time.sleep(60)
        """)
        proc = REAL_POPEN([sys.executable, str(holder)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        self.fx.children.append(proc)
        wait_for(ready.is_file)
        viewed = runs.recent(self.fx.data)[0]
        self.assertEqual(viewed["status"], "running")
        self.assertGreaterEqual(viewed["stale_heartbeat_s"], 60)
        duplicate = self.fx.start()
        record = self.fx.done(duplicate)
        self.assertEqual((record["status"], record["reason"]), ("refused", "already running"))
        proc.terminate()
        proc.wait(timeout=5)

    # 4.8.12: entry reservation is atomic, including case aliases on Windows.
    def test_atomic_entry_reservation(self):
        self.fx.script("one.py", """
            import time
            from pathlib import Path
            with Path('executions.txt').open('a') as f:
                f.write('execution\\n')
            time.sleep(3)
        """)
        first = self.fx.start()
        checkout = Path(str(self.fx.main).upper()) if WINDOWS else self.fx.main
        second = self.fx.start(checkout=checkout)
        records = [self.fx.done(rid) for rid in (first, second)]
        self.assertEqual(sorted(r["status"] for r in records), ["passed", "refused"])
        self.assertEqual((self.fx.main / "executions.txt").read_text().splitlines(), ["execution"])
        refusal = next(r for r in records if r["status"] == "refused")
        self.assertEqual(refusal["reason"], "already running")

    # 4.8.13: expected entry digest is checked again before any step starts.
    def test_entry_changed_after_display_refuses_without_execution(self):
        digest = runs.plan_run(self.fx.main, self.fx.main, "checks")["digest"]
        rid = self.fx.start(expect=digest, reserve=True)
        self.fx.script("changed.py", "from pathlib import Path\nPath('changed.marker').write_text('wrong')\n")
        self.fx.entry["steps"] = [["python", "tests/changed.py"]]
        self.fx.save()
        self.fx.launch_reserved(rid)
        record = self.fx.done(rid)
        self.assertEqual(record["status"], "refused")
        self.assertEqual(record["reason"], "the registry entry changed after it was shown")
        self.assertEqual(record["exits"], [])
        self.assertFalse((self.fx.main / "changed.marker").exists())

    # 4.8.14: wrapper refusal and a successful call ID are both recorded.
    def test_codex_wrapper_exit_and_call_id(self):
        rid = self.fx.start(codex=self.codex())
        record = self.fx.done(rid)
        self.assertEqual((record["status"], record["exits"]), ("failed", [2]))
        self.assertIsNone(record["call_id"])
        self.assertIsNone(record["entry_lock"])
        self.fx.wrapper(f"print('review {CALL_ID}: pass')\n")
        rid = self.fx.start(codex=self.codex())
        record = self.fx.done(rid)
        self.assertEqual((record["status"], record["call_id"]), ("passed", CALL_ID))
        self.assertIsNone(record["timeout_s"])
        self.assertIn("work/reviews/packets/check.md", record["inputs"]["files"])

    # 4.8.15: bounded record reads and incremental binary log tails.
    def test_malformed_records_are_counted_and_recent_is_bounded(self):
        rid = self.fx.start(reserve=True)
        directory = self.fx.data / "runs"
        (directory / "20260930T000000Z-00000001.json").write_bytes(b'{"rid":')
        (directory / "20260930T000000Z-00000002.json").write_bytes(b" " * (runs.JSON_LIMIT + 1))
        records = runs.recent(self.fx.data)
        self.assertEqual([r["rid"] for r in records], [rid])
        self.assertEqual(runs.recent.malformed, 2)
        self.assertEqual(len(runs.recent(self.fx.data, limit=1)), 1)
        self.assertEqual(runs.recent.malformed, 0)
        self.assertIsNone(records[0]["duration_s"])

    def test_recent_skips_wrong_field_types_and_keeps_the_attention_window(self):
        directory = self.fx.data / "runs"
        directory.mkdir(parents=True, exist_ok=True)
        now = time.time()
        base = {"schema": 1, "kind": "registry", "run": "checks", "title": "Fixture checks", "checkout": str(self.fx.main),
                "status": "failed", "created": iso_at(now - 60), "started": iso_at(now - 60), "ended": iso_at(now - 30),
                "alive_at": iso_at(now - 30), "exits": [1], "reason": None, "call_id": None, "outputs": []}

        def write(rid, **fields):
            (directory / f"{rid}.json").write_text(json.dumps({**base, "rid": rid, **fields}), encoding="utf-8")
            return rid

        # Each of these once reached the app, where run=[] raised TypeError in refresh.
        bad = [{"run": []}, {"run": ""}, {"title": None}, {"kind": "other"}, {"checkout": 3}, {"exits": [True]},
               {"exits": "1"}, {"reason": 5}, {"call_id": []}, {"started": "yesterday"}, {"outputs": {}}]
        for i, fields in enumerate(bad):
            write(f"20260930T0100{i:02d}Z-000000b{i:x}", **{"status": "running", **fields})
        good = write("20260930T020000Z-0000000a")
        self.assertEqual([r["rid"] for r in runs.recent(self.fx.data, now)], [good])
        self.assertEqual(runs.recent.malformed, len(bad))
        # Beyond the display limit, a record stays visible to the flag rules while it was written within the window:
        # a running record is rewritten at each heartbeat, an ended one at its end.
        live = write("20260930T000000Z-0000000c", status="running", ended=None)
        stale = write("20260930T000001Z-0000000d")
        os.utime(directory / f"{stale}.json", (now - 90000, now - 90000))
        newest = [write(f"20260930T03{i:02d}00Z-000000e{i}") for i in range(3)]
        self.assertEqual([r["rid"] for r in runs.recent(self.fx.data, now, limit=3)], newest[::-1])
        self.assertEqual([r["rid"] for r in runs.recent(self.fx.data, now, limit=3, window=86400)], [*newest[::-1], good, live])
        self.assertEqual(runs.recent.malformed, len(bad))

    def test_log_tail_partial_utf8_limits_shrink_and_missing(self):
        file = self.fx.main / "tail.log"
        tail = runs.LogTail(file)
        self.assertEqual(tail.poll(), [])
        file.write_bytes(b"first\npartial")
        self.assertEqual(tail.poll(max_read=3), [])
        self.assertEqual(tail.poll(max_read=3), ["first"])
        self.assertEqual(tail.poll(), [])
        with file.open("ab") as f:
            f.write(b" done\nutf8 \xe2")
        self.assertEqual(tail.poll(), ["partial done"])
        with file.open("ab") as f:
            f.write(b"\x82\xac\ninvalid \xff\r\n")
        self.assertEqual(tail.poll(), ["utf8 \u20ac", "invalid \ufffd"])
        self.assertEqual(tail.poll(), [])
        file.write_bytes(b"new\n")
        self.assertEqual(tail.poll(), ["new"])
        file.unlink()
        self.assertEqual(tail.poll(), [])

    # 4.8.16 and 4.8.17: the J0 ownership gap and suspended failure cleanup.
    @unittest.skipUnless(WINDOWS, "Windows suspended startup")
    def test_runner_crash_before_step_job_assignment(self):
        barrier = self.fx.data / "runs/barrier"
        barrier.mkdir(parents=True)
        rid = self.fx.start(env={"WORKBENCH_RUNNER_TEST_BARRIER": str(barrier)})
        reached = barrier / "before-assign.reached"
        pid = wait_for(lambda: int(reached.read_text()) if reached.is_file() and reached.read_text().strip() else None)
        self.assertTrue(pid_alive(pid))
        self.fx.kill(rid)
        self.assert_gone(pid)
        self.assertNotIn("one output", self.fx.log(rid))

    @unittest.skipUnless(WINDOWS, "Windows suspended startup")
    def test_assignment_failure_terminates_suspended_step(self):
        barrier = self.fx.data / "runs/barrier"
        barrier.mkdir(parents=True)
        (barrier / "go").touch()
        rid = self.fx.start(env={"WORKBENCH_RUNNER_TEST_BARRIER": str(barrier), "WORKBENCH_RUNNER_TEST_FAIL": "assign"})
        record = self.fx.done(rid)
        pid = int((barrier / "before-assign.reached").read_text())
        self.assertEqual((record["status"], record["exits"]), ("failed", [None]))
        self.assertIn("assignment", record["reason"])
        self.assert_gone(pid)
        self.assertNotIn("one output", self.fx.log(rid))

    # 4.8.18: a subsequent run's entry lock cannot make a dead run appear alive.
    @unittest.skipUnless(WINDOWS, "Windows runner crash ownership")
    def test_liveness_is_per_run_when_entry_is_reused(self):
        self.fx.script("one.py", "import time\nprint('sleeping')\ntime.sleep(60)\n")
        a = self.fx.start()
        wait_for(lambda: "sleeping" in self.fx.log(a))
        self.fx.kill(a)
        b = self.fx.start()
        wait_for(lambda: "sleeping" in self.fx.log(b))
        records = {r["rid"]: r for r in runs.recent(self.fx.data)}
        self.assertEqual(records[a]["status"], "interrupted")
        self.assertEqual(records[b]["status"], "running")
        runs.cancel(self.fx.data, b)
        self.assertEqual(self.fx.done(b)["status"], "cancelled")

    # 4.8.19: stale starting records cannot resurrect an interrupted run.
    def test_late_and_missing_starting_records(self):
        rid = self.fx.start(reserve=True)
        record = self.fx.record(rid)
        record["created"] = iso_at(time.time() - 40)
        runs._write_record(self.fx.data / "runs", record)
        before = self.fx.record_path(rid).read_bytes()
        recent = runs.recent(self.fx.data)[0]
        self.assertEqual((recent["status"], recent["reason"]), ("interrupted", "the runner did not start"))
        self.assertEqual(self.fx.record_path(rid).read_bytes(), before)
        record["created"] = iso_at(time.time() - 26)
        runs._write_record(self.fx.data / "runs", record)
        self.fx.launch_reserved(rid)
        ended = self.fx.done(rid)
        self.assertEqual((ended["status"], ended["reason"]), ("refused", "the runner started too late"))
        self.assertEqual(ended["exits"], [])
        self.assertNotIn("one output", self.fx.log(rid))
        missing = self.fx.start(reserve=True)
        self.fx.record_path(missing).unlink()
        self.fx.launch_reserved(missing)
        ended = self.fx.done(missing)
        self.assertEqual((ended["status"], ended["reason"]), ("refused", "no starting record"))
        self.assertEqual(ended["exits"], [])
        self.assertIsNone(ended["python"])

    # 4.8.20: Codex exceeds the registry minimum deadline, but remains cancellable.
    def test_codex_has_no_outer_time_limit_and_can_be_cancelled(self):
        self.fx.wrapper(f"import time\ntime.sleep(12)\nprint('implement {CALL_ID}: 1 changed file(s); acceptance passed')\n")
        rid = self.fx.start(codex=self.codex(kind="implement"))
        initial = wait_for(lambda: (r if (r := self.fx.record(rid)).get("status") == "running" else None))
        wait_for(lambda: self.fx.record(rid).get("alive_at") != initial["alive_at"], 7, "runner heartbeat was not rewritten")
        self.assertEqual(self.fx.record(rid)["status"], "running")
        record = self.fx.done(rid, 20)
        self.assertEqual((record["status"], record["call_id"], record["exits"]), ("passed", CALL_ID, [0]))
        duration = runs._timestamp(record["ended"]) - runs._timestamp(record["started"])
        self.assertGreaterEqual(duration, 12)
        self.assertIsNone(record["timeout_s"])
        self.fx.wrapper("import time\nprint('wrapper sleeping')\ntime.sleep(60)\n")
        rid = self.fx.start(codex=self.codex(kind="implement"))
        wait_for(lambda: "wrapper sleeping" in self.fx.log(rid))
        runs.cancel(self.fx.data, rid)
        self.assertEqual(self.fx.done(rid)["status"], "cancelled")

    def test_runner_rejects_invalid_rid_and_extra_options_without_writes(self):
        argv = [sys.executable, str(RUNNER), "--main", str(self.fx.main), "--checkout", str(self.fx.main),
                "--rid", "../bad", "--run", "checks", "--expect", "ignored"]
        for extra in ([], ["--extra", "value"], ["--timeout", "1"], ["--kind", "review"], ["--mai", "x"]):
            with self.subTest(extra=extra):
                result = subprocess.run([*argv, *extra], capture_output=True, timeout=10)
                self.assertEqual(result.returncode, 2)
                self.assertFalse(self.fx.data.exists())

    def test_start_uses_local_runner_detachment_and_console_interpreter(self):
        console = self.fx.file("interpreter/python.exe", "unused")
        windowless = console.with_name("pythonw.exe")
        with mock.patch.object(runs.sys, "executable", str(windowless)), mock.patch.object(runs.subprocess, "Popen") as spawn:
            rid = runs.start(self.fx.main, self.fx.main, run_id="checks")
        argv = spawn.call_args.args[0]
        kwargs = spawn.call_args.kwargs
        self.assertEqual(argv[0], str(console))
        self.assertEqual(argv[1], str(RUNNER.resolve()))
        self.assertEqual(kwargs["cwd"], str(self.fx.main))
        self.assertNotIn("shell", kwargs)
        for field in ("stdin", "stdout", "stderr"):
            self.assertEqual(kwargs[field], subprocess.DEVNULL)
        if WINDOWS:
            self.assertEqual(kwargs["creationflags"], subprocess.CREATE_NO_WINDOW | subprocess.CREATE_NEW_PROCESS_GROUP)
        else:
            self.assertTrue(kwargs["start_new_session"])
        self.assertEqual(self.fx.record(rid)["status"], "starting")
        for args in ({}, {"run_id": "checks", "codex": self.codex()}, {"codex": {**self.codex(), "extra": "value"}}):
            with self.assertRaises(runs.RunRefused):
                runs.start(self.fx.main, self.fx.main, **args)
        with mock.patch.object(runs.subprocess, "Popen", side_effect=OSError("spawn refused")):
            with self.assertRaises(runs.RunRefused):
                runs.start(self.fx.main, self.fx.main, run_id="checks")
        self.assertTrue(any(r["status"] == "refused" for r in runs.recent(self.fx.data)))
        with self.assertRaises(runs.RunRefused):
            runs.cancel(self.fx.data, "../escape")

    def test_log_cap_and_provenance_caps(self):
        self.fx.script("one.py", f"import sys\nsys.stdout.buffer.write(b'x' * {runner.LOG_LIMIT + 1000} + b'\\n')\n")
        rid = self.fx.start()
        record = self.fx.done(rid)
        self.assertEqual(record["status"], "passed")
        self.assertEqual((self.fx.data / "runs" / (rid + ".log")).stat().st_size, runner.LOG_LIMIT)
        self.assertGreaterEqual(record["log_dropped"], 1000)
        directory = self.fx.main / "inputs"
        directory.mkdir()
        for i in range(501):
            (directory / f"{i:03}.txt").write_text("payload", encoding="utf-8")
        snapshot = runner._inputs(self.fx.main, ["inputs/*.txt"])
        self.assertEqual(len(snapshot["files"]), 500)
        self.assertTrue(snapshot["incomplete"])
        huge = directory / "huge.bin"
        with huge.open("wb") as f:
            f.truncate(runner.INPUT_BYTES + 1)
        snapshot = runner._inputs(self.fx.main, ["inputs/huge.bin"])
        self.assertEqual(snapshot, {"files": {}, "incomplete": True})


if __name__ == "__main__":
    unittest.main(verbosity=2)
