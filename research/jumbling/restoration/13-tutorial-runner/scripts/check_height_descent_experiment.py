"""Independent, current-table-only replay of HD2P/1 experiment records.

Run with Python -I -B, --out RUN (an existing experiment directory). No search
module or old pose/u contract is imported. Replay proves literal word contracts;
the search ledger and differential tests, not replay, establish enumeration order.
All geometry imports are lazy, after thread settings, cache routing and budgets.
"""

import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import resource
import signal
import sys
import time

sys.dont_write_bytecode = True
for _name in ('OPENBLAS_NUM_THREADS', 'OMP_NUM_THREADS', 'MKL_NUM_THREADS', 'NUMEXPR_NUM_THREADS'):
    os.environ[_name] = '1'

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[4]
J = ROOT / 'research/jumbling'
P = 177120
RULE = 'HD2P/1'
FORMAT = 'height-descent-experiment/1'
TABLE_ROOT = 'research/jumbling/restoration/13-tutorial-runner/runs/dev/'
# These are the only development tables the executable can open. No discovery,
# history reconstruction, acceptance inputs or run-directory Python imports.
CASES = {
    7302: ('d2-s7302/component-checkpoint/tables/00021.json',
           '919632272288dfb32288b02fc07b561799419c1574e5120a48e53b266d00e710',
           '2430f4cc99ea121a6d3476ec21b66fc401e277d13809ad5e1c979e6e0abe1472', (102750, 734, 0)),
    7303: ('d2-s7303/run2/d2-s7303/tables/00138.json',
           'efa9fe65fbd5114e07458ee584fd5aba3f8ecbcf77376d72e8b540035fb79959',
           'd01dc91edd1c73da399b8acb13a57b9f526da64a5d754993f42ae02856d51889', (99634, 929, 87)),
    7304: ('d2-s7304/component/tables/00005.json',
           '021151122328b1c604bdea9a747e9fffc2a0b54cab2d811c0975e5a0f686454a',
           'ac127dc3488832b1d1b75a043d63e84b75addd1f4e73fe7e961407aea2ff0980', (95674, 797, 0)),
    7305: ('d2-s7305/component/tables/00010.json',
           'f285ff06040a9e378d9475d9cabf7699ff94571f7fe78b8acd9685967c55f458',
           'cc5ef7ecf0510ea9a74fc52bafe324d6d8f58294c4b0e20b40b97b414dada062', (108453, 1419, 0)),
    7306: ('d2-s7306/component-checkpoint/tables/00017.json',
           'de32248e227598c7460203019495d762784956a28ddcfaca38af58d5daaff00a',
           'ac4be82cf80e857137af957ff75438525ba950062e46a2fc5249aa0d7199d8f2', (108360, 1955, 0)),
    7307: ('d2-s7307/component/tables/00005.json',
           '57517593a86b6480e32aca14ceb307369d2bf5d0fe534185a9d2356ef2ee6d01',
           '895b45de8a9a83604ca8bf988a3c9a287e404e59bd22a5004703d0852b252507', (98984, 1585, 0)),
}
PROBES = {
    7306: [([[487, 12], [240, 14]], 1965, {'2': 107936, '4': 1539}, '81e2c4d14b2f375b'),
           ([[208, 5], [336, 15]], 1955, {'2': 108336, '4': 1623}, '782c2034c13135fc'),
           ([[445, 55], [252, 13]], 1955, {'2': 108733, '4': 1685}, 'cea4e7cfd50d7d03'),
           ([[506, 7], [561, 14]], 1955, {'2': 106697, '4': 1944}, 'baebf2b8cf052fcc')],
    7305: [([[233, 9], [281, 12]], 1419, {'2': 108018, '4': 1395}, '9aef5203dfbbbb7d'),
           ([[233, 11], [281, 12]], 1419, {'2': 108054, '4': 1395}, 'a36fbffc9687c653')],
    7304: [([[23, 10]], 797, {'2': 95674, '4': 797},
            '1143e3ff9466dea3aa6fc41c384c7af5f353f775d193687d2524fab4fb3594b6')],
}
SOURCE_PATHS = [
    'research/jumbling/' + p for p in (
        'exact.py', 'witness.py', 'fixtures/wj.py', 'theory/restore.py',
        'theory/rim.py', 'theory/trees.py', 'theory/groups.py',
        'sim/__init__.py', 'sim/model.py', 'sim/kplus.py', 'sim/regions.py',
        'sim/kernel.py', 'sim/state.py', 'sim/twists.py')
] + [str((HERE / name).relative_to(ROOT)) for name in (
    'height_descent_experiment.py', 'check_height_descent_experiment.py',
    'test_height_descent_experiment.py')]


