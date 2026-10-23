"""Independent, current-table-only replay of HD2P/1 experiment records.

Legacy: python3 -I -B check_height_descent_experiment.py --out RUN.
Segments: --chain SEARCH/manifest.json --out CAMPAIGN; add --resume-check
CHECK/manifest.json to resume unfinished verification, and --audit to enumerate
a candidate terminal STUCK independently. Full repository-root commands are in
height_descent_experiment.py's module docstring. No search module or old pose/u
contract is imported. Literal replay does not establish historical first choice;
only a completed separate audit establishes terminal exhaustion.
All geometry imports are lazy, after thread settings, cache routing and budgets.

Version 3 resume trust boundary. Hashes authenticate committed bytes, not their
author. The integrator retains an immutable, reviewed chain produced by these
pinned programs; a party able to replace every manifest and rehash the history
can forge self-certified receipts. authenticate_checker is NOT a proof checker
for untrusted receipts. Rebuild from the pinned input if that trust is absent.
Every finalized search segment has validated its predecessor's newest units;
its start equals that predecessor's authenticated end, and both manifests and
all artifacts are hash-linked. Inductively only the head's units remain to be
validated. A checker receipt records that head, so a retry need only hash those
committed bytes. Checker attempts use the same induction for prefix/audit/check
receipts: the next attempt validates the preceding attempt's new receipts once.
All ancestor bytes, links, interval boundaries, identities and budgets are still
checked on every resume. Missing, reordered, duplicate or skipped segments fail.
This trusts execution of the pinned finalizer, never cached geometry or rejected
search candidates. Accounting assumes ONE campaign output root per table and
retention of every allocation.json, including reservations of killed attempts.

Audit exactness. The independent audit classifies actual simulator states. At a
clean cap, a twist changes only inside poses: subtract each inside multiplicity
at its old height and add it at dist(l_twist * l_pose). Right factors fix the
root for I_a; lattice poses therefore contribute dist(l_twist), including all
lattice collateral. Retained twists preserve every height. Endpoint histograms
need no State.apply; openers and count-preserving PEEL trials still use it.
A twist fixes its pole and keeps the moved regions inside that cap. A cap that
does not meet it in the containing ball cannot change classification. The exact
>= meeting predicate includes tangency. PEEL therefore recounts all meeting caps
and retains all other statuses, giving the same GLOBAL Bk as a 600-cap recount.
Consecutive literal-replay snapshots use that same locality fact, reclassifying
the union of caps meeting any intervening twist. The first snapshot in a process
always recounts all 600; no persisted classification cache is trusted. Replay
states are mutated only through ReplayEngine.apply, which invalidates its cached
count before mutation. Every other snapshot field is still measured from the
actual state. A failed/uncertain application cannot publish a cached count.
An original straddling witness outside the opener's inside set is unchanged and
still blocks its target. This independent certificate avoids only impossible
targets; every survivor gets an actual intermediate-state classification. No
search classification, prediction, ledger or rejection is used by the audit.
"""

import argparse
import copy
from datetime import datetime, timezone
from functools import lru_cache
import io
import re
import tempfile
import uuid
import hashlib
import json
import math
import os
from pathlib import Path
import resource
import signal
import sys
import time

PROCESS_WALL_START = time.monotonic()
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


class ReplayFailure(ValueError):
    """An invalid accepted interval, with its first affected step preserved."""
    def __init__(self, reason, step):
        super().__init__(reason)
        self.step = step


RECORD_ERRORS = (ValueError, KeyError, TypeError, IndexError, AttributeError, AssertionError, OSError)


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
        self.snapshot_cpu, self.snapshot_count = 0., 0
        self._blocked_cache = (None, None)
        self._blocked_pending = (None, None, ())
        self._affected_caps = {}

    def blocked_basis(self, st):
        if self._blocked_cache[0] is st:
            return self._blocked_cache[1], ()
        if self._blocked_pending[0] is st:
            return self._blocked_pending[1:]
        return None, ()

    def clone(self, st):
        out = self.rt.sim.State(self.rt.ctx, menu=self.rt.menu, filtered=False)
        out.pose_id, out._poses, out._lookup = st.pose_id.copy(), list(st._poses), dict(st._lookup)
        blocked, changed = self.blocked_basis(st)
        self._blocked_pending = (out, blocked, changed)
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
        baseline, changed = self.blocked_basis(st)
        blocked = set() if baseline is None else set(baseline)
        if baseline is None:
            caps = range(600)
        else:
            caps = set()
            for c in changed:
                if c not in self._affected_caps:
                    self._affected_caps[c] = frozenset(d for d in range(600) if self.meets(c, d))
                caps.update(self._affected_caps[c])
        for c in caps:
            self.tick()
            cl = st.classify(c, need_inside=False, stop_at_straddle=True)
            require(not cl.uncertain, 'uncertain cap classification')
            if cl.straddle is not None:
                blocked.add(c)
            else:
                blocked.discard(c)
        self._blocked_cache = (st, frozenset(blocked))
        self._blocked_pending = (None, None, ())
        return blocked

    def snapshot(self, st, initial_lattice, ever):
        started = time.process_time()
        self.tick()
        hist, off = self.histogram(st), self.off(st)
        loss = initial_lattice & off
        ever = ever | loss
        bk = len(self.blocked(st))
        self.tick()
        n = int(self.rt.rim.Domains(st).total_misalignment())
        self.tick()
        q = qvalue(hist)
        snap = {'digest': st.digest(), 'heights': hist, 'maximum': maximum(hist),
                 'n2_n4_n6': [hist.get(str(h), 0) for h in (2, 4, 6)],
                 'Q': q, 'Bk': bk, 'M': 601 * q + bk, 'N': n, 'off': len(off),
                 'lattice_loss': len(loss), 'ever_lattice_loss': len(ever)}
        self.snapshot_cpu += time.process_time() - started
        self.snapshot_count += 1
        return snap, ever

    def apply(self, st, c, j):
        self.tick()
        baseline, changed = self.blocked_basis(st)
        # Invalidate BEFORE mutation: even an asynchronous interruption after
        # State.apply commits must never expose the preceding state's count.
        if self._blocked_cache[0] is st:
            self._blocked_cache = (None, None)
        if self._blocked_pending[0] is st:
            self._blocked_pending = (None, None, ())
        tw = self.rt.sim.Twist(self.rt.ctx, c, self.rt.restore.element(self.rt.menu, c, j),
                               'menu', {'menu': self.rt.menu.name})
        outcome = st.apply(tw, journal=False)
        require(outcome.applied and not outcome.uncertain, 'inadmissible or uncertain letter')
        if baseline is not None:
            self._blocked_pending = (st, baseline, tuple(sorted(set(changed) | {c})))
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


def replay_step(engine, st, row, initial_lattice, ever, index, letters, before_snapshot=None):
    require(row.get('type') == 'STEP' and row.get('rule') == RULE, 'STEP rule')
    word, family = row.get('word'), row.get('family')
    word_shape(word, family)
    same(index, row.get('index'), 'step index')
    same(len(word), row.get('letters'), 'letters')
    same(letters + len(word), row.get('cumulative_letters'), 'cumulative letters')
    before, old_ever = (engine.snapshot(st, initial_lattice, ever) if before_snapshot is None
                        else (before_snapshot, ever))
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
    probes, last = [], first
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
            st, ever = replay_step(engine, st, row, initial_lattice, ever, steps + 1, letters, last)
            last = row['after']  # Compared to the independently computed prefix above.
            steps += 1
            letters += len(row['word'])
        if check_stream is not None:
            emit(check_stream, {'type': 'REPLAY', 'event': row['type'], 'steps': steps,
                                'letters': letters, 'passed': True})
    final = rows[-1]
    snap = last
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
            'handoff': handoff(snap['heights']), 'exhaustion_verification': 'not_performed',
            'scope': 'literal replay and ledger integrity; ordering/exhaustion not replay-proved'}


def legacy_main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--out', required=True)
    ap.add_argument('--cpu-seconds', type=float, default=3300)
    args = ap.parse_args()
    limits = Limits(args.cpu_seconds)
    output = external_directory(args.out)
    # An independent later invocation never overwrites the experiment's report.
    with (output / ('recheck-' + uuid.uuid4().hex + '.jsonl')).open('x', encoding='utf-8') as stream:
        try:
            rt = Runtime(output)
            report = verify_output(output, rt, limits.check, allow_pending=True, check_stream=stream)
        except Budget as exc:
            report = {'status': 'incomplete', 'passed': False, 'reason': str(exc)}
        except (ValueError, KeyError, TypeError, OSError, AssertionError) as exc:
            report = {'status': 'error', 'passed': False, 'reason': str(exc)}
        emit(stream, dict(type='CHECK', **report, **limits.times()))
    print(canonical(report), flush=True)
    return 0 if report['status'] == 'complete' and report['passed'] else 1


# Versioned transport and coverage protocol. This section contains no search
# mechanics. The terminal enumerator below is independently written.
SEGMENT = 'height-descent-segment/3'
CHECK = 'height-descent-check/3'
CHAIN = 'hd2p-logical-units/1'
ORDER = 'SINGLE(c,y);CL2(c,x,d,min(Q,y));PEEL(c,x)/1'
ZERO_HASH = '0' * 64
CHUNK_BYTES = 4_000_000
MAX_FILE = 5_000_000


def schema(obj, fields, reason):
    require(type(obj) is dict and set(obj) == fields, reason)


def digest_json(value):
    return sha(canonical(value).encode())


