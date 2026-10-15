"""Independent replay check of lattice restorations, extended for the corrections-last order (a modified copy of
the lattice package's verify_restore.py; it still shares no code with the restorer).

usage: python3 -I verify_late.py <repo root> <research/jumbling dir> <library json> <restore dir> <report.json>
                                  [--chain <X table json> <word-full json>...]
Runs whose record has order 'retained-first' (restore_lattice_late.py) are checked as follows: the start Y is
rebuilt (from its pose table, or with --chain from the X table by applying the chain words twist by twist, each
an I_a menu element and admissible, every segment digest checked, ending at the run's input digest); the
solution program is evaluated with this file's own code and applied to the labels of Y; independently the
target T = solved . C^-1 is built in the exact simulator from solved (each twist in the menu and admissible,
each macro ending on the lattice) and must have the recorded digest and exactly the labels just obtained; then
C is applied to T twist by twist and must end at the solved configuration (empty digest, identity labels on all
259,800 stickers). With --chain only the run whose input digest is the chain's end digest is checked.
Other runs are checked exactly as in verify_restore.py.
--- original docstring follows ---

For every restored run: rebuild the start configuration from its input (the exact simulator for
pose tables and simulator-generated inputs, the stored labels for synthetic K inputs), replay the
recorded library corrections in the exact simulator (each twist must be admissible and in the I_a
menu, every macro must end on the lattice, the end must match the recorded digest), then evaluate the solution's
retained straight-line program with this file's own code and apply it: the result must be the
identity on all 259,800 stickers. For a failure certificate on a pose table: recompute nu with the
coordinate code of nucoords.py and check by rank that it lies outside the span of the library's nu
(each macro replayed from solved). Shares no code with the restorer.
"""
import gzip
import hashlib
import json
import random
import sys
import time
from pathlib import Path

import numpy as np

T0 = time.time()
root, J, LIBJ, D, OUTP = Path(sys.argv[1]), Path(sys.argv[2]).resolve(), sys.argv[3], Path(sys.argv[4]), sys.argv[5]
CHAIN = sys.argv[7:] if len(sys.argv) > 6 and sys.argv[6] == '--chain' else None
NS = 259800
summary = json.loads((D / 'restore-summary.json').read_text())
sol = json.loads((D / 'solutions.json').read_text())
for name, h in sol['model_files'].items():
    assert hashlib.sha256((root / name).read_bytes()).hexdigest() == h, name
pz = np.load(root / 'assets/primitives.npz', allow_pickle=False)
src, dst, off = pz['src'].astype(np.int64), pz['dst'].astype(np.int64), pz['offsets'].astype(np.int64)
z = np.load(root / 'assets/model.npz', allow_pickle=False)
so = z['orbit_id'].astype(np.int64)[z['slot_piece'].astype(np.int64)]
with gzip.open(root / 'assets/execution_trees.json.gz', 'rt') as f:
    trees = json.load(f)
_mv = {}


def move(m):
    r = _mv.get(m)
    if r is None:
        i = abs(m) - 1
        assert isinstance(m, int) and 0 <= i < 1200
        s, d = src[off[i]:off[i + 1]], dst[off[i]:off[i + 1]]
        if m < 0:
            s, d = d, s
        ix = np.argsort(s)
        r = (s[ix], d[ix])
        _mv[m] = r
    return r


def follow(x, w):
    x = np.array(x, np.int64, copy=True)
    for m in w:
        s, d = move(m)
        ix = np.minimum(np.searchsorted(s, x), len(s) - 1)
        hit = s[ix] == x
        x[hit] = d[ix[hit]]
    return x


def tree_path(o, n):
    t = trees[o]
    out = []
    while t['parent'][n] >= 0:
        out.append(int(t['move'][n]))
        n = t['parent'][n]
    return out[::-1]


def srt(s, d):
    ix = np.argsort(s)
    return s[ix], d[ix]


