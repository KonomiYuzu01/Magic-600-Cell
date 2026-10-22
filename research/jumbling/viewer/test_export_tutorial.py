"""Headless tutorial-export acceptance; uses committed development seeds 7001 and 7106.

    python3 -I -B research/jumbling/viewer/test_export_tutorial.py
"""
import contextlib
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('export_tutorial', HERE / 'export_tutorial.py')
exporter = importlib.util.module_from_spec(spec)
spec.loader.exec_module(exporter)
np, wj = exporter.np, exporter.wj
RUN = HERE.parent / 'restoration/13-tutorial-runner/runs/dev/d0-s7001/run'
DIGESTS = {'before': '1438ebed339490d2875e343063e7e79fd51b6a7621f2bf1a054beb5ef7c6d6b1',
           'after': 'ab254e2af1775b8161092aed06d35252e81e63fa427fd75dc2d7f9aeb5b1c7f9'}


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


class TutorialExport(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.temporary = tempfile.TemporaryDirectory(prefix='magic600-tutorial-test-')
        cls.addClassCleanup(cls.temporary.cleanup)
        cls.root = Path(cls.temporary.name)
        cls.outputs = {}
        for when in ('before', 'after'):
            path = cls.root / when
            exporter.export(RUN, 0, when, path)
            cls.outputs[when] = path
        # Independent single-twist oracle, without the exporter's replay helpers.
        menu = exporter.check_run.restore.menu('I_a')
        before = exporter.check_run.load_table(RUN / 'start-table.json', menu)
        after = exporter.check_run.load_table(RUN / 'start-table.json', menu)
        word = json.loads((RUN / 'words/00000.json').read_text())['word']
        for cap, item in word:
            outcome = after.apply(exporter.check_run.sim.Twist(after.ctx, cap,
                exporter.check_run.restore.element(menu, cap, item), 'menu', {'menu': menu.name}), journal=False)
            if not outcome.applied:
                raise AssertionError('independent oracle twist was not applied')
        cls.states = {'before': before, 'after': after}

    def test_recorded_states_and_array_bytes(self):
        for when, off in (('before', 26519), ('after', 23422)):
            with self.subTest(when=when):
                directory = self.outputs[when]
                header = json.loads((directory / 'header.json').read_text())
                tutorial = json.loads((directory / 'tutorial.json').read_text())
                self.assertEqual(header['format'], 'magic600-jumbling-render/1')
                self.assertEqual((header['dimension'], header['pieces']), (4, 177120))
                self.assertEqual(header['revision'], int(when == 'after'))
                self.assertEqual(header['digest'], DIGESTS[when])
                self.assertEqual(header['digest'], self.states[when].digest())
                self.assertEqual(tutorial['digest'], header['digest'])
                self.assertEqual(tutorial['off_lattice'], off)
                self.assertEqual(self.states[when].off_lattice_count(), off)
                arrays = wj.arrays(self.states[when])
                self.assertEqual(set(header['files']), set(arrays))
                for name, expected in arrays.items():
                    raw = (directory / name).read_bytes()
                    self.assertEqual(raw, np.ascontiguousarray(expected).tobytes())
                    self.assertEqual(header['files'][name], {
                        'sha256': sha(raw), 'shape': list(expected.shape), 'dtype': expected.dtype.str})
                index = np.fromfile(directory / 'pose_index.i32', '<i4')
                lattice = np.fromfile(directory / 'pose_lattice.i32', '<i4')
                self.assertEqual(int((lattice[index] < 0).sum()), off)

    def test_moved_set_compares_exact_poses_and_is_same_for_both_pictures(self):
        before, after = self.states['before'], self.states['after']
        # Group by the two tables' exact pose keys, not by their local integer ids.
        before_keys = [p.key for p in before._poses]
        after_keys = [p.key for p in after._poses]
        expected = np.array([p for p, (a, b) in enumerate(zip(before.pose_id, after.pose_id))
                             if before_keys[a] != after_keys[b]], '<i4')
        self.assertEqual(len(expected), 3097)
        raws = []
        for directory in self.outputs.values():
            raw = (directory / 'moved.i32').read_bytes()
            moved = np.frombuffer(raw, '<i4')
            np.testing.assert_array_equal(moved, expected)
            self.assertTrue(np.all(np.diff(moved) > 0))
            tutorial = json.loads((directory / 'tutorial.json').read_text())
            self.assertEqual(tutorial['moved'], {'file': 'moved.i32', 'count': 3097, 'sha256': sha(raw)})
            raws.append(raw)
        self.assertEqual(raws[0], raws[1])
        self.assertGreater(int((before.pose_id != after.pose_id).sum()), 3097,
                           'this fixture exercises canonical pose-table renumbering')

    def test_provenance(self):
        for when, directory in self.outputs.items():
            tutorial = json.loads((directory / 'tutorial.json').read_text())
            self.assertEqual(tutorial['format'], 'magic600-tutorial-shot/1')
            self.assertEqual(tutorial['record'], (RUN / 'run.json').relative_to(exporter.ROOT).as_posix())
            self.assertEqual(tutorial['run_sha256'], sha((RUN / 'run.json').read_bytes()))
            self.assertEqual(tutorial['start_table_sha256'], sha((RUN / 'start-table.json').read_bytes()))
            self.assertEqual(tutorial['words'], [{'file': 'words/00000.json',
                                                 'sha256': sha((RUN / 'words/00000.json').read_bytes())}])
            self.assertEqual((tutorial['step'], tutorial['when'], tutorial['phase'], tutorial['subphase'], tutorial['rule']),
                             (0, when, 'bulk', None, 'seam'))
            self.assertEqual(tutorial['letter_count'], 1)
            self.assertEqual(tutorial['letters'], [[72, 18]])
            self.assertEqual(tutorial['caps'], [72])

    def copy_record(self, directory):
        base = Path(directory) / 'run'
        (base / 'words').mkdir(parents=True)
        for name in ('run.json', 'start-table.json', 'words/00000.json'):
            shutil.copyfile(RUN / name, base / name)
        return base

    def refused(self, base, out, message, *, step=0, when='before'):
        with contextlib.redirect_stderr(io.StringIO()) as stderr:
            result = exporter.main([str(base), '--step', str(step), '--when', when, '--out', str(out)])
        self.assertEqual(result, 1)
        self.assertIn(message, stderr.getvalue())
        self.assertFalse(out.exists() and (not out.is_dir() or any(out.iterdir())), 'refusal wrote output')

    def test_tampered_word_cli_writes_nothing(self):
        with tempfile.TemporaryDirectory(prefix='magic600-tutorial-tamper-') as directory:
            base = self.copy_record(directory)
            word = json.loads((base / 'words/00000.json').read_text())
            word['word'][0][1] = 19
            (base / 'words/00000.json').write_text(json.dumps(word))
            out = Path(directory) / 'output'
            result = subprocess.run([sys.executable, '-I', '-B', str(HERE / 'export_tutorial.py'), str(base),
                                     '--step', '0', '--when', 'before', '--out', str(out)],
                                    capture_output=True, text=True, timeout=120)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('word_sha256', result.stderr)
            self.assertFalse(out.exists())

    def test_seed_9050_refused_before_replay(self):
        with tempfile.TemporaryDirectory(prefix='magic600-tutorial-seed-') as directory:
            base = self.copy_record(directory)
            record = json.loads((base / 'run.json').read_text())
            record['start']['seed'] = 9050
            (base / 'run.json').write_text(json.dumps(record))
            self.refused(base, Path(directory) / 'output', '9011..9073')

    def test_nonempty_output_keeps_existing_bytes(self):
        with tempfile.TemporaryDirectory(prefix='magic600-tutorial-existing-') as directory:
            out = Path(directory) / 'output'
            out.mkdir()
            sentinel = out / 'keep.txt'
            sentinel.write_bytes(b'keep this file')
            with self.assertRaisesRegex(ValueError, 'not empty'):
                exporter.export(RUN, 0, 'after', out)
            self.assertEqual(list(out.iterdir()), [sentinel])
            self.assertEqual(sentinel.read_bytes(), b'keep this file')

    def test_output_inside_record_refused_without_creating_directory(self):
        with tempfile.TemporaryDirectory(prefix='magic600-tutorial-inside-') as directory:
            base = self.copy_record(directory)
            before = {p.relative_to(base).as_posix(): sha(p.read_bytes()) for p in base.rglob('*') if p.is_file()}
            self.refused(base, base / 'new-output', 'inside the record')
            after = {p.relative_to(base).as_posix(): sha(p.read_bytes()) for p in base.rglob('*') if p.is_file()}
            self.assertEqual(before, after)
            with self.assertRaisesRegex(ValueError, 'inside the record'):
                exporter.export(base, 0, 'before', base)

    def test_start_digest_mismatch_writes_nothing(self):
        with tempfile.TemporaryDirectory(prefix='magic600-tutorial-start-') as directory:
            base = self.copy_record(directory)
            record = json.loads((base / 'run.json').read_text())
            record['start']['digest'] = record['steps'][0]['before']['digest'] = '0' * 64
            (base / 'run.json').write_text(json.dumps(record))
            self.refused(base, Path(directory) / 'output', 'start: replayed digest mismatch')

    def test_after_boundary_checked_even_for_before_picture(self):
        with tempfile.TemporaryDirectory(prefix='magic600-tutorial-boundary-') as directory:
            base = self.copy_record(directory)
            record = json.loads((base / 'run.json').read_text())
            record['steps'][0]['after']['digest'] = '0' * 64
            (base / 'run.json').write_text(json.dumps(record))
            path = base / 'words/00000.json'
            word = json.loads(path.read_text())
            word['output_digest'] = '0' * 64
            path.write_text(json.dumps(word))
            for when in ('before', 'after'):
                out = Path(directory) / when
                out.mkdir()  # A pre-existing empty output also stays empty on refusal.
                self.refused(base, out, 'step 0 after: replayed digest mismatch', when=when)

    def test_passed_step_digest_mismatch_writes_nothing(self):
        with tempfile.TemporaryDirectory(prefix='magic600-tutorial-chain-') as directory:
            base = self.copy_record(directory)
            record = json.loads((base / 'run.json').read_text())
            record['steps'][1]['before']['digest'] = '0' * 64
            (base / 'run.json').write_text(json.dumps(record))
            self.refused(base, Path(directory) / 'output', 'broken digest chain', step=1)

    def test_lattice_word_and_verified_later_table(self):
        run = RUN.parents[1] / 'd0-s7106/run'
        record = json.loads((run / 'run.json').read_text())
        with tempfile.TemporaryDirectory(prefix='magic600-tutorial-lattice-') as directory:
            out = Path(directory) / 'before'
            header, tutorial = exporter.export(run, 19, 'before', out)
            self.assertEqual(header['digest'], record['steps'][19]['before']['digest'])
            self.assertEqual(tutorial['off_lattice'], 0)
            self.assertEqual(tutorial['checkpoint']['file'], 'tables/bulk-end.json')
            self.assertEqual(tutorial['checkpoint']['after_step'], 18)
            self.assertEqual(tutorial['words'], [{'file': record['steps'][19]['word'],
                'sha256': sha((run / record['steps'][19]['word']).read_bytes())}])
            self.assertEqual(tutorial['letter_count'], 194)
            self.assertNotIn('letters', tutorial)
            letters = exporter.read_word(run, record['steps'][19])
            self.assertEqual(tutorial['letters_sha256'], sha(json.dumps(letters).encode('utf-8')))
            self.assertEqual(tutorial['caps'], list(dict.fromkeys(c for c, _ in letters)))


class JavaScriptSyntax(unittest.TestCase):
    @unittest.skipUnless(shutil.which('node'), 'node is missing; JavaScript syntax checks skipped')
    def test_node_check(self):
        for name in ('full.js', 'capture_tutorial.mjs'):
            with self.subTest(file=name):
                result = subprocess.run(['node', '--check', str(HERE / name)], capture_output=True, text=True, timeout=30)
                self.assertEqual(result.returncode, 0, result.stderr)


if __name__ == '__main__':
    unittest.main(verbosity=2)
