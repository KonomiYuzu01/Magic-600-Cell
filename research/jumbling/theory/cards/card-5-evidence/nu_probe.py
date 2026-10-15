"""Sign part of nu on lattice-returning conjugates u (d,a) u^-1 (I_a; exact; lead, not proof).
u = the first four records of the card-2 I_a counterexample journal.
Usage: python -I nu_probe.py <research/jumbling dir> <journal.json> <out.json>"""
import json, sys, time
from pathlib import Path
J = Path(sys.argv[1]).resolve()
for p in (J / 'theory', J, J / 'fixtures'):
    sys.path.insert(0, str(p))
sys.dont_write_bytecode = True
import numpy as np
import restore
from sim.state import State
from sim.twists import Twist, a4_element
from exact import matmul
ctx = restore.ctx
data, kp = ctx.data, ctx.kplus
M = restore.menu('I_a')
recs = json.load(open(sys.argv[2]))['records']
u = [Twist.from_record(ctx, r) for r in recs[:4]]
assert all(M.contains(t) for t in u)
uinv = [t.inverse() for t in reversed(u)]
perms = np.asarray(kp.perms, np.int64)
sig_index = {data.sigbits[p].tobytes(): p for p in range(len(data.sigbits))}
orb = data.orbit_id
orbits = sorted(set(orb.tolist()) - {-1})
ID = kp.matrix(0)

def odd_orbits(st):
    kpose = np.array([st._poses[i].kidx for i in st.pose_id], np.int64)
    dest = np.arange(len(kpose))
    for p in np.flatnonzero(kpose != 0):
        imgs = perms[kpose[p]][data.signature_list(p)]
        b = np.zeros(75, np.uint8)
        np.bitwise_or.at(b, imgs >> 3, (128 >> (imgs & 7)).astype(np.uint8))
        dest[p] = sig_index[b.tobytes()]
    odd = []
    for o in orbits:
        idx = np.flatnonzero(orb == o)
        seen = set(); ev = 0
        for p in idx:
            if p in seen or dest[p] == p:
                continue
            L, q = 0, p
            while q not in seen:
                seen.add(q); q = int(dest[q]); L += 1
            ev += (L % 2 == 0)
        if ev % 2:
            odd.append(int(o))
    return odd

st = State(ctx)
for t in u:
    assert st.apply(t).applied
cands = sorted(set().union(*[set(int(x) for x in restore.NEAR[c]) for c in (0, 2, 161, 139)]))
rows, t0 = [], time.time()
for d in cands:
    for i in range(1, 12):
        a = a4_element(ctx, d, i)
        half = kp.compose(a.kidx, a.kidx) == 0
        order3 = (not half) and kp.compose(a.kidx, kp.compose(a.kidx, a.kidx)) == 0
        if not half and d != 16:
            continue
        n = 0
        out = st.apply(a)
        if not out.applied:
            continue
        n = 1
        ok = True
        for t in uinv:
            if st.apply(t).applied:
                n += 1
            else:
                ok = False
                break
        row = {'d': d, 'i': i, 'half': bool(half), 'order3': bool(order3)}
        if not ok:
            row['status'] = 'u_inverse_blocked'
        elif not st.is_lattice():
            row['status'] = 'off_lattice'; row['off'] = int(st.off_lattice_count())
        else:
            row['status'] = 'lattice'; row['odd_orbits'] = odd_orbits(st)
        for _ in range(n):
            st.undo()
        rows.append(row)
        print(json.dumps(dict(row, sec=round(time.time() - t0))), flush=True)
lat = [r for r in rows if r['status'] == 'lattice']
vecs = sorted({tuple(r['odd_orbits']) for r in lat})
# F2 rank of the sign vectors
basis = []
for v in vecs:
    x = sum(1 << o for o in v)
    for b in basis:
        x = min(x, x ^ b)
    if x:
        basis.append(x)
summary = {'candidates_grips': len(cands), 'trials': len(rows), 'lattice_returns': len(lat),
           'distinct_odd_sets': [list(v) for v in vecs], 'f2_rank': len(basis),
           'third_turn_odd': [r for r in lat if r['order3'] and r['odd_orbits']]}
Path(sys.argv[3]).write_text(json.dumps({'summary': summary, 'rows': rows}, indent=1))
print(json.dumps(summary))
