"""Independent exact replay of card-2 counterexample journals (read-only).
Usage: python -I replay_c2.py <research/jumbling dir> <journal.json> <menu name>"""
import json, sys
from pathlib import Path
J = Path(sys.argv[1]).resolve()
for p in (J / 'theory', J, J / 'fixtures'):
    sys.path.insert(0, str(p))
sys.dont_write_bytecode = True
import numpy as np
import restore
from sim.state import State
from sim.twists import Twist
from exact import matmul

ctx = restore.ctx
data, kp = ctx.data, ctx.kplus
doc = json.load(open(sys.argv[2]))
name = sys.argv[3]
M = restore.menu(name)
recs = doc['records']
in_menu = [bool(M.contains(Twist.from_record(ctx, r))) for r in recs]
st = State.replay(doc, ctx=ctx)            # raises unless every record applies with its recorded moved count
print(json.dumps({'menu': name, 'steps': len(recs), 'all_in_menu': all(in_menu), 'in_menu': in_menu,
                  'retained_steps': [i for i, r in enumerate(recs) if r.get('retained')],
                  'off_lattice_end': int(st.off_lattice_count()), 'is_lattice': bool(st.is_lattice())}))
if not st.is_lattice():
    sys.exit(0)
# piece permutation on the lattice: piece p sits in the chamber whose signature is k(signature(p))
perms = np.asarray(kp.perms, np.int64)
# check perms direction on a few elements against exact matrices
for k in (1, 77, 4000):
    m = kp.matrix(k)
    for c in (0, 5, 333):
        img = tuple(sum((m[i][j] * data.N[c][j] for j in range(4)), type(data.N[c][0])(0)) for i in range(4))
        assert data.POLE[img] == perms[k][c], (k, c)
sig_index = {data.sigbits[p].tobytes(): p for p in range(len(data.sigbits))}
kpose = np.array([st._poses[i].kidx for i in st.pose_id], np.int64)
nonid = np.flatnonzero(kpose != 0)
dest = np.arange(len(kpose))
bits = np.zeros(75, np.uint8)
for p in nonid:
    imgs = perms[kpose[p]][data.signature_list(p)]
    b = np.zeros(75, np.uint8)
    np.bitwise_or.at(b, imgs >> 3, (128 >> (imgs & 7)).astype(np.uint8))
    dest[p] = sig_index[b.tobytes()]
assert len(set(dest.tolist())) == len(dest), 'not a permutation'
orb = data.orbit_id
res = {}
for o in sorted(set(orb.tolist())):
    idx = np.flatnonzero(orb == o)
    seen = set(); cycles_even = 0; moved = 0; twos = 0
    for p in idx:
        if p in seen: continue
        L = 0; q = p
        while q not in seen:
            seen.add(q); q = int(dest[q]); L += 1
        if L > 1:
            moved += L
            if L % 2 == 0: cycles_even += 1
            if L == 2: twos += 1
    if moved:
        res[int(o)] = {'size': int(len(idx)), 'moved': moved, 'even_length_cycles': cycles_even,
                       'two_cycles': twos, 'parity': 'odd' if cycles_even % 2 else 'even'}
kset = sorted(set(kpose.tolist()))
print(json.dumps({'distinct_K_poses_at_end': len(kset), 'pieces_not_identity': int(nonid.size),
                  'orbits_moved': res}))
