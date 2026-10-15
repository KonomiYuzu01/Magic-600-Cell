"""Search for a b_o potential (orbit o, menu I_a) of the form described in the notes:
pi: regions -> slots, K+-equivariant (pi(k R_c) = k pi(R_c)), pi = id on slots, and for every grip d the homomorphism
tau_d: Lambda_d -> Sym(slots of cap d), tau_d(h)(t) = a(t) with a in A4_d, a(e_d(t)) = h(e_d(t)), where e_d(t) in F(d)
is a K+-equivariant association of each slot t of cap d with a grip of the 36-degree family F(d) of the orbit.
Conditions over every state of the refined dynamics (refbfs2) of the orbit:
  (i)  pi(R) in cap d for every allowed grip d whose cap holds R,
  (ii) pi(h R) = tau_d(h) pi(R) for those d and every h in Lambda_d,
  (iii) pi(R) not in cap d for every allowed grip d whose cap R avoids.
usage: potsolve.py <research/jumbling> <kplus orbit> <out json>"""
import json, sys, time
from collections import Counter, defaultdict, deque
from itertools import product
from pathlib import Path
import numpy as np
JARG, O_IDX, OUT = sys.argv[1], int(sys.argv[2]), sys.argv[3]
sys.argv = [sys.argv[0], JARG]
sys.path.insert(0, str(Path(__file__).resolve().parent))
import refbfs2 as R
T, Q5, reg, d, kp = R.T, R.Q5, R.reg, R.d, R.ctx.kplus
from sim.kplus import to_tuple
t0 = time.time()
o = O_IDX
B, seen, hist, trans, complete = R.explore(o, 40, 10 ** 6)
assert complete
print('refined states', len(seen), 'classes', len(B.cls), round(time.time() - t0, 1), flush=True)
perms = kp.perms.astype(np.int64)
pidx = {tuple(perms[k].tolist()): k for k in range(len(perms))}
inv = np.array([pidx[tuple(np.argsort(perms[k]).tolist())] for k in range(len(perms))])
piece_of = reg.piece_of[o]
sig = {int(p): set(d.signature_list(int(p))) for p in piece_of.tolist()}
r0 = int(reg.reps[o]); sig0 = sorted(sig[r0])
# exact right factors of the menu rows, in refbfs2 order
MENU_R = []
for c in range(R.NP):
    seen_u = set(); rows = []
    for u in R.UN:
        k = tuple(np.round(R.qf(u), 9) + 0.0)
        if tuple(-x + 0.0 for x in k) in seen_u:
            continue
        seen_u.add(k); rows.append(u)
    MENU_R.append(rows)
E = [tuple(Q5(1 if k == b else 0) for k in range(4)) for b in range(4)]
def kidx_of(u, v):
    """K+ index of x -> u x v (u, v unit icosians)."""
    cols = [T.qmul(T.qmul(u, e), v) for e in E]
    M = tuple(tuple(cols[b][a] for b in range(4)) for a in range(4))
    i = kp.index_of_matrix(M)
    assert i >= 0
    return i
# 36-degree family of the orbit: e in F(c) iff angle 36 and some orbit slot lies in both caps
ang = R.ang
F = defaultdict(set)
for p, s in sig.items():
    for a in s:
        for b in s:
            if a != b and abs(ang[a, b] - 36.0) < 0.3:
                F[a].add(b)
print('family sizes', Counter(len(v) for v in F.values()), 'rep caps', sig0,
      'pairs at 36 in rep', [(a, b) for a in sig0 for b in sig0 if a < b and abs(ang[a, b] - 36) < 0.3], flush=True)
# out-caps of each class (closed complement), exact ties
def outcaps(l, Vf, V_ex):
    lf = R.qf(l)
    vals = (Vf @ R.Lmat(lf).T) @ R.NF.T
    maxs = vals.max(axis=0) - R.KAPf
    out = set(np.nonzero(maxs < -R.MARGIN)[0].tolist())
    tie = np.nonzero(np.abs(maxs) <= R.MARGIN)[0]
    if len(tie):
        ex = [T.qmul(l, v) for v in V_ex]
        for c in tie.tolist():
            n = R.N_ex[c]
            if all((sum((n[a] * y[a] for a in range(4)), Q5(0)) - R.KAP).sign() <= 0 for y in ex):
                out.add(c)
    return out
Vex = reg.rep_vertices[o]; Vf = np.array([[float(x) for x in v] for v in Vex])
allowed_in, allowed_out = defaultdict(set), defaultdict(set)
for key, Oset, cpl in seen:
    l, h, D = B.cls[key]
    for dg in D:
        if not any(dg in R.NB[e] for e in Oset if e != dg):
            allowed_in[key].add(dg)
    for dg in outcaps(l, Vf, Vex):
        if not any(dg in R.NB[e] for e in Oset):
            allowed_out[key].add(dg)
