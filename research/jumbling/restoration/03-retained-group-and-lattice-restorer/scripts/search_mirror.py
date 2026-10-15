"""Mirrored-setup half-turn returns (card 2, Lemmas 15-16) as correction-macro candidates.

w = v v^a (d,a) (v v^a)^-1 with a a retained half-turn at grip d, v a local jumble setup and v^a
its image under the global rotation a. Every candidate is replayed in the exact simulator; only
words that are admissible and end on the lattice are kept, and nu is computed from the end state.

usage: search_mirror.py <scriptdir> <repo root> <research/jumbling> <out dir> <budget s> <seed> <d>...
(write -d to skip the single-twist stage; the three half-turns of A4_d are conjugate under its
third-turns, which fix d, so one half-turn per grip already covers the single-twist family)"""
import json
import random
import sys
import time
from pathlib import Path

import numpy as np

sys.path.insert(0, sys.argv[1])
from nucoords import NuCoords, vec, Span      # noqa: E402

ROOT, J, OUT, BUDGET, SEED = sys.argv[2], Path(sys.argv[3]).resolve(), Path(sys.argv[4]), float(sys.argv[5]), int(sys.argv[6])
DS = [int(x) for x in sys.argv[7:]]
for p in (J / 'theory', J, J / 'fixtures'):
    sys.path.insert(0, str(p))
sys.dont_write_bytecode = True
import restore as RS                            # noqa: E402
from sim.state import State                     # noqa: E402
from sim.twists import Twist, a4_element        # noqa: E402
from exact import matmul, transpose             # noqa: E402
from sim.kplus import to_tuple                  # noqa: E402

T0 = time.time()
ctx = RS.ctx
kp = ctx.kplus
M = RS.menu('I_a')
JI = RS.jumble_items(M)
NC = NuCoords(ROOT)
OUT.mkdir(parents=True, exist_ok=True)
log = open(OUT / 'search.jsonl', 'a')
lib, seen, span = [], {}, Span()
NF = np.array([[float(v) for v in row] for row in ctx.data.N])
ANG = np.degrees(np.arccos(np.clip(NF @ NF.T / float(ctx.data.NN), -1, 1)))


def mirror(t, A, aperm):
    return Twist(ctx, int(aperm[t.grip]), to_tuple(matmul(matmul(A, t.matrix), transpose(A))))


def run_word(ws):
    st = State(ctx, menu=M)
    for t in ws:
        if not st.apply(t).applied:
            return None
    if not st.is_lattice() or not st.moved_count():
        return None
    return st


def consider(fam, ws, st):
    nu = NC.nu(np.asarray(st.lattice_stickers()['labels'], np.int64))
    key = json.dumps(sorted((int(o), list(v)) for o, v in nu.items()))
    if key in seen:
        return
    b2, b5 = vec(nu)
    i = len(lib)
    new = span.add(i, b2, b5)
    lib.append({'id': i, 'family': fam, 'records': [t.record() for t in ws], 'length': len(ws),
                'nu': {str(o): list(v) for o, v in nu.items()}, 'moved': int(st.moved_count()), 'new_rank': new})
    seen[key] = i
    log.write(json.dumps({'id': i, 'family': fam, 'len': len(ws), 'nu': lib[-1]['nu'], 'new': new,
                          'rank2': len(span.r2), 'rank5': len(span.r5), 't': round(time.time() - T0)}) + '\n')
    log.flush()


rng = random.Random(SEED)
tried = kept = 0
SINGLES = not any(d < 0 for d in DS)       # a negative grip number means: skip the single-twist stage
DS = [abs(d) for d in DS]
for d in (DS if SINGLES else []):
    halves = []
    for i in range(1, 12):
        a = a4_element(ctx, d, i)
        if kp.compose(a.kidx, a.kidx) == 0:
            halves.append(a)
    near = [int(c) for c in np.argsort(ANG[d]) if 0 < ANG[d][c] < 75 and int(c) != d]
    for a in halves:
        A = kp.matrix(a.kidx)
        aperm = kp.perms[a.kidx].astype(np.int64)
        # single-twist setups
        for c in near:
            if aperm[c] == c or ANG[c][aperm[c]] < 53:
                continue
            for qi in JI:
                if time.time() - T0 > BUDGET:
                    break
                v = [Twist(ctx, c, RS.element(M, c, qi))]
                va = [mirror(t, A, aperm) for t in v]
                if not all(M.contains(t) for t in va):
                    continue
                u = v + va
                ws = u + [a] + [t.inverse() for t in reversed(u)]
                tried += 1
                st = run_word(ws)
                if st is not None:
                    kept += 1
                    consider('mirror1', ws, st)
        print(json.dumps({'d': d, 'a': a.kidx, 'stage': 'single', 'tried': tried, 'kept': kept, 'lib': len(lib),
                          'rank2': len(span.r2), 'rank5': len(span.r5), 't': round(time.time() - T0)}), flush=True)
# two-twist setups, random
while time.time() - T0 < BUDGET:
    d = rng.choice(DS)
    a = rng.choice([x for x in (a4_element(ctx, d, i) for i in range(1, 12)) if kp.compose(x.kidx, x.kidx) == 0])
    A = kp.matrix(a.kidx)
    aperm = kp.perms[a.kidx].astype(np.int64)
    near = [int(c) for c in np.argsort(ANG[d]) if 0 < ANG[d][c] < 75 and int(c) != d]
    c = rng.choice(near)
    b = rng.choice([int(x) for x in np.argsort(ANG[c]) if 0 < ANG[c][x] < 60 and ANG[d][x] < 80])
    if any(aperm[x] == x or min(ANG[x][aperm[y]] for y in (c, b)) < 53 for x in (c, b)):
        continue
    v = [Twist(ctx, c, RS.element(M, c, rng.choice(JI))), Twist(ctx, b, RS.element(M, b, rng.choice(JI)))]
    va = [mirror(t, A, aperm) for t in v]
    if not all(M.contains(t) for t in va):
        continue
    u = v + va
    ws = u + [a] + [t.inverse() for t in reversed(u)]
    tried += 1
    st = run_word(ws)
    if st is not None:
        kept += 1
        consider('mirror2', ws, st)
    if tried % 200 == 0:
        print(json.dumps({'stage': 'pair', 'tried': tried, 'kept': kept, 'lib': len(lib), 'rank2': len(span.r2),
                          'rank5': len(span.r5), 't': round(time.time() - T0)}), flush=True)
(OUT / 'library.json').write_text(json.dumps({'library': lib, 'rank2': len(span.r2), 'rank5': len(span.r5),
                                              'tried': tried, 'kept': kept, 'seconds': round(time.time() - T0)}))
print('done', tried, kept, len(lib), len(span.r2), len(span.r5))
