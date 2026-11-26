"""ADR-D02 sample data (build framework B1): jumbled states rebuilt only through the Rust adapter.

Every state is rebuilt by the Rust jumbling adapter (`rust/c600-jumble`, `c600-jumble-adapter`) and checked against
the digest its research record names; nothing replays through the Python simulator (owner, 26 November 2026). The
simulator only reads a finished state, loaded from the adapter's own record (`j1_from_record`'s reading), to list the
pieces a twist moves for the piece masks. The chrome reads these files through `IJumbleSource` until K2 binds the
product engine.

Run from the repository root, after building the adapter in this checkout's own target directory:

    cargo build --release --locked --offline -p c600-jumble --bin c600-jumble-adapter \
        --manifest-path rust/Cargo.toml --target-dir rust/target
    python tools/looklab/fixtures/make_d02_samples.py --write
    python tools/looklab/fixtures/make_d02_samples.py --verify

`--verify` rebuilds everything in memory and compares it byte for byte with the tracked files. It
prints `D02_SAMPLES_PASS states=<n>` and exits 0, or names the first difference and exits 1. A run takes about ten
minutes; the two deep states take about two minutes each.

Evidence kind: exact research model through the Rust candidate engine. Nothing here is product engine, renderer,
Windows, input or performance evidence.
"""
import argparse
import base64
import hashlib
import json
from pathlib import Path
import sys
import time
import zlib

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
J = ROOT / 'research' / 'jumbling'
OUT = ROOT / 'tools' / 'looklab' / 'data' / 'samples' / 'jumble'
FORMAT = 'magic600-d02-sample/1'
PIECES = 177120
SEAM_CHUNK = 4096
SEAM_BUDGET = 10 ** 12
RELEASE_CHECKS = 12
REPAIR_ROWS = 9

# The research records each state is rebuilt from, relative to the repository root.
WJ = 'research/jumbling/fixtures/wj-I_a.json'
HOPF = 'tools/looklab/fixtures/hopf-rings.json'
DEEP = 'research/jumbling/restoration/11-scramble-census/runs/heights/ia-k3000-s7302.json'
S7001 = 'research/jumbling/restoration/11-scramble-census/runs/round1/ia-k10-s7001.json'
S7001_WORDS = 'research/jumbling/restoration/13-tutorial-runner/runs/depth10-seed7001/words'
H4_SOURCE = 'research/jumbling/differential/leads/results/summary.md:497'
# The 12 letters that take the deep state's descent end to the height-4 state (H4_SOURCE, copied as printed there;
# the end digest below checks them).
H4_LETTERS = [[64, 15], [185, 14], [357, 12], [471, 12], [263, 14], [407, 13], [523, 12],
              [124, 2], [274, 13], [46, 1], [46, 3], [104, 14]]
DEEP_DIGEST = {'deep-1': '1335f47e0c5647a948dd11f87ba4bad27c787d00992be1f783d970c88a542ace',
               'deep-2': 'bbef1c8ec0de78dead1473322a8c1236e48f9ccfe0017ce724461bb7f37ae299'}
SOLVED = 'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855'
# Two shipped corrections, the first and the last of the catalogue's fixed ID order.
CORRECTIONS = (0, 35)
# Not built in B1, with the step that builds each from its consumer's rule.
NOT_BUILT = {
    'contact.json': 'B15 builds it: the contact graph of the alignment domains needs the seam-face geometry',
    'hidden-piece masks': 'B16 builds them, under jumbling-observation.md rule 5',
}
FULL_STATES = ('wj-ia-300', 'wj-ia-end', 'ia-40', 'deep-1', 'deep-2')
CAP_STATES = ('wj-ia-300', 'wj-ia-end', 'deep-2')

if not __debug__:
    raise SystemExit('make_d02_samples.py: run without -O; its checks are assertions')


def log(*parts):
    print(time.strftime('%H:%M:%S'), *parts, file=sys.stderr, flush=True)


def read_json(relative):
    return json.loads((ROOT / relative).read_text(encoding='utf-8'))


def dump(obj):
    """Compact JSON with sorted keys and one closing newline: the files are data, checked by --verify byte for byte."""
    return (json.dumps(obj, sort_keys=True, separators=(',', ':'), ensure_ascii=False) + '\n').encode('utf-8')


def sha256(data):
    return hashlib.sha256(data).hexdigest()