class Budget(Exception):
    """An unfinished operation; never evidence of family exhaustion."""


def require(ok, reason):
    if not ok:
        raise ValueError(reason)


def canonical(obj):
    return json.dumps(obj, sort_keys=True, separators=(',', ':'), allow_nan=False)


def sha(data):
    return hashlib.sha256(data).hexdigest()


def unique_keys(pairs):
    result = {}
    for k, v in pairs:
        require(k not in result, 'duplicate JSON key')
        result[k] = v
    return result


def parse(raw):
    return json.loads(raw, object_pairs_hook=unique_keys,
                      parse_constant=lambda _: require(False, 'nonfinite JSON'))


def write_json(path, doc):
    path = Path(path)
    pending = path.with_name(path.name + '.pending')
    pending.write_text(canonical(doc) + '\n', encoding='utf-8')
    pending.replace(path)


def emit(stream, doc):
    stream.write(canonical(doc) + '\n')
    stream.flush()


def external_directory(path):
    path = Path(path).resolve()
    require(not path.is_relative_to(ROOT), 'outputs must be outside the repository')
    return path


class Limits:
    """Absolute process CPU budgets, including imports, with a hard Linux backstop.

    The profiling timer interrupts long Python/C work before RLIMIT_CPU kills it.
    A killed process leaves no completed FINAL. The one-second reserve is solely
    for writing an incomplete BUDGET record, not for further search or replay.
    """
    def __init__(self, total, search=None):
        require(math.isfinite(total) and 2 <= total <= 3300, 'invalid total CPU budget')
        if search is not None:
            require(math.isfinite(search) and 0 <= search <= 2400 and search < total,
                    'invalid search CPU budget')
        self.total, self.search = total, search
        self.wall_start = time.monotonic()
        old_hard = resource.getrlimit(resource.RLIMIT_CPU)[1]
        hard = math.floor(total)
        if old_hard != resource.RLIM_INFINITY:
            hard = min(hard, old_hard)
        resource.setrlimit(resource.RLIMIT_CPU, (hard, hard))
        signal.signal(signal.SIGPROF, self._expired)
        signal.setitimer(signal.ITIMER_PROF, max(.001, min(total, hard) - 1 - time.process_time()))

    @staticmethod
    def _expired(*_):
        raise Budget('total CPU allowance; verification incomplete')

    def check(self, search=False):
        if time.process_time() >= self.total - 1:
            raise Budget('total CPU allowance; verification incomplete')
        if search and self.search is not None and time.process_time() >= self.search:
            raise Budget('search CPU allowance')

    def times(self):
        return {'cpu_seconds': time.process_time(),
                'wall_seconds': time.monotonic() - self.wall_start}


class Runtime:
    """Fixed definitions only; deliberately avoids all centroid helpers."""
    def __init__(self, output):
        output = external_directory(output)
        os.environ['JUMBLING_HOME_CACHE'] = str(output / 'home_centroids.npy')
        if 'seams' in sys.modules:
            sys.modules['seams'].CACHE = output / 'home_centroids.npy'
        sys.path[:0] = [str(J / 'theory'), str(J), str(J / 'fixtures')]
        import numpy as np
        import restore
        import rim
        import sim
        import trees
        from exact import Q5, dot
        from sim.kplus import matrix_from_json, matrix_json
        self.np, self.restore, self.rim, self.sim, self.trees = np, restore, rim, sim, trees
        self.Q5, self.dot = Q5, dot
        self.matrix_from_json, self.matrix_json = matrix_from_json, matrix_json
        self.ctx, self.menu = restore.ctx, restore.menu('I_a')
        require(restore.jumble_items(self.menu) == list(range(12, 60)), 'I_a numbering')
        require(len(self.ctx.data.sigbits) == P, 'piece count')

    def identity(self):
        return {
            'sources': {p: sha((ROOT / p).read_bytes()) for p in SOURCE_PATHS},
            'model': dict(self.ctx.data.identity),
            'manifest_sha256': sha((ROOT / 'assets/manifest.json').read_bytes()),
            'menu': {'name': self.menu.name, 'identity': self.menu.identity,
                     'ordered_items_sha256': sha(canonical([
                         self.matrix_json(m) for _, m in self.menu.items]).encode()),
                     'frames_sha256': sha(canonical([
                         self.matrix_json(self.menu.frame(c)) for c in range(600)]).encode())},
        }


