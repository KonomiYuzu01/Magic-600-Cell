"""B4-12 fixture rules, with real engine checks when its environment is available."""
from __future__ import annotations

from contextlib import redirect_stderr, redirect_stdout
from copy import deepcopy
import hashlib
import io
import json
import os
from pathlib import Path
import re
import secrets
import shutil
import sqlite3
import subprocess
import sys
from types import SimpleNamespace
import unittest
from unittest import mock

ROOT = Path(__file__).resolve().parents[1]
TOOL = ROOT / 'tools/perf/b412_fixture.py'
sys.dont_write_bytecode = True
sys.path.insert(0, str(TOOL.parent))
import b412_fixture as fixture


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def good_record(profile='basic', content=b'hand-made database'):
    count = 0 if profile == 'basic' else int(profile.removeprefix('history-'))
    return {
        'format': 'magic600-b412-fixture-v1', 'profile': profile, 'word': [1, 601],
        'checkpoints': count, 'checkpoint_names': ['B412 history %04d' % i for i in range(1, count + 1)],
        'state_hash': 'a' * 64, 'head': 1, 'journal_depth': 1,
        'database_sha256': sha256(content), 'database_bytes': len(content),
        'builder_sha256': 'b' * 64,
        'engine_sources': {path: 'c' * 64 for path in
                           ('core.py', 'session.py', 'session_lock.py', 'assets/manifest.json')},
        'created_utc': '2026-09-30T20:00:00Z',
    }


BAD_FIELDS = {
    'format': 'other-format', 'profile': 'history-01', 'word': [True, 601],
    'checkpoints': True, 'checkpoint_names': ['extra checkpoint'], 'state_hash': 'g' * 64,
    'head': True, 'journal_depth': -1, 'database_sha256': 'a' * 63,
    'database_bytes': 0, 'builder_sha256': None, 'engine_sources': [],
    'created_utc': '2026-09-30T20:00:00',
}


class FixtureTestCase(unittest.TestCase):
    def setUp(self):
        # CPython 3.14's Windows mode-0700 temporary directories exclude the sandbox token.
        self.base = (ROOT / ('b412-test-' + secrets.token_hex(8))).resolve()
        self.assertEqual(self.base.parent, ROOT)
        self.base.mkdir()
        self.addCleanup(shutil.rmtree, self.base)
        self.local = self.base / 'local'
        environment = mock.patch.dict(os.environ, {'LOCALAPPDATA': str(self.local)})
        environment.start()
        self.addCleanup(environment.stop)

    def make_fixture(self, name='fixture'):
        directory = self.base / name
        (directory / 'template').mkdir(parents=True)
        content = b'hand-made database'
        (directory / 'template/session.sqlite3').write_bytes(content)
        record = good_record(content=content)
        self.write_record(directory, record)
        return directory, record

    @staticmethod
    def write_record(directory, record):
        (directory / 'fixture.json').write_text(json.dumps(record), encoding='utf-8')

    def cli(self, *arguments):
        environment = dict(os.environ, PYTHONDONTWRITEBYTECODE='1')
        return subprocess.run([sys.executable, str(TOOL), *map(str, arguments)], cwd=ROOT,
                              env=environment, capture_output=True, text=True, timeout=180)

    def assert_refusal(self, result):
        self.assertEqual(result.returncode, 2, result.stderr)
        self.assertEqual(result.stdout, '')
        self.assertEqual(len(result.stderr.splitlines()), 1)
        self.assertTrue(result.stderr.startswith('b412_fixture: '))
        self.assertNotIn('Traceback', result.stderr)
        self.assertIsNone(re.search(r'[A-Za-z]:[\\/]|(?:^|\s)(?:/|\\\\)', result.stderr))

    def assert_record(self, result):
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stderr, '')
        self.assertEqual(len(result.stdout.splitlines()), 1)
        return json.loads(result.stdout)