nodes = sol['nodes']
net, word, length = {}, {}, []
L = np.arange(NS, dtype=np.int64)
for i, nd in enumerate(nodes):
    op = nd['op']
    if op in ('word', 'path'):
        w = nd['w'] if op == 'word' else tree_path(nd['orbit'], nd['node'])
        word[i] = w
        length.append(len(w))
    elif op == 'conj':
        x, u, us = nd['x'], nd['u'], nd['us']
        if x not in net:                       # a word used as the conjugated element
            w = word[x]
            for m in w:
                s, d = move(m)
                L[d] = L[s]
            y = np.flatnonzero(L != np.arange(NS))
            net[x] = srt(L[y].copy(), y)
            L[:] = np.arange(NS)
        s, d = net[x]
        if u in word:
            w = word[u] if us > 0 else [-m for m in reversed(word[u])]
            net[i] = srt(follow(s, w), follow(d, w))
        else:
            us_s, us_d = net[u] if us > 0 else srt(net[u][1], net[u][0])

            def ap(a):
                a = a.copy()
                ix = np.minimum(np.searchsorted(us_s, a), max(len(us_s) - 1, 0))
                hit = us_s[ix] == a if len(us_s) else np.zeros(len(a), bool)
                a[hit] = us_d[ix[hit]]
                return a
            net[i] = srt(ap(s), ap(d))
        length.append(length[x] + 2 * length[u])
    elif op == 'prod':
        touched = []
        for j, sg in nd['f']:
            s, d = net[j] if sg > 0 else srt(net[j][1], net[j][0])
            L[d] = L[s]
            touched.append(d)
        t = np.unique(np.concatenate(touched))
        lt = L[t]
        mv_ = lt != t
        net[i] = srt(lt[mv_].copy(), t[mv_])
        L[t] = t
        length.append(sum(length[j] for j, _ in nd['f']))
    else:
        raise ValueError(op)
print('SLP evaluated: %d nodes, %.1fs' % (len(nodes), time.time() - T0), flush=True)

# ---- simulator for inputs and w9 -------------------------------------------------------------
for p in (J / 'theory', J, J / 'fixtures'):
    sys.path.insert(0, str(p))
sys.dont_write_bytecode = True
import restore as RS                        # noqa: E402
from sim.state import State, Pose           # noqa: E402
from sim.twists import Twist, primitive     # noqa: E402
from sim.kplus import matrix_from_json      # noqa: E402
ctx = RS.ctx
MENU = RS.menu('I_a')
assert hashlib.sha256(Path(LIBJ).read_bytes()).hexdigest() == summary['library']['sha256']
LIB = json.load(open(LIBJ))['macros']
MAC = [[Twist.from_record(ctx, r) for r in m['records']] for m in LIB]
W9 = MAC[0]


def labels(st):
    ls = st.lattice_stickers()
    assert bool(np.all(ls['frame_agrees']))
    return np.asarray(ls['labels'], np.int64)


_NC = None


def nu_vectors(st):
    """nu of a lattice configuration with the coordinate code of nucoords.py (the restorer uses its
    own), as one F2 vector (35 signs + 8 C2-type frames) and one F5 vector (orbits 17, 28)."""
    global _NC
    if _NC is None:
        sys.path.insert(0, str(Path(__file__).resolve().parent.parent / 'scripts'))
        from nucoords import NuCoords, vec
        _NC = (NuCoords(str(root)), vec)
    n = _NC[0].nu(labels(st))
    return {str(o): list(v) for o, v in n.items() if any(v)}, _NC[1](n)


def rank(rows, p):
    rows = [list(r) for r in rows]
    rk, col = 0, 0
    ncol = len(rows[0]) if rows else 0
    while rk < len(rows) and col < ncol:
        piv = next((i for i in range(rk, len(rows)) if rows[i][col] % p), None)
        if piv is None:
            col += 1
            continue
        rows[rk], rows[piv] = rows[piv], rows[rk]
        inv = pow(rows[rk][col], -1, p)
        rows[rk] = [x * inv % p for x in rows[rk]]
        for i in range(len(rows)):
            if i != rk and rows[i][col] % p:
                f = rows[i][col]
                rows[i] = [(a - f * b) % p for a, b in zip(rows[i], rows[rk])]
        rk += 1
        col += 1
    return rk


_LIBV = None


def library_vectors():
    global _LIBV
    if _LIBV is None:
        _LIBV = []
        for ws in MAC:
            st = State(ctx, menu=MENU)
            for t in ws:
                assert MENU.contains(t) and st.apply(t).applied
            assert st.is_lattice()
            _LIBV.append(nu_vectors(st)[1])
    return _LIBV