def input_identity(case):
    path, file_hash, state_hash, counts = CASES[case]
    return {'case': case, 'table': TABLE_ROOT + path, 'sha256': file_hash,
            'digest': state_hash, 'heights': {str(h): n for h, n in zip((2, 4, 6), counts) if n}}


def load_table(path, rt, expected_sha=None, expected_digest=None):
    """The current-only check_run loader, with explicit complete-table validation.

    Journal contents are discarded, never interpreted or passed to the rule.
    The pinned file digest authenticates the input matrices; final tables also
    have to reproduce the independently replayed configuration digest.
    """
    raw = Path(path).read_bytes()
    if expected_sha is not None:
        require(sha(raw) == expected_sha, 'table file hash')
    doc = parse(raw)
    require(isinstance(doc, dict) and set(doc) in (
        {'poses', 'pose_id', 'digest', 'journal'}, {'poses', 'pose_id', 'digest'}), 'table schema')
    poses, ids = doc['poses'], doc['pose_id']
    require(isinstance(poses, list) and bool(poses) and isinstance(ids, list) and len(ids) == P,
            'table assignment count')
    require(all(type(i) is int and 0 <= i < len(poses) for i in ids), 'table pose index')
    require(isinstance(doc['digest'], str) and len(doc['digest']) == 64, 'table digest format')
    if expected_digest is not None:
        require(doc['digest'] == expected_digest, 'pinned state digest')
    for m in poses:
        require(isinstance(m, list) and len(m) == 4 and
                all(isinstance(row, list) and len(row) == 4 for row in m), 'matrix shape')
        require(all(isinstance(q, list) and len(q) == 3 and
                    all(type(x) is int for x in q) and q[2] > 0
                    for row in m for q in row), 'matrix scalars')
    st = rt.sim.State(rt.ctx, menu=rt.menu, filtered=False)
    remap = rt.np.array([st._intern_matrix(rt.matrix_from_json(m)) for m in poses], rt.np.int32)
    st.pose_id = remap[rt.np.asarray(ids, rt.np.int64)]
    st._compact()
    st.journal, st._anchor_cache = [], {}
    require(st.digest() == doc['digest'], 'reconstructed state digest')
    return st


def load_case(case, rt, engine):
    pin = input_identity(case)
    st = load_table(ROOT / pin['table'], rt, pin['sha256'], pin['digest'])
    require(engine.histogram(st) == pin['heights'], 'pinned complete histogram')
    return st


def export_table(st, rt, path):
    write_json(path, {'poses': [rt.matrix_json(st._pose_matrix(i)) for i in range(len(st._poses))],
                      'pose_id': st.pose_id.tolist(), 'digest': st.digest(), 'journal': []})


def maximum(hist):
    return max(map(int, hist), default=0)


def qvalue(hist):
    return (P + 1) * hist.get('6', 0) + hist.get('4', 0)


def handoff(hist):
    return not (set(hist) - {'2'})


