from pathlib import Path
import json
import sqlite3
import sys
import threading
import tempfile
import unittest
from unittest.mock import patch

import assemble
import check_package
from package_contract import sha, safe_target, verify_package, write_json
from assemble import allowed_sources, check_local_imports, constants, E, ROOT


class PackageContractTests(unittest.TestCase):
    def test_collected_encoding_and_typing_dependency_notices_are_complete(self):
        with tempfile.TemporaryDirectory() as directory:
            bundle = Path(directory)
            with patch.object(sys, 'path', [str(E / 'packaging/toolchain')] + sys.path):
                assemble.copy_licenses(bundle)
            for name, label in (('charset-normalizer', 'Charset-Normalizer'), ('typing_extensions', 'Typing-Extensions')):
                distribution = assemble.importlib.metadata.distribution(name)
                source = next(distribution.locate_file(p) for p in distribution.files
                              if p.name == 'LICENSE' and 'licenses' in p.parts)
                self.assertEqual(sha(bundle / 'licenses' / label / 'LICENSE'), sha(source))

    def test_manifest_rejects_damage_extra_files_and_duplicate_paths(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            payload = root / 'app.py'
            payload.write_text('exact source\n', encoding='utf-8')
            row = dict(path='app.py', sha256=sha(payload), bytes=payload.stat().st_size)
            manifest = dict(version='0.4', layout_version=1, files=[row])
            path = root / '_internal/package-manifest.json'
            write_json(path, manifest)
            self.assertEqual(verify_package(root), manifest)
            payload.write_text('different source\n', encoding='utf-8')
            with self.assertRaisesRegex(ValueError, 'missing or damaged'):
                verify_package(root)
            payload.write_text('exact source\n', encoding='utf-8')
            extra = root / 'session.sqlite3'
            extra.write_bytes(b'not package data')
            with self.assertRaisesRegex(ValueError, 'Unexpected files'):
                verify_package(root)
            extra.unlink()
            manifest['files'].append(dict(row))
            write_json(path, manifest)
            with self.assertRaisesRegex(ValueError, 'Duplicate'):
                verify_package(root)

    def test_paths_cannot_escape_package(self):
        with tempfile.TemporaryDirectory() as directory:
            for name in ('../personal', '/outside', 'C:/outside', 'app/../../outside', 'app\\secret'):
                with self.assertRaises(ValueError):
                    safe_target(Path(directory), name)

    def test_actual_dynamic_source_and_proof_closure_is_explicit(self):
        contract, app, native, manifest = allowed_sources()
        names = {p.as_posix() for p in app}
        prefix = 'work/experiments/magic600-04/'
        self.assertTrue({'core.py', 'server.py', 'session.py', 'native/directx_runtime.py'}.issubset(names))
        for name in contract['BACKEND_SOURCES']:
            self.assertIn(prefix + name, names)
        for name in ('engine.py', 'adapter.py', 'orbit_invariants.py', 'transported_frames.py',
                     'endgame_library.py', 'endgame_invariants.py', 'reference_variants.py',
                     'evidence/orbit-invariants-20260916-generators.json'):
            self.assertIn(prefix + name, names)
        self.assertTrue(set(manifest['files']).issubset(names))
        self.assertFalse(any('sessions' in p.parts or p.suffix in ('.log', '.sqlite3') for p in app))
        expected = constants(E / 'endgame_invariants.py', ('AUDIT_SHA256',))['AUDIT_SHA256']
        self.assertEqual(sha(E / 'evidence/orbit-invariants-20260916-generators.json'), expected)
        self.assertTrue(all((ROOT / name).is_file() for name in native))

    def test_omitted_local_module_fails_before_freezing(self):
        _, app, _, _ = allowed_sources()
        with self.assertRaisesRegex(ValueError, 'Local import missing'):
            check_local_imports([p for p in app if p != Path('core.py')])


class AssemblyBindingTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='magic600-package-binding-')
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        root_patch = patch.object(assemble, 'ROOT', self.root)
        root_patch.start()
        self.addCleanup(root_patch.stop)
        self.app = [Path('core.py'), assemble.EXPERIMENT / 'adapter.py']
        self.native = [Path('native/NativeHost.cs')]
        self.config = Path('native/NativeHost.exe.config')
        self.contract = dict(BACKEND_SOURCES=('adapter.py',), SHARED_BACKEND_SOURCES=('core.py',))
        for relative in self.app + self.native + [self.config]:
            path = self.root / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(('Original ' + relative.as_posix()).encode())
        self.host = self.root / 'build/Magic600Experiment.exe'
        self.host.parent.mkdir()
        self.host.write_bytes(b'Fixture host, never executable')
        assemble.copy_file(self.root / self.config, self.host.with_suffix('.exe.config'))
        # A structurally complete v1 receipt: consumers refuse receipts without a valid evidence_version.
        self.receipt = dict(evidence_version=1, sources={}, model={}, toolchain={}, build_identity='fixture',
            executable_sha256=sha(self.host),
            checks=dict(after_build=dict(status='unchanged')),
            source={str(p): sha(self.root / p) for p in self.native},
            backend_sources={str(p): sha(self.root / p) for p in self.app})
        write_json(self.host.parent / 'build.json', self.receipt)
        self.host, self.receipt = assemble.checked_native(self.host.parent, self.native, self.contract)
        self.stage = self.root / 'stage'

    def capture_and_stage(self):
        self.hashes = {p.as_posix(): sha(self.root / p) for p in self.app + self.native + [self.config]}
        for relative in self.app:
            assemble.copy_file(self.root / relative, self.stage / 'app' / relative)
        assemble.copy_file(self.host, self.stage / 'native/Magic600Native.exe')
        assemble.copy_file(self.host.with_suffix('.exe.config'), self.stage / 'native/Magic600Native.exe.config')

    def validate(self):
        return assemble.validate_assembly_inputs(self.stage, self.hashes, self.app, self.native,
                                                 self.contract, self.host, self.receipt)

    def test_unchanged_source_receipt_and_staged_bytes_are_accepted(self):
        self.capture_and_stage()
        self.validate()

    def test_edit_between_native_check_and_source_capture_is_rejected(self):
        for relative in self.app + self.native:
            with self.subTest(source=relative.as_posix()):
                path = self.root / relative
                original = path.read_bytes()
                try:
                    path.write_bytes(original + b' changed before capture')
                    self.capture_and_stage()
                    # Both former final checks pass under this exact interleaving.
                    self.assertTrue(all(sha(self.root / p) == h for p, h in self.hashes.items()))
                    self.assertEqual(sha(self.host), self.receipt['executable_sha256'])
                    with self.assertRaisesRegex(ValueError, 'current backend|current retained'):
                        self.validate()
                finally:
                    path.write_bytes(original)

    def test_altered_staged_source_host_and_config_are_rejected(self):
        self.capture_and_stage()
        paths = [self.stage / 'app' / self.app[0], self.stage / 'native/Magic600Native.exe',
                 self.stage / 'native/Magic600Native.exe.config']
        for path in paths:
            with self.subTest(staged=path.relative_to(self.stage).as_posix()):
                original = path.read_bytes()
                try:
                    path.write_bytes(original + b' changed staged bytes')
                    with self.assertRaisesRegex(ValueError, 'Staged file'):
                        self.validate()
                finally:
                    path.write_bytes(original)

    def test_changed_native_receipt_is_not_silently_adopted(self):
        self.capture_and_stage()
        write_json(self.host.parent / 'build.json', dict(self.receipt, build_identity='changed receipt'))
        with self.assertRaisesRegex(ValueError, 'build receipt changed'):
            self.validate()