class Research:
    """The modules this script reads: the adapter client, and the simulator for reading finished states."""

    def __init__(self):
        sys.set_int_max_str_digits(0)
        if str(J) not in sys.path:
            sys.path.insert(0, str(J))
        # The census module first: it moves the research centroid cache out of the repository.
        from differential.leads import census
        from differential.candidate import rust_jumble
        from differential.j1_state import twists_from_letters
        from differential.references import obs_refs
        import numpy
        from sim import api
        from sim.kplus import matrix_from_json, matrix_json
        if Path(census.seams.CACHE).resolve().is_relative_to(ROOT):
            raise AssertionError('the centroid cache is inside the repository')
        self.census, self.R, self.obs_refs, self.np, self.api = census, rust_jumble, obs_refs, numpy, api
        self.twists_from_letters, self.matrix_from_json = twists_from_letters, matrix_from_json
        self.matrix_json = matrix_json

    def j1_from_record(self, record):
        """The simulator's state from an adapter record (tests/test_jumbling_rust_candidate.py's reading)."""
        np = self.np
        raw = base64.b64decode(record['pose_ids']['data'], validate=True)
        entries = [(p['k'], None) if 'k' in p else (-1, self.matrix_from_json(p['m'])) for p in record['pose_table']]
        return self.api.from_pose_table(self.census.restore.ctx, entries,
                                        np.frombuffer(raw, dtype='<u4').astype(np.int64),
                                        self.census.menu(), compact=False)

    def inside(self, state, letter):
        """The pieces one menu letter moves on a copy of `state`, and the copy after it."""
        copy = self.api.copy_state(state)
        twist = self.twists_from_letters(self.census.menu(), [list(letter)])[0]
        outcome, inside = self.api.apply_with_inside(copy, twist)
        assert outcome.applied, (letter, outcome.status)
        return [int(p) for p in inside], copy


def cap_layout(model, hopf, normals):
    """cap-layout.json (B8, M-RING redone): each cap's row and column in the ring sheet, 20 rows of 30.

    The adapter's 600 caps are the model's 600 pole normals in their stored order (rust/c600-jumble/src/model.rs reads
    `normals` of assets/model.npz as its poles), and hopf-rings.json's cells are the same file's cells (its modelDigest
    is the adapter's model.npz digest). So cap c is cell c: its row is the Hopf ring that holds cell c, its column the
    cell's place along that ring, and the columns are cyclic. Row neighbours are face neighbours: two cells share a
    facet exactly when their normals are nearest, which the check below confirms for every pair of row neighbours.
    """
    assert model['files']['assets/model.npz'] == hopf['modelDigest'], 'hopf-rings.json names another model'
    rows = [ring['cells'] for ring in hopf['rings']]
    assert len(rows) == 20 and all(len(row) == 30 for row in rows), 'not 20 rows of 30'
    assert sorted(c for row in rows for c in row) == list(range(600)), 'the rows are not a bijection onto the caps'
    unit = normals / ((normals * normals).sum(axis=1, keepdims=True) ** 0.5)
    dots = unit @ unit.T
    for c in range(600):
        dots[c, c] = -2.0
    nearest = dots.max(axis=1)
    for row in rows:
        for j, c in enumerate(row):
            d = row[(j + 1) % 30]
            assert abs(dots[c, d] - nearest[c]) < 1e-9 and abs(dots[c, d] - nearest[d]) < 1e-9, (c, d)
    position = [None] * 600
    for r, row in enumerate(rows):
        for j, c in enumerate(row):
            position[c] = [r, j]
    return {'format': FORMAT, 'kind': 'cap-layout', 'model': hopf['modelDigest'], 'source': HOPF,
            'columns_cyclic': True, 'rows': rows, 'position': position,
            'reading': 'cap c is cell c of assets/model.npz; row = its Hopf ring, column = its place along the ring; '
                       'row neighbours are face neighbours, and the last column neighbours the first'}


def model_normals():
    """The 600 pole normals of the tracked assets/model.npz, as the adapter reads them."""
    import numpy
    with numpy.load(ROOT / 'assets' / 'model.npz') as z:
        return numpy.asarray(z['normals'], dtype=numpy.float64)


