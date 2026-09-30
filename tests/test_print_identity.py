"""Read-only identity printing; standard-library tests, no compilation or GUI."""
import sys

sys.dont_write_bytecode = True

import ast
import getpass
import hashlib
import importlib
import io
import json
import os
import platform
import shutil
import subprocess
import unittest
import uuid
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path, PurePosixPath
from unittest import mock

ROOT = Path(__file__).resolve().parents[1]
EXPERIMENT = ROOT / 'work/experiments/magic600-04'
SCRIPT = EXPERIMENT / 'print_identity.py'
RUNNER = EXPERIMENT / 'tests/run_postapproval.py'
sys.path[:0] = [str(ROOT), str(EXPERIMENT)]
import native_launch

identity_v2 = native_launch.identity_v2
NOTES = 'source_identity is not build_identity; it excludes compiler and interpreter bindings.'


def harness_files():
    """Derive the expected set independently of the printer."""
    tree = ast.parse(RUNNER.read_text(encoding='utf-8'))
    main = next(node for node in tree.body
                if isinstance(node, ast.FunctionDef) and node.name == 'main')
    names = [node.value for node in ast.walk(main)
             if isinstance(node, ast.Constant) and isinstance(node.value, str)
             and node.value.endswith('.cs')]
    return (list(native_launch.HARNESS_SOURCES)
            + [EXPERIMENT / 'tests' / name for name in names]
            + [EXPERIMENT / 'tests/final_workflow_cases.json', RUNNER])


SKIPPED = {ROOT / 'work' / name for name in ('worktrees', 'reviews', 'loop-memory')}


def inventory():
    """Include ignored files and metadata, excluding Git administrative files and
    the directories concurrent agent calls write to."""
    files = {}
    for directory, dirs, names in os.walk(ROOT):
        dirs[:] = [name for name in dirs
                   if name != '.git' and Path(directory) / name not in SKIPPED]
        for name in names:
            if name == '.git':
                continue
            path = Path(directory) / name
            stat = path.stat()
            files[path.relative_to(ROOT).as_posix()] = (stat.st_size, stat.st_mtime_ns)
    return files


class OutputChecks(unittest.TestCase):
    def assert_public_output(self, text):
        for private in (str(ROOT), ROOT.as_posix(), str(native_launch.COMPILER),
                        sys.executable, getpass.getuser(), platform.node()):
            if private:
                self.assertNotIn(private.casefold(), text.casefold())
        self.assertNotRegex(text, r'[A-Za-z]:[\\/]')

    def assert_failure(self, result, stdout, stderr):
        self.assertEqual(result, 1)
        self.assertEqual(stdout, '')
        self.assertEqual(len(stderr.splitlines()), 1)
        self.assertTrue(stderr.strip())
        self.assert_public_output(stderr)


