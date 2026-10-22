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
NODE_HELPERS = r"""
import assert from 'node:assert/strict';
import { readFileSync, writeFileSync, mkdirSync, realpathSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { join, relative, isAbsolute, sep } from 'node:path';
import { runInNewContext } from 'node:vm';
const here = process.argv[1], input = JSON.parse(readFileSync(0, 'utf8'));
const json = (raw) => JSON.parse(raw.toString('utf8'));
const sha = (raw) => createHash('sha256').update(raw).digest('hex');
function sourceFunction(file, name, globals = {}) {
  const source = readFileSync(join(here, file), 'utf8');
  const match = source.match(new RegExp(`(?:async )?function ${name}\\([^]*?\\n\\}`));
  assert.ok(match, `${name}: source function not found`);
  return runInNewContext(`(${match[0]})`, globals);
}
"""


def run_node(test, code, value):
    result = subprocess.run(['node', '--input-type=module', '-e', NODE_HELPERS + code, str(HERE)],
                            input=json.dumps(value), capture_output=True, text=True, timeout=30)
    test.assertEqual(result.returncode, 0, result.stderr)
    return json.loads(result.stdout)


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

    @unittest.skipUnless(shutil.which('node'), 'node is missing; capture parameter check skipped')
    def test_capture_cap_and_ghost_defaults_overrides_and_manifest(self):
        empty = self.root / 'empty-caps'
        shutil.copytree(self.outputs['before'], empty)
        tutorial = json.loads((empty / 'tutorial.json').read_text())
        tutorial['caps'] = []
        (empty / 'tutorial.json').write_text(json.dumps(tutorial))
        result = run_node(self, r"""
const globals = { json, sha, insist: assert.ok, readFileSync, realpathSync, join, relative, isAbsolute, sep };
globals.insideFile = sourceFunction('capture_tutorial.mjs', 'insideFile', globals);
const dataFiles = sourceFunction('capture_tutorial.mjs', 'dataFiles', globals);
const entries = [
  { name: 'default', state: 'before', view: 'global', proj: 'persp', emph: 'moved', w: 1600, h: 1000 },
  { name: 'override', state: 'before', view: 'local', proj: 'stereo', emph: 'off', cap: 0, w: 1000, h: 1600 },
  { name: 'no-cap', state: 'empty-caps', view: 'global', proj: 'persp', emph: 'none', ghost: 'none', w: 800, h: 800 }
];
const shots = sourceFunction('capture_tutorial.mjs', 'shotsList', {
  json, insist: assert.ok, NAME: /^[A-Za-z0-9_-]{1,64}$/,
  readFileSync: () => Buffer.from(JSON.stringify(entries))
})('unused');
const files = new Map(), states = dataFiles(files, input.root, shots);
assert.deepEqual(Array.from(shots, (s) => s.cap), [72, 0, null]);
assert.throws(() => dataFiles(new Map(), input.root,
  [{ name: 'invalid-local', state: 'empty-caps', view: 'local' }]), /local view needs a cap/);
const urls = [];
// Exercise capture plumbing without a browser; these bytes are only a hash sentinel.
const png = Buffer.from('synthetic PNG hash sentinel');
const browser = { newContext: async () => ({
  newPage: async () => ({ setDefaultTimeout() {}, on() {}, route: async () => {},
    goto: async (url) => { urls.push(url); }, waitForFunction: async () => {},
    evaluate: async () => ({ shotReady: '1' }), locator: () => ({ screenshot: async () => png }) }),
  close: async () => {}
}) };
const render = sourceFunction('capture_tutorial.mjs', 'render', {
  mkdirSync, writeFileSync, join, sha, insist: assert.ok, URLSearchParams, console: { log() {} }
});
const manifest = await render(browser, 'http://127.0.0.1:1/', files, [], shots, states, input.out);
assert.equal(new URL(urls[0]).searchParams.get('cap'), '72');
assert.equal(new URL(urls[1]).searchParams.get('cap'), '0');
assert.equal(new URL(urls[2]).searchParams.has('cap'), false);
assert.deepEqual(urls.map((url) => new URL(url).searchParams.get('ghost')), ['all', 'near', 'none']);
assert.deepEqual(Array.from(manifest, (s) => s.cap), [72, 0, null]);
assert.deepEqual(Array.from(manifest, (s) => s.ghost), ['all', 'near', 'none']);
assert.ok(manifest.every((s) => s.png_sha256 === sha(png)));
process.stdout.write(JSON.stringify(manifest));
""", {'root': str(self.root), 'out': str(self.root / 'synthetic-capture')})
        self.assertEqual([s['cap'] for s in result], [72, 0, None])
        self.assertEqual([s['ghost'] for s in result], ['all', 'near', 'none'])

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


