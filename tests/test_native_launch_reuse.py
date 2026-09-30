"""Rejected native build records must never authorize executable reuse."""
from pathlib import Path
import json
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'work/experiments/magic600-04'))
import native_launch as nl


class NativeLaunchReuseTests(unittest.TestCase):
    def test_rejected_build_recompiles_after_sources_are_restored(self):
        with tempfile.TemporaryDirectory(ignore_cleanup_errors=True) as temporary:
            root = Path(temporary)
            here = root / 'work/experiments/magic600-04'
            sources = [root / 'native' / name for name in nl.RETAINED]
            sources += [here / 'native' / name for name in nl.EXPERIMENT_SOURCES]
            harness = tuple(root / path.relative_to(nl.ROOT) for path in nl.HARNESS_SOURCES)
            inputs = sources + [here / name for name in nl.BACKEND_SOURCES]
            inputs += [root / name for name in nl.SHARED_BACKEND_SOURCES] + list(harness)
            for path in inputs:
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(b'fixture input\n')
            retained = sources[0]
            original = retained.read_bytes()
            compile_calls = []

            def capture(sources, compiler):
                return dict(sources=nl.hash_files(inputs), checks={})

            def compile_fixture(compiler, host, sources, main_type, log):
                compile_calls.append(host)
                if len(compile_calls) == 1:
                    retained.write_bytes(original + b'changed during compilation\n')
                    host.write_bytes(b'A')
                else:
                    host.write_bytes(b'B')
                host.with_suffix('.exe.config').write_bytes(
                    (root / 'native/NativeHost.exe.config').read_bytes())

            with (patch.multiple(nl, ROOT=root, HERE=here, HARNESS_SOURCES=harness),
                  patch.object(nl.hash_files, '__defaults__', (root,)),
                  patch.object(nl.check_evidence, '__defaults__', (root,)),
                  patch.object(nl, 'capture_evidence', side_effect=capture),
                  patch.object(nl, 'compile_program', side_effect=compile_fixture) as compiler):
                directory = root / 'build'
                with self.assertRaisesRegex(RuntimeError, 'Native sources changed during compilation'):
                    nl.build(directory)
                record = next(directory.glob('*/build.json'))
                rejected = json.loads(record.read_text(encoding='utf-8'))
                self.assertEqual(rejected['checks']['after_build']['status'], 'changed')
                self.assertIn(str(retained.relative_to(root)),
                    rejected['checks']['after_build']['changed'])
                self.assertEqual((record.parent / 'Magic600Experiment.exe').read_bytes(), b'A')

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
