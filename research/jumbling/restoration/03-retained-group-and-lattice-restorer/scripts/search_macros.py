"""Search short I_a words that start and end on the lattice; record their nu and the span.

usage: search_macros.py <scriptdir> <repo root> <research/jumbling> <w9 journal> <targets.json> <out dir> <budget s> <seed>
Families: lens (0,q)(d,a)(0,q^-1) over every jumble q of cap 0, every grip d near 0 and every
non-identity a in A4_d; then random u = 2..4 chained jumble twists near cap 0 with every (d, a) near
u's grips, closed by u^-1. Each candidate runs in the exact simulator (admissibility checked)."""
import json
import random
import sys
import time
from pathlib import Path

import numpy as np

sys.path.insert(0, sys.argv[1])
from nucoords import NuCoords, vec, Span      # noqa: E402

ROOT, J, W9J, TG, OUT, BUDGET, SEED = (sys.argv[2], Path(sys.argv[3]).resolve(), sys.argv[4], sys.argv[5],
                                      Path(sys.argv[6]), float(sys.argv[7]), int(sys.argv[8]))
NLENS = int(sys.argv[9])           # how many jumble elements q of cap 0 to scan in the lens family
LMIN, LMAX = int(sys.argv[10]), int(sys.argv[11])
for p in (J / 'theory', J, J / 'fixtures'):
    sys.path.insert(0, str(p))
sys.dont_write_bytecode = True
import restore as RS                            # noqa: E402
from sim.state import State                     # noqa: E402
from sim.twists import Twist, a4_element        # noqa: E402

T0 = time.time()
ctx = RS.ctx
M = RS.menu('I_a')
JI = RS.jumble_items(M)
NC = NuCoords(ROOT)
OUT.mkdir(parents=True, exist_ok=True)
log = open(OUT / 'search.jsonl', 'a')
lib, seen = [], {}
span = Span()


def labels(st):
    return np.asarray(st.lattice_stickers()['labels'], np.int64)


def consider(family, recs, st):
    nu = NC.nu(labels(st))
    key = json.dumps(sorted((int(o), list(v)) for o, v in nu.items()))
    if key in seen:
        return False
    b2, b5 = vec(nu)
    i = len(lib)
    new = span.add(i, b2, b5)
    lib.append({'id': i, 'family': family, 'records': recs, 'length': len(recs),
                'nu': {str(o): list(v) for o, v in nu.items()}, 'moved': int(st.moved_count()), 'new_rank': new})
    seen[key] = i
    log.write(json.dumps({'id': i, 'family': family, 'len': len(recs), 'nu': lib[-1]['nu'], 'new': new,
                          'rank2': len(span.r2), 'rank5': len(span.r5), 't': round(time.time() - T0)}) + '\n')
    log.flush()
    return new


# w9 first
w9recs = json.load(open(W9J))['records']
st = State(ctx, menu=M)
for r in w9recs:
    assert st.apply(Twist.from_record(ctx, r)).applied
assert st.is_lattice()
consider('w9', w9recs, st)

targets = json.loads(Path(TG).read_text())


def inner(st, u_twists, grips, family):
    found = 0
    tried = 0
    for d in grips:
        for i in range(1, 12):
            if time.time() - T0 > BUDGET:
                return found, tried
            a = a4_element(ctx, d, i)
            tried += 1
            if not st.apply(a).applied:
                continue
            n = 1
            ok = True
            for t in reversed(u_twists):
                if st.apply(t.inverse()).applied:
                    n += 1
                else:
                    ok = False
                    break
            if ok and st.is_lattice() and st.moved_count():
                recs = [t.record() for t in u_twists] + [a.record()] + [t.inverse().record() for t in reversed(u_twists)]
                found += 1
                consider(family, recs, st)
            for _ in range(n):
                st.undo()
    return found, tried


# lens family
st = State(ctx, menu=M)
near0 = [int(d) for d in RS.NEAR[0] if d != 0]
for qi in JI[:NLENS]:
    if time.time() - T0 > BUDGET:
        break
    u = Twist(ctx, 0, RS.element(M, 0, qi))
    assert st.apply(u).applied
    f, n = inner(st, [u], near0, 'lens')
    st.undo()
    print(json.dumps({'lens_q': qi, 'returns': f, 'tried': n, 'lib': len(lib), 'rank2': len(span.r2),
                      'rank5': len(span.r5), 't': round(time.time() - T0)}), flush=True)

# random chained u
rng = random.Random(SEED)
while time.time() - T0 < BUDGET:
    L = rng.randint(LMIN, LMAX)
    st = State(ctx, menu=M)
    us, grips = [], [0]
    c = 0
    tries = 0
    while len(us) < L and tries < 50:
        tries += 1
        c2 = c if not us else rng.choice([int(x) for x in RS.NEAR[c] if int(x) != c])
        t = Twist(ctx, c2, RS.element(M, c2, rng.choice(JI)))
        if st.apply(t).applied:
            us.append(t)
            grips.append(c2)
            c = c2
    if len(us) < 2:
        continue
    near = sorted(set(int(x) for g in grips[-2:] for x in RS.NEAR[g]))
    rng.shuffle(near)
    f, n = inner(st, us, near[:80], 'chain%d' % len(us))
    print(json.dumps({'chain': [t.grip for t in us], 'returns': f, 'tried': n, 'lib': len(lib),
                      'rank2': len(span.r2), 'rank5': len(span.r5), 't': round(time.time() - T0)}), flush=True)

res = {'library': lib, 'rank2': len(span.r2), 'rank5': len(span.r5), 'seconds': round(time.time() - T0),
       'targets': {}}
for name, nu in targets.items():
    b2, b5 = vec({int(o): tuple(v) for o, v in nu.items()})
    res['targets'][name] = span.solve(b2, b5)
(OUT / 'library.json').write_text(json.dumps(res))
print(json.dumps({'rank2': res['rank2'], 'rank5': res['rank5'], 'targets': res['targets']}))