class PrintIdentityCommandTests(OutputChecks):
    @classmethod
    def setUpClass(cls):
        cls.runs, cls.inventories, cls.cache_entries = [], [], []
        # Python 3.14's Windows mode-0700 temporary directories exclude the
        # restricted sandbox token; inherit the worktree's permissions instead.
        cache = ROOT / ('tmp-print-identity-' + uuid.uuid4().hex)
        cache.mkdir()
        try:
            env = os.environ.copy()
            # CPython imports encodings/site before the script can set its flag.
            # Suppress startup writes, then test the script with writes enabled.
            env['PYTHONDONTWRITEBYTECODE'] = '1'
            env['PYTHONPYCACHEPREFIX'] = str(cache)
            bootstrap = (
                "import sys; sys.argv = sys.argv[1:]; "
                "sys.path.insert(0, 'work/experiments/magic600-04'); "
                "sys.dont_write_bytecode = False; "
                "exec(compile(open(sys.argv[0], encoding='utf-8').read(), "
                "sys.argv[0], 'exec'), "
                "{'__name__': '__main__', '__file__': sys.argv[0]})")
            commands = [[sys.executable, str(SCRIPT)], [sys.executable, str(SCRIPT)],
                        [sys.executable, '-B', '-c', bootstrap, str(SCRIPT)]]
            for command in commands:
                before = inventory()
                result = subprocess.run(command, cwd=ROOT, env=env,
                                        capture_output=True, timeout=120)
                after = inventory()
                cls.runs.append(result)
                cls.inventories.append((before, after))
                cls.cache_entries.append(list(cache.rglob('*')))
        finally:
            assert cache.resolve().parent == ROOT
            shutil.rmtree(cache)

    def data(self):
        return json.loads(self.runs[0].stdout)

    def test_command_prints_canonical_json(self):
        for result in self.runs:
            self.assertEqual(result.returncode, 0, result.stderr.decode())
            self.assertEqual(result.stderr, b'')
            data = json.loads(result.stdout)
            self.assertEqual(set(data), {'format', 'source_identity', 'product_files',
                                        'harness', 'build_identity',
                                        'build_identity_unavailable', 'notes'})
            self.assertEqual(data['format'], 'magic600-identity-print-v1')
            self.assertEqual(data['notes'], NOTES)
            expected = (json.dumps(data, sort_keys=True, indent=2) + '\n').encode()
            self.assertEqual(result.stdout.replace(b'\r\n', b'\n'), expected)
            self.assert_public_output(result.stdout.decode())

    def test_paths_are_repository_relative_posix(self):
        for name in self.data()['harness']:
            with self.subTest(path=name):
                self.assertNotIn('\\', name)
                self.assertNotIn(':', name)
                self.assertFalse(name.startswith('/'))
                self.assertNotIn('..', PurePosixPath(name).parts)
                self.assertNotIn(getpass.getuser().casefold(), name.casefold())
                self.assertEqual(PurePosixPath(name).as_posix(), name)

    def test_source_identity_and_product_count_match_checkout(self):
        product = identity_v2.product_binding(
            native_launch.product_sources(native_launch.native_sources()), ROOT)
        recipe = native_launch.compile_recipe('ExperimentProgram')
        data = self.data()
        self.assertEqual(data['source_identity'], identity_v2.digest(
            {'product': product, 'compile_recipe': recipe}))
        self.assertEqual(data['product_files'], len(identity_v2.product_files(product)))
        self.assertIs(type(data['product_files']), int)
        self.assertNotEqual(data['source_identity'], data['build_identity'])

    def test_harness_is_exactly_the_runner_inventory_with_sha256(self):
        expected = {identity_v2.relative(path, ROOT) for path in harness_files()}
        self.assertEqual(len(expected), 27)
        harness = self.data()['harness']
        self.assertEqual(len(harness), 27)
        self.assertEqual(set(harness), expected)
        self.assertNotIn('work/experiments/magic600-04/tests/test_startup_visibility.py', harness)
        for name, digest in harness.items():
            with self.subTest(path=name):
                path = ROOT / name
                self.assertTrue(path.is_file())
                self.assertEqual(digest, hashlib.sha256(path.read_bytes()).hexdigest())

    def test_two_runs_print_identical_bytes(self):
        self.assertEqual(self.runs[0].stdout, self.runs[1].stdout)

    def test_repository_files_are_unchanged(self):
        for before, after in self.inventories:
            self.assertEqual(after, before)

    def test_no_bytecode_cache_is_written(self):
        self.assertEqual(self.runs[2].stdout, self.runs[0].stdout)
        self.assertEqual(self.cache_entries, [[], [], []])

    def test_missing_numpy_reports_interpreter_unavailable(self):
        try:
            importlib.import_module('numpy')
        except ImportError:
            data = self.data()
            self.assertIsNone(data['build_identity'])
            self.assertIn('interpreter', data['build_identity_unavailable'])
        else:
            self.skipTest('NumPy imports; missing bindings are also exercised in process')

    def test_resolved_identity_matches_capture_identity(self):
        recipe = native_launch.compile_recipe('ExperimentProgram')
        tools = native_launch.current_tools({'compile_recipe': recipe})
        missing = [role for role in ('compiler', 'interpreter') if tools[role] is None]
        if missing:
            self.skipTest('Full build identity needs resolved tool bindings; unavailable: '
                          + ', '.join(missing))
        data = self.data()
        expected = native_launch.capture_identity(
            native_launch.native_sources(), 'ExperimentProgram')['build_identity']
        self.assertEqual(data['build_identity'], expected)
        self.assertEqual(data['build_identity_unavailable'], [])

    def test_any_argument_is_a_usage_error(self):
        for argument in ('--help', '', str(ROOT / 'private-input')):
            with self.subTest(argument=argument):
                result = subprocess.run([sys.executable, str(SCRIPT), argument], cwd=ROOT,
                                        capture_output=True, timeout=120)
                self.assertEqual(result.returncode, 2)
                self.assertEqual(result.stdout, b'')
                self.assertEqual(len(result.stderr.decode().splitlines()), 1)
                self.assertIn('usage', result.stderr.decode().lower())
                self.assert_public_output(result.stderr.decode())


