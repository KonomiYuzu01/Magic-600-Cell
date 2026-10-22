"""Batch-driver acceptance with synthetic capture bytes; no browser is started."""
import contextlib
import hashlib
import importlib.util
import io
import json
from pathlib import Path, PureWindowsPath
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest import mock

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('make_figures', HERE / 'make_figures.py')
batch = importlib.util.module_from_spec(spec)
spec.loader.exec_module(batch)
REPOSITORY, REAL_EXPORT = batch.ROOT, batch.export_state
RUN = 'research/jumbling/restoration/13-tutorial-runner/runs/dev/d0-s7001/run'
NODE_HELPERS = r"""
const assert = require('node:assert/strict');
const { readFileSync, writeFileSync, mkdirSync } = require('node:fs');
const { createHash } = require('node:crypto');
const { join } = require('node:path');
const { runInNewContext } = require('node:vm');
const here = process.argv[1], input = JSON.parse(readFileSync(0, 'utf8'));
const json = raw => JSON.parse(raw.toString('utf8'));
const sha = raw => createHash('sha256').update(raw).digest('hex');
function sourceFunction(file, name, globals = {}) {
  const source = readFileSync(join(here, file), 'utf8');
  const match = source.match(new RegExp(`(?:async )?function ${name}\\([^]*?\\n\\}`));
  assert.ok(match, `${name}: source function not found`);
  return runInNewContext(`(${match[0]})`, globals);
}
"""


def run_node(test, code, value=None):
    result = subprocess.run(['node', '-e', NODE_HELPERS + code, str(HERE)],
                            input=json.dumps(value), capture_output=True, text=True, timeout=30)
    test.assertEqual(result.returncode, 0, result.stderr)
    return json.loads(result.stdout)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


