"""Chapter 6 of the jumbling tutorial: the lattice-phase facts it quotes (headless research).

Read-only. It imports the two lattice restorers' libraries (sched11.py with the stage 03 kit, and the late restorer's
kit), the runner's lattice plugin and the literal player, and writes one JSON record. It reruns both restorers' orbit
solves at the lattice start of the committed D0 record of seed 7106 (depth 20), without their slow library check, and
compares what they build with the committed records. It plays nothing in the simulator and writes nothing under the
repository; --cache only redirects the theory modules' home cache.

Run from the repository root (about 90 seconds on a cloud machine):
    python3 -I -B research/jumbling/tutorial/scripts/ch06_examples.py --out <new file> --cache <new dir>

Sections of the record:
  A cascade  coll(o) of the 35 committed seeds; the cascade built in rounds as restore_lattice_late.py:64-69 does;
             the list a one-pass scan would give instead; both compared with realign.json.gz and the two records
  B orbits   per orbit: N_o, tree positions, A, B, C, stickers per piece, frame group, committed and realigned seed
             lengths, route maxima, return length h and swap length, and the T11-05 term (sum 20,866,844)
  C s11      schedule11 at the D0 lattice start (runs/dev/d0-s7106, tables/bulk-end.json): per orbit the method,
             the factor counts, the plan costs, the set-order picks; the solutions file's sha256 against the
             committed report, and every word's and tree's sha256 against the committed rows
  D factors  every factor of the orbits 6, 0, 17, 34 and 32, and the first factors of 15 and 21, under schedule11:
             kind, positions, setup words and their parts, chosen frame elements, lengths and step tree
  E late     the late restorer's kits at the same state: its program against the committed format 1 record
             (runs/depth20-seed7106/lattice/solutions.json.gz), per orbit letters and tree parts, the tree of the
             first factors of orbit 6, and the first step's word and tree sha256
  F depth40  the committed stage 12 lattice program at the depth 40 stop of seed 7101: rules, corrections, node and
             factor counts, tree sizes
  G library  the correction library: kinds, lengths and the bound of AU-05
  H frames   per frame orbit: the frame group as the late restorer builds it (closure of the pair values,
             restore_lattice_late.py:102-114) and as schedule11 builds it (the tree returns, sched11.py:234-243),
             whether the two sets are equal, and for the C5 orbits the element r0 of each
  I set order the set-order picks of both restorers' position loops at the D0 state, how many differ from the least
             unsolved position, and whether the slot model of a CPython 3.13 set (SlotSet) gives the same picks; and a
             fuzz test of that model against the running interpreter's set
"""
import argparse
import array
import gzip
import hashlib
import json
import os
import sys
import time
from pathlib import Path

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--out', required=True, type=Path)
parser.add_argument('--cache', required=True, type=Path)
parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[4])
args = parser.parse_args()
ROOT = args.root.resolve()
CACHE = args.cache.resolve()
CACHE.mkdir(parents=True, exist_ok=True)
os.environ['JUMBLING_HOME_CACHE'] = str(CACHE / 'home_centroids.npy')
for name in ('OPENBLAS_NUM_THREADS', 'OMP_NUM_THREADS', 'MKL_NUM_THREADS'):
    os.environ[name] = '1'
J = ROOT / 'research/jumbling'
G9 = J / 'restoration/09-general-method'
S03 = J / 'restoration/03-retained-group-and-lattice-restorer/scripts'
SCH = G9 / 'scripts/schedule11'
S13 = J / 'restoration/13-tutorial-runner/scripts'
D0 = J / 'restoration/13-tutorial-runner/runs/dev/d0-s7106/run'
F1 = J / 'restoration/13-tutorial-runner/runs/depth20-seed7106/lattice'
D40 = J / 'restoration/12-low-height/runs/round22/line-a/literal/lattice'
# The schedule11 restorer's import order (restore_schedule11_late.py:41-43): sched11 first, then the stage 03 kit.
sys.path[:0] = [str(SCH), str(S03), str(J / 'theory'), str(J), str(J / 'fixtures'), str(S13)]
sys.dont_write_bytecode = True
import numpy as np  # noqa: E402
import kit as KIT  # noqa: E402
import rlib  # noqa: E402
import sched11  # noqa: E402
import lattice_phase as LP  # noqa: E402
import literal_lattice as LL  # noqa: E402
import runner_api as API  # noqa: E402
from rlib import Retained, NS, invword  # noqa: E402
from kit import SLP, OrbitKit  # noqa: E402

T0 = time.time()
HG = {0: 'C2', 3: 'C2', 8: 'C2', 11: 'C2', 22: 'C2', 25: 'C2', 32: 'C2', 17: 'C5', 28: 'C5', 6: 'D5', 34: 'A5'}
CONVENTIONS = ('as card1-certificates-v1, plus {"op":"path","orbit":o,"node":n}: '
               'the execution-tree path from the root to node n')