class StdlibTests(FixtureTestCase):
    def test_import_does_not_load_engine(self):
        code = ("import sys; sys.path.insert(0, 'tools/perf'); import b412_fixture; "
                "assert not {'numpy', 'core', 'session'} & sys.modules.keys()")
        result = subprocess.run([sys.executable, '-B', '-c', code], cwd=ROOT,
                                capture_output=True, text=True, timeout=30)
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_missing_engine_skip_and_required_failure(self):
        with mock.patch.dict(sys.modules, {'numpy': None}):
            with mock.patch.dict(os.environ, {'C600_REQUIRE_ENGINE': '0'}):
                with self.assertRaisesRegex(unittest.SkipTest, 'NumPy or engine modules unavailable'):
                    EngineTests.setUpClass()
            with mock.patch.dict(os.environ, {'C600_REQUIRE_ENGINE': '1'}):
                with self.assertRaisesRegex(AssertionError, 'NumPy or engine modules unavailable'):
                    EngineTests.setUpClass()

    def test_profiles(self):
        self.assertEqual(fixture.WORD, [1, 601])
        self.assertEqual(fixture.parse_profile('basic'), 0)
        for count in (1, 3, 400, 1999, 2000):
            with self.subTest(count=count):
                self.assertEqual(fixture.parse_profile('history-' + str(count)), count)
        for profile in ('history-0', 'history-01', 'history-2001', 'history--1', 'history-1.0',
                        'history-1\n', 'history-\u0661', 'basic ', 'garbage', 'history-' + '9' * 5000,
                        '', None, 1, False):
            with self.subTest(profile=str(profile)[:40]):
                with self.assertRaises(ValueError):
                    fixture.parse_profile(profile)

    def test_default_directory_guard(self):
        default = self.local / 'C600Studio'
        release = self.local / 'Magic600Cell'
        blocked = [default, default / 'child', default / 'child/../nested',
                   release, release / '0.4', release / '0.4/child']
        if os.name == 'nt':
            blocked.append(Path(str(default / 'nested').swapcase()))
            blocked.append(Path(str(release / '0.4').swapcase()))
        for candidate in blocked:
            with self.subTest(candidate=candidate.name):
                with self.assertRaises(ValueError):
                    fixture.guard_path(candidate)
        for candidate in (self.base / 'outside', self.local / 'C600Studio-other',
                          self.local / 'Magic600Cell-other', self.local):
            self.assertEqual(fixture.guard_path(candidate), candidate.resolve())
        with mock.patch.dict(os.environ), mock.patch.object(Path, 'home', return_value=self.base):
            os.environ.pop('LOCALAPPDATA', None)
            with self.assertRaises(ValueError):
                fixture.guard_path(self.base / 'C600Studio/child')
            with self.assertRaises(ValueError):
                fixture.guard_path(self.base / 'Magic600Cell/0.4')
        self.assertFalse(default.exists())
        self.assertFalse(release.exists())

    def test_valid_records(self):
        for profile in ('basic', 'history-3', 'history-2000'):
            fixture.validate_fixture(good_record(profile))
        record = good_record()
        record['state_hash'] = 'A' * 64
        record['created_utc'] = '2026-09-30T20:00:00.123456Z'
        fixture.validate_fixture(record)

    def test_exact_fields_and_field_validation(self):
        for missing in good_record():
            record = good_record()
            del record[missing]
            with self.subTest(missing=missing), self.assertRaises(ValueError):
                fixture.validate_fixture(record)
        for record in (None, [], dict(good_record(), extra='value')):
            with self.assertRaises(ValueError):
                fixture.validate_fixture(record)
        for field, value in BAD_FIELDS.items():
            record = good_record()
            record[field] = value
            with self.subTest(field=field), self.assertRaises(ValueError):
                fixture.validate_fixture(record)

    def test_record_types_and_consistency(self):
        replacements = {
            'format': [None, 1], 'profile': [None, 1],
            'word': [None, (1, 601), [1, 602], [1.0, 601], [1]],
            'checkpoints': [None, 0.0, -1, 1],
            'checkpoint_names': [None, '', [False]],
            'head': [None, 1.0, -1], 'journal_depth': [None, False, 1.0],
            'database_bytes': [None, True, 1.0, -1],
            'created_utc': [None, 1, 'garbageZ', '2026-02-30T20:00:00Z',
                            '2026-09-30T20:00:00+00:00'],
        }
        for field, values in replacements.items():
            for value in values:
                record = good_record()
                record[field] = value
                with self.subTest(field=field, value=value), self.assertRaises(ValueError):
                    fixture.validate_fixture(record)
        record = good_record('history-3')
        for names in ([], ['B412 history 0001'] * 2, [None] * 3):
            record['checkpoint_names'] = names
            with self.assertRaises(ValueError):
                fixture.validate_fixture(record)
        for key in ('state_hash', 'database_sha256', 'builder_sha256'):
            for value in (None, 1, '', 'f' * 63, 'f' * 65, 'x' * 64):
                record = good_record()
                record[key] = value
                with self.subTest(key=key, value=value), self.assertRaises(ValueError):
                    fixture.validate_fixture(record)
        for sources in ({}, {'core.py': 'c' * 64},
                        dict(good_record()['engine_sources'], extra='c' * 64)):
            record = good_record()
            record['engine_sources'] = sources
            with self.assertRaises(ValueError):
                fixture.validate_fixture(record)
        for source in good_record()['engine_sources']:
            record = good_record()
            record['engine_sources'][source] = 'bad hash'
            with self.subTest(source=source), self.assertRaises(ValueError):
                fixture.validate_fixture(record)

    def test_absolute_path_values(self):
        for value in ('C:\\private\\session', 'D:/private/session', '/private/session', '\\\\host\\session'):
            record = good_record('history-1')
            record['checkpoint_names'] = [value]
            with self.subTest(value=value), self.assertRaisesRegex(ValueError, 'absolute-path'):
                fixture.validate_fixture(record)
        record = good_record()
        record['engine_sources']['core.py'] = '/private/source'
        with self.assertRaisesRegex(ValueError, 'absolute-path'):
            fixture.validate_fixture(record)

    def test_copy_api_and_cli(self):
        directory, record = self.make_fixture()
        original = {p.relative_to(directory): p.read_bytes() for p in directory.rglob('*') if p.is_file()}
        expected = {'format': 'magic600-b412-fixture-copy-v1', 'profile': 'basic',
                    'database_sha256': record['database_sha256'], 'state_hash': record['state_hash']}
        for name in ('api-data', 'cli-data'):
            data = self.base / name
            actual = (fixture.copy_fixture(directory, data) if name == 'api-data' else
                      self.assert_record(self.cli('copy', '--fixture', directory, '--data', data)))
            self.assertEqual(actual, expected)
            self.assertEqual([p.name for p in data.iterdir()], ['session.sqlite3'])
            self.assertEqual(sha256((data / 'session.sqlite3').read_bytes()), record['database_sha256'])
        self.assertEqual(original, {p.relative_to(directory): p.read_bytes()
                                    for p in directory.rglob('*') if p.is_file()})

    def test_copy_rechecks_written_hash(self):
        directory, _ = self.make_fixture()
        data = self.base / 'data'
        with mock.patch.object(fixture.shutil, 'copyfile',
                               side_effect=lambda source, target: target.write_bytes(b'corrupted')):
            with self.assertRaisesRegex(ValueError, 'sha256-mismatch'):
                fixture.copy_fixture(directory, data)

    def test_existing_paths_are_preserved(self):
        directory, _ = self.make_fixture()
        for kind in ('directory', 'file'):
            target = self.base / kind
            if kind == 'directory':
                target.mkdir()
                sentinel = target / 'keep'
            else:
                sentinel = target
            sentinel.write_bytes(b'keep this')
            with self.assertRaises(ValueError):
                fixture.build_fixture('basic', target)
            with self.assertRaises(ValueError):
                fixture.copy_fixture(directory, target)
            self.assert_refusal(self.cli('build', '--profile', 'basic', '--output', target))
            self.assert_refusal(self.cli('copy', '--fixture', directory, '--data', target))
            self.assertEqual(sentinel.read_bytes(), b'keep this')

    def test_invalid_profiles_cli(self):
        output = self.base / 'output'
        for profile in ('history-0', 'history-01', 'history-2001', 'history-1\n', 'garbage'):
            with self.subTest(profile=profile):
                self.assert_refusal(self.cli('build', '--profile', profile, '--output', output))
                self.assertFalse(output.exists())

    def test_default_directory_refusals_cli(self):
        directory, _ = self.make_fixture()
        default = self.local / 'C600Studio'
        release = self.local / 'Magic600Cell/0.4'
        for target in (default, default / 'child', release, release / 'child'):
            calls = [
                ('build', '--profile', 'basic', '--output', target),
                ('copy', '--fixture', directory, '--data', target),
                ('verify', '--fixture', directory, '--data', target),
                ('copy', '--fixture', target, '--data', self.base / 'data'),
                ('verify', '--fixture', target, '--data', self.base / 'data'),
            ]
            for call in calls:
                with self.subTest(command=call[0], path=target.name):
                    self.assert_refusal(self.cli(*call))
        self.assertFalse(default.exists())
        self.assertFalse(release.parent.exists())
        self.assertFalse((self.base / 'data').exists())

    def test_unclean_templates_and_hash_mismatch(self):
        directory, _ = self.make_fixture()
        data = self.base / 'data'
        for extra in ('engine.lock', 'session.sqlite3-wal', 'other'):
            path = directory / 'template' / extra
            path.write_bytes(b'extra')
            with self.subTest(extra=extra):
                with self.assertRaises(ValueError):
                    fixture.copy_fixture(directory, data)
                self.assert_refusal(self.cli('copy', '--fixture', directory, '--data', data))
                self.assertFalse(data.exists())
            path.unlink()
        extra_directory = directory / 'template/extra-directory'
        extra_directory.mkdir()
        self.assert_refusal(self.cli('copy', '--fixture', directory, '--data', data))
        extra_directory.rmdir()
        database = directory / 'template/session.sqlite3'
        database.write_bytes(b'changed')
        with self.assertRaisesRegex(ValueError, 'sha256-mismatch'):
            fixture.copy_fixture(directory, data)
        self.assert_refusal(self.cli('copy', '--fixture', directory, '--data', data))
        database.unlink()
        self.assert_refusal(self.cli('copy', '--fixture', directory, '--data', data))
        database.mkdir()
        self.assert_refusal(self.cli('copy', '--fixture', directory, '--data', data))
        self.assertFalse(data.exists())

    def test_invalid_fixture_json_cli(self):
        directory, good = self.make_fixture()
        for field, value in BAD_FIELDS.items():
            record = deepcopy(good)
            record[field] = value
            self.write_record(directory, record)
            for command in ('copy', 'verify'):
                with self.subTest(field=field, command=command):
                    self.assert_refusal(self.cli(command, '--fixture', directory, '--data', self.base / 'data'))
        for content in ('{invalid json', '[]', '{}'):
            (directory / 'fixture.json').write_text(content, encoding='utf-8')
            self.assert_refusal(self.cli('copy', '--fixture', directory, '--data', self.base / 'data'))
        record = good_record('history-1')
        record['checkpoint_names'] = ['/private/checkpoint']
        self.write_record(directory, record)
        self.assert_refusal(self.cli('copy', '--fixture', directory, '--data', self.base / 'data'))
        (directory / 'fixture.json').unlink()
        self.assert_refusal(self.cli('copy', '--fixture', directory, '--data', self.base / 'data'))
        self.assertFalse((self.base / 'data').exists())

    def test_verify_refuses_template_and_missing_database(self):
        directory, _ = self.make_fixture()
        template = directory / 'template'
        self.assert_refusal(self.cli('verify', '--fixture', directory, '--data', template))
        self.assertEqual([p.name for p in template.iterdir()], ['session.sqlite3'])
        missing = self.base / 'missing'
        self.assert_refusal(self.cli('verify', '--fixture', directory, '--data', missing))
        self.assertFalse(missing.exists())

    def test_usage_errors_are_one_line_and_redacted(self):
        for args in ((), ('unknown',), ('build',), ('copy', '--fixture', '/private/fixture'),
                     ('verify', '--unknown', 'C:\\private\\argument')):
            with self.subTest(args=args):
                self.assert_refusal(self.cli(*args))

    def test_failed_build_closes_session_and_preserves_evidence(self):
        expected = 'a' * 64
        cases = {'state_hash': 'state-mismatch', 'post_state': 'state-mismatch',
                 'extra_file': 'template-not-clean'}
        for mismatch, reason in cases.items():
            output = self.base / mismatch
            model = mock.Mock()
            model.ids = mock.MagicMock()
            model.net.return_value = ([0], [1], 2, [])
            session = mock.Mock()
            session.preview.return_value = {'token': 'test', 'post_state': expected}
            session.status.return_value = {'state_hash': expected, 'head': 1}
            session.event.return_value = {'depth': 1}
            if mismatch == 'state_hash':
                session.status.return_value['state_hash'] = 'b' * 64
            elif mismatch == 'post_state':
                session.preview.return_value['post_state'] = 'b' * 64

            def open_session(model, directory):
                if mismatch == 'extra_file':
                    connection = sqlite3.connect(directory / 'session.sqlite3')
                    connection.execute('PRAGMA journal_mode=WAL')
                    connection.execute('CREATE TABLE evidence(value)')
                    connection.execute("INSERT INTO evidence VALUES ('kept')")
                    connection.commit()
                    session.close.side_effect = connection.close
                    (directory / 'unexpected').write_bytes(b'evidence')
                else:
                    (directory / 'session.sqlite3').write_bytes(b'evidence')
                (directory / 'engine.lock').write_bytes(b'lock')
                return session

            modules = {'core': SimpleNamespace(Model=lambda: model, state_hash=lambda labels: expected),
                       'session': SimpleNamespace(Session=open_session)}
            with mock.patch.dict(sys.modules, modules):
                with self.assertRaisesRegex(fixture.FixtureFailure, reason):
                    fixture.build_fixture('basic', output)
            session.close.assert_called_once_with()
            self.assertFalse((output / 'fixture.json').exists())
            if mismatch == 'extra_file':
                self.assertEqual(sorted(p.name for p in (output / 'template').iterdir()),
                                 ['session.sqlite3', 'unexpected'])
                connection = sqlite3.connect(output / 'template/session.sqlite3')
                try:
                    self.assertEqual(connection.execute('PRAGMA journal_mode').fetchone()[0], 'delete')
                    self.assertEqual(connection.execute('SELECT value FROM evidence').fetchone()[0], 'kept')
                finally:
                    connection.close()
            else:
                self.assertEqual((output / 'template/session.sqlite3').read_bytes(), b'evidence')
                self.assertTrue((output / 'template/engine.lock').exists())

    def test_failed_operations_exit_one_without_private_errors(self):
        for error, reason in ((fixture.FixtureFailure('state-mismatch'), 'state-mismatch'),
                              (fixture.FixtureFailure('template-not-clean'), 'template-not-clean'),
                              (ImportError('missing engine'), 'engine-unavailable'),
                              (OSError('cannot read C:\\private\\database'), 'operation-failed'),
                              (ValueError('/private/engine'), 'operation-failed')):
            stdout, stderr = io.StringIO(), io.StringIO()
            with mock.patch.object(fixture, 'build_fixture', side_effect=error), redirect_stdout(stdout), redirect_stderr(stderr):
                code = fixture.main(['build', '--profile', 'basic', '--output', 'unused'])
            self.assertEqual(code, 1)
            self.assertEqual(stdout.getvalue(), '')
            self.assertEqual(stderr.getvalue(), 'b412_fixture: ' + reason + '\n')
        record = {'format': 'magic600-b412-fixture-verify-v1', 'ok': False, 'mismatches': ['head']}
        stdout, stderr = io.StringIO(), io.StringIO()
        with mock.patch.object(fixture, 'verify_data', return_value=record), redirect_stdout(stdout), redirect_stderr(stderr):
            code = fixture.main(['verify', '--data', 'unused', '--fixture', 'unused'])
        self.assertEqual(code, 1)
        self.assertEqual(json.loads(stdout.getvalue()), record)
        self.assertEqual(stderr.getvalue(), '')


