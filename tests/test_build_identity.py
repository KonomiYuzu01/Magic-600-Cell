"""Build identity v2 (0.4.1 step 1): headless tests over fixture trees; no compiler or GUI runs."""
from __future__ import annotations

import base64
import copy
import hashlib
import json
import os
import shutil
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EXPERIMENT = ROOT / 'work/experiments/magic600-04'
sys.path[:0] = [str(ROOT), str(EXPERIMENT), str(EXPERIMENT / 'packaging')]
import build_identity as bid  # noqa: E402
from native import bootstrap  # noqa: E402
import native_launch  # noqa: E402
import assemble  # noqa: E402


def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def record_hash(data: bytes) -> str:
    return 'sha256=' + base64.urlsafe_b64encode(hashlib.sha256(data).digest()).rstrip(b'=').decode()


class Fixture:
    """A miniature repository with product, model, runtime, harness and tool files."""

    def __init__(self, test: unittest.TestCase, name='repo'):
        tmp = tempfile.TemporaryDirectory()
        test.addCleanup(tmp.cleanup)
        self.base = Path(tmp.name)
        self.root = self.base / name
        files = {
            'native/Host.cs': b'class Host {}\r\n', 'native/NativeHost.exe.config': b'<configuration/>\n',
            'engine.py': b'print("engine")\n', 'native/runtime/MPUlt.exe': b'MZ\0\0',
            'native/runtime/MPUlt_puzzles.txt': b'puzzles\r\n', 'native/runtime/MPUlt_settings.txt': b'defaults\r\n',
            'native/directx_runtime.py': b'def find_directx(): pass\n', 'assets/seed.json': b'[1]\n',
            'tests/run_postapproval.py': b'print("harness")\n',
        }
        files['assets/manifest.json'] = json.dumps(
            {'model_id': 'm1', 'files': {'assets/seed.json': sha(b'[1]\n')}}).encode()
        for rel, data in files.items():
            (self.root / rel).parent.mkdir(parents=True, exist_ok=True)
            (self.root / rel).write_bytes(data)
        self.sources = [self.root / 'native/Host.cs', self.root / 'engine.py', self.root / 'native/NativeHost.exe.config']
        # A compiler directory with every role file and reference assembly.
        self.framework = self.base / 'Framework'
        for name in ['csc.exe', 'csc.exe.config', 'alink.dll', '1033/cscui.dll', '1033/alinkui.dll',
                     'default.win32manifest'] + bootstrap.system_references():
            (self.framework / name).parent.mkdir(parents=True, exist_ok=True)
            (self.framework / name).write_bytes(name.encode())
        self.csc = self.framework / 'csc.exe'
        # An interpreter: base binaries, a launcher and an installed NumPy.
        self.base_prefix = self.base / 'Python314'
        for name in ('python.exe', 'python3.dll', 'python314.dll'):
            (self.base_prefix / name).parent.mkdir(parents=True, exist_ok=True)
            (self.base_prefix / name).write_bytes(name.encode())
        self.site = self.install_numpy(self.base / 'venv-a' / 'Lib' / 'site-packages')
        self.launcher = self.base / 'venv-a' / 'Scripts' / 'python.exe'
        self.launcher.parent.mkdir(parents=True, exist_ok=True)
        self.launcher.write_bytes(b'launcher')

    @staticmethod
    def install_numpy(site: Path) -> Path:
        payload = {'numpy/__init__.py': b'__version__ = "2.3.5"\n', 'numpy/_core/_multiarray_umath.pyd': b'\0binary',
                   'numpy.libs/openblas.dll': b'\0blas', 'numpy-2.3.5.dist-info/METADATA': b'Name: numpy\n',
                   'numpy-2.3.5.dist-info/WHEEL': b'Wheel-Version: 1.0\n', 'numpy-2.3.5.dist-info/INSTALLER': b'uv\n',
                   'numpy-2.3.5.dist-info/REQUESTED': b''}
        rows = []
        for rel, data in payload.items():
            (site / rel).parent.mkdir(parents=True, exist_ok=True)
            (site / rel).write_bytes(data)
            rows.append(f'{rel},{record_hash(data)},{len(data)}')
        wrapper = f'launcher for {site}'.encode()  # console scripts embed their install path
        (site.parent.parent / 'Scripts').mkdir(parents=True, exist_ok=True)
        (site.parent.parent / 'Scripts' / 'f2py.exe').write_bytes(wrapper)
        rows.append(f'../../Scripts/f2py.exe,{record_hash(wrapper)},{len(wrapper)}')
        rows.append('numpy-2.3.5.dist-info/RECORD,,')
        (site / 'numpy-2.3.5.dist-info/RECORD').write_text('\n'.join(rows) + '\n', encoding='utf-8')
        return site

    def interpreter(self, version='3.14.7', site=None, launcher=None):
        payload = bid.numpy_payload(site or self.site, 'numpy-2.3.5.dist-info')
        return bid.interpreter_binding('cpython', version, 64, self.base_prefix, launcher or self.launcher, '2.3.5', payload)

    def identity(self, main_type='ExperimentProgram', **changes):
        recipe = bootstrap.compile_recipe(main_type)
        payload = bid.identity_payload(product=bid.product_binding(self.sources, self.root), recipe=recipe,
                                       compiler=bid.compiler_binding(self.csc, recipe, 'csc 4.8'),
                                       interpreter=self.interpreter())
        payload.update(changes)
        return payload

    def receipt(self):
        identity = self.identity()
        receipt = dict(evidence_version=2, identity=identity, build_identity=bid.digest(identity),
                       harness=bid.hash_inputs([self.root / 'tests/run_postapproval.py'], self.root),
                       environment=dict(platform='Windows-11'), checks={})
        host = self.root / 'build/Magic600Experiment.exe'
        host.parent.mkdir(parents=True, exist_ok=True)
        host.write_bytes(b'MZ host')
        shutil.copy2(self.root / 'native/NativeHost.exe.config', host.with_suffix('.exe.config'))
        receipt.update(bid.legacy_aliases(identity['product'], self.root, [self.root / 'native/Host.cs'],
                                          [self.root / 'engine.py']),
                       artifacts=bid.hash_inputs([host, host.with_suffix('.exe.config')], self.root),
                       executable_sha256=sha(host.read_bytes()), backend_sha256=sha(b'print("engine")\n'))
        return receipt, host

    def tools(self, **overrides):
        identity = self.identity()
        now = {'compiler': copy.deepcopy(identity['compiler']), 'interpreter': copy.deepcopy(identity['interpreter'])}
        now.update(overrides)
        return lambda recorded: now