class ReplayEngine:
    """Measurements from actual states, never predicted count deltas."""
    def __init__(self, rt, tick=lambda: None):
        self.rt, self.tick = rt, tick
        self._height = {}

    def clone(self, st):
        out = self.rt.sim.State(self.rt.ctx, menu=self.rt.menu, filtered=False)
        out.pose_id, out._poses, out._lookup = st.pose_id.copy(), list(st._poses), dict(st._lookup)
        return out

    def off(self, st):
        return set(self.rt.np.flatnonzero(st._kpose()[st.pose_id] < 0).tolist())

    def histogram(self, st):
        hist = {}
        ids, counts = self.rt.np.unique(st.pose_id, return_counts=True)
        for pid, count in zip(ids.tolist(), counts.tolist()):
            self.tick()
            if st._poses[pid].kidx >= 0:
                continue
            matrix = st._pose_matrix(pid)
            if matrix not in self._height:
                left, right = self.rt.trees.factors(matrix)
                h = int(self.rt.trees.dist(left))
                require(self.rt.trees.dist(right) == 0 and h > 0 and h % 2 == 0,
                        'I_a pose invariant')
                self._height[matrix] = h
            h = str(self._height[matrix])
            hist[h] = hist.get(h, 0) + count
        return hist

    def blocked(self, st):
        blocked = set()
        for c in range(600):
            self.tick()
            cl = st.classify(c, need_inside=False, stop_at_straddle=True)
            require(not cl.uncertain, 'uncertain cap classification')
            if cl.straddle is not None:
                blocked.add(c)
        return blocked

    def snapshot(self, st, initial_lattice, ever):
        self.tick()
        hist, off = self.histogram(st), self.off(st)
        loss = initial_lattice & off
        ever = ever | loss
        bk = len(self.blocked(st))
        self.tick()
        n = int(self.rt.rim.Domains(st).total_misalignment())
        self.tick()
        q = qvalue(hist)
        return ({'digest': st.digest(), 'heights': hist, 'maximum': maximum(hist),
                 'n2_n4_n6': [hist.get(str(h), 0) for h in (2, 4, 6)],
                 'Q': q, 'Bk': bk, 'M': 601 * q + bk, 'N': n, 'off': len(off),
                 'lattice_loss': len(loss), 'ever_lattice_loss': len(ever)}, ever)

    def apply(self, st, c, j):
        self.tick()
        tw = self.rt.sim.Twist(self.rt.ctx, c, self.rt.restore.element(self.rt.menu, c, j),
                               'menu', {'menu': self.rt.menu.name})
        outcome = st.apply(tw, journal=False)
        require(outcome.applied and not outcome.uncertain, 'inadmissible or uncertain letter')
        self.tick()

    def meets(self, c, d):
        # Independent evaluation of r22d81_pairs.py:88-92, with its proved R².
        data, dot = self.rt.ctx.data, self.rt.dot
        return ((data.NN + dot(data.N[c], data.N[d])) * self.rt.Q5(96, -32)
                - 2 * data.KAPPA * data.KAPPA).sign() >= 0


def word_shape(word, family):
    require(isinstance(word, list) and all(isinstance(t, list) and len(t) == 2
            and type(t[0]) is int and 0 <= t[0] < 600
            and type(t[1]) is int and 1 <= t[1] < 60 for t in word), 'literal word')
    require(family in ('SINGLE', 'CL2', 'PEEL'), 'family')
    require(len(word) == (2 if family == 'CL2' else 1), 'family word length')
    if family == 'SINGLE':
        require(word[0][1] >= 12, 'SINGLE item')
    if family == 'CL2':
        require(word[0][0] != word[1][0] and word[1][1] >= 12, 'CL2 word shape')


def contract(before, prefixes, word, family):
    """Only the new height/progress contract. In particular no pose or u guards."""
    word_shape(word, family)
    require(len(prefixes) == len(word), 'prefix count')
    h = maximum(before['heights'])
    require(h in (4, 6), 'step input maximum')
    require(all(maximum(p['heights']) <= h for p in prefixes), 'prefix maximum')
    end = prefixes[-1]
    if family in ('SINGLE', 'CL2'):
        require(qvalue(end['heights']) < qvalue(before['heights']), 'Q descent')
    else:
        require(all(end['heights'].get(str(k), 0) == before['heights'].get(str(k), 0)
                    for k in (6, 4)), 'PEEL counts')
        require(end['Bk'] < before['Bk'], 'PEEL global blocked count')
    require(601 * qvalue(end['heights']) + end['Bk'] <
            601 * qvalue(before['heights']) + before['Bk'], 'M descent')


def same(actual, reported, field):
    # Canonical equality also distinguishes bool from int and missing fields.
    require(canonical(actual) == canonical(reported), field)