class MakeFigures(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory(prefix='magic600-figures-test-')
        self.addCleanup(temporary.cleanup)
        self.directory = Path(temporary.name)
        self.repository = self.directory / 'repo'
        self.run = self.repository / 'records/example/run'
        self.run.mkdir(parents=True)
        batch.write_json(self.run / 'run.json', {'start': {'seed': 7001}})
        self.list = self.directory / 'figures.json'
        self.out = self.directory / 'figures'
        self.entries = [{'id': 'step0-before', 'record': 'records/example/run', 'step': 0,
                         'when': 'before', 'view': 'local', 'proj': 'stereo', 'emph': 'moved'}]
        self.changed_png = set()
        self.changed_framing = set()
        self.sheets = []
        self.real_png = False
        patcher = mock.patch.object(batch, 'ROOT', self.repository)
        patcher.start()
        self.addCleanup(patcher.stop)
        for attribute, replacement in (('export_state', self.export_stub), ('capture', self.capture_stub)):
            patcher = mock.patch.object(batch, attribute, side_effect=replacement)
            setattr(self, attribute, patcher.start())
            self.addCleanup(patcher.stop)

    def write_list(self):
        batch.write_json(self.list, {'format': batch.LIST_FORMAT, 'figures': self.entries, 'sheets': self.sheets})

    def export_stub(self, record, step, when, out):
        self.assertFalse(out.exists(), 'each export needs a fresh directory')
        out.mkdir(parents=True)
        tutorial = {'format': 'magic600-tutorial-shot/1',
                    'record': (record / 'run.json').relative_to(batch.ROOT).as_posix(),
                    'run_sha256': sha((record / 'run.json').read_bytes()), 'start_table_sha256': 'a' * 64,
                    'words': [{'file': 'words/00000.json', 'sha256': 'b' * 64}],
                    'step': step, 'when': when, 'caps': [72], 'off_lattice': 26519,
                    'digest': sha(f'{record.relative_to(batch.ROOT)}/{step}/{when}'.encode()),
                    'moved': {'file': 'moved.i32', 'count': 3097, 'sha256': 'c' * 64}}
        batch.write_json(out / 'tutorial.json', tutorial)

    def capture_stub(self, data, list_path, out, vendor):
        self.assertFalse(out.exists(), 'capture also needs a fresh directory')
        out.mkdir()
        results = []
        for shot in batch.read_json(list_path):
            raw = (data / shot['state'] / 'tutorial.json').read_bytes()
            tutorial = json.loads(raw)
            resolved = {'ghost': 'near' if shot['view'] == 'local' else 'all',
                        'w': 1600, 'h': 1000, 'cap': tutorial['caps'][0], 'fit': 'all', **shot}
            parameters = {key: resolved[key] for key in batch.PARAMETERS}
            # Deliberately synthetic bytes, sufficient for hashing and file handling.
            picture = json.dumps([parameters, tutorial['digest'], shot['name'] in self.changed_png]).encode()
            if self.real_png:
                from PIL import Image
                buffer = io.BytesIO()
                color = 'blue' if shot['name'] in self.changed_png else 'red'
                Image.new('RGB', (48, 32), color).save(buffer, format='PNG')
                picture = buffer.getvalue()
            (out / f'{shot["name"]}.png').write_bytes(picture)
            framing = {'fit': resolved['fit'], 'pieces': 4,
                       'in_frame': 2 if shot['name'] in self.changed_framing else 3}
            results.append({**resolved, 'framing': framing, 'header_digest': tutorial['digest'],
                            'tutorial_sha256': sha(raw), 'png_sha256': sha(picture)})
        batch.write_json(out / 'manifest.json', {'format': 'magic600-tutorial-capture/1',
            'versions': {'node': 'stub', 'playwright': 'stub', 'chromium': 'stub'},
            'sources': {name: 'd' * 64 for name in
                        ('full.js', 'full.html', 'three.module.js', 'OrbitControls.js')}, 'shots': results})

    def invoke(self, *options):
        self.write_list()
        stdout, stderr = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
            result = batch.main(['--list', str(self.list), '--out', str(self.out), *options])
        return result, stdout.getvalue() + stderr.getvalue()

    def assert_refused(self, text):
        result, output = self.invoke()
        self.assertEqual(result, 1, output)
        self.assertIn(text, output)
        self.export_state.assert_not_called()
        self.capture.assert_not_called()
        self.assertFalse(self.out.exists())

    def test_bad_id(self):
        for name in ('', '-first', 'Upper', '../escape', 'a_b', 'a' * 65, None, 3):
            with self.subTest(id=name):
                self.entries[0]['id'] = name
                self.assert_refused('figure id')

    def test_duplicate_id(self):
        self.entries.append(dict(self.entries[0], when='after'))
        self.assert_refused('duplicate figure id')

    def test_records_outside_repository(self):
        for record in ('/outside/run', '../outside/run', 'C:/outside/run', 'C:run',
                       '\\\\server\\share\\run', 'records/../../../outside/run'):
            with self.subTest(record=record):
                self.entries[0]['record'] = record
                self.assert_refused('repository-relative')

    def test_reserved_seeds_are_refused_before_any_export(self):
        reserved = self.repository / 'records/reserved/run'
        reserved.mkdir(parents=True)
        self.entries.append(dict(self.entries[0], id='reserved', record='records/reserved/run'))
        for seed in (9011, 9050, 9073):
            with self.subTest(seed=seed):
                batch.write_json(reserved / 'run.json', {'start': {'seed': seed}})
                self.assert_refused('reserved and refused')

    def test_invalid_parameters_and_unknown_fields(self):
        original = dict(self.entries[0])
        for field, value in (('step', True), ('step', -1), ('when', 'during'), ('cap', 600),
                             ('cap', True), ('w', 0), ('h', 8193), ('h', 1.5), ('view', 'bad'),
                             ('proj', 'bad'), ('emph', 'bad'), ('ghost', None), ('unknown', 1),
                             ('fit', 'bad'), ('fit', ''), ('fit', None), ('fit', False),
                             ('cs', 0.3), ('cs', 1.2), ('ss', 0.555), ('ss', True)):
            with self.subTest(field=field, value=value):
                self.entries = [{**original, field: value}]
                self.assert_refused('invalid')

    def test_fit_requirements_are_refused_before_any_export(self):
        original = dict(self.entries[0])
        # Put a valid figure first: a later invalid fit must still prevent all exports.
        for settings, message in (({'fit': 'cap', 'view': 'global'}, 'requires view=local'),
                                  ({'fit': 'core'}, 'requires ghost=none'),
                                  ({'fit': 'core', 'ghost': 'near'}, 'requires ghost=none'),
                                  ({'fit': 'core', 'ghost': 'all'}, 'requires ghost=none'),
                                  ({'fit': 'core', 'ghost': 'none', 'emph': 'none'}, 'emph other than none')):
            with self.subTest(settings=settings):
                self.entries = [original, {**original, 'id': 'invalid-fit', **settings}]
                self.assert_refused(message)

    def test_fit_passthrough_defaults_and_framing_in_manifest(self):
        original = dict(self.entries[0])
        self.entries += [dict(original, id='all', fit='all'),
                         dict(original, id='cap', fit='cap', ghost='none'),
                         dict(original, id='core', fit='core', ghost='none')]
        result, output = self.invoke()
        self.assertEqual(result, 0, output)
        self.export_state.assert_called_once()
        rows = batch.read_json(self.out / 'manifest.json')['figures']
        self.assertNotIn('fit', rows[0]['entry'])
        self.assertEqual(rows[0]['entry'], original)
        self.assertEqual([row['capture']['fit'] for row in rows], ['all', 'all', 'cap', 'core'])
        for entry, row in zip(self.entries, rows):
            self.assertEqual(row['entry'], entry)
            self.assertEqual(row['capture']['framing'],
                             {'fit': row['capture']['fit'], 'pieces': 4, 'in_frame': 3})

    def test_compact_json_round_trip_scalar_lists_and_order(self):
        document = {'caps': [64, 185, 357], 'mixed': [1, -2.5, 1e-12, True, False, None,
                    'comma, brackets [] and quote "', 'new\nline', '\\escaped'],
                    'nested': [{'empty': [], 'ids': ['a', 'b']}, [1, 2], {'object': {'x': 3}}],
                    'empty': {}, 'brackets': 'text [1,   2] stays as written'}
        path = self.directory / 'compact.json'
        batch.write_json(path, document)
        raw = path.read_text()
        self.assertEqual(json.loads(raw), document)
        self.assertEqual(list(json.loads(raw)), list(document))
        for key in ('caps', 'mixed'):
            self.assertIn(f'"{key}": {json.dumps(document[key])}', raw.splitlines()[list(document).index(key) + 1])
        self.assertIn('   "ids": ["a", "b"]', raw.splitlines())
        self.assertIn('  [1, 2],', raw.splitlines())
        self.assertIn('   "object": {\n    "x": 3\n   }', raw)
        batch.write_json(path, [1, 'top level', True, None])
        self.assertEqual(path.read_text(), '[1, "top level", true, null]\n')
        with self.assertRaises(ValueError):
            batch.write_json(path, {'bad': [float('nan')]})

    def test_wrong_format_empty_list_and_duplicate_json_keys(self):
        for document in ({'format': 'wrong', 'figures': self.entries},
                         {'format': batch.LIST_FORMAT, 'figures': []}, self.entries):
            with self.subTest(document=document):
                batch.write_json(self.list, document)
                with self.assertRaisesRegex(ValueError, batch.LIST_FORMAT):
                    batch.load_list(self.list)
        self.list.write_text('{"format": "a", "format": "b", "figures": []}')
        with self.assertRaisesRegex(ValueError, 'duplicate JSON key'):
            batch.load_list(self.list)

    def test_export_deduplication_and_capture_parameters(self):
        self.entries += [dict(self.entries[0], id='another-view', view='global', ghost='none',
                              cap=0, w=900, h=1200), dict(self.entries[0], id='after', when='after')]
        result, output = self.invoke('--vendor', str(self.directory / 'vendor'))
        self.assertEqual(result, 0, output)
        self.assertEqual(self.export_state.call_count, 2)
        self.assertEqual([(c.args[0], c.args[1], c.args[2]) for c in self.export_state.call_args_list],
                         [(self.run, 0, 'before'), (self.run, 0, 'after')])
        self.capture.assert_called_once()
        self.assertEqual(self.capture.call_args.args[3], self.directory / 'vendor')
        manifest = batch.read_json(self.out / 'manifest.json')
        self.assertEqual(manifest['format'], batch.MANIFEST_FORMAT)
        self.assertEqual(manifest['figures'][1]['capture']['cap'], 0)
        self.assertEqual(manifest['figures'][1]['capture']['ghost'], 'none')
        self.assertEqual(manifest['figures'][1]['capture']['w'], 900)
        self.assertEqual(manifest['figures'][1]['capture']['h'], 1200)
        # Both exports and capture outputs are removed after they have been copied.
        self.assertFalse(self.export_state.call_args.args[3].exists())
        self.assertFalse(self.capture.call_args.args[2].exists())

    def test_recipe_passthrough_deduplication_and_manifest(self):
        recipe = {'start': {'scramble': [0, 7001]}, 'digest': 'a' * 64, 'emph': {'off': True}}
        self.entries = [{key: value for key, value in self.entries[0].items()
                         if key not in ('record', 'step', 'when')} | {'state': recipe}]
        self.entries.append(dict(self.entries[0], id='second-view', view='global',
                                 state=dict(reversed(list(recipe.items())))))
        module = batch.recipe_exporter()

        def export(recipe, out, session):
            out.mkdir(parents=True)
            batch.write_json(out / 'tutorial.json', {'format': 'magic600-tutorial-shot/1', 'state': recipe,
                'inputs': [], 'digest': recipe['digest'], 'caps': [0], 'moved': {'count': 0}})

        with mock.patch.object(module, 'export_recipe', side_effect=export) as exporter:
            result, output = self.invoke()
            self.assertEqual(result, 0, output)
            exporter.assert_called_once()
            self.assertEqual(exporter.call_args.args[0], recipe)
        self.export_state.assert_not_called()
        rows = batch.read_json(self.out / 'manifest.json')['figures']
        self.assertEqual([row['tutorial']['state'] for row in rows], [recipe, recipe])

    def sheet_setup(self):
        self.real_png = True
        self.entries.append(dict(self.entries[0], id='step0-after', when='after'))
        self.sheets = [{'id': 'pair', 'panels': ['step0-before', 'step0-after'],
                        'labels': ['Before', 'After'], 'columns': 2}]

    def test_sheet_composition_is_deterministic_and_hashes_panels(self):
        from PIL import Image
        self.sheet_setup()
        self.assertEqual(self.invoke()[0], 0)
        manifest = batch.read_json(self.out / 'manifest.json')
        row = manifest['sheets'][0]
        self.assertEqual(row['entry'], self.sheets[0])
        self.assertEqual(row['composition']['font'], 'Pillow default (scalable)')
        self.assertEqual(row['composition']['size'], 24)
        for panel in row['panels']:
            self.assertEqual(panel['png_sha256'], sha((self.out / f'png/{panel["id"]}.png').read_bytes()))
        path = self.out / row['png']
        original = path.read_bytes()
        with Image.open(path) as im:
            self.assertEqual(im.mode, 'RGB')
            self.assertEqual(im.getpixel((0, 0)), (255, 255, 255))
            self.assertEqual(im.getpixel((12, 12)), (255, 0, 0))
            self.assertGreater(im.height, 32 + 24)
        self.assertEqual(self.invoke()[0], 0)
        self.assertEqual(path.read_bytes(), original)
        self.assertEqual(self.invoke('--check')[0], 0)

    def test_sheet_font_scales_with_canvas_width_and_wraps_deterministically(self):
        from PIL import Image
        self.sheet_setup()
        pictures = self.directory / 'wide-panels'
        pictures.mkdir()
        for name in self.sheets[0]['panels']:
            Image.new('RGB', (1024, 32), 'red').save(pictures / f'{name}.png')
        sheet = {**self.sheets[0], 'labels': ['A long label ' * 30, 'After']}
        first = batch.compose_sheet(sheet, pictures)
        output = (pictures / 'pair.png').read_bytes()
        # Two 1024-pixel panels, two 12-pixel margins and one 12-pixel gap.
        self.assertEqual(first['composition']['size'], round(2084 / 60))
        with Image.open(pictures / 'pair.png') as im:
            self.assertEqual(im.width, 2084)
            self.assertGreater(im.height, 32 + 24 + 2 * first['composition']['size'])
        self.assertEqual(batch.compose_sheet(sheet, pictures), first)
        self.assertEqual((pictures / 'pair.png').read_bytes(), output)

    def test_check_sheets_detects_bytes_labels_layout_panels_and_missing_sheet(self):
        self.sheet_setup()
        self.assertEqual(self.invoke()[0], 0)
        path = self.out / 'png/pair.png'
        original = path.read_bytes()
        self.changed_png.add('step0-after')
        result, output = self.invoke('--check')
        self.assertEqual(result, 1, output)
        self.assertIn('pair', output)
        self.assertEqual(path.read_bytes(), original)
        self.changed_png.clear()
        baseline = batch.read_json(self.out / 'manifest.json')
        baseline['sheets'][0]['panels'][0]['png_sha256'] = '0' * 64
        batch.write_json(self.out / 'manifest.json', baseline)
        self.assertEqual(self.invoke('--check')[0], 1)
        self.assertEqual(self.invoke()[0], 0)
        self.sheets[0]['labels'][0] = 'Changed'
        self.assertEqual(self.invoke('--check')[0], 1)
        self.sheets[0]['labels'][0] = 'Before'
        self.sheets[0]['columns'] = 1
        self.assertEqual(self.invoke('--check')[0], 1)
        self.sheets[0]['columns'] = 2
        path.write_bytes(b'damaged sheet')
        self.assertEqual(self.invoke('--check')[0], 1)
        self.assertEqual(path.read_bytes(), b'damaged sheet')
        path.unlink()
        self.assertEqual(self.invoke('--check')[0], 1)
        self.assertFalse(path.exists())
        self.sheets = []
        self.assertEqual(self.invoke('--check')[0], 1)

    def test_sheet_validation_precedes_exports(self):
        good = {'id': 'pair', 'panels': ['step0-before'], 'labels': ['Before'], 'columns': 1}
        for bad in ({**good, 'unknown': 1}, {**good, 'id': 'step0-before'},
                    {**good, 'panels': ['missing']}, {**good, 'panels': []},
                    {**good, 'labels': []}, {**good, 'labels': [1]},
                    {**good, 'columns': True}, {**good, 'columns': 0}, {**good, 'columns': 2}):
            with self.subTest(sheet=bad):
                self.sheets = [bad]
                self.assert_refused('sheet')

    def test_bad_panel_png_does_not_publish_or_prune(self):
        self.sheet_setup()
        self.assertEqual(self.invoke()[0], 0)
        extra = self.out / 'png/unlisted.png'
        extra.write_bytes(b'keep')
        original = {p.relative_to(self.out): p.read_bytes() for p in self.out.rglob('*') if p.is_file()}
        self.real_png = False
        result, output = self.invoke('--prune')
        self.assertEqual(result, 1, output)
        self.assertEqual(original,
                         {p.relative_to(self.out): p.read_bytes() for p in self.out.rglob('*') if p.is_file()})

    def test_recipe_reserved_seed_is_refused_before_other_exports(self):
        self.entries.append({'id': 'reserved', 'state': {'start': {'scramble': [0, 9050]}, 'digest': 'a' * 64},
                             'view': 'global', 'proj': 'persp', 'emph': 'moved'})
        self.assert_refused('reserved and refused')

    def test_recipe_checker_uses_temporary_exports_and_reports_each_failure(self):
        recipe = {'start': {'table': RUN + '/start-table.json'},
                  'play': [{'word': RUN + '/words/00000.json'}],
                  'digest': 'ab254e2af1775b8161092aed06d35252e81e63fa427fd75dc2d7f9aeb5b1c7f9'}
        entry = {'id': 'good', 'state': recipe, 'view': 'local', 'proj': 'stereo', 'emph': 'moved', 'cap': 72}
        command = [sys.executable, '-I', '-B', str(HERE / 'check_recipes.py'), '--list', str(self.list)]
        solved = dict(entry, id='solved', state={'start': {'scramble': [0, 7001]}, 'digest': sha(b'')})
        batch.write_json(self.list, {'format': batch.LIST_FORMAT,
                                    'figures': [entry, dict(entry, id='same'), solved]})
        result = subprocess.run(command, capture_output=True, text=True, timeout=120)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(len(result.stdout.splitlines()), 3)
        self.assertIn('good: ' + recipe['digest'] + ' emphasis=3097', result.stdout)
        bad = dict(entry, id='bad', state={**recipe, 'digest': '0' * 64})
        batch.write_json(self.list, {'format': batch.LIST_FORMAT, 'figures': [entry, bad]})
        result = subprocess.run(command, capture_output=True, text=True, timeout=120)
        self.assertEqual(result.returncode, 1, result.stderr)
        self.assertIn('bad: FAIL (state digest mismatch', result.stdout)
        self.assertFalse(self.out.exists())

    def test_recipe_checker_refuses_invalid_fit_before_exports(self):
        entry = {'id': 'bad-fit', 'state': {'start': {'scramble': [0, 7001]}, 'digest': sha(b'')},
                 'view': 'global', 'proj': 'persp', 'emph': 'moved', 'fit': 'cap'}
        batch.write_json(self.list, {'format': batch.LIST_FORMAT, 'figures': [entry]})
        result = subprocess.run([sys.executable, '-I', '-B', str(HERE / 'check_recipes.py'),
                                 '--list', str(self.list)], capture_output=True, text=True, timeout=30)
        self.assertEqual(result.returncode, 1, result.stderr)
        self.assertIn('fit=cap requires view=local', result.stderr)
        self.assertEqual(result.stdout, '')
        self.assertFalse(self.out.exists())

    def test_distinct_records_and_steps_are_not_deduplicated(self):
        other = self.repository / 'records/other/run'
        other.mkdir(parents=True)
        batch.write_json(other / 'run.json', {'start': {'seed': 7106}})
        self.entries += [dict(self.entries[0], id='step1', step=1),
                         dict(self.entries[0], id='other', record='records/other/run')]
        self.assertEqual(self.invoke()[0], 0)
        self.assertEqual(self.export_state.call_count, 3)
        self.capture.assert_called_once()

    def test_manifest_from_committed_export_with_capture_stub(self):
        batch.ROOT = REPOSITORY
        self.export_state.side_effect = REAL_EXPORT
        self.entries[0]['record'] = RUN
        self.entries += [dict(self.entries[0], id='global', view='global'),
                         dict(self.entries[0], id='after', when='after')]
        result, output = self.invoke()
        self.assertEqual(result, 0, output)
        self.assertEqual(self.export_state.call_count, 2)
        self.capture.assert_called_once()
        manifest = batch.read_json(self.out / 'manifest.json')
        self.assertEqual(manifest['versions']['chromium'], 'stub')
        self.assertEqual(set(manifest['sources']),
                         {'full.js', 'full.html', 'three.module.js', 'OrbitControls.js'})
        for entry, row in zip(self.entries, manifest['figures']):
            self.assertEqual(row['id'], entry['id'])
            self.assertEqual(row['entry'], entry)
            tutorial = row['tutorial']
            self.assertEqual(tutorial['record'], f'{RUN}/run.json')
            self.assertEqual(tutorial['run_sha256'], sha((REPOSITORY / RUN / 'run.json').read_bytes()))
            self.assertEqual(tutorial['words'], [{'file': 'words/00000.json',
                'sha256': sha((REPOSITORY / RUN / 'words/00000.json').read_bytes())}])
            self.assertEqual((tutorial['step'], tutorial['when']), (0, entry['when']))
            self.assertEqual(tutorial['moved']['count'], 3097)
            self.assertEqual(tutorial['off_lattice'], 26519 if entry['when'] == 'before' else 23422)
            self.assertEqual(tutorial['digest'], row['capture']['header_digest'])
            self.assertEqual(row['capture']['cap'], 72)
            self.assertEqual(row['capture']['ghost'], 'near' if entry['view'] == 'local' else 'all')
            self.assertEqual(row['png_sha256'], sha((self.out / row['png']).read_bytes()))

        def no_absolute_paths(value):
            if isinstance(value, str):
                self.assertFalse(Path(value).is_absolute(), value)
                self.assertFalse(PureWindowsPath(value).drive, value)
            elif isinstance(value, dict):
                for key, item in value.items():
                    no_absolute_paths(key)
                    no_absolute_paths(item)
            elif isinstance(value, list):
                for item in value:
                    no_absolute_paths(item)
        no_absolute_paths(manifest)

    def test_check_regenerates_and_preserves_original_outputs(self):
        self.assertEqual(self.invoke()[0], 0)
        original = {path.name: path.read_bytes() for path in self.out.rglob('*') if path.is_file()}
        self.export_state.reset_mock()
        self.capture.reset_mock()
        result, output = self.invoke('--check')
        self.assertEqual(result, 0, output)
        self.export_state.assert_called_once()
        self.capture.assert_called_once()
        self.assertEqual(original, {p.name: p.read_bytes() for p in self.out.rglob('*') if p.is_file()})
        self.changed_png.add(self.entries[0]['id'])
        result, output = self.invoke('--check')
        self.assertEqual(result, 1, output)
        self.assertIn('Figures differ: step0-before', output)
        self.assertEqual(original, {p.name: p.read_bytes() for p in self.out.rglob('*') if p.is_file()})

    def test_check_detects_state_digest_change_even_with_same_png_hash(self):
        self.assertEqual(self.invoke()[0], 0)
        baseline = batch.read_json(self.out / 'manifest.json')
        baseline['figures'][0]['tutorial']['digest'] = '0' * 64
        batch.write_json(self.out / 'manifest.json', baseline)
        result, output = self.invoke('--check')
        self.assertEqual(result, 1, output)
        self.assertIn('Figures differ: step0-before', output)

    def test_check_detects_fit_and_framing_changes_with_identical_pngs(self):
        self.assertEqual(self.invoke()[0], 0)
        original = (self.out / 'manifest.json').read_bytes()
        png = (self.out / 'png/step0-before.png').read_bytes()
        for key, value in (('fit', 'cap'),
                           ('framing', {'fit': 'all', 'pieces': 4, 'in_frame': 2})):
            with self.subTest(key=key):
                baseline = json.loads(original)
                baseline['figures'][0]['capture'][key] = value
                batch.write_json(self.out / 'manifest.json', baseline)
                before = (self.out / 'manifest.json').read_bytes()
                result, output = self.invoke('--check')
                self.assertEqual(result, 1, output)
                self.assertIn('Figures differ: step0-before', output)
                self.assertEqual((self.out / 'manifest.json').read_bytes(), before)
                self.assertEqual((self.out / 'png/step0-before.png').read_bytes(), png)
        (self.out / 'manifest.json').write_bytes(original)
        self.changed_framing.add('step0-before')
        self.assertEqual(self.invoke('--check')[0], 1)
        self.assertEqual((self.out / 'png/step0-before.png').read_bytes(), png)

    def test_check_detects_missing_added_or_damaged_figures(self):
        self.assertEqual(self.invoke()[0], 0)
        (self.out / 'png/step0-before.png').write_bytes(b'damaged')
        self.assertIn('Figures differ: step0-before', self.invoke('--check')[1])
        (self.out / 'png/step0-before.png').unlink()
        self.assertIn('Figures differ: step0-before', self.invoke('--check')[1])
        self.entries[0]['id'] = 'replacement'
        result, output = self.invoke('--check')
        self.assertEqual(result, 1, output)
        self.assertIn('Figures differ: replacement, step0-before', output)

    def test_extra_files_are_reported_and_only_prune_removes_them(self):
        self.assertEqual(self.invoke()[0], 0)
        extra = self.out / 'png/unlisted.png'
        extra.write_bytes(b'keep until prune')
        for options in ((), ('--check',)):
            result, output = self.invoke(*options)
            self.assertEqual(result, 0, output)
            self.assertIn('Unlisted files in png/: unlisted.png', output)
            self.assertEqual(extra.read_bytes(), b'keep until prune')
        result, output = self.invoke('--prune')
        self.assertEqual(result, 0, output)
        self.assertFalse(extra.exists())

    def test_check_prune_conflict_is_refused(self):
        result, output = self.invoke('--check', '--prune')
        self.assertEqual(result, 1, output)
        self.assertIn('cannot be combined', output)
        self.capture.assert_not_called()

    def test_failed_capture_does_not_publish_or_prune(self):
        self.assertEqual(self.invoke()[0], 0)
        extra = self.out / 'png/unlisted.png'
        extra.write_bytes(b'keep')
        original = (self.out / 'manifest.json').read_bytes()
        self.capture.side_effect = subprocess.CalledProcessError(1, ['capture-stub'])
        result, output = self.invoke('--prune')
        self.assertEqual(result, 1, output)
        self.assertEqual(extra.read_bytes(), b'keep')
        self.assertEqual((self.out / 'manifest.json').read_bytes(), original)


@unittest.skipUnless(shutil.which('node'), 'node is missing; shot parameter/capture checks skipped')
class CaptureFraming(unittest.TestCase):
    def test_page_and_capture_fit_choices_and_requirements(self):
        self.assertTrue(run_node(self, r"""
const parse = query => sourceFunction('full.js', 'shotParameters', {
  PARAMETERS: new URLSearchParams(query), requireValue: assert.ok
})();
const list = entry => sourceFunction('capture_tutorial.mjs', 'shotsList', {
  readFileSync: () => Buffer.from(JSON.stringify([{ name: 'a', state: 's', ...entry }])),
  json, insist: assert.ok, NAME: /^[A-Za-z0-9_-]{1,64}$/
})('unused')[0];
assert.equal(parse('shot=s').fit, 'all');
assert.equal(list({}).fit, 'all');
for (const view of ['local', 'global']) for (const proj of ['persp', 'stereo']) {
  for (const ghost of ['near', 'all', 'none']) for (const emph of ['moved', 'off', 'none']) {
    const entry = { view, proj, ghost, emph };
    const query = new URLSearchParams({ shot: 's', ...entry });
    assert.equal(parse(query).fit, 'all');
    for (const fit of ['all', 'cap', 'core']) {
      query.set('fit', fit);
      const error = fit === 'cap' && view !== 'local' ? /fit=cap requires view=local/
        : fit === 'core' && (ghost !== 'none' || emph === 'none') ? /fit=core requires ghost=none/ : null;
      if (error) {
        assert.throws(() => parse(query), error);
        assert.throws(() => list({ ...entry, fit }), error);
      } else {
        assert.equal(parse(query).fit, fit);
        assert.equal(list({ ...entry, fit }).fit, fit);
      }
    }
  }
}
for (const fit of ['', 'bad', 'ALL', 'null', '0']) {
  assert.throws(() => parse(`shot=s&fit=${fit}`), /Invalid shot fit/);
  assert.throws(() => list({ fit }), /Invalid fit/);
}
for (const fit of [null, false, 0, [], {}]) assert.throws(() => list({ fit }), /Invalid fit/);
for (const view of ['local', 'global']) {
  assert.throws(() => parse(`shot=s&view=${view}&fit=core`), /fit=core requires ghost=none/);
  assert.throws(() => list({ view, fit: 'core' }), /fit=core requires ghost=none/);
}
process.stdout.write('true');
"""))

    def test_capture_passes_fit_and_records_page_framing(self):
        with tempfile.TemporaryDirectory(prefix='magic600-capture-framing-') as temporary:
            results = run_node(self, r"""
(async () => {
  const entries = [
    { name: 'default', view: 'global' }, { name: 'all', view: 'local', fit: 'all' },
    { name: 'cap', view: 'local', ghost: 'none', fit: 'cap' },
    { name: 'core', view: 'global', ghost: 'none', fit: 'core' }
  ].map(s => ({ state: 's', cap: 0, ...s }));
  const shots = sourceFunction('capture_tutorial.mjs', 'shotsList', {
    readFileSync: () => Buffer.from(JSON.stringify(entries)), json, insist: assert.ok,
    NAME: /^[A-Za-z0-9_-]{1,64}$/
  })('unused');
  const reports = shots.map(s => ({ fit: s.fit, pieces: 8, in_frame: s.fit === 'core' ? 6 : 8 }));
  const urls = [], png = Buffer.from('synthetic PNG hash sentinel');
  let index = 0;
  const browser = { newContext: async () => {
    const report = reports[index++];
    return { newPage: async () => ({ setDefaultTimeout() {}, on() {}, route: async () => {},
      goto: async url => { urls.push(url); }, waitForFunction: async () => {},
      evaluate: async fn => runInNewContext(`(${fn})()`, {
        document: { documentElement: { dataset: { shotReady: '1' } } },
        window: { fullViewer: { view: { framing: report } } }
      }), locator: () => ({ screenshot: async () => png }) }), close: async () => {} };
  } };
  const render = sourceFunction('capture_tutorial.mjs', 'render', {
    mkdirSync, writeFileSync, join, sha, insist: assert.ok, URLSearchParams, console: { log() {} }
  });
  const results = await render(browser, 'http://127.0.0.1:1/', new Map(), [], shots,
    new Map([['s', { header_digest: 'a'.repeat(64), tutorial_sha256: 'b'.repeat(64) }]]), input);
  const plain = JSON.parse(JSON.stringify(results));
  assert.deepEqual(urls.map(url => new URL(url).searchParams.get('fit')), ['all', 'all', 'cap', 'core']);
  assert.deepEqual(plain.map(s => s.fit), ['all', 'all', 'cap', 'core']);
  assert.deepEqual(plain.map(s => s.framing), Array.from(reports));
  assert.ok(plain.every(s => s.png_sha256 === sha(png)));
  process.stdout.write(JSON.stringify(plain));
})().catch(e => { console.error(e); process.exitCode = 1; });
""", temporary)
            self.assertEqual([s['framing']['in_frame'] for s in results], [8, 8, 8, 6])


@unittest.skipUnless(shutil.which('node'), 'node is missing; framing computation check skipped')
class GhostFreeFraming(unittest.TestCase):
    # Synthetic geometry and minimal CPU matrix operations: exercise the actual
    # frameTutorial function without importing three.js, a DOM or a browser.
    HARNESS = r"""
const assert = require('node:assert/strict');
const { readFileSync } = require('node:fs');
const { runInNewContext } = require('node:vm');
const identity = () => Array.from({ length: 16 }, (_, i) => i % 5 === 0 ? 1 : 0);
const product = (m, v) => [0, 1, 2, 3].map(r => v.reduce((s, x, k) => s + m[k * 4 + r] * x, 0));
class Matrix4 {
  fromArray(a, o = 0) { this.a = Array.from(a).slice(o, o + 16); return this; }
  transpose() { this.a = this.a.map((_, i, a) => a[(i % 4) * 4 + Math.floor(i / 4)]); return this; }
  multiplyMatrices(a, b) {
    const columns = [0, 1, 2, 3].flatMap(i => product(a.a, b.a.slice(i * 4, i * 4 + 4)));
    this.a = columns; return this;
  }
  multiply(b) { return this.multiplyMatrices(this, b); }
}
class Vector4 {
  constructor(...a) { this.fromArray(a); }
  fromArray(a, o = 0) { [this.x, this.y, this.z, this.w] = Array.from(a).slice(o, o + 4); return this; }
  applyMatrix4(m) { return this.fromArray(product(m.a, [this.x, this.y, this.z, this.w])); }
  dot(v) { return this.x*v.x + this.y*v.y + this.z*v.z + this.w*v.w; }
}
const source = readFileSync(process.argv[1], 'utf8').match(/function frameTutorial\([^]*?\n\}/)[0];
const coordinates = () => ({ set(...a) { this.a = a; } });
function frame(view, proj, emph, ghost, moving = [5], aspect = 1.6, fit, fixture) {
  const cells = fixture?.coordinates || [0, 1, 2, 3, 4, 5].map(i => [i * 10, 0, 0, 1]);
  const ids = fixture?.ids || cells.map((_, i) => i);
  const frames = cells.flatMap(p => { const a = identity(); a.splice(12, 4, ...p); return a; });
  const pose = identity(); pose[3] = 20; pose[7] = 5; // row-major pose; moves the last cell's piece
  pose[15] = -0.5; // beyond the cap-facing stereo hemisphere, still a finite projection
  const camera = { near: 0.05, far: 100, fov: 90, aspect, position: coordinates(),
    updateProjectionMatrix() {
      const f = 1 / Math.tan(this.fov * Math.PI / 360), n = this.near, z = this.far;
      this.projectionMatrix = new Matrix4().fromArray([
        f / this.aspect, 0, 0, 0, 0, f, 0, 0, 0, 0, (z + n) / (n - z), -1,
        0, 0, 2 * z * n / (n - z), 0]);
    },
    updateMatrixWorld() {
      const a = identity(); a.splice(12, 3, ...this.position.a.map(x => -x));
      this.matrixWorldInverse = new Matrix4().fromArray(a);
    } };
  camera.position.set(0, 0, 5);
  camera.updateProjectionMatrix();
  const controls = { target: coordinates(), update() {} };
  controls.target.set(0, 0, 0);
  const slots = new Uint32Array(cells.length * 433); ids.forEach((id, i) => { slots[i * 433] = id; });
  const app = { view: {} };
  const globals = { THREE: { Matrix4, Vector4 }, shot: { view, proj, emph, ghost, cs: 0.76, ss: 0.82,
      ...(fit === undefined ? {} : { fit }) }, app,
    COUNTS: { cells: cells.length, base_stickers: 1, base_vertices: 3 }, camera, controls,
    capCenter: i => [i, 0, 0, 1], capOrientation: identity, cameraQ: identity, $: () => ({}),
    requireValue: assert.ok, poleParameters: () => ({ pole: [0, 0, 0, -1],
      basis: [[1, 0, 0, 0], [0, 1, 0, 0], [0, 0, 1, 0]] }),
    geometry: { anchors: [0, 0, 0, 1], mesh: { normal: [0, 0, 0, 1], normal_length: 1, offsets: [0, 3] },
      A: { 'cell_frames.f32': frames, 'slot_piece.u32': slots, 'mesh_sticker.u32': [0, 0, 0],
           'mesh_vertices.f32': [-1, -0.5, 0, 1, 1, -0.5, 0, 1, 0, 0.5, 0, 1] } },
    state: fixture ? { poses: identity(), index: ids.map(() => 0), lattice: [-1] }
      : { poses: [...identity(), ...pose], index: [0, 0, 0, 0, 0, 1], lattice: [0, -1] } };
  runInNewContext(`(${source})`, globals)(0, moving);
  return { eye: camera.position.a, target: controls.target.a, far: camera.far,
    framing: JSON.parse(JSON.stringify(app.view.framing)) };
}
const close = (a, b) => assert.ok(Math.abs(a - b) < 1e-9, `${a} != ${b}`);
"""

    def run_frame(self, code):
        result = subprocess.run(['node', '-e', self.HARNESS + code, str(HERE / 'full.js')],
                                capture_output=True, text=True, timeout=30)
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_posed_emphasis_extent_and_ghost_framing(self):
        self.run_frame(r"""
for (const proj of ['persp', 'stereo']) {
  const scale = proj === 'persp' ? 1.18 / 1.68 : 2;
  for (const view of ['local', 'global']) {
    for (const emph of ['moved', 'off']) {
      for (const aspect of [1.6, 0.6]) {
        const result = frame(view, proj, emph, 'none', [5], aspect);
        close(result.target[0], 70 * scale);
        close(result.target[1], 5 * scale);
        close(result.eye[0], result.target[0]); close(result.eye[1], result.target[1]);
        close(result.eye[2], 0.76 * 0.82 * scale / (0.8 * Math.min(1, aspect)));
        assert.ok(Number.isFinite(result.far));
      }
    }
    const all = frame(view, proj, 'none', 'none');
    const minX = -0.76 * 0.82 * (proj === 'persp' ? 1.18 / 0.18 : 0.5);
    const maxX = Math.max((40 + 0.76 * 0.82) * (proj === 'persp' ? 1.18 / 0.18 : 0.5),
      (70 + 0.76 * 0.82) * scale);
    close(all.target[0], (minX + maxX) / 2);
    const empty = frame(view, proj, 'moved', 'none', []);
    assert.deepEqual(empty.eye, [0, 0, 5]);
  }
}
for (const ghost of ['near', 'all']) {
  for (const view of ['local', 'global']) {
    const result = frame(view, 'stereo', 'moved', ghost);
    close(result.target[0], 0); close(result.target[1], 0);
    // Local uses the five unposed cells; global excludes the sixth posed cell in
    // the opposite hemisphere. Their projected rightmost coordinate is identical.
    close(result.eye[2], (40 + 0.76 * 0.82) * 0.5 / (view === 'local' ? 0.8 : 0.85));
  }
}
// Absent fit and fit=all retain the same camera for every combination, including
// no emphasis and empty sets. The known target/distance values above bracket it.
for (const view of ['local', 'global']) for (const proj of ['persp', 'stereo']) {
  for (const emph of ['moved', 'off', 'none']) for (const ghost of ['near', 'all', 'none']) {
    for (const aspect of [1.6, 0.6]) for (const moving of [[5], []]) {
      assert.deepEqual(frame(view, proj, emph, ghost, moving, aspect),
        frame(view, proj, emph, ghost, moving, aspect, 'all'));
    }
  }
}
""")

    def test_cap_uses_fixed_local_cells_with_or_without_ghosts(self):
        self.run_frame(r"""
for (const proj of ['persp', 'stereo']) for (const emph of ['moved', 'off', 'none']) {
  for (const aspect of [1.6, 0.6]) {
    const cap = frame('local', proj, emph, 'none', [5], aspect, 'cap');
    for (const ghost of ['near', 'all']) {
      const previous = frame('local', proj, emph, ghost, [5], aspect);
      assert.deepEqual(cap.eye, previous.eye);
      assert.deepEqual(cap.target, previous.target);
      assert.equal(cap.far, previous.far);
      assert.equal(cap.framing.pieces, emph === 'none' ? 0 : 1);
      // The posed sixth cell is beyond the cap frame in stereo; perspective's
      // fixed w=1 cells are much larger than the posed w=-0.5 cell.
      assert.equal(cap.framing.in_frame, proj === 'persp' && emph !== 'none' ? 1 : 0);
      assert.equal(previous.framing.pieces, cap.framing.pieces);
      assert.equal(previous.framing.in_frame, cap.framing.in_frame);
    }
  }
}
""")

    def test_core_excludes_outlier_and_counts_projected_piece_centroids(self):
        self.run_frame(r"""
const shrink = 0.76 * 0.82;
// The projection has scale 1 (w=0). Median x is 4, y is -shrink/6;
// ceil(3*5/4)=4 keeps x=0,2,4,6, excluding x=100. Bounds are
// [-shrink, 6+shrink] by [-shrink/2, shrink/2], with z=2 throughout.
const fixture = { coordinates: [0, 2, 4, 6, 100].map(x => [x, 0, 2, 0]) };
for (const view of ['local', 'global']) for (const proj of ['persp', 'stereo']) {
  for (const emph of ['moved', 'off']) for (const aspect of [1.6, 0.6]) {
    const result = frame(view, proj, emph, 'none', [4, 2, 0, 3, 1], aspect, 'core', fixture);
    assert.deepEqual(result.target, [3, 0, 2]);
    close(result.eye[2], 2 + (3 + shrink) / (0.8 * Math.min(1, aspect)));
    assert.equal(result.far, 100);
    assert.deepEqual(result.framing, { fit: 'core', pieces: 5, in_frame: 4 });
    assert.deepEqual(result, frame(view, proj, emph, 'none', [0, 1, 2, 3, 4], aspect, 'core', fixture));
    const all = frame(view, proj, emph, 'none', [0, 1, 2, 3, 4], aspect, 'all', fixture);
    close(all.target[0], 50);
    assert.deepEqual(all.framing, { fit: 'all', pieces: 5, in_frame: 5 });
  }
}
// One piece has stickers in two cells: use its mean projected vertices and
// count it only once. The core includes both its stickers, including x=-2.
const shared = { coordinates: [-2, 2, 0, 2, 100].map(x => [x, 0, 2, 0]), ids: [0, 0, 1, 2, 3] };
const multiple = frame('global', 'stereo', 'moved', 'none', [0, 1, 2, 3], 1.6, 'core', shared);
assert.deepEqual(multiple.target, [0, 0, 2]);
close(multiple.eye[2], 2 + (2 + shrink) / 0.8);
assert.deepEqual(multiple.framing, { fit: 'core', pieces: 4, in_frame: 3 });
""")

    def test_core_ties_depth_far_empty_set_and_report_front_check(self):
        self.run_frame(r"""
const shrink = 0.76 * 0.82;
// Median x=1; the two end pieces are tied at L-infinity distance 3.
// Piece 0 at x=4 wins over piece 3 at x=-2, despite appearing last.
const ties = { coordinates: [-2, 0, 2, 4].map(x => [x, 0, 0, 0]), ids: [3, 2, 1, 0] };
const tied = frame('global', 'stereo', 'moved', 'none', [3, 2, 1, 0], 1.6, 'core', ties);
assert.deepEqual(tied.target, [2, 0, 0]);
close(tied.eye[2], (2 + shrink) / 0.8);
// Depth is excluded from selection distance, but included in bounds and far.
const depth = { coordinates: [[0, 0, -200, 0], [2, 0, 0, 0], [4, 0, 10, 0], [100, 0, 0, 0]] };
const deep = frame('global', 'stereo', 'moved', 'none', [0, 1, 2, 3], 1.6, 'core', depth);
assert.deepEqual(deep.target, [2, 0, -95]);
close(deep.eye[2], 10 + (2 + shrink) / 0.8);
close(deep.far, deep.eye[2] + 201);
assert.deepEqual(deep.framing, { fit: 'core', pieces: 4, in_frame: 3 });
const empty = frame('local', 'stereo', 'moved', 'none', [], 0.6, 'core');
assert.deepEqual(empty.eye, [0, 0, 5]);
assert.deepEqual(empty.target, [0, 0, 0]);
assert.equal(empty.far, 100);
assert.deepEqual(empty.framing, { fit: 'core', pieces: 0, in_frame: 0 });
const behind = { coordinates: [0, 1, 2, 3, 4].map(x => [x, 0, 0, 0]).concat([[0, 0, 20, 0]]) };
for (const ghost of ['none', 'near', 'all']) {
  const report = frame('local', 'stereo', 'moved', ghost, [5], 1.6, 'cap', behind).framing;
  assert.deepEqual(report, { fit: 'cap', pieces: 1, in_frame: 0 });
}
// A centroid just inside the image edge is included even when some of its vertices
// extend beyond it. Aspect applies to the image rectangle, not to fit's square.
const edge = { coordinates: [0, 0, 0, 0, 0].map(x => [x, 0, 0, 0])
  .concat([[1.25 * shrink * (1 - 1e-12), 0, 0, 0]]) };
assert.equal(frame('local', 'stereo', 'moved', 'none', [5], 1, 'cap', edge).framing.in_frame, 1);
edge.coordinates[5][0] *= 1.01;
assert.equal(frame('local', 'stereo', 'moved', 'none', [5], 1, 'cap', edge).framing.in_frame, 0);
assert.equal(frame('local', 'stereo', 'moved', 'none', [5], 1.6, 'cap', edge).framing.in_frame, 1);
""")


if __name__ == '__main__':
    unittest.main(verbosity=2)
