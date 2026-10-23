"""Round 11 lattice restorer, copied from restore_lattice_late.py.

Usage: restore_schedule11_late.py <repo root> <research/jumbling dir>
       <REALIGN.json[.gz]> <library json> <out dir> <input>...
Inputs: table:<pose table json>, uniform:<seed>, word:<seed>:<length>,
        w9word:<seed>:<length>, mix:<seed>:<length>:<k>.

Preserve the late restorer's nu coordinates and correction choice C. Build
T = solved . C^-1 in the exact simulator, solve L_T^-1 o L_Y with Schedule11,
then apply C to T and require the solved digest and all 259,800 identity labels.
Every correction twist is admissible and in I_a; every macro ends on the lattice.
Only the retained cascade changes. Products remain chronological.

Research evidence only: headless exact synthetic or simulator-built lattice
configurations, with no Windows/DirectX or performance claim.
"""
import gzip
import hashlib
import json
import random
import sys
import time
from pathlib import Path

import numpy as np

USAGE = ('usage: restore_schedule11_late.py <repo root> <research/jumbling dir> '
         '<REALIGN.json[.gz]> <library json> <out dir> <input>...')
if len(sys.argv) < 7:
    raise SystemExit(USAGE)
ROOT, J, REALIGN, LIBJ, OUT = (sys.argv[1], Path(sys.argv[2]).resolve(), Path(sys.argv[3]),
                              sys.argv[4], Path(sys.argv[5]))
INPUTS = sys.argv[6:]
if not REALIGN.is_file():
    raise SystemExit(USAGE + '\nrealign file not found: ' + str(REALIGN))
with (gzip.open(REALIGN, 'rt') if REALIGN.suffix == '.gz' else REALIGN.open()) as f:
    RJ = json.load(f)

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
S03 = HERE.parents[1].parent / '03-retained-group-and-lattice-restorer'
sys.path.insert(0, str(S03 / 'scripts'))
sys.path.insert(0, str(HERE))
from rlib import Retained, NS              # noqa: E402
from kit import SLP                        # noqa: E402
from nucoords import Span, vec, C2F, C5F   # noqa: E402
from sched11 import Schedule11, BOUND      # noqa: E402

for p in (J / 'theory', J, J / 'fixtures'):
    sys.path.insert(0, str(p))
T0 = time.time()
import restore as RS                       # noqa: E402
from sim.state import State, Pose          # noqa: E402
from sim.twists import Twist, primitive    # noqa: E402
from sim.kplus import matrix_from_json     # noqa: E402

ctx = RS.ctx
MENU = RS.menu('I_a')
R = Retained(ROOT)
print('loaded %.1fs' % (time.time() - T0), flush=True)

HG = {0: 'C2', 3: 'C2', 8: 'C2', 11: 'C2', 22: 'C2', 25: 'C2', 32: 'C2', 17: 'C5', 28: 'C5', 6: 'D5', 34: 'A5'}

# ---- round 11 cascade; OrbitKit.lab still comes from the committed trees and atlas ------------
slp = SLP(R)
schedule = Schedule11(R, slp, RJ)
cascade, kits = schedule.cascade, schedule.kits

# ---- coordinates and nu --------------------------------------------------------------------
pos_of_slot = np.full(NS, -1, np.int64)
lab_of_slot = np.full(NS, -1, np.int64)
LABS = {}
for o, kit in kits.items():
    for X in kit.lab:
        LABS[X] = kit.lab[X]
        pos_of_slot[kit.lab[X]] = X
        lab_of_slot[kit.lab[X]] = np.arange(len(kit.lab[X]))
assert np.all((pos_of_slot >= 0) == (R.so >= 0))


def ab_maps():
    out = {}
    for o in range(35):
        kit = kits[o]
        k = kit.k
        if o not in HG:
            out[o] = (None, 1)
            continue
        # Enumerate the same frame group from the tree returns, as in the round 11 runner.
        kit.prepare()
        H = set(kit.H)

        def order_of(x):
            e, y, n = tuple(range(k)), x, 1
            while y != e:
                y, n = tuple(x[y[i]] for i in range(k)), n + 1
            return n
        if HG[o] == 'C2':
            out[o] = ({h: int(h != tuple(range(k))) for h in H}, 2)
        elif HG[o] == 'D5':
            out[o] = ({h: int(order_of(h) == 2) for h in H}, 2)
        elif HG[o] == 'C5':
            r0 = min(h for h in H if order_of(h) == 5)
            m, x = {}, tuple(range(k))
            for i in range(5):
                m[x] = i
                x = tuple(r0[x[j]] for j in range(k))
            out[o] = (m, 5)
        else:
            out[o] = ({h: 0 for h in H}, 1)
    return out


