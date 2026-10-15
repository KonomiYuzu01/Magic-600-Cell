"""Shape-collision search for lattice-returning I_a words (card 2, Lemmas 2 and 3).

Random walks of twists at a small grip set D start from the solved state. After each twist the
decorated shape (the posed region and the K+ coset of every piece, labels forgotten) is hashed.
When a walk w reaches a shape that an earlier prefix p reached, the word w p^-1 is admissible and
returns to the lattice (Lemma 2: the decorated shape determines admissibility). These words are
the edge returns of Lemma 3 for the explored part of the shape graph.

The hash only proposes candidates: every candidate is replayed twist by twist on the exact
simulator, and only words that apply and end on the lattice with a moved sticker are kept.
nu is computed from the end state.

usage: search_collide.py <scriptdir> <repo root> <research/jumbling> <out dir> <budget s> <seed>
                         <max walk> <k grips per walk> <d>...

Each walk uses d and k grips drawn from those 35 to 50 degrees from d (caps that meet d's cap
without being among the 16 grips a jumble twist at d always blocks).
"""
import hashlib
import json
import random
import sys
import time
from collections import Counter
from pathlib import Path

import numpy as np

sys.path.insert(0, sys.argv[1])
from nucoords import NuCoords, vec, Span      # noqa: E402

ROOT, J, OUT = sys.argv[2], Path(sys.argv[3]).resolve(), Path(sys.argv[4])
BUDGET, SEED, LMAX, KN = float(sys.argv[5]), int(sys.argv[6]), int(sys.argv[7]), int(sys.argv[8])
DS = [int(x) for x in sys.argv[9:]]
for p in (J / 'theory', J, J / 'fixtures'):
    sys.path.insert(0, str(p))
sys.dont_write_bytecode = True
import restore as RS                            # noqa: E402
from sim.state import State                     # noqa: E402
from sim.twists import Twist                    # noqa: E402
from sim.kernel import q5_float                 # noqa: E402

T0 = time.time()
ctx = RS.ctx
kp, reg, data = ctx.kplus, ctx.regions, ctx.data
M = RS.menu('I_a')
STAB = {kp.matrix(s) for s in kp.stab0}
NC = NuCoords(ROOT)
OUT.mkdir(parents=True, exist_ok=True)
log = open(OUT / 'search.jsonl', 'a')

NF = np.array([[float(v) for v in row] for row in data.N])
NF /= np.linalg.norm(NF[0])
ANG = np.degrees(np.arccos(np.clip(NF @ NF.T, -1, 1)))
# K+-equivariant point of every piece: the sum of the poles of its signature
mo, mv = np.asarray(data.mask_offsets, np.int64), np.asarray(data.mask_values, np.int64)
cs = np.vstack([np.zeros((1, 4)), np.cumsum(NF[mv], axis=0)])
PA = cs[mo[1:]] - cs[mo[:-1]]
ORB = reg.orbit_of
TR = reg.transport

_coset, _pf = {}, {}


def pose_float(st, pid):
    key = st._poses[pid].key
    f = _pf.get(key)
    if f is None:
        f = np.array([[q5_float(x) for x in row] for row in st._pose_matrix(pid)], np.float64)
        _pf[key] = f
    return f


def coset_id(st, pid):
    """Canonical id of the coset P K+ = the image of the pole set under P."""
    key = st._poses[pid].key
    c = _coset.get(key)
    if c is None:
        pts = np.round(NF @ pose_float(st, pid).T, 6) + 0.0
        pts = pts[np.lexsort(pts.T[::-1])]
        c = hashlib.sha1(pts.tobytes()).hexdigest()[:16]
        _coset[key] = c
    return c


def shape_key(st):
    moved = np.nonzero(st.pose_id != 0)[0]
    if not moved.size:
        return 'solved'
    pids = st.pose_id[moved]
    kpose = st._kpose()[pids]
    lat = kpose >= 0
    have = Counter()
    if lat.any():
        comp = kp.compose_vec(kpose[lat], TR[moved[lat]])
        pos = reg.piece_of[ORB[moved[lat]], comp]
        have.update(('L', int(x)) for x in pos)
    for pid in np.unique(pids[~lat]):
        mem = moved[pids == pid]
        cid = coset_id(st, int(pid))
        pts = np.round(PA[mem] @ pose_float(st, int(pid)).T, 5) + 0.0
        have.update((cid, int(o), tuple(p)) for o, p in zip(ORB[mem].tolist(), pts.tolist()))
    home = Counter(('L', int(x)) for x in moved)
    plus, minus = have - home, home - have
    if not plus and not minus:
        return 'solved'
    h = hashlib.sha1(repr((sorted(plus.items()), sorted(minus.items()))).encode()).hexdigest()
    return h