class CompileRecipeTests(unittest.TestCase):
    def test_the_invocation_is_sealed_and_uses_only_the_compiler_directory(self):
        f = Fixture(self)
        cmd = bootstrap.compile_command(f.csc, 'out.exe', ['a.cs'], 'ExperimentProgram')
        self.assertEqual(cmd[1:4], ['/nologo', '/noconfig', '/nostdlib+'])
        refs = [c[len('/reference:'):] for c in cmd if c.startswith('/reference:')]
        self.assertEqual(len(refs), len(bootstrap.system_references()))
        self.assertTrue(all(Path(r).parent == f.framework.resolve() for r in refs))
        self.assertIn(str(f.framework.resolve() / 'mscorlib.dll'), refs, 'the implicit standard library is explicit')
        self.assertIn('/win32manifest:' + str(f.framework.resolve() / 'default.win32manifest'), cmd)
        self.assertNotIn('csc.rsp', ' '.join(cmd))

    def test_the_recipe_holds_no_paths_and_ignores_harness_text(self):
        recipe = bootstrap.compile_recipe('ExperimentProgram')
        self.assertNotIn(os.sep + 'Windows', json.dumps(recipe))
        self.assertEqual(recipe, bootstrap.compile_recipe('ExperimentProgram'))
        self.assertEqual(recipe['compiler'], 'csc')
        # Harness-only text in native/bootstrap.py (help, fixtures) is not part of the recipe.
        self.assertNotIn('--self-test-only', json.dumps(recipe))

    def test_extra_references_keep_their_paths_and_are_checked(self):
        f = Fixture(self)
        dx = f.base / 'runtime'
        dx.mkdir()
        names = ('Microsoft.DirectX.dll', 'Microsoft.DirectX.Direct3D.dll', 'Microsoft.DirectX.Direct3DX.dll')
        for name in names:
            (dx / name).write_bytes(name.encode())
        cmd = bootstrap.compile_command(f.csc, 'out.exe', ['a.cs'], 'NativeRendererRegression', [dx / n for n in names])
        for name in names:
            self.assertIn('/reference:' + str((dx / name).resolve()), cmd)
        with self.assertRaises(RuntimeError):
            bootstrap.compile_command(f.csc, 'out.exe', ['a.cs'], 'X', [dx / 'absent.dll'])
        (dx / 'System.dll').write_bytes(b'shadow')
        with self.assertRaises(RuntimeError):
            bootstrap.compile_command(f.csc, 'out.exe', ['a.cs'], 'X', [dx / 'System.dll'])

    def test_ambient_and_missing_references_are_refused(self):
        f = Fixture(self)
        saved = os.environ.get('LIB')
        os.environ['LIB'] = str(f.base)
        try:
            with self.assertRaises(RuntimeError):
                bootstrap.compile_command(f.csc, 'out.exe', ['a.cs'], 'X')
        finally:
            if saved is None:
                os.environ.pop('LIB', None)
            else:
                os.environ['LIB'] = saved
        (f.framework / 'System.Xml.dll').unlink()
        with self.assertRaises(RuntimeError):
            bootstrap.compile_command(f.csc, 'out.exe', ['a.cs'], 'X')


