"""Catalogue integrity checks on temporary, synthetic figure trees; no capture."""
import contextlib
import copy
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('tutorial_catalogue', HERE / 'catalogue.py')
C = importlib.util.module_from_spec(spec)
spec.loader.exec_module(C)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


class CatalogueTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory(prefix='magic600-catalogue-test-')
        self.addCleanup(temporary.cleanup)
        self.t = Path(temporary.name)
        self.f = self.t / 'figures'
        (self.f / 'png').mkdir(parents=True)
        (self.f / 'svg').mkdir()
        self.picture = {'id': 'single', 'state': {'start': {'scramble': [0, 7001]},
                        'digest': 'a' * 64}, 'view': 'global', 'proj': 'persp', 'emph': 'moved'}
        self.sheet = {'id': 'sheet', 'panels': ['single'], 'labels': ['Before'], 'columns': 1}
        self.listed = {'figures': [self.picture], 'sheets': [self.sheet]}
        rows = []
        for entry in (self.picture, self.sheet):
            raw = ('synthetic picture sentinel: ' + entry['id']).encode()
            path = 'png/' + entry['id'] + '.png'
            (self.f / path).write_bytes(raw)
            rows.append({'id': entry['id'], 'entry': copy.deepcopy(entry), 'png': path, 'png_sha256': sha(raw)})
        # As make_figures.py records it: the sheet lists the panel bytes it was composed from.
        rows[1]['panels'] = [{'id': 'single', 'png_sha256': rows[0]['png_sha256']}]
        self.manifest = {'figures': rows[:1], 'sheets': rows[1:]}
        self.save('figures.json', self.listed)
        self.save('manifest.json', self.manifest)
        raw = b'<svg xmlns="http://www.w3.org/2000/svg"><text>synthetic</text></svg>'
        (self.f / 'svg/diagram.svg').write_bytes(raw)
        self.save('svg/manifest.json', {'figures': {'diagram': {
            'output': {'path': 'diagram.svg', 'sha256': sha(raw)}}}})
        (self.t / '01-example.md').write_text('\n'.join([
            '![Figure 1.1: Picture](figures/png/single.png)',
            '![Figure 1.2: Sheet](figures/png/sheet.png)',
            '![Figure 1.3: Diagram](figures/svg/diagram.svg)',
        ]))
        for name, value in (('F', self.f), ('T', self.t)):
            patcher = patch.object(C, name, value)
            patcher.start()
            self.addCleanup(patcher.stop)
        self.publish_catalogue()

    def save(self, name, document):
        (self.f / name).write_text(json.dumps(document))

    def publish_catalogue(self):
        (self.f / 'CATALOGUE.md').write_text(C.render()[0])

    def check(self):
        before = {p.relative_to(self.t): p.read_bytes() for p in self.t.rglob('*') if p.is_file()}
        with contextlib.redirect_stdout(io.StringIO()) as output:
            result = C.main(['--check'])
        self.assertEqual(before, {p.relative_to(self.t): p.read_bytes() for p in self.t.rglob('*') if p.is_file()})
        return result, output.getvalue()

    def assert_stale(self, name):
        text, _ = C.render()
        row = next(line for line in text.splitlines() if f'/{name}.' in line)
        self.assertTrue(row.endswith('| stale |'), row)
        self.assertEqual(self.check()[0], 1)
        # An up-to-date catalogue must still fail if a referenced figure is stale.
        self.publish_catalogue()
        self.assertEqual(self.check()[0], 1)

    def test_unchanged_figures_are_drawn(self):
        text, invalid = C.render()
        self.assertFalse(invalid)
        self.assertEqual(text.count('| drawn |'), 3)
        self.assertNotIn('Recipe pictures await capture.', text)
        self.assertEqual(self.check()[0], 0)

    def test_corrupted_png_is_stale(self):
        for name in ('single', 'sheet'):
            with self.subTest(name=name):
                path = self.f / 'png' / (name + '.png')
                original = path.read_bytes()
                path.write_bytes(b'corrupted saved bytes')
                self.assert_stale(name)
                path.write_bytes(original)
                self.publish_catalogue()

    def test_changed_recipe_or_sheet_labels_are_stale(self):
        self.picture['state']['digest'] = 'b' * 64
        self.save('figures.json', self.listed)
        self.assert_stale('single')
        self.sheet['labels'] = ['Changed label']
        self.save('figures.json', self.listed)
        self.assert_stale('sheet')

    def embed_only_the_sheet(self):
        (self.t / '01-example.md').write_text('![Figure 1.2: Sheet](figures/png/sheet.png)\n')
        self.publish_catalogue()
        self.assertEqual(self.check()[0], 0)

    def test_sheet_with_changed_panel_recipe_is_stale(self):
        # VB-01: a chapter embeds only the sheet; its panel's recipe changes.
        self.embed_only_the_sheet()
        self.picture['fit'] = 'core'
        self.save('figures.json', self.listed)
        self.assert_stale('sheet')

    def test_sheet_composed_from_older_panel_bytes_is_stale(self):
        # The panel was captured again (bytes and manifest row agree), but the sheet was not composed again.
        self.embed_only_the_sheet()
        raw = b'synthetic picture sentinel: single, captured again'
        (self.f / 'png/single.png').write_bytes(raw)
        self.manifest['figures'][0]['png_sha256'] = sha(raw)
        self.save('manifest.json', self.manifest)
        self.assert_stale('sheet')

    def test_missing_png_is_stale(self):
        (self.f / 'png/single.png').unlink()
        self.assert_stale('single')

    def test_svg_digest_is_checked(self):
        (self.f / 'svg/diagram.svg').write_bytes(b'changed SVG')
        self.assert_stale('diagram')

    def test_svg_without_recorded_digests_keeps_existence_rule(self):
        self.save('svg/manifest.json', {'figures': {'diagram': {'inputs': {}}}})
        (self.f / 'svg/diagram.svg').write_bytes(b'changed SVG')
        self.assertEqual(self.check()[0], 0)


if __name__ == '__main__':
    unittest.main(verbosity=2)
