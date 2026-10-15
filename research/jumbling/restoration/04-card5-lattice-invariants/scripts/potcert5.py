"""Certificate POT5: an orbit-5 potential for the menu I_a (exact).

Objects. The refined single-piece dynamics of orbit 5 (refexact.py; classes with exact representatives), the
36-degree family F5(d) of each grip d (grips at 36 degrees sharing an orbit-5 chamber with cap d), an association
e: (slot t of cap d) -> F5(d), K+-equivariant, given on the 5 caps of the representative chamber, and the potential
pi: class -> K+ index (pi(R_class) = chamber t_class).
Checks (all exact):
  S1  every menu element of Lambda_0 maps F5(0) onto F5(0) (pole images exact); its permutation of F5(0) is even;
  S2  A4_0 acts simply transitively on F5(0);
  S3  the orbit-5 representative chamber has trivial isometry group (distinct vertex Gram profiles, vertices span R^4),
      so a posed region determines its class and its K+ frame;
  P   for every class j, every grip d allowed at j (some refined state of class j has d unblocked), and every row h of
      Lambda_d:  (i) pi(R_j) lies in cap d when d holds R_j, (ii) pi(h R_j) = tau_d(h) pi(R_j), and (iii) pi(R_j) does
      not lie in cap d when R_j lies outside cap d; tau_d(h)(t) = a(t) with a the unique element of A4_d mapping e_d(t)
      to h(e_d(t)).
usage: potcert5.py <research/jumbling> <out json> [assoc index]"""
import json, sys, time, hashlib
from collections import Counter, deque
from itertools import product
from pathlib import Path
import numpy as np
sys.path.insert(0, str(Path(__file__).resolve().parent))
import refexact as RX
t0 = time.time()
C = RX.Ctx(sys.argv[1])
OUT = sys.argv[2]
AI = int(sys.argv[3]) if len(sys.argv) > 3 else 0
T, Q5, kp, reg, d = C.T, C.Q5, C.kp, C.reg, C.d
o = 7                      # K+-orbit index of model orbit 5
assert C.reg.report['kplus_orbit_g_classes'][str(o)] == [5]
R = RX.explore(C, o, 40, 10 ** 6)
assert R.complete
B = R.B
print('refined dynamics: states', len(R.seen), 'classes', len(B.rep), 'sec', round(time.time() - t0, 1), flush=True)
perms = C.perms
pidx = {tuple(perms[k].tolist()): k for k in range(len(perms))}
inv = np.array([pidx[tuple(np.argsort(perms[k]).tolist())] for k in range(len(perms))])
piece_of = reg.piece_of[o].astype(np.int64)
sig = {int(p): set(int(c) for c in d.signature_list(int(p))) for p in piece_of.tolist()}
r0 = int(reg.reps[o]); sig0 = sorted(sig[r0])
# exact 36-degree inner product
dots = {}
for c in range(600):
    dots.setdefault(C.dot(C.N[0], C.N[c]), []).append(c)
ang0 = np.degrees(np.arccos(np.clip(C.NFu @ C.NFu[0], -1, 1)))
v36 = [v for v, cs in dots.items() if abs(ang0[cs[0]] - 36.0) < 0.3]
assert len(v36) == 1
v36 = v36[0]
def fam(c):
    """F5(c): grips e with exact inner product v36 sharing an orbit-5 chamber with c."""
    out = set()
    for p, s in sig.items():
        if c in s:
            for e in s:
                if e != c and C.dot(C.N[c], C.N[e]) == v36:
                    out.add(e)
    return out
F0 = sorted(fam(0))
assert len(F0) == 12
FAM = {}
def F(c):
    if c not in FAM:
        k = C.to_grip[c]
        FAM[c] = {int(perms[k][e]) for e in F0}
    return FAM[c]
# exact pole images under menu rows
def same_dir(x, y):
    i = next(k for k in range(4) if not y[k].is_zero())
    lam = x[i] / y[i]
    return lam.sign() > 0 and all(x[k] == lam * y[k] for k in range(4))
HM = {}
def himg(c, ri, e):
    key = (c, ri, e)
    g = HM.get(key)
    if g is None:
        L, Rt, ret = C.MENU[c][ri]
        y = T.qmul(T.qmul(L, C.N[e]), T.qconj(Rt))
        yf = np.array([float(v) for v in y]); yf /= np.linalg.norm(yf)
        cands = np.nonzero(np.abs(C.NFu @ yf - 1) < 1e-9)[0].tolist()
        g = None
        for cnd in cands:
            if same_dir(y, C.N[cnd]):
                g = cnd
        HM[key] = g
    return g