class IdentityTests(unittest.TestCase):
    def test_the_same_inputs_give_the_same_identity_anywhere(self):
        a, b = Fixture(self, 'first'), Fixture(self, 'second/deeper')
        self.assertEqual(bid.digest(a.identity()['product']), bid.digest(b.identity()['product']))
        moved = a.install_numpy(a.base / 'elsewhere' / 'Lib' / 'site-packages')
        self.assertEqual(a.interpreter(site=moved)['numpy'], a.interpreter()['numpy'])
        (a.site / 'numpy' / '__pycache__').mkdir()
        (a.site / 'numpy' / '__pycache__' / '__init__.cpython-314.pyc').write_bytes(b'bytecode with a path')
        self.assertEqual(a.interpreter()['numpy'], a.interpreter(site=moved)['numpy'], 'bytecode is never bound')

    def test_harness_and_settings_edits_do_not_change_the_identity(self):
        f = Fixture(self)
        before = bid.digest(f.identity())
        (f.root / 'tests/run_postapproval.py').write_bytes(b'print("edited harness")\n')
        self.assertEqual(bid.digest(f.identity()), before)
        self.assertFalse({p.name for p in native_launch.HARNESS_SOURCES}
                         & {p.name for p in native_launch.product_sources([])})

    def test_every_product_input_changes_the_identity_or_is_refused(self):
        for rel in ('native/Host.cs', 'engine.py', 'native/NativeHost.exe.config', 'native/runtime/MPUlt.exe',
                    'native/runtime/MPUlt_puzzles.txt', 'native/runtime/MPUlt_settings.txt',
                    'native/directx_runtime.py', 'assets/manifest.json'):
            with self.subTest(rel=rel):
                f = Fixture(self)
                before = bid.digest(f.identity())
                if rel == 'assets/manifest.json':
                    data = json.loads((f.root / rel).read_text())
                    data['model_id'] = 'm2'
                    (f.root / rel).write_text(json.dumps(data))
                else:
                    (f.root / rel).write_bytes((f.root / rel).read_bytes() + b' ')
                self.assertNotEqual(bid.digest(f.identity()), before)
        f = Fixture(self)
        (f.root / 'assets/seed.json').write_bytes(b'[2]\n')
        with self.assertRaises(ValueError):
            f.identity()

    def test_recipe_compiler_and_interpreter_changes_change_the_identity(self):
        f = Fixture(self)
        before = bid.digest(f.identity())
        self.assertNotEqual(bid.digest(f.identity('ExperimentRegression')), before, 'target flag')
        recipe = bootstrap.compile_recipe('ExperimentProgram')
        recipe['flags'] = [flag for flag in recipe['flags'] if flag != '/optimize+']
        self.assertNotEqual(bid.digest(f.identity(compile_recipe=recipe)), before)
        for name in ['csc.exe', 'alink.dll', 'default.win32manifest', 'mscorlib.dll', 'System.Xml.dll']:
            with self.subTest(compiler_file=name):
                g = Fixture(self)
                reference = bid.digest(g.identity())
                (g.framework / name).write_bytes(b'changed')
                self.assertNotEqual(bid.digest(g.identity()), reference)
        g = Fixture(self)
        (g.framework / 'System.Xml.dll').unlink()
        with self.assertRaises(ValueError):
            g.identity()
        self.assertNotEqual(bid.digest(f.identity(interpreter=f.interpreter('3.14.6'))), before)
        (f.base_prefix / 'python314.dll').write_bytes(b'changed')
        self.assertNotEqual(bid.digest(f.identity()), before)
        g = Fixture(self)
        reference = bid.digest(g.identity())
        (g.site / 'numpy.libs/openblas.dll').write_bytes(b'\0blas')  # same bytes: unchanged
        self.assertEqual(bid.digest(g.identity()), reference)
        (g.site / 'numpy.libs/openblas.dll').write_bytes(b'\0other')
        with self.assertRaises(ValueError):
            g.identity()  # bytes differ from RECORD

    def test_unhashed_payload_entries_are_refused(self):
        f = Fixture(self)
        record = f.site / 'numpy-2.3.5.dist-info/RECORD'
        record.write_text(record.read_text() + 'numpy/extra.py,,\n', encoding='utf-8')
        (f.site / 'numpy/extra.py').write_text('x = 1\n')
        with self.assertRaises(ValueError):
            f.interpreter()