AB = ab_maps()
bounds = schedule.orbit_bounds()
assert sum(bounds.values()) == BOUND, ('computed tariffs differ', sum(bounds.values()))


def nu(L):
    """Per orbit (position sign, frame class) of the labelled state L (L[slot] = home slot)."""
    res = {}
    for o in range(35):
        kit = kits[o]
        P = list(kit.lab)
        cur_home = {X: int(R.sp[L[kit.lab[X][0]]]) for X in P}       # home position of piece at X
        seen, par = set(), 0
        for X in P:
            if X in seen:
                continue
            n, Y = 0, X
            while Y not in seen:
                seen.add(Y)
                Y = cur_home[Y]
                n += 1
            par ^= (n - 1) & 1
        tab, mod = AB[o]
        tot = 0
        if tab is not None:
            for X in P:
                Y = cur_home[X]
                # sticker of home label t of piece Y sits at label v(t) of X
                v = [0] * kit.k
                for t_here, s in enumerate(kit.lab[X]):
                    v[int(lab_of_slot[L[s]])] = t_here
                tot += tab[tuple(v)]
        res[o] = (par, tot % mod)
    return res


def nu_nonzero(n):
    return {o: v for o, v in n.items() if v != (0, 0)}


# ---- simulator helpers -----------------------------------------------------------------------
LIB = json.load(open(LIBJ))['macros']
assert LIB[0]['family'] == 'w9'
MACROS = [[Twist.from_record(ctx, r) for r in m['records']] for m in LIB]
assert all(MENU.contains(t) for ws in MACROS for t in ws)
W9 = MACROS[0]


def labels_of(st):
    ls = st.lattice_stickers()
    assert bool(np.all(ls['frame_agrees']))
    return np.asarray(ls['labels'], np.int64)


def apply_w9(st):
    for i, t in enumerate(W9):
        out = st.apply(t)
        if not out.applied:
            raise RuntimeError('w9 step %d not admissible: %s' % (i, out.status))
    return st.is_lattice()


st0 = State(ctx, menu=MENU)
assert apply_w9(st0)
NU_W9 = nu_nonzero(nu(labels_of(st0)))
print('nu(w9)', NU_W9, flush=True)
SPAN = Span()
for i, m in enumerate(LIB):
    st0 = State(ctx, menu=MENU)
    for t in MACROS[i]:
        assert st0.apply(t).applied
    assert st0.is_lattice()
    got = {str(o): list(v) for o, v in nu_nonzero(nu(labels_of(st0))).items()}
    assert got == m['nu'], ('library nu mismatch', i, got, m['nu'])
    SPAN.add(i, *vec({int(o): tuple(v) for o, v in got.items()}))
print('library', len(LIB), 'rank2', len(SPAN.r2), 'rank5', len(SPAN.r5), flush=True)


def apply_macro(st, i, inverse):
    ws = MACROS[i]
    seq = [t.inverse() for t in reversed(ws)] if inverse else ws
    for k, t in enumerate(seq):
        out = st.apply(t)
        if not out.applied:
            raise RuntimeError('macro %d step %d not admissible: %s' % (i, k, out.status))
    assert st.is_lattice(), 'macro left the lattice'


def set_table(st, doc):
    st._poses, st._lookup = [Pose(0, None, ('K', 0))], {('K', 0): 0}
    ids = np.array([st._intern_matrix(matrix_from_json(m)) for m in doc['poses']], np.int32)
    st.pose_id = ids[np.asarray(doc['pose_id'], np.int64)]
    st._compact()
    st.journal, st._anchor_cache = [], {}
    assert st.digest() == doc['digest'], 'table digest mismatch'


