"""Rejected native build records must never authorize executable reuse."""
from pathlib import Path
import json
import sys
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parent))
from test_build_identity import Fixture, bid  # noqa: E402
import native_launch as nl  # noqa: E402


class NativeLaunchReuseTests(unittest.TestCase):
    def test_rejected_build_recompiles_after_sources_are_restored(self):
        f = Fixture(self)
        root = f.root
        (root / 'adapter.py').write_bytes(b'print("adapter")\n')
        retained = root / 'native/Host.cs'
        original = retained.read_bytes()
        compile_calls = []

        def compile_fixture(compiler, host, sources, main_type, log):
            compile_calls.append(host)
            if len(compile_calls) == 1:
                retained.write_bytes(original + b'changed during compilation\n')
                host.write_bytes(b'A')
            else:
                host.write_bytes(b'B')
            host.with_suffix('.exe.config').write_bytes((root / 'native/NativeHost.exe.config').read_bytes())

        defaults = (root,) + nl.check_evidence.__defaults__[1:]
        with (patch.multiple(nl, ROOT=root, HERE=root, COMPILER=f.csc, BACKEND_SOURCES=(),
                             SHARED_BACKEND_SOURCES=('engine.py', 'adapter.py'),
                             PRODUCT_CONFIG=root / 'native/NativeHost.exe.config',
                             HARNESS_SOURCES=(root / 'tests/run_postapproval.py',),
                             native_sources=lambda: [retained], compiler_banner=lambda compiler: 'csc 4.8'),
              patch.object(bid, 'current_interpreter', f.interpreter),
              patch.object(nl.check_evidence, '__defaults__', defaults),
              patch.object(nl, 'compile_program', side_effect=compile_fixture) as compiler):
            directory = root / 'native-build'
            with self.assertRaisesRegex(RuntimeError, 'Native sources changed during compilation'):
                nl.build(directory)
            record = next(directory.glob('*/build.json'))
            rejected = json.loads(record.read_text(encoding='utf-8'))
            self.assertEqual(rejected['checks']['after_build']['status'], 'changed')
            self.assertIn('native/Host.cs', rejected['checks']['after_build']['changed'])
            self.assertEqual((record.parent / 'Magic600Experiment.exe').read_bytes(), b'A')

            # The restored sources have the rejected build's identity, and its executable still
            # matches the rejected receipt; only the after_build check prevents reuse.
            retained.write_bytes(original)
            host = nl.build(directory)
            self.assertEqual(host.parent, record.parent)
            self.assertEqual(compiler.call_count, 2)
            self.assertEqual(host.read_bytes(), b'B')
            accepted = json.loads(record.read_text(encoding='utf-8'))
            self.assertEqual(accepted['checks']['after_build']['status'], 'unchanged')

            self.assertEqual(nl.build(directory), host)
            self.assertEqual(compiler.call_count, 2)
            self.assertEqual(host.read_bytes(), b'B')


if __name__ == '__main__':
    unittest.main()