# transitions: (key, d) -> list of (row, key2, kidx)
TR = {}
for key in B.cls:
    l, h, D = B.cls[key]
    for dg in allowed_in[key]:
        rows = []
        for ri, (HL, HLf, HRf, ret) in enumerate(R.MENU[dg]):
            nl = T.qmul(HL, l)
            i, k2 = B.klass(nl)
            rows.append((ri, k2, kidx_of(R.UN[i], MENU_R[dg][ri])))
        TR[(key, dg)] = rows
print('transitions built', sum(len(v) for v in TR.values()), round(time.time() - t0, 1), flush=True)
# action of menu rows on family grips: hmap[(dg, ri)][e] = image grip
def grip_of(vec):
    v = vec / np.linalg.norm(vec)
    return R.pole_key.get(tuple(np.round(v, 7) + 0.0))
hmap = {}
for (key, dg) in TR:
    if (dg, 0) in hmap:
        continue
    for ri, (HL, HLf, HRf, ret) in enumerate(R.MENU[dg]):
        vinv = HRf * np.array([1, -1, -1, -1])
        hmap[(dg, ri)] = {e: grip_of(R.Lmat(HLf) @ (R.Rmat(vinv) @ R.NFu[e])) for e in F[dg]}
A4 = {}
def a4_of(dg):
    if dg not in A4:
        A4[dg] = [int(x) for x in kp.a4(dg)]
    return A4[dg]
def a_mapping(dg, e, e2):
    for a in a4_of(dg):
        if perms[a][e] == e2:
            return a
    return None
key0 = next(k for k, v in B.cls.items() if v[1] == 0)
ONE = (Q5(1), Q5(0), Q5(0), Q5(0))
J0 = kidx_of(B.cls[key0][0], ONE)   # canonical lattice region [u, 1](A_rep) is the chamber of x -> u x
assert sorted(sig[int(piece_of[J0])]) == sorted(B.cls[key0][2]), 'lattice class frame'
def solve(assoc):
    """assoc: dict cap of r0 -> family grip. Returns (ok, reason, t)."""
    def e_of(dg, j):   # association of slot j(r0) in cap dg
        ji = inv[j]
        d0 = int(perms[ji][dg])
        return int(perms[j][assoc[d0]])
    t = {key0: J0}
    q = deque([key0])
    while q:
        key = q.popleft(); j = t[key]
        slot = int(piece_of[j])
        for dg in allowed_out[key]:
            if dg in sig[slot]:
                return False, ('iii', key, dg), t
        for dg in allowed_in[key]:
            if dg not in sig[slot]:
                return False, ('i', key, dg), t
            e = e_of(dg, j)
            for ri, k2, kk in TR[(key, dg)]:
                e2 = hmap[(dg, ri)][e]
                a = a_mapping(dg, e, e2)
                if a is None:
                    return False, ('tau', key, dg, ri), t
                j2 = kp.compose(kk, kp.compose(a, j))
                if k2 in t:
                    if t[k2] != j2:
                        return False, ('ii', key, dg, ri, k2), t
                else:
                    t[k2] = j2; q.append(k2)
    return True, None, t
lens = [(a, b) for a in sig0 for b in sig0 if a < b and b in F[a]]
choices = {}
for c in sig0:
    fam = sorted(F[c])
    choices[c] = fam
forced = {}
for a, b in lens:
    forced[a] = [b]; forced[b] = [a]
caps_order = sig0
opts = [forced.get(c, choices[c]) for c in caps_order]
print('lens pairs', lens, 'option counts', [len(x) for x in opts], flush=True)
reasons = Counter(); sols = []
for combo in product(*opts):
    assoc = dict(zip(caps_order, combo))
    ok, why, t = solve(assoc)
    if ok:
        sols.append({'assoc': {str(k): v for k, v in assoc.items()}, 'classes_assigned': len(t)})
        print('SOLUTION', assoc, len(t), flush=True)
    else:
        reasons[why[0]] += 1
res = {'orbit_index': o, 'model_orbit': reg.report['kplus_orbit_g_classes'][str(o)], 'states': len(seen),
       'classes': len(B.cls), 'rep': r0, 'rep_caps': sig0, 'lens_pairs': lens, 'combos': int(np.prod([len(x) for x in opts])),
       'solutions': sols, 'failure_reasons': dict(reasons), 'sec': round(time.time() - t0, 1)}
Path(OUT).write_text(json.dumps(res, indent=1) + '\n')
print(json.dumps({k: v for k, v in res.items() if k != 'solutions'}), 'solutions', len(sols))