WORDS = [json.loads(Path(p).read_text()) for p in CHAIN[1:]] if CHAIN else None
CHAIN_END = WORDS[-1]['output_digest'] if CHAIN else None
SOLVED_DIGEST = State(ctx, menu=MENU).digest()
report = {'runs': [], 'chain_end_digest': CHAIN_END}
allok = True
for run in summary['runs']:
    spec = run['input']
    if CHAIN and not (run['status'] == 'restored' and run.get('input_digest') == CHAIN_END):
        continue
    late = run.get('order') == 'retained-first'
    r = {'input': spec, 'status_claimed': run['status']}
    if run['status'].startswith('failure certificate') and spec.startswith('table:'):
        doc = json.loads(Path(spec[6:]).read_text())
        st = State(ctx, menu=MENU)
        st._poses, st._lookup = [Pose(0, None, ('K', 0))], {('K', 0): 0}
        ids = np.array([st._intern_matrix(matrix_from_json(m)) for m in doc['poses']], np.int32)
        st.pose_id = ids[np.asarray(doc['pose_id'], np.int64)]
        st._compact()
        st.journal = []
        assert st.digest() == doc['digest'] == run['input_digest'] and st.is_lattice()
        nu_in, (b2, b5) = nu_vectors(st)
        lv = library_vectors()
        r2 = rank([v[0] for v in lv], 2)
        r5 = rank([v[1] for v in lv], 5)
        outside = rank([v[0] for v in lv] + [b2], 2) > r2 or rank([v[1] for v in lv] + [b5], 5) > r5
        r['nu_matches_restorer'] = nu_in == run['nu_input']
        r['library_rank'] = [r2, r5]
        r['outside_span_confirmed'] = bool(outside)
        allok &= r['nu_matches_restorer'] and outside
        report['runs'].append(r)
        print(json.dumps(r), flush=True)
        continue
    if run['status'] != 'restored':
        report['runs'].append(r)
        allok = False
        continue
    tag = spec.replace(':', '_').replace('/', '_')[-60:]
    stored = np.load(D / ('start-labels-%s.npy' % tag))
    st = None
    if spec.startswith('table:') and CHAIN:
        doc = json.loads(Path(CHAIN[0]).read_text())
        st = State(ctx, menu=MENU)
        st._poses, st._lookup = [Pose(0, None, ('K', 0))], {('K', 0): 0}
        ids = np.array([st._intern_matrix(matrix_from_json(m)) for m in doc['poses']], np.int32)
        st.pose_id = ids[np.asarray(doc['pose_id'], np.int64)]
        st._compact()
        st.journal = []
        assert st.digest() == doc['digest'] == WORDS[0]['input_digest']
        r['x_table_digest'] = doc['digest']
        r['x_off_lattice_pieces'] = int(st.off_lattice_count())
        segs = []
        for wf in WORDS:
            assert st.digest() == wf['input_digest']
            t1 = time.time()
            for k, (c, j) in enumerate(wf['word']):
                tw = Twist(ctx, int(c), RS.element(MENU, int(c), int(j)), 'menu', {'menu': MENU.name})
                assert MENU.contains(tw)
                out = st.apply(tw)
                assert out.applied, 'chain twist %d inadmissible: %s' % (k, out.status)
            assert st.digest() == wf['output_digest']
            segs.append({'twists': len(wf['word']), 'input_digest': wf['input_digest'],
                         'output_digest': wf['output_digest'], 'off_end': int(st.off_lattice_count()),
                         'seconds': round(time.time() - t1, 1)})
            print(json.dumps(segs[-1]), flush=True)
        r['chain_segments'] = segs
        r['chain_twists'] = sum(x['twists'] for x in segs)
        assert st.digest() == run['input_digest']
        r['chain_end_equals_restorer_input'] = True
    elif spec.startswith('table:'):
        doc = json.loads(Path(spec[6:]).read_text())
        st = State(ctx, menu=MENU)
        st._poses, st._lookup = [Pose(0, None, ('K', 0))], {('K', 0): 0}
        ids = np.array([st._intern_matrix(matrix_from_json(m)) for m in doc['poses']], np.int32)
        st.pose_id = ids[np.asarray(doc['pose_id'], np.int64)]
        st._compact()
        st.journal = []
        assert st.digest() == doc['digest'] == run['input_digest']
    elif spec.startswith('uniform:'):
        Lin = stored.copy()
        ok_in = np.array_equal(np.sort(Lin), np.arange(NS)) and np.array_equal(so[Lin], so)
        r['synthetic_input_valid'] = bool(ok_in)
        allok &= ok_in
    elif spec.startswith('mix:'):
        _, seed, n, k = spec.split(':')
        rng = random.Random(int(seed))
        st = State(ctx, menu=MENU)
        items = [('p', rng.choice([1, -1]) * rng.randrange(1, 1201)) for _ in range(int(n))]
        for _ in range(int(k)):
            items.insert(rng.randrange(len(items) + 1), ('m', rng.randrange(len(MAC)), rng.random() < 0.5))
        planted = []
        for it in items:
            if it[0] == 'p':
                assert st.apply(primitive(ctx, it[1])).applied
                continue
            ws = MAC[it[1]]
            for t in ([t.inverse() for t in reversed(ws)] if it[2] else ws):
                assert MENU.contains(t) and st.apply(t).applied
            assert st.is_lattice()
            planted.append({'macro': it[1], 'inverse': it[2]})
        assert planted == run['planted'] and st.digest() == run['input_digest']
        r['planted'] = planted
    else:
        kind, seed, n = spec.split(':')
        rng = random.Random(int(seed))
        st = State(ctx, menu=MENU)
        w = [rng.choice([1, -1]) * rng.randrange(1, 1201) for _ in range(int(n))]
        for m in w:
            assert st.apply(primitive(ctx, m)).applied
        if kind == 'w9word':
            for t in W9:
                assert MENU.contains(t) and st.apply(t).applied
        assert st.digest() == run['input_digest']
    if st is not None:
        assert st.is_lattice()
        for c in ([] if late else run['corrections']):
            ws = MAC[c['macro']]
            seq = [t.inverse() for t in reversed(ws)] if c['inverse'] else ws
            for _ in range(c['times']):
                for k, t in enumerate(seq):
                    assert MENU.contains(t)
                    out = st.apply(t)
                    assert out.applied, 'macro %d twist %d inadmissible' % (c['macro'], k)
                assert st.is_lattice()
        if run['corrections'] and not late:
            assert st.digest() == run['post_correction_digest']
            r['corrections_replayed'] = run['corrections']
        Lin = labels(st)
    same = bool(np.array_equal(Lin, stored))
    r['start_matches_restorer'] = same
    fs = sol['solutions'][tag]
    Lw = Lin.copy()
    for j, sg in fs:
        s, d = net[j] if sg > 0 else srt(net[j][1], net[j][0])
        Lw[d] = Lw[s]
    if late:
        t1 = time.time()
        stT = State(ctx, menu=MENU)
        for c in reversed(run['corrections']):
            ws = MAC[c['macro']]
            seq = ws if c['inverse'] else [t.inverse() for t in reversed(ws)]
            for _ in range(c['times']):
                for k, t in enumerate(seq):
                    assert MENU.contains(t)
                    assert stT.apply(t).applied, 'C^-1: macro %d twist %d inadmissible' % (c['macro'], k)
                assert stT.is_lattice()
        r['T_digest_matches'] = stT.digest() == run['T_digest']
        r['retained_part_reaches_T'] = bool(np.array_equal(Lw, labels(stT)))
        n_tw = 0
        for c in run['corrections']:
            ws = MAC[c['macro']]
            seq = [t.inverse() for t in reversed(ws)] if c['inverse'] else ws
            for _ in range(c['times']):
                for k, t in enumerate(seq):
                    assert MENU.contains(t)
                    assert stT.apply(t).applied, 'C: macro %d twist %d inadmissible' % (c['macro'], k)
                n_tw += len(seq)
                assert stT.is_lattice()
        r['correction_twists'] = n_tw
        r['corrections_replayed'] = run['corrections']
        r['final_digest_is_solved'] = stT.digest() == SOLVED_DIGEST
        r['final_labels_identity'] = bool(np.array_equal(labels(stT), np.arange(NS)))
        r['late_seconds'] = round(time.time() - t1, 1)
        ident = r['T_digest_matches'] and r['retained_part_reaches_T'] and r['final_digest_is_solved'] and r['final_labels_identity']
    else:
        ident = bool(np.array_equal(Lw, np.arange(NS)))
    r['w9'] = run['w9']
    r['restored_to_identity'] = ident
    r['factors'] = len(fs)
    r['retained_length'] = int(sum(length[j] for j, _ in fs))
    r['length_matches_restorer'] = r['retained_length'] == run['retained_length']
    allok &= same and ident and r['length_matches_restorer']
    report['runs'].append(r)
    print(json.dumps(r), flush=True)
report['pass'] = bool(allok)
report['seconds'] = round(time.time() - T0, 1)
Path(OUTP).write_text(json.dumps(report, indent=1))
print('PASS' if allok else 'FAIL', report['seconds'])