class ReceiptTests(unittest.TestCase):
    V1 = {'evidence_version': 1, 'sources': {}, 'model': {}, 'toolchain': {}, 'build_identity': 'x', 'checks': {},
          'artifacts': {}, 'executable_sha256': 'e', 'source': {}, 'backend_sources': {}, 'backend_sha256': 'b',
          'stage': 's', 'approvals': {}}

    def test_version_dispatch_refuses_inconsistent_receipts(self):
        f = Fixture(self)
        receipt, _ = f.receipt()
        self.assertEqual(bid.receipt_version(receipt), 2)
        self.assertEqual(bid.receipt_version(dict(self.V1)), 1, 'a genuine v1 receipt stays readable')
        bad = {'missing': {k: v for k, v in receipt.items() if k != 'evidence_version'},
               'null': dict(receipt, evidence_version=None), 'string': dict(receipt, evidence_version='2'),
               'bool': dict(receipt, evidence_version=True), 'unsupported': dict(receipt, evidence_version=3),
               'v2 labelled 1': dict(receipt, evidence_version=1),
               'v1 missing sections': {k: v for k, v in self.V1.items() if k != 'toolchain'},
               'v2 missing harness': {k: v for k, v in receipt.items() if k != 'harness'},
               'v2 extra identity key': dict(receipt, identity=dict(receipt['identity'], extra={}))}
        for name, value in bad.items():
            with self.subTest(case=name), self.assertRaises(ValueError):
                bid.receipt_version(value)

    def test_check_evidence_reports_every_changed_or_missing_input(self):
        f = Fixture(self)
        receipt, _ = f.receipt()
        self.assertTrue(native_launch.check_evidence(receipt, 'after_build', f.root, f.tools()))
        self.assertGreater(receipt['checks']['after_build']['checked_files'], 50, 'tool roles are counted')
        cases = {
            'product file': lambda g, r: (g.root / 'native/runtime/MPUlt_settings.txt').write_bytes(b'user edit'),
            'harness file': lambda g, r: (g.root / 'tests/run_postapproval.py').write_bytes(b'edited'),
            'identity': lambda g, r: r['identity']['product']['sources'].update({'native/Host.cs': '0' * 64}),
            'artifact': lambda g, r: (g.root / 'build/Magic600Experiment.exe').write_bytes(b'MZ other'),
        }
        for name, mutate in cases.items():
            with self.subTest(case=name):
                g = Fixture(self)
                r, _ = g.receipt()
                mutate(g, r)
                self.assertFalse(native_launch.check_evidence(r, 'x', g.root, g.tools()))
                self.assertEqual(r['checks']['x']['status'], 'changed')
        g = Fixture(self)
        r, _ = g.receipt()
        tools = g.tools()(None)
        del tools['compiler']['files']['reference:system.xml.dll']
        self.assertFalse(native_launch.check_evidence(r, 'x', g.root, lambda _: tools))
        self.assertIn('compiler:reference:system.xml.dll', r['checks']['x']['missing'])
        self.assertFalse(native_launch.check_evidence(r, 'y', g.root, lambda _: {'compiler': None, 'interpreter': None}))
        self.assertEqual(sorted(r['checks']['y']['missing']), ['compiler', 'interpreter'])
        newer = copy.deepcopy(g.tools()(None))
        newer['interpreter']['version'] = '3.14.8'
        self.assertFalse(native_launch.check_evidence(r, 'z', g.root, lambda _: newer))
        self.assertIn('interpreter:version', r['checks']['z']['changed'])

    def test_launch_updates_keep_the_identity_intact(self):
        f = Fixture(self)
        receipt, _ = f.receipt()
        path = f.base / 'build.json'
        native_launch.write_evidence(path, receipt)
        receipt = json.loads(path.read_text(encoding='utf-8'))
        runtime = f.root / 'session/runtime'
        runtime.mkdir(parents=True)
        (runtime / 'MPUlt_settings.txt').write_bytes(b'defaults\r\n')
        receipt['runtime'] = dict(origins={}, immutable={}, reflection_contract='verified',
                                  mutable_initial={'session/runtime/MPUlt_settings.txt': sha(b'defaults\r\n')})
        for phase in ('before_engine', 'before_start', 'after_run'):
            if phase == 'after_run':
                (runtime / 'MPUlt_settings.txt').write_bytes(b'owner preferences\r\n')  # settings may change
            self.assertTrue(native_launch.check_evidence(receipt, phase, f.root, f.tools()), phase)
            native_launch.write_evidence(path, receipt)
            receipt = json.loads(path.read_text(encoding='utf-8'))
            self.assertTrue(bid.identity_intact(receipt), phase)
        self.assertNotEqual(receipt['runtime']['mutable_after_run'], receipt['runtime']['mutable_initial'])