def replay_step(engine, st, row, initial_lattice, ever, index, letters):
    require(row.get('type') == 'STEP' and row.get('rule') == RULE, 'STEP rule')
    word, family = row.get('word'), row.get('family')
    word_shape(word, family)
    same(index, row.get('index'), 'step index')
    same(len(word), row.get('letters'), 'letters')
    same(letters + len(word), row.get('cumulative_letters'), 'cumulative letters')
    before, old_ever = engine.snapshot(st, initial_lattice, ever)
    same(before, row.get('before'), 'before measurements')
    require(old_ever == ever, 'boundary loss tracking')
    if family == 'CL2':
        require(engine.meets(word[0][0], word[1][0]), 'CL2 caps do not meet')
    trial, prefixes = engine.clone(st), []
    trial_ever = set(ever)
    for c, j in word:
        engine.apply(trial, c, j)
        snap, trial_ever = engine.snapshot(trial, initial_lattice, trial_ever)
        prefixes.append(snap)
    # Initial S_d containment follows from exact applicability at both letters:
    # an unmoved straddler would still block d. No extra search prune is trusted.
    contract(before, prefixes, word, family)
    same(prefixes, row.get('prefixes'), 'prefix measurements')
    same(prefixes[-1], row.get('after'), 'after measurements')
    same(before['digest'], row.get('input_digest'), 'input digest')
    same([p['digest'] for p in prefixes], row.get('prefix_digests'), 'prefix digests')
    same(prefixes[-1]['digest'], row.get('output_digest'), 'output digest')
    return trial, trial_ever


def read_events(path, allow_pending=False):
    raw = Path(path).read_bytes()
    require(bool(raw) and raw.endswith(b'\n'), 'truncated JSONL')
    rows = [parse(line) for line in raw.splitlines()]
    require(all(isinstance(r, dict) for r in rows), 'event object')
    require(rows[0].get('type') == 'START', 'missing START')
    require(rows[-1].get('type') == 'FINAL', 'missing FINAL')
    require(all(r.get('type') in ('STEP', 'PROBE') for r in rows[1:-1]), 'event order')
    verification = rows[-1].get('verification', {})
    require(allow_pending or verification.get('status') == 'complete', 'verification incomplete')
    return rows


def check_ledger(path, rows):
    """Validate ledger integrity and each boundary summary, not search decisions."""
    summaries = {}
    expected_step = 1
    h, count, counts, complete, frontier = hashlib.sha256(), 0, {}, [], None
    with Path(path).open(encoding='utf-8') as f:
        for line in f:
            require(line.endswith('\n'), 'truncated search ledger')
            entry = parse(line)
            step = entry['step']
            if step != expected_step:
                require(step == expected_step + 1, 'ledger step order')
                summaries[expected_step] = (h.hexdigest(), count, counts, complete, frontier)
                expected_step = step
                h, count, counts, complete = hashlib.sha256(), 0, {}, []
            require(entry['sequence'] == count + 1, 'ledger sequence')
            h.update((canonical(entry) + '\n').encode())
            count += 1
            key = entry['at']['family'] + ':' + entry['outcome']
            counts[key] = counts.get(key, 0) + 1
            if entry['outcome'] == 'exhausted':
                complete.append(entry['at']['family'])
            frontier = dict(entry['at'], status='completed')
    summaries[expected_step] = (h.hexdigest(), count, counts, complete, frontier)
    for row in rows:
        if row['type'] not in ('STEP', 'FINAL') or row.get('search') is None:
            continue
        s = row['search']
        # A fresh HANDOFF/step-limit boundary has no enumeration ledger.
        got = summaries.get(s['step'], (sha(b''), 0, {}, [], None))
        same(list(got[:4]), [s['sha256'], s['entries'], s['counts'], s['complete_families']],
             'ledger summary')
        if s['frontier'] is not None and s['frontier'].get('status') == 'completed':
            same(got[4], s['frontier'], 'ledger frontier')