# S1, S2 at grip 0
s1_ok, parities = True, Counter()
for ri in range(60):
    imgs = [himg(0, ri, e) for e in F0]
    if any(g is None or g not in set(F0) for g in imgs) or len(set(imgs)) != 12:
        s1_ok = False
        continue
    pi_ = [F0.index(g) for g in imgs]
    seen_ = [False] * 12; par = 0
    for i in range(12):
        if not seen_[i]:
            jj, ln = i, 0
            while not seen_[jj]:
                seen_[jj] = True; jj = pi_[jj]; ln += 1
            par += ln - 1
    parities[par % 2] += 1
A40 = [int(x) for x in kp.a4(0)]
s2_ok = bool(len(A40) == 12 and all(sorted(int(perms[a][e]) for a in A40) == F0 for e in F0))
# S3: isometry group of the representative chamber
V = reg.rep_vertices[o]
prof = [tuple(sorted((str(C.dot(V[i], V[k])) for k in range(len(V))))) for i in range(len(V))]
Vf = np.array([[float(x) for x in v] for v in V])
s3_ok = bool(len(set(prof)) == len(prof) and int(np.linalg.matrix_rank(Vf)) == 4)
print('S1', s1_ok, dict(parities), 'S2', s2_ok, 'S3', s3_ok, flush=True)
assert s1_ok and parities.get(1, 0) == 0 and s2_ok and s3_ok
# association choices on the representative's caps (lens partners forced)
lens = [(a, b) for a in sig0 for b in sig0 if a < b and b in F(a)]
opts = []
for c in sig0:
    part = [b for a, b in lens if a == c] + [a for a, b in lens if b == c]
    opts.append(part if part else sorted(F(c)))
combos = list(product(*opts))
assoc = dict(zip(sig0, combos[AI % len(combos)]))
def e_of(dg, j):
    d0 = int(perms[inv[j]][dg])
    return int(perms[j][assoc[d0]])
A4c = {}
def a_map(dg, e, e2):
    if dg not in A4c:
        A4c[dg] = [int(x) for x in kp.a4(dg)]
    hits = [a for a in A4c[dg] if perms[a][e] == e2]
    assert len(hits) == 1
    return hits[0]
k0 = 0
ONE = (Q5(1), Q5(0), Q5(0), Q5(0))
# lattice class: canonical rep is a unit u; its region is the chamber of x -> u x
J0 = C.kidx_of(C.UN.index(B.rep[k0]) if B.rep[k0] in C.UN else None, ONE) if B.rep[k0] in C.UN else None
if J0 is None:
    u = tuple(-x for x in B.rep[k0])
    J0 = C.kidx_of(C.UN.index(u), ONE)
assert sorted(sig[int(piece_of[J0])]) == sorted(B.caps[k0])
tbl = {k0: J0}
q = deque([k0]); checks = Counter()
while q:
    j = q.popleft(); jt = tbl[j]
    slot = int(piece_of[jt])
    for dg in R.allowed_out.get(j, ()):
        assert dg not in sig[slot], ('iii', j, dg)
        checks['iii'] += 1
    for dg in R.allowed_in.get(j, ()):
        assert dg in sig[slot], ('i', j, dg)
        checks['i'] += 1
        e = e_of(dg, jt)
        assert e in F(dg)
        for ri, j2, kk in R.TR[(j, dg)]:
            e2 = himg(dg, ri, e)
            assert e2 is not None and e2 in F(dg), ('tau', j, dg, ri)
            a = a_map(dg, e, e2)
            j2t = kp.compose(kk, kp.compose(a, jt))
            if j2 in tbl:
                assert tbl[j2] == j2t, ('ii', j, dg, ri)
            else:
                tbl[j2] = j2t; q.append(j2)
            checks['ii'] += 1
assert len(tbl) == len(B.rep)
res = {'certificate': 'POT5', 'kplus_orbit': o, 'model_orbit': 5, 'refined_states': len(R.seen),
       'classes': len(B.rep), 'classes_by_height': dict(sorted(Counter(B.height).items())), 'class_digest': R.digest,
       'family_F5_0': F0, 'S1_menu_preserves_F5': s1_ok, 'S1_parities': dict(parities), 'S2_A4_simply_transitive': s2_ok,
       'S3_trivial_isometry_group': s3_ok, 'association_on_rep_caps': {str(k): int(v) for k, v in assoc.items()},
       'lens_pairs': lens, 'association_choices': len(combos), 'checks': dict(checks),
       'potential': [{'class': j, 'rep': [str(x) for x in B.rep[j]], 'height': B.height[j], 'kplus_index': int(tbl[j]),
                      'slot': int(piece_of[tbl[j]])} for j in range(len(B.rep))],
       'all_checks_passed': True, 'sec': round(time.time() - t0, 1)}
Path(OUT).write_text(json.dumps(res, indent=1) + '\n')
print(json.dumps({k: v for k, v in res.items() if k != 'potential'}))