def uniform_labels(seed):
    rng = np.random.default_rng(seed)
    L = np.arange(NS, dtype=np.int64)
    for o in range(35):
        kit = kits[o]
        P = np.array(sorted(kit.lab))
        img = rng.permutation(len(P))
        # even permutation
        seen, par = np.zeros(len(P), bool), 0
        for i in range(len(P)):
            if seen[i]:
                continue
            n, j = 0, i
            while not seen[j]:
                seen[j] = True
                j = img[j]
                n += 1
            par ^= (n - 1) & 1
        if par:
            img[[0, 1]] = img[[1, 0]]
        tab, mod = AB[o]
        H = sorted(tab) if tab is not None else [tuple(range(kit.k))]
        frames = [H[i] for i in rng.integers(0, len(H), len(P))]
        if tab is not None:
            tot = sum(tab[h] for h in frames[:-1]) % mod
            need = (-tot) % mod
            frames[-1] = next(h for h in H if tab[h] == need)
        for i, Y in enumerate(P):
            X = int(P[img[i]])
            # home sticker (Y, t) goes to (X, v(t))
            v = frames[i]
            for t in range(kit.k):
                L[kit.lab[X][v[t]]] = kit.lab[Y][t]
    return L


def run(spec):
    t0 = time.time()
    rec = {'input': spec}
    st = None
    if spec.startswith('table:'):
        doc = json.loads(Path(spec[6:]).read_text())
        st = State(ctx, menu=MENU)
        set_table(st, doc)
        rec['input_digest'] = st.digest()
    elif spec.startswith('uniform:'):
        L = uniform_labels(int(spec.split(':')[1]))
    elif spec.startswith('mix:'):
        _, seed, n, k = spec.split(':')
        rng = random.Random(int(seed))
        st = State(ctx, menu=MENU)
        items = [('p', rng.choice([1, -1]) * rng.randrange(1, 1201)) for _ in range(int(n))]
        for _ in range(int(k)):
            items.insert(rng.randrange(len(items) + 1), ('m', rng.randrange(len(LIB)), rng.random() < 0.5))
        planted = []
        for it in items:
            if it[0] == 'p':
                assert st.apply(primitive(ctx, it[1])).applied
            else:
                apply_macro(st, it[1], it[2])
                planted.append({'macro': it[1], 'inverse': it[2]})
        rec['planted'] = planted
        rec['input_digest'] = st.digest()
    else:
        kind, seed, n = spec.split(':')
        rng = random.Random(int(seed))
        st = State(ctx, menu=MENU)
        word = [rng.choice([1, -1]) * rng.randrange(1, 1201) for _ in range(int(n))]
        for m in word:
            assert st.apply(primitive(ctx, m)).applied
        if kind == 'w9word':
            assert apply_w9(st)
        rec['input_digest'] = st.digest()
    if st is not None:
        if not st.is_lattice():
            rec['status'] = 'not a lattice configuration'
            return rec, None
        L = labels_of(st)
    rec['input_labels_sha256'] = hashlib.sha256(L.astype('<i8').tobytes()).hexdigest()
    n_in = nu_nonzero(nu(L))
    rec['nu_input'] = {str(o): list(v) for o, v in n_in.items()}
    rec['moved_pieces_input'] = int(len(np.unique(R.sp[np.flatnonzero(L != np.arange(NS))])))
    rec['corrections'] = []
    if n_in:
        b2, b5 = vec(n_in)
        comb = SPAN.solve(b2, [(-x) % 5 for x in b5])
        if comb is None:
            rec['status'] = 'failure certificate: nu outside the span of the correction library'
            rec['library_rank'] = [len(SPAN.r2), len(SPAN.r5)]
            x, y = SPAN.residual(b2, [(-v) % 5 for v in b5])
            rec['residual'] = {'sign_orbits': [o for o in range(35) if x[o]],
                               'c2_frame_orbits': [o for j, o in enumerate(C2F) if x[35 + j]],
                               'c5_frame': {str(o): y[j] for j, o in enumerate(C5F) if y[j]}}
            return rec, None
        if st is None:
            rec['status'] = 'needs corrections but the input has no pose table'
            return rec, None
        for i, n in sorted(comb.items()):
            m2, m5 = vec({int(o): tuple(v) for o, v in LIB[i]['nu'].items()})
            order = (2 if any(m2) else 1) * (5 if any(m5) else 1)
            n %= order
            if n == 0:
                continue
            inv = n > order // 2
            times = order - n if inv else n
            rec['corrections'].append({'macro': i, 'times': times, 'inverse': inv})
        rec['order'] = 'retained-first'
        rec['correction_twists'] = int(sum(c['times'] * len(MACROS[c['macro']]) for c in rec['corrections']))
        t1 = time.time()
        stT = State(ctx, menu=MENU)                      # T = solved . C^-1
        for c in reversed(rec['corrections']):
            for _ in range(c['times']):
                apply_macro(stT, c['macro'], not c['inverse'])
        rec['T_digest'] = stT.digest()
        rec['T_seconds'] = round(time.time() - t1, 1)
        LT = labels_of(stT)
        assert nu_nonzero(nu(LT)) == n_in, 'nu(T) differs from nu(Y)'
        LTinv = np.empty(NS, np.int64)
        LTinv[LT] = np.arange(NS, dtype=np.int64)
        LY = L
        L = LTinv[LY]                                    # relabelled state L_T^-1 o L_Y
        assert nu_nonzero(nu(L)) == {}, 'relabelled state has nonzero nu'
    rec['w9'] = any(c['macro'] == 0 for c in rec['corrections'])
    L0 = L.copy()
    start = len(slp.nodes)
    factors, per, per_len = [], {}, {}
    for o in cascade:
        f = schedule.solve_orbit(o, L)
        per[o] = len(f)
        per_len[o] = int(sum(slp.length[i] for i, _ in f))
        factors += f
    assert np.array_equal(L, np.arange(NS)), 'not restored'
    assert sum(bounds.values()) == BOUND, ('computed tariffs differ', sum(bounds.values()))
    assert all(per_len[o] <= bounds[o] for o in cascade), 'per-orbit bound exceeded'
    if rec.get('order') == 'retained-first':
        L0 = LY.copy()                                   # stored start labels are those of Y itself
        t1 = time.time()
        for c in rec['corrections']:
            for _ in range(c['times']):
                apply_macro(stT, c['macro'], c['inverse'])
        rec['C_seconds'] = round(time.time() - t1, 1)
        rec['final_digest'] = stT.digest()
        assert rec['final_digest'] == State(ctx, menu=MENU).digest(), 'C did not end at solved'
        assert np.array_equal(labels_of(stT), np.arange(NS)), 'C did not end at solved labels'
    rec['status'] = 'restored'
    rec['factors'] = len(factors)
    rec['factors_per_orbit'] = {str(o): per[o] for o in cascade}
    rec['length_per_orbit'] = {str(o): per_len[o] for o in cascade}
    rec['method_per_orbit'] = {str(o): schedule.stats[o]['method'] for o in cascade}
    rec['counts_per_orbit'] = {str(o): schedule.stats[o]['counts'] for o in cascade}
    rec['bound'] = BOUND
    rec['retained_length'] = int(sum(per_len.values()))
    assert rec['retained_length'] <= BOUND
    rec['new_nodes'] = len(slp.nodes) - start
    rec['seconds'] = round(time.time() - t0, 1)
    return rec, (L0, factors)