def verify_output(output, rt, tick=lambda: None, allow_pending=False, check_stream=None):
    """Reload the pinned table and replay the entire accepted suffix independently."""
    output = external_directory(output)
    manifest = parse((output / 'manifest.json').read_bytes())
    require(manifest.get('format') == FORMAT and manifest.get('rule') == RULE, 'manifest format')
    case = manifest.get('case')
    require(type(case) is int and case in CASES, 'case')
    same(input_identity(case), manifest.get('input'), 'manifest input identity')
    same(rt.identity(), manifest.get('identity'), 'source/model/menu identity')
    rows = read_events(output / 'events.jsonl', allow_pending)
    engine = ReplayEngine(rt, tick)
    st = load_case(case, rt, engine)
    initial_lattice = set(range(P)) - engine.off(st)
    ever, steps, letters = set(), 0, 0
    first, ever = engine.snapshot(st, initial_lattice, ever)
    same(first, rows[0].get('initial'), 'START measurements')
    same(input_identity(case), rows[0].get('input'), 'START input identity')
    same(FORMAT, rows[0].get('format'), 'START format')
    same(RULE, rows[0].get('rule'), 'START rule')
    probes = []
    for row in rows[1:-1]:
        tick()
        if row['type'] == 'PROBE':
            require(steps == 0, 'probe after trajectory')
            k = len(probes)
            require(k < len(PROBES.get(case, [])), 'extra PROBE')
            word, n4, hist, digest = PROBES[case][k]
            same(word, row.get('word'), 'PROBE word')
            require(row.get('excluded_from_trajectory') is True, 'PROBE totals')
            trial, probe_ever, prefixes = engine.clone(st), set(), []
            for c, j in word:
                engine.apply(trial, c, j)
                snap, probe_ever = engine.snapshot(trial, initial_lattice, probe_ever)
                prefixes.append(snap)
            same(first, row.get('before'), 'PROBE before')
            same(prefixes, row.get('prefixes'), 'PROBE prefixes')
            require(prefixes[0]['heights'].get('4', 0) == n4 and
                    prefixes[-1]['heights'] == hist and prefixes[-1]['digest'].startswith(digest),
                    'PROBE known witness')
            require(all(p['maximum'] <= 4 for p in prefixes), 'PROBE maximum')
            if case == 7304:
                before_blocked, after_blocked = engine.blocked(st), engine.blocked(trial)
                require(len(before_blocked) == 559 and len(after_blocked) == 556 and
                        before_blocked - after_blocked == {61, 112, 184} and
                        not after_blocked - before_blocked, 'PROBE global preparation')
            probes.append(word)
        else:
            require(len(probes) == len(PROBES.get(case, [])), 'missing diagnostic probes')
            st, ever = replay_step(engine, st, row, initial_lattice, ever, steps + 1, letters)
            steps += 1
            letters += len(row['word'])
        if check_stream is not None:
            emit(check_stream, {'type': 'REPLAY', 'event': row['type'], 'steps': steps,
                                'letters': letters, 'passed': True})
    final = rows[-1]
    snap, ever = engine.snapshot(st, initial_lattice, ever)
    same(snap, final.get('last'), 'FINAL measurements')
    same(steps, final.get('steps'), 'FINAL steps')
    same(letters, final.get('letters'), 'FINAL letters')
    result = final.get('result')
    require(result in ('HANDOFF', 'STUCK', 'BUDGET', 'ERROR'), 'FINAL result')
    if result in ('HANDOFF', 'STUCK'):
        require(len(probes) == len(PROBES.get(case, [])), 'incomplete diagnostics')
    if result == 'HANDOFF':
        require(handoff(snap['heights']), 'false HANDOFF')
    if result == 'STUCK':
        require(not handoff(snap['heights']) and maximum(snap['heights']) in (4, 6), 'false STUCK state')
        same(['SINGLE', 'CL2', 'PEEL'], final.get('search', {}).get('complete_families'),
             'STUCK without complete families')
    check_ledger(output / 'search.jsonl', rows)
    table = load_table(output / 'final-table.json', rt, expected_digest=snap['digest'])
    same(engine.histogram(table), snap['heights'], 'final table histogram')
    tick()
    return {'status': 'complete', 'passed': result != 'ERROR', 'steps': steps, 'letters': letters,
            'digest': snap['digest'], 'heights': snap['heights'],
            'handoff': handoff(snap['heights']),
            'scope': 'literal replay and ledger integrity; ordering/exhaustion not replay-proved'}


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--out', required=True)
    ap.add_argument('--cpu-seconds', type=float, default=3300)
    args = ap.parse_args()
    limits = Limits(args.cpu_seconds)
    output = external_directory(args.out)
    # An independent later invocation never overwrites the experiment's report.
    with (output / 'recheck.jsonl').open('x', encoding='utf-8') as stream:
        try:
            rt = Runtime(output)
            report = verify_output(output, rt, limits.check, check_stream=stream)
        except Budget as exc:
            report = {'status': 'incomplete', 'passed': False, 'reason': str(exc)}
        except (ValueError, KeyError, TypeError, OSError, AssertionError) as exc:
            report = {'status': 'error', 'passed': False, 'reason': str(exc)}
        emit(stream, dict(type='CHECK', **report, **limits.times()))
    print(canonical(report), flush=True)
    return 0 if report['status'] == 'complete' and report['passed'] else 1


if __name__ == '__main__':
    sys.exit(main())
