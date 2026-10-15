"""Rigid-unit certificate for one orbit (menu I_a), from a pose table and its exact cap sets (capsets.py).
Members: every off piece (all orbits) and every hole of the wanted orbit. A grip is free when no off piece straddles
it. Group the members by their free-cap signature (the free grips whose closed cap holds the member: 'in' sets for
off pieces, cap signatures for holes). For every group holding a wanted-orbit member, check
  (L) self-locking: every grip whose cap holds some but not all members of the group (or that some member
      straddles) is straddled by an off piece of the group itself;
  (S) the K+ stabilizer of the group (k mapping the group's holes onto themselves and its off regions onto
      themselves) and the sign of each stabilizer element on the wanted-orbit holes and wanted-orbit off pieces;
  (N) the number n of wanted-orbit members and the K+-type of the group (groups of one type can be exchanged).
If every wanted-orbit member lies in a self-locked group, every retained word admissible from the table moves each
such group rigidly (Lemma U); with (S) even and n even (or no two groups of one type with n odd), every retained
loop R from the table has an even wanted-orbit sign (Proposition U). usage: units5.py <research/jumbling>
<classes.json> <arrays.npz> <pose table> <capsets json> <orbit> <out json>"""
import json, sys
from collections import Counter, defaultdict
from pathlib import Path
import numpy as np
sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parents[2] / 'e3s'))
sys.path.insert(0, str(Path(__file__).resolve().parent))  # package layout: e3lib.py next to this script
from e3lib import Lib  # noqa: E402
L = Lib(sys.argv[1], sys.argv[2], sys.argv[3])
from sim.kplus import matrix_from_json  # noqa: E402
from sim.state import Pose  # noqa: E402
doc = json.loads(Path(sys.argv[4]).read_text())
st = L.sim.State(L.ctx, menu=L.M)
st._poses, st._lookup = [Pose(0, None, ('K', 0))], {('K', 0): 0}
ids = np.array([st._intern_matrix(matrix_from_json(m)) for m in doc['poses']], np.int32)
st.pose_id = ids[np.asarray(doc['pose_id'], np.int64)]
st._compact(); st.journal, st._anchor_cache = [], {}
assert st.digest() == doc['digest']
cap = json.load(open(sys.argv[5]))
assert cap['table_digest'] == doc['digest']
O = int(sys.argv[6])
oid = np.asarray(L.d.orbit_id)
H = L.vacancies(st)
rows = {r['piece']: r for r in cap['pieces']}
blocked = {c for r in rows.values() for c in r['straddle']}
free = [g for g in range(600) if g not in blocked]
fset = set(free)
mem = {}
for p, r in rows.items():
    mem[('p', p)] = {'in': set(r['in']), 'str': set(r['straddle'])}
for h in H.tolist():
    if oid[h] == O:
        mem[('h', h)] = {'in': set(L.d.signature_list(h)), 'str': set()}
groups = defaultdict(list)
for m, v in mem.items():
    groups[tuple(sorted(v['in'] & fset))].append(m)
# off-piece regions as K+-comparable keys: centroid of the posed region
NFc = None
repV = [np.array([[float(v) for v in vert] for vert in vs]) for vs in L.reg.rep_vertices]


def centroid(p):
    V = (repV[L.orb[p]] @ L.Kf[L.tr[p]].T) @ L.fm(st.pose(p)).T
    return V.mean(0)


def sgn(perm):
    seen, s = set(), 1
    for a in perm:
        if a in seen:
            continue
        n, b = 0, a
        while b not in seen:
            seen.add(b); b = perm[b]; n += 1
        if n % 2 == 0:
            s = -s
    return s


out_groups = []
for key, ms in groups.items():
    wanted = [m for m in ms if (m[0] == 'h') or oid[m[1]] == O]
    if not wanted:
        continue
    offp = [m[1] for m in ms if m[0] == 'p']
    holes = [m[1] for m in ms if m[0] == 'h']
    allin = set.intersection(*[mem[m]['in'] for m in ms])
    anyin = set.union(*[mem[m]['in'] | mem[m]['str'] for m in ms])
    partial = sorted(anyin - allin)
    selfstr = set.union(*[mem[('p', p)]['str'] for p in offp]) if offp else set()
    unlocked = [g for g in partial if g not in selfstr]
    # stabilizer: k in K+ with k(holes) = holes and k(off regions) = off regions (centroids, rounded)
    C = np.array([centroid(p) for p in offp]) if offp else np.zeros((0, 4))
    hs = np.array(holes, np.int64)
    stab = []
    for k in range(7200):
        if len(hs):
            img = L.act(k, hs)
            if set(img.tolist()) != set(holes):
                continue
            hperm = dict(zip(holes, img.tolist()))
        else:
            hperm = {}
        if len(offp):
            CI = C @ L.Kf[k].T
            D2 = ((CI[:, None, :] - C[None, :, :]) ** 2).sum(-1)
            j = D2.argmin(1)
            if (D2[np.arange(len(C)), j] > 1e-12).any() or len(set(j.tolist())) != len(C):
                continue
            operm = {i: int(j[i]) for i in range(len(C))}
        else:
            operm = {}
        wh = {a: b for a, b in hperm.items() if oid[a] == O}
        wo = {i: j for i, j in operm.items() if oid[offp[i]] == O}
        stab.append({'k': k, 'sign_holes': sgn(wh), 'sign_off': sgn(wo)})
    n_w = len(wanted)
    out_groups.append({'free_caps': list(key), 'members': len(ms), 'off_pieces': len(offp), 'wanted_holes': len(holes),
                       'wanted_off': sum(1 for p in offp if oid[p] == O), 'n_wanted': n_w,
                       'partial_caps': partial, 'unlocked_partial_caps': unlocked, 'self_locked': not unlocked,
                       'stabilizer_order': len(stab), 'stabilizer_odd_elements': [s for s in stab if s['sign_holes'] * s['sign_off'] == -1],
                       'hole_ids': holes, 'off_ids': offp})
res = {'table_digest': doc['digest'], 'orbit': O, 'free_grips': len(free), 'groups_with_wanted': len(out_groups),
       'all_self_locked': all(g['self_locked'] for g in out_groups),
       'all_stabilizers_even': all(not g['stabilizer_odd_elements'] for g in out_groups),
       'n_wanted_parities': dict(Counter(g['n_wanted'] % 2 for g in out_groups)), 'groups': out_groups}
json.dump(res, open(sys.argv[7], 'w'), indent=1)
print(json.dumps({k: v for k, v in res.items() if k != 'groups'}))
for g in out_groups:
    print({k: v for k, v in g.items() if k not in ('hole_ids', 'off_ids', 'partial_caps')})