class AcceptanceExtensionTests(unittest.TestCase):
    def test_verification_output_cannot_pollute_package_or_existing_data(self):
        self.assertTrue(callable(getattr(check_package, 'create_check_output', None)), 'Missing external fresh output guard')
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); bundle = root / 'bundle'; bundle.mkdir()
            with self.assertRaises(ValueError):
                check_package.create_check_output(bundle, bundle / 'new-output')
            self.assertFalse((bundle / 'new-output').exists())
            existing = root / 'existing'; existing.mkdir()
            with self.assertRaises(ValueError):
                check_package.create_check_output(bundle, existing)
            fresh = root / 'new-output'
            self.assertEqual(check_package.create_check_output(bundle, fresh), fresh.resolve())

    def test_copy_is_bounded_new_and_byte_identical(self):
        self.assertTrue(callable(getattr(check_package, 'copy_generated_profile', None)), 'Missing bounded generated-profile copy')
        with tempfile.TemporaryDirectory() as directory:
            owner = Path(directory)
            source = owner / 'generated-original'; source.mkdir()
            (source / 'session.sqlite3').write_bytes(b'closed fixture bytes')
            (source / '.package-fixture.json').write_text('{}')
            destination = owner / 'copied-profile'
            before = check_package.copy_generated_profile(source, destination, owner)
            self.assertEqual(before, check_package.profile_hashes(destination))
            self.assertEqual(before, check_package.profile_hashes(source))
            with self.assertRaises(ValueError):
                check_package.copy_generated_profile(source, destination, owner)
            with self.assertRaises(ValueError):
                check_package.copy_generated_profile(source, owner.parent / 'outside-copy', owner)
            (source / '.package-fixture.json').unlink()
            with self.assertRaises(ValueError):
                check_package.copy_generated_profile(source, owner / 'another-copy', owner)

    def test_changed_original_is_not_reported_as_rollback_preserved(self):
        self.assertTrue(callable(getattr(check_package, 'assert_original_unchanged', None)), 'Missing original-profile preservation check')
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); (root / 'saved.log').write_bytes(b'original')
            before = check_package.profile_hashes(root)
            (root / 'saved.log').write_bytes(b'changed')
            with self.assertRaisesRegex(ValueError, 'original'):
                check_package.assert_original_unchanged(root, before)

    def test_geometry_witness_requires_bytes_and_packaged_runtime_binding(self):
        self.assertTrue(callable(getattr(check_package, 'load_native_export', None)), 'Missing explicit native witness validation')
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'actual-geometry.json'
            path.write_text(json.dumps(dict(format='MPUlt-native600-v1', n=259800, executable_sha256='runtime')))
            with self.assertRaisesRegex(ValueError, 'hash'):
                check_package.load_native_export(path, 'wrong', 'runtime')
            with self.assertRaisesRegex(ValueError, 'runtime'):
                check_package.load_native_export(path, sha(path), 'other-runtime')
            self.assertEqual(check_package.load_native_export(path, sha(path), 'runtime')['n'], 259800)

    def test_legacy_checkpoints_and_keyfile_must_survive_copy_use(self):
        self.assertTrue(callable(getattr(check_package, 'retained_profile_records', None)), 'Missing retained checkpoint/keyfile evidence')
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            with sqlite3.connect(root / 'session.sqlite3') as db:
                db.execute('PRAGMA journal_mode=WAL')
                db.execute('CREATE TABLE snapshots(name TEXT,head INTEGER,labels BLOB,hash TEXT,prefs TEXT,created REAL)')
                db.execute('INSERT INTO snapshots VALUES (?,?,?,?,?,?)', ('Old checkpoint', 7, b'labels', 'state', '{"bank":"old"}', 1.0))
            db.close()
            keys = root / 'native_keys.json'; keys.write_bytes(b'{"F9":"checkpoint"}')
            original_files = check_package.profile_hashes(root)
            before = check_package.retained_profile_records(root)
            check_package.assert_retained_profile_records(root, before)
            self.assertEqual(check_package.profile_hashes(root), original_files)
            keys.write_bytes(b'{}')
            with self.assertRaisesRegex(ValueError, 'key file'):
                check_package.assert_retained_profile_records(root, before)
            keys.write_bytes(b'{"F9":"checkpoint"}')
            with sqlite3.connect(root / 'session.sqlite3') as db:
                db.execute("UPDATE snapshots SET labels=? WHERE name='Old checkpoint'", (b'changed',))
            db.close()
            with self.assertRaisesRegex(ValueError, 'checkpoint'):
                check_package.assert_retained_profile_records(root, before)
            with sqlite3.connect(root / 'session.sqlite3') as db:
                db.execute('DELETE FROM snapshots')
            db.close()
            with self.assertRaisesRegex(ValueError, 'checkpoint'):
                check_package.assert_retained_profile_records(root, before)

    def test_profile_record_inspection_refuses_uncheckpointed_wal(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            db = sqlite3.connect(root / 'session.sqlite3')
            try:
                db.execute('PRAGMA journal_mode=WAL')
                db.execute('CREATE TABLE snapshots(name TEXT,head INTEGER,labels BLOB,hash TEXT,prefs TEXT,created REAL)')
                db.execute('INSERT INTO snapshots VALUES (?,?,?,?,?,?)', ('Pending checkpoint', 7, b'labels', 'state', '{}', 1.0))
                db.commit()
                (root / 'native_keys.json').write_bytes(b'{}')
                self.assertGreater((root / 'session.sqlite3-wal').stat().st_size, 0)
                before = check_package.profile_hashes(root)
                with self.assertRaisesRegex(ValueError, 'checkpointed'):
                    check_package.retained_profile_records(root)
                self.assertEqual(check_package.profile_hashes(root), before)
            finally:
                db.close()

    def test_original_drift_writes_failed_nested_receipt_before_raising(self):
        self.assertTrue(callable(getattr(check_package, 'finish_copy_report', None)), 'Missing truthful copied-profile receipt boundary')
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); source = root / 'original'; source.mkdir()
            (source / 'session.sqlite3').write_bytes(b'old')
            before = check_package.profile_hashes(source)
            (source / 'session.sqlite3').write_bytes(b'changed')
            report = {'passed': True}; path = root / 'receipt.json'
            with self.assertRaisesRegex(ValueError, 'original'):
                check_package.finish_copy_report(source, before, report, path)
            saved = json.loads(path.read_text())
            self.assertIs(saved['passed'], False)
            self.assertIs(saved['original_unchanged'], False)

    def test_launcher_transaction_uses_only_authorized_experiment_routes(self):
        source = (Path(__file__).parent / 'launcher.py').read_text()
        transaction = source[source.index('def engine_test('):source.index('\ndef main(')]
        for route in ('/api/preview', '/api/commit', '/api/undo'):
            self.assertNotIn("'" + route + "'", transaction)
        for action in ('draft', 'review', 'preview', 'commit', 'undo'):
            self.assertIn("action='" + action + "'", transaction)

    def test_file_route_scenario_against_real_workbench(self):
        self.assertTrue(callable(getattr(check_package, 'exercise_session_files', None)), 'Missing frozen route acceptance scenario')
        sys.path[:0] = [str(E), str(ROOT)]
        from adapter import Workbench
        from core import Model
        from enhanced import Workflow
        from session import Session
        model = Model()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            session = Session(model, root / 'session')
            try:
                pending = session.preview([dict(kind='word', moves=[1])]); session.commit(pending['token'])
                profile = json.loads((E / 'native-baseline/postapproval-g2-20260917-002905/session/native_profile.json').read_text())
                work = Workbench(session, threading.RLock(), workflow=Workflow(session), native_profile=lambda: profile)
                def read(path, raw=False):
                    if path == '/api/labels': return session.st.labels.astype('<u4').tobytes()
                    if path == '/api/status': return session.status()
                    if path == '/api/experiment/snapshot': return work.snapshot()
                    raise AssertionError(path)
                def command(body, route='/api/experiment/command'):
                    self.assertIn(route, ('/api/experiment/command', '/api/experiment/session-log'))
                    return work.command(body, response_snapshot=False)
                expected = read('/api/labels', True)
                result = check_package.exercise_session_files(read, command, root / 'files', expected, mpult=True)
                self.assertTrue(result['passed'])
                self.assertEqual(result['formats'], ['c600', 'mpult'])
                self.assertEqual(read('/api/labels', True), expected)
                self.assertEqual(work.w['keybinds'], result['keybindings'])
                self.assertIsNone(work.session_workflow.report()['completion'])
            finally:
                session.close()


if __name__ == '__main__':
    unittest.main(verbosity=2)