class EngineTests(FixtureTestCase):
    @classmethod
    def setUpClass(cls):
        sys.path.insert(0, str(ROOT))
        try:
            import numpy
            from core import Model, state_hash
            from session import Session
        except Exception as error:
            reason = 'NumPy or engine modules unavailable (' + type(error).__name__ + ')'
            if os.getenv('C600_REQUIRE_ENGINE') == '1':
                raise AssertionError(reason) from None
            raise unittest.SkipTest(reason)
        cls.model = Model()
        cls.state_hash = staticmethod(state_hash)

    def test_real_build_copy_verify(self):
        source, destination, _, _ = self.model.net([{'kind': 'word', 'moves': [1, 601]}])
        labels = self.model.ids.copy()
        labels[destination] = labels[source]
        expected = self.state_hash(labels)
        for profile, count in (('basic', 0), ('history-3', 3)):
            with self.subTest(profile=profile):
                directory = self.base / profile
                built = self.assert_record(self.cli('build', '--profile', profile, '--output', directory))
                text = (directory / 'fixture.json').read_text(encoding='utf-8')
                record = json.loads(text)
                self.assertEqual(built, record)
                self.assertEqual(text, json.dumps(record, indent=2, sort_keys=True))
                fixture.validate_fixture(record)
                self.assertEqual(record['state_hash'], expected)
                self.assertEqual(record['head'], 1)
                self.assertEqual(record['journal_depth'], 1)
                self.assertEqual(record['checkpoints'], count)
                self.assertEqual(record['checkpoint_names'], ['B412 history %04d' % i for i in range(1, count + 1)])
                self.assertEqual(record['word'], [1, 601])
                self.assertEqual(record['builder_sha256'], sha256(TOOL.read_bytes()))
                self.assertEqual(record['engine_sources'], {path: sha256((ROOT / path).read_bytes())
                    for path in ('core.py', 'session.py', 'session_lock.py', 'assets/manifest.json')})
                template = directory / 'template'
                self.assertEqual([p.name for p in template.iterdir()], ['session.sqlite3'])
                database = template / 'session.sqlite3'
                self.assertEqual(record['database_sha256'], sha256(database.read_bytes()))
                self.assertEqual(record['database_bytes'], database.stat().st_size)
                connection = sqlite3.connect(database)
                try:
                    self.assertEqual(connection.execute('PRAGMA journal_mode').fetchone()[0], 'delete')
                finally:
                    connection.close()
                data = self.base / (profile + '-data')
                copied = self.assert_record(self.cli('copy', '--fixture', directory, '--data', data))
                self.assertEqual(copied, {'format': 'magic600-b412-fixture-copy-v1', 'profile': profile,
                                         'database_sha256': record['database_sha256'], 'state_hash': expected})
                self.assertEqual([p.name for p in data.iterdir()], ['session.sqlite3'])
                verified = self.assert_record(self.cli('verify', '--data', data, '--fixture', directory))
                self.assertEqual(verified, {'format': 'magic600-b412-fixture-verify-v1', 'ok': True, 'mismatches': []})
                # Verify must close its Session so the API can immediately reopen it.
                self.assertEqual(fixture.verify_data(data, directory), verified)
                if count:
                    record['state_hash'] = '0' * 64
                    record['head'] = 2
                    self.write_record(directory, record)
                    connection = sqlite3.connect(data / 'session.sqlite3')
                    try:
                        with connection:
                            connection.execute("DELETE FROM snapshots WHERE name='B412 history 0001'")
                    finally:
                        connection.close()
                    result = self.cli('verify', '--data', data, '--fixture', directory)
                    self.assertEqual(result.returncode, 1, result.stderr)
                    self.assertEqual(result.stderr, '')
                    self.assertEqual(json.loads(result.stdout), {'format': 'magic600-b412-fixture-verify-v1',
                        'ok': False, 'mismatches': ['state_hash', 'head', 'checkpoint_names']})
                self.assertEqual(sha256(database.read_bytes()), built['database_sha256'])


if __name__ == '__main__':
    unittest.main()