OUT.mkdir(parents=True, exist_ok=True)
summary = {'cascade_order': cascade, 'nu_w9': {str(o): list(v) for o, v in NU_W9.items()},
           'library': {'sha256': hashlib.sha256(Path(LIBJ).read_bytes()).hexdigest(), 'macros': len(LIB),
                       'rank2': len(SPAN.r2), 'rank5': len(SPAN.r5)},
           'realign_sha256': hashlib.sha256(REALIGN.read_bytes()).hexdigest(), 'bound': BOUND, 'runs': []}
results = []
for spec in INPUTS:
    rec, sol = run(spec)
    print(json.dumps({k: v for k, v in rec.items() if not k.endswith('_per_orbit')}), flush=True)
    summary['runs'].append(rec)
    if sol is not None:
        tag = spec.replace(':', '_').replace('/', '_')[-60:]
        np.save(OUT / ('start-labels-%s.npy' % tag), sol[0])
        results.append((tag, sol[1]))
# one SLP for all runs
used = set()
stack = [i for _, fs in results for i, _ in fs]
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


def rn(nd):
    if nd['op'] in ('word', 'path'):
        return nd
    if nd['op'] == 'conj':
        return {'op': 'conj', 'x': remap[nd['x']], 'u': remap[nd['u']], 'us': nd['us']}
    return {'op': 'prod', 'f': [[remap[a], b] for a, b in nd['f']]}


doc = {'format': 'lattice-restoration-v1', 'model_files': R.files,
       'conventions': 'as card1-certificates-v1, plus {"op":"path","orbit":o,"node":n}: the execution-tree path from the root to node n',
       'nodes': [rn(slp.nodes[i]) for i in keep],
       'solutions': {tag: [[remap[i], sg] for i, sg in fs] for tag, fs in results}}
(OUT / 'solutions.json').write_text(json.dumps(doc))
(OUT / 'restore-summary.json').write_text(json.dumps(summary, indent=1))
print('done %.1fs nodes %d' % (time.time() - T0, len(keep)))