class PrintIdentityInProcessTests(OutputChecks):
    @classmethod
    def setUpClass(cls):
        cls.printer = importlib.import_module('print_identity')
        cls.recipe = native_launch.compile_recipe('ExperimentProgram')
        cls.product = identity_v2.product_binding(
            native_launch.product_sources(native_launch.native_sources()), ROOT)
        cls.compiler = dict(banner='fixture compiler', files={
            **{'compiler:' + name: '0' * 64 for name in identity_v2.COMPILER_FILES},
            **{'reference:' + name.lower(): '1' * 64 for name in cls.recipe['references']}})
        cls.interpreter = dict(implementation='cpython', version='3.14.7', bits=64,
                               files={role: '2' * 64 for role in
                                      ('base:python.exe', 'base:python3.dll',
                                       'base:python314.dll', 'launcher')},
                               numpy=dict(version='2.3.5',
                                          payload=dict(files=1, digest='3' * 64)))

    def run_main(self):
        stdout, stderr = io.StringIO(), io.StringIO()
        with mock.patch.object(sys, 'argv', [str(SCRIPT)]), \
                redirect_stdout(stdout), redirect_stderr(stderr):
            result = self.printer.main()
        return result, stdout.getvalue(), stderr.getvalue()

    def test_resolved_bindings_produce_full_v2_identity_without_compiling(self):
        with mock.patch.object(native_launch, 'compiler_banner', return_value='fixture compiler'), \
                mock.patch.object(identity_v2, 'compiler_binding', return_value=self.compiler), \
                mock.patch.object(identity_v2, 'current_interpreter', return_value=self.interpreter), \
                mock.patch.object(native_launch, 'compile_program',
                                  side_effect=AssertionError('printer must not compile')):
            data = self.printer.report()
            expected = native_launch.capture_identity(
                native_launch.native_sources(), 'ExperimentProgram')['build_identity']
            result, stdout, stderr = self.run_main()
        self.assertIsInstance(data, dict)
        self.assertEqual(data['build_identity'], expected)
        self.assertEqual(data['build_identity_unavailable'], [])
        self.assertNotEqual(data['source_identity'], data['build_identity'])
        self.assertEqual(result, 0)
        self.assertEqual(stderr, '')
        self.assertEqual(stdout, json.dumps(data, sort_keys=True, indent=2) + '\n')

    def test_both_missing_bindings_preserve_source_identity_and_role_order(self):
        private = str(ROOT / 'private-tool') + '\nprivate diagnostic'
        with mock.patch.object(native_launch, 'compiler_banner', return_value='fixture compiler'), \
                mock.patch.object(identity_v2, 'compiler_binding', side_effect=ValueError(private)), \
                mock.patch.object(identity_v2, 'current_interpreter', side_effect=ImportError(private)):
            result, stdout, stderr = self.run_main()
        self.assertEqual(result, 0)
        self.assertEqual(stderr, '')
        data = json.loads(stdout)
        self.assertIsNone(data['build_identity'])
        self.assertEqual(data['build_identity_unavailable'], ['compiler', 'interpreter'])
        self.assertEqual(data['source_identity'], identity_v2.digest(
            {'product': self.product, 'compile_recipe': self.recipe}))
        self.assert_public_output(stdout)

    def test_each_binding_uses_current_tools_exception_policy(self):
        failures = [('compiler', error) for error in
                    (OSError, ValueError, subprocess.SubprocessError)]
        failures += [('interpreter', error) for error in (OSError, ValueError, ImportError)]
        for role, error in failures:
            with self.subTest(role=role, error=error.__name__):
                compiler = {'return_value': self.compiler}
                interpreter = {'return_value': self.interpreter}
                (compiler if role == 'compiler' else interpreter).update(
                    side_effect=error(str(ROOT / 'private-tool')))
                with mock.patch.object(native_launch, 'compiler_banner', return_value='fixture compiler'), \
                        mock.patch.object(identity_v2, 'compiler_binding', **compiler), \
                        mock.patch.object(identity_v2, 'current_interpreter', **interpreter):
                    data = self.printer.report()
                self.assertIsNone(data['build_identity'])
                self.assertEqual(data['build_identity_unavailable'], [role])

    def test_ast_inventory_ignores_constants_outside_main(self):
        expected = {identity_v2.relative(path, ROOT) for path in harness_files()}
        tree = ast.parse(RUNNER.read_text(encoding='utf-8'))
        tree.body.append(ast.Expr(value=ast.Constant(value='UnboundOutsideMain.cs')))
        with mock.patch.object(ast, 'parse', return_value=tree):
            data = self.printer.report()
        self.assertEqual(set(data['harness']), expected)

    def test_wrong_cs_count_fails_without_json(self):
        for count in (21, 23):
            with self.subTest(count=count):
                tree = ast.parse(RUNNER.read_text(encoding='utf-8'))
                main = next(node for node in tree.body
                            if isinstance(node, ast.FunctionDef) and node.name == 'main')
                if count == 21:
                    name = next(node for node in ast.walk(main)
                                if isinstance(node, ast.Constant)
                                and isinstance(node.value, str) and node.value.endswith('.cs'))
                    name.value = 'not-a-check.txt'
                else:
                    # Use an existing file so missing-file handling cannot hide
                    # a missing count check.
                    main.body.append(ast.Expr(value=ast.Constant(value='SessionLogNativeChecks.cs')))
                with mock.patch.object(ast, 'parse', return_value=tree):
                    self.assert_failure(*self.run_main())

    def test_missing_harness_file_fails_without_json_or_private_paths(self):
        original_open = Path.open
        for missing in (EXPERIMENT / 'tests/SessionLogNativeChecks.cs',
                        EXPERIMENT / 'tests/final_workflow_cases.json', RUNNER):
            with self.subTest(file=identity_v2.relative(missing, ROOT)):
                def open_file(path, *args, **kwargs):
                    if path == missing:
                        raise FileNotFoundError(str(missing) + '\nprivate diagnostic')
                    return original_open(path, *args, **kwargs)

                with mock.patch.object(Path, 'open', open_file):
                    self.assert_failure(*self.run_main())


if __name__ == '__main__':
    unittest.main()