@unittest.skipUnless(shutil.which('node'), 'node is missing; tutorial view math checks skipped')
class TutorialViewMath(unittest.TestCase):
    def test_near_ghost_angle_contains_exactly_vertex_touching_cells_for_every_cap(self):
        angle = run_node(self, r"""
const source = readFileSync(join(here, 'full.js'), 'utf8');
const declaration = source.match(/^const NEAR_GHOST_ANGLE = [^;]+;/m);
assert.ok(declaration, 'near ghost angle not found');
process.stdout.write(JSON.stringify(runInNewContext(`${declaration[0]}\nNEAR_GHOST_ANGLE`)));
""", None)
        self.assertAlmostEqual(angle, np.pi / 4, places=12)
        # Topological oracle: vertex pieces identify all 20 cells incident at each
        # retained vertex, independently of the angular classifier and shell order.
        with np.load(exporter.ROOT / 'assets/model.npz') as model:
            vertices = np.flatnonzero(model['orbit_id'] == 34)
            self.assertEqual(len(vertices), 120)
            offsets, faces = model['face_offsets'], model['face_values']
            incidence = np.zeros((600, 120), dtype=np.int32)
            for vertex, piece in enumerate(vertices):
                hosts = faces[offsets[piece]:offsets[piece + 1]]
                self.assertEqual(len(hosts), 20)
                incidence[hosts, vertex] = 1
        np.testing.assert_array_equal(incidence.sum(axis=1), np.full(600, 4))
        touching = incidence @ incidence.T > 0
        # 56 neighbours, plus the cap itself.
        np.testing.assert_array_equal(touching.sum(axis=1), np.full(600, 57))
        mesh = json.loads((exporter.ROOT / 'assets/mesh.json').read_text())
        frames = np.fromfile(exporter.ROOT / 'assets/cell_frames.f32', '<f4').reshape(600, 4, 4).transpose(0, 2, 1)
        centers = frames @ np.array(mesh['normal'])
        centers /= np.linalg.norm(centers, axis=1)[:, None]
        angles = np.arccos(np.clip(centers @ centers.T, -1, 1))
        np.testing.assert_array_equal(angles <= angle, touching)
        self.assertAlmostEqual(np.rad2deg(angles[touching].max()), 44.4775, places=4)
        self.assertAlmostEqual(np.rad2deg(angles[~touching].min()), 49.1176, places=4)

    def test_cap_orientation_is_proper_and_faces_positive_w(self):
        # Independent retained centres; include exact coordinate-axis edge cases.
        mesh = json.loads((exporter.ROOT / 'assets/mesh.json').read_text())
        frames = np.fromfile(exporter.ROOT / 'assets/cell_frames.f32', '<f4').reshape(600, 4, 4).transpose(0, 2, 1)
        centers = frames @ np.array(mesh['normal']) / mesh['normal_length']
        points = np.concatenate((centers, np.eye(4), -np.eye(4), [[0.1, -0.2, 0.3, -0.4]]))
        matrices = run_node(self, r"""
const orient = sourceFunction('full.js', 'capOrientation');
for (const invalid of [[0, 0, 0, 0], [1, 2, 3], [NaN, 0, 0, 1]]) {
  assert.throws(() => orient(invalid), /Invalid cap centre/);
}
process.stdout.write(JSON.stringify(input.map((p) => orient(p))));
""", points.tolist())
        for i, (center, values) in enumerate(zip(points, matrices)):
            with self.subTest(cap=i):
                q = np.array(values).reshape(4, 4).T  # Renderer consumes column-major matrices.
                np.testing.assert_allclose(q.T @ q, np.eye(4), atol=1e-12)
                self.assertAlmostEqual(np.linalg.det(q), 1, places=12)
                rotated = q @ center
                np.testing.assert_allclose(rotated, [0, 0, 0, np.linalg.norm(center)], atol=1e-12)
                # Positive w is nearest d4=+1.18; opposite stereo pole is -w.
                np.testing.assert_allclose(1.18 * rotated[:3] / (1.18 - rotated[3]), 0, atol=1e-12)
                np.testing.assert_allclose(rotated[:3] / (1 + rotated[3]), 0, atol=1e-12)
                if i < len(centers):
                    self.assertEqual(int(np.argmax(centers @ q[3])), i)

    def test_page_and_capture_parameters(self):
        result = run_node(self, r"""
const parse = (query) => sourceFunction('full.js', 'shotParameters', {
  PARAMETERS: new URLSearchParams(query), requireValue: assert.ok
})();
assert.equal(parse('shot=step').cap, null);
assert.equal(parse('shot=step').emph, 'moved');
assert.equal(parse('shot=step').ghost, 'all');
assert.equal(parse('shot=step&view=local&proj=stereo').ghost, 'near');
assert.equal(parse('shot=step&view=local&proj=persp').ghost, 'near');
for (const view of ['global', 'local']) {
  for (const ghost of ['near', 'all', 'none']) {
    assert.equal(parse(`shot=step&view=${view}&ghost=${ghost}`).ghost, ghost);
  }
}
for (const ghost of ['', 'bad', 'NEAR', 'null', '0']) {
  assert.throws(() => parse(`shot=step&ghost=${ghost}`), /Invalid shot ghost/);
}
assert.equal(parse('shot=step&cap=0&emph=off').cap, 0);
assert.equal(parse('shot=step&cap=599&emph=off').emph, 'off');
for (const value of ['', '-1', '600', '1.5', 'NaN', '1x']) {
  assert.throws(() => parse(`shot=step&cap=${value}`), /Invalid shot cap/);
}
const list = (entry) => sourceFunction('capture_tutorial.mjs', 'shotsList', {
  readFileSync: () => Buffer.from(JSON.stringify([entry])), json, insist: assert.ok,
  NAME: /^[A-Za-z0-9_-]{1,64}$/
})('unused');
assert.equal(list({ name: 'a', state: 's', cap: 0, emph: 'off' })[0].cap, 0);
assert.equal(list({ name: 'a', state: 's', cap: 599 })[0].cap, 599);
assert.equal(list({ name: 'a', state: 's' })[0].cap, undefined);
assert.equal(list({ name: 'a', state: 's' })[0].ghost, 'all');
assert.equal(list({ name: 'a', state: 's', view: 'local' })[0].ghost, 'near');
for (const view of ['global', 'local']) {
  for (const ghost of ['near', 'all', 'none']) {
    assert.equal(list({ name: 'a', state: 's', view, ghost })[0].ghost, ghost);
  }
}
for (const ghost of ['', 'bad', 'NEAR', null, false, 0]) {
  assert.throws(() => list({ name: 'a', state: 's', ghost }), /Invalid view, proj, emph or ghost/);
}
for (const cap of [-1, 600, 1.5, '72', null]) {
  assert.throws(() => list({ name: 'a', state: 's', cap }), /Invalid cap/);
}
process.stdout.write(JSON.stringify({ ok: true }));
""", None)
        self.assertTrue(result['ok'])


if __name__ == '__main__':
    unittest.main(verbosity=2)
