"""Exact certificate for the conservation windows used by the inner-orbit theorem.

For every chamber orbit o (centres = -1 and moving orbits 0..34) compute exactly
  lambda_o = (|S| kappa)^2 / |sum_{c in S} n_c|^2,  S = signature of a representative home,
a lower bound for |x|^2 on that home (Cauchy-Schwarz: x lies in every cap of S). Then construct exact interior
points z_{-1}, z_0, z_1 (Q-rational coordinates) of a centre chamber, an orbit-0 home and an orbit-1 home, verify
every strict chamber inequality exactly (n_c.z > kappa on S, < kappa off S, < rho^2 for all 600 facets), and
verify  |z_-1|^2 < min_{o>=0} lambda_o,  |z_0|^2 < min_{o>=1} lambda_o,  |z_1|^2 < min_{o>=2} lambda_o."""
import json, sys
from fractions import Fraction
from pathlib import Path
import numpy as np
J = Path(sys.argv[1]).resolve()
for p in (J / 'theory', J, J / 'fixtures'):
    sys.path.insert(0, str(p))
sys.dont_write_bytecode = True
import sim
from exact import Q5, dot
ctx = sim.get_context()
d = ctx.data
reg = ctx.regions
N = d.N
NN = d.NN
kap = d.KAPPA
NF = np.array([[float(v) for v in row] for row in N])
mo, mv = d.mask_offsets, d.mask_values
def sig(p):
    return [int(c) for c in mv[mo[p]:mo[p + 1]]]
lam = {}
rep = {}
for o in range(-1, 35):
    p = int(np.flatnonzero(d.orbit_id == o)[0])
    S = sig(p)
    v = [sum((N[c][i] for c in S), Q5(0)) for i in range(4)]
    lam[o] = (Q5(len(S)) * kap) * (Q5(len(S)) * kap) / dot(v, v)
    rep[o] = p
def lam_min(os):
    best = None
    for o in os:
        if best is None or lam[o] < best[1]:
            best = (o, lam[o])
    return best
def interior(p, t):
    """(1-t) * float min-norm point + t * vertex average, rounded to rationals; exact strict check."""
    V = np.array([[float(x) for x in v] for v in reg.vertices(p)])
    w = np.ones(len(V)) / len(V)
    for it in range(20000):
        x = w @ V; g = V @ x; j = int(np.argmin(g))
        dv = -w.copy(); dv[j] += 1; dx = dv @ V; den = dx @ dx
        if den < 1e-30: break
        s = max(0.0, min(1.0, -(x @ dx) / den))
        if s < 1e-14: break
        w = w + s * dv
    xmin = w @ V
    cen = V.mean(axis=0)
    zf = (1 - t) * xmin + t * cen
    z = [Q5(Fraction(round(c * 10**8), 10**8).numerator, 0, Fraction(round(c * 10**8), 10**8).denominator) for c in zf]
    S = set(sig(p))
    ok = True
    for c in range(600):
        s = dot(N[c], z) - kap
        if c in S:
            ok &= s.sign() > 0
        else:
            ok &= s.sign() < 0
        ok &= (dot(N[c], z) - NN).sign() < 0
        if not ok:
            break
    return z, ok
out = {'lambda_over_rho2_float': {str(o): float(lam[o]) / float(NN) for o in lam}}
res = {}
for o, others, t in ((-1, range(0, 35), 0.02), (0, range(1, 35), 0.02), (1, range(2, 35), 0.01)):
    om, lm = lam_min(others)
    for tt in (t, t / 2, t / 4, t / 8, t / 16):
        z, ok = interior(rep[o], tt)
        n2 = dot(z, z)
        if ok and n2 < lm:
            break
    res[o] = {'home': rep[o], 'z': [[x.a, x.b, x.d] if hasattr(x, 'a') else str(x) for x in z], 'z_float': [float(x) for x in z],
              'interior_exact': ok, 'norm2_over_rho2': float(n2) / float(NN), 'bound_orbit': om,
              'bound_lambda_over_rho2': float(lm) / float(NN), 'norm2_below_bound_exact': bool(n2 < lm)}
    print(o, res[o]['interior_exact'], res[o]['norm2_over_rho2'], '<', res[o]['bound_lambda_over_rho2'], 'from orbit', om,
          res[o]['norm2_below_bound_exact'], flush=True)
out['windows'] = res
out['all_pass'] = all(r['interior_exact'] and r['norm2_below_bound_exact'] for r in res.values())
json.dump(out, open(sys.argv[2], 'w'), indent=1)
print('all pass', out['all_pass'])