def mask(pieces):
    """A piece set as 177,120 bits (piece i is bit i % 8 of byte i // 8), zlib-compressed and base64-encoded."""
    bits = bytearray(PIECES // 8)
    for p in pieces:
        assert 0 <= p < PIECES
        bits[p >> 3] |= 1 << (p & 7)
    raw = bytes(bits)
    return {'count': len(set(pieces)), 'encoding': 'bits-le-zlib-base64', 'sha256': sha256(raw),
            'data': base64.b64encode(zlib.compress(raw, 9)).decode('ascii')}


class Builder:
    def __init__(self, research, out):
        self.r = research
        self.out = out
        self.files = {}
        self.index = []

    def put(self, relative, obj):
        assert relative not in self.files, relative
        self.files[relative] = dump(obj)

    # ------------------------------------------------------------------ one state
    def state(self, engine, sid, source, expected, *, full, caps=False, extra=None):
        record = engine.record()
        digest = record['digest']
        if expected is not None and digest != expected:
            raise AssertionError(f'{sid}: digest {digest} is not the recorded {expected}')
        np = self.r.np
        ids = np.frombuffer(base64.b64decode(record['pose_ids']['data'], validate=True), dtype='<u4')
        assert len(ids) == PIECES and record['pose_ids']['count'] == PIECES
        assert sha256(ids.tobytes()) == record['pose_ids']['sha256'], sid
        lattice_pose = np.array([1 if 'k' in p else 0 for p in record['pose_table']], dtype=np.uint8)
        lattice = lattice_pose[ids.astype(np.int64)]
        heights = engine.heights()
        seams = engine.seam_facts('all', chunk=SEAM_CHUNK, budget=SEAM_BUDGET, rows='none')
        assert seams['complete'] and not seams['cut_off'] and not seams['cancelled'], sid
        rim = engine.rim_summary()
        assert len(rim['rows']) == 600 and [r['cap'] for r in rim['rows']] == list(range(600)), sid
        classes = cap_classes(rim['rows'])
        header = {
            'format': FORMAT, 'state': sid, 'source': source,
            'digests': {'adapter': digest, 'pose_ids_sha256': record['pose_ids']['sha256'],
                        'pose_lattice_sha256': sha256(lattice.tobytes())},
            'expected_digest': expected,
            'pose_ids': {'encoding': 'u32le per piece, index into style.json rows',
                         'note': 'the adapter record\'s pose table order; B5 binds a shell session by these digests'},
        }
        totals = {
            'format': FORMAT, 'state': sid, 'digest': digest,
            'N': seams['totals']['total_N'], 'N_on_grip': seams['totals']['on_grip_mismatches'],
            'N_off_grip': seams['totals']['off_grip_mismatches'], 'aligned': seams['totals']['aligned'],
            'cells': seams['cells'], 'domains': seams['domains'],
            'off_lattice': heights['off'], 'height_counts': heights['height_counts'],
            'max_left_height': heights['max_left_height'], 'pose_classes': len(record['pose_table']),
            'grip_n': rim['grip_n'], 'grip_n_undefined': rim['grip_n_undefined'],
            'rim_complete': rim['complete'], 'uncertain_caps': rim['uncertain_caps'], 'cap_classes': classes,
            'nu': engine.nu(),
        }
        if extra:
            totals.update(extra)
        assert int((lattice == 0).sum()) == heights['off'], sid
        self.put(f'{sid}/state.json', header)
        self.put(f'{sid}/totals.json', totals)
        files = ['state.json', 'totals.json']
        named = []
        if full:
            self.put(f'{sid}/rim.json', {'format': FORMAT, 'state': sid, 'digest': rim['digest'],
                                         'complete': rim['complete'], 'uncertain_caps': rim['uncertain_caps'],
                                         'grip_n': rim['grip_n'], 'rows': rim['rows']})
            j1 = self.r.j1_from_record(record)
            assert j1.digest() == digest, f'{sid}: the simulator reads the record as {j1.digest()}'
            self.put(f'{sid}/style.json', self.style(sid, digest, heights, record, j1))
            files += ['rim.json', 'style.json']
            if caps:
                for cap in pick_caps(rim['rows']):
                    self.put(f'{sid}/cap-{cap}.json', self.cap(engine, j1, sid, digest, rim['rows'][cap]))
                    files.append(f'cap-{cap}.json')
                    named.append(cap)
            assert engine.record()['digest'] == digest, f'{sid}: a read changed the state'
        self.index.append({'state': sid, 'source': source, 'digest': digest, 'expected_digest': expected,
                           'files': files, 'named_caps': named, 'N': totals['N'], 'off_lattice': heights['off'],
                           'cap_classes': classes})
        log(sid, digest[:16], 'N', totals['N'], 'off', heights['off'], 'caps', named)

    def style(self, sid, digest, heights, record, j1):
        """Per pose (the record's pose table order): K+ index or none, domain index, heights, branches, count."""
        entries = self.r.api.pose_entries(j1)
        assert len(entries) == len(record['pose_table']) == len(heights['poses']), sid
        domains = sorted({row['domain'] for row in heights['poses'].values()})
        lattice_domains = set()
        rows = []
        for pid, entry in enumerate(entries):
            # The adapter keys a pose class by its exact matrix as JSON, a K+ pose too.
            key = json.dumps(self.r.matrix_json(self.r.api.pose_matrix(j1, pid)), separators=(',', ':'))
            if 'm' in record['pose_table'][pid]:
                assert key == json.dumps(record['pose_table'][pid]['m'], separators=(',', ':')), (sid, pid)
            row = heights['poses'][key]
            kidx = record['pose_table'][pid].get('k')
            assert (kidx is None) == (entry.kidx < 0), (sid, pid)
            if kidx is not None:
                lattice_domains.add(row['domain'])
            rows.append({'k': kidx, 'domain': domains.index(row['domain']), 'heights': row['heights'],
                         'branches': row['branches'], 'count': row['count']})
        assert len(lattice_domains) <= 1, sid
        lattice_domain = domains.index(lattice_domains.pop()) if lattice_domains else None
        return {'format': FORMAT, 'state': sid, 'digest': digest, 'lattice_key': heights['lattice_key'],
                'domains': domains, 'lattice_domain': lattice_domain, 'rows': rows}

    # ------------------------------------------------------------------ one named cap
    def cap(self, engine, j1, sid, digest, rim_row):
        cap = rim_row['cap']
        identity = menu_identity_item()
        items = [i for i in range(len(menu_items())) if i != identity]
        doc = {'format': FORMAT, 'state': sid, 'digest': digest, 'cap': cap, 'rim': rim_row}
        masks = {}
        if rim_row['blocked']:
            row = engine.straddlers([cap])[cap]
            certs = row['certificates']
            doc['straddlers'] = {'status': row['status'], 'pieces': len(row['pieces']),
                                 'uncertain_pieces': row['uncertain_pieces'],
                                 'certificates': len(certs), 'certificate_points': 2 * len(certs),
                                 'exact_evaluations': row['exact_evaluations']}
            masks['straddlers'] = mask(row['pieces'])
            doc['covers'] = refusable(lambda: engine.covers(cap))
            doc['repair_list'] = {'completed': refusable(lambda: engine.repair_list(cap))}
        else:
            doc['covers'] = engine.covers(cap)
            completed = engine.repair_list(cap)
            doc['repair_list'] = {'completed': completed,
                                  'cut_off': engine.repair_list(cap, max_pairs=3),
                                  'cancelled': engine.repair_list(cap, cancel_after=2)}
            doc['release_checks'] = {'items': items[:RELEASE_CHECKS],
                                     'note': 'the first twelve non-identity menu items at the cap; B10 maps a seat '
                                             'to its items',
                                     'rows': [slim_check(engine.release_check([cap, i]))
                                              for i in items[:RELEASE_CHECKS]]}
            inside, _ = self.r.inside(j1, [cap, items[0]])
            masks['inside'] = mask(inside)
            steps = []
            for k, (x, d, y, z, delta) in enumerate((completed.get('rows') or [])[:REPAIR_ROWS], 1):
                letters = [[cap, x], [d, y]] + ([] if z == identity else [[cap, z]])
                state = j1
                row_masks = []
                for letter in letters:
                    moved, state = self.r.inside(state, letter)
                    row_masks.append(mask(moved))
                steps.append({'row': k, 'letters': letters, 'delta_N': delta, 'moving_sets': row_masks})
            doc['repair_steps'] = steps
        doc['masks'] = masks
        return doc

    # ------------------------------------------------------------------ the whole set
    def build(self):
        r = self.r
        wj = read_json(WJ)
        records = wj['journal']['records']
        stages = wj['stages']
        assert len(records) == 841, len(records)
        menu = wj['menu']
        with r.R.RustJumbleEngine() as e:
            e.start(menu)
            identity = e.identity()
            model = e.model()
            self.put('cap-layout.json', cap_layout(model, read_json(HOPF), model_normals()))
            start = e.record()['digest']
            start_case = start == stages['start']['digest']
            self.state(e, 'on-zero', 'start(I_a), no twist', stages['start']['digest'], full=False)
            apply_records(e, records[:300])
            self.state(e, 'wj-ia-300', f'{WJ}, journal records 0 to 299', None, full=True, caps=True)
            apply_records(e, records[300:420])
            self.state(e, 'wj-ia-420', f'{WJ}, journal records 0 to 419', stages['mid']['digest'], full=False)
            apply_records(e, records[420:])
            self.state(e, 'wj-ia-end', f'{WJ}, journal records 0 to 840', stages['end']['digest'], full=True,
                       caps=True)

            stop = r.obs_refs.stop40_reference()
            e.start(menu)
            apply_records(e, records[:40])
            apply_word(e, identity, stop['moves'])
            self.state(e, 'ia-40', f'{WJ}, journal records 0 to 39, then the 62 restorer letters of '
                       'obs_refs.stop40_reference()', stop['digest'], full=True)

            deep = read_json(DEEP)
            assert (deep['seed'], deep['k'], deep['menu']) == (7302, 3000, 'I_a')
            e.start(menu)
            apply_word(e, identity, deep['history'])
            self.state(e, 'deep-1', f'{DEEP}, the 3,000-twist history', DEEP_DIGEST['deep-1'], full=True)
            apply_word(e, identity, deep['moves'])
            apply_word(e, identity, H4_LETTERS)
            self.state(e, 'deep-2', f'deep-1, the record\'s {len(deep["moves"])} moves, then the 12 letters at '
                       f'{H4_SOURCE}', DEEP_DIGEST['deep-2'], full=True, caps=True)

            s7001 = read_json(S7001)
            assert (s7001['seed'], s7001['k'], s7001['menu']) == (7001, 10, 'I_a')
            words = [read_json(f'{S7001_WORDS}/{k:05d}.json') for k in range(9)]
            e.start(menu)
            apply_word(e, identity, s7001['history'])
            self.state(e, 's7001-0', f'{S7001}, the 10-twist history', words[0]['input_digest'], full=False)
            for k, word in enumerate(words):
                if k:
                    assert word['input_digest'] == words[k - 1]['output_digest']
                apply_word(e, identity, word['word'])
                self.state(e, f's7001-{k + 1}', f'{S7001_WORDS}/{k:05d}.json applied to s7001-{k}',
                           word['output_digest'], full=False)
            assert words[-1]['output_digest'] == SOLVED

            for cid in CORRECTIONS:
                e.start(menu)
                found = e.correction_lookup(cid, twists=True)
                assert found['result'] == 'found', found
                apply_correction(e, identity, found['word'])
                self.state(e, f'nu-c{cid}', f'on-zero plus correction {cid} (correction_lookup, twists)', None,
                           full=False, extra={'correction': found['correction']})

            e.start(menu)
            self.put('corrections.json', {'format': FORMAT, 'catalogue': e.corrections(),
                                          'nu_constants': e.nu_constants()})
        self.put('states.json', {
            'format': FORMAT,
            'generator': 'tools/looklab/fixtures/make_d02_samples.py',
            'engine': {'adapter': 'rust/c600-jumble c600-jumble-adapter', 'model_id': model['model_id'],
                       'contract_revision': model['contract_revision']},
            'menu': {'name': menu['name'], 'identity': identity},
            'start_digest_case': {'adapter_start_digest': start, 'wj_stages_start_digest': stages['start']['digest'],
                                  'equal': start_case,
                                  'reading': 'the adapter digest and the W-J fixture digests are one function, so the '
                                             'W-J stage digests check wj-ia-420 and wj-ia-end'},
            'masks': 'piece i is bit i % 8 of byte i // 8 of 22,140 bytes, zlib then base64; sha256 of the raw bytes',
            'not_built': NOT_BUILT,
            'states': self.index,
        })


def apply_records(engine, records):
    for index, record in enumerate(records):
        step = engine.apply({'kind': 'record', 'record': record})
        assert step.status == 'admissible', (index, step.status)


def apply_word(engine, identity, letters):
    out = engine.word(identity, [list(letter) for letter in letters])
    assert out['status'] == 'admissible' and out['applied'] == len(letters), (out.get('status'), out.get('applied'))


def apply_correction(engine, identity, word):
    if 'letters' in word:
        apply_word(engine, identity, word['letters'])
        return
    twists = word['twists']
    for index, twist in enumerate(twists):
        step = engine.apply({'kind': 'record', 'record': twist})
        assert step.status == 'admissible', (index, step.status)


ID_LISTS = ('collateral_ids', 'others_changed_ids', 'pose_changed_ids')


def slim_check(row):
    """A release check with its piece-id lists replaced by their count and the SHA-256 of their JSON: the chrome
    shows the check's numerals, never the lists."""
    row = dict(row)
    for key in ID_LISTS:
        ids = row.pop(key)
        row[key[:-4] + '_count'] = len(ids)
        row[key[:-4] + '_sha256'] = sha256(json.dumps(ids, separators=(',', ':')).encode('ascii'))
    return row


def cap_classes(rows):
    """Counts of the cap map's classes (ADR-D02.14) from the rim rows: blocked, pending (no certified row), closed
    (N_c = 0), at floor (N_c = F_c > 0) and above floor."""
    counts = {'blocked': 0, 'pending': 0, 'closed': 0, 'at_floor': 0, 'above_floor': 0}
    for row in rows:
        if row['blocked']:
            counts['blocked'] += 1
        elif row['N_c'] is None:
            counts['pending'] += 1
        elif row['N_c'] == 0:
            counts['closed'] += 1
        elif row['at_floor']:
            counts['at_floor'] += 1
        else:
            counts['above_floor'] += 1
    assert sum(counts.values()) == 600
    return counts


def pick_caps(rows):
    """The named caps of a state: the lowest-id cap above floor, the lowest-id cap at floor and the lowest-id blocked
    cap, each where one exists."""
    picked = []
    for wanted in ('above_floor', 'at_floor', 'blocked'):
        for row in rows:
            kind = ('blocked' if row['blocked'] else None if row['N_c'] is None or row['N_c'] == 0
                    else 'at_floor' if row['at_floor'] else 'above_floor')
            if kind == wanted:
                picked.append(row['cap'])
                break
    return picked


IDENTITY = [[[1 if i == j else 0, 0, 1] for j in range(4)] for i in range(4)]


def menu_items():
    return read_json(WJ)['menu']['base']


def menu_identity_item():
    """The index of the identity rotation in the menu's base (I_a[0])."""
    hits = [i for i, item in enumerate(menu_items()) if item['matrix'] == IDENTITY]
    assert len(hits) == 1, hits
    return hits[0]


def refusable(call):
    """A request's answer, or its refusal as data (a blocked cap may refuse a cover or repair request)."""
    try:
        return call()
    except Exception as error:  # the adapter client's CandidateError; anything else is re-raised by its kind
        kind = getattr(error, 'kind', None)
        if kind is None:
            raise
        return {'status': 'refused', 'kind': kind, 'message': getattr(error, 'message', str(error))}


def write(files, folder):
    for relative, data in sorted(files.items()):
        path = folder / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)


