"""ADR-D02's sample data (build framework B1), checked without the adapter.

The samples under tools/looklab/data/samples/jumble/ are written by tools/looklab/fixtures/make_d02_samples.py through
the Rust jumbling adapter. This test reads only the tracked files: their format, the digests the research records
name, the counts the build framework's section 6 states, and the internal sums (rim rows, pose counts, masks).
Standard library only.
"""
import base64
import hashlib
import json
from pathlib import Path
import struct
import unittest
import zlib

ROOT = Path(__file__).resolve().parents[1]
SAMPLES = ROOT / 'tools' / 'looklab' / 'data' / 'samples' / 'jumble'
WORDS = ROOT / 'research/jumbling/restoration/13-tutorial-runner/runs/depth10-seed7001/words'
WJ = ROOT / 'research/jumbling/fixtures/wj-I_a.json'
FORMAT = 'magic600-d02-sample/1'
PIECES = 177120
SOLVED = 'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855'
STATES = (['on-zero', 'wj-ia-300', 'wj-ia-420', 'wj-ia-end', 'ia-40', 'deep-1', 'deep-2']
          + [f's7001-{k}' for k in range(10)] + ['nu-c0', 'nu-c35'])
FULL = ('wj-ia-300', 'wj-ia-end', 'ia-40', 'deep-1', 'deep-2')
CAPS = ('wj-ia-300', 'wj-ia-end', 'deep-2')
# Digests the research records name (build framework section 6).
DIGESTS = {
    'on-zero': SOLVED,
    'deep-1': '1335f47e0c5647a948dd11f87ba4bad27c787d00992be1f783d970c88a542ace',
    'deep-2': 'bbef1c8ec0de78dead1473322a8c1236e48f9ccfe0017ce724461bb7f37ae299',
    's7001-9': SOLVED,
    # Pinned by B1's two equal runs and by the tracked bytes since (checkpoint C1 asked for constants).
    'wj-ia-300': '88d8b110bbf0eae42ca22a857c289ee36c5204d0f82631d5b56317611b9ab9cd',
    'nu-c0': '3d7669e3933bbc1644f30daad23c04cb8e677916b82b5038e57ad7423d303f00',
    'nu-c35': '80642dd327e7c86b085019db9fda41c87a2f540386d2bfeb90397bf1abf325d1',
}
# Counts the build framework's section 6 states.
COUNTS = {
    'wj-ia-420': {'off_lattice': 114930},
    'wj-ia-end': {'off_lattice': 121184},
    'deep-1': {'N': 145574, 'off_lattice': 118821, 'n_2': 116665, 'n_4': 1971, 'n_6': 185},
    'deep-2': {'off_lattice': 103896, 'n_2': 102960, 'n_4': 936},
    's7001-0': {'N': 13270, 'off_lattice': 26519},
    'ia-40': {'off_lattice': 860},
}


def read(relative):
    return json.loads((SAMPLES / relative).read_text(encoding='utf-8'))


def bits(entry):
    raw = zlib.decompress(base64.b64decode(entry['data'], validate=True))
    return raw


class SampleTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.index = read('states.json')
        cls.by_id = {row['state']: row for row in cls.index['states']}

    def test_index(self):
        self.assertEqual(self.index['format'], FORMAT)
        self.assertEqual([row['state'] for row in self.index['states']], STATES)
        case = self.index['start_digest_case']
        self.assertTrue(case['equal'])
        self.assertEqual(case['adapter_start_digest'], case['wj_stages_start_digest'])
        self.assertEqual(set(self.index['not_built']), {'hidden-piece masks'})

    def test_every_file_listed_exists_and_nothing_else(self):
        # j1-session.json and j0-session.json are the reduced Track J's sessions (B32, B33) and defects.json
        # holds Opus's planted J0 defects; none is adapter output.
        listed = {'states.json', 'corrections.json', 'cap-layout.json', 'meeting-caps.json', 'seed7001-words.json',
                  'j1-session.json', 'j0-session.json', 'defects.json'}
        for row in self.index['states']:
            listed |= {f"{row['state']}/{name}" for name in row['files']}
        present = {str(p.relative_to(SAMPLES)).replace('\\', '/') for p in SAMPLES.rglob('*') if p.is_file()}
        self.assertEqual(present, listed)

    def test_seed7001_words_are_the_word_files(self):
        """B25: the stored word's nine parts are the tutorial runner's word files, chained, ending solved."""
        words = read('seed7001-words.json')
        runner = ROOT / 'research' / 'jumbling' / 'restoration' / '13-tutorial-runner' / 'runs' / 'depth10-seed7001' / 'words'
        self.assertEqual(words['format'], FORMAT)
        self.assertEqual(len(words['parts']), 9)
        for k, part in enumerate(words['parts']):
            source = json.loads((runner / f'{k:05d}.json').read_text(encoding='utf-8'))
            self.assertEqual((part['letters'], part['input_digest'], part['output_digest']),
                             (source['word'], source['input_digest'], source['output_digest']))
            self.assertEqual(part['after'], f's7001-{k + 1}')
            if k:
                self.assertEqual(part['input_digest'], words['parts'][k - 1]['output_digest'])
        self.assertEqual(words['parts'][-1]['output_digest'], words['solved_digest'])

    def test_digests(self):
        wj = json.loads(WJ.read_text(encoding='utf-8'))['stages']
        expected = dict(DIGESTS, **{'wj-ia-420': wj['mid']['digest'], 'wj-ia-end': wj['end']['digest']})
        for sid in STATES:
            state = read(f'{sid}/state.json')
            self.assertEqual(state['format'], FORMAT, sid)
            digest = state['digests']['adapter']
            self.assertEqual(self.by_id[sid]['digest'], digest, sid)
            if state['expected_digest'] is not None:
                self.assertEqual(digest, state['expected_digest'], sid)
            if sid in expected:
                self.assertEqual(digest, expected[sid], sid)
            for key in ('pose_ids_sha256', 'pose_lattice_sha256'):
                self.assertRegex(state['digests'][key], '^[0-9a-f]{64}$', sid)

    def test_seed_7001_chain_follows_the_word_files(self):
        words = [json.loads((WORDS / f'{k:05d}.json').read_text(encoding='utf-8')) for k in range(9)]
        self.assertEqual(self.by_id['s7001-0']['digest'], words[0]['input_digest'])
        for k, word in enumerate(words):
            self.assertEqual(self.by_id[f's7001-{k + 1}']['digest'], word['output_digest'], k)

    def test_counts(self):
        for sid, wanted in COUNTS.items():
            totals = read(f'{sid}/totals.json')
            for key, value in wanted.items():
                got = totals['height_counts'][key] if key.startswith('n_') else totals[key]
                self.assertEqual(got, value, f'{sid} {key}')
        for sid in STATES:
            totals = read(f'{sid}/totals.json')
            self.assertEqual(sum(totals['cap_classes'].values()), 600, sid)
            self.assertEqual(totals['N'], totals['N_on_grip'] + totals['N_off_grip'], sid)
            if totals['off_lattice'] == 0:
                self.assertEqual(totals['N'], 0, sid)
        self.assertEqual(read('wj-ia-end/totals.json')['cap_classes']['blocked'], 573)

    def test_lattice_states_have_nu_and_off_lattice_states_do_not(self):
        for sid in STATES:
            totals = read(f'{sid}/totals.json')
            if totals['off_lattice']:
                self.assertEqual(totals['nu'].get('result'), 'undefined', sid)
            else:
                self.assertNotEqual(totals['nu'].get('result'), 'undefined', sid)

    def test_rim_and_style(self):
        for sid in FULL:
            totals = read(f'{sid}/totals.json')
            rim = read(f'{sid}/rim.json')
            self.assertEqual([row['cap'] for row in rim['rows']], list(range(600)), sid)
            blocked = sum(1 for row in rim['rows'] if row['blocked'])
            self.assertEqual(blocked, totals['cap_classes']['blocked'], sid)
            style = read(f'{sid}/style.json')
            self.assertEqual(len(style['rows']), totals['pose_classes'], sid)
            self.assertEqual(sum(row['count'] for row in style['rows']), PIECES, sid)
            off = sum(row['count'] for row in style['rows'] if row['k'] is None)
            self.assertEqual(off, totals['off_lattice'], sid)
            for row in style['rows']:
                self.assertIn(row['domain'], range(len(style['domains'])), sid)
                if row['k'] is not None:
                    self.assertEqual(row['domain'], style['lattice_domain'], sid)

    def test_poses_are_the_pose_ids(self):
        """B15: each full state's poses.json is its pose ids (u32le per piece), the digest its state.json names, and
        each style row's count is the number of pieces in that row."""
        for sid in FULL:
            poses = read(f'{sid}/poses.json')
            state = read(f'{sid}/state.json')
            style = read(f'{sid}/style.json')
            self.assertEqual((poses['format'], poses['state'], poses['count']), (FORMAT, sid, PIECES), sid)
            self.assertEqual(poses['digest'], state['digests']['adapter'], sid)
            raw = zlib.decompress(base64.b64decode(poses['data'], validate=True))
            self.assertEqual(len(raw), 4 * PIECES, sid)
            self.assertEqual(hashlib.sha256(raw).hexdigest(), poses['sha256'], sid)
            self.assertEqual(poses['sha256'], state['digests']['pose_ids_sha256'], sid)
            counts = [0] * len(style['rows'])
            for (pose,) in struct.iter_unpack('<I', raw):
                counts[pose] += 1
            self.assertEqual(counts, [row['count'] for row in style['rows']], sid)

    def test_contact_graphs(self):
        """B15 (ADR-D02.21): each full state's domain contact graph and its first class assignment: edges between
        the style's domains, contacts with the lattice domain and on off-grip cuts included; a proper colouring of
        the off-lattice domains with classes 1 to 6, the lattice domain neutral (0), no overflow in the samples. The
        A4 counts at wj-ia-300 (54 contacts, 20 with the lattice domain, 707 planes, N seam faces) hold."""
        for sid in FULL:
            contact = read(f'{sid}/contact.json')
            style = read(f'{sid}/style.json')
            totals = read(f'{sid}/totals.json')
            self.assertEqual((contact['format'], contact['state'], contact['digest']), (FORMAT, sid, style['digest']), sid)
            n, lattice = contact['domains'], contact['lattice_domain']
            self.assertEqual(n, len(style['domains']), sid)
            self.assertEqual(lattice, style['lattice_domain'], sid)
            self.assertEqual(contact['seam_faces'], totals['N'], sid)
            edges = contact['edges']
            self.assertEqual(edges, sorted(edges), sid)
            self.assertEqual(len({(a, b) for a, b, *_ in edges}), len(edges), sid)
            for a, b, on, off in edges:
                self.assertTrue(0 <= a < b < n, sid)
                self.assertTrue(on or off, sid)
            self.assertEqual(contact['edges_with_lattice'], sum(1 for a, b, *_ in edges if lattice in (a, b)), sid)
            self.assertEqual(contact['edges_off_grip'], sum(1 for *_, off in edges if off), sid)
            classes = contact['classes']
            self.assertEqual(len(classes), n, sid)
            for d, c in enumerate(classes):
                self.assertEqual(c == 0, d == lattice, f'{sid} domain {d}')
                self.assertIn(c, range(8), sid)
            for a, b, *_ in edges:
                if lattice not in (a, b):
                    self.assertNotEqual(classes[a], classes[b], f'{sid} contact {a}-{b}')
            used = {c for c in classes if 1 <= c <= 6}
            self.assertEqual(contact['class_count'], len(used), sid)
            self.assertEqual(contact['overflow'], classes.count(7), sid)
            self.assertEqual(contact['overflow'], 0, sid)
        wj = read('wj-ia-300/contact.json')
        self.assertEqual((len(wj['edges']), wj['edges_with_lattice'], wj['planes'], wj['seam_faces']), (54, 20, 707, 98048))
        self.assertEqual(wj['class_count'], 4)
        self.assertEqual(len(read('ia-40/contact.json')['edges']), 3)
        self.assertEqual(read('ia-40/contact.json')['class_count'], 1)

    def check_mask(self, entry, label):
        self.assertEqual(entry['encoding'], 'bits-le-zlib-base64', label)
        raw = bits(entry)
        self.assertEqual(len(raw), PIECES // 8, label)
        self.assertEqual(hashlib.sha256(raw).hexdigest(), entry['sha256'], label)
        self.assertEqual(sum(bin(b).count('1') for b in raw), entry['count'], label)

    def test_named_caps(self):
        for sid in CAPS:
            caps = self.by_id[sid]['named_caps']
            self.assertTrue(caps, sid)
            kinds = set()
            for cap in caps:
                doc = read(f'{sid}/cap-{cap}.json')
                label = f'{sid} cap {cap}'
                self.assertEqual(doc['cap'], cap, label)
                self.assertEqual(doc['rim']['cap'], cap, label)
                for name, entry in doc['masks'].items():
                    self.check_mask(entry, f'{label} {name}')
                if doc['rim']['blocked']:
                    kinds.add('blocked')
                    self.assertEqual(doc['masks']['straddlers']['count'], doc['straddlers']['pieces'], label)
                    self.assertEqual(doc['straddlers']['certificate_points'], 2 * doc['straddlers']['certificates'])
                    continue
                kinds.add('admissible')
                lists = doc['repair_list']
                self.assertEqual(lists['completed']['status'], 'completed', label)
                self.assertEqual(lists['cut_off']['status'], 'cut_off', label)
                self.assertEqual(lists['cancelled']['status'], 'cancelled', label)
                self.assertIsNone(lists['cut_off']['rows'], label)
                self.assertIsNone(lists['cancelled']['rows'], label)
                self.assertEqual(len(doc['covers']['answers']), 80, label)
                self.assertEqual([a['cap'] for a in doc['covers']['answers']],
                                 sorted(a['cap'] for a in doc['covers']['answers']), label)
                self.assertEqual(len(doc['release_checks']['rows']), 12, label)
                self.assertGreater(doc['masks']['inside']['count'], 0, label)
                rows = lists['completed']['rows']
                # C2-4: the adapter's order, menu order of x and then d, strictly; never by delta N.
                keys = [(row[0], row[1]) for row in rows]
                self.assertTrue(all(a < b for a, b in zip(keys, keys[1:])), f'{label}: repair rows not in (x, d) order')
                self.assertEqual(len(doc['repair_steps']), min(9, len(rows)), label)
                for step, row in zip(doc['repair_steps'], rows):
                    self.assertEqual(step['delta_N'], row[4], label)
                    self.assertIn(len(step['letters']), (2, 3), label)
                    self.assertEqual(len(step['moving_sets']), len(step['letters']), label)
                    for entry in step['moving_sets']:
                        self.check_mask(entry, f'{label} repair row {step["row"]}')
            self.assertIn('blocked', kinds, sid)

    def test_cap_layout_is_the_ring_sheet(self):
        # B8 (M-RING redone): a bijection of the 600 caps onto 20 rows of 30, each row a Hopf ring of
        # hopf-rings.json, cap c being cell c of assets/model.npz; row neighbours, the last and the first included,
        # are face neighbours (their normals are each other's nearest).
        import numpy
        layout = read('cap-layout.json')
        hopf = json.loads((ROOT / 'tools/looklab/fixtures/hopf-rings.json').read_text(encoding='utf-8'))
        self.assertEqual(layout['format'], FORMAT)
        self.assertEqual(layout['model'], hopf['modelDigest'])
        self.assertEqual(layout['rows'], [ring['cells'] for ring in hopf['rings']])
        self.assertEqual(sorted(c for row in layout['rows'] for c in row), list(range(600)))
        self.assertEqual([len(row) for row in layout['rows']], [30] * 20)
        for c, (r, j) in enumerate(layout['position']):
            self.assertEqual(layout['rows'][r][j], c)
        with numpy.load(ROOT / 'assets' / 'model.npz') as z:
            normals = numpy.asarray(z['normals'], dtype=numpy.float64)
        unit = normals / numpy.linalg.norm(normals, axis=1, keepdims=True)
        dots = unit @ unit.T
        numpy.fill_diagonal(dots, -2.0)
        nearest = dots.max(axis=1)
        for row in layout['rows']:
            for j, c in enumerate(row):
                d = row[(j + 1) % 30]
                self.assertAlmostEqual(dots[c, d], nearest[c], places=9)
                self.assertAlmostEqual(dots[c, d], nearest[d], places=9)

    def test_meeting_caps_are_j1s_rule(self):
        # B9: every cap meets 80 caps, J1's meeting_caps: n_c·n_d at least the 81st largest n_0·n_x. The products
        # fall on a few levels, so the float threshold is taken halfway to the next level, and the relation is
        # symmetric. Each named cap's covers record has the same domain.
        import numpy
        doc = read('meeting-caps.json')
        self.assertEqual(doc['format'], FORMAT)
        self.assertEqual(doc['model'], read('cap-layout.json')['model'])
        with numpy.load(ROOT / 'assets' / 'model.npz') as z:
            normals = numpy.asarray(z['normals'], dtype=numpy.float64)
        unit = normals / numpy.linalg.norm(normals, axis=1, keepdims=True)
        dots = unit @ unit.T
        levels = sorted(dots[0], reverse=True)
        self.assertGreater(levels[80] - levels[81], 1e-3)
        threshold = (levels[80] + levels[81]) / 2
        expected = [[d for d in range(600) if d != c and dots[c, d] > threshold] for c in range(600)]
        self.assertEqual(doc['caps'], expected)
        self.assertEqual({len(row) for row in doc['caps']}, {80})
        for c, row in enumerate(doc['caps']):
            for d in row:
                self.assertIn(c, doc['caps'][d])
        named = 0
        for path in sorted(SAMPLES.glob('*/cap-*.json')):
            record = json.loads(path.read_text(encoding='utf-8'))
            domain = record['covers'].get('domain')
            if domain is not None:
                self.assertEqual(domain, doc['caps'][record['cap']], path.name)
                named += 1
        self.assertGreaterEqual(named, 5)

    def test_corrections(self):
        doc = read('corrections.json')
        rows = doc['catalogue']['corrections']
        self.assertEqual([row['id'] for row in rows], list(range(36)))
        for sid, cid in (('nu-c0', 0), ('nu-c35', 35)):
            totals = read(f'{sid}/totals.json')
            self.assertEqual(totals['correction']['id'], cid, sid)
            self.assertEqual(totals['N'], 0, sid)
            # ν at the corrected state equals the correction's row (build framework section 6).
            self.assertEqual(totals['nu']['nu'], rows[cid]['row'], sid)


if __name__ == '__main__':
    unittest.main()