def sha_file(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def sha_bytes(data):
    return hashlib.sha256(data).hexdigest()


def read_json(path):
    raw = Path(path).read_bytes()
    return json.loads(gzip.decompress(raw) if raw[:2] == b'\x1f\x8b' else raw)


def rel(path):
    return Path(path).resolve().relative_to(ROOT).as_posix()


out = {'format': 'tutorial-ch06-examples/1',
       'scripts': {rel(p): sha_file(p) for p in (
           SCH / 'sched11.py', SCH / 'restore_schedule11_late.py', S03 / 'kit.py', S03 / 'rlib.py',
           G9 / 'scripts/lattice/scripts/kit.py', G9 / 'scripts/lattice/scripts/rlib.py',
           G9 / 'scripts/lattice/scripts/restore_lattice_late.py', S13 / 'lattice_phase.py',
           S13 / 'literal_lattice.py', S13 / 'runner_api.py')},
       'kit_copies_identical': sha_file(S03 / 'kit.py') == sha_file(G9 / 'scripts/lattice/scripts/kit.py')
       and sha_file(S03 / 'rlib.py') == sha_file(G9 / 'scripts/lattice/scripts/rlib.py'),
       'kit_imported': rel(KIT.__file__), 'rlib_imported': rel(rlib.__file__)}

R = Retained(str(ROOT))
TREES = LL.read_json(ROOT / 'assets/execution_trees.json.gz')
REALIGN = G9 / 'data/realign.json.gz'
RJ = json.load(gzip.open(REALIGN, 'rt'))
REPORT = read_json(D0 / 'lattice/report.json')
RUN = read_json(D0 / 'run.json')
ROWS = [s for s in RUN['steps'] if s['phase'] == 'lattice']
LATE_SUMMARY = read_json(F1 / 'restore-summary.json')
LATE_DOC = read_json(F1 / 'solutions.json.gz')

# ---- A: the cascade order ------------------------------------------------------------------------------------
# The late restorer's own construction, in its own node order (restore_lattice_late.py:59-80), so that section E's
# rerun builds the same program as the committed record.
slpL = SLP(R)
coll, seed_node = {}, {}
for o in range(35):
    seed_node[o] = slpL.word(R.trees[o]['seed'])
    coll[o] = sorted(set(int(x) for x in R.so[slpL.net[seed_node[o]][0]]) - {o})
rounds, done = [], set()
while len(done) < 35:
    batch = [o for o in range(35) if o not in done and set(coll[o]) <= done]   # tested against the list before
    rounds.append(batch)
    done |= set(batch)
cascade = [o for b in rounds for o in b][::-1]
scan, seen = [], set()
while len(scan) < 35:
    for o in range(35):                                                        # tested against the growing list
        if o not in seen and set(coll[o]) <= seen:
            scan.append(o)
            seen.add(o)
scan_cascade = scan[::-1]
out['A_cascade'] = {
    'coll': {str(o): coll[o] for o in range(35)},
    'rounds': rounds, 'cascade': cascade, 'one_pass_scan_cascade': scan_cascade,
    'scan_differs_at': [k for k in range(35) if scan_cascade[k] != cascade[k]],
    'equals_realign_cascade_order': cascade == RJ['cascade_order'],
    'equals_late_record': cascade == LATE_SUMMARY['cascade_order'],
    'equals_d0_report': cascade == REPORT['cascade_order'],
}

# ---- B and C: schedule11 at the D0 lattice start ---------------------------------------------------------------
slp = SLP(R)
S = sched11.Schedule11(R, slp, RJ)
sim, load_table, ctx, menu = LL.simulator()
state = sim.State(ctx, menu=menu)
load_table(state, read_json(D0 / 'tables/bulk-end.json'))
L = np.asarray(LL.state_labels(state), np.int64)
L_start = L.copy()
start = {'table': rel(D0 / 'tables/bulk-end.json'), 'digest': state.digest(),
         'labels_sha256': sha_bytes(L.astype('<i8').tobytes()),
         'moved_pieces': int(len(np.unique(R.sp[np.flatnonzero(L != np.arange(NS))]))),
         'first_lattice_row_before': ROWS[0]['before']['digest']}


class SlotSet:
    """The iteration order of a CPython 3.13 set of non-negative integers below 2**61 - 1 (hash = value),
    as section 6.7 of the tutorial states it; checked against the running interpreter by fuzz_slot_set."""
    EMPTY, DELETED = object(), object()

    def __init__(self):
        self.table, self.fill, self.used = [self.EMPTY] * 8, 0, 0

    @staticmethod
    def probe(key, size):
        i, perturb = key % size, key
        while True:
            yield from (range(i, i + 10) if i + 9 <= size - 1 else (i,))
            perturb >>= 5
            i = (5 * i + 1 + perturb) % size

    def add(self, key):
        deleted = None
        for j in self.probe(key, len(self.table)):
            slot = self.table[j]
            if slot is self.EMPTY:
                break
            if slot is self.DELETED:
                deleted = j                       # the last deleted slot passed before the first empty one
            elif slot == key:
                return
        if deleted is not None:
            self.table[deleted] = key
            self.used += 1
            return
        self.table[j] = key
        self.fill += 1
        self.used += 1
        if 5 * self.fill >= 3 * (len(self.table) - 1):
            minused = self.used * (2 if self.used > 50000 else 4)
            size = 8
            while size <= minused:
                size *= 2
            old, self.table = self.table, [self.EMPTY] * size
            for key_ in old:
                if key_ is not self.EMPTY and key_ is not self.DELETED:
                    self.table[next(j for j in self.probe(key_, size) if self.table[j] is self.EMPTY)] = key_
            self.fill = self.used

    def discard(self, key):
        for j in self.probe(key, len(self.table)):
            slot = self.table[j]
            if slot is self.EMPTY:
                return
            if slot is not self.DELETED and slot == key:
                self.table[j] = self.DELETED
                self.used -= 1
                return

    def __iter__(self):
        return (s for s in self.table if s is not self.EMPTY and s is not self.DELETED)

    def __len__(self):
        return self.used


def fuzz_slot_set(sequences=40, operations=3000):
    """SlotSet against this interpreter's set: random adds and discards, iteration order after every operation."""
    import random
    mismatches = 0
    for seed in range(sequences):
        rng = random.Random(seed)
        span = (200, 5000, 180000, 1000000)[seed % 4]
        real, model, keys = set(), SlotSet(), []
        for _ in range(rng.randrange(1, 3000)):
            key = rng.randrange(span)
            real.add(key)
            model.add(key)
            keys.append(key)
        ok = list(real) == list(model)
        for _ in range(operations if ok else 0):
            key = rng.randrange(span) if rng.random() < 0.5 else rng.choice(keys)
            if rng.random() < (0.3, 0.5, 0.7)[seed % 3]:
                real.add(key)
                model.add(key)
                keys.append(key)
            else:
                real.discard(key)
                model.discard(key)
            if list(real) != list(model):
                ok = False
                break
        mismatches += not ok
    return {'python': sys.version.split()[0], 'implementation': sys.implementation.name,
            'sequences': sequences, 'operations_each': operations, 'mismatching_sequences': mismatches}


def loop_with_picks(cur, A, make=None):
    """sched11._loop_cycles (:116-141), with a log of the two set-order picks (:124, :129). With make=SlotSet the
    set of unsolved positions is the model of section 6.7 instead of the interpreter's set."""
    cur = cur.copy()
    if make is None:
        unsolved = {p for p, h in cur.items() if p != h}
    else:
        unsolved = make()
        for p, h in cur.items():
            if p != h:
                unsolved.add(p)
    out_, picks = [], []
    while unsolved:
        h = cur[A]
        if h == A:
            X = next(iter(unsolved))
            picks.append({'case': 'A home', 'picked': X, 'least': min(unsolved), 'size': len(unsolved)})
            Y = cur[X]
        else:
            X, Y = h, cur[h]
            if Y == A:
                rest = [p for p in unsolved if p not in (A, X)]
                Y = rest[0] if rest else None
                picks.append({'case': 'Y = A', 'picked': Y, 'least': min(rest) if rest else None,
                              'size': len(rest)})
                if Y is None:
                    raise ValueError('odd position permutation')
        out_.append((A, X, Y))
        cur[A], cur[X], cur[Y] = cur[Y], cur[A], cur[X]
        for p in (A, X, Y):
            if cur[p] == p:
                unsolved.discard(p)
            else:
                unsolved.add(p)
    return out_, picks


# Factor logs for section D: wrap the constructions; every wrapper recomputes the construction's choices with the
# same formulas and creates no SLP node of its own.
mul, inv = sched11.mul, sched11.inv
LOG = {}
LOGGED = {6: None, 0: None, 17: None, 34: None, 32: None, 15: 2, 21: 2}
current = {'orbit': None}


def setup_parts(k, X, Y, Z):
    w1 = [] if X == k.A else list(k.routes['a'][X])
    y, z = k.R.sp[k.R.map_through([k.lab[Y][0], k.lab[Z][0]], w1)]
    w2 = list(k.routes['e'][int(y)])
    z2 = int(k.R.sp[k.R.map_through([k.lab[int(z)][0]], w2)[0]])
    return {'w1': len(w1), 'w2': len(w2), 'path': len(k.path(k.node_at[z2])), 'z_after_w1_w2': z2}


def h_index(k, x):
    return k.H.index(x)


def record(kind, k, **fields):
    o = current['orbit']
    if o not in LOGGED:
        return
    items = LOG.setdefault(o, [])
    if LOGGED[o] is not None and len(items) >= LOGGED[o]:
        return
    items.append(dict(kind=kind, **fields))


def wrap(cls):
    orig = {name: getattr(cls, name) for name in
            ('direct_cycle', 'twisted_cycle', 'direct_pair', 'c2_pair', 'frame_triple', 'cleanup_A', 'cleanup_B',
             'single')}

    def direct_cycle(self, xyz, g):
        node = orig['direct_cycle'](self, xyz, g)
        record('direct-three-cycle', self, positions=list(xyz), g=len(g), setup=setup_parts(self, *xyz),
               length=self.slp.length[node], seed=self.s)
        return node

    def twisted_cycle(self, xyz, g, L_, finalized):
        self.prepare()
        maps = [self.fibre(g, p, q) for p, q in zip(xyz, (self.A, self.B, self.C))]
        u, v = self.alpha, self.beta
        if xyz[1] in finalized:
            u = mul(inv(maps[0]), self.needed(L_, xyz[0], xyz[1]), maps[1])
        if xyz[2] in finalized:
            v = mul(inv(maps[1]), self.needed(L_, xyz[1], xyz[2]), maps[2])
        y = next(y for y in self.H if mul(self.alpha, y) == u)
        x = next(x for x in self.H if mul(inv(y), self.beta, x) == v)
        fin = sorted(int(p) for p in finalized)
        node = orig['twisted_cycle'](self, xyz, g, L_, finalized)
        record('twisted-three-cycle', self, positions=[int(p) for p in xyz], finalized=fin, g=len(g),
               setup=setup_parts(self, *xyz), y=list(y), x=list(x), y_index=h_index(self, y),
               x_index=h_index(self, x), at_B_y=len(self.at_B[y]), return_x=len(self.returns[x]),
               r=len(self.at_B[y]) + len(self.returns[x]), length=self.slp.length[node], seed=self.s)
        return node

    def direct_pair(self, X, v):
        self.prepare()
        g = list(self.routes['f'][X])
        a = self.fibre(g, X, self.B)
        x = next(x for x in self.H if mul(a, self.beta, x, inv(self.beta), inv(a)) == v)
        node = orig['direct_pair'](self, X, v)
        record('direct-pair', self, positions=[int(X), int(self.C)], value=list(v), g=len(g), x=list(x),
               x_index=h_index(self, x), return_x=len(self.returns[x]), length=self.slp.length[node], seed=self.s)
        return node

    def c2_pair(self, X, Y, v, other):
        self.prepare()
        w1 = list(self.routes['b'][Y])
        xx = int(self.R.sp[self.R.map_through([self.lab[X][0]], w1)[0]])
        g = w1 + list(self.routes['f'][xx])
        a = self.fibre(g, X, self.B)
        x = next(x for x in self.H if mul(a, self.beta, x, inv(self.beta), inv(a)) == v)
        node = orig['c2_pair'](self, X, Y, v, other)
        record('c2-pair', self, positions=[int(X), int(Y)], g=len(g), b_route=len(w1), f_route=len(g) - len(w1),
               x=list(x), return_x=len(self.returns[x]), length=self.slp.length[node], seed=self.s)
        return node

    def frame_triple(self, X, Y, va, vb):
        self.prepare()
        xyz = (X, Y, self.C)
        g = self.setup(*xyz)
        a, b, _ = [self.fibre(g, p, q) for p, q in zip(xyz, (self.A, self.B, self.C))]
        y = next(y for y in self.H if mul(a, self.alpha, y, inv(self.alpha), inv(a)) == va)
        x = next(x for x in self.H if mul(b, inv(y), self.beta, x, inv(self.beta), inv(b)) == vb)
        node = orig['frame_triple'](self, X, Y, va, vb)
        record('frame-triple', self, positions=[int(X), int(Y), int(self.C)], g=len(g),
               setup=setup_parts(self, *xyz), y_index=h_index(self, y), x_index=h_index(self, x),
               r=len(self.at_B[y]) + len(self.returns[x]), length=self.slp.length[node], seed=self.s)
        return node

    def cleanup_A(self, v):
        self.prepare()
        x = next(x for x in self.H if mul(inv(self.gamma), inv(x), self.gamma) == v)
        node = orig['cleanup_A'](self, v)
        record('cleanup-A', self, value=list(v), x=list(x), x_index=h_index(self, x),
               return_x=len(self.returns[x]), length=self.slp.length[node], seed=self.s)
        return node

    def cleanup_B(self, v):
        self.prepare()
        x = next(x for x in self.H if mul(self.beta, x, inv(self.beta)) == v)
        node = orig['cleanup_B'](self, v)
        record('cleanup-B', self, value=list(v), x=list(x), return_x=len(self.returns[x]),
               length=self.slp.length[node], seed=self.s)
        return node

    def single(self, v):
        self.prepare()
        cost, x, y, z = self.single_choices[v]
        node = orig['single'](self, v)
        record('single', self, value=list(v), cost=cost, x_index=h_index(self, x), y_index=h_index(self, y),
               z_index=h_index(self, z), returns=[len(self.returns[t]) for t in (x, y, z)],
               length=self.slp.length[node], seed=self.s)
        return node

    for name, fn in (('direct_cycle', direct_cycle), ('twisted_cycle', twisted_cycle),
                     ('direct_pair', direct_pair), ('c2_pair', c2_pair), ('frame_triple', frame_triple),
                     ('cleanup_A', cleanup_A), ('cleanup_B', cleanup_B), ('single', single)):
        setattr(cls, name, fn)


wrap(sched11._Orbit11)

per_orbit, factors = [], []
node_start = len(slp.nodes)
for o in S.cascade:
    k = S.kits[o]
    current['orbit'] = o
    cur = {p: int(R.sp[L[sl[0]]]) for p, sl in k.lab.items()}
    loop, picks = loop_with_picks(cur, k.A)
    assert loop == sched11._loop_cycles(cur, k.A)
    loop_m, picks_m = loop_with_picks(cur, k.A, SlotSet)
    unsolved = sum(1 for p, h in cur.items() if p != h)
    fixed_errors = [p for p in k.lab if p not in (k.A, k.B, k.C) and cur[p] == p and k.needed(L, p, p) != k.ident]
    entry = {'orbit': o, 'H': k.group_name, 'unsolved_positions': unsolved, 'fixed_errors': len(fixed_errors),
             'loop_cycles': len(loop), 'set_order_picks': len(picks), 'first_picks': picks[:3],
             'picks_not_least': sum(1 for q in picks if q['picked'] != q['least']),
             'slot_model_equal': loop_m == loop and picks_m == picks}
    if k.group_name == '1':
        fewest = sched11._fewest_cycles(cur)
        entry['fewest_cycles'] = len(fewest)
        entry['plan_costs'] = {m: sum(k.s + 2 * len(k.setup(*xyz)) for xyz in t)
                               for m, t in (('direct-loop', loop), ('fewest-three-cycles', fewest))}
    fs = S.solve_orbit(o, L)
    entry.update(method=S.stats[o]['method'], counts=S.stats[o]['counts'], factors=len(fs),
                 letters=int(sum(slp.length[i] for i, _ in fs)))
    per_orbit.append(entry)
    factors += fs
assert np.array_equal(L, np.arange(NS))
used, stack = set(), [i for i, _ in factors]
while stack:
    i = stack.pop()
    if i in used:
        continue
    used.add(i)
    nd = slp.nodes[i]
    if nd['op'] == 'conj':
        stack += [nd['x'], nd['u']]
    elif nd['op'] == 'prod':
        stack += [f[0] for f in nd['f']]
keep = sorted(used)
remap = {old: new for new, old in enumerate(keep)}


def renumber(nd):
    if nd['op'] in ('word', 'path'):
        return nd
    if nd['op'] == 'conj':
        return {'op': 'conj', 'x': remap[nd['x']], 'u': remap[nd['u']], 'us': nd['us']}
    return {'op': 'prod', 'f': [[remap[a], b] for a, b in nd['f']]}


TAG = 'table:start-table.json'.replace(':', '_').replace('/', '_')[-60:]
doc = {'format': 'lattice-restoration-v1', 'model_files': R.files, 'conventions': CONVENTIONS,
       'nodes': [renumber(slp.nodes[i]) for i in keep],
       'solutions': {TAG: [[remap[i], sg] for i, sg in factors]}}
solutions_sha = sha_bytes(json.dumps(doc).encode())

# Words and trees of every step, as the player and orbit_tree make them, against the committed rows.
OL = LL.ObserverLetters(sim, ctx, menu)
program = LL.Program(doc['nodes'], TREES)
flist = doc['solutions'][TAG]
partition = []
first = 0
for o in S.cascade:
    n = next(e['factors'] for e in per_orbit if e['orbit'] == o)
    if n:
        partition.append((o, first, n))
    first += n


def word_and_tree(program_, flist_, orbit, first_, n):
    codes = array.array('H', (OL.primitives[m] for j, s in flist_[first_:first_ + n] for m in program_.expand(j, s)))
    assert sys.byteorder == 'little'
    tree, folded, depth = LP.orbit_tree(program_, flist_[first_:first_ + n], orbit)
    size = API.check_tree(tree, len(codes))
    data = (json.dumps(tree, separators=(',', ':'), allow_nan=False) + '\n').encode('utf-8')
    return codes, tree, {'letters': len(codes), 'word_sha256': sha_bytes(codes.tobytes()),
                         'tree_sha256': sha_bytes(data), 'tree_size': size, 'tree_folded': folded,
                         'tree_folded_depth': depth}


steps = []
TREE_OF = {}
for (o, first_, n), row in zip(partition, ROWS):
    codes, tree, info = word_and_tree(program, flist, o, first_, n)
    TREE_OF[o] = tree
    match = (row['rule'] == f'lattice/orbit-{o}' and row['factor_first'] == first_ and row['factor_count'] == n
             and row['letters'] == info['letters'] and row['word_sha256'] == info['word_sha256']
             and row['tree']['sha256'] == info['tree_sha256'] and row['tree']['size'] == info['tree_size']
             and row['tree_folded'] == info['tree_folded'] and row['tree_folded_depth'] == info['tree_folded_depth'])
    steps.append({'index': row['index'], 'rule': row['rule'], 'factor_first': first_, 'factor_count': n,
                  'letters': info['letters'], 'tree_size': info['tree_size'], 'before': row['before']['digest'],
                  'after': row['after']['digest'], 'matches_committed_row': match})
terms = S.orbit_bounds()
for e in per_orbit:
    e['term'] = terms[e['orbit']]
    e['within_term'] = e['letters'] <= e['term']

out['C_s11'] = {
    'record': rel(D0 / 'run.json'), 'start': start,
    'solutions_sha256': solutions_sha, 'committed_solutions_sha256': REPORT['solutions_sha256'],
    'solutions_match': solutions_sha == REPORT['solutions_sha256'],
    'nodes': len(keep), 'new_nodes': len(slp.nodes) - node_start, 'factors': len(factors),
    'retained_length': int(sum(e['letters'] for e in per_orbit)), 'labels_identity_after': True,
    'sum_of_terms': int(sum(terms.values())), 'bound': sched11.BOUND,
    'per_orbit': per_orbit, 'steps': steps,
    'all_rows_match': all(s['matches_committed_row'] for s in steps) and len(steps) == len(ROWS),
    'rows': len(ROWS),
}


def compact(part, depth=0, limit=None, rows=None):
    """One line per part: depth, name, letters and reference, in played order."""
    rows = [] if rows is None else rows
    line = {'depth': depth, 'name': part['name'], 'letters': part['letters']}
    for key in ('inverse_of', 'same_as'):
        if key in part:
            line[key] = part[key]
    rows.append(line)
    for child in part.get('parts', []):
        if limit is not None and len(rows) >= limit:
            break
        compact(child, depth + 1, limit, rows)
    return rows


factor_trees = {}
for o, items in LOG.items():
    tree = TREE_OF[o]
    kinds = {}
    for i, item in enumerate(items):
        kinds.setdefault(item['kind'], i)
    factor_trees[str(o)] = {kind: {'factor': i, 'tree': compact(tree['parts'][i])} for kind, i in kinds.items()}
out['D_factors'] = {'log': {str(o): v for o, v in LOG.items()}, 'first_tree_of_each_kind': factor_trees,
                    'orbit6_tree': compact(TREE_OF[6])}

# ---- B: the orbits ------------------------------------------------------------------------------------------
orbits = []
for o in range(35):
    k, t, rec = S.kits[o], R.trees[o], next(r for r in RJ['orbits'] if r['orbit'] == o)
    tree_positions = len(set(int(p) for p in t['positions']))
    entry = {'orbit': o, 'H': k.group_name, 'N': len(k.lab), 'tree_positions': tree_positions,
             'pieces': int(len(R.orbit_pieces(o))), 'A': k.A, 'B': k.B, 'C': k.C, 'k': k.k,
             'A_B_in_tree': k.A in set(t['positions']) or k.B in set(t['positions']),
             'tree_depth': int(max(t['depth'])), 'seed': len(t['seed']), 'relocation': len(t['relocation']),
             's0': len(t['seed']) + 2 * len(t['relocation']), 'realign_seed_s0': rec['seed_s0'],
             'replaced': rec['replaced'], 's': rec['s'], 'collateral': len(k.allowed),
             'cascade_place': S.cascade.index(o) + 1,
             'a_max': k.routes['a_max'], 'e_max': k.routes['e_max'], 'term': terms[o]}
    if k.group_name != '1':
        entry.update(H_size=len(k.H), h=k.h, swap=len(k.swap), b_max=k.routes['b_max'], f_max=k.routes['f_max'],
                     returns_from_C_nodes=sum(1 for p in t['positions'] if p == k.C) - 1)
    orbits.append(entry)
out['B_orbits'] = {'orbits': orbits, 'replaced': sum(e['replaced'] for e in orbits),
                   'kept': [e['orbit'] for e in orbits if not e['replaced']],
                   'N_is_tree_positions_plus_2': all(e['N'] == e['tree_positions'] + 2 == e['pieces'] for e in orbits),
                   'A_B_never_tree_nodes': not any(e['A_B_in_tree'] for e in orbits),
                   'realign_sha256_gz': sha_file(REALIGN),
                   'realign_sha256_decompressed': sha_bytes(gzip.decompress(REALIGN.read_bytes()))}

# ---- E: the late restorer at the same state --------------------------------------------------------------------
cascadeL = cascade
kitsL = {}
for o in cascadeL:
    w = [int(m) for m in R.trees[o]['relocation']]
    reloc = slpL._add({'op': 'word', 'w': w}, (np.zeros(0, np.int64), np.zeros(0, np.int64)), len(w))
    slpL.word_of[reloc] = w
    c = slpL.conj(seed_node[o], reloc)
    fa, fb, fc = [np.array(f, np.int64) for f in R.atlas[o]['frames']]
    for a, b in ((fa, fb), (fb, fc), (fc, fa)):
        assert np.array_equal(SLP.apply_sparse(a, slpL.net[c]), b)
    kitsL[o] = OrbitKit(R, slpL, o, c, allowed=set(coll[o]), compact_paths=True)
for o in range(35):                      # ab_maps() builds the pairs of the frame orbits first (:94-102)
    if o in HG:
        kitsL[o]._pairs()
L = L_start.copy()
late_factors, late_per = [], []
startL = len(slpL.nodes)
for o in cascadeL:
    k = kitsL[o]
    cur = {P: int(R.sp[L[k.lab[P][0]]]) for P in k.lab}
    loop, picks = loop_with_picks(cur, k.A)
    loop_m, picks_m = loop_with_picks(cur, k.A, SlotSet)
    fs = k.solve(L)
    late_per.append({'orbit': o, 'factors': len(fs), 'letters': int(sum(slpL.length[i] for i, _ in fs)),
                     'loop_cycles': len(loop), 'set_order_picks': len(picks), 'first_picks': picks[:2],
                     'picks_not_least': sum(1 for q in picks if q['picked'] != q['least']),
                     'slot_model_equal': loop_m == loop and picks_m == picks})
    late_factors += fs
assert np.array_equal(L, np.arange(NS))
used, stack = set(), [i for i, _ in late_factors]
while stack:
    i = stack.pop()
    if i in used:
        continue
    used.add(i)
    nd = slpL.nodes[i]
    if nd['op'] == 'conj':
        stack += [nd['x'], nd['u']]
    elif nd['op'] == 'prod':
        stack += [f[0] for f in nd['f']]
keep = sorted(used)
remap = {old: new for new, old in enumerate(keep)}
late_nodes = [renumber(slpL.nodes[i]) for i in keep]
late_flist = [[remap[i], sg] for i, sg in late_factors]
committed_flist = list(LATE_DOC['solutions'].values())
programL = LL.Program(LATE_DOC['nodes'], TREES)
flistL = committed_flist[0]
late_steps, first_ = [], 0
for e in late_per:
    if e['factors']:
        codes, tree, info = word_and_tree(programL, flistL, e['orbit'], first_, e['factors'])
        if e['orbit'] == 6:
            late_tree6, late_info6 = tree, info
        late_steps.append({'orbit': e['orbit'], 'factors': e['factors'], 'letters': info['letters'],
                           'tree_size': info['tree_size'], 'tree_folded': info['tree_folded']})
    first_ += e['factors']
ops = {}
for nd in LATE_DOC['nodes']:
    ops[nd['op']] = ops.get(nd['op'], 0) + 1
out['E_late'] = {
    'record': rel(F1 / 'solutions.json.gz'),
    'program_matches_committed': late_nodes == LATE_DOC['nodes'] and len(committed_flist) == 1
    and late_flist == committed_flist[0],
    'factors': len(late_factors), 'retained_length': int(sum(e['letters'] for e in late_per)),
    'committed_nodes': len(LATE_DOC['nodes']), 'node_ops': ops,
    'per_orbit': late_per, 'steps': late_steps,
    'largest_tree': max(late_steps, key=lambda s: s['tree_size']),
    'orbit6': {'word_sha256': late_info6['word_sha256'], 'tree_sha256': late_info6['tree_sha256'],
               'letters': late_info6['letters'], 'tree_size': late_info6['tree_size'],
               'first_four_factors': [compact(p) for p in late_tree6['parts'][:4]],
               'factor_8': compact(late_tree6['parts'][8])},
}

# ---- F: the depth 40 program of stage 12 ---------------------------------------------------------------------
summary40 = read_json(D40 / 'restore-summary.json')
doc40 = read_json(D40 / 'solutions.json.gz')
library = json.loads(gzip.decompress((G9 / 'data/library-merged.json.gz').read_bytes()))
plan40 = LP.expected_steps(summary40, library)
program40 = LL.Program(doc40['nodes'], TREES)
flist40 = list(doc40['solutions'].values())[0]
sizes40 = []
for o, first_, n in LP.orbit_partition(summary40):
    if n:
        tree, folded, depth = LP.orbit_tree(program40, flist40[first_:first_ + n], o)
        sizes40.append({'orbit': o, 'factors': n, 'letters': tree['letters'], 'tree_size': API.tree_size(tree),
                        'folded': folded, 'folded_depth': depth})
run40 = summary40['runs'][0]
out['F_depth40'] = {
    'record': rel(D40 / 'restore-summary.json'), 'nu_input': run40['nu_input'],
    'moved_pieces_input': run40['moved_pieces_input'], 'order': run40['order'],
    'corrections': [dict(c, family=library['macros'][c['macro']]['family'],
                         letters=c['times'] * len(library['macros'][c['macro']]['records']))
                    for c in run40['corrections']],
    'correction_twists': run40['correction_twists'], 'T_digest': run40['T_digest'],
    'steps': len(plan40), 'rules': [r for r, _ in plan40],
    'orbit_letters': sum(s['letters'] for s in sizes40), 'factors': len(flist40), 'nodes': len(doc40['nodes']),
    'largest_tree': max(sizes40, key=lambda s: s['tree_size']), 'any_fold': any(s['folded'] or s['folded_depth']
                                                                              for s in sizes40),
}

# ---- G: the correction library ------------------------------------------------------------------------------
raw = (G9 / 'data/library-merged.json.gz').read_bytes()
C2F = [0, 3, 6, 8, 11, 22, 25, 32]
kinds = []
for i, m in enumerate(library['macros']):
    bits = any(v[0] or (int(o) in C2F and v[1]) for o, v in m['nu'].items())
    c5 = any(int(o) in (17, 28) and v[1] for o, v in m['nu'].items())
    kinds.append({'macro': i, 'family': m['family'], 'twists': len(m['records']), 'bits': bits, 'c5': c5})
bit = [x for x in kinds if x['bits'] and not x['c5']]
c5 = [x for x in kinds if x['c5'] and not x['bits']]
out['G_library'] = {
    'sha256_gz': sha_bytes(raw), 'sha256_decompressed': sha_bytes(gzip.decompress(raw)), 'menu': library['menu'],
    'macros': len(kinds), 'bit_macros': len(bit), 'bit_twists': sum(x['twists'] for x in bit),
    'bit_min': min(x['twists'] for x in bit), 'bit_max': max(x['twists'] for x in bit),
    'c5_macros': [(x['macro'], x['family'], x['twists']) for x in c5],
    'both': [x['macro'] for x in kinds if x['bits'] and x['c5']],
    'bound_AU05': sum(x['twists'] for x in bit) + sum(2 * x['twists'] for x in c5),
    'macro0': kinds[0]['family'],
}

# ---- H: the frame groups and r0 --------------------------------------------------------------------------------
def order_of(x):
    e, y, n = tuple(range(len(x))), x, 1
    while y != e:
        y, n = tuple(x[y[i]] for i in range(len(x))), n + 1
    return n


frames = []
for o in sorted(HG):
    kL, kS = kitsL[o], S.kits[o]
    vals = list(kL.pair_by_value)                 # restore_lattice_late.py:102-114, the closure of the pair values
    HL, fr = set(vals), list(vals)
    while fr:
        nx = []
        for x in fr:
            for g in vals:
                y = tuple(g[x[i]] for i in range(kL.k))
                if y not in HL:
                    HL.add(y)
                    nx.append(y)
        fr = nx
    kS.prepare()
    HS = set(kS.H)                                # restore_schedule11_late.py:90-91, the tree returns
    entry = {'orbit': o, 'H': HG[o], 'size_late': len(HL), 'size_schedule11': len(HS), 'equal': HL == HS,
             'identity': list(range(kL.k))}
    if HG[o] == 'C5':
        r0L = min(h for h in HL if order_of(h) == 5)
        r0S = min(h for h in HS if order_of(h) == 5)
        entry.update(r0_late=list(r0L), r0_schedule11=list(r0S), elements=[list(h) for h in sorted(HS)])
    frames.append(entry)
out['H_frames'] = {'orbits': frames, 'all_equal': all(e['equal'] for e in frames)}

# ---- I: the set-order picks ------------------------------------------------------------------------------------
s11_picks = [e for e in out['C_s11']['per_orbit']]
out['I_set_order'] = {
    'fuzz': fuzz_slot_set(),
    'schedule11': {'picks': sum(e['set_order_picks'] for e in s11_picks),
                   'not_least': sum(e['picks_not_least'] for e in s11_picks),
                   'model_equal_all_orbits': all(e['slot_model_equal'] for e in s11_picks)},
    'late': {'picks': sum(e['set_order_picks'] for e in late_per),
             'not_least': sum(e['picks_not_least'] for e in late_per),
             'model_equal_all_orbits': all(e['slot_model_equal'] for e in late_per)},
}

out['seconds'] = round(time.time() - T0, 1)
args.out.parent.mkdir(parents=True, exist_ok=True)
args.out.write_text(json.dumps(out, indent=1) + '\n')
print(json.dumps({'seconds': out['seconds'], 'solutions_match': out['C_s11']['solutions_match'],
                  'all_rows_match': out['C_s11']['all_rows_match'],
                  'late_program_matches': out['E_late']['program_matches_committed'],
                  'cascade_ok': out['A_cascade']['equals_realign_cascade_order'],
                  'frames_equal': out['H_frames']['all_equal'],
                  'slot_model': out['I_set_order']}))