def compare(files, folder):
    tracked = sorted(str(p.relative_to(folder)).replace('\\', '/') for p in folder.rglob('*') if p.is_file())
    if tracked != sorted(files):
        missing = sorted(set(files) - set(tracked))
        extra = sorted(set(tracked) - set(files))
        return f'file set differs: missing {missing[:5]}, extra {extra[:5]}'
    for relative in sorted(files):
        if (folder / relative).read_bytes() != files[relative]:
            return f'{relative} differs'
    return None


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument('--write', action='store_true', help='rebuild the tracked samples')
    mode.add_argument('--verify', action='store_true', help='rebuild into a temporary folder and compare')
    mode.add_argument('--layout', action='store_true', help="write cap-layout.json only (B8), from the adapter's model")
    args = parser.parse_args(argv)
    started = time.perf_counter()
    if args.layout:
        research = Research()
        with research.R.RustJumbleEngine() as e:
            e.start(read_json(WJ)['menu'])
            model = e.model()
        (OUT / 'cap-layout.json').write_bytes(dump(cap_layout(model, read_json(HOPF), model_normals())))
        log('wrote cap-layout.json')
        return 0
    builder = Builder(Research(), OUT)
    builder.build()
    states = len(builder.index)
    if args.write:
        if OUT.exists():
            for path in sorted(OUT.rglob('*'), reverse=True):
                if path.is_file():
                    path.unlink()
                elif path.is_dir():
                    path.rmdir()
        write(builder.files, OUT)
        log(f'wrote {len(builder.files)} files for {states} states in {time.perf_counter() - started:.0f} s')
        return 0
    problem = compare(builder.files, OUT)
    if problem:
        print(f'D02_SAMPLES_FAIL {problem}')
        return 1
    print(f'D02_SAMPLES_PASS states={states}')
    return 0


if __name__ == '__main__':
    sys.exit(main())