def bitset(pieces):
    bits = bytearray((P + 7) // 8)
    for p in pieces:
        require(type(p) is int and 0 <= p < P, 'loss bitset index')
        bits[p // 8] |= 1 << (p % 8)
    return bits.hex()


def unbitset(encoded):
    require(isinstance(encoded, str), 'loss bitset encoding')
    return set(_decode_bitset(encoded))


@lru_cache(maxsize=16)
def _decode_bitset(encoded):
    require(isinstance(encoded, str) and len(encoded) == 2 * ((P + 7) // 8)
            and re.fullmatch('[0-9a-f]*', encoded) is not None, 'loss bitset encoding')
    raw = bytes.fromhex(encoded)
    result = {8 * i + b for i, byte in enumerate(raw) if byte
              for b in range(8) if byte & (1 << b)}
    same(bitset(result), encoded, 'loss bitset padding')
    return frozenset(result)


def safe_relative(name):
    require(isinstance(name, str) and bool(name) and '\\' not in name and ':' not in name,
            'portable artifact path')
    p = Path(name)
    require(not p.is_absolute() and all(t not in ('', '.', '..') for t in name.split('/')),
            'artifact path traversal')
    return p


def confined(root, name):
    p = root / safe_relative(name)
    require(not any(q.is_symlink() for q in [p, *p.parents] if q != root.parent), 'artifact symlink')
    require(p.resolve().is_relative_to(root.resolve()), 'artifact escaped root')
    return p


def pack_artifact(directory, name, source):
    """Chunk raw bytes, including JSONL, without assuming a record fits a chunk."""
    safe_relative(name)
    require('/' not in name, 'artifact name')
    if isinstance(source, bytes):
        source = io.BytesIO(source)
    total, h, chunks = 0, hashlib.sha256(), []
    while True:
        block = source.read(CHUNK_BYTES)
        if not block:
            break
        filename = f'{name}.{len(chunks):05d}.chunk'
        with (directory / filename).open('xb') as f:
            f.write(block)
            f.flush()
            os.fsync(f.fileno())
        chunks.append(dict(path=filename, size=len(block), sha256=sha(block)))
        h.update(block)
        total += len(block)
    return dict(size=total, sha256=h.hexdigest(), chunks=chunks)


def artifact_blocks(directory, descriptor):
    schema(descriptor, {'size', 'sha256', 'chunks'}, 'artifact schema')
    require(type(descriptor['size']) is int and descriptor['size'] >= 0, 'artifact size')
    total, h, seen = 0, hashlib.sha256(), set()
    for chunk in descriptor['chunks']:
        schema(chunk, {'path', 'size', 'sha256'}, 'chunk schema')
        require(chunk['path'] not in seen, 'duplicate chunk')
        seen.add(chunk['path'])
        require(type(chunk['size']) is int and 0 < chunk['size'] < MAX_FILE, 'chunk size')
        path = confined(directory, chunk['path'])
        require(path.stat().st_size == chunk['size'], 'truncated artifact')
        block = path.read_bytes()
        same(sha(block), chunk['sha256'], 'artifact hash')
        h.update(block)
        total += len(block)
        yield block
    same(total, descriptor['size'], 'artifact total size')
    same(h.hexdigest(), descriptor['sha256'], 'artifact total hash')


def artifact_bytes(directory, descriptor):
    return b''.join(artifact_blocks(directory, descriptor))


def artifact_lines(directory, descriptor):
    pending = b''
    for block in artifact_blocks(directory, descriptor):
        lines = (pending + block).split(b'\n')
        pending = lines.pop()
        for line in lines:
            require(bool(line), 'empty JSONL entry')
            yield parse(line)
    require(not pending, 'truncated JSONL artifact')


def atomic_manifest(directory, manifest, interrupt=lambda _: None):
    """Only this rename publishes a segment. Partial directories are disposable."""
    raw = (canonical(manifest) + '\n').encode()
    require(len(raw) < MAX_FILE, 'manifest size')
    require(not (directory / 'manifest.json').exists(), 'immutable manifest exists')
    interrupt('before_manifest')
    with (directory / 'manifest.pending').open('xb') as f:
        f.write(raw)
        f.flush()
        os.fsync(f.fileno())
    interrupt('before_finalize')
    (directory / 'manifest.pending').replace(directory / 'manifest.json')
    fd = os.open(directory, os.O_RDONLY)
    try:
        os.fsync(fd)
    finally:
        os.close(fd)
    return directory / 'manifest.json'


def portable_command(mode, case=None):
    script = 'research/jumbling/restoration/13-tutorial-runner/scripts/'
    if mode in ('checker', 'audit'):
        result = ['python3', '-I', '-B', script + 'check_height_descent_experiment.py',
                  '--chain', '../campaign/SEARCH/manifest.json', '--out', '../campaign']
        if mode == 'audit':
            result += ['--audit', '--resume-check', '../campaign/CHECK/manifest.json']
    else:
        result = ['python3', '-I', '-B', script + 'height_descent_experiment.py',
                  '--segment', '--case', str(case), '--out', '../campaign']
        if mode == 'diagnostics':
            result.append('--diagnostics-only')
        if mode == 'qualification':
            result += ['--qualify-ledger', '../first-run/search.jsonl', '--units', '1000']
    return result


def recorded_command(manifest):
    cmd = portable_command(manifest['kind'], manifest['input']['case'])
    previous = manifest['predecessor']
    if manifest['kind'] in ('checker', 'audit'):
        cmd[cmd.index('--chain') + 1] = '../campaign/' + manifest['target']['path']
        if '--resume-check' in cmd:
            i = cmd.index('--resume-check')
            del cmd[i:i + 2]
        if previous:
            cmd += ['--resume-check', '../campaign/' + previous['path']]
    elif previous:
        cmd += ['--resume', '../campaign/' + previous['path']]
    if manifest['kind'] == 'qualification':
        cmd[cmd.index('--units') + 1] = str(manifest['qualification']['requested_units'])
    cmd += ['--not-after', datetime.fromtimestamp(manifest['not_after'], timezone.utc).isoformat()]
    for option, value in sorted(manifest['budget'].get('limits', {}).items()):
        cmd += ['--' + option.replace('_', '-'), str(value)]
    return cmd


class SegmentLimits:
    """One process, with setup charged and a distinct trajectory CPU origin.

    Useful work stops at 3000 absolute CPU seconds (or earlier). 300 seconds
    remain to publish. A forced kill retains only the preceding manifest.
    Tests inject clocks and disable OS timers; CLI callers always arm them.
    """
    def __init__(self, used=0, ceiling=19800, total=3300, useful=3000, wall=3600,
                 not_after=None, arm=True, cpu=time.process_time, monotonic=time.monotonic,
                 utc=time.time):
        require(2 <= total <= 3300 and 0 <= useful <= min(3000, total - 1), 'segment CPU envelope')
        require(1 <= wall <= 3600 and 0 <= used <= ceiling, 'cumulative budget')
        self.cpu, self.clock, self.utc = cpu, monotonic, utc
        self.start, self.used, self.ceiling = (PROCESS_WALL_START if arm else monotonic()), used, ceiling
        self.total, self.useful, self.wall = total, useful, wall
        self.not_after = not_after if not_after is not None else utc() + 86400
        require(math.isfinite(self.not_after) and self.not_after <= utc() + 86400, '24-hour campaign envelope')
        self.work_start = None
        if arm:
            hard = math.ceil(min(total, ceiling - used))
            require(hard > cpu() + 1, 'no remaining process allowance')
            old = resource.getrlimit(resource.RLIMIT_CPU)[1]
            if old != resource.RLIM_INFINITY:
                hard = min(hard, old)
            resource.setrlimit(resource.RLIMIT_CPU, (hard, hard))
            signal.signal(signal.SIGPROF, self._expired)
            signal.signal(signal.SIGALRM, self._expired)
            signal.setitimer(signal.ITIMER_PROF, max(.001, min(useful, hard - 1) - cpu()))
            signal.setitimer(signal.ITIMER_REAL, max(.001, min(wall - 300, self.not_after - utc() - 300)))
        self.armed = arm

    @staticmethod
    def _expired(*_):
        raise Budget('segment useful-work allowance')

    def boundary(self):
        self.work_start = self.cpu()
        self.check()

    def check(self):
        if (self.cpu() >= min(self.useful, self.ceiling - self.used - 300)
                or self.clock() - self.start >= self.wall - 300
                or self.utc() >= self.not_after - 300):
            raise Budget('segment/cumulative/campaign useful-work allowance')

    def finalizing(self):
        if self.armed:
            signal.setitimer(signal.ITIMER_PROF, 0)
            signal.setitimer(signal.ITIMER_REAL, 0)
            signal.setitimer(signal.ITIMER_PROF, max(.001, min(self.total, self.ceiling - self.used)
                                                     - self.cpu() - .5))
            signal.setitimer(signal.ITIMER_REAL, max(.001, min(self.wall - (self.clock() - self.start),
                                                              self.not_after - self.utc())))

    def times(self):
        return dict(cpu_seconds=self.cpu(), wall_seconds=self.clock() - self.start,
                    trajectory_cpu_seconds=0 if self.work_start is None else self.cpu() - self.work_start,
                    setup_cpu_seconds=self.cpu() if self.work_start is None else self.work_start)


def utc_deadline(value):
    if value is None:
        return None
    dt = datetime.fromisoformat(value.replace('Z', '+00:00'))
    require(dt.utcoffset() is not None and dt.utcoffset().total_seconds() == 0, 'not-after must be UTC')
    return dt.timestamp()


def search_cursor(digest, step=1):
    return dict(version='hd2p-cursor/1', digest=digest, step=step, phase='histogram',
                hist=None, classes=[], ci=0, xi=0, di=0, targets=[], filtered=False, middle=None)


def cursor_hash(cursor):
    return digest_json(cursor)


def step_schema(row, deciding_cursor=None):
    required = {'type', 'rule', 'index', 'family', 'word', 'letters', 'cumulative_letters',
                'input_digest', 'prefix_digests', 'output_digest', 'before', 'prefixes', 'after', 'search'}
    timing = {'cpu_seconds', 'wall_seconds', 'setup_cpu_seconds', 'trajectory_cpu_seconds'}
    require(type(row) is dict and required <= set(row) <= required | timing, 'STEP schema')
    same([row['type'], row['rule']], ['STEP', RULE], 'STEP type/rule')
    word_shape(row['word'], row['family'])
    n = len(row['word'])
    require(type(row['prefixes']) is list and len(row['prefixes']) == n, 'STEP prefix count')
    require(type(row['prefix_digests']) is list and len(row['prefix_digests']) == n,
            'STEP prefix digest count')
    for snapshot in [row['before'], *row['prefixes'], row['after']]:
        schema(snapshot, {'digest', 'heights', 'maximum', 'n2_n4_n6', 'Q', 'Bk', 'M', 'N',
                          'off', 'lattice_loss', 'ever_lattice_loss'}, 'STEP measurement schema')
    same(row['prefix_digests'], [p['digest'] for p in row['prefixes']], 'STEP prefix digests')
    for h in [row['input_digest'], *row['prefix_digests'], row['output_digest']]:
        require(type(h) is str and re.fullmatch('[0-9a-f]{64}', h) is not None, 'STEP digest format')
    schema(row['search'], {'format', 'deciding_cursor'}, 'STEP search schema')
    same(row['search']['format'], CHAIN, 'STEP search version')
    h = row['search']['deciding_cursor']
    require(type(h) is str and re.fullmatch('[0-9a-f]{64}', h) is not None, 'STEP deciding cursor')
    if deciding_cursor is not None:
        same(h, cursor_hash(deciding_cursor), 'STEP deciding cursor binding')


def advance_cursor(cursor, events, row=None, caps=tuple(range(600)),
                   items=tuple(range(1, 60)), jumble=tuple(range(12, 60))):
    """Validate interval coverage, including every endpoint, independently of hashes.

    Outcomes are claims, not geometry proofs. The separate audit checks all
    rejection claims on the terminal state. This reducer is a file protocol,
    shared with the writer; it never classifies, predicts or selects a move.
    """
    # Entries and histograms are immutable; only these two lists are appended.
    cur = dict(cursor, classes=list(cursor['classes']), targets=list(cursor['targets']))
    require(cur['version'] == 'hd2p-cursor/1', 'cursor version')
    clean = [caps[i] for i, cl in enumerate(cur['classes']) if cl['clean']]
    phase, hist = cur['phase'], cur['hist']
    pos = 0

    def take(family, phase_name, outcomes, **at):
        nonlocal pos
        require(pos < len(events), 'missing candidate coverage')
        e = events[pos]
        pos += 1
        schema(e, {'at', 'outcome', 'detail'}, 'unit event schema')
        same(e['at'], dict(family=family, phase=phase_name, **at), 'candidate coordinates/order')
        require(e['outcome'] in outcomes, 'candidate outcome')
        return e

    def next_single():
        cur['xi'] += 1
        if cur['xi'] == len(jumble):
            cur['ci'], cur['xi'] = cur['ci'] + 1, 0
        if cur['ci'] == len(clean):
            cur['phase'] = 'single_end'

    def next_opener():
        cur['xi'] += 1
        cur['di'], cur['middle'] = 0, None
        if cur['xi'] == len(items):
            cur['ci'], cur['xi'] = cur['ci'] + 1, 0
            cur['filtered'], cur['targets'] = False, []
        cur['phase'] = 'cl2_end' if cur['ci'] == len(clean) else 'opener'

    def next_group():
        cur['di'] += 1
        if cur['di'] == len(cur['targets']):
            next_opener()

    chosen, predicted = None, None
    if phase == 'histogram':
        e = take('INPUT', 'histogram', ('measured',))
        cur['hist'] = e['detail']['heights']
        require(handoff(cur['hist']) or maximum(cur['hist']) in (4, 6), 'input height bound')
        cur['phase'] = 'handoff' if handoff(cur['hist']) else 'input'
    elif phase == 'input':
        c = caps[len(cur['classes'])]
        e = take('INPUT', 'classify', ('clean', 'blocked'), c=c)
        b = e['detail']['blocker']
        require(b is None if e['outcome'] == 'clean' else type(b) is int and 0 <= b < P,
                'classification certificate')
        cur['classes'].append(dict(clean=e['outcome'] == 'clean', blocker=b))
        if len(cur['classes']) == len(caps):
            cur['phase'] = 'single' if any(x['clean'] for x in cur['classes']) else 'single_end'
    elif phase == 'single':
        c, x = clean[cur['ci']], jumble[cur['xi']]
        e = take('SINGLE', 'candidate', ('inadmissible', 'height_prune', 'no_descent', 'lowering'), c=c, x=x)
        end = e['detail'].get('heights')
        want = ('inadmissible' if end is None else 'height_prune' if maximum(end) > maximum(hist)
                else 'no_descent' if qvalue(end) >= qvalue(hist) else 'lowering')
        same(e['outcome'], want, 'SINGLE outcome/height')
        if want == 'lowering':
            chosen, predicted = ('SINGLE', [[c, x]]), [end]
        else:
            next_single()
    elif phase in ('single_end', 'cl2_end', 'peel_end'):
        family = {'single_end': 'SINGLE', 'cl2_end': 'CL2', 'peel_end': 'PEEL'}[phase]
        take(family, 'end', ('exhausted',))
        cur.update(ci=0, xi=0, di=0)
        cur['phase'] = {'single_end': 'opener' if clean else 'cl2_end',
                        'cl2_end': 'peel' if clean else 'peel_end', 'peel_end': 'stuck'}[phase]
    elif phase == 'opener':
        c, x = clean[cur['ci']], items[cur['xi']]
        e = take('CL2', 'opener', ('opener_prune', 'opener_inadmissible', 'opener_survives'), c=c, x=x)
        middle = e['detail'].get('heights')
        if e['outcome'] != 'opener_survives':
            if e['outcome'] == 'opener_prune':
                require(middle is None or maximum(middle) > maximum(hist), 'opener prune')
            next_opener()
        else:
            require(middle is not None and maximum(middle) <= maximum(hist), 'opener bound')
            cur['middle'], cur['di'] = middle, 0
            if not cur['filtered']:
                cur['phase'] = 'filter'
            elif cur['targets']:
                cur['phase'] = 'group'
            else:
                next_opener()
    elif phase == 'filter':
        c, d = clean[cur['ci']], caps[cur['di']]
        e = take('CL2', 'target_filter', ('same_cap', 'not_meeting', 'blocker_certificate',
                                        'blocker_containment', 'target_survives'), c=c, d=d)
        require((c == d) == (e['outcome'] == 'same_cap'), 'same cap filter')
        if e['outcome'] == 'target_survives':
            cur['targets'].append(d)
        cur['di'] += 1
        if cur['di'] == len(caps):
            cur.update(filtered=True, di=0, phase='group')
            if not cur['targets']:
                next_opener()
    elif phase == 'group':
        c, x, d = clean[cur['ci']], items[cur['xi']], cur['targets'][cur['di']]
        e = take('CL2', 'second_cap', ('intermediate_blocked', 'intermediate_clean'), c=c, x=x, d=d)
        best = None
        if e['outcome'] == 'intermediate_clean':
            for y in jumble:
                e = take('CL2', 'second_item', ('endpoint', 'endpoint_prune'), c=c, x=x, d=d, y=y)
                end = e['detail']['heights']
                good = end is not None and maximum(end) <= maximum(hist)
                same(e['outcome'], 'endpoint' if good else 'endpoint_prune', 'endpoint bound')
                if good:
                    score = qvalue(end), y
                    same(e['detail']['Q'], score[0], 'endpoint Q')
                    if best is None or score < best[0]:
                        best = score, end
            if best is not None and best[0][0] < qvalue(hist):
                chosen = 'CL2', [[c, x], [d, best[0][1]]]
                predicted = [cur['middle'], best[1]]
        if chosen is None:
            next_group()
    elif phase == 'peel':
        c, x = clean[cur['ci']], items[cur['xi']]
        e = take('PEEL', 'candidate', ('height_or_count_prune', 'inadmissible', 'global_blocked'), c=c, x=x)
        end = e['detail'].get('heights')
        if e['outcome'] == 'global_blocked':
            require(maximum(end) <= maximum(hist) and all(end.get(str(k), 0) == hist.get(str(k), 0)
                                                        for k in (6, 4)), 'PEEL counts')
            same(e['detail']['before'], len(caps) - len(clean), 'PEEL input Bk')
            if e['detail']['after'] < e['detail']['before']:
                chosen, predicted = ('PEEL', [[c, x]]), [end]
        cur['xi'] += 1
        if cur['xi'] == len(items):
            cur['ci'], cur['xi'] = cur['ci'] + 1, 0
        if cur['ci'] == len(clean):
            cur['phase'] = 'peel_end'
    else:
        raise ValueError('completed or invalid cursor')
    same(pos, len(events), 'duplicate candidate coverage')
    if chosen is not None:
        require(row is not None, 'unpublished STEP transaction')
        step_schema(row, cursor)
        same([row['family'], row['word']], list(chosen), 'selected word/minimum')
        same([p['heights'] for p in row['prefixes']], predicted, 'selected prefix histograms')
        same(row['input_digest'], cursor['digest'], 'STEP state binding')
        same(row['index'], cursor['step'], 'STEP global index')
        return search_cursor(row['output_digest'], cursor['step'] + 1)
    require(row is None, 'unsolicited STEP')
    return cur


def unit_record(ordinal, previous, before, after, events, row=None):
    body = dict(version=CHAIN, ordinal=ordinal, previous=previous, before=cursor_hash(before),
                after=cursor_hash(after), events=events, step=row)
    return dict(body, sha256=digest_json(body))


def validate_unit(unit, ordinal, previous, cursor, **domain):
    schema(unit, {'version', 'ordinal', 'previous', 'before', 'after', 'events', 'step', 'sha256'},
         'logical unit schema')
    same(unit['version'], CHAIN, 'logical unit version')
    same(unit['ordinal'], ordinal, 'overlapping or skipped interval')
    same(unit['previous'], previous, 'unit predecessor')
    same(unit['before'], cursor_hash(cursor), 'unit cursor boundary')
    same(unit['sha256'], digest_json({k: v for k, v in unit.items() if k != 'sha256'}), 'unit hash')
    end = advance_cursor(cursor, unit['events'], unit['step'], **domain)
    same(unit['after'], cursor_hash(end), 'unit end cursor')
    return end


def table_bytes(st, rt):
    return (canonical({'poses': [rt.matrix_json(st._pose_matrix(i)) for i in range(len(st._poses))],
                       'pose_id': st.pose_id.tolist(), 'digest': st.digest(), 'journal': []}) + '\n').encode()


def load_artifact_table(directory, descriptor, rt, digest, temporary):
    # Reuse complete table validation; the temporary file is never evidence.
    with tempfile.NamedTemporaryFile(dir=temporary, suffix='.json') as f:
        for block in artifact_blocks(directory, descriptor):
            f.write(block)
        f.flush()
        return load_table(f.name, rt, expected_digest=digest)


def segment_identity(rt):
    result = rt.identity()
    result['sources'][str((HERE / 'test_height_descent_segments.py').relative_to(ROOT))] = sha(
        (HERE / 'test_height_descent_segments.py').read_bytes())
    result['ordering'] = ORDER
    return result


def manifest_ref(path):
    path = Path(path)
    return dict(path=path.parent.name + '/manifest.json', sha256=sha(path.read_bytes()))


def new_output(root, kind):
    root = external_directory(root)
    root.mkdir(parents=True, exist_ok=True)
    directory = root / (kind + '-' + uuid.uuid4().hex)
    directory.mkdir()
    return directory


def publish(directory, kind, identity, pin, campaign, predecessor, start, end,
            artifacts, budget, deadline, status, extra=None, interrupt=lambda _: None):
    manifest = dict(format=CHECK if kind in ('checker', 'audit') else SEGMENT, rule=RULE,
                    kind=kind, campaign=campaign, segment=directory.name, predecessor=predecessor,
                    input=pin, identity=identity, start=start, end=end, artifacts=artifacts,
                    budget=budget, not_after=deadline, status=status,
                    verification='provisional' if kind not in ('checker', 'audit') else status,
                    command=portable_command(kind, pin['case']), private_files=[],
                    commit_files=['manifest.json'] + (['allocation.json'] if 'allocation' in artifacts else []) + [c['path'] for a in artifacts.values() for c in a['chunks']])
    manifest['commit_files'].sort()
    if extra:
        manifest.update(extra)
    manifest['command'] = recorded_command(manifest)
    cache = directory / 'home_centroids.npy'
    if cache.exists():
        manifest['private_files'].append(dict(path=cache.name, size=cache.stat().st_size,
                                              sha256=sha(cache.read_bytes())))
    return atomic_manifest(directory, manifest, interrupt)


def read_manifest(path, expected_format, identity=None):
    path = Path(path).resolve()
    raw = path.read_bytes()
    require(len(raw) < MAX_FILE and raw.endswith(b'\n'), 'manifest size/completion')
    m = parse(raw)
    required = {'format', 'rule', 'kind', 'campaign', 'segment', 'predecessor', 'input', 'identity',
                'start', 'end', 'artifacts', 'budget', 'not_after', 'status', 'verification',
                'command', 'private_files', 'commit_files'}
    require(required <= set(m) <= required | {'target', 'report', 'qualification', 'diagnostics'},
            'manifest schema')
    same(m['format'], expected_format, 'manifest version')
    same(m['rule'], RULE, 'rule identity')
    same(m['segment'], path.parent.name, 'segment identity')
    require(type(m['not_after']) in (int, float) and math.isfinite(m['not_after']), 'campaign deadline')
    pin = m['input']
    require(type(pin.get('case')) is int and pin['case'] in CASES, 'pinned case')
    same(pin, input_identity(pin['case']), 'input identity')
    if identity is not None:
        same(identity, m['identity'], 'source/model/menu/ordering identity')
    expected = ['manifest.json'] + (['allocation.json'] if 'allocation' in m['artifacts'] else []) + [c['path'] for a in m['artifacts'].values() for c in a['chunks']]
    expected.sort()
    same(expected, m['commit_files'], 'commit inventory')
    require(len(expected) == len(set(expected)), 'duplicate artifact file')
    for name in expected:
        safe_relative(name)
    for a in m['artifacts'].values():
        for _ in artifact_blocks(path.parent, a):
            pass
    if 'allocation' in m['artifacts']:
        same(parse(confined(path.parent, 'allocation.json').read_bytes()),
             parse(artifact_bytes(path.parent, m['artifacts']['allocation'])), 'budget reservation artifact')
    # Private files are intentionally optional and can be deleted between runs.
    for private in m['private_files']:
        schema(private, {'path', 'size', 'sha256'}, 'private inventory')
        safe_relative(private['path'])
    same(m['command'], recorded_command(m), 'portable command')
    return m


def manifest_chain(head, expected_format, identity=None):
    head = Path(head).resolve()
    root = external_directory(head.parent.parent)
    path, reverse, seen = head, [], set()
    while True:
        require(path not in seen, 'cyclic/duplicate predecessor')
        seen.add(path)
        m = read_manifest(path, expected_format, identity)
        reverse.append((path, m))
        ref = m['predecessor']
        if ref is None:
            break
        schema(ref, {'path', 'sha256'}, 'predecessor schema')
        path = confined(root, ref['path'])
        same(sha(path.read_bytes()), ref['sha256'], 'predecessor hash')
    chain = list(reversed(reverse))
    for (_, a), (_, b) in zip(chain, chain[1:]):
        for k in ('campaign', 'input', 'identity', 'not_after'):
            same(a[k], b[k], 'chain ' + k)
        same(a['end'], b['start'], 'chain state/cursor continuity')
    return chain


def search_chain(head, identity=None, *, validated=None, chain=None):
    """Authenticate every byte; reduce only units not certified by a successor.

    ``validated`` may only come from an authenticated checker checkpoint. The
    version-3 writer's predecessor validation supplies the induction for older
    search segments (see the module trust boundary).
    """
    chain = manifest_chain(head, SEGMENT, identity) if chain is None else chain
    refs = [manifest_ref(path) for path, _ in chain]
    require(validated is None or validated in refs, 'validated search is not an ancestor')
    pin = chain[0][1]['input']
    cursor, previous, ordinal, steps, letters = search_cursor(pin['digest']), ZERO_HASH, 0, 0, 0
    cpu, segments, diagnostics, qualification = 0, 0, None, None
    last = None
    for path, m in chain:
        require(m['kind'] in ('search', 'qualification', 'diagnostics'), 'search kind')
        cp = parse(artifact_bytes(path.parent, m['artifacts']['checkpoint']))
        schema(cp, {'cursor', 'last', 'ever', 'steps', 'letters', 'unit_count', 'unit_hash',
                       'diagnostics', 'qualification'}, 'checkpoint schema')
        require(cp['last']['ever_lattice_loss'] == len(unbitset(cp['ever'])), 'loss bitset cardinality')
        same(m['start']['cursor'], cursor_hash(cursor), 'starting search cursor')
        same(m['start']['units'], ordinal, 'starting interval')
        same(m['start']['unit_hash'], previous, 'starting chain hash')
        same(m['start']['digest'], cursor['digest'], 'starting accepted state')
        same(m['start']['steps'], steps, 'starting global steps')
        same(m['start']['letters'], letters, 'starting global letters')
        if ordinal == 0 and steps == 0:
            same(m['start']['ever'], bitset(set()), 'original loss set')
        if last is None:
            last = cp['last'] if cp['steps'] == 0 else None
        if path == chain[-1][0] and validated != refs[-1]:
            event_rows = iter(artifact_lines(path.parent, m['artifacts']['events']))
            heights = pin['heights'] if steps == 0 else last['heights']
            for unit in artifact_lines(path.parent, m['artifacts']['units']):
                failing_step = cursor['step']
                try:
                    ordinal += 1
                    if cursor['phase'] == 'histogram':
                        same(unit['events'][0]['detail']['heights'], heights,
                             'histogram unit/accepted measurements')
                    cursor = validate_unit(unit, ordinal, previous, cursor)
                    previous = unit['sha256']
                    if unit['step'] is not None:
                        row = unit['step']
                        steps += 1
                        letters += len(row['word'])
                        same(row['cumulative_letters'], letters, 'cumulative letters')
                        same(next(event_rows, None), row, 'events/unit STEP equality')
                        last, heights = row['after'], row['after']['heights']
                except RECORD_ERRORS as exc:
                    raise ReplayFailure(str(exc), failing_step) from exc
            require(next(event_rows, None) is None, 'extra STEP artifact')
            same(cp['cursor'], cursor, 'checkpoint search cursor')
            same([cp['steps'], cp['letters'], cp['unit_count'], cp['unit_hash']],
                 [steps, letters, ordinal, previous], 'checkpoint counters')
            same(cp['last']['heights'], heights, 'checkpoint accepted histogram')
        else:
            # A successor (or an authenticated checker receipt) already reduced
            # this interval. Its artifact hashes and boundary are still checked.
            require(cp['unit_count'] >= ordinal and cp['steps'] >= steps and cp['letters'] >= letters,
                    'reversed search interval')
            cursor, previous, ordinal = cp['cursor'], cp['unit_hash'], cp['unit_count']
            steps, letters = cp['steps'], cp['letters']
        last = cp['last']
        same(cp['last']['digest'], cursor['digest'], 'checkpoint measurement binding')
        if cursor['hist'] is not None:
            same(cursor['hist'], cp['last']['heights'], 'cursor/accepted histogram')
        end = boundary(cp)
        same(m['end'], end, 'ending checkpoint boundary')
        require(steps <= 200, 'cumulative step limit')
        b = m['budget']
        require(type(b['local_cpu']) in (int, float) and 0 <= b['local_cpu'] <= 3300,
                'local CPU accounting')
        cpu += b['local_cpu']
        segments += 1
        same([b['segments'], b['cpu']], [segments, cpu], 'cumulative search accounting')
        require(segments <= 6 and cpu <= 19800, 'search allocation exceeded')
        require(m['status'] in ('BUDGET', 'HANDOFF', 'STUCK', 'QUALIFIED', 'DIAGNOSTICS'), 'search status')
        if m['kind'] == 'search':
            require(m['status'] not in ('QUALIFIED', 'DIAGNOSTICS'), 'search kind/status')
        if m['status'] == 'STUCK':
            same(cursor['phase'], 'stuck', 'STUCK without catalogue coverage')
        if m['status'] == 'HANDOFF':
            require(handoff(cp['last']['heights']), 'false HANDOFF')
        if m['kind'] == 'diagnostics':
            require(diagnostics is None and steps == ordinal == 0, 'duplicate or late diagnostics')
            if m['status'] == 'DIAGNOSTICS':
                require('diagnostics' in m['artifacts'], 'missing diagnostic certificate')
                certificate = parse(artifact_bytes(path.parent, m['artifacts']['diagnostics']))
                same(certificate['input'], pin, 'diagnostic input')
                same(certificate['identity'], m['identity'], 'diagnostic identity')
                diagnostics = m['artifacts']['diagnostics']['sha256']
            else:
                same(m['status'], 'BUDGET', 'unfinished diagnostics')
        same(cp['diagnostics'], diagnostics, 'diagnostic binding')
        if m['kind'] == 'qualification':
            q = m['qualification']
            if qualification is not None:
                same(q['ledger_sha256'], qualification['ledger_sha256'], 'qualification evidence changed')
            same(q['completed_units'], ordinal, 'qualification units')
            qualification = q
        same(cp['qualification'], qualification, 'qualification binding')
    return chain, cp


def boundary(cp):
    return dict(digest=cp['last']['digest'], cursor=cursor_hash(cp['cursor']),
                steps=cp['steps'], letters=cp['letters'], units=cp['unit_count'],
                unit_hash=cp['unit_hash'], ever=cp['ever'])


def reserve_attempt(root, directory, case, role):
    """Durable reservations survive killed/unfinalized processes.

    Keep allocation.json even for interrupted attempts. A finalized manifest
    includes it in the commit inventory. No elapsed work disappears on resume;
    a lost attempt conservatively consumes its full 3300-second reservation.
    """
    import fcntl
    with (root / '.allocation-lock').open('a+b') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        prior = []
        for path in root.glob('*/allocation.json'):
            require(not path.is_symlink(), 'allocation symlink')
            rec = parse(path.read_bytes())
            if rec['case'] == case and rec['role'] == role:
                prior.append(rec)
        limit = 6 if role == 'search' else 4
        require(len(prior) < limit, 'reserved ' + role + ' segment allocation exhausted')
        require(sorted(x['ordinal'] for x in prior) == list(range(1, len(prior) + 1)),
                'missing/duplicate budget reservation')
        record = dict(format='hd2p-allocation/1', case=case, role=role,
                      ordinal=len(prior) + 1, cpu_reserved=3300, attempt=directory.name)
        with (directory / 'allocation.json').open('x', encoding='utf-8') as f:
            emit(f, record)
            os.fsync(f.fileno())
        return record


def search_rows(chain):
    return [row for path, m in chain for row in artifact_lines(path.parent, m['artifacts']['events'])]


def check_search_boundary(path, manifest, cp):
    end = parse(artifact_bytes(path.parent, manifest['artifacts']['checkpoint']))
    same(end['last'], cp['last'], 'segment last measurements')
    same(end['ever'], cp['ever'], 'segment exact lattice-loss set')
    same(end['letters'], cp['letters'], 'segment global letters')
    if end['cursor']['hist'] is not None:
        same(end['cursor']['hist'], cp['last']['heights'], 'cursor histogram/independent replay')


def checker_initial(first, ever):
    return dict(version='hd2p-checkpoint/3', last=first, ever=bitset(ever), steps=0, letters=0,
                snapshots=1, before=None, prefixes=[], audit=None, audit_units=0, audit_hash=ZERO_HASH,
                validated_search=None, search_segments=[], failure=None,
                checks=dict(boundaries=[], final_last=None, final_ever=None, tables=[]))


def checker_schema(cp):
    schema(cp, {'version', 'last', 'ever', 'steps', 'letters', 'snapshots', 'before', 'prefixes',
                'audit', 'audit_units', 'audit_hash', 'validated_search', 'search_segments', 'checks', 'failure'},
           'checker checkpoint schema')
    same(cp['version'], 'hd2p-checkpoint/3', 'checker checkpoint version')
    schema(cp['checks'], {'boundaries', 'final_last', 'final_ever', 'tables'}, 'checker checks schema')
    require(type(cp['prefixes']) is list and len(cp['prefixes']) <= 1, 'checker pending prefixes')
    require(all(type(cp[k]) is int and cp[k] >= 0 for k in ('steps', 'letters', 'snapshots', 'audit_units')),
            'checker counters')
    require(type(cp['search_segments']) is list, 'checker search segments')
    for name in ('boundaries', 'tables'):
        values = cp['checks'][name]
        require(type(values) is list and len(values) == len(set(values)), 'duplicate completed check')
        require(all(v in cp['search_segments'] for v in values), 'completed check outside search chain')
    same(len(unbitset(cp['ever'])), cp['last']['ever_lattice_loss'], 'checker loss bitset')
    if cp['failure'] is not None:
        schema(cp['failure'], {'phase', 'reason', 'step', 'candidate'}, 'checker failure schema')
        require(cp['failure']['phase'] in ('literal_replay', 'exhaustion_verification')
                and type(cp['failure']['reason']) is str and type(cp['failure']['step']) is int,
                'checker failure fields')


def checker_boundary(cp):
    return dict(digest=cp['last']['digest'], steps=cp['steps'], letters=cp['letters'],
                prefixes=len(cp['prefixes']), ever=cp['ever'], snapshots=cp['snapshots'],
                audit=cp['audit'], audit_units=cp['audit_units'], audit_hash=cp['audit_hash'],
                checkpoint_sha256=digest_json(cp))


def check_receipt(cp, name, ref):
    checks = copy.deepcopy(cp['checks'])
    if name in ('boundaries', 'tables'):
        require(ref not in checks[name], 'duplicate completed check')
        checks[name].append(ref)
    else:
        require(name in ('final_last', 'final_ever') and checks[name] != ref, 'duplicate FINAL check')
        checks[name] = ref
    new = dict(cp, checks=checks)
    return new, dict(type='CHECK', check=name, segment=ref, checkpoint=new)


def post_replay_complete(cp):
    head = cp['validated_search']
    return (head is not None and bool(cp['search_segments']) and not cp['prefixes']
            and cp['checks']['boundaries'] == cp['search_segments']
            and cp['checks']['tables'] == cp['search_segments']
            and cp['checks']['final_last'] == cp['checks']['final_ever'] == head['sha256'])


def checker_post_unit(cp, chain, final, rt, temporary, at_end):
    """One independently completed boundary/FINAL/table check, then its receipt.

    The caller commits the receipt transactionally and checks its budget between
    units. An interruption can repeat at most this check, never the whole list.
    """
    if cp['prefixes']:
        return None
    for path, manifest in chain:
        ref = manifest_ref(path)['sha256']
        if ref not in cp['checks']['boundaries'] and manifest['end']['steps'] == cp['steps']:
            try:
                check_search_boundary(path, manifest, cp)
            except RECORD_ERRORS as exc:
                raise ReplayFailure(str(exc), manifest['end']['steps']) from exc
            return check_receipt(cp, 'boundaries', ref)
    if not at_end:
        return None
    same(cp['steps'], final['steps'], 'FINAL step count')
    same(cp['checks']['boundaries'], cp['search_segments'], 'missing segment replay boundary')
    target = cp['validated_search']['sha256']
    for name, key in (('final_last', 'last'), ('final_ever', 'ever')):
        if cp['checks'][name] != target:
            same(cp[key], final[key], 'FINAL ' + key)
            return check_receipt(cp, name, target)
    for path, manifest in chain:
        ref = manifest_ref(path)['sha256']
        if ref not in cp['checks']['tables']:
            try:
                load_artifact_table(path.parent, manifest['artifacts']['table'], rt,
                                    manifest['end']['digest'], temporary)
            except RECORD_ERRORS as exc:
                raise ReplayFailure(str(exc), manifest['end']['steps']) from exc
            return check_receipt(cp, 'tables', ref)
    return None


def replay_letter(engine, state, cp, rows, initial_lattice):
    """Transactional single-prefix verification, with one full snapshot per letter."""
    require(cp['steps'] < len(rows), 'replay beyond STEP rows')
    row = rows[cp['steps']]
    step_schema(row)
    k = len(cp['prefixes'])
    word, family = row['word'], row['family']
    require(k < len(word), 'pending prefix position')
    same(row['rule'], RULE, 'STEP rule')
    same(row['type'], 'STEP', 'STEP type')
    same(row['index'], cp['steps'] + 1, 'STEP index')
    same(row['letters'], len(word), 'STEP letters')
    same(row['cumulative_letters'], cp['letters'] - k + len(word), 'STEP cumulative letters')
    before = cp['before'] if k else cp['last']
    same(row['before'], before, 'before measurements')
    same(row['input_digest'], before['digest'], 'input digest')
    if family == 'CL2':
        require(engine.meets(word[0][0], word[1][0]), 'CL2 caps do not meet')
    trial = engine.clone(state)
    engine.apply(trial, *word[k])
    snap, ever = engine.snapshot(trial, initial_lattice, unbitset(cp['ever']))
    same(snap, row['prefixes'][k], 'prefix measurements')
    same(snap['digest'], row['prefix_digests'][k], 'prefix digest')
    require(maximum(snap['heights']) <= maximum(before['heights']), 'current input height bound')
    new = copy.deepcopy(cp)
    new.update(last=snap, ever=bitset(ever), letters=cp['letters'] + 1,
               snapshots=cp['snapshots'] + 1, before=before, prefixes=cp['prefixes'] + [snap])
    if k + 1 == len(word):
        contract(before, new['prefixes'], word, family)
        same(row['after'], snap, 'after measurements')
        same(row['output_digest'], snap['digest'], 'output digest')
        new.update(steps=cp['steps'] + 1, prefixes=[], before=None)
    receipt = dict(type='PREFIX', step=row['index'], prefix=k + 1,
                   row_sha256=digest_json(row), checkpoint=new)
    return trial, new, receipt


def audit_cursor(hist):
    require(maximum(hist) in (4, 6), 'audit height')
    return dict(version='hd2p-independent-audit/1', hist=hist, phase='input',
                clean=[], classified=0, ci=0, xi=0, di=0)


def audit_advance(cursor, proof, caps=tuple(range(600)), items=tuple(range(1, 60)), jumble=tuple(range(12, 60))):
    """Audit coverage protocol, intentionally separate from the search cursor."""
    cur = copy.deepcopy(cursor)
    same(proof['at'], {k: cur[k] for k in ('phase', 'classified', 'ci', 'xi', 'di')}, 'audit interval')
    result, phase = proof['outcome'], cur['phase']
    require(result != 'candidate', 'false STUCK: independent candidate found')
    if phase == 'input':
        require(result in ('clean', 'blocked'), 'audit classification')
        if result == 'clean':
            cur['clean'].append(caps[cur['classified']])
        cur['classified'] += 1
        if cur['classified'] == len(caps):
            cur['phase'] = 'single' if cur['clean'] else 'complete'
    elif phase == 'single':
        require(result == 'rejected', 'audit SINGLE outcome')
        require(maximum(proof['heights']) > maximum(cur['hist'])
                or qvalue(proof['heights']) >= qvalue(cur['hist']), 'false audited SINGLE rejection')
        cur['xi'] += 1
        if cur['xi'] == len(jumble):
            cur['ci'], cur['xi'] = cur['ci'] + 1, 0
        if cur['ci'] == len(cur['clean']):
            cur.update(phase='open', ci=0)
    elif phase in ('open', 'target'):
        advance_open = False
        if phase == 'open':
            require(result in ('survives', 'pruned'), 'audit opener outcome')
            same(result, 'pruned' if maximum(proof['heights']) > maximum(cur['hist']) else 'survives',
                 'audit opener height bound')
            if result == 'survives':
                cur.update(phase='target', di=0)
            else:
                advance_open = True
        else:
            require(result in ('ineligible', 'blocked', 'endpoints'), 'audit target outcome')
            if result == 'endpoints':
                same(proof['ys'], list(jumble), 'incomplete audit y group')
                same(len(proof['heights']), len(jumble), 'audit endpoint coverage')
                require(all(h is None or maximum(h) > maximum(cur['hist']) or qvalue(h) >= qvalue(cur['hist'])
                            for h in proof['heights']), 'false audited exhaustion')
            cur['di'] += 1
            advance_open = cur['di'] == len(caps)
        if advance_open:
            cur.update(phase='open', di=0, xi=cur['xi'] + 1)
            if cur['xi'] == len(items):
                cur['ci'], cur['xi'] = cur['ci'] + 1, 0
            if cur['ci'] == len(cur['clean']):
                cur.update(phase='peel', ci=0)
    elif phase == 'peel':
        require(result == 'rejected', 'audit PEEL outcome')
        h = proof['heights']
        if maximum(h) <= maximum(cur['hist']) and all(h.get(str(k), 0) == cur['hist'].get(str(k), 0) for k in (4, 6)):
            require(type(proof['Bk']) is int and len(caps) - len(cur['clean']) <= proof['Bk'] <= len(caps),
                    'false audited PEEL rejection')
        cur['xi'] += 1
        if cur['xi'] == len(items):
            cur['ci'], cur['xi'] = cur['ci'] + 1, 0
        if cur['ci'] == len(cur['clean']):
            cur['phase'] = 'complete'
    else:
        raise ValueError('invalid/finished audit cursor')
    return cur


class FalseStuck(ValueError):
    def __init__(self, proof):
        self.proof = proof
        super().__init__('false STUCK: independent candidate ' + canonical(proof))


class ExhaustionAudit:
    """Independent exact catalogue; see the module's histogram/locality proof.

    Only actual intermediate states are classified. Caches are bounded, private
    to this auditor, and contain no search data or mutable local pose-ID keys.
    """
    def __init__(self, engine, caps=tuple(range(600)), items=tuple(range(1, 60)), jumble=tuple(range(12, 60))):
        self.engine, self.caps, self.items, self.jumble = engine, caps, items, jumble
        self.opened = (None, None, None)
        self.base_state, self.base_classes, self.insides = None, {}, {}
        self.distribution = (None, ())
        self.factor = lru_cache(maxsize=4096)(self._factor)
        self.product = lru_cache(maxsize=20000)(self._product)
        self.twist = lru_cache(maxsize=600)(self._twist)
        self.neighbours = {}
        self.stats = {}

    def timed(self, name, started):
        self.stats[name + '_cpu'] = self.stats.get(name + '_cpu', 0.) + time.process_time() - started
        self.stats[name + '_count'] = self.stats.get(name + '_count', 0) + 1

    def classification(self, state, c, inside=True):
        self.engine.tick()
        started = time.process_time()
        cl = state.classify(c, need_inside=inside, stop_at_straddle=True)
        require(not cl.uncertain, 'uncertain independent audit')
        self.timed('classification', started)
        return cl

    def original(self, state, c):
        if state is not self.base_state:
            self.base_state, self.base_classes, self.insides = state, {}, {}
        if c not in self.base_classes:
            self.base_classes[c] = self.classification(state, c)
        return self.base_classes[c]

    def moved(self, state, c):
        cl = self.original(state, c)
        require(cl.straddle is None, 'blocked audit opener')
        if c not in self.insides:
            self.insides[c] = set(map(int, cl.inside))
        return self.insides[c]

    def meeting_caps(self, c):
        if c not in self.neighbours:
            self.neighbours[c] = tuple(d for d in self.caps if d != c and self.engine.meets(c, d))
        return self.neighbours[c]

    def _twist(self, c, x):
        rt = self.engine.rt
        return rt.sim.Twist(rt.ctx, c, rt.restore.element(rt.menu, c, x), 'menu', {'menu': rt.menu.name})

    def _factor(self, matrix):
        trees = self.engine.rt.trees
        left, right = trees.factors(matrix)
        require(trees.dist(right) == 0, 'audit right factor')
        h = int(trees.dist(left))
        require(h >= 0 and h % 2 == 0, 'audit left height')
        return h, left

    def _product(self, item, pose):
        trees = self.engine.rt.trees
        return int(trees.dist(trees.qmul(item, pose)))

    def endpoint(self, state, c, x, cl, hist):
        """Exact multiplicity transfer on an independently classified actual state."""
        self.engine.tick()
        started = time.process_time()
        require(cl.straddle is None and not cl.uncertain, 'uncertified audit endpoint')
        tw = self.twist(c, x)
        if tw.kidx >= 0:
            answer = dict(hist)
        else:
            rt = self.engine.rt
            # State objects are immutable while used here; apply operates on a clone.
            distribution_key, distribution = self.distribution
            if distribution_key != (state, c):
                ids, ns = rt.np.unique(state.pose_id[cl.inside], return_counts=True)
                distribution = []
                for pid, n in zip(ids.tolist(), ns.tolist()):
                    self.engine.tick()
                    h, factor = ((0, None) if state._poses[pid].kidx >= 0
                                 else self.factor(state._pose_matrix(pid)))
                    require(factor is None or h > 0, 'audit off-pose height')
                    distribution.append((h, factor, n))
                self.distribution = ((state, c), distribution)
            item_height, item = self.factor(tw.matrix)
            answer = dict(hist)
            for height, factor, n in distribution:
                self.engine.tick()
                if height:
                    answer[str(height)] = answer.get(str(height), 0) - n
                    new = self.product(item, factor)
                else:
                    new = item_height
                if new:
                    answer[str(new)] = answer.get(str(new), 0) + n
            require(all(n >= 0 for n in answer.values()), 'negative audit histogram')
            answer = {h: n for h, n in answer.items() if n}
        self.timed('prediction', started)
        return answer

    def move(self, state, c, x):
        started = time.process_time()
        trial = self.engine.clone(state)
        self.engine.apply(trial, c, x)
        self.timed('apply', started)
        return trial

    def peel_blocked(self, state, end, c, clean):
        """Full Bk from unchanged nonmeeting caps and exact affected-cap checks."""
        started = time.process_time()
        blocked, moved = len(self.caps) - len(clean), self.moved(state, c)
        for d in self.meeting_caps(c):
            self.engine.tick()
            original = self.original(state, d)
            was = original.straddle is not None
            if was and original.straddle['piece'] not in moved:
                continue
            now = self.classification(end, d, inside=False).straddle is not None
            blocked += int(now) - int(was)
        self.timed('peel_recount', started)
        return blocked

    def unit(self, state, cursor):
        engine, cur = self.engine, cursor
        proof = dict(at={k: cur[k] for k in ('phase', 'classified', 'ci', 'xi', 'di')})
        phase, hist = cur['phase'], cur['hist']
        if phase == 'input':
            cl = self.original(state, self.caps[cur['classified']])
            proof['outcome'] = 'clean' if cl.straddle is None else 'blocked'
        elif phase == 'single':
            c, x = cur['clean'][cur['ci']], self.jumble[cur['xi']]
            end = self.endpoint(state, c, x, self.original(state, c), hist)
            found = maximum(end) <= maximum(hist) and qvalue(end) < qvalue(hist)
            proof.update(outcome='candidate' if found else 'rejected', word=[[c, x]], heights=end)
        elif phase in ('open', 'target'):
            c, x = cur['clean'][cur['ci']], self.items[cur['xi']]
            if phase == 'open':
                mh = self.endpoint(state, c, x, self.original(state, c), hist)
                proof.update(outcome='pruned' if maximum(mh) > maximum(hist) else 'survives', heights=mh)
            else:
                d = self.caps[cur['di']]
                if d == c or d not in self.meeting_caps(c):
                    proof['outcome'] = 'ineligible'
                elif (self.original(state, d).straddle is not None
                      and self.original(state, d).straddle['piece'] not in self.moved(state, c)):
                    proof['outcome'] = 'blocked'
                else:
                    key = state, c, x
                    open_key, middle, middle_hist = self.opened
                    if open_key != key:
                        middle = self.move(state, c, x)
                        middle_hist = self.endpoint(state, c, x, self.original(state, c), hist)
                        self.opened = (key, middle, middle_hist)
                    cl = self.classification(middle, d)
                    if cl.straddle is not None:
                        proof['outcome'] = 'blocked'
                    else:
                        heights = [self.endpoint(middle, d, y, cl, middle_hist) for y in self.jumble]
                        candidates = [(qvalue(h), y) for h, y in zip(heights, self.jumble)
                                      if maximum(h) <= maximum(hist)]
                        found = bool(candidates) and min(candidates)[0] < qvalue(hist)
                        proof.update(outcome='candidate' if found else 'endpoints', ys=list(self.jumble), heights=heights)
                        if found:
                            proof['word'] = [[c, x], [d, min(candidates)[1]]]
        elif phase == 'peel':
            c, x = cur['clean'][cur['ci']], self.items[cur['xi']]
            h = self.endpoint(state, c, x, self.original(state, c), hist)
            found, bk = False, None
            if maximum(h) <= maximum(hist) and all(h.get(str(k), 0) == hist.get(str(k), 0) for k in (4, 6)):
                end = self.move(state, c, x)
                bk = self.peel_blocked(state, end, c, cur['clean'])
                found = bk < len(self.caps) - len(cur['clean'])
            proof.update(outcome='candidate' if found else 'rejected', word=[[c, x]], heights=h, Bk=bk)
        else:
            raise ValueError('audit is complete')
        if proof['outcome'] == 'candidate':
            raise FalseStuck(proof)
        return audit_advance(cur, proof, self.caps, self.items, self.jumble), proof


def audit_record(cp, after, proof):
    body = dict(version='hd2p-audit-units/1', ordinal=cp['audit_units'] + 1,
                previous=cp['audit_hash'], before=digest_json(cp['audit']),
                after=digest_json(after), proof=proof)
    return dict(body, sha256=digest_json(body))


def authenticate_checker(head, identity, target_chain, rows):
    """Authenticate execution receipts under the module's self-certification trust.

    All committed bytes are hashed. Older attempts were validated when their
    successor was finalized; only the last attempt's new receipts are reduced.
    """
    chain = manifest_chain(head, CHECK, identity)
    targets = [manifest_ref(path) for path, _ in target_chain]
    target_map = {ref['sha256']: (path, m) for ref, (path, m) in zip(targets, target_chain)}
    prior, cpu, count, previous_target = None, 0, 0, -1
    for path, m in chain:
        require(m['target'] in targets, 'checker belongs to different search chain')
        target_index = targets.index(m['target'])
        require(target_index >= previous_target, 'checker target moved backwards')
        previous_target = target_index
        target_path = confined(path.parent.parent, m['target']['path'])
        same(sha(target_path.read_bytes()), m['target']['sha256'], 'checker target hash')
        cp = parse(artifact_bytes(path.parent, m['artifacts']['checkpoint']))
        checker_schema(cp)
        if cp['failure'] and cp['failure']['phase'] == 'literal_replay':
            raise ReplayFailure('failed literal replay invalidates steps from ' + str(cp['failure']['step']),
                                cp['failure']['step'])
        target_manifest = target_chain[target_index][1]
        target_steps = target_manifest['end']['steps']
        target_refs = [r['sha256'] for r in targets[:target_index + 1]]
        require(cp['steps'] <= target_steps, 'checker ahead of target')
        if path == chain[-1][0]:
            for receipt in artifact_lines(path.parent, m['artifacts']['verification']):
                new = receipt['checkpoint']
                checker_schema(new)
                kind = receipt['type']
                if kind == 'INITIAL':
                    require(prior is None, 'duplicate checker origin')
                    same(new['last']['digest'], m['input']['digest'], 'checker original state')
                    same(new['last']['heights'], m['input']['heights'], 'checker original histogram')
                    same(new, checker_initial(new['last'], set()), 'checker original counters')
                elif kind == 'SEARCH':
                    require(prior is not None, 'checker missing origin')
                    expected = dict(prior, validated_search=m['target'], search_segments=target_refs)
                    same(new, expected, 'checker validated search receipt')
                elif kind == 'CHECK':
                    require(prior is not None and not prior['prefixes'], 'check before completed prefix')
                    name, ref = receipt['check'], receipt['segment']
                    require(ref in target_refs, 'check target is not an ancestor')
                    search_path, search_manifest = target_map[ref]
                    if name == 'boundaries':
                        same(search_manifest['end']['steps'], prior['steps'], 'boundary receipt position')
                        check_search_boundary(search_path, search_manifest, prior)
                    else:
                        same(prior['steps'], target_steps, 'post-replay check before replay completion')
                        if name in ('final_last', 'final_ever'):
                            same(ref, m['target']['sha256'], 'FINAL check target')
                            final = parse(artifact_bytes(target_path.parent, target_manifest['artifacts']['checkpoint']))
                            key = 'last' if name == 'final_last' else 'ever'
                            same(prior[key], final[key], 'FINAL receipt ' + key)
                    expected, _ = check_receipt(prior, name, ref)
                    same(new, expected, 'completed check receipt')
                elif kind == 'AUDIT_START':
                    require(prior is not None and prior['audit'] is None and not prior['prefixes']
                            and prior['steps'] == target_steps and prior['steps'] == len(rows)
                            and post_replay_complete(prior), 'audit before complete literal replay')
                    same(new, dict(prior, audit=audit_cursor(prior['last']['heights'])), 'audit origin')
                else:
                    require(prior is not None and kind == 'PREFIX', 'checker missing origin/receipt type')
                    require(prior['steps'] < len(rows), 'receipt beyond STEP rows')
                    row = rows[prior['steps']]
                    step_schema(row)
                    k = len(prior['prefixes'])
                    require(k < len(row['word']), 'receipt prefix position')
                    same([receipt['step'], receipt['prefix'], receipt['row_sha256']],
                         [row['index'], k + 1, digest_json(row)], 'checker receipt position')
                    same(new['last'], row['prefixes'][k], 'checker receipt measurement')
                    require(unbitset(prior['ever']) <= unbitset(new['ever']), 'lost transient lattice loss')
                    expected = copy.deepcopy(prior)
                    before = prior['before'] if k else prior['last']
                    same(before, row['before'], 'checker before fields')
                    same(row['input_digest'], before['digest'], 'receipt input digest')
                    same(row['cumulative_letters'], prior['letters'] - k + len(row['word']), 'receipt letters')
                    expected.update(last=new['last'], ever=new['ever'], letters=prior['letters'] + 1,
                                    snapshots=prior['snapshots'] + 1, before=before,
                                    prefixes=prior['prefixes'] + [new['last']])
                    if k + 1 == len(row['word']):
                        contract(before, expected['prefixes'], row['word'], row['family'])
                        same(row['after'], new['last'], 'receipt after measurements')
                        same(row['output_digest'], new['last']['digest'], 'receipt output digest')
                        expected.update(steps=prior['steps'] + 1, prefixes=[], before=None)
                    same(new, expected, 'checker receipt continuity')
                prior = new
            require(prior is not None, 'missing checker receipts')
            for unit in artifact_lines(path.parent, m['artifacts']['audit']):
                require(prior['audit'] is not None and not prior['prefixes']
                        and prior['steps'] == len(rows) == target_steps and post_replay_complete(prior),
                        'audit before complete literal replay')
                same(prior['audit']['hist'], prior['last']['heights'], 'audit/final histogram')
                same(unit['version'], 'hd2p-audit-units/1', 'audit chain version')
                same(unit['sha256'], digest_json({k: v for k, v in unit.items() if k != 'sha256'}), 'audit hash')
                same([unit['ordinal'], unit['previous'], unit['before']],
                     [prior['audit_units'] + 1, prior['audit_hash'], digest_json(prior['audit'])], 'audit interval chain')
                after = audit_advance(prior['audit'], unit['proof'])
                same(unit['after'], digest_json(after), 'audit next cursor')
                prior = dict(prior, audit=after, audit_units=unit['ordinal'], audit_hash=unit['sha256'])
            if cp['failure'] is not None:
                require(post_replay_complete(prior) and prior['audit'] is not None,
                        'exhaustion failure before literal replay')
                prior = dict(prior, failure=cp['failure'])
            same(cp, prior, 'authenticated checker checkpoint')
        else:
            prior = cp
        if cp['audit'] is not None:
            require(not cp['prefixes'] and cp['steps'] == target_steps and post_replay_complete(cp),
                    'audit before complete literal replay')
            same(cp['audit']['hist'], cp['last']['heights'], 'audit/final histogram')
        same(m['end'], checker_boundary(cp), 'checker ending boundary')
        same(m['start'], previous_end if count else None, 'checker start boundary')
        require((m['status'] == 'FAILED') == (cp['failure'] is not None), 'checker failure status')
        replay_done = cp['steps'] == target_steps and post_replay_complete(cp)
        audit_done = cp['audit'] is not None and cp['audit']['phase'] == 'complete'
        complete = replay_done and (target_manifest.get('status') != 'STUCK' or audit_done)
        same(m['status'], 'FAILED' if cp['failure'] else 'complete' if complete else 'incomplete',
             'checker completion status')
        previous_end = m['end']
        count += 1
        cpu += m['budget']['local_cpu']
        same([m['budget']['segments'], m['budget']['cpu']], [count, cpu], 'checker cumulative budget')
        require(count <= 4 and cpu <= 13200, 'checker allocation')
    return chain, cp


def checking_report(cp, final):
    replayed = (not cp['prefixes'] and cp['steps'] == final['steps'] and post_replay_complete(cp))
    audited = cp['audit'] is not None and cp['audit']['phase'] == 'complete'
    failure = cp['failure']
    literal = 'complete' if replayed else 'incomplete'
    exhaustion = 'complete' if audited else 'incomplete' if cp['audit'] else 'not_performed'
    if failure:
        if failure['phase'] == 'literal_replay':
            literal = 'failed'
        else:
            exhaustion = 'failed'
    complete = literal == 'complete' and (final['cursor']['phase'] != 'stuck' or exhaustion == 'complete')
    result = 'BUDGET'
    if complete and handoff(cp['last']['heights']):
        result = 'HANDOFF'
    if complete and exhaustion == 'complete' and final['cursor']['phase'] == 'stuck':
        result = 'STUCK'
    report = dict(status='FAILED' if failure else 'complete' if complete else 'incomplete',
                  result=result if not failure else 'BUDGET', passed=complete and failure is None,
                  literal_replay=literal, exhaustion_verification=exhaustion,
                  steps=cp['steps'], letters=cp['letters'], digest=cp['last']['digest'],
                  snapshots=cp['snapshots'], checks=cp['checks'],
                  scope='accepted suffix with boundary/FINAL/table checks; exhaustion reported separately')
    if failure:
        report.update(reason=failure['reason'], invalidates=(
            dict(accepted_steps_from=failure['step'], dependent_descendants=True)
            if failure['phase'] == 'literal_replay' else dict(stuck_claim=True, accepted_steps=False)))
        if failure['candidate'] is not None:
            report['independent_candidate'] = failure['candidate']
    return report


def run_checker(args):
    root = external_directory(args.out)
    # Usage errors precede directory creation, reservations and publication.
    chain = manifest_chain(args.chain, SEGMENT)
    head = chain[-1][1]
    require(Path(args.chain).resolve().parent.parent == root, 'checker campaign root')
    final = parse(artifact_bytes(Path(args.chain).parent, head['artifacts']['checkpoint']))
    if args.audit:
        require(final['cursor']['phase'] == 'stuck', 'audit requires a candidate STUCK')
    deadline = head['not_after']
    if args.not_after:
        same(utc_deadline(args.not_after), deadline, 'campaign deadline cannot reset')
    directory = new_output(root, 'audit' if args.audit else 'checker')
    reservation = reserve_attempt(root, directory, head['input']['case'], 'checker')
    prior = previous = validation_error = None
    rows = []
    malformed = RECORD_ERRORS
    try:
        rows = search_rows(chain)
        if args.resume_check:
            require(Path(args.resume_check).resolve().parent.parent == root, 'checker resume campaign root')
            old_chain, prior = authenticate_checker(args.resume_check, head['identity'], chain, rows)
            previous = old_chain[-1][1]
            require(previous['budget']['segments'] < 4, 'four checker/audit segment limit')
    except malformed as exc:
        validation_error = exc
        prior = previous = None
    used = previous['budget']['cpu'] if previous else 0
    limits = SegmentLimits(used, ceiling=13200, total=args.cpu_seconds, useful=args.work_cpu_seconds,
                           wall=args.wall_seconds, not_after=deadline)
    rt = Runtime(directory)
    identity = segment_identity(rt)
    same(head['identity'], identity, 'checker source/model/menu identity')
    engine = ReplayEngine(rt, limits.check)
    original = load_case(head['input']['case'], rt, engine)
    initial_lattice = set(range(P)) - engine.off(original)
    receipts, audit_cpu, auditor = [], 0., None
    if prior:
        state = load_artifact_table(Path(args.resume_check).parent, previous['artifacts']['table'],
                                    rt, prior['last']['digest'], directory)
        cp, start = copy.deepcopy(prior), checker_boundary(prior)
        require(unbitset(cp['ever']) <= initial_lattice, 'checker lattice basis')
    else:
        state = original
        first, ever = engine.snapshot(state, initial_lattice, set())
        cp, start = checker_initial(first, ever), None
        receipts.append(dict(type='INITIAL', checkpoint=copy.deepcopy(cp)))

    def commit(new, receipt):
        nonlocal cp
        saved, size = cp, len(receipts)
        try:
            receipts.append(receipt)
            cp = new
        except BaseException:
            cp = saved
            del receipts[size:]
            raise

    phase, failing_step = 'literal_replay', cp['steps'] + 1
    with (directory / 'audit.disposable').open('w+b') as audit_stream:
        try:
            limits.boundary()
            if validation_error is not None:
                raise ReplayFailure('checker input/receipt validation: ' + str(validation_error),
                                    getattr(validation_error, 'step', failing_step))
            chain, final = search_chain(args.chain, validated=cp['validated_search'], chain=chain)
            target = manifest_ref(args.chain)
            if cp['validated_search'] != target:
                require(cp['audit'] is None, 'cannot extend an audited terminal state')
                new = dict(cp, validated_search=target,
                           search_segments=[manifest_ref(path)['sha256'] for path, _ in chain])
                commit(new, dict(type='SEARCH', checkpoint=new))
            while cp['failure'] is None:
                limits.check()
                at_end = cp['steps'] == len(rows) and not cp['prefixes']
                failing_step = cp['steps'] if at_end else cp['steps'] + 1
                checked = checker_post_unit(cp, chain, final, rt, directory, at_end)
                if checked is not None:
                    commit(*checked)
                    continue
                if at_end:
                    break
                saved, prior_state, nreceipts = cp, state, len(receipts)
                try:
                    state, cp, receipt = replay_letter(engine, state, cp, rows, initial_lattice)
                    receipts.append(receipt)
                except BaseException:
                    cp, state = saved, prior_state
                    del receipts[nreceipts:]
                    raise
            if args.audit and cp['failure'] is None:
                require(cp['steps'] == len(rows) and not cp['prefixes'] and post_replay_complete(cp),
                        'audit before complete literal replay')
                phase = 'exhaustion_verification'
                if cp['audit'] is None:
                    new = dict(cp, audit=audit_cursor(cp['last']['heights']))
                    commit(new, dict(type='AUDIT_START', checkpoint=new))
                same(cp['audit']['hist'], final['last']['heights'], 'audit/final histogram')
                auditor = ExhaustionAudit(engine)
                while cp['audit']['phase'] != 'complete':
                    limits.check()
                    audit_started = time.process_time()
                    try:
                        after, proof = auditor.unit(state, cp['audit'])
                    finally:
                        audit_cpu += time.process_time() - audit_started
                    unit = audit_record(cp, after, proof)
                    limits.check()
                    saved, offset = cp, audit_stream.tell()
                    try:
                        audit_stream.write((canonical(unit) + '\n').encode())
                        cp = dict(cp, audit=after, audit_units=unit['ordinal'], audit_hash=unit['sha256'])
                    except BaseException:
                        cp = saved
                        audit_stream.seek(offset)
                        audit_stream.truncate()
                        raise
        except Budget:
            pass
        except malformed as exc:
            cp = dict(cp, failure=dict(phase=phase, reason=str(exc), step=getattr(exc, 'step', failing_step),
                                      candidate=exc.proof if isinstance(exc, FalseStuck) else None))
        limits.finalizing()
        artifacts = {}
        audit_stream.flush()
        audit_stream.seek(0)
        artifacts['audit'] = pack_artifact(directory, 'audit', audit_stream)
    artifacts['verification'] = pack_artifact(directory, 'verification',
                                             b''.join((canonical(r) + '\n').encode() for r in receipts))
    artifacts['checkpoint'] = pack_artifact(directory, 'checkpoint', (canonical(cp) + '\n').encode())
    artifacts['table'] = pack_artifact(directory, 'table', table_bytes(state, rt))
    artifacts['allocation'] = pack_artifact(directory, 'allocation', (canonical(reservation) + '\n').encode())
    report = checking_report(cp, final)
    timing = limits.times()
    artifacts['profile'] = pack_artifact(directory, 'profile', (canonical(dict(
        **timing, snapshot_cpu_seconds=engine.snapshot_cpu, audit_cpu_seconds=audit_cpu,
        audit_costs={} if auditor is None else auditor.stats,
        python=sys.version, numpy=rt.np.__version__, snapshots=cp['snapshots'] - (prior['snapshots'] if prior else 0),
        audit_units=cp['audit_units'] - (prior['audit_units'] if prior else 0),
        max_rss_kib=resource.getrusage(resource.RUSAGE_SELF).ru_maxrss)) + '\n').encode())
    timing = limits.times()
    budget = dict(segments=1 + (previous['budget']['segments'] if previous else 0),
                  cpu=used + timing['cpu_seconds'], local_cpu=timing['cpu_seconds'],
                  local_wall=timing['wall_seconds'], reserved_segments=reservation['ordinal'],
                  reserved_cpu=3300 * reservation['ordinal'],
                  limits=dict(cpu_seconds=args.cpu_seconds, work_cpu_seconds=args.work_cpu_seconds,
                              wall_seconds=args.wall_seconds))
    path = publish(directory, 'audit' if args.audit else 'checker', identity, head['input'], head['campaign'],
                   manifest_ref(args.resume_check) if previous else None,
                   start, checker_boundary(cp), artifacts, budget, deadline, report['status'],
                   extra=dict(target=manifest_ref(args.chain), report=report))
    (directory / 'audit.disposable').unlink()
    print(canonical(dict(report, manifest=str(path.relative_to(root)), **timing)), flush=True)
    return 0 if report['status'] == 'complete' and report['passed'] else 1


def main():
    if '--chain' not in sys.argv:
        return legacy_main()
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--out', required=True)
    ap.add_argument('--chain', required=True)
    ap.add_argument('--resume-check')
    ap.add_argument('--audit', action='store_true')
    ap.add_argument('--cpu-seconds', type=float, default=3300)
    ap.add_argument('--work-cpu-seconds', type=float, default=3000)
    ap.add_argument('--wall-seconds', type=float, default=3600)
    ap.add_argument('--not-after')
    args = ap.parse_args()
    try:
        SegmentLimits(total=args.cpu_seconds, useful=args.work_cpu_seconds, wall=args.wall_seconds,
                      not_after=utc_deadline(args.not_after))
        return run_checker(args)
    except Budget as exc:
        print(canonical(dict(result='BUDGET', finalized=False, reason=str(exc))), flush=True)
        return 1
    except (ValueError, OSError, KeyError, TypeError, IndexError, AttributeError, AssertionError) as exc:
        print(canonical(dict(result='ERROR', finalized=False, reason=str(exc))), flush=True)
        return 1


if __name__ == '__main__':
    sys.exit(main())