class PackagingTests(unittest.TestCase):
    def setUp(self):
        self.f = Fixture(self)
        self.receipt, self.host = self.f.receipt()
        self.saved_root = assemble.ROOT
        assemble.ROOT = self.f.root
        self.addCleanup(setattr, assemble, 'ROOT', self.saved_root)
        self.contract = dict(BACKEND_SOURCES=(), SHARED_BACKEND_SOURCES=('engine.py',))
        self.native = [Path('native/Host.cs')]
        self.receipt['checks'] = {'after_build': {'status': 'unchanged'}}

    def check(self, receipt):
        (self.host.parent / 'build.json').write_text(json.dumps(receipt), encoding='utf-8')
        return assemble.checked_native(self.host.parent, self.native, self.contract)

    def test_a_current_v2_receipt_is_accepted(self):
        self.check(self.receipt)

    def test_stale_or_malformed_v2_receipts_are_rejected_despite_after_build(self):
        cases = {
            'runtime input changed': lambda r: (self.f.root / 'native/runtime/MPUlt.exe').write_bytes(b'MZ new'),
            'model changed': lambda r: (self.f.root / 'assets/manifest.json').write_bytes(b'{"model_id": "m2"}'),
            'identity corrupted': lambda r: r.update(build_identity='0' * 64),
            'section missing': lambda r: r.pop('harness'),
            'version removed': lambda r: r.pop('evidence_version'),
            'executable changed': lambda r: self.host.write_bytes(b'MZ other'),
            'config changed': lambda r: self.host.with_suffix('.exe.config').write_bytes(b'<other/>'),
        }
        for name, mutate in cases.items():
            with self.subTest(case=name):
                f = Fixture(self)
                receipt, host = f.receipt()
                receipt['checks'] = {'after_build': {'status': 'unchanged'}}
                self.f, self.host = f, host
                assemble.ROOT = f.root
                mutate(receipt)
                with self.assertRaises((ValueError, KeyError)):
                    self.check(receipt)


if __name__ == '__main__':
    unittest.main()