def twist(c, i, inv=False):
    t = Twist(ctx, c, RS.element(M, c, i))
    return t.inverse() if inv else t


lib, seen_nu, span = [], {}, Span()
stats = Counter()


def consider(fam, recs, st):
    labels = np.asarray(st.lattice_stickers()['labels'], np.int64)
    nu = NC.nu(labels)
    key = json.dumps(sorted((int(o), list(v)) for o, v in nu.items()))
    stats['returns'] += 1
    if key in seen_nu:
        return
    b2, b5 = vec(nu)
    i = len(lib)
    new = span.add(i, b2, b5)
    lib.append({'id': i, 'family': fam, 'records': recs, 'length': len(recs),
                'nu': {str(o): list(v) for o, v in nu.items()}, 'moved': int(st.moved_count()), 'new_rank': new})
    seen_nu[key] = i
    log.write(json.dumps({'id': i, 'family': fam, 'len': len(recs), 'nu': lib[-1]['nu'], 'new': new,
                          'rank2': len(span.r2), 'rank5': len(span.r5), 't': round(time.time() - T0)}) + '\n')
    log.flush()


rng = random.Random(SEED)
NIT = len(M.items)
RET = [i for i in range(NIT) if M.items[i][1] in STAB]
JUM = [i for i in range(NIT) if M.items[i][1] not in STAB]
seen = {}
d_sets = {}
for d in DS:
    d_sets[d] = [int(c) for c in np.argsort(ANG[d]) if 35 < ANG[d][c] < 50]
print(json.dumps({'grip_sets': d_sets, 'retained_items': len(RET), 'jumble_items': len(JUM)}), flush=True)

walks = 0
while time.time() - T0 < BUDGET:
    d = rng.choice(DS)
    D = [d] + rng.sample(d_sets[d], KN)
    st = State(ctx, menu=M)
    word, recs = [], []
    L = rng.randint(2, LMAX)
    walks += 1
    for step in range(L):
        for attempt in range(12):
            c = rng.choice(D)
            i = rng.choice(RET[1:] if rng.random() < 0.3 else JUM)
            t = twist(c, i)
            if st.apply(t).applied:
                break
        else:
            break
        word.append((c, i))
        recs.append(t.record())
        key = shape_key(st)
        stats['steps'] += 1
        if key == 'solved':
            if st.is_lattice() and st.moved_count() and any(i in JUM for _, i in word):
                consider('walk', list(recs), st)
            continue
        prev = seen.get(key)
        if prev is None:
            seen[key] = (tuple(word), list(recs))
            continue
        pw, prec = prev
        if pw == tuple(word) or not any(i in JUM for _, i in word + list(pw)):
            continue
        stats['candidates'] += 1
        applied = 0
        ok = True
        back = []
        for (c2, i2) in reversed(pw):
            t2 = twist(c2, i2, inv=True)
            if not st.apply(t2).applied:
                ok = False
                break
            applied += 1
            back.append(t2.record())
        if ok and st.is_lattice() and st.moved_count():
            consider('collide', list(recs) + back, st)
        elif not ok or not st.is_lattice():
            stats['hash_false'] += 1
        for _ in range(applied):
            st.undo()
    if walks % 50 == 0:
        print(json.dumps({'walks': walks, 'shapes': len(seen), **stats, 'lib': len(lib), 'rank2': len(span.r2),
                          'rank5': len(span.r5), 't': round(time.time() - T0)}), flush=True)
        (OUT / 'library.json').write_text(json.dumps({'library': lib, 'rank2': len(span.r2),
                                                      'rank5': len(span.r5), 'stats': dict(stats)}))
(OUT / 'library.json').write_text(json.dumps({'library': lib, 'rank2': len(span.r2), 'rank5': len(span.r5),
                                              'stats': dict(stats), 'seconds': round(time.time() - T0)}))
print('done', dict(stats), len(lib), len(span.r2), len(span.r5))
